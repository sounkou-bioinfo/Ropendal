use savvy::{OwnedIntegerSexp, OwnedListSexp, OwnedLogicalSexp, OwnedRealSexp, OwnedStringSexp};

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
