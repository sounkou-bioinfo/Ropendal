use std::os::raw::c_char;
use std::ptr;
use std::slice;

use crate::codec::{decode_bytes, encode_bytes};

use super::{CErrorInfo, c_str, ropendal_bytes, ropendal_error, set_c_error};

fn invalid_arg(message: &str, operation: &str) -> CErrorInfo {
    CErrorInfo {
        status: 2,
        kind: "InvalidArgument".to_string(),
        message: message.to_string(),
        operation: operation.to_string(),
        path: String::new(),
    }
}

fn codec_error(message: String, operation: &str) -> CErrorInfo {
    let unsupported = message.starts_with("unsupported codec");
    CErrorInfo {
        status: if unsupported { 3 } else { 1 },
        kind: if unsupported {
            "Unsupported".to_string()
        } else {
            "Unexpected".to_string()
        },
        message,
        operation: operation.to_string(),
        path: String::new(),
    }
}

unsafe fn input_slice<'a>(src: *const u8, src_len: usize) -> Result<&'a [u8], CErrorInfo> {
    if src_len == 0 {
        return Ok(&[]);
    }
    if src.is_null() {
        return Err(invalid_arg(
            "src pointer is null with non-zero length",
            "codec",
        ));
    }
    Ok(unsafe { slice::from_raw_parts(src, src_len) })
}

unsafe fn codec_transform(
    operation: &str,
    codec: *const c_char,
    src: *const u8,
    src_len: usize,
    out: *mut *mut ropendal_bytes,
    err: *mut *mut ropendal_error,
    transform: fn(&str, &[u8]) -> Result<Vec<u8>, String>,
) -> i32 {
    if out.is_null() {
        set_c_error(err, invalid_arg("out pointer is null", operation));
        return 2;
    }
    unsafe { *out = ptr::null_mut() };

    let codec = match unsafe { c_str(codec) } {
        Ok(v) => v,
        Err(mut e) => {
            e.operation = operation.to_string();
            set_c_error(err, e);
            return 2;
        }
    };
    let input = match unsafe { input_slice(src, src_len) } {
        Ok(v) => v,
        Err(mut e) => {
            e.operation = operation.to_string();
            set_c_error(err, e);
            return 2;
        }
    };
    match transform(&codec, input) {
        Ok(bytes) => {
            unsafe { *out = Box::into_raw(Box::new(ropendal_bytes { bytes })) };
            0
        }
        Err(message) => {
            let info = codec_error(message, operation);
            let status = info.status;
            set_c_error(err, info);
            status
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_codec_encode(
    codec: *const c_char,
    src: *const u8,
    src_len: usize,
    out: *mut *mut ropendal_bytes,
    err: *mut *mut ropendal_error,
) -> i32 {
    unsafe { codec_transform("codec_encode", codec, src, src_len, out, err, encode_bytes) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_codec_decode(
    codec: *const c_char,
    src: *const u8,
    src_len: usize,
    out: *mut *mut ropendal_bytes,
    err: *mut *mut ropendal_error,
) -> i32 {
    unsafe { codec_transform("codec_decode", codec, src, src_len, out, err, decode_bytes) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_bytes_data(bytes: *const ropendal_bytes) -> *const u8 {
    if bytes.is_null() || unsafe { (*bytes).bytes.is_empty() } {
        ptr::null()
    } else {
        unsafe { (*bytes).bytes.as_ptr() }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_bytes_len(bytes: *const ropendal_bytes) -> usize {
    if bytes.is_null() {
        0
    } else {
        unsafe { (*bytes).bytes.len() }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_bytes_release(bytes: *mut ropendal_bytes) {
    if !bytes.is_null() {
        unsafe { drop(Box::from_raw(bytes)) };
    }
}
