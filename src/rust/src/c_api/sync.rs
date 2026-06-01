use std::time::Duration;

use super::{ropendal_aio, ropendal_cv, ropendal_error, ropendal_monitor};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_cv_alloc(
    out: *mut *mut ropendal_cv,
    err: *mut *mut ropendal_error,
) -> i32 {
    let _ = err;
    if out.is_null() {
        return 2;
    }
    *out = Box::into_raw(Box::new(ropendal_cv {
        state: std::sync::Mutex::new(0),
        cv: std::sync::Condvar::new(),
    }));
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_cv_release(cv: *mut ropendal_cv) {
    if !cv.is_null() {
        drop(Box::from_raw(cv));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_cv_wait(
    cv: *mut ropendal_cv,
    err: *mut *mut ropendal_error,
) -> i32 {
    let _ = err;
    if cv.is_null() {
        return 2;
    }
    let guard = (*cv).state.lock().unwrap();
    let _guard = (*cv).cv.wait(guard).unwrap();
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_cv_until(
    cv: *mut ropendal_cv,
    timeout_ms: i32,
    err: *mut *mut ropendal_error,
) -> i32 {
    let _ = err;
    if cv.is_null() {
        return 2;
    }
    let guard = (*cv).state.lock().unwrap();
    let dur = Duration::from_millis(timeout_ms.max(0) as u64);
    let _ = (*cv).cv.wait_timeout(guard, dur).unwrap();
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_cv_value(cv: *mut ropendal_cv) -> u64 {
    if cv.is_null() {
        0
    } else {
        *(*cv).state.lock().unwrap()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_cv_reset(cv: *mut ropendal_cv) {
    if !cv.is_null() {
        *(*cv).state.lock().unwrap() = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_cv_signal(cv: *mut ropendal_cv) {
    if !cv.is_null() {
        let mut v = (*cv).state.lock().unwrap();
        *v += 1;
        (*cv).cv.notify_all();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_aio_notify(
    aio: *mut ropendal_aio,
    cv: *mut ropendal_cv,
    id: u64,
    err: *mut *mut ropendal_error,
) -> i32 {
    let _ = (aio, cv, id, err);
    3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_monitor_create(
    cv: *mut ropendal_cv,
    out: *mut *mut ropendal_monitor,
    err: *mut *mut ropendal_error,
) -> i32 {
    let _ = (cv, out, err);
    3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_monitor_add_aio(
    monitor: *mut ropendal_monitor,
    aio: *mut ropendal_aio,
    id: u64,
    err: *mut *mut ropendal_error,
) -> i32 {
    let _ = (monitor, aio, id, err);
    3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_monitor_read(
    monitor: *mut ropendal_monitor,
    events: *mut *const std::os::raw::c_void,
    len: *mut usize,
    err: *mut *mut ropendal_error,
) -> i32 {
    let _ = (monitor, events, len, err);
    3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_monitor_release(monitor: *mut ropendal_monitor) {
    let _ = monitor;
}
