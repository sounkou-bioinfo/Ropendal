use std::ptr;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use super::{AioCallback, CErrorInfo, set_c_error};
use super::{COutcome, ropendal_aio, ropendal_entry, ropendal_error, ropendal_readv_result};

pub(crate) fn c_submit_handle(
    runtime: Arc<tokio::runtime::Runtime>,
    handle: tokio::task::JoinHandle<COutcome>,
    out: *mut *mut ropendal_aio,
    callback: AioCallback,
    userdata_addr: usize,
) {
    let aio = Box::into_raw(Box::new(ropendal_aio {
        refs: std::sync::atomic::AtomicUsize::new(1),
        runtime,
        handle: std::sync::Mutex::new(Some(handle)),
        cached: std::sync::Mutex::new(None),
    }));
    unsafe {
        *out = aio;
    }
    if callback.is_some() {
        unsafe {
            c_aio_retain(aio);
        }
        let aio_addr = aio as usize;
        std::thread::spawn(move || {
            let aio = aio_addr as *mut ropendal_aio;
            loop {
                let ready = unsafe { c_aio_ready(aio).unwrap_or(true) };
                if ready {
                    break;
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            unsafe {
                let _ = c_aio_finish(aio);
            }
            if let Some(cb) = callback {
                cb(aio, userdata_addr as *mut std::os::raw::c_void);
            }
            unsafe {
                c_aio_release_ref(aio);
            }
        });
    }
}

pub(crate) unsafe fn c_aio_finish(aio: *mut ropendal_aio) -> Result<COutcome, CErrorInfo> {
    if aio.is_null() {
        return Err(CErrorInfo {
            status: 2,
            kind: "InvalidArgument".to_string(),
            message: "aio pointer is null".to_string(),
            operation: "aio".to_string(),
            path: String::new(),
        });
    }
    let mut cached = (*aio).cached.lock().unwrap();
    if let Some(cached) = cached.clone() {
        return Ok(cached);
    }
    let handle = (*aio).handle.lock().unwrap().take();
    let outcome = match handle {
        Some(handle) => match (*aio).runtime.block_on(handle) {
            Ok(outcome) => outcome,
            Err(e) if e.is_cancelled() => COutcome::Cancelled,
            Err(e) => COutcome::Error(CErrorInfo {
                status: 1,
                kind: "Unexpected".to_string(),
                message: e.to_string(),
                operation: "aio".to_string(),
                path: String::new(),
            }),
        },
        None => COutcome::Cancelled,
    };
    *cached = Some(outcome.clone());
    Ok(outcome)
}

pub(crate) unsafe fn c_aio_retain(aio: *mut ropendal_aio) {
    if !aio.is_null() {
        (*aio).refs.fetch_add(1, Ordering::Relaxed);
    }
}

pub(crate) unsafe fn c_aio_release_ref(aio: *mut ropendal_aio) {
    if !aio.is_null() && (*aio).refs.fetch_sub(1, Ordering::Release) == 1 {
        std::sync::atomic::fence(Ordering::Acquire);
        drop(Box::from_raw(aio));
    }
}

unsafe fn c_aio_ready(aio: *mut ropendal_aio) -> Result<bool, CErrorInfo> {
    if aio.is_null() {
        return Err(CErrorInfo {
            status: 2,
            kind: "InvalidArgument".to_string(),
            message: "aio pointer is null".to_string(),
            operation: "aio".to_string(),
            path: String::new(),
        });
    }
    if (*aio).cached.lock().unwrap().is_some() {
        return Ok(true);
    }
    Ok(match &*(*aio).handle.lock().unwrap() {
        Some(handle) => handle.is_finished(),
        None => true,
    })
}

fn aio_timeout_error() -> CErrorInfo {
    CErrorInfo {
        status: 8,
        kind: "Timeout".to_string(),
        message: "aio wait timed out".to_string(),
        operation: "aio".to_string(),
        path: String::new(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_aio_poll(aio: *mut ropendal_aio) -> i32 {
    match c_aio_ready(aio) {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(_) => 2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_aio_wait(
    aio: *mut ropendal_aio,
    timeout_ms: i32,
    err: *mut *mut ropendal_error,
) -> i32 {
    if timeout_ms >= 0 {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms as u64);
        loop {
            match c_aio_ready(aio) {
                Ok(true) => break,
                Ok(false) => {
                    if timeout_ms == 0 || Instant::now() >= deadline {
                        let info = aio_timeout_error();
                        let status = info.status;
                        set_c_error(err, info);
                        return status;
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(info) => {
                    let status = info.status;
                    set_c_error(err, info);
                    return status;
                }
            }
        }
    }
    match c_aio_finish(aio) {
        Ok(COutcome::Error(info)) => {
            let status = info.status;
            set_c_error(err, info);
            status
        }
        Ok(COutcome::Cancelled) => 7,
        Ok(_) => 0,
        Err(info) => {
            let status = info.status;
            set_c_error(err, info);
            status
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_aio_cancel(aio: *mut ropendal_aio) {
    if !aio.is_null() && (*aio).cached.lock().unwrap().is_none() {
        if let Some(handle) = &*(*aio).handle.lock().unwrap() {
            handle.abort();
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_aio_release(aio: *mut ropendal_aio) {
    c_aio_release_ref(aio);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_aio_result_bytes(
    aio: *mut ropendal_aio,
    data: *mut *const u8,
    len: *mut usize,
    err: *mut *mut ropendal_error,
) -> i32 {
    if data.is_null() || len.is_null() {
        return 2;
    }
    match c_aio_finish(aio) {
        Ok(COutcome::Bytes(_bytes)) => {
            let cached = (*aio).cached.lock().unwrap();
            if let Some(COutcome::Bytes(ref b)) = *cached {
                *data = b.as_ptr();
                *len = b.len();
                0
            } else {
                1
            }
        }
        Ok(COutcome::Nread(n)) => {
            *data = ptr::null();
            *len = n;
            0
        }
        Ok(COutcome::Error(info)) => {
            let status = info.status;
            set_c_error(err, info);
            status
        }
        Ok(COutcome::Cancelled) => 7,
        Ok(COutcome::Readv(_)) => {
            let cached = (*aio).cached.lock().unwrap();
            if let Some(COutcome::Readv(ref set)) = *cached {
                if set.bytes.is_empty() {
                    *data = ptr::null();
                } else {
                    *data = set.bytes.as_ptr();
                }
                *len = set.bytes.len();
                0
            } else {
                1
            }
        }
        Ok(COutcome::Unit | COutcome::Bool(_) | COutcome::Entry(_) | COutcome::Entries(_)) => {
            *data = ptr::null();
            *len = 0;
            0
        }
        Err(info) => {
            let status = info.status;
            set_c_error(err, info);
            status
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_aio_result_nread(
    aio: *mut ropendal_aio,
    nread: *mut usize,
    err: *mut *mut ropendal_error,
) -> i32 {
    if nread.is_null() {
        return 2;
    }
    match c_aio_finish(aio) {
        Ok(COutcome::Nread(n)) => {
            *nread = n;
            0
        }
        Ok(COutcome::Bytes(bytes)) => {
            *nread = bytes.len();
            0
        }
        Ok(COutcome::Readv(set)) => {
            *nread = set.total_nread;
            0
        }
        Ok(COutcome::Error(info)) => {
            let status = info.status;
            set_c_error(err, info);
            status
        }
        Ok(COutcome::Cancelled) => 7,
        Ok(COutcome::Unit | COutcome::Bool(_) | COutcome::Entry(_) | COutcome::Entries(_)) => {
            *nread = 0;
            0
        }
        Err(info) => {
            let status = info.status;
            set_c_error(err, info);
            status
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_aio_result_readv(
    aio: *mut ropendal_aio,
    results: *mut *const ropendal_readv_result,
    len: *mut usize,
    err: *mut *mut ropendal_error,
) -> i32 {
    if results.is_null() || len.is_null() {
        return 2;
    }
    match c_aio_finish(aio) {
        Ok(COutcome::Readv(_)) => {
            let cached = (*aio).cached.lock().unwrap();
            if let Some(COutcome::Readv(ref set)) = *cached {
                *results = set.results.as_ptr();
                *len = set.results.len();
                0
            } else {
                1
            }
        }
        Ok(COutcome::Error(info)) => {
            let status = info.status;
            set_c_error(err, info);
            status
        }
        Ok(COutcome::Cancelled) => 7,
        Ok(_) => {
            *results = ptr::null();
            *len = 0;
            0
        }
        Err(info) => {
            let status = info.status;
            set_c_error(err, info);
            status
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_aio_result_bool(
    aio: *mut ropendal_aio,
    value: *mut i32,
    err: *mut *mut ropendal_error,
) -> i32 {
    if value.is_null() {
        return 2;
    }
    match c_aio_finish(aio) {
        Ok(COutcome::Bool(v)) => {
            *value = if v { 1 } else { 0 };
            0
        }
        Ok(COutcome::Error(info)) => {
            let status = info.status;
            set_c_error(err, info);
            status
        }
        Ok(COutcome::Cancelled) => 7,
        Ok(_) => {
            *value = 0;
            0
        }
        Err(info) => {
            let status = info.status;
            set_c_error(err, info);
            status
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_aio_result_entries(
    aio: *mut ropendal_aio,
    entries: *mut *const ropendal_entry,
    len: *mut usize,
    err: *mut *mut ropendal_error,
) -> i32 {
    if entries.is_null() || len.is_null() {
        return 2;
    }
    match c_aio_finish(aio) {
        Ok(COutcome::Entries(_)) => {
            let cached = (*aio).cached.lock().unwrap();
            if let Some(COutcome::Entries(ref set)) = *cached {
                *entries = set.entries.as_ptr();
                *len = set.entries.len();
                0
            } else {
                1
            }
        }
        Ok(COutcome::Entry(_)) => {
            let cached = (*aio).cached.lock().unwrap();
            if let Some(COutcome::Entry(ref set)) = *cached {
                *entries = set.entries.as_ptr();
                *len = set.entries.len();
                0
            } else {
                1
            }
        }
        Ok(COutcome::Error(info)) => {
            let status = info.status;
            set_c_error(err, info);
            status
        }
        Ok(COutcome::Cancelled) => 7,
        Ok(_) => {
            *entries = ptr::null();
            *len = 0;
            0
        }
        Err(info) => {
            let status = info.status;
            set_c_error(err, info);
            status
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_aio_result_entry(
    aio: *mut ropendal_aio,
    entry: *mut *const ropendal_entry,
    err: *mut *mut ropendal_error,
) -> i32 {
    if entry.is_null() {
        return 2;
    }
    match c_aio_finish(aio) {
        Ok(COutcome::Entry(_)) => {
            let cached = (*aio).cached.lock().unwrap();
            if let Some(COutcome::Entry(ref set)) = *cached {
                *entry = set.entries.as_ptr();
                0
            } else {
                1
            }
        }
        Ok(COutcome::Entries(_)) => {
            let cached = (*aio).cached.lock().unwrap();
            if let Some(COutcome::Entries(ref set)) = *cached {
                *entry = set.entries.first().map_or(ptr::null(), |e| e as *const _);
                0
            } else {
                1
            }
        }
        Ok(COutcome::Error(info)) => {
            let status = info.status;
            set_c_error(err, info);
            status
        }
        Ok(COutcome::Cancelled) => 7,
        Ok(_) => {
            *entry = ptr::null();
            0
        }
        Err(info) => {
            let status = info.status;
            set_c_error(err, info);
            status
        }
    }
}
