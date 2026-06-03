use std::ptr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::aio::{c_aio_finish, c_aio_release_ref, c_aio_retain};
use super::{
    CErrorInfo, COutcome, ropendal_aio, ropendal_cv, ropendal_error, ropendal_monitor,
    ropendal_monitor_event, set_c_error,
};

const ROPENDAL_OK: i32 = 0;
const ROPENDAL_INVALID_ARGUMENT: i32 = 2;
const ROPENDAL_EVENT_AIO_READY: i32 = 1;
const ROPENDAL_EVENT_AIO_ERROR: i32 = 2;
const ROPENDAL_EVENT_AIO_CANCELLED: i32 = 3;

fn invalid_sync_error(message: &str) -> CErrorInfo {
    CErrorInfo {
        status: ROPENDAL_INVALID_ARGUMENT,
        kind: "InvalidArgument".to_string(),
        message: message.to_string(),
        operation: "sync".to_string(),
        path: String::new(),
    }
}

unsafe fn cv_retain(cv: *mut ropendal_cv) {
    if !cv.is_null() {
        (*cv).refs.fetch_add(1, Ordering::Relaxed);
    }
}

unsafe fn cv_release_ref(cv: *mut ropendal_cv) {
    if !cv.is_null() && (*cv).refs.fetch_sub(1, Ordering::Release) == 1 {
        std::sync::atomic::fence(Ordering::Acquire);
        drop(Box::from_raw(cv));
    }
}

unsafe fn cv_signal_ptr(cv: *mut ropendal_cv) {
    if !cv.is_null() {
        let mut v = (*cv).state.lock().unwrap();
        *v += 1;
        (*cv).cv.notify_all();
    }
}

fn event_kind(result: &Result<COutcome, CErrorInfo>) -> i32 {
    match result {
        Ok(COutcome::Cancelled) => ROPENDAL_EVENT_AIO_CANCELLED,
        Ok(COutcome::Error(_)) | Err(_) => ROPENDAL_EVENT_AIO_ERROR,
        Ok(_) => ROPENDAL_EVENT_AIO_READY,
    }
}

fn spawn_aio_notification(
    aio_addr: usize,
    cv_addr: usize,
    id: u64,
    queue: Option<Arc<Mutex<Vec<ropendal_monitor_event>>>>,
    release_aio: bool,
    release_cv: bool,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let aio = aio_addr as *mut ropendal_aio;
        loop {
            let ready = unsafe {
                if aio.is_null() {
                    true
                } else if (*aio).cached.lock().unwrap().is_some() {
                    true
                } else {
                    match &*(*aio).handle.lock().unwrap() {
                        Some(handle) => handle.is_finished(),
                        None => true,
                    }
                }
            };
            if ready {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        let result = unsafe { c_aio_finish(aio) };
        let kind = event_kind(&result);
        if let Some(queue) = queue {
            queue.lock().unwrap().push(ropendal_monitor_event {
                struct_size: std::mem::size_of::<ropendal_monitor_event>(),
                kind,
                aio,
                id,
            });
        }
        unsafe {
            cv_signal_ptr(cv_addr as *mut ropendal_cv);
            if release_aio {
                c_aio_release_ref(aio);
            }
            if release_cv {
                cv_release_ref(cv_addr as *mut ropendal_cv);
            }
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_cv_alloc(
    out: *mut *mut ropendal_cv,
    err: *mut *mut ropendal_error,
) -> i32 {
    let _ = err;
    if out.is_null() {
        return ROPENDAL_INVALID_ARGUMENT;
    }
    *out = Box::into_raw(Box::new(ropendal_cv {
        refs: std::sync::atomic::AtomicUsize::new(1),
        state: std::sync::Mutex::new(0),
        cv: std::sync::Condvar::new(),
    }));
    ROPENDAL_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_cv_release(cv: *mut ropendal_cv) {
    cv_release_ref(cv);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_cv_wait(
    cv: *mut ropendal_cv,
    err: *mut *mut ropendal_error,
) -> i32 {
    let _ = err;
    if cv.is_null() {
        return ROPENDAL_INVALID_ARGUMENT;
    }
    let guard = (*cv).state.lock().unwrap();
    let _guard = (*cv).cv.wait(guard).unwrap();
    ROPENDAL_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_cv_until(
    cv: *mut ropendal_cv,
    timeout_ms: i32,
    err: *mut *mut ropendal_error,
) -> i32 {
    let _ = err;
    if cv.is_null() {
        return ROPENDAL_INVALID_ARGUMENT;
    }
    let guard = (*cv).state.lock().unwrap();
    let dur = Duration::from_millis(timeout_ms.max(0) as u64);
    let _ = (*cv).cv.wait_timeout(guard, dur).unwrap();
    ROPENDAL_OK
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
    cv_signal_ptr(cv);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_aio_notify(
    aio: *mut ropendal_aio,
    cv: *mut ropendal_cv,
    id: u64,
    err: *mut *mut ropendal_error,
) -> i32 {
    let _ = id;
    if aio.is_null() || cv.is_null() {
        set_c_error(err, invalid_sync_error("aio and cv must not be null"));
        return ROPENDAL_INVALID_ARGUMENT;
    }
    c_aio_retain(aio);
    cv_retain(cv);
    let _ = spawn_aio_notification(aio as usize, cv as usize, id, None, true, true);
    ROPENDAL_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_monitor_create(
    cv: *mut ropendal_cv,
    out: *mut *mut ropendal_monitor,
    err: *mut *mut ropendal_error,
) -> i32 {
    if cv.is_null() || out.is_null() {
        set_c_error(err, invalid_sync_error("cv and out must not be null"));
        return ROPENDAL_INVALID_ARGUMENT;
    }
    cv_retain(cv);
    *out = Box::into_raw(Box::new(ropendal_monitor {
        cv,
        queue: Arc::new(Mutex::new(Vec::new())),
        snapshot: Mutex::new(Vec::new()),
        retained_aios: Mutex::new(Vec::new()),
        threads: Mutex::new(Vec::new()),
    }));
    ROPENDAL_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_monitor_add_aio(
    monitor: *mut ropendal_monitor,
    aio: *mut ropendal_aio,
    id: u64,
    err: *mut *mut ropendal_error,
) -> i32 {
    if monitor.is_null() || aio.is_null() {
        set_c_error(err, invalid_sync_error("monitor and aio must not be null"));
        return ROPENDAL_INVALID_ARGUMENT;
    }
    c_aio_retain(aio);
    (*monitor).retained_aios.lock().unwrap().push(aio);
    let handle = spawn_aio_notification(
        aio as usize,
        (*monitor).cv as usize,
        id,
        Some((*monitor).queue.clone()),
        false,
        false,
    );
    (*monitor).threads.lock().unwrap().push(handle);
    ROPENDAL_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_monitor_read(
    monitor: *mut ropendal_monitor,
    events: *mut *const ropendal_monitor_event,
    len: *mut usize,
    err: *mut *mut ropendal_error,
) -> i32 {
    if monitor.is_null() || events.is_null() || len.is_null() {
        set_c_error(
            err,
            invalid_sync_error("monitor, events, and len must not be null"),
        );
        return ROPENDAL_INVALID_ARGUMENT;
    }
    let mut queue = (*monitor).queue.lock().unwrap();
    let mut snapshot = (*monitor).snapshot.lock().unwrap();
    snapshot.clear();
    snapshot.extend(queue.drain(..));
    *len = snapshot.len();
    *events = if snapshot.is_empty() {
        ptr::null()
    } else {
        snapshot.as_ptr()
    };
    ROPENDAL_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ropendal_monitor_release(monitor: *mut ropendal_monitor) {
    if monitor.is_null() {
        return;
    }
    let monitor = Box::from_raw(monitor);
    let threads = {
        let mut guard = monitor.threads.lock().unwrap();
        std::mem::take(&mut *guard)
    };
    for handle in threads {
        let _ = handle.join();
    }
    let retained_aios = {
        let mut guard = monitor.retained_aios.lock().unwrap();
        std::mem::take(&mut *guard)
    };
    for aio in retained_aios {
        c_aio_release_ref(aio);
    }
    cv_release_ref(monitor.cv);
}
