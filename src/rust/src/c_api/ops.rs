use std::os::raw::c_void;
use std::ptr;

use opendal::options::{DeleteOptions, ListOptions};

use crate::ops::{read_bytes, write_bytes};
use crate::path::normalize_user_path;
use crate::r_values::copy_buffer_to_slice;

use super::{AioCallback, CEntrySet, CErrorInfo, c_error_from_opendal, c_str, set_c_error};
use super::{
    COutcome, ropendal_aio, ropendal_delete_options, ropendal_error, ropendal_fs,
    ropendal_ls_options, ropendal_read_options, ropendal_write_options,
};

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
    let size = if opt.has_size != 0 {
        Some(opt.size)
    } else {
        None
    };
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
                    let n = bytes.len();
                    unsafe {
                        let dst = std::slice::from_raw_parts_mut(dst_addr as *mut u8, dst_len);
                        copy_buffer_to_slice(bytes, &mut dst[..n]);
                    }
                    COutcome::Nread(n)
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
    let size = if opt.has_size != 0 {
        Some(opt.size)
    } else {
        None
    };
    let handle = runtime.spawn(async move {
        match read_bytes(op, path.clone(), offset, size).await {
            Ok(bytes) => COutcome::Bytes(bytes.to_vec()),
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
            opendal::Buffer::new()
        } else {
            std::slice::from_raw_parts(src, src_len).to_vec().into()
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

fn submit_handle(
    runtime: std::sync::Arc<tokio::runtime::Runtime>,
    handle: tokio::task::JoinHandle<COutcome>,
    out: *mut *mut ropendal_aio,
) {
    unsafe {
        *out = Box::into_raw(Box::new(ropendal_aio {
            runtime,
            handle: std::sync::Mutex::new(Some(handle)),
            cached: std::sync::Mutex::new(None),
        }));
    }
}

fn invalid_ptr_error(err: *mut *mut ropendal_error, operation: &str) -> i32 {
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
    2
}

fn unsupported_option(
    err: *mut *mut ropendal_error,
    operation: &str,
    path: &str,
    message: &str,
) -> i32 {
    set_c_error(
        err,
        CErrorInfo {
            status: 3,
            kind: "Unsupported".to_string(),
            message: message.to_string(),
            operation: operation.to_string(),
            path: path.to_string(),
        },
    );
    3
}

fn submit_unit_path_op<F, Fut>(
    fs: *mut ropendal_fs,
    path: *const std::os::raw::c_char,
    callback: AioCallback,
    userdata: *mut c_void,
    out: *mut *mut ropendal_aio,
    err: *mut *mut ropendal_error,
    operation: &'static str,
    directory: bool,
    op_fn: F,
) -> i32
where
    F: FnOnce(opendal::Operator, String) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<(), opendal::Error>> + Send + 'static,
{
    unsafe {
        if fs.is_null() || path.is_null() || out.is_null() {
            return invalid_ptr_error(err, operation);
        }
        let path_raw = match c_str(path) {
            Ok(v) => v,
            Err(mut e) => {
                e.operation = operation.to_string();
                set_c_error(err, e);
                return 2;
            }
        };
        let path = match normalize_user_path(&path_raw, directory) {
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
        let native = (*fs).native.clone();
        let op = native.op.clone();
        let runtime = native.runtime.clone();
        let userdata_addr = userdata as usize;
        let handle = runtime.spawn(async move {
            let result = match op_fn(op, path.clone()).await {
                Ok(_) => COutcome::Unit,
                Err(e) => COutcome::Error(c_error_from_opendal(e, operation, &path)),
            };
            if let Some(cb) = callback {
                cb(ptr::null_mut(), userdata_addr as *mut c_void);
            }
            result
        });
        submit_handle(runtime, handle, out);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_stat_aio(
    fs: *mut ropendal_fs,
    path: *const std::os::raw::c_char,
    callback: AioCallback,
    userdata: *mut c_void,
    out: *mut *mut ropendal_aio,
    err: *mut *mut ropendal_error,
) -> i32 {
    if fs.is_null() || path.is_null() || out.is_null() {
        return invalid_ptr_error(err, "stat");
    }
    let path_raw = match c_str(path) {
        Ok(v) => v,
        Err(mut e) => {
            e.operation = "stat".to_string();
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
                    operation: "stat".to_string(),
                    path: path_raw,
                },
            );
            return 2;
        }
    };
    let native = (*fs).native.clone();
    let op = native.op.clone();
    let runtime = native.runtime.clone();
    let userdata_addr = userdata as usize;
    let handle = runtime.spawn(async move {
        let result = match op.stat(&path).await {
            Ok(meta) => COutcome::Entry(CEntrySet::one(&path, &meta)),
            Err(e) => COutcome::Error(c_error_from_opendal(e, "stat", &path)),
        };
        if let Some(cb) = callback {
            cb(ptr::null_mut(), userdata_addr as *mut c_void);
        }
        result
    });
    submit_handle(runtime, handle, out);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_exists_aio(
    fs: *mut ropendal_fs,
    path: *const std::os::raw::c_char,
    callback: AioCallback,
    userdata: *mut c_void,
    out: *mut *mut ropendal_aio,
    err: *mut *mut ropendal_error,
) -> i32 {
    if fs.is_null() || path.is_null() || out.is_null() {
        return invalid_ptr_error(err, "exists");
    }
    let path_raw = match c_str(path) {
        Ok(v) => v,
        Err(mut e) => {
            e.operation = "exists".to_string();
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
                    operation: "exists".to_string(),
                    path: path_raw,
                },
            );
            return 2;
        }
    };
    let native = (*fs).native.clone();
    let op = native.op.clone();
    let runtime = native.runtime.clone();
    let userdata_addr = userdata as usize;
    let handle = runtime.spawn(async move {
        let result = match op.exists(&path).await {
            Ok(value) => COutcome::Bool(value),
            Err(e) => COutcome::Error(c_error_from_opendal(e, "exists", &path)),
        };
        if let Some(cb) = callback {
            cb(ptr::null_mut(), userdata_addr as *mut c_void);
        }
        result
    });
    submit_handle(runtime, handle, out);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_ls_aio(
    fs: *mut ropendal_fs,
    opts: *const ropendal_ls_options,
    out: *mut *mut ropendal_aio,
    err: *mut *mut ropendal_error,
) -> i32 {
    if fs.is_null() || opts.is_null() || out.is_null() {
        return invalid_ptr_error(err, "ls");
    }
    let opt = &*opts;
    if opt.versions != 0 || opt.deleted != 0 {
        return unsupported_option(err, "ls", "", "versions/deleted listing is not supported");
    }
    let path_raw = if opt.path.is_null() {
        String::new()
    } else {
        match c_str(opt.path) {
            Ok(v) => v,
            Err(mut e) => {
                e.operation = "ls".to_string();
                set_c_error(err, e);
                return 2;
            }
        }
    };
    let path = match normalize_user_path(&path_raw, true) {
        Ok(p) => p,
        Err(msg) => {
            set_c_error(
                err,
                CErrorInfo {
                    status: 2,
                    kind: "InvalidArgument".to_string(),
                    message: msg,
                    operation: "ls".to_string(),
                    path: path_raw,
                },
            );
            return 2;
        }
    };
    let start_after = if opt.start_after.is_null() {
        None
    } else {
        match c_str(opt.start_after) {
            Ok(v) => Some(v),
            Err(mut e) => {
                e.operation = "ls".to_string();
                set_c_error(err, e);
                return 2;
            }
        }
    };
    let native = (*fs).native.clone();
    let op = native.op.clone();
    let runtime = native.runtime.clone();
    let recursive = opt.recursive != 0;
    let limit = opt.limit;
    let callback = opt.callback;
    let userdata_addr = opt.userdata as usize;
    let handle = runtime.spawn(async move {
        let mut list_opts = ListOptions::default();
        list_opts.recursive = recursive;
        let result = match op.list_options(&path, list_opts).await {
            Ok(entries) => {
                let mut values = entries
                    .iter()
                    .filter(|entry| entry.path() != "/" && !entry.path().is_empty())
                    .filter(|entry| {
                        start_after
                            .as_ref()
                            .is_none_or(|start_after| entry.path() > start_after.as_str())
                    })
                    .map(|entry| (entry.path().to_string(), entry.metadata().clone()))
                    .collect::<Vec<_>>();
                if limit > 0 && values.len() > limit {
                    values.truncate(limit);
                }
                COutcome::Entries(CEntrySet::from_entries(values))
            }
            Err(e) => COutcome::Error(c_error_from_opendal(e, "ls", &path)),
        };
        if let Some(cb) = callback {
            cb(ptr::null_mut(), userdata_addr as *mut c_void);
        }
        result
    });
    submit_handle(runtime, handle, out);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_delete_aio(
    fs: *mut ropendal_fs,
    opts: *const ropendal_delete_options,
    out: *mut *mut ropendal_aio,
    err: *mut *mut ropendal_error,
) -> i32 {
    if fs.is_null() || opts.is_null() || out.is_null() {
        return invalid_ptr_error(err, "delete");
    }
    let opt = &*opts;
    let path_raw = match c_str(opt.path) {
        Ok(v) => v,
        Err(mut e) => {
            e.operation = "delete".to_string();
            set_c_error(err, e);
            return 2;
        }
    };
    if !opt.version.is_null() {
        return unsupported_option(
            err,
            "delete",
            &path_raw,
            "versioned delete is not supported",
        );
    }
    let path = match normalize_user_path(&path_raw, false) {
        Ok(p) => p,
        Err(msg) => {
            set_c_error(
                err,
                CErrorInfo {
                    status: 2,
                    kind: "InvalidArgument".to_string(),
                    message: msg,
                    operation: "delete".to_string(),
                    path: path_raw,
                },
            );
            return 2;
        }
    };
    let native = (*fs).native.clone();
    let op = native.op.clone();
    let runtime = native.runtime.clone();
    let recursive = opt.recursive != 0;
    let callback = opt.callback;
    let userdata_addr = opt.userdata as usize;
    let handle = runtime.spawn(async move {
        let mut opts = DeleteOptions::default();
        opts.recursive = recursive;
        let result = match op.delete_options(&path, opts).await {
            Ok(_) => COutcome::Unit,
            Err(e) => COutcome::Error(c_error_from_opendal(e, "delete", &path)),
        };
        if let Some(cb) = callback {
            cb(ptr::null_mut(), userdata_addr as *mut c_void);
        }
        result
    });
    submit_handle(runtime, handle, out);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_copy_aio(
    fs: *mut ropendal_fs,
    from: *const std::os::raw::c_char,
    to: *const std::os::raw::c_char,
    callback: AioCallback,
    userdata: *mut c_void,
    out: *mut *mut ropendal_aio,
    err: *mut *mut ropendal_error,
) -> i32 {
    unsafe {
        if fs.is_null() || from.is_null() || to.is_null() || out.is_null() {
            return invalid_ptr_error(err, "copy");
        }
        let from_raw = match c_str(from) {
            Ok(v) => v,
            Err(mut e) => {
                e.operation = "copy".to_string();
                set_c_error(err, e);
                return 2;
            }
        };
        let to_raw = match c_str(to) {
            Ok(v) => v,
            Err(mut e) => {
                e.operation = "copy".to_string();
                set_c_error(err, e);
                return 2;
            }
        };
        let from_path = match normalize_user_path(&from_raw, false) {
            Ok(p) => p,
            Err(msg) => {
                set_c_error(
                    err,
                    CErrorInfo {
                        status: 2,
                        kind: "InvalidArgument".to_string(),
                        message: msg,
                        operation: "copy".to_string(),
                        path: from_raw,
                    },
                );
                return 2;
            }
        };
        let to_path = match normalize_user_path(&to_raw, false) {
            Ok(p) => p,
            Err(msg) => {
                set_c_error(
                    err,
                    CErrorInfo {
                        status: 2,
                        kind: "InvalidArgument".to_string(),
                        message: msg,
                        operation: "copy".to_string(),
                        path: to_raw,
                    },
                );
                return 2;
            }
        };
        let native = (*fs).native.clone();
        let op = native.op.clone();
        let runtime = native.runtime.clone();
        let userdata_addr = userdata as usize;
        let handle = runtime.spawn(async move {
            let result = match op.copy(&from_path, &to_path).await {
                Ok(_) => COutcome::Unit,
                Err(e) => COutcome::Error(c_error_from_opendal(e, "copy", &from_path)),
            };
            if let Some(cb) = callback {
                cb(ptr::null_mut(), userdata_addr as *mut c_void);
            }
            result
        });
        submit_handle(runtime, handle, out);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_rename_aio(
    fs: *mut ropendal_fs,
    from: *const std::os::raw::c_char,
    to: *const std::os::raw::c_char,
    callback: AioCallback,
    userdata: *mut c_void,
    out: *mut *mut ropendal_aio,
    err: *mut *mut ropendal_error,
) -> i32 {
    unsafe {
        if fs.is_null() || from.is_null() || to.is_null() || out.is_null() {
            return invalid_ptr_error(err, "rename");
        }
        let from_raw = match c_str(from) {
            Ok(v) => v,
            Err(mut e) => {
                e.operation = "rename".to_string();
                set_c_error(err, e);
                return 2;
            }
        };
        let to_raw = match c_str(to) {
            Ok(v) => v,
            Err(mut e) => {
                e.operation = "rename".to_string();
                set_c_error(err, e);
                return 2;
            }
        };
        let from_path = match normalize_user_path(&from_raw, false) {
            Ok(p) => p,
            Err(msg) => {
                set_c_error(
                    err,
                    CErrorInfo {
                        status: 2,
                        kind: "InvalidArgument".to_string(),
                        message: msg,
                        operation: "rename".to_string(),
                        path: from_raw,
                    },
                );
                return 2;
            }
        };
        let to_path = match normalize_user_path(&to_raw, false) {
            Ok(p) => p,
            Err(msg) => {
                set_c_error(
                    err,
                    CErrorInfo {
                        status: 2,
                        kind: "InvalidArgument".to_string(),
                        message: msg,
                        operation: "rename".to_string(),
                        path: to_raw,
                    },
                );
                return 2;
            }
        };
        let native = (*fs).native.clone();
        let op = native.op.clone();
        let runtime = native.runtime.clone();
        let userdata_addr = userdata as usize;
        let handle = runtime.spawn(async move {
            let result = match op.rename(&from_path, &to_path).await {
                Ok(_) => COutcome::Unit,
                Err(e) => COutcome::Error(c_error_from_opendal(e, "rename", &from_path)),
            };
            if let Some(cb) = callback {
                cb(ptr::null_mut(), userdata_addr as *mut c_void);
            }
            result
        });
        submit_handle(runtime, handle, out);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_mkdir_aio(
    fs: *mut ropendal_fs,
    path: *const std::os::raw::c_char,
    callback: AioCallback,
    userdata: *mut c_void,
    out: *mut *mut ropendal_aio,
    err: *mut *mut ropendal_error,
) -> i32 {
    submit_unit_path_op(
        fs,
        path,
        callback,
        userdata,
        out,
        err,
        "mkdir",
        true,
        |op, path| async move { op.create_dir(&path).await },
    )
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
