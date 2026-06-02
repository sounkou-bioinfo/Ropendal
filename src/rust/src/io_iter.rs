use std::sync::{Arc, Mutex};

use bytes::Bytes;
use futures::SinkExt;
use opendal::{Buffer, FuturesBytesSink, Operator};
use savvy::savvy;
use savvy::{OwnedListSexp, Sexp, TypedSexp};

use crate::common::NativeFs;
use crate::error::{error_list, op_error_list};
use crate::ops::{ReadTuning, WriteTuning, read_bytes_with};
use crate::path::normalize_user_path;
use crate::r_values::{
    bool_scalar, buffer_to_raw_sexp, buffers_to_raw_sexp, real_scalar, success_value,
};

struct ReadIterState {
    path: String,
    start: u64,
    end: Option<u64>,
    position: u64,
    chunk_size: u64,
    tuning: ReadTuning,
    done: bool,
}

/// Chunked read iterator over one object.
/// @export
#[savvy]
pub struct OpendalReadIter {
    inner: Arc<NativeFs>,
    state: Mutex<ReadIterState>,
}

impl OpendalReadIter {
    pub(crate) fn new(
        inner: Arc<NativeFs>,
        path: String,
        offset: u64,
        size: Option<u64>,
        chunk_size: u64,
        tuning: ReadTuning,
    ) -> Self {
        Self {
            inner,
            state: Mutex::new(ReadIterState {
                path,
                start: offset,
                end: size.map(|n| offset.saturating_add(n)),
                position: offset,
                chunk_size,
                tuning,
                done: size == Some(0),
            }),
        }
    }
}

#[savvy]
impl OpendalReadIter {
    /// Return the next chunk as list(done, data).
    /// @export
    fn next(&self) -> savvy::Result<savvy::Sexp> {
        let (path, offset, size, tuning) = {
            let state = self
                .state
                .lock()
                .map_err(|_| savvy::Error::new("read iterator lock poisoned"))?;
            if state.done {
                return iter_chunk(true, Buffer::new());
            }
            let remaining = state
                .end
                .map(|end| end.saturating_sub(state.position))
                .unwrap_or(state.chunk_size);
            let size = remaining.min(state.chunk_size);
            (state.path.clone(), state.position, size, state.tuning)
        };

        if size == 0 {
            let mut state = self
                .state
                .lock()
                .map_err(|_| savvy::Error::new("read iterator lock poisoned"))?;
            state.done = true;
            return iter_chunk(true, Buffer::new());
        }

        let op = self.inner.op.clone();
        let result = self.inner.runtime.block_on(read_bytes_with(
            op,
            path.clone(),
            offset,
            Some(size),
            tuning,
        ));

        match result {
            Ok(bytes) => {
                let len = bytes.len() as u64;
                let mut state = self
                    .state
                    .lock()
                    .map_err(|_| savvy::Error::new("read iterator lock poisoned"))?;
                state.position = state.position.saturating_add(len);
                if state.end.is_some_and(|end| state.position >= end) || len < size || len == 0 {
                    state.done = true;
                }
                if len == 0 {
                    iter_chunk(true, Buffer::new())
                } else {
                    iter_chunk(false, bytes)
                }
            }
            Err(e) => op_error_list(e, "read_iter", &path),
        }
    }

    /// Read all remaining chunks and concatenate them.
    /// @export
    fn collect(&self) -> savvy::Result<savvy::Sexp> {
        let mut out = Vec::new();
        loop {
            let (path, offset, size, tuning) = {
                let state = self
                    .state
                    .lock()
                    .map_err(|_| savvy::Error::new("read iterator lock poisoned"))?;
                if state.done {
                    break;
                }
                let remaining = state
                    .end
                    .map(|end| end.saturating_sub(state.position))
                    .unwrap_or(state.chunk_size);
                let size = remaining.min(state.chunk_size);
                (state.path.clone(), state.position, size, state.tuning)
            };
            if size == 0 {
                let mut state = self
                    .state
                    .lock()
                    .map_err(|_| savvy::Error::new("read iterator lock poisoned"))?;
                state.done = true;
                break;
            }
            let bytes = match self.inner.runtime.block_on(read_bytes_with(
                self.inner.op.clone(),
                path.clone(),
                offset,
                Some(size),
                tuning,
            )) {
                Ok(bytes) => bytes,
                Err(e) => return op_error_list(e, "read_iter", &path),
            };
            let len = bytes.len() as u64;
            {
                let mut state = self
                    .state
                    .lock()
                    .map_err(|_| savvy::Error::new("read iterator lock poisoned"))?;
                state.position = state.position.saturating_add(len);
                if state.end.is_some_and(|end| state.position >= end) || len < size || len == 0 {
                    state.done = true;
                }
            }
            if len == 0 {
                break;
            }
            out.push(bytes);
        }
        buffers_to_raw_sexp(out).map(|x| x.into())
    }

    /// Return the current byte position relative to this iterator's start.
    /// @export
    fn tell(&self) -> savvy::Result<savvy::Sexp> {
        let state = self
            .state
            .lock()
            .map_err(|_| savvy::Error::new("read iterator lock poisoned"))?;
        real_scalar(state.position.saturating_sub(state.start) as f64)?.into()
    }

    /// Seek to a byte position within this iterator's read window.
    /// @export
    fn seek(&self, offset: f64, whence: Option<&str>) -> savvy::Result<savvy::Sexp> {
        let offset = checked_i128(offset, "offset")?;
        let whence = whence.unwrap_or("start");
        let mut state = self
            .state
            .lock()
            .map_err(|_| savvy::Error::new("read iterator lock poisoned"))?;
        let current = state.position.saturating_sub(state.start) as i128;
        let target = match whence {
            "start" => offset,
            "current" => current + offset,
            "end" => {
                let Some(end) = state.end else {
                    return Err(savvy::Error::new(
                        "cannot seek from end without known iterator size",
                    ));
                };
                end.saturating_sub(state.start) as i128 + offset
            }
            _ => return Err(savvy::Error::new("whence must be start, current, or end")),
        };
        if target < 0 {
            return Err(savvy::Error::new("seek target cannot be negative"));
        }
        if let Some(end) = state.end {
            let len = end.saturating_sub(state.start) as i128;
            if target > len {
                return Err(savvy::Error::new("seek target is beyond iterator end"));
            }
        }
        if target > u64::MAX as i128 {
            return Err(savvy::Error::new("seek target is too large"));
        }
        state.position = state.start.saturating_add(target as u64);
        state.done = state.end.is_some_and(|end| state.position >= end);
        real_scalar(target as f64)?.into()
    }
}

/// Chunked write sink for one object.
/// @export
#[savvy]
pub struct OpendalWriteIter {
    runtime: Arc<tokio::runtime::Runtime>,
    path: String,
    operation: String,
    sink: Mutex<Option<FuturesBytesSink>>,
    position: Mutex<u64>,
}

impl OpendalWriteIter {
    pub(crate) fn new(
        inner: Arc<NativeFs>,
        path: String,
        create_only: bool,
        append: bool,
        tuning: WriteTuning,
        operation: &str,
    ) -> savvy::Result<Self> {
        let op: Operator = inner.op.clone();
        let path_for_error = path.clone();
        let sink = inner.runtime.block_on(async move {
            let mut req = op.writer_with(&path);
            if create_only {
                req = req.if_not_exists(true);
            }
            if append {
                req = req.append(true);
            }
            if let Some(concurrent) = tuning.write_concurrency {
                req = req.concurrent(concurrent);
            }
            if let Some(chunk_size) = tuning.chunk_size {
                req = req.chunk(chunk_size);
            }
            req.await.map(|writer| writer.into_bytes_sink())
        });
        match sink {
            Ok(sink) => Ok(Self {
                runtime: inner.runtime.clone(),
                path: path_for_error,
                operation: operation.to_string(),
                sink: Mutex::new(Some(sink)),
                position: Mutex::new(0),
            }),
            Err(e) => Err(savvy::Error::new(&format!(
                "cannot open write iterator for {}: {}",
                path_for_error, e
            ))),
        }
    }
}

#[savvy]
impl OpendalWriteIter {
    /// Submit one raw chunk to the sink.
    /// @export
    fn write(&self, data: Sexp) -> savvy::Result<savvy::Sexp> {
        let bytes = raw_from_sexp(data)?;
        let len = bytes.len() as u64;
        let mut sink = {
            let mut guard = self
                .sink
                .lock()
                .map_err(|_| savvy::Error::new("write iterator lock poisoned"))?;
            match guard.take() {
                Some(sink) => sink,
                None => {
                    return error_list(
                        "InvalidState",
                        1,
                        "write iterator is already closed",
                        &self.operation,
                        &self.path,
                    );
                }
            }
        };

        let result = self
            .runtime
            .block_on(async { sink.send(Bytes::from(bytes)).await });

        let mut guard = self
            .sink
            .lock()
            .map_err(|_| savvy::Error::new("write iterator lock poisoned"))?;
        *guard = Some(sink);

        match result {
            Ok(_) => {
                let mut position = self
                    .position
                    .lock()
                    .map_err(|_| savvy::Error::new("write iterator position lock poisoned"))?;
                *position = position.saturating_add(len);
                success_value()
            }
            Err(e) => error_list("Unexpected", 1, &e.to_string(), &self.operation, &self.path),
        }
    }

    /// Finalize the sink.
    /// @export
    fn close(&self) -> savvy::Result<savvy::Sexp> {
        let mut sink = {
            let mut guard = self
                .sink
                .lock()
                .map_err(|_| savvy::Error::new("write iterator lock poisoned"))?;
            match guard.take() {
                Some(sink) => sink,
                None => return success_value(),
            }
        };
        match self.runtime.block_on(async { sink.close().await }) {
            Ok(_) => success_value(),
            Err(e) => error_list("Unexpected", 1, &e.to_string(), &self.operation, &self.path),
        }
    }

    /// Return the number of bytes submitted to this write sink.
    /// @export
    fn tell(&self) -> savvy::Result<savvy::Sexp> {
        let position = self
            .position
            .lock()
            .map_err(|_| savvy::Error::new("write iterator position lock poisoned"))?;
        real_scalar(*position as f64)?.into()
    }
}

fn iter_chunk(done: bool, bytes: Buffer) -> savvy::Result<savvy::Sexp> {
    let mut out = OwnedListSexp::new(2, true)?;
    out.set_name_and_value(0, "done", bool_scalar(done)?)?;
    out.set_name_and_value(1, "data", buffer_to_raw_sexp(bytes)?)?;
    out.into()
}

fn raw_from_sexp(data: Sexp) -> savvy::Result<Vec<u8>> {
    match data.into_typed() {
        TypedSexp::Raw(raw) => Ok(raw.to_vec()),
        _ => Err(savvy::Error::new("data must be a raw vector")),
    }
}

pub(crate) fn normalize_iter_path(path: &str, operation: &str) -> savvy::Result<String> {
    normalize_user_path(path, false).map_err(|e| savvy::Error::new(&format!("{operation}: {e}")))
}

pub(crate) fn checked_chunk_size(value: f64, name: &str) -> savvy::Result<u64> {
    let value = crate::path::checked_u64(value, name)?;
    if value == 0 {
        Err(savvy::Error::new(&format!(
            "{name} must be greater than zero"
        )))
    } else {
        Ok(value)
    }
}

fn checked_i128(value: f64, name: &str) -> savvy::Result<i128> {
    if !value.is_finite() || value.fract() != 0.0 {
        return Err(savvy::Error::new(&format!("{name} must be a whole number")));
    }
    if value < i64::MIN as f64 || value > i64::MAX as f64 {
        return Err(savvy::Error::new(&format!("{name} is too large")));
    }
    Ok(value as i128)
}
