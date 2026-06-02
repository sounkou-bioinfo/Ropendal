use std::ptr;

use super::{CErrorInfo, set_c_error};
use super::{COutcome, ropendal_aio, ropendal_entry, ropendal_error};

fn c_aio_finish(aio: *mut ropendal_aio) -> Result<COutcome, CErrorInfo> {
    unsafe {
        if aio.is_null() {
            return Err(CErrorInfo {
                status: 2,
                kind: "InvalidArgument".to_string(),
                message: "aio pointer is null".to_string(),
                operation: "aio".to_string(),
                path: String::new(),
            });
        }
        if let Some(cached) = (*aio).cached.lock().unwrap().clone() {
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
        *(*aio).cached.lock().unwrap() = Some(outcome.clone());
        Ok(outcome)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_aio_poll(aio: *mut ropendal_aio) -> i32 {
    if aio.is_null() {
        return 2;
    }
    if (*aio).cached.lock().unwrap().is_some() {
        return 1;
    }
    match &*(*aio).handle.lock().unwrap() {
        Some(handle) if handle.is_finished() => 1,
        Some(_) => 0,
        None => 1,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_aio_wait(
    aio: *mut ropendal_aio,
    timeout_ms: i32,
    err: *mut *mut ropendal_error,
) -> i32 {
    let _ = timeout_ms;
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
    if !aio.is_null() {
        if let Some(handle) = &*(*aio).handle.lock().unwrap() {
            handle.abort();
        }
        *(*aio).cached.lock().unwrap() = Some(COutcome::Cancelled);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_aio_release(aio: *mut ropendal_aio) {
    if !aio.is_null() {
        drop(Box::from_raw(aio));
    }
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
