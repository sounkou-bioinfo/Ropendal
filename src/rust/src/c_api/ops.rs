use std::os::raw::c_void;
use std::ptr;

use crate::ops::{read_bytes, write_bytes};
use crate::path::normalize_user_path;

use super::{c_error_from_opendal, c_str, set_c_error, CErrorInfo};
use super::{ropendal_aio, ropendal_error, ropendal_fs, ropendal_read_options, ropendal_write_options, COutcome};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_read_into_aio(
    fs: *mut ropendal_fs,
    opts: *const ropendal_read_options,
    dst: *mut u8,
    dst_len: usize,
    out: *mut *mut ropendal_aio,
    err: *mut *mut ropendal_error,
) -> i32 {
    if fs.is_null() || opts.is_null() || out.is_null() || dst.is_null() {
        set_c_error(
            err,
            CErrorInfo {
                status: 2,
                kind: "InvalidArgument".to_string(),
                message: "required pointer is null".to_string(),
                operation: "read_into".to_string(),
                path: String::new(),
            },
        );
        return 2;
    }
    let opt = &*opts;
    let path_raw = match c_str(opt.path) {
        Ok(v) => v,
        Err(mut e) => {
            e.operation = "read_into".to_string();
            set_c_error(err, e);
            return 2;
        }
    };
    let path = match normalize_user_path(&path_raw, false) {
        Ok(p) => p,
        Err(msg) => {
            set_c_error(
                err,
                CErrorInfo {
                    status: 2,
                    kind: "InvalidArgument".to_string(),
                    message: msg,
                    operation: "read_into".to_string(),
                    path: path_raw,
                },
            );
            return 2;
        }
    };
    if opt.has_size != 0 && opt.size as usize > dst_len {
        set_c_error(
            err,
            CErrorInfo {
                status: 2,
                kind: "InvalidArgument".to_string(),
                message: "destination buffer is smaller than requested size".to_string(),
                operation: "read_into".to_string(),
                path,
            },
        );
        return 2;
    }
    let native = (*fs).native.clone();
    let op = native.op.clone();
    let runtime = native.runtime.clone();
    let offset = opt.offset;
    let size = if opt.has_size != 0 { Some(opt.size) } else { None };
    let dst_addr = dst as usize;
    let callback = opt.callback;
    let userdata_addr = opt.userdata as usize;
    let handle = runtime.spawn(async move {
        let result = match read_bytes(op, path.clone(), offset, size).await {
            Ok(bytes) => {
                if bytes.len() > dst_len {
                    COutcome::Error(CErrorInfo {
                        status: 2,
                        kind: "InvalidArgument".to_string(),
                        message: "destination buffer is smaller than result".to_string(),
                        operation: "read_into".to_string(),
                        path,
                    })
                } else {
                    unsafe {
                        ptr::copy_nonoverlapping(bytes.as_ptr(), dst_addr as *mut u8, bytes.len());
                    }
                    COutcome::Nread(bytes.len())
                }
            }
            Err(e) => COutcome::Error(c_error_from_opendal(e, "read_into", &path)),
        };
        if let Some(cb) = callback {
            cb(ptr::null_mut(), userdata_addr as *mut c_void);
        }
        result
    });
    *out = Box::into_raw(Box::new(ropendal_aio {
        runtime,
        handle: std::sync::Mutex::new(Some(handle)),
        cached: std::sync::Mutex::new(None),
    }));
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_read_aio(
    fs: *mut ropendal_fs,
    opts: *const ropendal_read_options,
    out: *mut *mut ropendal_aio,
    err: *mut *mut ropendal_error,
) -> i32 {
    if fs.is_null() || opts.is_null() || out.is_null() {
        return 2;
    }
    let opt = &*opts;
    let path_raw = match c_str(opt.path) {
        Ok(v) => v,
        Err(mut e) => {
            e.operation = "read".to_string();
            set_c_error(err, e);
            return 2;
        }
    };
    let path = match normalize_user_path(&path_raw, false) {
        Ok(p) => p,
        Err(msg) => {
            set_c_error(
                err,
                CErrorInfo {
                    status: 2,
                    kind: "InvalidArgument".to_string(),
                    message: msg,
                    operation: "read".to_string(),
                    path: path_raw,
                },
            );
            return 2;
        }
    };
    let native = (*fs).native.clone();
    let op = native.op.clone();
    let runtime = native.runtime.clone();
    let offset = opt.offset;
    let size = if opt.has_size != 0 { Some(opt.size) } else { None };
    let handle = runtime.spawn(async move {
        match read_bytes(op, path.clone(), offset, size).await {
            Ok(bytes) => COutcome::Bytes(bytes),
            Err(e) => COutcome::Error(c_error_from_opendal(e, "read", &path)),
        }
    });
    *out = Box::into_raw(Box::new(ropendal_aio {
        runtime,
        handle: std::sync::Mutex::new(Some(handle)),
        cached: std::sync::Mutex::new(None),
    }));
    0
}

fn c_submit_write(
    fs: *mut ropendal_fs,
    opts: *const ropendal_write_options,
    src: *const u8,
    src_len: usize,
    out: *mut *mut ropendal_aio,
    err: *mut *mut ropendal_error,
    create_only: bool,
    append: bool,
    operation: &str,
) -> i32 {
    unsafe {
        if fs.is_null() || opts.is_null() || out.is_null() || (src.is_null() && src_len > 0) {
            set_c_error(
                err,
                CErrorInfo {
                    status: 2,
                    kind: "InvalidArgument".to_string(),
                    message: "required pointer is null".to_string(),
                    operation: operation.to_string(),
                    path: String::new(),
                },
            );
            return 2;
        }
        let opt = &*opts;
        let path_raw = match c_str(opt.path) {
            Ok(v) => v,
            Err(mut e) => {
                e.operation = operation.to_string();
                set_c_error(err, e);
                return 2;
            }
        };
        let path = match normalize_user_path(&path_raw, false) {
            Ok(p) => p,
            Err(msg) => {
                set_c_error(
                    err,
                    CErrorInfo {
                        status: 2,
                        kind: "InvalidArgument".to_string(),
                        message: msg,
                        operation: operation.to_string(),
                        path: path_raw,
                    },
                );
                return 2;
            }
        };
        let bytes = if src_len == 0 {
            Vec::new()
        } else {
            std::slice::from_raw_parts(src, src_len).to_vec()
        };
        let native = (*fs).native.clone();
        let op = native.op.clone();
        let runtime = native.runtime.clone();
        let operation_owned = operation.to_string();
        let handle = runtime.spawn(async move {
            match write_bytes(op, path.clone(), bytes, create_only, append).await {
                Ok(_) => COutcome::Unit,
                Err(e) => COutcome::Error(c_error_from_opendal(e, &operation_owned, &path)),
            }
        });
        *out = Box::into_raw(Box::new(ropendal_aio {
            runtime,
            handle: std::sync::Mutex::new(Some(handle)),
            cached: std::sync::Mutex::new(None),
        }));
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_write_aio(
    fs: *mut ropendal_fs,
    opts: *const ropendal_write_options,
    src: *const u8,
    src_len: usize,
    out: *mut *mut ropendal_aio,
    err: *mut *mut ropendal_error,
) -> i32 {
    c_submit_write(fs, opts, src, src_len, out, err, true, false, "write")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_replace_aio(
    fs: *mut ropendal_fs,
    opts: *const ropendal_write_options,
    src: *const u8,
    src_len: usize,
    out: *mut *mut ropendal_aio,
    err: *mut *mut ropendal_error,
) -> i32 {
    c_submit_write(fs, opts, src, src_len, out, err, false, false, "replace")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_append_aio(
    fs: *mut ropendal_fs,
    opts: *const ropendal_write_options,
    src: *const u8,
    src_len: usize,
    out: *mut *mut ropendal_aio,
    err: *mut *mut ropendal_error,
) -> i32 {
    c_submit_write(fs, opts, src, src_len, out, err, false, true, "append")
}

macro_rules! unsupported_c_fn {
    ($name:ident ( $($arg:ident : $typ:ty),* )) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name($($arg:$typ),*) -> i32 {
            $(let _ = $arg;)*
            3
        }
    };
}

unsupported_c_fn!(ropendal_readv_aio(fs: *mut ropendal_fs, requests: *const c_void, n_requests: usize, opts: *const c_void, out: *mut *mut ropendal_aio, err: *mut *mut ropendal_error));
unsupported_c_fn!(ropendal_readv_into_aio(fs: *mut ropendal_fs, requests: *const c_void, n_requests: usize, opts: *const c_void, out: *mut *mut ropendal_aio, err: *mut *mut ropendal_error));
unsupported_c_fn!(ropendal_stat_aio(fs: *mut ropendal_fs, path: *const std::os::raw::c_char, callback: *mut c_void, userdata: *mut c_void, out: *mut *mut ropendal_aio, err: *mut *mut ropendal_error));
unsupported_c_fn!(ropendal_ls_aio(fs: *mut ropendal_fs, opts: *const c_void, out: *mut *mut ropendal_aio, err: *mut *mut ropendal_error));
unsupported_c_fn!(ropendal_delete_aio(fs: *mut ropendal_fs, opts: *const c_void, out: *mut *mut ropendal_aio, err: *mut *mut ropendal_error));
unsupported_c_fn!(ropendal_copy_aio(fs: *mut ropendal_fs, from: *const std::os::raw::c_char, to: *const std::os::raw::c_char, callback: *mut c_void, userdata: *mut c_void, out: *mut *mut ropendal_aio, err: *mut *mut ropendal_error));
unsupported_c_fn!(ropendal_rename_aio(fs: *mut ropendal_fs, from: *const std::os::raw::c_char, to: *const std::os::raw::c_char, callback: *mut c_void, userdata: *mut c_void, out: *mut *mut ropendal_aio, err: *mut *mut ropendal_error));
unsupported_c_fn!(ropendal_mkdir_aio(fs: *mut ropendal_fs, path: *const std::os::raw::c_char, callback: *mut c_void, userdata: *mut c_void, out: *mut *mut ropendal_aio, err: *mut *mut ropendal_error));
