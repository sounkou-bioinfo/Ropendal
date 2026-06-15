use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use opendal::Operator;

use crate::common::{NativeFs, build_runtime, init_registry};

use super::{CErrorInfo, c_error_from_opendal, c_str, set_c_error};
use super::{ropendal_error, ropendal_fs, ropendal_kv};

#[unsafe(no_mangle)]
pub extern "C" fn ropendal_api_version() -> u32 {
    4
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_fs_open(
    scheme: *const std::os::raw::c_char,
    config: *const ropendal_kv,
    config_len: usize,
    out: *mut *mut ropendal_fs,
    err: *mut *mut ropendal_error,
) -> i32 {
    if out.is_null() {
        set_c_error(
            err,
            CErrorInfo {
                status: 2,
                kind: "InvalidArgument".to_string(),
                message: "out pointer is null".to_string(),
                operation: "fs_open".to_string(),
                path: String::new(),
            },
        );
        return 2;
    }
    let scheme = match c_str(scheme) {
        Ok(v) => v,
        Err(mut e) => {
            e.operation = "fs_open".to_string();
            set_c_error(err, e);
            return 2;
        }
    };
    let mut pairs = Vec::with_capacity(config_len);
    if config_len > 0 && config.is_null() {
        set_c_error(
            err,
            CErrorInfo {
                status: 2,
                kind: "InvalidArgument".to_string(),
                message: "config pointer is null".to_string(),
                operation: "fs_open".to_string(),
                path: String::new(),
            },
        );
        return 2;
    }
    for i in 0..config_len {
        let kv = &*config.add(i);
        let key = match c_str(kv.key) {
            Ok(v) => v,
            Err(mut e) => {
                e.operation = "fs_open".to_string();
                set_c_error(err, e);
                return 2;
            }
        };
        let value = match c_str(kv.value) {
            Ok(v) => v,
            Err(mut e) => {
                e.operation = "fs_open".to_string();
                set_c_error(err, e);
                return 2;
            }
        };
        pairs.push((key, value));
    }
    init_registry();
    let op = match Operator::via_iter(&scheme, pairs) {
        Ok(op) => op,
        Err(e) => {
            set_c_error(err, c_error_from_opendal(e, "fs_open", ""));
            return 1;
        }
    };
    let info = op.info();
    let runtime = match build_runtime(None) {
        Ok(rt) => rt,
        Err(e) => {
            set_c_error(
                err,
                CErrorInfo {
                    status: 1,
                    kind: "Unexpected".to_string(),
                    message: e.to_string(),
                    operation: "fs_open".to_string(),
                    path: String::new(),
                },
            );
            return 1;
        }
    };
    let native = NativeFs {
        op,
        runtime,
        scheme: info.scheme().to_string(),
        root: info.root(),
    };
    *out = Box::into_raw(Box::new(ropendal_fs {
        refs: AtomicUsize::new(1),
        native: Arc::new(native),
    }));
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_fs_from_uri(
    uri: *const std::os::raw::c_char,
    out: *mut *mut ropendal_fs,
    err: *mut *mut ropendal_error,
) -> i32 {
    if out.is_null() {
        return 2;
    }
    let uri = match c_str(uri) {
        Ok(v) => v,
        Err(mut e) => {
            e.operation = "fs_from_uri".to_string();
            set_c_error(err, e);
            return 2;
        }
    };
    init_registry();
    let op = match Operator::from_uri(uri.as_str()) {
        Ok(op) => op,
        Err(e) => {
            set_c_error(err, c_error_from_opendal(e, "fs_from_uri", ""));
            return 1;
        }
    };
    let info = op.info();
    let runtime = match build_runtime(None) {
        Ok(rt) => rt,
        Err(e) => {
            set_c_error(
                err,
                CErrorInfo {
                    status: 1,
                    kind: "Unexpected".to_string(),
                    message: e.to_string(),
                    operation: "fs_from_uri".to_string(),
                    path: String::new(),
                },
            );
            return 1;
        }
    };
    let native = NativeFs {
        op,
        runtime,
        scheme: info.scheme().to_string(),
        root: info.root(),
    };
    *out = Box::into_raw(Box::new(ropendal_fs {
        refs: AtomicUsize::new(1),
        native: Arc::new(native),
    }));
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_fs_retain(fs: *mut ropendal_fs) {
    if !fs.is_null() {
        (*fs).refs.fetch_add(1, Ordering::Relaxed);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_fs_release(fs: *mut ropendal_fs) {
    if !fs.is_null() && (*fs).refs.fetch_sub(1, Ordering::Release) == 1 {
        std::sync::atomic::fence(Ordering::Acquire);
        drop(Box::from_raw(fs));
    }
}
