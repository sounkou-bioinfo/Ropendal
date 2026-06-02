use opendal::Buffer;
use savvy::{
    OwnedIntegerSexp, OwnedListSexp, OwnedLogicalSexp, OwnedRawSexp, OwnedRealSexp, OwnedStringSexp,
};

pub(crate) fn str_scalar(value: &str) -> savvy::Result<OwnedStringSexp> {
    let mut out = OwnedStringSexp::new(1)?;
    out.set_elt(0, value)?;
    Ok(out)
}

pub(crate) fn bool_scalar(value: bool) -> savvy::Result<OwnedLogicalSexp> {
    let mut out = OwnedLogicalSexp::new(1)?;
    out.set_elt(0, value)?;
    Ok(out)
}

pub(crate) fn int_scalar(value: i32) -> savvy::Result<OwnedIntegerSexp> {
    let mut out = OwnedIntegerSexp::new(1)?;
    out.set_elt(0, value)?;
    Ok(out)
}

pub(crate) fn real_scalar(value: f64) -> savvy::Result<OwnedRealSexp> {
    let mut out = OwnedRealSexp::new(1)?;
    out.set_elt(0, value)?;
    Ok(out)
}

pub(crate) fn set_str_or_null(
    out: &mut OwnedListSexp,
    i: usize,
    name: &str,
    value: Option<&str>,
) -> savvy::Result<()> {
    out.set_name(i, name)?;
    if let Some(v) = value {
        out.set_value(i, str_scalar(v)?)?;
    }
    Ok(())
}

pub(crate) fn success_value() -> savvy::Result<savvy::Sexp> {
    bool_scalar(true)?.into()
}

pub(crate) fn buffer_to_raw_sexp(buffer: Buffer) -> savvy::Result<OwnedRawSexp> {
    let len = buffer.len();
    let mut out = unsafe { OwnedRawSexp::new_without_init(len)? };
    copy_buffer_to_slice(buffer, out.as_mut_slice());
    Ok(out)
}

pub(crate) fn buffers_to_raw_sexp(buffers: Vec<Buffer>) -> savvy::Result<OwnedRawSexp> {
    let len = buffers.iter().map(Buffer::len).sum();
    let mut out = unsafe { OwnedRawSexp::new_without_init(len)? };
    let mut offset = 0;
    for buffer in buffers {
        let n = buffer.len();
        copy_buffer_to_slice(buffer, &mut out.as_mut_slice()[offset..offset + n]);
        offset += n;
    }
    Ok(out)
}

pub(crate) fn copy_buffer_to_slice(buffer: Buffer, dst: &mut [u8]) {
    let mut offset = 0;
    for chunk in buffer {
        let n = chunk.len();
        dst[offset..offset + n].copy_from_slice(&chunk);
        offset += n;
    }
}
