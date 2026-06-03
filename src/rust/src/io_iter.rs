use std::sync::{Arc, Mutex};

use bytes::Bytes;
use futures::{SinkExt, TryStreamExt};
use opendal::{Buffer, FuturesBytesSink, Lister, Operator};
use savvy::savvy;
use savvy::{OwnedListSexp, Sexp, TypedSexp};
use tokio::sync::mpsc;

use crate::common::NativeFs;
use crate::error::{error_list, kind_code, op_error_list};
use crate::metadata::metadata_list;
use crate::ops::{ReadTuning, WriteTuning, read_bytes_with};
use crate::path::normalize_user_path;
use crate::r_values::{
    bool_scalar, buffer_to_raw_sexp, buffers_to_raw_sexp, real_scalar, set_str_or_null,
    success_value,
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

/// Streaming listing iterator over one prefix.
/// @export
#[savvy]
pub struct OpendalLsIter {
    runtime: Arc<tokio::runtime::Runtime>,
    path: String,
    operation: String,
    lister: Mutex<Option<Lister>>,
    prefetch_rx: Mutex<Option<mpsc::Receiver<PrefetchItem>>>,
    initial_error: Mutex<Option<InitialError>>,
    done: Mutex<bool>,
    page_size: usize,
    remaining: Mutex<Option<usize>>,
    start_after: Option<String>,
    cursor: Mutex<Option<String>>,
}

struct InitialError {
    kind: String,
    code: i32,
    message: String,
}

enum PrefetchItem {
    Entry(String, opendal::Metadata),
    Error(InitialError),
}

impl OpendalLsIter {
    pub(crate) fn new(
        inner: Arc<NativeFs>,
        path: String,
        recursive: bool,
        page_size: usize,
        limit: Option<usize>,
        start_after: Option<String>,
        prefetch: usize,
        operation: &str,
    ) -> savvy::Result<Self> {
        let mut opts = opendal::options::ListOptions::default();
        opts.recursive = recursive;
        opts.limit = Some(
            limit
                .filter(|value| *value > 0)
                .map_or(page_size, |value| page_size.min(value)),
        );
        opts.start_after = start_after.clone();
        let path_for_error = path.clone();
        let runtime = inner.runtime.clone();
        let op = inner.op.clone();
        let lister_path = path.clone();
        let lister = runtime.block_on(async move { op.lister_options(&lister_path, opts).await });
        let (lister, prefetch_rx, initial_error) = match lister {
            Ok(lister) => {
                if prefetch > 0 && limit != Some(0) {
                    let (tx, rx) = mpsc::channel(prefetch);
                    let root_path = path.clone();
                    let start_after_for_task = start_after.clone();
                    let mut remaining = limit;
                    runtime.spawn(async move {
                        let mut lister = lister;
                        loop {
                            if remaining.is_some_and(|value| value == 0) {
                                break;
                            }
                            match lister.try_next().await {
                                Ok(Some(entry)) => {
                                    let entry_path = entry.path();
                                    if keep_listing_entry(
                                        &root_path,
                                        start_after_for_task.as_deref(),
                                        entry_path,
                                    ) {
                                        if tx
                                            .send(PrefetchItem::Entry(
                                                entry_path.to_string(),
                                                entry.metadata().clone(),
                                            ))
                                            .await
                                            .is_err()
                                        {
                                            break;
                                        }
                                        if let Some(value) = remaining.as_mut() {
                                            *value = value.saturating_sub(1);
                                        }
                                    }
                                }
                                Ok(None) => break,
                                Err(e) => {
                                    let _ = tx
                                        .send(PrefetchItem::Error(InitialError {
                                            kind: e.kind().to_string(),
                                            code: kind_code(e.kind()),
                                            message: e.to_string(),
                                        }))
                                        .await;
                                    break;
                                }
                            }
                        }
                    });
                    (None, Some(rx), None)
                } else {
                    (Some(lister), None, None)
                }
            }
            Err(e) => (
                None,
                None,
                Some(InitialError {
                    kind: e.kind().to_string(),
                    code: kind_code(e.kind()),
                    message: e.to_string(),
                }),
            ),
        };
        Ok(Self {
            runtime,
            path: path_for_error,
            operation: operation.to_string(),
            lister: Mutex::new(lister),
            prefetch_rx: Mutex::new(prefetch_rx),
            initial_error: Mutex::new(initial_error),
            done: Mutex::new(false),
            page_size,
            remaining: Mutex::new(limit),
            start_after,
            cursor: Mutex::new(None),
        })
    }

    fn keep_entry(&self, path: &str) -> bool {
        keep_listing_entry(&self.path, self.start_after.as_deref(), path)
    }

    fn has_prefetch(&self) -> savvy::Result<bool> {
        self.prefetch_rx
            .lock()
            .map(|guard| guard.is_some())
            .map_err(|_| savvy::Error::new("listing iterator prefetch lock poisoned"))
    }

    fn recv_prefetch(&self) -> savvy::Result<Option<PrefetchItem>> {
        let mut guard = self
            .prefetch_rx
            .lock()
            .map_err(|_| savvy::Error::new("listing iterator prefetch lock poisoned"))?;
        let Some(rx) = guard.as_mut() else {
            return Ok(None);
        };
        let item = self.runtime.block_on(rx.recv());
        if item.is_none() {
            *guard = None;
        }
        Ok(item)
    }

    fn next_prefetched_page(&self, target: usize) -> savvy::Result<savvy::Sexp> {
        let mut entries = Vec::new();
        let mut exhausted = false;
        while entries.len() < target {
            match self.recv_prefetch()? {
                Some(PrefetchItem::Entry(path, meta)) => entries.push((path, meta)),
                Some(PrefetchItem::Error(error)) => {
                    *self
                        .done
                        .lock()
                        .map_err(|_| savvy::Error::new("listing iterator done lock poisoned"))? =
                        true;
                    return error_list(
                        &error.kind,
                        error.code,
                        &error.message,
                        &self.operation,
                        &self.path,
                    );
                }
                None => {
                    exhausted = true;
                    break;
                }
            }
        }

        let limit_reached = {
            let mut remaining = self
                .remaining
                .lock()
                .map_err(|_| savvy::Error::new("listing iterator limit lock poisoned"))?;
            if let Some(value) = remaining.as_mut() {
                *value = value.saturating_sub(entries.len());
                *value == 0
            } else {
                false
            }
        };
        let done_now = exhausted || limit_reached;
        if done_now {
            *self
                .done
                .lock()
                .map_err(|_| savvy::Error::new("listing iterator done lock poisoned"))? = true;
        }

        let cursor = if let Some((path, _)) = entries.last() {
            let cursor = path.clone();
            *self
                .cursor
                .lock()
                .map_err(|_| savvy::Error::new("listing iterator cursor lock poisoned"))? =
                Some(cursor.clone());
            Some(cursor)
        } else {
            self.cursor
                .lock()
                .map_err(|_| savvy::Error::new("listing iterator cursor lock poisoned"))?
                .clone()
        };

        if done_now && entries.is_empty() {
            iter_entries_page(true, Vec::new(), cursor.as_deref())
        } else {
            iter_entries_page(false, entries, cursor.as_deref())
        }
    }

    fn collect_prefetched(&self) -> savvy::Result<savvy::Sexp> {
        let mut all = Vec::new();
        loop {
            if *self
                .done
                .lock()
                .map_err(|_| savvy::Error::new("listing iterator done lock poisoned"))?
            {
                break;
            }
            if self
                .remaining
                .lock()
                .map_err(|_| savvy::Error::new("listing iterator limit lock poisoned"))?
                .is_some_and(|value| value == 0)
            {
                *self
                    .done
                    .lock()
                    .map_err(|_| savvy::Error::new("listing iterator done lock poisoned"))? = true;
                break;
            }

            match self.recv_prefetch()? {
                Some(PrefetchItem::Entry(path, meta)) => {
                    all.push((path, meta));
                    let limit_reached = {
                        let mut remaining = self.remaining.lock().map_err(|_| {
                            savvy::Error::new("listing iterator limit lock poisoned")
                        })?;
                        if let Some(value) = remaining.as_mut() {
                            *value = value.saturating_sub(1);
                            *value == 0
                        } else {
                            false
                        }
                    };
                    if limit_reached {
                        *self.done.lock().map_err(|_| {
                            savvy::Error::new("listing iterator done lock poisoned")
                        })? = true;
                        break;
                    }
                }
                Some(PrefetchItem::Error(error)) => {
                    *self
                        .done
                        .lock()
                        .map_err(|_| savvy::Error::new("listing iterator done lock poisoned"))? =
                        true;
                    return error_list(
                        &error.kind,
                        error.code,
                        &error.message,
                        &self.operation,
                        &self.path,
                    );
                }
                None => {
                    *self
                        .done
                        .lock()
                        .map_err(|_| savvy::Error::new("listing iterator done lock poisoned"))? =
                        true;
                    break;
                }
            }
        }
        if let Some((path, _)) = all.last() {
            *self
                .cursor
                .lock()
                .map_err(|_| savvy::Error::new("listing iterator cursor lock poisoned"))? =
                Some(path.clone());
        }
        entries_list(all)
    }
}

fn keep_listing_entry(root_path: &str, start_after: Option<&str>, path: &str) -> bool {
    path != "/"
        && !path.is_empty()
        && path != root_path
        && start_after.is_none_or(|start_after| path > start_after)
}

#[savvy]
impl OpendalLsIter {
    /// Return the next listing page as list(done, entries, cursor).
    /// @export
    fn next(&self) -> savvy::Result<savvy::Sexp> {
        if *self
            .done
            .lock()
            .map_err(|_| savvy::Error::new("listing iterator done lock poisoned"))?
        {
            let cursor = self
                .cursor
                .lock()
                .map_err(|_| savvy::Error::new("listing iterator cursor lock poisoned"))?
                .clone();
            return iter_entries_page(true, Vec::new(), cursor.as_deref());
        }
        if let Some(error) = self
            .initial_error
            .lock()
            .map_err(|_| savvy::Error::new("listing iterator error lock poisoned"))?
            .take()
        {
            *self
                .done
                .lock()
                .map_err(|_| savvy::Error::new("listing iterator done lock poisoned"))? = true;
            return error_list(
                &error.kind,
                error.code,
                &error.message,
                &self.operation,
                &self.path,
            );
        }

        let target = {
            let remaining = self
                .remaining
                .lock()
                .map_err(|_| savvy::Error::new("listing iterator limit lock poisoned"))?;
            match *remaining {
                Some(0) => {
                    *self
                        .done
                        .lock()
                        .map_err(|_| savvy::Error::new("listing iterator done lock poisoned"))? =
                        true;
                    let cursor = self
                        .cursor
                        .lock()
                        .map_err(|_| savvy::Error::new("listing iterator cursor lock poisoned"))?
                        .clone();
                    return iter_entries_page(true, Vec::new(), cursor.as_deref());
                }
                Some(value) => self.page_size.min(value),
                None => self.page_size,
            }
        };

        if self.has_prefetch()? {
            return self.next_prefetched_page(target);
        }

        let mut lister = {
            let mut guard = self
                .lister
                .lock()
                .map_err(|_| savvy::Error::new("listing iterator lock poisoned"))?;
            match guard.take() {
                Some(lister) => lister,
                None => {
                    *self
                        .done
                        .lock()
                        .map_err(|_| savvy::Error::new("listing iterator done lock poisoned"))? =
                        true;
                    let cursor = self
                        .cursor
                        .lock()
                        .map_err(|_| savvy::Error::new("listing iterator cursor lock poisoned"))?
                        .clone();
                    return iter_entries_page(true, Vec::new(), cursor.as_deref());
                }
            }
        };

        let mut entries = Vec::new();
        let mut exhausted = false;
        while entries.len() < target {
            match self.runtime.block_on(lister.try_next()) {
                Ok(Some(entry)) => {
                    if self.keep_entry(entry.path()) {
                        entries.push((entry.path().to_string(), entry.metadata().clone()));
                    }
                }
                Ok(None) => {
                    exhausted = true;
                    break;
                }
                Err(e) => {
                    let mut guard = self
                        .lister
                        .lock()
                        .map_err(|_| savvy::Error::new("listing iterator lock poisoned"))?;
                    *guard = Some(lister);
                    return op_error_list(e, &self.operation, &self.path);
                }
            }
        }

        let limit_reached = {
            let mut remaining = self
                .remaining
                .lock()
                .map_err(|_| savvy::Error::new("listing iterator limit lock poisoned"))?;
            if let Some(value) = remaining.as_mut() {
                *value = value.saturating_sub(entries.len());
                *value == 0
            } else {
                false
            }
        };
        let done_now = exhausted || limit_reached;
        if done_now {
            *self
                .done
                .lock()
                .map_err(|_| savvy::Error::new("listing iterator done lock poisoned"))? = true;
        } else {
            let mut guard = self
                .lister
                .lock()
                .map_err(|_| savvy::Error::new("listing iterator lock poisoned"))?;
            *guard = Some(lister);
        }

        let cursor = if let Some((path, _)) = entries.last() {
            let cursor = path.clone();
            *self
                .cursor
                .lock()
                .map_err(|_| savvy::Error::new("listing iterator cursor lock poisoned"))? =
                Some(cursor.clone());
            Some(cursor)
        } else {
            self.cursor
                .lock()
                .map_err(|_| savvy::Error::new("listing iterator cursor lock poisoned"))?
                .clone()
        };

        if done_now && entries.is_empty() {
            iter_entries_page(true, Vec::new(), cursor.as_deref())
        } else {
            iter_entries_page(false, entries, cursor.as_deref())
        }
    }

    /// Collect all remaining entries.
    /// @export
    fn collect(&self) -> savvy::Result<savvy::Sexp> {
        if let Some(error) = self
            .initial_error
            .lock()
            .map_err(|_| savvy::Error::new("listing iterator error lock poisoned"))?
            .take()
        {
            *self
                .done
                .lock()
                .map_err(|_| savvy::Error::new("listing iterator done lock poisoned"))? = true;
            return error_list(
                &error.kind,
                error.code,
                &error.message,
                &self.operation,
                &self.path,
            );
        }

        if self.has_prefetch()? {
            return self.collect_prefetched();
        }

        let mut all = Vec::new();
        loop {
            if *self
                .done
                .lock()
                .map_err(|_| savvy::Error::new("listing iterator done lock poisoned"))?
            {
                break;
            }
            if self
                .remaining
                .lock()
                .map_err(|_| savvy::Error::new("listing iterator limit lock poisoned"))?
                .is_some_and(|value| value == 0)
            {
                *self
                    .done
                    .lock()
                    .map_err(|_| savvy::Error::new("listing iterator done lock poisoned"))? = true;
                break;
            }

            let mut lister = {
                let mut guard = self
                    .lister
                    .lock()
                    .map_err(|_| savvy::Error::new("listing iterator lock poisoned"))?;
                match guard.take() {
                    Some(lister) => lister,
                    None => break,
                }
            };

            let result = self.runtime.block_on(lister.try_next());
            match result {
                Ok(Some(entry)) => {
                    let mut limit_reached = false;
                    if self.keep_entry(entry.path()) {
                        all.push((entry.path().to_string(), entry.metadata().clone()));
                        let mut remaining = self.remaining.lock().map_err(|_| {
                            savvy::Error::new("listing iterator limit lock poisoned")
                        })?;
                        if let Some(value) = remaining.as_mut() {
                            *value = value.saturating_sub(1);
                            limit_reached = *value == 0;
                        }
                    }
                    if limit_reached {
                        *self.done.lock().map_err(|_| {
                            savvy::Error::new("listing iterator done lock poisoned")
                        })? = true;
                        break;
                    }
                    let mut guard = self
                        .lister
                        .lock()
                        .map_err(|_| savvy::Error::new("listing iterator lock poisoned"))?;
                    *guard = Some(lister);
                }
                Ok(None) => {
                    *self
                        .done
                        .lock()
                        .map_err(|_| savvy::Error::new("listing iterator done lock poisoned"))? =
                        true;
                    break;
                }
                Err(e) => {
                    let mut guard = self
                        .lister
                        .lock()
                        .map_err(|_| savvy::Error::new("listing iterator lock poisoned"))?;
                    *guard = Some(lister);
                    return op_error_list(e, &self.operation, &self.path);
                }
            }
        }
        if let Some((path, _)) = all.last() {
            *self
                .cursor
                .lock()
                .map_err(|_| savvy::Error::new("listing iterator cursor lock poisoned"))? =
                Some(path.clone());
        }
        entries_list(all)
    }
}

fn iter_entries_page(
    done: bool,
    entries: Vec<(String, opendal::Metadata)>,
    cursor: Option<&str>,
) -> savvy::Result<savvy::Sexp> {
    let mut out = OwnedListSexp::new(3, true)?;
    out.set_name_and_value(0, "done", bool_scalar(done)?)?;
    out.set_name_and_value(1, "entries", entries_list(entries)?)?;
    set_str_or_null(&mut out, 2, "cursor", cursor)?;
    out.into()
}

fn entries_list(entries: Vec<(String, opendal::Metadata)>) -> savvy::Result<savvy::Sexp> {
    let mut out = OwnedListSexp::new(entries.len(), false)?;
    for (i, (path, meta)) in entries.into_iter().enumerate() {
        out.set_value(i, metadata_list(&path, &meta)?)?;
    }
    out.into()
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
