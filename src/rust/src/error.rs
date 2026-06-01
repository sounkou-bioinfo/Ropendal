use opendal::{Error, ErrorKind};
use savvy::OwnedListSexp;

use crate::r_values::{bool_scalar, int_scalar, str_scalar};

pub(crate) fn kind_code(kind: ErrorKind) -> i32 {
    match kind {
        ErrorKind::Unexpected => 1,
        ErrorKind::Unsupported => 2,
        ErrorKind::ConfigInvalid => 3,
        ErrorKind::NotFound => 4,
        ErrorKind::PermissionDenied => 5,
        ErrorKind::IsADirectory => 6,
        ErrorKind::NotADirectory => 7,
        ErrorKind::AlreadyExists => 8,
        ErrorKind::RateLimited => 9,
        ErrorKind::IsSameFile => 10,
        ErrorKind::ConditionNotMatch => 11,
        ErrorKind::RangeNotSatisfied => 12,
        _ => 1,
    }
}

pub(crate) fn error_list(
    kind: &str,
    code: i32,
    message: &str,
    operation: &str,
    path: &str,
) -> savvy::Result<savvy::Sexp> {
    let mut out = OwnedListSexp::new(6, true)?;
    out.set_name_and_value(0, "__ropendal_error__", bool_scalar(true)?)?;
    out.set_name_and_value(1, "code", int_scalar(code)?)?;
    out.set_name_and_value(2, "kind", str_scalar(kind)?)?;
    out.set_name_and_value(3, "message", str_scalar(message)?)?;
    out.set_name_and_value(4, "operation", str_scalar(operation)?)?;
    out.set_name_and_value(5, "path", str_scalar(path)?)?;
    let kind_class = format!("opendal{kind}Value");
    out.set_class(&[
        kind_class.as_str(),
        "opendalErrorValue",
        "errorValue",
        "list",
    ])?;
    out.into()
}

pub(crate) fn op_error_list(err: Error, operation: &str, path: &str) -> savvy::Result<savvy::Sexp> {
    let kind = err.kind();
    error_list(
        kind.into_static(),
        kind_code(kind),
        &err.to_string(),
        operation,
        path,
    )
}

pub(crate) fn unsupported_value(operation: &str, path: &str) -> savvy::Result<savvy::Sexp> {
    error_list(
        "Unsupported",
        kind_code(ErrorKind::Unsupported),
        "operation is not supported",
        operation,
        path,
    )
}
