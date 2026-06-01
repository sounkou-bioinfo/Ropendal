use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use opendal::ErrorKind;

use super::types::ropendal_error;

#[derive(Clone)]
pub(crate) struct CErrorInfo {
    pub(crate) status: i32,
    pub(crate) kind: String,
    pub(crate) message: String,
    pub(crate) operation: String,
    pub(crate) path: String,
}

fn cstring_lossy(s: &str) -> CString {
    CString::new(s.replace('\0', " ")).unwrap_or_else(|_| CString::new("").unwrap())
}

pub(crate) fn set_c_error(err: *mut *mut ropendal_error, info: CErrorInfo) {
    if err.is_null() {
        return;
    }
    let boxed = Box::new(ropendal_error {
        status: info.status,
        kind: cstring_lossy(&info.kind),
        message: cstring_lossy(&info.message),
        operation: cstring_lossy(&info.operation),
        path: cstring_lossy(&info.path),
    });
    unsafe {
        *err = Box::into_raw(boxed);
    }
}

fn c_status_from_kind(kind: ErrorKind) -> i32 {
    match kind {
        ErrorKind::Unsupported => 3,
        ErrorKind::NotFound => 4,
        ErrorKind::PermissionDenied => 5,
        ErrorKind::ConditionNotMatch => 6,
        _ => 1,
    }
}

pub(crate) fn c_error_from_opendal(e: opendal::Error, operation: &str, path: &str) -> CErrorInfo {
    let kind = e.kind();
    CErrorInfo {
        status: c_status_from_kind(kind),
        kind: kind.into_static().to_string(),
        message: e.to_string(),
        operation: operation.to_string(),
        path: path.to_string(),
    }
}

pub(crate) unsafe fn c_str(ptr: *const c_char) -> Result<String, CErrorInfo> {
    if ptr.is_null() {
        return Err(CErrorInfo {
            status: 2,
            kind: "InvalidArgument".to_string(),
            message: "null string pointer".to_string(),
            operation: String::new(),
            path: String::new(),
        });
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map(|s| s.to_string())
        .map_err(|e| CErrorInfo {
            status: 2,
            kind: "InvalidArgument".to_string(),
            message: e.to_string(),
            operation: String::new(),
            path: String::new(),
        })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_error_message(err: *const ropendal_error) -> *const c_char {
    if err.is_null() {
        ptr::null()
    } else {
        (*err).message.as_ptr()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_error_kind(err: *const ropendal_error) -> *const c_char {
    if err.is_null() {
        ptr::null()
    } else {
        (*err).kind.as_ptr()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_error_operation(err: *const ropendal_error) -> *const c_char {
    if err.is_null() {
        ptr::null()
    } else {
        (*err).operation.as_ptr()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_error_path(err: *const ropendal_error) -> *const c_char {
    if err.is_null() {
        ptr::null()
    } else {
        (*err).path.as_ptr()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_error_release(err: *mut ropendal_error) {
    if !err.is_null() {
        drop(Box::from_raw(err));
    }
}
