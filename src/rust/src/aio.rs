use std::sync::{Arc, Mutex};

use opendal::{Buffer, ErrorKind};
use savvy::savvy;
use tokio::task::JoinHandle;

use crate::error::{error_list, kind_code};
use crate::r_values::{buffer_to_raw_sexp, str_scalar, success_value};

#[derive(Clone)]
pub(crate) enum AioOutcome {
    Bytes(Buffer),
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

        let outcome = match handle {
            Some(handle) => match self.inner.runtime.block_on(handle) {
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
        };

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
