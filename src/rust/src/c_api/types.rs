use std::ffi::CString;
use std::os::raw::{c_char, c_void};
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle as StdJoinHandle;

use opendal::Metadata;
use tokio::task::JoinHandle;

use crate::common::NativeFs;

pub(crate) type AioCallback = Option<extern "C" fn(*mut ropendal_aio, *mut c_void)>;

pub struct ropendal_bytes {
    pub(crate) bytes: Vec<u8>,
}

#[repr(C)]
pub struct ropendal_kv {
    pub(crate) struct_size: usize,
    pub(crate) key: *const c_char,
    pub(crate) value: *const c_char,
}

#[repr(C)]
pub struct ropendal_store_options {
    pub(crate) struct_size: usize,
    pub(crate) prefix: *const c_char,
}

#[repr(C)]
pub struct ropendal_store_cache_options {
    pub(crate) struct_size: usize,
    pub(crate) validate: i32,
}

#[repr(C)]
pub struct ropendal_store_block_cache_options {
    pub(crate) struct_size: usize,
    pub(crate) block_size: u64,
    pub(crate) validate: i32,
}

#[repr(C)]
pub struct ropendal_store_read_options {
    pub(crate) struct_size: usize,
    pub(crate) key: *const c_char,
    pub(crate) has_offset: i32,
    pub(crate) offset: u64,
    pub(crate) has_size: i32,
    pub(crate) size: u64,
    pub(crate) part_concurrency: usize,
    pub(crate) chunk_size: usize,
    pub(crate) coalesce_gap: usize,
    pub(crate) callback: AioCallback,
    pub(crate) userdata: *mut c_void,
}

#[repr(C)]
pub struct ropendal_store_write_options {
    pub(crate) struct_size: usize,
    pub(crate) key: *const c_char,
    pub(crate) part_concurrency: usize,
    pub(crate) chunk_size: usize,
    pub(crate) callback: AioCallback,
    pub(crate) userdata: *mut c_void,
}

#[repr(C)]
pub struct ropendal_store_ls_options {
    pub(crate) struct_size: usize,
    pub(crate) path: *const c_char,
    pub(crate) recursive: i32,
    pub(crate) limit: usize,
    pub(crate) start_after: *const c_char,
    pub(crate) callback: AioCallback,
    pub(crate) userdata: *mut c_void,
}

#[repr(C)]
pub struct ropendal_store_delete_options {
    pub(crate) struct_size: usize,
    pub(crate) key: *const c_char,
    pub(crate) recursive: i32,
    pub(crate) callback: AioCallback,
    pub(crate) userdata: *mut c_void,
}

#[repr(C)]
pub struct ropendal_read_options {
    pub(crate) struct_size: usize,
    pub(crate) path: *const c_char,
    pub(crate) offset: u64,
    pub(crate) size: u64,
    pub(crate) has_size: i32,
    pub(crate) content_length_hint: u64,
    pub(crate) has_content_length_hint: i32,
    pub(crate) version: *const c_char,
    pub(crate) if_match: *const c_char,
    pub(crate) if_none_match: *const c_char,
    pub(crate) part_concurrency: usize,
    pub(crate) chunk_size: usize,
    pub(crate) coalesce_gap: usize,
    pub(crate) prefetch: usize,
    pub(crate) callback: AioCallback,
    pub(crate) userdata: *mut c_void,
}

#[repr(C)]
pub struct ropendal_read_request {
    pub(crate) struct_size: usize,
    pub(crate) path: *const c_char,
    pub(crate) offset: u64,
    pub(crate) size: u64,
    pub(crate) has_size: i32,
    pub(crate) content_length_hint: u64,
    pub(crate) has_content_length_hint: i32,
    pub(crate) version: *const c_char,
}

#[repr(C)]
pub struct ropendal_read_into_request {
    pub(crate) struct_size: usize,
    pub(crate) path: *const c_char,
    pub(crate) offset: u64,
    pub(crate) size: u64,
    pub(crate) has_size: i32,
    pub(crate) dst: *mut u8,
    pub(crate) dst_len: usize,
}

#[repr(C)]
pub struct ropendal_readv_options {
    pub(crate) struct_size: usize,
    pub(crate) batch_concurrency: usize,
    pub(crate) part_concurrency: usize,
    pub(crate) chunk_size: usize,
    pub(crate) coalesce_gap: usize,
    pub(crate) preserve_order: i32,
    pub(crate) callback: AioCallback,
    pub(crate) userdata: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ropendal_monitor_event {
    pub(crate) struct_size: usize,
    pub(crate) kind: i32,
    pub(crate) aio: *mut ropendal_aio,
    pub(crate) id: u64,
}

// `ropendal_monitor_event` values are copied into monitor-owned queues and
// snapshots. The pointed-to Aio is retained by the monitor until release.
unsafe impl Send for ropendal_monitor_event {}
unsafe impl Sync for ropendal_monitor_event {}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ropendal_readv_result {
    pub(crate) struct_size: usize,
    pub(crate) index: usize,
    pub(crate) status: i32,
    pub(crate) nread: usize,
    pub(crate) kind: *const c_char,
    pub(crate) message: *const c_char,
    pub(crate) path: *const c_char,
}

// `ropendal_readv_result` pointers borrow from `CString`s owned by the same
// `CReadvResultSet` outcome. The bytes are immutable and live until Aio release.
unsafe impl Send for ropendal_readv_result {}
unsafe impl Sync for ropendal_readv_result {}

#[repr(C)]
pub struct ropendal_write_options {
    pub(crate) struct_size: usize,
    pub(crate) path: *const c_char,
    pub(crate) if_match: *const c_char,
    pub(crate) if_none_match: *const c_char,
    pub(crate) content_type: *const c_char,
    pub(crate) content_encoding: *const c_char,
    pub(crate) content_disposition: *const c_char,
    pub(crate) cache_control: *const c_char,
    pub(crate) part_concurrency: usize,
    pub(crate) chunk_size: usize,
    pub(crate) callback: AioCallback,
    pub(crate) userdata: *mut c_void,
}

#[repr(C)]
pub struct ropendal_ls_options {
    pub(crate) struct_size: usize,
    pub(crate) path: *const c_char,
    pub(crate) recursive: i32,
    pub(crate) limit: usize,
    pub(crate) start_after: *const c_char,
    pub(crate) versions: i32,
    pub(crate) deleted: i32,
    pub(crate) callback: AioCallback,
    pub(crate) userdata: *mut c_void,
}

#[repr(C)]
pub struct ropendal_delete_options {
    pub(crate) struct_size: usize,
    pub(crate) path: *const c_char,
    pub(crate) recursive: i32,
    pub(crate) version: *const c_char,
    pub(crate) callback: AioCallback,
    pub(crate) userdata: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ropendal_entry {
    pub(crate) struct_size: usize,
    pub(crate) path: *const c_char,
    pub(crate) name: *const c_char,
    pub(crate) mode: i32,
    pub(crate) content_length: u64,
    pub(crate) has_content_length: i32,
    pub(crate) etag: *const c_char,
    pub(crate) content_type: *const c_char,
    pub(crate) content_encoding: *const c_char,
    pub(crate) last_modified: *const c_char,
    pub(crate) version: *const c_char,
}

// `ropendal_entry` pointers always borrow from `CString`s owned by the same
// `CEntrySet` outcome. The outcome is moved between worker and caller threads,
// but the pointed-to bytes are immutable and remain owned until Aio release.
unsafe impl Send for ropendal_entry {}
unsafe impl Sync for ropendal_entry {}

pub(crate) struct CEntrySet {
    pub(crate) entries: Vec<ropendal_entry>,
    _strings: Vec<CEntryStrings>,
}

pub(crate) struct CReadvResultSet {
    pub(crate) results: Vec<ropendal_readv_result>,
    pub(crate) total_nread: usize,
    pub(crate) bytes: Vec<u8>,
    _strings: Vec<CReadvResultStrings>,
}

#[derive(Clone)]
struct CEntryStrings {
    path: CString,
    name: CString,
    etag: Option<CString>,
    content_type: Option<CString>,
    content_encoding: Option<CString>,
    last_modified: Option<CString>,
    version: Option<CString>,
}

#[derive(Clone)]
struct CReadvResultStrings {
    kind: CString,
    message: CString,
    path: CString,
}

pub(crate) enum CReadvTaskResult {
    Ok {
        index: usize,
        path: String,
        nread: usize,
        bytes: Option<Vec<u8>>,
    },
    Error {
        index: usize,
        info: super::error::CErrorInfo,
    },
}

#[repr(i32)]
enum CEntryMode {
    Unknown = 0,
    File = 1,
    Dir = 2,
}

fn cstring_lossy(s: &str) -> CString {
    CString::new(s.replace('\0', " ")).unwrap_or_else(|_| CString::new("").unwrap())
}

fn opt_cstring(value: Option<&str>) -> Option<CString> {
    value.map(cstring_lossy)
}

fn entry_name(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    let name = trimmed.rsplit('/').next().unwrap_or(trimmed);
    if name.is_empty() {
        path.to_string()
    } else {
        name.to_string()
    }
}

fn entry_mode(meta: &Metadata) -> i32 {
    if meta.is_file() {
        CEntryMode::File as i32
    } else if meta.is_dir() {
        CEntryMode::Dir as i32
    } else {
        CEntryMode::Unknown as i32
    }
}

impl Clone for CEntrySet {
    fn clone(&self) -> Self {
        let strings = self._strings.clone();
        let mut entries = self.entries.clone();
        for (entry, entry_strings) in entries.iter_mut().zip(strings.iter()) {
            entry.path = entry_strings.path.as_ptr();
            entry.name = entry_strings.name.as_ptr();
            entry.etag = entry_strings
                .etag
                .as_ref()
                .map_or(std::ptr::null(), |s| s.as_ptr());
            entry.content_type = entry_strings
                .content_type
                .as_ref()
                .map_or(std::ptr::null(), |s| s.as_ptr());
            entry.content_encoding = entry_strings
                .content_encoding
                .as_ref()
                .map_or(std::ptr::null(), |s| s.as_ptr());
            entry.last_modified = entry_strings
                .last_modified
                .as_ref()
                .map_or(std::ptr::null(), |s| s.as_ptr());
            entry.version = entry_strings
                .version
                .as_ref()
                .map_or(std::ptr::null(), |s| s.as_ptr());
        }
        Self {
            entries,
            _strings: strings,
        }
    }
}

impl Clone for CReadvResultSet {
    fn clone(&self) -> Self {
        let strings = self._strings.clone();
        let mut results = self.results.clone();
        for (result, result_strings) in results.iter_mut().zip(strings.iter()) {
            result.kind = result_strings.kind.as_ptr();
            result.message = result_strings.message.as_ptr();
            result.path = result_strings.path.as_ptr();
        }
        Self {
            results,
            total_nread: self.total_nread,
            bytes: self.bytes.clone(),
            _strings: strings,
        }
    }
}

impl CReadvResultSet {
    pub(crate) fn from_task_results(mut values: Vec<CReadvTaskResult>) -> Self {
        values.sort_by_key(|value| match value {
            CReadvTaskResult::Ok { index, .. } => *index,
            CReadvTaskResult::Error { index, .. } => *index,
        });
        let mut strings = Vec::with_capacity(values.len());
        let mut results = Vec::with_capacity(values.len());
        let mut total_nread = 0usize;
        let mut bytes = Vec::new();
        for value in values {
            let (index, status, nread, kind, message, path) = match value {
                CReadvTaskResult::Ok {
                    index,
                    path,
                    nread,
                    bytes: result_bytes,
                } => {
                    total_nread = total_nread.saturating_add(nread);
                    if let Some(result_bytes) = result_bytes {
                        bytes.extend_from_slice(&result_bytes);
                    }
                    (index, 0, nread, String::new(), String::new(), path)
                }
                CReadvTaskResult::Error { index, info } => {
                    (index, info.status, 0, info.kind, info.message, info.path)
                }
            };
            let result_strings = CReadvResultStrings {
                kind: cstring_lossy(&kind),
                message: cstring_lossy(&message),
                path: cstring_lossy(&path),
            };
            let result = ropendal_readv_result {
                struct_size: std::mem::size_of::<ropendal_readv_result>(),
                index,
                status,
                nread,
                kind: result_strings.kind.as_ptr(),
                message: result_strings.message.as_ptr(),
                path: result_strings.path.as_ptr(),
            };
            strings.push(result_strings);
            results.push(result);
        }
        Self {
            results,
            total_nread,
            bytes,
            _strings: strings,
        }
    }
}

impl CEntrySet {
    pub(crate) fn one(path: &str, meta: &Metadata) -> Self {
        Self::from_entries(vec![(path.to_string(), meta.clone())])
    }

    pub(crate) fn from_entries(values: Vec<(String, Metadata)>) -> Self {
        let mut strings = Vec::with_capacity(values.len());
        let mut entries = Vec::with_capacity(values.len());
        for (path, meta) in values {
            let last_modified = meta.last_modified().map(|v| v.to_string());
            let entry_strings = CEntryStrings {
                path: cstring_lossy(&path),
                name: cstring_lossy(&entry_name(&path)),
                etag: opt_cstring(meta.etag()),
                content_type: opt_cstring(meta.content_type()),
                content_encoding: opt_cstring(meta.content_encoding()),
                last_modified: opt_cstring(last_modified.as_deref()),
                version: opt_cstring(meta.version()),
            };
            let entry = ropendal_entry {
                struct_size: std::mem::size_of::<ropendal_entry>(),
                path: entry_strings.path.as_ptr(),
                name: entry_strings.name.as_ptr(),
                mode: entry_mode(&meta),
                content_length: meta.content_length(),
                has_content_length: if meta.is_file() { 1 } else { 0 },
                etag: entry_strings
                    .etag
                    .as_ref()
                    .map_or(std::ptr::null(), |s| s.as_ptr()),
                content_type: entry_strings
                    .content_type
                    .as_ref()
                    .map_or(std::ptr::null(), |s| s.as_ptr()),
                content_encoding: entry_strings
                    .content_encoding
                    .as_ref()
                    .map_or(std::ptr::null(), |s| s.as_ptr()),
                last_modified: entry_strings
                    .last_modified
                    .as_ref()
                    .map_or(std::ptr::null(), |s| s.as_ptr()),
                version: entry_strings
                    .version
                    .as_ref()
                    .map_or(std::ptr::null(), |s| s.as_ptr()),
            };
            strings.push(entry_strings);
            entries.push(entry);
        }
        Self {
            entries,
            _strings: strings,
        }
    }
}

#[repr(C)]
pub struct ropendal_error {
    pub(crate) status: i32,
    pub(crate) kind: CString,
    pub(crate) message: CString,
    pub(crate) operation: CString,
    pub(crate) path: CString,
}

pub struct ropendal_fs {
    pub(crate) refs: AtomicUsize,
    pub(crate) native: Arc<NativeFs>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CStoreCacheValidate {
    LastModifiedSize,
    None,
}

pub(crate) enum CStoreBackend {
    Direct {
        native: Arc<NativeFs>,
        prefix: String,
    },
    Cached {
        parent: Arc<CStoreBackend>,
        cache: Arc<CStoreBackend>,
        validate: CStoreCacheValidate,
    },
    BlockCached {
        parent: Arc<CStoreBackend>,
        cache: Arc<CStoreBackend>,
        validate: CStoreCacheValidate,
        block_size: u64,
    },
}

pub struct ropendal_store {
    pub(crate) refs: AtomicUsize,
    pub(crate) backend: Arc<CStoreBackend>,
}

#[derive(Clone)]
pub(crate) enum COutcome {
    Bytes(Vec<u8>),
    Nread(usize),
    Unit,
    Bool(bool),
    Entry(CEntrySet),
    Entries(CEntrySet),
    Readv(CReadvResultSet),
    Error(super::error::CErrorInfo),
    Cancelled,
}

pub struct ropendal_aio {
    pub(crate) refs: AtomicUsize,
    pub(crate) runtime: Arc<tokio::runtime::Runtime>,
    pub(crate) handle: Mutex<Option<JoinHandle<COutcome>>>,
    pub(crate) cached: Mutex<Option<COutcome>>,
}

pub struct ropendal_cv {
    pub(crate) refs: AtomicUsize,
    pub(crate) state: Mutex<u64>,
    pub(crate) cv: Condvar,
}

pub struct ropendal_monitor {
    pub(crate) cv: *mut ropendal_cv,
    pub(crate) queue: Arc<Mutex<Vec<ropendal_monitor_event>>>,
    pub(crate) snapshot: Mutex<Vec<ropendal_monitor_event>>,
    pub(crate) retained_aios: Mutex<Vec<*mut ropendal_aio>>,
    pub(crate) threads: Mutex<Vec<StdJoinHandle<()>>>,
}
