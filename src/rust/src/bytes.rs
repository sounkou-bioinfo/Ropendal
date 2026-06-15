use std::ffi::{c_int, c_void};
use std::sync::Once;

use opendal::Buffer;
use savvy::ffi::SEXP;
use savvy::{RawSexp, Sexp, savvy};

use crate::r_values::{buffer_to_raw_sexp, real_scalar};

unsafe extern "C" {
    static mut R_NilValue: SEXP;

    fn R_MakeExternalPtr(p: *mut c_void, tag: SEXP, prot: SEXP) -> SEXP;
    fn R_PreserveObject(x: SEXP);
    fn R_ExternalPtrTag(s: SEXP) -> SEXP;
    fn R_SetExternalPtrTag(s: SEXP, tag: SEXP);
    fn Rf_protect(x: SEXP) -> SEXP;
    fn Rf_unprotect(n: c_int);
}

struct LocalProtect;

impl Drop for LocalProtect {
    fn drop(&mut self) {
        unsafe { Rf_unprotect(1) };
    }
}

fn local_protect(value: &Sexp) -> LocalProtect {
    unsafe { Rf_protect(value.0) };
    LocalProtect
}

static OPENDAL_BYTES_TAG_INIT: Once = Once::new();
static mut OPENDAL_BYTES_TAG: SEXP = std::ptr::null_mut();
static OPENDAL_BYTES_TAG_SENTINEL: u8 = 0;

fn opendal_bytes_tag() -> SEXP {
    OPENDAL_BYTES_TAG_INIT.call_once(|| unsafe {
        let tag = R_MakeExternalPtr(
            (&OPENDAL_BYTES_TAG_SENTINEL as *const u8)
                .cast_mut()
                .cast::<c_void>(),
            R_NilValue,
            R_NilValue,
        );
        let tag_sexp = Sexp(tag);
        let _tag_guard = local_protect(&tag_sexp);
        R_PreserveObject(tag);
        OPENDAL_BYTES_TAG = tag;
    });

    unsafe { OPENDAL_BYTES_TAG }
}

fn tag_opendal_bytes_ptr(value: &Sexp) {
    unsafe { R_SetExternalPtrTag(value.0, opendal_bytes_tag()) };
}

fn is_tagged_opendal_bytes_ptr(value: &Sexp) -> bool {
    unsafe { R_ExternalPtrTag(value.0) == opendal_bytes_tag() }
}

/// Immutable Rust-owned byte buffer.
///
/// @export
#[savvy]
pub struct OpendalBytes {
    buffer: Buffer,
}

impl OpendalBytes {
    pub(crate) fn new(buffer: Buffer) -> Self {
        Self { buffer }
    }

    pub(crate) fn buffer(&self) -> Buffer {
        self.buffer.clone()
    }
}

#[savvy]
fn opendal_bytes_len(bytes: Sexp) -> savvy::Result<savvy::Sexp> {
    let Some(buffer) = buffer_from_opendal_bytes_sexp(&bytes)? else {
        return Err(savvy::Error::new("expected OpendalBytes"));
    };
    real_scalar(buffer.len() as f64)?.into()
}

#[savvy]
fn opendal_bytes_as_raw(bytes: Sexp) -> savvy::Result<savvy::Sexp> {
    let Some(buffer) = buffer_from_opendal_bytes_sexp(&bytes)? else {
        return Err(savvy::Error::new("expected OpendalBytes"));
    };
    buffer_to_raw_sexp(buffer).map(|x| x.into())
}

#[savvy]
fn opendal_bytes_from_raw(data: RawSexp) -> savvy::Result<savvy::Sexp> {
    opendal_bytes_to_sexp(Buffer::from(data.to_vec()))
}

#[savvy]
fn opendal_bytes_slice(bytes: Sexp, offset: f64, size: Option<f64>) -> savvy::Result<savvy::Sexp> {
    let Some(buffer) = buffer_from_opendal_bytes_sexp(&bytes)? else {
        return Err(savvy::Error::new("expected OpendalBytes"));
    };
    let len = buffer.len();
    let start = numeric_byte_count(offset, "offset")?;
    let requested = match size {
        Some(value) => Some(numeric_byte_count(value, "size")?),
        None => None,
    };
    if start >= len {
        return opendal_bytes_to_sexp(Buffer::new());
    }
    let end = match requested {
        Some(n) => start.saturating_add(n).min(len),
        None => len,
    };
    opendal_bytes_to_sexp(buffer.slice(start..end))
}

fn numeric_byte_count(value: f64, name: &str) -> savvy::Result<usize> {
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 {
        return Err(savvy::Error::new(format!(
            "{name} must be a non-negative whole number"
        )));
    }
    if value > usize::MAX as f64 {
        return Err(savvy::Error::new(format!(
            "{name} is too large for this platform"
        )));
    }
    Ok(value as usize)
}

pub(crate) fn opendal_bytes_to_sexp(buffer: Buffer) -> savvy::Result<savvy::Sexp> {
    let mut out = <savvy::Sexp>::try_from(OpendalBytes::new(buffer))?;
    let _out_guard = local_protect(&out);
    tag_opendal_bytes_ptr(&out);
    out.set_class([
        "Ropendal::OpendalBytes",
        "OpendalBytes",
        "savvy_Ropendal__sealed",
    ])?;
    Ok(out)
}

pub(crate) fn buffer_from_opendal_bytes_sexp(value: &Sexp) -> savvy::Result<Option<Buffer>> {
    let Some(classes) = value.get_class() else {
        return Ok(None);
    };
    if !classes.iter().any(|class| *class == "OpendalBytes") {
        return Ok(None);
    }

    let ptr_value = if value.is_environment() {
        let env = savvy::EnvironmentSexp(value.0);
        let Some(ptr) = env.get(".ptr")? else {
            return Err(savvy::Error::new("OpendalBytes object is missing .ptr"));
        };
        ptr
    } else {
        Sexp(value.0)
    };

    ptr_value.assert_external_pointer()?;
    if !is_tagged_opendal_bytes_ptr(&ptr_value) {
        return Err(savvy::Error::new("invalid OpendalBytes pointer"));
    }

    let ptr = unsafe { savvy::get_external_pointer_addr(ptr_value.0)? as *const OpendalBytes };
    let Some(bytes) = (unsafe { ptr.as_ref() }) else {
        return Err(savvy::Error::new("invalid OpendalBytes pointer"));
    };
    Ok(Some(bytes.buffer()))
}
