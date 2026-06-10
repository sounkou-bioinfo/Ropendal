use std::os::raw::{c_char, c_void};

use futures::{StreamExt, stream};
use opendal::options::{DeleteOptions, ListOptions, ReadOptions};
use opendal::{Buffer, Operator};

use crate::ops::{ReadTuning, WriteTuning, write_bytes_with};
use crate::path::normalize_user_path;
use crate::r_values::copy_buffer_to_slice;

use super::aio::c_submit_handle;
use super::{
    AioCallback, CEntrySet, CErrorInfo, CReadvResultSet, CReadvTaskResult, c_error_from_opendal,
    c_str, set_c_error,
};
use super::{
    COutcome, ropendal_aio, ropendal_delete_options, ropendal_error, ropendal_fs,
    ropendal_ls_options, ropendal_read_into_request, ropendal_read_options, ropendal_read_request,
    ropendal_readv_options, ropendal_write_options,
};

#[derive(Clone, Default)]
struct CReadParams {
    tuning: ReadTuning,
    content_length_hint: Option<u64>,
    version: Option<String>,
    if_match: Option<String>,
    if_none_match: Option<String>,
}

struct CReadTask {
    index: usize,
    path: String,
    offset: u64,
    size: Option<u64>,
    params: CReadParams,
}

struct CReadIntoTask {
    index: usize,
    path: String,
    offset: u64,
    size: Option<u64>,
    dst_addr: usize,
    dst_len: usize,
}

fn c_optional_usize(value: usize) -> Option<usize> {
    if value == 0 { None } else { Some(value) }
}

fn c_write_tuning_from_options(opt: &ropendal_write_options) -> WriteTuning {
    WriteTuning {
        write_concurrency: c_optional_usize(opt.part_concurrency),
        chunk_size: c_optional_usize(opt.chunk_size),
    }
}

unsafe fn c_optional_string(
    value: *const c_char,
    operation: &str,
    path: &str,
) -> Result<Option<String>, CErrorInfo> {
    if value.is_null() {
        return Ok(None);
    }
    match c_str(value) {
        Ok(v) => Ok(Some(v)),
        Err(mut e) => {
            e.operation = operation.to_string();
            e.path = path.to_string();
            Err(e)
        }
    }
}

unsafe fn c_read_params_from_read_options(
    opt: &ropendal_read_options,
    operation: &str,
    path: &str,
) -> Result<CReadParams, CErrorInfo> {
    Ok(CReadParams {
        tuning: ReadTuning {
            read_concurrency: c_optional_usize(opt.part_concurrency),
            chunk_size: c_optional_usize(opt.chunk_size),
            coalesce_gap: c_optional_usize(opt.coalesce_gap),
        },
        content_length_hint: if opt.has_content_length_hint != 0 {
            Some(opt.content_length_hint)
        } else {
            None
        },
        version: c_optional_string(opt.version, operation, path)?,
        if_match: c_optional_string(opt.if_match, operation, path)?,
        if_none_match: c_optional_string(opt.if_none_match, operation, path)?,
    })
}

fn c_read_params_from_readv_options(opt: Option<&ropendal_readv_options>) -> CReadParams {
    match opt {
        Some(opt) => CReadParams {
            tuning: ReadTuning {
                read_concurrency: c_optional_usize(opt.part_concurrency),
                chunk_size: c_optional_usize(opt.chunk_size),
                coalesce_gap: c_optional_usize(opt.coalesce_gap),
            },
            ..CReadParams::default()
        },
        None => CReadParams::default(),
    }
}

async fn c_read_bytes_with(
    op: Operator,
    path: String,
    offset: u64,
    size: Option<u64>,
    params: CReadParams,
) -> Result<Buffer, opendal::Error> {
    let mut opts = ReadOptions::default();
    if let Some(n) = size {
        opts.range = (offset..offset.saturating_add(n)).into();
    } else if offset != 0 {
        opts.range = (offset..).into();
    }
    opts.content_length_hint = params.content_length_hint;
    opts.version = params.version;
    opts.if_match = params.if_match;
    opts.if_none_match = params.if_none_match;
    if let Some(concurrent) = params.tuning.read_concurrency {
        opts.concurrent = concurrent;
    }
    if let Some(chunk_size) = params.tuning.chunk_size {
        opts.chunk = Some(chunk_size);
    }
    if let Some(gap) = params.tuning.coalesce_gap {
        opts.gap = Some(gap);
    }
    op.read_options(&path, opts).await
}

async fn c_read_into_task(
    op: Operator,
    task: CReadIntoTask,
    params: CReadParams,
    operation: &'static str,
) -> Result<usize, CErrorInfo> {
    let path_for_error = task.path.clone();
    match c_read_bytes_with(op, task.path, task.offset, task.size, params).await {
        Ok(bytes) => {
            if bytes.len() > task.dst_len {
                Err(CErrorInfo {
                    status: 2,
                    kind: "InvalidArgument".to_string(),
                    message: "destination buffer is smaller than result".to_string(),
                    operation: operation.to_string(),
                    path: path_for_error,
                })
            } else {
                let n = bytes.len();
                if n > 0 {
                    unsafe {
                        let dst =
                            std::slice::from_raw_parts_mut(task.dst_addr as *mut u8, task.dst_len);
                        copy_buffer_to_slice(bytes, &mut dst[..n]);
                    }
                }
                Ok(n)
            }
        }
        Err(e) => Err(c_error_from_opendal(e, operation, &path_for_error)),
    }
}

fn c_batch_concurrency(value: usize, n: usize) -> usize {
    if n == 0 {
        1
    } else if value == 0 {
        n.min(16)
    } else {
        value.max(1).min(n)
    }
}

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
    if opt.has_size != 0 && opt.size > dst_len as u64 {
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
    let params = match c_read_params_from_read_options(opt, "read_into", &path) {
        Ok(v) => v,
        Err(e) => {
            set_c_error(err, e);
            return 2;
        }
    };
    let native = (*fs).native.clone();
    let op = native.op.clone();
    let runtime = native.runtime.clone();
    let task = CReadIntoTask {
        index: 0,
        path,
        offset: opt.offset,
        size: if opt.has_size != 0 {
            Some(opt.size)
        } else {
            None
        },
        dst_addr: dst as usize,
        dst_len,
    };
    let callback = opt.callback;
    let userdata_addr = opt.userdata as usize;
    let handle = runtime.spawn(async move {
        match c_read_into_task(op, task, params, "read_into").await {
            Ok(n) => COutcome::Nread(n),
            Err(info) => COutcome::Error(info),
        }
    });
    c_submit_handle(runtime, handle, out, callback, userdata_addr);
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
    let params = match c_read_params_from_read_options(opt, "read", &path) {
        Ok(v) => v,
        Err(e) => {
            set_c_error(err, e);
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
    let callback = opt.callback;
    let userdata_addr = opt.userdata as usize;
    let handle = runtime.spawn(async move {
        match c_read_bytes_with(op, path.clone(), offset, size, params).await {
            Ok(bytes) => COutcome::Bytes(bytes.to_vec()),
            Err(e) => COutcome::Error(c_error_from_opendal(e, "read", &path)),
        }
    });
    c_submit_handle(runtime, handle, out, callback, userdata_addr);
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
        if !opt.if_match.is_null()
            || !opt.if_none_match.is_null()
            || !opt.content_type.is_null()
            || !opt.content_encoding.is_null()
            || !opt.content_disposition.is_null()
            || !opt.cache_control.is_null()
        {
            return unsupported_option(
                err,
                operation,
                &path,
                "conditional and content-header write options are not implemented yet",
            );
        }
        let tuning = c_write_tuning_from_options(opt);
        let bytes = if src_len == 0 {
            opendal::Buffer::new()
        } else {
            std::slice::from_raw_parts(src, src_len).to_vec().into()
        };
        let native = (*fs).native.clone();
        let op = native.op.clone();
        let runtime = native.runtime.clone();
        let operation_owned = operation.to_string();
        let callback = opt.callback;
        let userdata_addr = opt.userdata as usize;
        let handle = runtime.spawn(async move {
            match write_bytes_with(op, path.clone(), bytes, create_only, append, tuning).await {
                Ok(_) => COutcome::Unit,
                Err(e) => COutcome::Error(c_error_from_opendal(e, &operation_owned, &path)),
            }
        });
        c_submit_handle(runtime, handle, out, callback, userdata_addr);
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
            match op_fn(op, path.clone()).await {
                Ok(_) => COutcome::Unit,
                Err(e) => COutcome::Error(c_error_from_opendal(e, operation, &path)),
            }
        });
        c_submit_handle(runtime, handle, out, callback, userdata_addr);
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
        match op.stat(&path).await {
            Ok(meta) => COutcome::Entry(CEntrySet::one(&path, &meta)),
            Err(e) => COutcome::Error(c_error_from_opendal(e, "stat", &path)),
        }
    });
    c_submit_handle(runtime, handle, out, callback, userdata_addr);
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
        match op.exists(&path).await {
            Ok(value) => COutcome::Bool(value),
            Err(e) => COutcome::Error(c_error_from_opendal(e, "exists", &path)),
        }
    });
    c_submit_handle(runtime, handle, out, callback, userdata_addr);
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
        match op.list_options(&path, list_opts).await {
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
        }
    });
    c_submit_handle(runtime, handle, out, callback, userdata_addr);
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
        match op.delete_options(&path, opts).await {
            Ok(_) => COutcome::Unit,
            Err(e) => COutcome::Error(c_error_from_opendal(e, "delete", &path)),
        }
    });
    c_submit_handle(runtime, handle, out, callback, userdata_addr);
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
            match op.copy(&from_path, &to_path).await {
                Ok(_) => COutcome::Unit,
                Err(e) => COutcome::Error(c_error_from_opendal(e, "copy", &from_path)),
            }
        });
        c_submit_handle(runtime, handle, out, callback, userdata_addr);
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
            match op.rename(&from_path, &to_path).await {
                Ok(_) => COutcome::Unit,
                Err(e) => COutcome::Error(c_error_from_opendal(e, "rename", &from_path)),
            }
        });
        c_submit_handle(runtime, handle, out, callback, userdata_addr);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_readv_aio(
    fs: *mut ropendal_fs,
    requests: *const ropendal_read_request,
    n_requests: usize,
    opts: *const ropendal_readv_options,
    out: *mut *mut ropendal_aio,
    err: *mut *mut ropendal_error,
) -> i32 {
    if fs.is_null() || out.is_null() || (requests.is_null() && n_requests > 0) {
        return invalid_ptr_error(err, "readv");
    }

    let opt = if opts.is_null() { None } else { Some(&*opts) };
    let base_params = c_read_params_from_readv_options(opt);
    let mut tasks = Vec::with_capacity(n_requests);
    for i in 0..n_requests {
        let req = &*requests.add(i);
        let path_raw = match c_str(req.path) {
            Ok(v) => v,
            Err(mut e) => {
                e.operation = "readv".to_string();
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
                        operation: "readv".to_string(),
                        path: path_raw,
                    },
                );
                return 2;
            }
        };
        let mut params = base_params.clone();
        params.content_length_hint = if req.has_content_length_hint != 0 {
            Some(req.content_length_hint)
        } else {
            None
        };
        params.version = match c_optional_string(req.version, "readv", &path) {
            Ok(v) => v,
            Err(e) => {
                set_c_error(err, e);
                return 2;
            }
        };
        tasks.push(CReadTask {
            index: i,
            path,
            offset: req.offset,
            size: if req.has_size != 0 {
                Some(req.size)
            } else {
                None
            },
            params,
        });
    }

    let callback = opt.map(|o| o.callback).unwrap_or(None);
    let userdata_addr = opt.map(|o| o.userdata as usize).unwrap_or(0);
    let concurrency =
        c_batch_concurrency(opt.map(|o| o.batch_concurrency).unwrap_or(0), n_requests);
    let native = (*fs).native.clone();
    let op = native.op.clone();
    let runtime = native.runtime.clone();
    let handle = runtime.spawn(async move {
        let values = stream::iter(tasks.into_iter())
            .map(|task| {
                let op = op.clone();
                async move {
                    let index = task.index;
                    let path = task.path.clone();
                    match c_read_bytes_with(op, task.path, task.offset, task.size, task.params)
                        .await
                    {
                        Ok(bytes) => {
                            let bytes = bytes.to_vec();
                            CReadvTaskResult::Ok {
                                index,
                                path,
                                nread: bytes.len(),
                                bytes: Some(bytes),
                            }
                        }
                        Err(e) => CReadvTaskResult::Error {
                            index,
                            info: c_error_from_opendal(e, "readv", &path),
                        },
                    }
                }
            })
            .buffer_unordered(concurrency)
            .collect::<Vec<_>>()
            .await;
        COutcome::Readv(CReadvResultSet::from_task_results(values))
    });
    c_submit_handle(runtime, handle, out, callback, userdata_addr);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_readv_into_aio(
    fs: *mut ropendal_fs,
    requests: *const ropendal_read_into_request,
    n_requests: usize,
    opts: *const ropendal_readv_options,
    out: *mut *mut ropendal_aio,
    err: *mut *mut ropendal_error,
) -> i32 {
    if fs.is_null() || out.is_null() || (requests.is_null() && n_requests > 0) {
        return invalid_ptr_error(err, "readv_into");
    }

    let mut tasks = Vec::with_capacity(n_requests);
    for i in 0..n_requests {
        let req = &*requests.add(i);
        let path_raw = match c_str(req.path) {
            Ok(v) => v,
            Err(mut e) => {
                e.operation = "readv_into".to_string();
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
                        operation: "readv_into".to_string(),
                        path: path_raw,
                    },
                );
                return 2;
            }
        };
        if req.dst.is_null() {
            set_c_error(
                err,
                CErrorInfo {
                    status: 2,
                    kind: "InvalidArgument".to_string(),
                    message: "destination buffer pointer is null".to_string(),
                    operation: "readv_into".to_string(),
                    path,
                },
            );
            return 2;
        }
        if req.has_size != 0 && req.size > req.dst_len as u64 {
            set_c_error(
                err,
                CErrorInfo {
                    status: 2,
                    kind: "InvalidArgument".to_string(),
                    message: "destination buffer is smaller than requested size".to_string(),
                    operation: "readv_into".to_string(),
                    path,
                },
            );
            return 2;
        }
        tasks.push(CReadIntoTask {
            index: i,
            path,
            offset: req.offset,
            size: if req.has_size != 0 {
                Some(req.size)
            } else {
                None
            },
            dst_addr: req.dst as usize,
            dst_len: req.dst_len,
        });
    }

    let opt = if opts.is_null() { None } else { Some(&*opts) };
    let params = c_read_params_from_readv_options(opt);
    let callback = opt.map(|o| o.callback).unwrap_or(None);
    let userdata_addr = opt.map(|o| o.userdata as usize).unwrap_or(0);
    let concurrency =
        c_batch_concurrency(opt.map(|o| o.batch_concurrency).unwrap_or(0), n_requests);
    let native = (*fs).native.clone();
    let op = native.op.clone();
    let runtime = native.runtime.clone();
    let handle = runtime.spawn(async move {
        let values = stream::iter(tasks.into_iter())
            .map(|task| {
                let op = op.clone();
                let params = params.clone();
                let index = task.index;
                let path = task.path.clone();
                async move {
                    match c_read_into_task(op, task, params, "readv_into").await {
                        Ok(nread) => CReadvTaskResult::Ok {
                            index,
                            path,
                            nread,
                            bytes: None,
                        },
                        Err(info) => CReadvTaskResult::Error { index, info },
                    }
                }
            })
            .buffer_unordered(concurrency)
            .collect::<Vec<_>>()
            .await;
        COutcome::Readv(CReadvResultSet::from_task_results(values))
    });
    c_submit_handle(runtime, handle, out, callback, userdata_addr);
    0
}
