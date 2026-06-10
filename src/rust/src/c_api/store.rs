use std::os::raw::c_void;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

use opendal::options::{ListOptions, ReadOptions};
use opendal::{Buffer, Operator};

use crate::ops::{ReadTuning, WriteTuning, write_bytes_with};
use crate::path::normalize_user_path;
use crate::r_values::copy_buffer_to_slice;

use super::{
    CEntrySet, CErrorInfo, COutcome, c_error_from_opendal, c_str, ropendal_aio, ropendal_error,
    ropendal_fs, ropendal_store, ropendal_store_delete_options, ropendal_store_ls_options,
    ropendal_store_options, ropendal_store_read_options, ropendal_store_write_options, set_c_error,
};

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

fn submit_handle(
    runtime: std::sync::Arc<tokio::runtime::Runtime>,
    handle: tokio::task::JoinHandle<COutcome>,
    out: *mut *mut ropendal_aio,
) {
    unsafe {
        *out = Box::into_raw(Box::new(ropendal_aio {
            refs: AtomicUsize::new(1),
            runtime,
            handle: std::sync::Mutex::new(Some(handle)),
            cached: std::sync::Mutex::new(None),
        }));
    }
}

fn c_optional_usize(value: usize) -> Option<usize> {
    // Public C option structs are documented as zero-initializable: 0 means unset,
    // unlike R-facing arguments where explicit zero can be an error.
    if value == 0 { None } else { Some(value) }
}

fn read_tuning_from_store_options(opt: &ropendal_store_read_options) -> ReadTuning {
    ReadTuning {
        read_concurrency: c_optional_usize(opt.part_concurrency),
        chunk_size: c_optional_usize(opt.chunk_size),
        coalesce_gap: c_optional_usize(opt.coalesce_gap),
    }
}

fn write_tuning_from_store_options(opt: &ropendal_store_write_options) -> WriteTuning {
    WriteTuning {
        write_concurrency: c_optional_usize(opt.part_concurrency),
        chunk_size: c_optional_usize(opt.chunk_size),
    }
}

fn join_store_key(
    prefix: &str,
    key: &str,
    directory: bool,
    allow_empty: bool,
) -> Result<String, String> {
    if !allow_empty && key.is_empty() {
        return Err("key must not be empty".to_string());
    }
    if !directory && key.ends_with('/') {
        return Err("key must be an object key, not a directory".to_string());
    }
    let normalized = normalize_user_path(key, directory)?;
    if !allow_empty && normalized.is_empty() {
        return Err("key must not be empty".to_string());
    }
    if !directory && normalized.ends_with('/') {
        return Err("key must be an object key, not a directory".to_string());
    }
    Ok(format!("{prefix}{normalized}"))
}

fn strip_store_prefix(prefix: &str, path: &str) -> Option<String> {
    let rel = if prefix.is_empty() {
        path.to_string()
    } else {
        path.strip_prefix(prefix)?.to_string()
    };
    if rel.is_empty() { None } else { Some(rel) }
}

async fn store_read_bytes_with(
    op: Operator,
    path: String,
    offset: u64,
    size: Option<u64>,
    tuning: ReadTuning,
) -> Result<Buffer, opendal::Error> {
    let mut opts = ReadOptions::default();
    if let Some(n) = size {
        opts.range = (offset..offset.saturating_add(n)).into();
    } else if offset != 0 {
        opts.range = (offset..).into();
    }
    if let Some(concurrent) = tuning.read_concurrency {
        opts.concurrent = concurrent;
    }
    if let Some(chunk_size) = tuning.chunk_size {
        opts.chunk = Some(chunk_size);
    }
    if let Some(gap) = tuning.coalesce_gap {
        opts.gap = Some(gap);
    }
    op.read_options(&path, opts).await
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_store_open(
    fs: *mut ropendal_fs,
    opts: *const ropendal_store_options,
    out: *mut *mut ropendal_store,
    err: *mut *mut ropendal_error,
) -> i32 {
    if fs.is_null() || out.is_null() {
        return invalid_ptr_error(err, "store_open");
    }
    let prefix_raw = if opts.is_null() || (*opts).prefix.is_null() {
        String::new()
    } else {
        match c_str((*opts).prefix) {
            Ok(v) => v,
            Err(mut e) => {
                e.operation = "store_open".to_string();
                set_c_error(err, e);
                return 2;
            }
        }
    };
    let prefix = if prefix_raw.is_empty() {
        String::new()
    } else {
        match normalize_user_path(&prefix_raw, true) {
            Ok(v) => v,
            Err(msg) => {
                set_c_error(
                    err,
                    CErrorInfo {
                        status: 2,
                        kind: "InvalidArgument".to_string(),
                        message: msg,
                        operation: "store_open".to_string(),
                        path: prefix_raw,
                    },
                );
                return 2;
            }
        }
    };
    *out = Box::into_raw(Box::new(ropendal_store {
        refs: AtomicUsize::new(1),
        native: (*fs).native.clone(),
        prefix,
    }));
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_store_retain(store: *mut ropendal_store) {
    if !store.is_null() {
        (*store).refs.fetch_add(1, Ordering::Relaxed);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_store_release(store: *mut ropendal_store) {
    if !store.is_null() && (*store).refs.fetch_sub(1, Ordering::Release) == 1 {
        std::sync::atomic::fence(Ordering::Acquire);
        drop(Box::from_raw(store));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_store_read_aio(
    store: *mut ropendal_store,
    opts: *const ropendal_store_read_options,
    out: *mut *mut ropendal_aio,
    err: *mut *mut ropendal_error,
) -> i32 {
    if store.is_null() || opts.is_null() || out.is_null() {
        return invalid_ptr_error(err, "store_read");
    }
    let opt = &*opts;
    let key_raw = match c_str(opt.key) {
        Ok(v) => v,
        Err(mut e) => {
            e.operation = "store_read".to_string();
            set_c_error(err, e);
            return 2;
        }
    };
    let path = match join_store_key(&(*store).prefix, &key_raw, false, false) {
        Ok(p) => p,
        Err(msg) => {
            set_c_error(
                err,
                CErrorInfo {
                    status: 2,
                    kind: "InvalidArgument".to_string(),
                    message: msg,
                    operation: "store_read".to_string(),
                    path: key_raw,
                },
            );
            return 2;
        }
    };
    let native = (*store).native.clone();
    let op = native.op.clone();
    let runtime = native.runtime.clone();
    let offset = if opt.has_offset != 0 { opt.offset } else { 0 };
    let size = if opt.has_size != 0 {
        Some(opt.size)
    } else {
        None
    };
    let tuning = read_tuning_from_store_options(opt);
    let callback = opt.callback;
    let userdata_addr = opt.userdata as usize;
    let handle = runtime.spawn(async move {
        let result = match store_read_bytes_with(op, path.clone(), offset, size, tuning).await {
            Ok(bytes) => COutcome::Bytes(bytes.to_vec()),
            Err(e) => COutcome::Error(c_error_from_opendal(e, "store_read", &path)),
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
pub unsafe extern "C" fn ropendal_store_read_into_aio(
    store: *mut ropendal_store,
    opts: *const ropendal_store_read_options,
    dst: *mut u8,
    dst_len: usize,
    out: *mut *mut ropendal_aio,
    err: *mut *mut ropendal_error,
) -> i32 {
    if store.is_null() || opts.is_null() || out.is_null() || (dst.is_null() && dst_len > 0) {
        return invalid_ptr_error(err, "store_read_into");
    }
    let opt = &*opts;
    let key_raw = match c_str(opt.key) {
        Ok(v) => v,
        Err(mut e) => {
            e.operation = "store_read_into".to_string();
            set_c_error(err, e);
            return 2;
        }
    };
    let path = match join_store_key(&(*store).prefix, &key_raw, false, false) {
        Ok(p) => p,
        Err(msg) => {
            set_c_error(
                err,
                CErrorInfo {
                    status: 2,
                    kind: "InvalidArgument".to_string(),
                    message: msg,
                    operation: "store_read_into".to_string(),
                    path: key_raw,
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
                operation: "store_read_into".to_string(),
                path,
            },
        );
        return 2;
    }
    let native = (*store).native.clone();
    let op = native.op.clone();
    let runtime = native.runtime.clone();
    let offset = if opt.has_offset != 0 { opt.offset } else { 0 };
    let size = if opt.has_size != 0 {
        Some(opt.size)
    } else {
        None
    };
    let tuning = read_tuning_from_store_options(opt);
    let dst_addr = dst as usize;
    let callback = opt.callback;
    let userdata_addr = opt.userdata as usize;
    let handle = runtime.spawn(async move {
        let result = match store_read_bytes_with(op, path.clone(), offset, size, tuning).await {
            Ok(bytes) => {
                if bytes.len() > dst_len {
                    COutcome::Error(CErrorInfo {
                        status: 2,
                        kind: "InvalidArgument".to_string(),
                        message: "destination buffer is smaller than result".to_string(),
                        operation: "store_read_into".to_string(),
                        path,
                    })
                } else {
                    let n = bytes.len();
                    if n > 0 {
                        unsafe {
                            let dst = std::slice::from_raw_parts_mut(dst_addr as *mut u8, dst_len);
                            copy_buffer_to_slice(bytes, &mut dst[..n]);
                        }
                    }
                    COutcome::Nread(n)
                }
            }
            Err(e) => COutcome::Error(c_error_from_opendal(e, "store_read_into", &path)),
        };
        if let Some(cb) = callback {
            cb(ptr::null_mut(), userdata_addr as *mut c_void);
        }
        result
    });
    submit_handle(runtime, handle, out);
    0
}

fn submit_store_write(
    store: *mut ropendal_store,
    opts: *const ropendal_store_write_options,
    src: *const u8,
    src_len: usize,
    out: *mut *mut ropendal_aio,
    err: *mut *mut ropendal_error,
    create_only: bool,
    operation: &str,
) -> i32 {
    unsafe {
        if store.is_null() || opts.is_null() || out.is_null() || (src.is_null() && src_len > 0) {
            return invalid_ptr_error(err, operation);
        }
        let opt = &*opts;
        let key_raw = match c_str(opt.key) {
            Ok(v) => v,
            Err(mut e) => {
                e.operation = operation.to_string();
                set_c_error(err, e);
                return 2;
            }
        };
        let path = match join_store_key(&(*store).prefix, &key_raw, false, false) {
            Ok(p) => p,
            Err(msg) => {
                set_c_error(
                    err,
                    CErrorInfo {
                        status: 2,
                        kind: "InvalidArgument".to_string(),
                        message: msg,
                        operation: operation.to_string(),
                        path: key_raw,
                    },
                );
                return 2;
            }
        };
        let bytes: Buffer = if src_len == 0 {
            Buffer::new()
        } else {
            std::slice::from_raw_parts(src, src_len).to_vec().into()
        };
        let tuning = write_tuning_from_store_options(opt);
        let native = (*store).native.clone();
        let op = native.op.clone();
        let runtime = native.runtime.clone();
        let operation_owned = operation.to_string();
        let callback = opt.callback;
        let userdata_addr = opt.userdata as usize;
        let handle = runtime.spawn(async move {
            let result =
                match write_bytes_with(op, path.clone(), bytes, create_only, false, tuning).await {
                    Ok(_) => COutcome::Unit,
                    Err(e) => COutcome::Error(c_error_from_opendal(e, &operation_owned, &path)),
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
pub unsafe extern "C" fn ropendal_store_write_aio(
    store: *mut ropendal_store,
    opts: *const ropendal_store_write_options,
    src: *const u8,
    src_len: usize,
    out: *mut *mut ropendal_aio,
    err: *mut *mut ropendal_error,
) -> i32 {
    submit_store_write(store, opts, src, src_len, out, err, true, "store_write")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_store_replace_aio(
    store: *mut ropendal_store,
    opts: *const ropendal_store_write_options,
    src: *const u8,
    src_len: usize,
    out: *mut *mut ropendal_aio,
    err: *mut *mut ropendal_error,
) -> i32 {
    submit_store_write(store, opts, src, src_len, out, err, false, "store_replace")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_store_exists_aio(
    store: *mut ropendal_store,
    key: *const std::os::raw::c_char,
    callback: super::AioCallback,
    userdata: *mut c_void,
    out: *mut *mut ropendal_aio,
    err: *mut *mut ropendal_error,
) -> i32 {
    if store.is_null() || out.is_null() {
        return invalid_ptr_error(err, "store_exists");
    }
    let key_raw = match c_str(key) {
        Ok(v) => v,
        Err(mut e) => {
            e.operation = "store_exists".to_string();
            set_c_error(err, e);
            return 2;
        }
    };
    let path = match join_store_key(&(*store).prefix, &key_raw, false, false) {
        Ok(p) => p,
        Err(msg) => {
            set_c_error(
                err,
                CErrorInfo {
                    status: 2,
                    kind: "InvalidArgument".to_string(),
                    message: msg,
                    operation: "store_exists".to_string(),
                    path: key_raw,
                },
            );
            return 2;
        }
    };
    let native = (*store).native.clone();
    let op = native.op.clone();
    let runtime = native.runtime.clone();
    let userdata_addr = userdata as usize;
    let handle = runtime.spawn(async move {
        let result = match op.exists(&path).await {
            Ok(v) => COutcome::Bool(v),
            Err(e) => COutcome::Error(c_error_from_opendal(e, "store_exists", &path)),
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
pub unsafe extern "C" fn ropendal_store_ls_aio(
    store: *mut ropendal_store,
    opts: *const ropendal_store_ls_options,
    out: *mut *mut ropendal_aio,
    err: *mut *mut ropendal_error,
) -> i32 {
    if store.is_null() || opts.is_null() || out.is_null() {
        return invalid_ptr_error(err, "store_ls");
    }
    let opt = &*opts;
    let path_raw = match c_str(opt.path) {
        Ok(v) => v,
        Err(mut e) => {
            e.operation = "store_ls".to_string();
            set_c_error(err, e);
            return 2;
        }
    };
    let path = match join_store_key(&(*store).prefix, &path_raw, true, true) {
        Ok(p) => p,
        Err(msg) => {
            set_c_error(
                err,
                CErrorInfo {
                    status: 2,
                    kind: "InvalidArgument".to_string(),
                    message: msg,
                    operation: "store_ls".to_string(),
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
            Ok(v) => match join_store_key(&(*store).prefix, &v, false, false) {
                Ok(p) => Some(p),
                Err(msg) => {
                    set_c_error(
                        err,
                        CErrorInfo {
                            status: 2,
                            kind: "InvalidArgument".to_string(),
                            message: msg,
                            operation: "store_ls".to_string(),
                            path: v,
                        },
                    );
                    return 2;
                }
            },
            Err(mut e) => {
                e.operation = "store_ls".to_string();
                set_c_error(err, e);
                return 2;
            }
        }
    };
    let native = (*store).native.clone();
    let op = native.op.clone();
    let runtime = native.runtime.clone();
    let prefix = (*store).prefix.clone();
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
                    .filter_map(|entry| {
                        strip_store_prefix(&prefix, entry.path())
                            .map(|path| (path, entry.metadata().clone()))
                    })
                    .filter(|(path, _)| {
                        start_after.as_ref().is_none_or(|marker| {
                            let marker_rel = strip_store_prefix(&prefix, marker)
                                .unwrap_or_else(|| marker.clone());
                            path > &marker_rel
                        })
                    })
                    .collect::<Vec<_>>();
                if limit > 0 && values.len() > limit {
                    values.truncate(limit);
                }
                COutcome::Entries(CEntrySet::from_entries(values))
            }
            Err(e) => COutcome::Error(c_error_from_opendal(e, "store_ls", &path)),
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
pub unsafe extern "C" fn ropendal_store_delete_aio(
    store: *mut ropendal_store,
    opts: *const ropendal_store_delete_options,
    out: *mut *mut ropendal_aio,
    err: *mut *mut ropendal_error,
) -> i32 {
    if store.is_null() || opts.is_null() || out.is_null() {
        return invalid_ptr_error(err, "store_delete");
    }
    let opt = &*opts;
    let key_raw = match c_str(opt.key) {
        Ok(v) => v,
        Err(mut e) => {
            e.operation = "store_delete".to_string();
            set_c_error(err, e);
            return 2;
        }
    };
    let path = match join_store_key(&(*store).prefix, &key_raw, opt.recursive != 0, false) {
        Ok(p) => p,
        Err(msg) => {
            set_c_error(
                err,
                CErrorInfo {
                    status: 2,
                    kind: "InvalidArgument".to_string(),
                    message: msg,
                    operation: "store_delete".to_string(),
                    path: key_raw,
                },
            );
            return 2;
        }
    };
    let native = (*store).native.clone();
    let op = native.op.clone();
    let runtime = native.runtime.clone();
    let recursive = opt.recursive != 0;
    let callback = opt.callback;
    let userdata_addr = opt.userdata as usize;
    let handle = runtime.spawn(async move {
        let result = if recursive {
            match op.delete_with(&path).recursive(true).await {
                Ok(_) => COutcome::Unit,
                Err(e) => COutcome::Error(c_error_from_opendal(e, "store_delete", &path)),
            }
        } else {
            match op.delete(&path).await {
                Ok(_) => COutcome::Unit,
                Err(e) => COutcome::Error(c_error_from_opendal(e, "store_delete", &path)),
            }
        };
        if let Some(cb) = callback {
            cb(ptr::null_mut(), userdata_addr as *mut c_void);
        }
        result
    });
    submit_handle(runtime, handle, out);
    0
}
