use std::os::raw::{c_char, c_void};
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use opendal::options::{ListOptions, ReadOptions};
use opendal::{Buffer, Metadata, Operator};

use crate::ops::{ReadTuning, WriteTuning, write_bytes_with};
use crate::path::normalize_user_path;
use crate::r_values::copy_buffer_to_slice;

use super::{
    AioCallback, CEntrySet, CErrorInfo, COutcome, CStoreBackend, CStoreCacheValidate,
    c_error_from_opendal, c_str, ropendal_aio, ropendal_error, ropendal_fs, ropendal_store,
    ropendal_store_cache_options, ropendal_store_delete_options, ropendal_store_ls_options,
    ropendal_store_options, ropendal_store_read_options, ropendal_store_write_options, set_c_error,
};

const STORE_CACHE_VALIDATE_LAST_MODIFIED_SIZE: i32 = 0;
const STORE_CACHE_VALIDATE_NONE: i32 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
struct StoreMeta {
    size: u64,
    last_modified: Option<String>,
}

fn c_error(
    status: i32,
    kind: &str,
    message: impl Into<String>,
    operation: &str,
    path: &str,
) -> CErrorInfo {
    CErrorInfo {
        status,
        kind: kind.to_string(),
        message: message.into(),
        operation: operation.to_string(),
        path: path.to_string(),
    }
}

fn invalid_ptr_error(err: *mut *mut ropendal_error, operation: &str) -> i32 {
    set_c_error(
        err,
        c_error(
            2,
            "InvalidArgument",
            "required pointer is null",
            operation,
            "",
        ),
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

fn normalize_store_key(key: &str, directory: bool, allow_empty: bool) -> Result<String, String> {
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
    Ok(normalized)
}

fn join_store_key(
    prefix: &str,
    key: &str,
    directory: bool,
    allow_empty: bool,
) -> Result<String, String> {
    let normalized = normalize_store_key(key, directory, allow_empty)?;
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

fn parse_store_key(
    ptr: *const c_char,
    operation: &str,
    directory: bool,
    allow_empty: bool,
    err: *mut *mut ropendal_error,
) -> Result<String, i32> {
    let raw = match unsafe { c_str(ptr) } {
        Ok(v) => v,
        Err(mut e) => {
            e.operation = operation.to_string();
            set_c_error(err, e);
            return Err(2);
        }
    };
    match normalize_store_key(&raw, directory, allow_empty) {
        Ok(v) => Ok(v),
        Err(msg) => {
            set_c_error(err, c_error(2, "InvalidArgument", msg, operation, &raw));
            Err(2)
        }
    }
}

fn cache_validate_from_options(
    opts: *const ropendal_store_cache_options,
    err: *mut *mut ropendal_error,
) -> Result<CStoreCacheValidate, i32> {
    unsafe {
        if opts.is_null() {
            return Ok(CStoreCacheValidate::LastModifiedSize);
        }
        match (*opts).validate {
            STORE_CACHE_VALIDATE_LAST_MODIFIED_SIZE => Ok(CStoreCacheValidate::LastModifiedSize),
            STORE_CACHE_VALIDATE_NONE => Ok(CStoreCacheValidate::None),
            other => {
                set_c_error(
                    err,
                    c_error(
                        2,
                        "InvalidArgument",
                        format!("unknown store cache validation mode: {other}"),
                        "store_cache_open",
                        "",
                    ),
                );
                Err(2)
            }
        }
    }
}

fn backend_is_cached(backend: &Arc<CStoreBackend>) -> bool {
    matches!(backend.as_ref(), CStoreBackend::Cached { .. })
}

fn direct_parts(
    backend: &Arc<CStoreBackend>,
    operation: &str,
) -> Result<(Arc<crate::common::NativeFs>, String), CErrorInfo> {
    match backend.as_ref() {
        CStoreBackend::Direct { native, prefix } => Ok((native.clone(), prefix.clone())),
        CStoreBackend::Cached { .. } => Err(c_error(
            1,
            "Unexpected",
            "internal cached store recursion is unsupported",
            operation,
            "",
        )),
    }
}

async fn read_bytes_with_options(
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

async fn direct_read_bytes(
    backend: Arc<CStoreBackend>,
    key: &str,
    offset: u64,
    size: Option<u64>,
    tuning: ReadTuning,
    operation: &str,
) -> Result<Vec<u8>, CErrorInfo> {
    let (native, prefix) = direct_parts(&backend, operation)?;
    let path = join_store_key(&prefix, key, false, false)
        .map_err(|msg| c_error(2, "InvalidArgument", msg, operation, key))?;
    match read_bytes_with_options(native.op.clone(), path.clone(), offset, size, tuning).await {
        Ok(bytes) => Ok(bytes.to_vec()),
        Err(e) => Err(c_error_from_opendal(e, operation, &path)),
    }
}

async fn direct_write_bytes(
    backend: Arc<CStoreBackend>,
    key: &str,
    bytes: Vec<u8>,
    create_only: bool,
    tuning: WriteTuning,
    operation: &str,
) -> Result<(), CErrorInfo> {
    let (native, prefix) = direct_parts(&backend, operation)?;
    let path = join_store_key(&prefix, key, false, false)
        .map_err(|msg| c_error(2, "InvalidArgument", msg, operation, key))?;
    let buffer: Buffer = if bytes.is_empty() {
        Buffer::new()
    } else {
        bytes.into()
    };
    write_bytes_with(
        native.op.clone(),
        path.clone(),
        buffer,
        create_only,
        false,
        tuning,
    )
    .await
    .map_err(|e| c_error_from_opendal(e, operation, &path))
}

async fn direct_exists(
    backend: Arc<CStoreBackend>,
    key: &str,
    operation: &str,
) -> Result<bool, CErrorInfo> {
    let (native, prefix) = direct_parts(&backend, operation)?;
    let path = join_store_key(&prefix, key, false, false)
        .map_err(|msg| c_error(2, "InvalidArgument", msg, operation, key))?;
    native
        .op
        .exists(&path)
        .await
        .map_err(|e| c_error_from_opendal(e, operation, &path))
}

fn meta_from_opendal(meta: &Metadata) -> StoreMeta {
    StoreMeta {
        size: meta.content_length(),
        last_modified: meta.last_modified().map(|v| v.to_string()),
    }
}

async fn direct_stat(
    backend: Arc<CStoreBackend>,
    key: &str,
    operation: &str,
) -> Result<StoreMeta, CErrorInfo> {
    let (native, prefix) = direct_parts(&backend, operation)?;
    let path = join_store_key(&prefix, key, false, false)
        .map_err(|msg| c_error(2, "InvalidArgument", msg, operation, key))?;
    native
        .op
        .stat(&path)
        .await
        .map(|meta| meta_from_opendal(&meta))
        .map_err(|e| c_error_from_opendal(e, operation, &path))
}

async fn direct_delete(
    backend: Arc<CStoreBackend>,
    key: &str,
    recursive: bool,
    operation: &str,
) -> Result<(), CErrorInfo> {
    let (native, prefix) = direct_parts(&backend, operation)?;
    let path = join_store_key(&prefix, key, recursive, false)
        .map_err(|msg| c_error(2, "InvalidArgument", msg, operation, key))?;
    let result = if recursive {
        native.op.delete_with(&path).recursive(true).await
    } else {
        native.op.delete(&path).await
    };
    result.map_err(|e| c_error_from_opendal(e, operation, &path))
}

async fn direct_list(
    backend: Arc<CStoreBackend>,
    key: &str,
    recursive: bool,
    limit: usize,
    start_after: Option<String>,
    operation: &str,
) -> Result<CEntrySet, CErrorInfo> {
    let (native, prefix) = direct_parts(&backend, operation)?;
    let path = join_store_key(&prefix, key, true, true)
        .map_err(|msg| c_error(2, "InvalidArgument", msg, operation, key))?;
    let start_after_rel = start_after;
    let mut list_opts = ListOptions::default();
    list_opts.recursive = recursive;
    match native.op.list_options(&path, list_opts).await {
        Ok(entries) => {
            let mut values = entries
                .iter()
                .filter_map(|entry| {
                    strip_store_prefix(&prefix, entry.path())
                        .map(|path| (path, entry.metadata().clone()))
                })
                .filter(|(path, _)| start_after_rel.as_ref().is_none_or(|marker| path > marker))
                .collect::<Vec<_>>();
            if limit > 0 && values.len() > limit {
                values.truncate(limit);
            }
            Ok(CEntrySet::from_entries(values))
        }
        Err(e) => Err(c_error_from_opendal(e, operation, &path)),
    }
}

async fn store_exists_backend(
    backend: Arc<CStoreBackend>,
    key: &str,
    operation: &str,
) -> Result<bool, CErrorInfo> {
    match backend.as_ref() {
        CStoreBackend::Direct { .. } => direct_exists(backend, key, operation).await,
        CStoreBackend::Cached { parent, .. } => direct_exists(parent.clone(), key, operation).await,
    }
}

async fn store_list_backend(
    backend: Arc<CStoreBackend>,
    key: &str,
    recursive: bool,
    limit: usize,
    start_after: Option<String>,
    operation: &str,
) -> Result<CEntrySet, CErrorInfo> {
    match backend.as_ref() {
        CStoreBackend::Direct { .. } => {
            direct_list(backend, key, recursive, limit, start_after, operation).await
        }
        CStoreBackend::Cached { parent, .. } => {
            direct_list(
                parent.clone(),
                key,
                recursive,
                limit,
                start_after,
                operation,
            )
            .await
        }
    }
}

fn cache_object_key(key: &str) -> String {
    format!("objects/{}", hex_key(key))
}

fn cache_meta_key(key: &str) -> String {
    format!("meta/{}.txt", hex_key(key))
}

fn hex_key(key: &str) -> String {
    key.as_bytes().iter().map(|b| format!("{b:02x}")).collect()
}

fn encode_meta(meta: &StoreMeta) -> Vec<u8> {
    format!(
        "ropendal-c-store-cache-v1\n{}\n{}\n",
        meta.size,
        meta.last_modified.as_deref().unwrap_or("")
    )
    .into_bytes()
}

fn decode_meta(bytes: &[u8]) -> Option<StoreMeta> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut lines = text.lines();
    if lines.next()? != "ropendal-c-store-cache-v1" {
        return None;
    }
    let size = lines.next()?.parse::<u64>().ok()?;
    let last_modified = match lines.next() {
        Some("") | None => None,
        Some(v) => Some(v.to_string()),
    };
    Some(StoreMeta {
        size,
        last_modified,
    })
}

async fn cache_entry_valid(
    parent: Arc<CStoreBackend>,
    cache: Arc<CStoreBackend>,
    key: &str,
    cache_key: &str,
    meta_key: &str,
    validate: CStoreCacheValidate,
) -> bool {
    match direct_exists(cache.clone(), cache_key, "store_cache_exists").await {
        Ok(true) => {}
        _ => return false,
    }
    if validate == CStoreCacheValidate::None {
        return true;
    }
    let current = match direct_stat(parent, key, "store_cache_stat").await {
        Ok(v) => v,
        Err(_) => return false,
    };
    let cached_meta = match direct_read_bytes(
        cache,
        meta_key,
        0,
        None,
        ReadTuning::default(),
        "store_cache_meta",
    )
    .await
    {
        Ok(v) => v,
        Err(_) => return false,
    };
    decode_meta(&cached_meta).is_some_and(|cached| cached == current)
}

async fn fill_cache(
    parent: Arc<CStoreBackend>,
    cache: Arc<CStoreBackend>,
    key: &str,
    bytes: &[u8],
) {
    let cache_key = cache_object_key(key);
    let meta_key = cache_meta_key(key);
    let _ = direct_write_bytes(
        cache.clone(),
        &cache_key,
        bytes.to_vec(),
        false,
        WriteTuning::default(),
        "store_cache_fill",
    )
    .await;
    if let Ok(meta) = direct_stat(parent, key, "store_cache_stat").await {
        let _ = direct_write_bytes(
            cache,
            &meta_key,
            encode_meta(&meta),
            false,
            WriteTuning::default(),
            "store_cache_meta",
        )
        .await;
    }
}

async fn invalidate_cache_key(cache: Arc<CStoreBackend>, key: &str) {
    let cache_key = cache_object_key(key);
    let meta_key = cache_meta_key(key);
    let _ = direct_delete(cache.clone(), &cache_key, false, "store_cache_invalidate").await;
    let _ = direct_delete(cache, &meta_key, false, "store_cache_invalidate").await;
}

async fn clear_cache(cache: Arc<CStoreBackend>) {
    let _ = direct_delete(cache.clone(), "objects/", true, "store_cache_clear").await;
    let _ = direct_delete(cache, "meta/", true, "store_cache_clear").await;
}

async fn store_read_backend(
    backend: Arc<CStoreBackend>,
    key: &str,
    offset: u64,
    size: Option<u64>,
    tuning: ReadTuning,
    operation: &str,
) -> Result<Vec<u8>, CErrorInfo> {
    match backend.as_ref() {
        CStoreBackend::Direct { .. } => {
            direct_read_bytes(backend, key, offset, size, tuning, operation).await
        }
        CStoreBackend::Cached {
            parent,
            cache,
            validate,
        } => {
            if offset != 0 || size.is_some() {
                return direct_read_bytes(parent.clone(), key, offset, size, tuning, operation)
                    .await;
            }
            let cache_key = cache_object_key(key);
            let meta_key = cache_meta_key(key);
            if cache_entry_valid(
                parent.clone(),
                cache.clone(),
                key,
                &cache_key,
                &meta_key,
                *validate,
            )
            .await
            {
                return direct_read_bytes(cache.clone(), &cache_key, 0, None, tuning, operation)
                    .await;
            }
            let bytes = direct_read_bytes(parent.clone(), key, 0, None, tuning, operation).await?;
            fill_cache(parent.clone(), cache.clone(), key, &bytes).await;
            Ok(bytes)
        }
    }
}

async fn store_write_backend(
    backend: Arc<CStoreBackend>,
    key: &str,
    bytes: Vec<u8>,
    create_only: bool,
    tuning: WriteTuning,
    operation: &str,
) -> Result<(), CErrorInfo> {
    match backend.as_ref() {
        CStoreBackend::Direct { .. } => {
            direct_write_bytes(backend, key, bytes, create_only, tuning, operation).await
        }
        CStoreBackend::Cached { parent, cache, .. } => {
            direct_write_bytes(parent.clone(), key, bytes, create_only, tuning, operation).await?;
            invalidate_cache_key(cache.clone(), key).await;
            Ok(())
        }
    }
}

async fn store_delete_backend(
    backend: Arc<CStoreBackend>,
    key: &str,
    recursive: bool,
    operation: &str,
) -> Result<(), CErrorInfo> {
    match backend.as_ref() {
        CStoreBackend::Direct { .. } => direct_delete(backend, key, recursive, operation).await,
        CStoreBackend::Cached { parent, cache, .. } => {
            direct_delete(parent.clone(), key, recursive, operation).await?;
            if recursive {
                clear_cache(cache.clone()).await;
            } else {
                invalidate_cache_key(cache.clone(), key).await;
            }
            Ok(())
        }
    }
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
                    c_error(2, "InvalidArgument", msg, "store_open", &prefix_raw),
                );
                return 2;
            }
        }
    };
    *out = Box::into_raw(Box::new(ropendal_store {
        refs: AtomicUsize::new(1),
        backend: Arc::new(CStoreBackend::Direct {
            native: (*fs).native.clone(),
            prefix,
        }),
    }));
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_store_cache_open(
    parent: *mut ropendal_store,
    cache: *mut ropendal_store,
    opts: *const ropendal_store_cache_options,
    out: *mut *mut ropendal_store,
    err: *mut *mut ropendal_error,
) -> i32 {
    if parent.is_null() || cache.is_null() || out.is_null() {
        return invalid_ptr_error(err, "store_cache_open");
    }
    if backend_is_cached(&(*parent).backend) || backend_is_cached(&(*cache).backend) {
        set_c_error(
            err,
            c_error(
                2,
                "InvalidArgument",
                "store_cache_open expects uncached parent and cache stores",
                "store_cache_open",
                "",
            ),
        );
        return 2;
    }
    let validate = match cache_validate_from_options(opts, err) {
        Ok(v) => v,
        Err(code) => return code,
    };
    *out = Box::into_raw(Box::new(ropendal_store {
        refs: AtomicUsize::new(1),
        backend: Arc::new(CStoreBackend::Cached {
            parent: (*parent).backend.clone(),
            cache: (*cache).backend.clone(),
            validate,
        }),
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
    let key = match parse_store_key(opt.key, "store_read", false, false, err) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let native = match (*store).backend.as_ref() {
        CStoreBackend::Direct { native, .. } => native.clone(),
        CStoreBackend::Cached { parent, .. } => match parent.as_ref() {
            CStoreBackend::Direct { native, .. } => native.clone(),
            CStoreBackend::Cached { .. } => {
                return invalid_ptr_error(err, "store_read");
            }
        },
    };
    let runtime = native.runtime.clone();
    let backend = (*store).backend.clone();
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
        let result =
            match store_read_backend(backend, &key, offset, size, tuning, "store_read").await {
                Ok(bytes) => COutcome::Bytes(bytes),
                Err(info) => COutcome::Error(info),
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
    let key = match parse_store_key(opt.key, "store_read_into", false, false, err) {
        Ok(v) => v,
        Err(code) => return code,
    };
    if opt.has_size != 0 && opt.size > dst_len as u64 {
        set_c_error(
            err,
            c_error(
                2,
                "InvalidArgument",
                "destination buffer is smaller than requested size",
                "store_read_into",
                &key,
            ),
        );
        return 2;
    }
    let native = match (*store).backend.as_ref() {
        CStoreBackend::Direct { native, .. } => native.clone(),
        CStoreBackend::Cached { parent, .. } => match parent.as_ref() {
            CStoreBackend::Direct { native, .. } => native.clone(),
            CStoreBackend::Cached { .. } => {
                return invalid_ptr_error(err, "store_read_into");
            }
        },
    };
    let runtime = native.runtime.clone();
    let backend = (*store).backend.clone();
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
    let handle =
        runtime.spawn(async move {
            let result =
                match store_read_backend(backend, &key, offset, size, tuning, "store_read_into")
                    .await
                {
                    Ok(bytes) => {
                        if bytes.len() > dst_len {
                            COutcome::Error(c_error(
                                2,
                                "InvalidArgument",
                                "destination buffer is smaller than result",
                                "store_read_into",
                                &key,
                            ))
                        } else {
                            let n = bytes.len();
                            if n > 0 {
                                unsafe {
                                    let dst = std::slice::from_raw_parts_mut(
                                        dst_addr as *mut u8,
                                        dst_len,
                                    );
                                    copy_buffer_to_slice(bytes.into(), &mut dst[..n]);
                                }
                            }
                            COutcome::Nread(n)
                        }
                    }
                    Err(info) => COutcome::Error(info),
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
        let key = match parse_store_key(opt.key, operation, false, false, err) {
            Ok(v) => v,
            Err(code) => return code,
        };
        let bytes = if src_len == 0 {
            Vec::new()
        } else {
            std::slice::from_raw_parts(src, src_len).to_vec()
        };
        let native = match (*store).backend.as_ref() {
            CStoreBackend::Direct { native, .. } => native.clone(),
            CStoreBackend::Cached { parent, .. } => match parent.as_ref() {
                CStoreBackend::Direct { native, .. } => native.clone(),
                CStoreBackend::Cached { .. } => return invalid_ptr_error(err, operation),
            },
        };
        let runtime = native.runtime.clone();
        let backend = (*store).backend.clone();
        let tuning = write_tuning_from_store_options(opt);
        let operation_owned = operation.to_string();
        let callback = opt.callback;
        let userdata_addr = opt.userdata as usize;
        let handle = runtime.spawn(async move {
            let result = match store_write_backend(
                backend,
                &key,
                bytes,
                create_only,
                tuning,
                &operation_owned,
            )
            .await
            {
                Ok(_) => COutcome::Unit,
                Err(info) => COutcome::Error(info),
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
    key: *const c_char,
    callback: AioCallback,
    userdata: *mut c_void,
    out: *mut *mut ropendal_aio,
    err: *mut *mut ropendal_error,
) -> i32 {
    if store.is_null() || out.is_null() {
        return invalid_ptr_error(err, "store_exists");
    }
    let key = match parse_store_key(key, "store_exists", false, false, err) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let native = match (*store).backend.as_ref() {
        CStoreBackend::Direct { native, .. } => native.clone(),
        CStoreBackend::Cached { parent, .. } => match parent.as_ref() {
            CStoreBackend::Direct { native, .. } => native.clone(),
            CStoreBackend::Cached { .. } => return invalid_ptr_error(err, "store_exists"),
        },
    };
    let runtime = native.runtime.clone();
    let backend = (*store).backend.clone();
    let userdata_addr = userdata as usize;
    let handle = runtime.spawn(async move {
        let result = match store_exists_backend(backend, &key, "store_exists").await {
            Ok(v) => COutcome::Bool(v),
            Err(info) => COutcome::Error(info),
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
    let key = match parse_store_key(opt.path, "store_ls", true, true, err) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let start_after = if opt.start_after.is_null() {
        None
    } else {
        match parse_store_key(opt.start_after, "store_ls", false, false, err) {
            Ok(v) => Some(v),
            Err(code) => return code,
        }
    };
    let native = match (*store).backend.as_ref() {
        CStoreBackend::Direct { native, .. } => native.clone(),
        CStoreBackend::Cached { parent, .. } => match parent.as_ref() {
            CStoreBackend::Direct { native, .. } => native.clone(),
            CStoreBackend::Cached { .. } => return invalid_ptr_error(err, "store_ls"),
        },
    };
    let runtime = native.runtime.clone();
    let backend = (*store).backend.clone();
    let recursive = opt.recursive != 0;
    let limit = opt.limit;
    let callback = opt.callback;
    let userdata_addr = opt.userdata as usize;
    let handle =
        runtime.spawn(async move {
            let result =
                match store_list_backend(backend, &key, recursive, limit, start_after, "store_ls")
                    .await
                {
                    Ok(entries) => COutcome::Entries(entries),
                    Err(info) => COutcome::Error(info),
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
    let recursive = opt.recursive != 0;
    let key = match parse_store_key(opt.key, "store_delete", recursive, false, err) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let native = match (*store).backend.as_ref() {
        CStoreBackend::Direct { native, .. } => native.clone(),
        CStoreBackend::Cached { parent, .. } => match parent.as_ref() {
            CStoreBackend::Direct { native, .. } => native.clone(),
            CStoreBackend::Cached { .. } => return invalid_ptr_error(err, "store_delete"),
        },
    };
    let runtime = native.runtime.clone();
    let backend = (*store).backend.clone();
    let callback = opt.callback;
    let userdata_addr = opt.userdata as usize;
    let handle = runtime.spawn(async move {
        let result = match store_delete_backend(backend, &key, recursive, "store_delete").await {
            Ok(_) => COutcome::Unit,
            Err(info) => COutcome::Error(info),
        };
        if let Some(cb) = callback {
            cb(ptr::null_mut(), userdata_addr as *mut c_void);
        }
        result
    });
    submit_handle(runtime, handle, out);
    0
}
