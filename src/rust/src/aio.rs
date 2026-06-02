use std::sync::{Arc, Mutex};

use opendal::{Buffer, ErrorKind, Metadata};
use savvy::{NullSexp, savvy};
use tokio::task::JoinHandle;

use crate::bytes::opendal_bytes_to_sexp;
use crate::error::{error_list, kind_code};
use crate::metadata::metadata_list;
use crate::r_values::{bool_scalar, buffer_to_raw_sexp, str_scalar, success_value};

#[derive(Clone)]
pub(crate) struct EntryOutcome {
    pub(crate) path: String,
    pub(crate) meta: Metadata,
}

#[derive(Clone)]
pub(crate) enum AioOutcome {
    Bytes(Buffer),
    BytesHandle(Buffer),
    Unit,
    Bool(bool),
    Metadata {
        path: String,
        meta: Metadata,
    },
    Entries(Vec<EntryOutcome>),
    Many(Vec<AioOutcome>),
    Error {
        kind: String,
        code: i32,
        message: String,
        operation: String,
        path: String,
    },
    Cancelled,
}

fn outcome_to_sexp(outcome: AioOutcome) -> savvy::Result<savvy::Sexp> {
    match outcome {
        AioOutcome::Bytes(bytes) => buffer_to_raw_sexp(bytes).map(|x| x.into()),
        AioOutcome::BytesHandle(bytes) => opendal_bytes_to_sexp(bytes),
        AioOutcome::Unit => success_value(),
        AioOutcome::Bool(value) => bool_scalar(value)?.into(),
        AioOutcome::Metadata { path, meta } => metadata_list(&path, &meta),
        AioOutcome::Entries(entries) => {
            let mut out = savvy::OwnedListSexp::new(entries.len(), false)?;
            for (i, entry) in entries.into_iter().enumerate() {
                out.set_value(i, metadata_list(&entry.path, &entry.meta)?)?;
            }
            out.into()
        }
        AioOutcome::Many(values) => {
            let mut out = savvy::OwnedListSexp::new(values.len(), false)?;
            for (i, value) in values.into_iter().enumerate() {
                out.set_value(i, outcome_to_sexp(value)?)?;
            }
            out.into()
        }
        AioOutcome::Error {
            kind,
            code,
            message,
            operation,
            path,
        } => error_list(&kind, code, &message, &operation, &path),
        AioOutcome::Cancelled => error_list("Cancelled", 13, "operation cancelled", "aio", ""),
    }
}

fn outcome_error_to_sexp(outcome: AioOutcome) -> savvy::Result<savvy::Sexp> {
    match outcome {
        AioOutcome::Error {
            kind,
            code,
            message,
            operation,
            path,
        } => error_list(&kind, code, &message, &operation, &path),
        AioOutcome::Cancelled => error_list("Cancelled", 13, "operation cancelled", "aio", ""),
        _ => Ok(NullSexp.into()),
    }
}

fn outcome_state(outcome: &AioOutcome) -> &'static str {
    match outcome {
        AioOutcome::Error { .. } => "error",
        AioOutcome::Cancelled => "cancelled",
        _ => "resolved",
    }
}

fn join_outcome(
    runtime: &tokio::runtime::Runtime,
    handle: Option<JoinHandle<AioOutcome>>,
) -> AioOutcome {
    match handle {
        Some(handle) => match runtime.block_on(handle) {
            Ok(outcome) => outcome,
            Err(e) if e.is_cancelled() => AioOutcome::Cancelled,
            Err(e) => AioOutcome::Error {
                kind: "Unexpected".to_string(),
                code: kind_code(ErrorKind::Unexpected),
                message: e.to_string(),
                operation: "aio".to_string(),
                path: String::new(),
            },
        },
        None => AioOutcome::Cancelled,
    }
}

struct AioState {
    handle: Option<JoinHandle<AioOutcome>>,
    cached: Option<AioOutcome>,
}

struct AioInner {
    runtime: Arc<tokio::runtime::Runtime>,
    state: Mutex<AioState>,
}

/// Asynchronous operation handle.
/// @export
#[savvy]
pub struct OpendalAio {
    inner: Arc<AioInner>,
}

impl OpendalAio {
    pub(crate) fn new(
        runtime: Arc<tokio::runtime::Runtime>,
        handle: JoinHandle<AioOutcome>,
    ) -> Self {
        Self {
            inner: Arc::new(AioInner {
                runtime,
                state: Mutex::new(AioState {
                    handle: Some(handle),
                    cached: None,
                }),
            }),
        }
    }
}

#[savvy]
impl OpendalAio {
    /// Return pending or ready.
    /// @export
    fn poll(&self) -> savvy::Result<savvy::Sexp> {
        let state = self
            .inner
            .state
            .lock()
            .map_err(|_| savvy::Error::new("aio lock poisoned"))?;
        if state.cached.is_some() {
            return str_scalar("ready")?.into();
        }
        match &state.handle {
            Some(handle) if handle.is_finished() => str_scalar("ready")?.into(),
            Some(_) => str_scalar("pending")?.into(),
            None => str_scalar("ready")?.into(),
        }
    }

    /// Return detailed readiness/materialization state.
    /// @export
    fn state_name(&self) -> savvy::Result<savvy::Sexp> {
        let state = self
            .inner
            .state
            .lock()
            .map_err(|_| savvy::Error::new("aio lock poisoned"))?;
        if let Some(cached) = &state.cached {
            return str_scalar(outcome_state(cached))?.into();
        }
        match &state.handle {
            Some(handle) if handle.is_finished() => str_scalar("ready")?.into(),
            Some(_) => str_scalar("pending")?.into(),
            None => str_scalar("cancelled")?.into(),
        }
    }

    /// Return error value if resolved with an error, otherwise NULL.
    /// @export
    fn error_value(&self) -> savvy::Result<savvy::Sexp> {
        if let Some(cached) = self
            .inner
            .state
            .lock()
            .map_err(|_| savvy::Error::new("aio lock poisoned"))?
            .cached
            .clone()
        {
            return outcome_error_to_sexp(cached);
        }

        let is_ready = {
            let state = self
                .inner
                .state
                .lock()
                .map_err(|_| savvy::Error::new("aio lock poisoned"))?;
            match &state.handle {
                Some(handle) => handle.is_finished(),
                None => true,
            }
        };
        if !is_ready {
            return Ok(NullSexp.into());
        }

        let handle = {
            let mut state = self
                .inner
                .state
                .lock()
                .map_err(|_| savvy::Error::new("aio lock poisoned"))?;
            state.handle.take()
        };
        let outcome = join_outcome(&self.inner.runtime, handle);
        {
            let mut state = self
                .inner
                .state
                .lock()
                .map_err(|_| savvy::Error::new("aio lock poisoned"))?;
            state.cached = Some(outcome.clone());
        }
        outcome_error_to_sexp(outcome)
    }

    /// Collect the operation result.
    /// @export
    fn collect(&self) -> savvy::Result<savvy::Sexp> {
        if let Some(cached) = self
            .inner
            .state
            .lock()
            .map_err(|_| savvy::Error::new("aio lock poisoned"))?
            .cached
            .clone()
        {
            return outcome_to_sexp(cached);
        }

        let handle = {
            let mut state = self
                .inner
                .state
                .lock()
                .map_err(|_| savvy::Error::new("aio lock poisoned"))?;
            state.handle.take()
        };

        let outcome = join_outcome(&self.inner.runtime, handle);

        {
            let mut state = self
                .inner
                .state
                .lock()
                .map_err(|_| savvy::Error::new("aio lock poisoned"))?;
            state.cached = Some(outcome.clone());
        }
        outcome_to_sexp(outcome)
    }

    /// Request cancellation.
    /// @export
    fn cancel(&self) -> savvy::Result<savvy::Sexp> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| savvy::Error::new("aio lock poisoned"))?;
        if let Some(handle) = &state.handle {
            handle.abort();
        }
        if state.cached.is_none() {
            state.cached = Some(AioOutcome::Cancelled);
        }
        success_value()
    }
}
