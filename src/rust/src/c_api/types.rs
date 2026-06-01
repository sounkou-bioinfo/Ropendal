use std::ffi::CString;
use std::os::raw::{c_char, c_void};
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Condvar, Mutex};

use tokio::task::JoinHandle;

use crate::common::NativeFs;

pub(crate) type AioCallback = Option<extern "C" fn(*mut ropendal_aio, *mut c_void)>;

#[repr(C)]
pub struct ropendal_kv {
    pub(crate) struct_size: usize,
    pub(crate) key: *const c_char,
    pub(crate) value: *const c_char,
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

#[derive(Clone)]
pub(crate) enum COutcome {
    Bytes(Vec<u8>),
    Nread(usize),
    Unit,
    Error(super::error::CErrorInfo),
    Cancelled,
}

pub struct ropendal_aio {
    pub(crate) runtime: Arc<tokio::runtime::Runtime>,
    pub(crate) handle: Mutex<Option<JoinHandle<COutcome>>>,
    pub(crate) cached: Mutex<Option<COutcome>>,
}

pub struct ropendal_cv {
    pub(crate) state: Mutex<u64>,
    pub(crate) cv: Condvar,
}

pub struct ropendal_monitor;
