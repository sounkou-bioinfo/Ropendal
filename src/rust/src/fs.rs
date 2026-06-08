use std::sync::Arc;
use std::time::Duration;

use futures::{StreamExt, stream};
use opendal::layers::{ConcurrentLimitLayer, TimeoutLayer};
use opendal::options::{DeleteOptions, ListOptions};
use opendal::{Buffer, Metadata, Operator};
use savvy::savvy;
use savvy::{ListSexp, OwnedListSexp, Sexp, StringSexp, TypedSexp};

use crate::aio::{AioOutcome, EntryOutcome, OpendalAio};
use crate::bytes::{buffer_from_opendal_bytes_sexp, opendal_bytes_to_sexp};
use crate::common::{NativeFs, build_runtime, init_registry};
use crate::error::{kind_code, op_error_list, unsupported_value};
use crate::http_headers::apply_http_headers;
use crate::io_iter::{
    OpendalLsIter, OpendalReadIter, OpendalWriteIter, checked_chunk_size, normalize_iter_path,
};
use crate::metadata::metadata_list;
use crate::ops::{ReadTuning, WriteTuning, read_bytes_with, write_bytes_with};
use crate::path::{checked_u64, normalize_user_path};
use crate::r_values::{bool_scalar, buffer_to_raw_sexp, str_scalar, success_value};

/// Filesystem handle backed by Apache OpenDAL.
/// @export
#[savvy]
pub struct OpendalFs {
    pub(crate) inner: Arc<NativeFs>,
}

#[savvy]
impl OpendalFs {
    /// Open an OpenDAL filesystem from a scheme and named config lists.
    /// @export
    fn open(
        scheme: &str,
        dots: ListSexp,
        config: ListSexp,
        root: Option<&str>,
        auth_config: Option<ListSexp>,
        headers: Option<ListSexp>,
        runtime_threads: Option<f64>,
        max_inflight: Option<f64>,
        timeout_seconds: Option<f64>,
        io_timeout_seconds: Option<f64>,
    ) -> savvy::Result<Self> {
        let mut pairs = Vec::new();
        let headers = header_pairs_from_list(headers)?;
        append_named_config(&mut pairs, config, "config")?;
        if let Some(auth_config) = auth_config {
            append_named_config(&mut pairs, auth_config, "auth")?;
        }
        append_named_config(&mut pairs, dots, "...")?;
        if let Some(root) = root {
            if !root.is_empty() {
                pairs.push(("root".to_string(), root.to_string()));
            }
        }
        Self::open_from_pairs(
            scheme,
            pairs,
            headers,
            runtime_threads,
            max_inflight,
            timeout_seconds,
            io_timeout_seconds,
        )
    }

    /// Open an OpenDAL filesystem from a URI.
    /// @export
    fn from_uri(
        uri: &str,
        headers: Option<ListSexp>,
        runtime_threads: Option<f64>,
        max_inflight: Option<f64>,
        timeout_seconds: Option<f64>,
        io_timeout_seconds: Option<f64>,
    ) -> savvy::Result<Self> {
        init_registry();
        let headers = header_pairs_from_list(headers)?;
        let runtime_threads = checked_positive_usize(runtime_threads, "runtime_threads")?;
        let max_inflight = checked_positive_usize(max_inflight, "max_inflight")?;
        let request_timeout = checked_positive_duration(timeout_seconds, "request_timeout")?;
        let io_timeout = checked_positive_duration(io_timeout_seconds, "io_timeout")?;
        let op = Operator::from_uri(uri)
            .map_err(|e| savvy::Error::new(&format!("cannot open OpenDAL URI: {e}")))?;
        let op = apply_http_headers(op, headers)?;
        let op = apply_timeout_layer(op, request_timeout, io_timeout);
        let op = apply_concurrent_limit(op, max_inflight);
        let info = op.info();
        let native = NativeFs {
            op,
            runtime: build_runtime(runtime_threads)?,
            scheme: info.scheme().to_string(),
            root: info.root(),
        };
        Ok(Self {
            inner: Arc::new(native),
        })
    }

    /// Return filesystem identity.
    /// @export
    fn info(&self) -> savvy::Result<savvy::Sexp> {
        let info = self.inner.op.info();
        let mut out = OwnedListSexp::new(4, true)?;
        out.set_name_and_value(0, "scheme", str_scalar(info.scheme())?)?;
        out.set_name_and_value(1, "root", str_scalar(&info.root())?)?;
        out.set_name_and_value(2, "name", str_scalar(&info.name())?)?;
        out.set_name_and_value(3, "package", str_scalar("Ropendal")?)?;
        out.into()
    }

    /// Return selected capability flags.
    /// @export
    fn capabilities(&self) -> savvy::Result<savvy::Sexp> {
        let caps = self.inner.op.info().full_capability();
        let ops = [
            ("stat", caps.stat, "native", "metadata lookup"),
            ("read", caps.read, "native", "byte read"),
            (
                "read_range",
                caps.read,
                "native",
                "offset/size range reads via fs_read()",
            ),
            (
                "read_concurrent",
                caps.read,
                "opendal_adapter",
                "per-object read concurrency where supported",
            ),
            ("write", caps.write, "native", "create-only byte write"),
            (
                "write_concurrent",
                caps.write,
                "opendal_adapter",
                "per-object write concurrency where supported",
            ),
            ("replace", caps.write, "native", "overwrite byte write"),
            (
                "append",
                caps.write && caps.write_can_append,
                "native",
                "append if backend supports append writes",
            ),
            ("mkdir", caps.create_dir, "native", "directory creation"),
            ("delete", caps.delete, "native", "delete path"),
            (
                "delete_recursive",
                caps.delete,
                "native",
                "recursive delete option where supported",
            ),
            ("copy", caps.copy, "native", "backend copy"),
            (
                "rename",
                caps.rename,
                "native",
                "backend rename where supported",
            ),
            ("ls", caps.list, "native", "non-recursive listing"),
            (
                "list_recursive",
                caps.list,
                "native",
                "recursive listing/walk where supported",
            ),
        ];
        let mut operations = OwnedListSexp::new(ops.len(), true)?;
        for (i, (name, supported, semantics, notes)) in ops.iter().enumerate() {
            let implementation = if *supported { "opendal" } else { "unsupported" };
            let semantics = if *supported {
                *semantics
            } else {
                "unsupported"
            };
            let mut one = OwnedListSexp::new(4, true)?;
            one.set_name_and_value(0, "supported", bool_scalar(*supported)?)?;
            one.set_name_and_value(1, "implementation", str_scalar(implementation)?)?;
            one.set_name_and_value(2, "semantics", str_scalar(semantics)?)?;
            one.set_name_and_value(3, "notes", str_scalar(*notes)?)?;
            one.set_class(&["opendalCapabilityOperation", "list"])?;
            operations.set_name_and_value(i, name, one)?;
        }
        operations.set_class(&["opendalCapabilityOperations", "list"])?;
        let mut out = OwnedListSexp::new(3, true)?;
        out.set_name_and_value(0, "scheme", str_scalar(&self.inner.scheme)?)?;
        out.set_name_and_value(1, "root", str_scalar(&self.inner.root)?)?;
        out.set_name_and_value(2, "operations", operations)?;
        out.set_class(&["opendalCapabilityValue", "list"])?;
        out.into()
    }

    /// Normalize a root-relative path.
    /// @export
    fn normalize_path(&self, path: &str, directory: bool) -> savvy::Result<savvy::Sexp> {
        match normalize_user_path(path, directory) {
            Ok(p) => str_scalar(&p)?.into(),
            Err(e) => Err(savvy::Error::new(&e)),
        }
    }

    /// Read bytes from path(s)/range(s).
    /// @export
    fn read(
        &self,
        path: StringSexp,
        offset: Option<Sexp>,
        size: Option<Sexp>,
        end: Option<Sexp>,
        result: Option<&str>,
        batch_concurrency: Option<f64>,
        read_concurrency: Option<f64>,
        chunk_size: Option<f64>,
        coalesce_gap: Option<f64>,
    ) -> savvy::Result<savvy::Sexp> {
        let result = result.unwrap_or("auto");
        match result {
            "auto" | "flat" | "nested" => {}
            _ => return Err(savvy::Error::new("result must be auto, flat, or nested")),
        }
        let plan = read_requests_from_options(path, offset, size, end)?;
        let tuning = read_tuning(read_concurrency, chunk_size, coalesce_gap)?;
        self.read_requests(plan, result, batch_concurrency, tuning)
    }

    /// Read bytes into Rust-owned byte handle(s).
    /// @export
    fn read_bytes(
        &self,
        path: StringSexp,
        offset: Option<Sexp>,
        size: Option<Sexp>,
        end: Option<Sexp>,
        result: Option<&str>,
        batch_concurrency: Option<f64>,
        read_concurrency: Option<f64>,
        chunk_size: Option<f64>,
        coalesce_gap: Option<f64>,
    ) -> savvy::Result<savvy::Sexp> {
        let result = result.unwrap_or("auto");
        match result {
            "auto" | "flat" | "nested" => {}
            _ => return Err(savvy::Error::new("result must be auto, flat, or nested")),
        }
        let plan = read_requests_from_options(path, offset, size, end)?;
        let tuning = read_tuning(read_concurrency, chunk_size, coalesce_gap)?;
        self.read_bytes_requests(plan, result, batch_concurrency, tuning)
    }

    /// Submit asynchronous read(s) into Rust-owned byte handle(s).
    /// @export
    fn read_bytes_aio(
        &self,
        path: StringSexp,
        offset: Option<Sexp>,
        size: Option<Sexp>,
        end: Option<Sexp>,
        result: Option<&str>,
        batch_concurrency: Option<f64>,
        read_concurrency: Option<f64>,
        chunk_size: Option<f64>,
        coalesce_gap: Option<f64>,
    ) -> savvy::Result<OpendalAio> {
        let result = result.unwrap_or("auto");
        match result {
            "auto" | "flat" | "nested" => {}
            _ => return Err(savvy::Error::new("result must be auto, flat, or nested")),
        }
        let plan = read_requests_from_options(path, offset, size, end)?;
        let tuning = read_tuning(read_concurrency, chunk_size, coalesce_gap)?;
        let n = plan.requests.len();
        let concurrency = batch_concurrency_limit(batch_concurrency, n)?;
        let op = self.inner.op.clone();
        let result = result.to_string();
        let shape = plan.shape;
        let requests = plan.requests;
        let handle = self.inner.runtime.spawn(async move {
            let mut values = stream::iter(requests.into_iter())
                .map(|req| {
                    let op = op.clone();
                    let key = (req.path_index, req.range_index);
                    async move { (key, read_request_bytes_outcome(op, req, tuning).await) }
                })
                .buffer_unordered(concurrency)
                .collect::<Vec<_>>()
                .await;
            values.sort_by_key(|(key, _)| *key);
            read_outcomes_to_aio(
                values.into_iter().map(|(_, value)| value).collect(),
                &result,
                &shape,
            )
        });
        Ok(OpendalAio::new(self.inner.runtime.clone(), handle))
    }

    /// Submit asynchronous read(s).
    /// @export
    fn read_aio(
        &self,
        path: StringSexp,
        offset: Option<Sexp>,
        size: Option<Sexp>,
        end: Option<Sexp>,
        result: Option<&str>,
        batch_concurrency: Option<f64>,
        read_concurrency: Option<f64>,
        chunk_size: Option<f64>,
        coalesce_gap: Option<f64>,
    ) -> savvy::Result<OpendalAio> {
        let result = result.unwrap_or("auto");
        match result {
            "auto" | "flat" | "nested" => {}
            _ => return Err(savvy::Error::new("result must be auto, flat, or nested")),
        }
        let plan = read_requests_from_options(path, offset, size, end)?;
        let tuning = read_tuning(read_concurrency, chunk_size, coalesce_gap)?;
        let n = plan.requests.len();
        let concurrency = batch_concurrency_limit(batch_concurrency, n)?;
        let op = self.inner.op.clone();
        let result = result.to_string();
        let shape = plan.shape;
        let requests = plan.requests;
        let handle = self.inner.runtime.spawn(async move {
            let mut values = stream::iter(requests.into_iter())
                .map(|req| {
                    let op = op.clone();
                    let key = (req.path_index, req.range_index);
                    async move { (key, read_request_outcome(op, req, tuning).await) }
                })
                .buffer_unordered(concurrency)
                .collect::<Vec<_>>()
                .await;
            values.sort_by_key(|(key, _)| *key);
            read_outcomes_to_aio(
                values.into_iter().map(|(_, value)| value).collect(),
                &result,
                &shape,
            )
        });
        Ok(OpendalAio::new(self.inner.runtime.clone(), handle))
    }

    /// Create a chunked read iterator for one object.
    /// @export
    fn read_iter(
        &self,
        path: &str,
        chunk_size: f64,
        offset: Option<f64>,
        size: Option<f64>,
        read_concurrency: Option<f64>,
        coalesce_gap: Option<f64>,
    ) -> savvy::Result<OpendalReadIter> {
        let path = normalize_iter_path(path, "read_iter")?;
        let chunk_size = checked_chunk_size(chunk_size, "chunk_size")?;
        let offset = checked_u64(offset.unwrap_or(0.0), "offset")?;
        let size = if let Some(value) = size {
            Some(checked_u64(value, "size")?)
        } else {
            let op = self.inner.op.clone();
            let stat_path = path.clone();
            let meta = self
                .inner
                .runtime
                .block_on(async move { op.stat(&stat_path).await })
                .map_err(|e| {
                    savvy::Error::new(&format!("read_iter stat failed for {path}: {e}"))
                })?;
            Some(meta.content_length().saturating_sub(offset))
        };
        let tuning = read_tuning(read_concurrency, Some(chunk_size as f64), coalesce_gap)?;
        Ok(OpendalReadIter::new(
            self.inner.clone(),
            path,
            offset,
            size,
            chunk_size,
            tuning,
        ))
    }

    /// Create a chunked write sink for one object.
    /// @export
    fn write_iter(
        &self,
        path: &str,
        create: Option<bool>,
        append: Option<bool>,
        write_concurrency: Option<f64>,
        chunk_size: Option<f64>,
    ) -> savvy::Result<OpendalWriteIter> {
        let path = normalize_iter_path(path, "write_iter")?;
        let create = create.unwrap_or(true);
        let append = append.unwrap_or(false);
        if create && append {
            return Err(savvy::Error::new("create and append cannot both be true"));
        }
        let tuning = write_tuning(write_concurrency, chunk_size)?;
        OpendalWriteIter::new(
            self.inner.clone(),
            path,
            create,
            append,
            tuning,
            if append { "append_iter" } else { "write_iter" },
        )
    }

    /// Write bytes to new path(s).
    /// @export
    fn write(
        &self,
        path: StringSexp,
        data: Sexp,
        batch_concurrency: Option<f64>,
        write_concurrency: Option<f64>,
        chunk_size: Option<f64>,
    ) -> savvy::Result<savvy::Sexp> {
        let tuning = write_tuning(write_concurrency, chunk_size)?;
        self.write_many_common(path, data, true, false, "write", batch_concurrency, tuning)
    }

    /// Replace bytes at path(s).
    /// @export
    fn replace(
        &self,
        path: StringSexp,
        data: Sexp,
        batch_concurrency: Option<f64>,
        write_concurrency: Option<f64>,
        chunk_size: Option<f64>,
    ) -> savvy::Result<savvy::Sexp> {
        let tuning = write_tuning(write_concurrency, chunk_size)?;
        self.write_many_common(
            path,
            data,
            false,
            false,
            "replace",
            batch_concurrency,
            tuning,
        )
    }

    /// Append bytes to path(s).
    /// @export
    fn append(
        &self,
        path: StringSexp,
        data: Sexp,
        batch_concurrency: Option<f64>,
        write_concurrency: Option<f64>,
        chunk_size: Option<f64>,
    ) -> savvy::Result<savvy::Sexp> {
        let tuning = write_tuning(write_concurrency, chunk_size)?;
        self.write_many_common(path, data, false, true, "append", batch_concurrency, tuning)
    }

    /// Submit asynchronous write(s) to new path(s).
    /// @export
    fn write_aio(
        &self,
        path: StringSexp,
        data: Sexp,
        batch_concurrency: Option<f64>,
        write_concurrency: Option<f64>,
        chunk_size: Option<f64>,
    ) -> savvy::Result<OpendalAio> {
        let tuning = write_tuning(write_concurrency, chunk_size)?;
        self.write_many_aio_common(path, data, true, false, "write", batch_concurrency, tuning)
    }

    /// Submit asynchronous replacement write(s).
    /// @export
    fn replace_aio(
        &self,
        path: StringSexp,
        data: Sexp,
        batch_concurrency: Option<f64>,
        write_concurrency: Option<f64>,
        chunk_size: Option<f64>,
    ) -> savvy::Result<OpendalAio> {
        let tuning = write_tuning(write_concurrency, chunk_size)?;
        self.write_many_aio_common(
            path,
            data,
            false,
            false,
            "replace",
            batch_concurrency,
            tuning,
        )
    }

    /// Submit asynchronous append write(s).
    /// @export
    fn append_aio(
        &self,
        path: StringSexp,
        data: Sexp,
        batch_concurrency: Option<f64>,
        write_concurrency: Option<f64>,
        chunk_size: Option<f64>,
    ) -> savvy::Result<OpendalAio> {
        let tuning = write_tuning(write_concurrency, chunk_size)?;
        self.write_many_aio_common(path, data, false, true, "append", batch_concurrency, tuning)
    }

    /// Return metadata for path(s).
    /// @export
    fn stat(&self, path: StringSexp, batch_concurrency: Option<f64>) -> savvy::Result<savvy::Sexp> {
        let n = path.len();
        let concurrency = batch_concurrency_limit(batch_concurrency, n)?;
        if n == 0 {
            return OwnedListSexp::new(0, false)?.into();
        }
        if n == 1 {
            return self.stat_one(path.iter().next().unwrap_or(""));
        }

        let mut values: Vec<Option<StatValue>> = (0..n).map(|_| None).collect();
        let mut requests = Vec::new();
        for (i, p) in path.iter().enumerate() {
            match normalize_user_path(p, false) {
                Ok(p) => requests.push((i, p)),
                Err(e) => {
                    values[i] = Some(StatValue::Error {
                        kind: "InvalidArgument".to_string(),
                        code: 14,
                        message: e,
                        path: p.to_string(),
                    });
                }
            }
        }

        let op = self.inner.op.clone();
        let async_values = self.inner.runtime.block_on(async move {
            stream::iter(requests.into_iter())
                .map(|(i, p)| {
                    let op = op.clone();
                    async move { (i, stat_request(op, p).await) }
                })
                .buffer_unordered(concurrency)
                .collect::<Vec<_>>()
                .await
        });
        for (i, value) in async_values {
            values[i] = Some(value);
        }

        let mut out = OwnedListSexp::new(n, false)?;
        for (i, value) in values.into_iter().enumerate() {
            out.set_value(
                i,
                stat_value_to_sexp(value.unwrap_or(StatValue::Error {
                    kind: "Unexpected".to_string(),
                    code: 1,
                    message: "missing stat result".to_string(),
                    path: String::new(),
                }))?,
            )?;
        }
        out.into()
    }

    /// Submit asynchronous metadata request(s).
    /// @export
    fn stat_aio(
        &self,
        path: StringSexp,
        batch_concurrency: Option<f64>,
    ) -> savvy::Result<OpendalAio> {
        let n = path.len();
        let concurrency = batch_concurrency_limit(batch_concurrency, n)?;
        let mut values: Vec<Option<AioOutcome>> = (0..n).map(|_| None).collect();
        let mut requests = Vec::new();
        for (i, p) in path.iter().enumerate() {
            match normalize_user_path(p, false) {
                Ok(p) => requests.push((i, p)),
                Err(e) => values[i] = Some(invalid_argument_outcome("stat", p, e)),
            }
        }
        let op = self.inner.op.clone();
        let handle = self.inner.runtime.spawn(async move {
            if n == 0 {
                return AioOutcome::Many(Vec::new());
            }
            if n == 1 {
                if let Some(value) = values.into_iter().next().flatten() {
                    return value;
                }
                return stat_request_outcome(op, requests.into_iter().next().unwrap().1).await;
            }
            let async_values = stream::iter(requests.into_iter())
                .map(|(i, p)| {
                    let op = op.clone();
                    async move { (i, stat_request_outcome(op, p).await) }
                })
                .buffer_unordered(concurrency)
                .collect::<Vec<_>>()
                .await;
            for (i, value) in async_values {
                values[i] = Some(value);
            }
            AioOutcome::Many(
                values
                    .into_iter()
                    .map(|value| value.unwrap_or_else(|| unexpected_outcome("stat")))
                    .collect(),
            )
        });
        Ok(OpendalAio::new(self.inner.runtime.clone(), handle))
    }

    /// Check whether path(s) exist.
    /// @export
    fn exists(
        &self,
        path: StringSexp,
        batch_concurrency: Option<f64>,
    ) -> savvy::Result<savvy::Sexp> {
        let n = path.len();
        let concurrency = batch_concurrency_limit(batch_concurrency, n)?;
        if n == 0 {
            return OwnedListSexp::new(0, false)?.into();
        }
        if n == 1 {
            return self.exists_one(path.iter().next().unwrap_or(""));
        }

        let mut values: Vec<Option<ExistsValue>> = (0..n).map(|_| None).collect();
        let mut requests = Vec::new();
        for (i, p) in path.iter().enumerate() {
            match normalize_user_path(p, false) {
                Ok(p) => requests.push((i, p)),
                Err(e) => {
                    values[i] = Some(ExistsValue::Error {
                        kind: "InvalidArgument".to_string(),
                        code: 14,
                        message: e,
                        path: p.to_string(),
                    });
                }
            }
        }

        let op = self.inner.op.clone();
        let async_values = self.inner.runtime.block_on(async move {
            stream::iter(requests.into_iter())
                .map(|(i, p)| {
                    let op = op.clone();
                    async move { (i, exists_request(op, p).await) }
                })
                .buffer_unordered(concurrency)
                .collect::<Vec<_>>()
                .await
        });
        for (i, value) in async_values {
            values[i] = Some(value);
        }

        let mut out = OwnedListSexp::new(n, false)?;
        for (i, value) in values.into_iter().enumerate() {
            out.set_value(
                i,
                exists_value_to_sexp(value.unwrap_or(ExistsValue::Error {
                    kind: "Unexpected".to_string(),
                    code: 1,
                    message: "missing exists result".to_string(),
                    path: String::new(),
                }))?,
            )?;
        }
        out.into()
    }

    /// Submit asynchronous existence check(s).
    /// @export
    fn exists_aio(
        &self,
        path: StringSexp,
        batch_concurrency: Option<f64>,
    ) -> savvy::Result<OpendalAio> {
        let n = path.len();
        let concurrency = batch_concurrency_limit(batch_concurrency, n)?;
        let mut values: Vec<Option<AioOutcome>> = (0..n).map(|_| None).collect();
        let mut requests = Vec::new();
        for (i, p) in path.iter().enumerate() {
            match normalize_user_path(p, false) {
                Ok(p) => requests.push((i, p)),
                Err(e) => values[i] = Some(invalid_argument_outcome("exists", p, e)),
            }
        }
        let op = self.inner.op.clone();
        let handle = self.inner.runtime.spawn(async move {
            if n == 0 {
                return AioOutcome::Many(Vec::new());
            }
            if n == 1 {
                if let Some(value) = values.into_iter().next().flatten() {
                    return value;
                }
                return exists_request_outcome(op, requests.into_iter().next().unwrap().1).await;
            }
            let async_values = stream::iter(requests.into_iter())
                .map(|(i, p)| {
                    let op = op.clone();
                    async move { (i, exists_request_outcome(op, p).await) }
                })
                .buffer_unordered(concurrency)
                .collect::<Vec<_>>()
                .await;
            for (i, value) in async_values {
                values[i] = Some(value);
            }
            AioOutcome::Many(
                values
                    .into_iter()
                    .map(|value| value.unwrap_or_else(|| unexpected_outcome("exists")))
                    .collect(),
            )
        });
        Ok(OpendalAio::new(self.inner.runtime.clone(), handle))
    }

    /// Create a directory.
    /// @export
    fn mkdir(&self, path: &str) -> savvy::Result<savvy::Sexp> {
        let path = match normalize_user_path(path, true) {
            Ok(p) => p,
            Err(e) => return crate::error::error_list("InvalidArgument", 14, &e, "mkdir", path),
        };
        let op = self.inner.op.clone();
        let path_for_error = path.clone();
        match self
            .inner
            .runtime
            .block_on(async move { op.create_dir(&path).await })
        {
            Ok(_) => success_value(),
            Err(e) => op_error_list(e, "mkdir", &path_for_error),
        }
    }

    /// Submit asynchronous directory creation request(s).
    /// @export
    fn mkdir_aio(
        &self,
        path: StringSexp,
        batch_concurrency: Option<f64>,
    ) -> savvy::Result<OpendalAio> {
        let n = path.len();
        let concurrency = batch_concurrency_limit(batch_concurrency, n)?;
        let mut values: Vec<Option<AioOutcome>> = (0..n).map(|_| None).collect();
        let mut requests = Vec::new();
        for (i, p) in path.iter().enumerate() {
            match normalize_user_path(p, true) {
                Ok(p) => requests.push((i, p)),
                Err(e) => values[i] = Some(invalid_argument_outcome("mkdir", p, e)),
            }
        }
        let op = self.inner.op.clone();
        let handle = self.inner.runtime.spawn(async move {
            if n == 0 {
                return AioOutcome::Many(Vec::new());
            }
            if n == 1 {
                if let Some(value) = values.into_iter().next().flatten() {
                    return value;
                }
                return mkdir_request_outcome(op, requests.into_iter().next().unwrap().1).await;
            }
            let async_values = stream::iter(requests.into_iter())
                .map(|(i, p)| {
                    let op = op.clone();
                    async move { (i, mkdir_request_outcome(op, p).await) }
                })
                .buffer_unordered(concurrency)
                .collect::<Vec<_>>()
                .await;
            for (i, value) in async_values {
                values[i] = Some(value);
            }
            AioOutcome::Many(
                values
                    .into_iter()
                    .map(|value| value.unwrap_or_else(|| unexpected_outcome("mkdir")))
                    .collect(),
            )
        });
        Ok(OpendalAio::new(self.inner.runtime.clone(), handle))
    }

    /// Delete path(s).
    /// @export
    fn delete(
        &self,
        path: StringSexp,
        recursive: Option<bool>,
        batch_concurrency: Option<f64>,
    ) -> savvy::Result<savvy::Sexp> {
        let recursive = recursive.unwrap_or(false);
        let n = path.len();
        let concurrency = batch_concurrency_limit(batch_concurrency, n)?;
        if n == 0 {
            return OwnedListSexp::new(0, false)?.into();
        }
        if n == 1 {
            return self.delete_one(path.iter().next().unwrap_or(""), recursive);
        }

        let mut values: Vec<Option<DeleteValue>> = (0..n).map(|_| None).collect();
        let mut requests = Vec::new();
        for (i, p) in path.iter().enumerate() {
            match normalize_user_path(p, false) {
                Ok(p) => requests.push((i, p)),
                Err(e) => {
                    values[i] = Some(DeleteValue::Error {
                        kind: "InvalidArgument".to_string(),
                        code: 14,
                        message: e,
                        path: p.to_string(),
                    });
                }
            }
        }

        let op = self.inner.op.clone();
        let async_values = self.inner.runtime.block_on(async move {
            stream::iter(requests.into_iter())
                .map(|(i, p)| {
                    let op = op.clone();
                    async move { (i, delete_request(op, p, recursive).await) }
                })
                .buffer_unordered(concurrency)
                .collect::<Vec<_>>()
                .await
        });
        for (i, value) in async_values {
            values[i] = Some(value);
        }

        let mut out = OwnedListSexp::new(n, false)?;
        for (i, value) in values.into_iter().enumerate() {
            out.set_value(
                i,
                delete_value_to_sexp(value.unwrap_or(DeleteValue::Error {
                    kind: "Unexpected".to_string(),
                    code: 1,
                    message: "missing delete result".to_string(),
                    path: String::new(),
                }))?,
            )?;
        }
        out.into()
    }

    /// Submit asynchronous deletion request(s).
    /// @export
    fn delete_aio(
        &self,
        path: StringSexp,
        recursive: Option<bool>,
        batch_concurrency: Option<f64>,
    ) -> savvy::Result<OpendalAio> {
        let recursive = recursive.unwrap_or(false);
        let n = path.len();
        let concurrency = batch_concurrency_limit(batch_concurrency, n)?;
        let mut values: Vec<Option<AioOutcome>> = (0..n).map(|_| None).collect();
        let mut requests = Vec::new();
        for (i, p) in path.iter().enumerate() {
            match normalize_user_path(p, false) {
                Ok(p) => requests.push((i, p)),
                Err(e) => values[i] = Some(invalid_argument_outcome("delete", p, e)),
            }
        }
        let op = self.inner.op.clone();
        let handle = self.inner.runtime.spawn(async move {
            if n == 0 {
                return AioOutcome::Many(Vec::new());
            }
            if n == 1 {
                if let Some(value) = values.into_iter().next().flatten() {
                    return value;
                }
                return delete_request_outcome(
                    op,
                    requests.into_iter().next().unwrap().1,
                    recursive,
                )
                .await;
            }
            let async_values = stream::iter(requests.into_iter())
                .map(|(i, p)| {
                    let op = op.clone();
                    async move { (i, delete_request_outcome(op, p, recursive).await) }
                })
                .buffer_unordered(concurrency)
                .collect::<Vec<_>>()
                .await;
            for (i, value) in async_values {
                values[i] = Some(value);
            }
            AioOutcome::Many(
                values
                    .into_iter()
                    .map(|value| value.unwrap_or_else(|| unexpected_outcome("delete")))
                    .collect(),
            )
        });
        Ok(OpendalAio::new(self.inner.runtime.clone(), handle))
    }

    /// Copy path(s) with strict length matching.
    /// @export
    fn copy(&self, from: StringSexp, to: StringSexp) -> savvy::Result<savvy::Sexp> {
        if from.len() != to.len() {
            return Err(savvy::Error::new("from and to lengths must match"));
        }
        if from.len() == 1 {
            return self.copy_one(
                from.iter().next().unwrap_or(""),
                to.iter().next().unwrap_or(""),
            );
        }
        let mut out = OwnedListSexp::new(from.len(), false)?;
        for (i, (from_path, to_path)) in from.iter().zip(to.iter()).enumerate() {
            out.set_value(i, self.copy_one(from_path, to_path)?)?;
        }
        out.into()
    }

    /// Rename path(s) with strict length matching.
    /// @export
    fn rename(&self, from: StringSexp, to: StringSexp) -> savvy::Result<savvy::Sexp> {
        if from.len() != to.len() {
            return Err(savvy::Error::new("from and to lengths must match"));
        }
        if from.len() == 1 {
            return self.rename_one(
                from.iter().next().unwrap_or(""),
                to.iter().next().unwrap_or(""),
            );
        }
        let mut out = OwnedListSexp::new(from.len(), false)?;
        for (i, (from_path, to_path)) in from.iter().zip(to.iter()).enumerate() {
            out.set_value(i, self.rename_one(from_path, to_path)?)?;
        }
        out.into()
    }

    /// Submit asynchronous copy request(s) with strict length matching.
    /// @export
    fn copy_aio(
        &self,
        from: StringSexp,
        to: StringSexp,
        batch_concurrency: Option<f64>,
    ) -> savvy::Result<OpendalAio> {
        self.copy_or_rename_aio_common(from, to, batch_concurrency, "copy", copy_request_outcome)
    }

    /// Submit asynchronous rename request(s) with strict length matching.
    /// @export
    fn rename_aio(
        &self,
        from: StringSexp,
        to: StringSexp,
        batch_concurrency: Option<f64>,
    ) -> savvy::Result<OpendalAio> {
        self.copy_or_rename_aio_common(
            from,
            to,
            batch_concurrency,
            "rename",
            rename_request_outcome,
        )
    }

    /// List entries under a directory.
    /// @export
    fn ls(
        &self,
        path: &str,
        recursive: bool,
        limit: Option<f64>,
        start_after: Option<&str>,
    ) -> savvy::Result<savvy::Sexp> {
        let path = match normalize_user_path(path, true) {
            Ok(p) => p,
            Err(e) => return crate::error::error_list("InvalidArgument", 14, &e, "ls", path),
        };
        let limit = checked_limit(limit, "limit")?;
        let start_after = normalize_optional_start_after(start_after)
            .map_err(|e| savvy::Error::new(&format!("start_after: {e}")))?;
        let op = self.inner.op.clone();
        let path_for_error = path.clone();
        let mut opts = ListOptions::default();
        opts.recursive = recursive;
        opts.limit = limit.filter(|value| *value > 0);
        opts.start_after = start_after.clone();
        match self
            .inner
            .runtime
            .block_on(async move { op.list_options(&path, opts).await })
        {
            Ok(entries) => {
                let mut entries = entries
                    .iter()
                    .filter(|entry| entry.path() != "/" && !entry.path().is_empty())
                    .filter(|entry| {
                        start_after
                            .as_ref()
                            .is_none_or(|start_after| entry.path() > start_after.as_str())
                    })
                    .collect::<Vec<_>>();
                if let Some(limit) = limit {
                    entries.truncate(limit);
                }
                let mut out = OwnedListSexp::new(entries.len(), false)?;
                for (i, entry) in entries.iter().enumerate() {
                    out.set_value(i, metadata_list(entry.path(), entry.metadata())?)?;
                }
                out.into()
            }
            Err(e) => op_error_list(e, "ls", &path_for_error),
        }
    }

    /// Submit asynchronous listing request.
    /// @export
    fn ls_aio(
        &self,
        path: &str,
        recursive: bool,
        limit: Option<f64>,
        start_after: Option<&str>,
    ) -> savvy::Result<OpendalAio> {
        let path = match normalize_user_path(path, true) {
            Ok(p) => p,
            Err(e) => {
                let path = path.to_string();
                let handle = self
                    .inner
                    .runtime
                    .spawn(async move { invalid_argument_outcome("ls", &path, e) });
                return Ok(OpendalAio::new(self.inner.runtime.clone(), handle));
            }
        };
        let limit = checked_limit(limit, "limit")?;
        let start_after = normalize_optional_start_after(start_after)
            .map_err(|e| savvy::Error::new(&format!("start_after: {e}")))?;
        let op = self.inner.op.clone();
        let handle = self.inner.runtime.spawn(async move {
            ls_request_outcome(op, path, recursive, limit, start_after).await
        });
        Ok(OpendalAio::new(self.inner.runtime.clone(), handle))
    }

    /// Create a streaming listing iterator.
    /// @export
    fn ls_iter(
        &self,
        path: &str,
        recursive: Option<bool>,
        page_size: Option<f64>,
        limit: Option<f64>,
        start_after: Option<&str>,
        prefetch: Option<f64>,
    ) -> savvy::Result<OpendalLsIter> {
        let path = normalize_user_path(path, true)
            .map_err(|e| savvy::Error::new(&format!("ls_iter: {e}")))?;
        let page_size = checked_page_size(page_size.unwrap_or(1000.0), "page_size")?;
        let limit = checked_limit(limit, "limit")?;
        let prefetch = checked_nonnegative_usize(prefetch, "prefetch")?;
        let start_after = normalize_optional_start_after(start_after)
            .map_err(|e| savvy::Error::new(&format!("start_after: {e}")))?;
        OpendalLsIter::new(
            self.inner.clone(),
            path,
            recursive.unwrap_or(false),
            page_size,
            limit,
            start_after,
            prefetch,
            "ls_iter",
        )
    }

    /// Create a streaming recursive traversal iterator.
    /// @export
    fn walk_iter(
        &self,
        path: &str,
        page_size: Option<f64>,
        limit: Option<f64>,
        start_after: Option<&str>,
        prefetch: Option<f64>,
    ) -> savvy::Result<OpendalLsIter> {
        let path = normalize_user_path(path, true)
            .map_err(|e| savvy::Error::new(&format!("walk_iter: {e}")))?;
        let page_size = checked_page_size(page_size.unwrap_or(1000.0), "page_size")?;
        let limit = checked_limit(limit, "limit")?;
        let prefetch = checked_nonnegative_usize(prefetch, "prefetch")?;
        let start_after = normalize_optional_start_after(start_after)
            .map_err(|e| savvy::Error::new(&format!("start_after: {e}")))?;
        OpendalLsIter::new(
            self.inner.clone(),
            path,
            true,
            page_size,
            limit,
            start_after,
            prefetch,
            "walk_iter",
        )
    }
}

impl OpendalFs {
    fn open_from_pairs(
        scheme: &str,
        config: Vec<(String, String)>,
        headers: Vec<(String, String)>,
        runtime_threads: Option<f64>,
        max_inflight: Option<f64>,
        timeout_seconds: Option<f64>,
        io_timeout_seconds: Option<f64>,
    ) -> savvy::Result<Self> {
        init_registry();
        let runtime_threads = checked_positive_usize(runtime_threads, "runtime_threads")?;
        let max_inflight = checked_positive_usize(max_inflight, "max_inflight")?;
        let request_timeout = checked_positive_duration(timeout_seconds, "request_timeout")?;
        let io_timeout = checked_positive_duration(io_timeout_seconds, "io_timeout")?;
        let op = Operator::via_iter(scheme, config)
            .map_err(|e| savvy::Error::new(&format!("cannot open OpenDAL operator: {e}")))?;
        let op = apply_http_headers(op, headers)?;
        let op = apply_timeout_layer(op, request_timeout, io_timeout);
        let op = apply_concurrent_limit(op, max_inflight);
        let info = op.info();
        let native = NativeFs {
            op,
            runtime: build_runtime(runtime_threads)?,
            scheme: info.scheme().to_string(),
            root: info.root(),
        };
        Ok(Self {
            inner: Arc::new(native),
        })
    }

    fn delete_one(&self, path: &str, recursive: bool) -> savvy::Result<savvy::Sexp> {
        let path = match normalize_user_path(path, false) {
            Ok(p) => p,
            Err(e) => return crate::error::error_list("InvalidArgument", 14, &e, "delete", path),
        };
        let op = self.inner.op.clone();
        let path_for_error = path.clone();
        let mut opts = DeleteOptions::default();
        opts.recursive = recursive;
        match self
            .inner
            .runtime
            .block_on(async move { op.delete_options(&path, opts).await })
        {
            Ok(_) => success_value(),
            Err(e) => op_error_list(e, "delete", &path_for_error),
        }
    }

    fn stat_one(&self, path: &str) -> savvy::Result<savvy::Sexp> {
        let path = match normalize_user_path(path, false) {
            Ok(p) => p,
            Err(e) => return crate::error::error_list("InvalidArgument", 14, &e, "stat", path),
        };
        let op = self.inner.op.clone();
        let path_for_error = path.clone();
        match self
            .inner
            .runtime
            .block_on(async move { op.stat(&path).await })
        {
            Ok(meta) => metadata_list(&path_for_error, &meta),
            Err(e) => op_error_list(e, "stat", &path_for_error),
        }
    }

    fn exists_one(&self, path: &str) -> savvy::Result<savvy::Sexp> {
        let path = match normalize_user_path(path, false) {
            Ok(p) => p,
            Err(e) => return crate::error::error_list("InvalidArgument", 14, &e, "exists", path),
        };
        let op = self.inner.op.clone();
        let path_for_error = path.clone();
        match self
            .inner
            .runtime
            .block_on(async move { op.exists(&path).await })
        {
            Ok(exists) => bool_scalar(exists)?.into(),
            Err(e) => op_error_list(e, "exists", &path_for_error),
        }
    }

    fn copy_one(&self, from: &str, to: &str) -> savvy::Result<savvy::Sexp> {
        let from_norm = match normalize_user_path(from, false) {
            Ok(p) => p,
            Err(e) => return crate::error::error_list("InvalidArgument", 14, &e, "copy", from),
        };
        let to_norm = match normalize_user_path(to, false) {
            Ok(p) => p,
            Err(e) => return crate::error::error_list("InvalidArgument", 14, &e, "copy", to),
        };
        let op = self.inner.op.clone();
        let from_for_error = from_norm.clone();
        match self
            .inner
            .runtime
            .block_on(async move { op.copy(&from_norm, &to_norm).await })
        {
            Ok(_) => success_value(),
            Err(e) => op_error_list(e, "copy", &from_for_error),
        }
    }

    fn rename_one(&self, from: &str, to: &str) -> savvy::Result<savvy::Sexp> {
        let from_norm = match normalize_user_path(from, false) {
            Ok(p) => p,
            Err(e) => return crate::error::error_list("InvalidArgument", 14, &e, "rename", from),
        };
        let to_norm = match normalize_user_path(to, false) {
            Ok(p) => p,
            Err(e) => return crate::error::error_list("InvalidArgument", 14, &e, "rename", to),
        };
        let op = self.inner.op.clone();
        let from_for_error = from_norm.clone();
        match self
            .inner
            .runtime
            .block_on(async move { op.rename(&from_norm, &to_norm).await })
        {
            Ok(_) => success_value(),
            Err(e) => op_error_list(e, "rename", &from_for_error),
        }
    }

    fn read_requests(
        &self,
        plan: ReadPlan,
        result: &str,
        batch_concurrency: Option<f64>,
        tuning: ReadTuning,
    ) -> savvy::Result<savvy::Sexp> {
        self.read_requests_with_materializer(
            plan,
            result,
            batch_concurrency,
            tuning,
            read_value_to_sexp,
        )
    }

    fn read_bytes_requests(
        &self,
        plan: ReadPlan,
        result: &str,
        batch_concurrency: Option<f64>,
        tuning: ReadTuning,
    ) -> savvy::Result<savvy::Sexp> {
        self.read_requests_with_materializer(
            plan,
            result,
            batch_concurrency,
            tuning,
            read_value_to_bytes_sexp,
        )
    }

    fn read_requests_with_materializer<F>(
        &self,
        plan: ReadPlan,
        result: &str,
        batch_concurrency: Option<f64>,
        tuning: ReadTuning,
        materialize: F,
    ) -> savvy::Result<savvy::Sexp>
    where
        F: Fn(ReadValue) -> savvy::Result<savvy::Sexp> + Copy,
    {
        let n = plan.requests.len();
        let concurrency = batch_concurrency_limit(batch_concurrency, n)?;
        let op = self.inner.op.clone();
        let requests = plan.requests;
        let mut values = self.inner.runtime.block_on(async move {
            stream::iter(requests.into_iter())
                .map(|req| {
                    let op = op.clone();
                    let key = (req.path_index, req.range_index);
                    async move { (key, read_request(op, req, tuning).await) }
                })
                .buffer_unordered(concurrency)
                .collect::<Vec<_>>()
                .await
        });
        values.sort_by_key(|(key, _)| *key);

        materialize_read_values(
            values.into_iter().map(|(_, value)| value).collect(),
            result,
            &plan.shape,
            materialize,
        )
    }

    fn write_many_aio_common(
        &self,
        paths: StringSexp,
        data: Sexp,
        create_only: bool,
        append: bool,
        operation: &str,
        batch_concurrency: Option<f64>,
        tuning: WriteTuning,
    ) -> savvy::Result<OpendalAio> {
        let n = paths.len();
        let payloads = payloads_from_sexp(data, n)?;
        let concurrency = batch_concurrency_limit(batch_concurrency, n)?;
        let mut requests = Vec::with_capacity(n);
        let mut values: Vec<Option<AioOutcome>> = (0..n).map(|_| None).collect();
        for (i, path) in paths.iter().enumerate() {
            match normalize_user_path(path, false) {
                Ok(path) => requests.push((i, path, payloads[i].clone())),
                Err(e) => values[i] = Some(invalid_argument_outcome(operation, path, e)),
            }
        }

        let op = self.inner.op.clone();
        let operation_owned = operation.to_string();
        let handle = self.inner.runtime.spawn(async move {
            if n == 0 {
                return AioOutcome::Many(Vec::new());
            }
            if n == 1 {
                if let Some(value) = values.into_iter().next().flatten() {
                    return value;
                }
                let (_, path, data) = requests.into_iter().next().unwrap();
                return write_request_outcome(
                    op,
                    path,
                    data,
                    create_only,
                    append,
                    &operation_owned,
                    tuning,
                )
                .await;
            }
            let async_values = stream::iter(requests.into_iter())
                .map(|(i, path, data)| {
                    let op = op.clone();
                    let operation = operation_owned.clone();
                    async move {
                        (
                            i,
                            write_request_outcome(
                                op,
                                path,
                                data,
                                create_only,
                                append,
                                &operation,
                                tuning,
                            )
                            .await,
                        )
                    }
                })
                .buffer_unordered(concurrency)
                .collect::<Vec<_>>()
                .await;
            for (i, value) in async_values {
                values[i] = Some(value);
            }
            AioOutcome::Many(
                values
                    .into_iter()
                    .map(|value| value.unwrap_or_else(|| unexpected_outcome(&operation_owned)))
                    .collect(),
            )
        });
        Ok(OpendalAio::new(self.inner.runtime.clone(), handle))
    }

    fn copy_or_rename_aio_common<F, Fut>(
        &self,
        from: StringSexp,
        to: StringSexp,
        batch_concurrency: Option<f64>,
        operation: &str,
        submit: F,
    ) -> savvy::Result<OpendalAio>
    where
        F: Fn(Operator, String, String) -> Fut + Copy + Send + Sync + 'static,
        Fut: std::future::Future<Output = AioOutcome> + Send + 'static,
    {
        if from.len() != to.len() {
            return Err(savvy::Error::new("from and to lengths must match"));
        }
        let n = from.len();
        let concurrency = batch_concurrency_limit(batch_concurrency, n)?;
        let mut requests = Vec::with_capacity(n);
        let mut values: Vec<Option<AioOutcome>> = (0..n).map(|_| None).collect();
        for (i, (from_path, to_path)) in from.iter().zip(to.iter()).enumerate() {
            let from_norm = match normalize_user_path(from_path, false) {
                Ok(path) => path,
                Err(e) => {
                    values[i] = Some(invalid_argument_outcome(operation, from_path, e));
                    continue;
                }
            };
            let to_norm = match normalize_user_path(to_path, false) {
                Ok(path) => path,
                Err(e) => {
                    values[i] = Some(invalid_argument_outcome(operation, to_path, e));
                    continue;
                }
            };
            requests.push((i, from_norm, to_norm));
        }

        let op = self.inner.op.clone();
        let operation_owned = operation.to_string();
        let handle = self.inner.runtime.spawn(async move {
            if n == 0 {
                return AioOutcome::Many(Vec::new());
            }
            if n == 1 {
                if let Some(value) = values.into_iter().next().flatten() {
                    return value;
                }
                let (_, from, to) = requests.into_iter().next().unwrap();
                return submit(op, from, to).await;
            }
            let async_values = stream::iter(requests.into_iter())
                .map(|(i, from, to)| {
                    let op = op.clone();
                    async move { (i, submit(op, from, to).await) }
                })
                .buffer_unordered(concurrency)
                .collect::<Vec<_>>()
                .await;
            for (i, value) in async_values {
                values[i] = Some(value);
            }
            AioOutcome::Many(
                values
                    .into_iter()
                    .map(|value| value.unwrap_or_else(|| unexpected_outcome(&operation_owned)))
                    .collect(),
            )
        });
        Ok(OpendalAio::new(self.inner.runtime.clone(), handle))
    }

    fn write_many_common(
        &self,
        paths: StringSexp,
        data: Sexp,
        create_only: bool,
        append: bool,
        operation: &str,
        batch_concurrency: Option<f64>,
        tuning: WriteTuning,
    ) -> savvy::Result<savvy::Sexp> {
        let n = paths.len();
        let payloads = payloads_from_sexp(data, n)?;
        let concurrency = batch_concurrency_limit(batch_concurrency, n)?;
        if n == 0 {
            return OwnedListSexp::new(0, false)?.into();
        }
        if n == 1 {
            let path = paths.iter().next().unwrap_or("");
            return self.write_common(
                path,
                payloads.into_iter().next().unwrap_or_default(),
                create_only,
                append,
                operation,
                tuning,
            );
        }

        let mut requests = Vec::with_capacity(n);
        let mut values: Vec<Option<WriteValue>> = (0..n).map(|_| None).collect();
        for (i, path) in paths.iter().enumerate() {
            match normalize_user_path(path, false) {
                Ok(path) => requests.push((i, path, payloads[i].clone())),
                Err(e) => {
                    values[i] = Some(WriteValue::Error {
                        kind: "InvalidArgument".to_string(),
                        code: 14,
                        message: e,
                        path: path.to_string(),
                    });
                }
            }
        }

        let op = self.inner.op.clone();
        let operation_owned = operation.to_string();
        let async_values = self.inner.runtime.block_on(async move {
            stream::iter(requests.into_iter())
                .map(|(i, path, data)| {
                    let op = op.clone();
                    let operation = operation_owned.clone();
                    async move {
                        (
                            i,
                            write_request(op, path, data, create_only, append, &operation, tuning)
                                .await,
                        )
                    }
                })
                .buffer_unordered(concurrency)
                .collect::<Vec<_>>()
                .await
        });
        for (i, value) in async_values {
            values[i] = Some(value);
        }

        let mut out = OwnedListSexp::new(n, false)?;
        for (i, value) in values.into_iter().enumerate() {
            out.set_value(
                i,
                write_value_to_sexp(
                    value.unwrap_or(WriteValue::Error {
                        kind: "Unexpected".to_string(),
                        code: 1,
                        message: "missing write result".to_string(),
                        path: String::new(),
                    }),
                    operation,
                )?,
            )?;
        }
        out.into()
    }

    fn write_common(
        &self,
        path: &str,
        data: Buffer,
        create_only: bool,
        append: bool,
        operation: &str,
        tuning: WriteTuning,
    ) -> savvy::Result<savvy::Sexp> {
        let path = match normalize_user_path(path, false) {
            Ok(p) => p,
            Err(e) => return crate::error::error_list("InvalidArgument", 14, &e, operation, path),
        };
        if append && !self.inner.op.info().full_capability().write_can_append {
            return unsupported_value(operation, &path);
        }
        let op = self.inner.op.clone();
        let path_for_error = path.clone();
        match self.inner.runtime.block_on(write_bytes_with(
            op,
            path,
            data,
            create_only,
            append,
            tuning,
        )) {
            Ok(_) => success_value(),
            Err(e) => op_error_list(e, operation, &path_for_error),
        }
    }
}

struct ReadPlan {
    requests: Vec<ReadRequest>,
    shape: ReadShape,
}

struct ReadShape {
    n_paths: usize,
    ranges_per_path: Vec<usize>,
}

impl ReadShape {
    fn is_scalar(&self) -> bool {
        self.n_paths == 1 && self.ranges_per_path.first().copied().unwrap_or(0) == 1
    }

    fn auto_nested(&self) -> bool {
        self.n_paths > 1 && self.ranges_per_path.iter().any(|n| *n != 1)
    }
}

struct ReadRequest {
    path_index: usize,
    range_index: usize,
    path: String,
    offset: u64,
    size: Option<u64>,
    error: Option<String>,
}

enum StatValue {
    Metadata {
        path: String,
        meta: Metadata,
    },
    Error {
        kind: String,
        code: i32,
        message: String,
        path: String,
    },
}

async fn stat_request(op: Operator, path: String) -> StatValue {
    let path_for_error = path.clone();
    match op.stat(&path).await {
        Ok(meta) => StatValue::Metadata {
            path: path_for_error,
            meta,
        },
        Err(e) => {
            let kind = e.kind();
            StatValue::Error {
                kind: kind.into_static().to_string(),
                code: kind_code(kind),
                message: e.to_string(),
                path: path_for_error,
            }
        }
    }
}

fn stat_value_to_sexp(value: StatValue) -> savvy::Result<savvy::Sexp> {
    match value {
        StatValue::Metadata { path, meta } => metadata_list(&path, &meta),
        StatValue::Error {
            kind,
            code,
            message,
            path,
        } => crate::error::error_list(&kind, code, &message, "stat", &path),
    }
}

enum WriteValue {
    Written,
    Error {
        kind: String,
        code: i32,
        message: String,
        path: String,
    },
}

async fn write_request(
    op: Operator,
    path: String,
    data: Buffer,
    create_only: bool,
    append: bool,
    operation: &str,
    tuning: WriteTuning,
) -> WriteValue {
    if append && !op.info().full_capability().write_can_append {
        return WriteValue::Error {
            kind: "Unsupported".to_string(),
            code: kind_code(opendal::ErrorKind::Unsupported),
            message: format!("{operation} is not supported by this backend"),
            path,
        };
    }
    let path_for_error = path.clone();
    match write_bytes_with(op, path, data, create_only, append, tuning).await {
        Ok(_) => WriteValue::Written,
        Err(e) => {
            let kind = e.kind();
            WriteValue::Error {
                kind: kind.into_static().to_string(),
                code: kind_code(kind),
                message: e.to_string(),
                path: path_for_error,
            }
        }
    }
}

fn write_value_to_sexp(value: WriteValue, operation: &str) -> savvy::Result<savvy::Sexp> {
    match value {
        WriteValue::Written => success_value(),
        WriteValue::Error {
            kind,
            code,
            message,
            path,
        } => crate::error::error_list(&kind, code, &message, operation, &path),
    }
}

enum DeleteValue {
    Deleted,
    Error {
        kind: String,
        code: i32,
        message: String,
        path: String,
    },
}

async fn delete_request(op: Operator, path: String, recursive: bool) -> DeleteValue {
    let path_for_error = path.clone();
    let mut opts = DeleteOptions::default();
    opts.recursive = recursive;
    match op.delete_options(&path, opts).await {
        Ok(_) => DeleteValue::Deleted,
        Err(e) => {
            let kind = e.kind();
            DeleteValue::Error {
                kind: kind.into_static().to_string(),
                code: kind_code(kind),
                message: e.to_string(),
                path: path_for_error,
            }
        }
    }
}

fn delete_value_to_sexp(value: DeleteValue) -> savvy::Result<savvy::Sexp> {
    match value {
        DeleteValue::Deleted => success_value(),
        DeleteValue::Error {
            kind,
            code,
            message,
            path,
        } => crate::error::error_list(&kind, code, &message, "delete", &path),
    }
}

enum ExistsValue {
    Exists(bool),
    Error {
        kind: String,
        code: i32,
        message: String,
        path: String,
    },
}

async fn exists_request(op: Operator, path: String) -> ExistsValue {
    let path_for_error = path.clone();
    match op.exists(&path).await {
        Ok(exists) => ExistsValue::Exists(exists),
        Err(e) => {
            let kind = e.kind();
            ExistsValue::Error {
                kind: kind.into_static().to_string(),
                code: kind_code(kind),
                message: e.to_string(),
                path: path_for_error,
            }
        }
    }
}

fn exists_value_to_sexp(value: ExistsValue) -> savvy::Result<savvy::Sexp> {
    match value {
        ExistsValue::Exists(exists) => bool_scalar(exists)?.into(),
        ExistsValue::Error {
            kind,
            code,
            message,
            path,
        } => crate::error::error_list(&kind, code, &message, "exists", &path),
    }
}

async fn stat_request_outcome(op: Operator, path: String) -> AioOutcome {
    match stat_request(op, path).await {
        StatValue::Metadata { path, meta } => AioOutcome::Metadata { path, meta },
        StatValue::Error {
            kind,
            code,
            message,
            path,
        } => AioOutcome::Error {
            kind,
            code,
            message,
            operation: "stat".to_string(),
            path,
        },
    }
}

async fn exists_request_outcome(op: Operator, path: String) -> AioOutcome {
    match exists_request(op, path).await {
        ExistsValue::Exists(exists) => AioOutcome::Bool(exists),
        ExistsValue::Error {
            kind,
            code,
            message,
            path,
        } => AioOutcome::Error {
            kind,
            code,
            message,
            operation: "exists".to_string(),
            path,
        },
    }
}

async fn mkdir_request_outcome(op: Operator, path: String) -> AioOutcome {
    let path_for_error = path.clone();
    match op.create_dir(&path).await {
        Ok(_) => AioOutcome::Unit,
        Err(e) => op_error_outcome(e, "mkdir", &path_for_error),
    }
}

async fn delete_request_outcome(op: Operator, path: String, recursive: bool) -> AioOutcome {
    match delete_request(op, path, recursive).await {
        DeleteValue::Deleted => AioOutcome::Unit,
        DeleteValue::Error {
            kind,
            code,
            message,
            path,
        } => AioOutcome::Error {
            kind,
            code,
            message,
            operation: "delete".to_string(),
            path,
        },
    }
}

async fn write_request_outcome(
    op: Operator,
    path: String,
    data: Buffer,
    create_only: bool,
    append: bool,
    operation: &str,
    tuning: WriteTuning,
) -> AioOutcome {
    match write_request(op, path, data, create_only, append, operation, tuning).await {
        WriteValue::Written => AioOutcome::Unit,
        WriteValue::Error {
            kind,
            code,
            message,
            path,
        } => AioOutcome::Error {
            kind,
            code,
            message,
            operation: operation.to_string(),
            path,
        },
    }
}

async fn copy_request_outcome(op: Operator, from: String, to: String) -> AioOutcome {
    let from_for_error = from.clone();
    match op.copy(&from, &to).await {
        Ok(_) => AioOutcome::Unit,
        Err(e) => op_error_outcome(e, "copy", &from_for_error),
    }
}

async fn rename_request_outcome(op: Operator, from: String, to: String) -> AioOutcome {
    let from_for_error = from.clone();
    match op.rename(&from, &to).await {
        Ok(_) => AioOutcome::Unit,
        Err(e) => op_error_outcome(e, "rename", &from_for_error),
    }
}

async fn ls_request_outcome(
    op: Operator,
    path: String,
    recursive: bool,
    limit: Option<usize>,
    start_after: Option<String>,
) -> AioOutcome {
    let path_for_error = path.clone();
    let mut opts = ListOptions::default();
    opts.recursive = recursive;
    opts.limit = limit.filter(|value| *value > 0);
    opts.start_after = start_after.clone();
    match op.list_options(&path, opts).await {
        Ok(entries) => {
            let mut entries = entries
                .iter()
                .filter(|entry| entry.path() != "/" && !entry.path().is_empty())
                .filter(|entry| {
                    start_after
                        .as_ref()
                        .is_none_or(|start_after| entry.path() > start_after.as_str())
                })
                .map(|entry| EntryOutcome {
                    path: entry.path().to_string(),
                    meta: entry.metadata().clone(),
                })
                .collect::<Vec<_>>();
            if let Some(limit) = limit {
                entries.truncate(limit);
            }
            AioOutcome::Entries(entries)
        }
        Err(e) => op_error_outcome(e, "ls", &path_for_error),
    }
}

fn op_error_outcome(e: opendal::Error, operation: &str, path: &str) -> AioOutcome {
    let kind = e.kind();
    AioOutcome::Error {
        kind: kind.into_static().to_string(),
        code: kind_code(kind),
        message: e.to_string(),
        operation: operation.to_string(),
        path: path.to_string(),
    }
}

fn invalid_argument_outcome(operation: &str, path: &str, message: String) -> AioOutcome {
    AioOutcome::Error {
        kind: "InvalidArgument".to_string(),
        code: 14,
        message,
        operation: operation.to_string(),
        path: path.to_string(),
    }
}

fn unexpected_outcome(operation: &str) -> AioOutcome {
    AioOutcome::Error {
        kind: "Unexpected".to_string(),
        code: 1,
        message: format!("missing {operation} result"),
        operation: operation.to_string(),
        path: String::new(),
    }
}

enum ReadValue {
    Bytes(Buffer),
    Error {
        kind: String,
        code: i32,
        message: String,
        path: String,
    },
}

async fn read_request_outcome(op: Operator, req: ReadRequest, tuning: ReadTuning) -> AioOutcome {
    read_request_outcome_with(op, req, tuning, "read", AioOutcome::Bytes).await
}

async fn read_request_bytes_outcome(
    op: Operator,
    req: ReadRequest,
    tuning: ReadTuning,
) -> AioOutcome {
    read_request_outcome_with(op, req, tuning, "read_bytes", AioOutcome::BytesHandle).await
}

async fn read_request_outcome_with<F>(
    op: Operator,
    req: ReadRequest,
    tuning: ReadTuning,
    operation: &str,
    success: F,
) -> AioOutcome
where
    F: Fn(Buffer) -> AioOutcome,
{
    if let Some(message) = req.error {
        return AioOutcome::Error {
            kind: "InvalidArgument".to_string(),
            code: 14,
            message,
            operation: operation.to_string(),
            path: req.path,
        };
    }
    let path_for_error = req.path.clone();
    match read_bytes_with(op, req.path, req.offset, req.size, tuning).await {
        Ok(bytes) => success(bytes),
        Err(e) => {
            let kind = e.kind();
            AioOutcome::Error {
                kind: kind.into_static().to_string(),
                code: kind_code(kind),
                message: e.to_string(),
                operation: operation.to_string(),
                path: path_for_error,
            }
        }
    }
}

async fn read_request(op: Operator, req: ReadRequest, tuning: ReadTuning) -> ReadValue {
    if let Some(message) = req.error {
        return ReadValue::Error {
            kind: "InvalidArgument".to_string(),
            code: 14,
            message,
            path: req.path,
        };
    }
    let path_for_error = req.path.clone();
    match read_bytes_with(op, req.path, req.offset, req.size, tuning).await {
        Ok(bytes) => ReadValue::Bytes(bytes),
        Err(e) => {
            let kind = e.kind();
            ReadValue::Error {
                kind: kind.into_static().to_string(),
                code: kind_code(kind),
                message: e.to_string(),
                path: path_for_error,
            }
        }
    }
}

fn read_value_to_sexp(value: ReadValue) -> savvy::Result<savvy::Sexp> {
    read_value_to_sexp_with(value, "read", |bytes| {
        buffer_to_raw_sexp(bytes).map(|x| x.into())
    })
}

fn read_value_to_bytes_sexp(value: ReadValue) -> savvy::Result<savvy::Sexp> {
    read_value_to_sexp_with(value, "read_bytes", opendal_bytes_to_sexp)
}

fn read_value_to_sexp_with<F>(
    value: ReadValue,
    operation: &str,
    materialize: F,
) -> savvy::Result<savvy::Sexp>
where
    F: Fn(Buffer) -> savvy::Result<savvy::Sexp>,
{
    match value {
        ReadValue::Bytes(bytes) => materialize(bytes),
        ReadValue::Error {
            kind,
            code,
            message,
            path,
        } => crate::error::error_list(&kind, code, &message, operation, &path),
    }
}

fn materialize_read_values<F>(
    values: Vec<ReadValue>,
    result: &str,
    shape: &ReadShape,
    materialize: F,
) -> savvy::Result<savvy::Sexp>
where
    F: Fn(ReadValue) -> savvy::Result<savvy::Sexp> + Copy,
{
    if result == "auto" && shape.is_scalar() {
        return materialize(
            values
                .into_iter()
                .next()
                .unwrap_or_else(unexpected_read_value),
        );
    }

    let use_nested = result == "nested" || (result == "auto" && shape.auto_nested());
    if use_nested {
        let mut iter = values.into_iter();
        let mut out = OwnedListSexp::new(shape.n_paths, false)?;
        for (path_index, range_count) in shape.ranges_per_path.iter().copied().enumerate() {
            let mut one_path = OwnedListSexp::new(range_count, false)?;
            for range_index in 0..range_count {
                one_path.set_value(
                    range_index,
                    materialize(iter.next().unwrap_or_else(unexpected_read_value))?,
                )?;
            }
            out.set_value(path_index, one_path)?;
        }
        return out.into();
    }

    let mut out = OwnedListSexp::new(values.len(), false)?;
    for (i, value) in values.into_iter().enumerate() {
        out.set_value(i, materialize(value)?)?;
    }
    out.into()
}

fn read_outcomes_to_aio(values: Vec<AioOutcome>, result: &str, shape: &ReadShape) -> AioOutcome {
    if result == "auto" && shape.is_scalar() {
        return values
            .into_iter()
            .next()
            .unwrap_or_else(|| unexpected_outcome("read"));
    }

    let use_nested = result == "nested" || (result == "auto" && shape.auto_nested());
    if use_nested {
        let mut iter = values.into_iter();
        let mut out = Vec::with_capacity(shape.n_paths);
        for range_count in shape.ranges_per_path.iter().copied() {
            let mut one_path = Vec::with_capacity(range_count);
            for _ in 0..range_count {
                one_path.push(iter.next().unwrap_or_else(|| unexpected_outcome("read")));
            }
            out.push(AioOutcome::Many(one_path));
        }
        AioOutcome::Many(out)
    } else {
        AioOutcome::Many(values)
    }
}

fn unexpected_read_value() -> ReadValue {
    ReadValue::Error {
        kind: "Unexpected".to_string(),
        code: 1,
        message: "missing read result".to_string(),
        path: String::new(),
    }
}

fn batch_concurrency_limit(value: Option<f64>, n: usize) -> savvy::Result<usize> {
    let Some(value) = value else {
        return Ok(if n == 0 { 1 } else { n.min(16) });
    };
    let checked = checked_u64(value, "batch_concurrency")?;
    if checked == 0 {
        return Err(savvy::Error::new(
            "batch_concurrency must be NULL or greater than zero",
        ));
    }
    if checked > usize::MAX as u64 {
        Err(savvy::Error::new("batch_concurrency is too large"))
    } else if n == 0 {
        Ok(1)
    } else {
        Ok((checked as usize).min(n))
    }
}

fn optional_usize(value: Option<f64>, name: &str) -> savvy::Result<Option<usize>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let checked = checked_u64(value, name)?;
    if checked == 0 {
        Ok(None)
    } else if checked > usize::MAX as u64 {
        Err(savvy::Error::new(&format!("{name} is too large")))
    } else {
        Ok(Some(checked as usize))
    }
}

fn read_tuning(
    read_concurrency: Option<f64>,
    chunk_size: Option<f64>,
    coalesce_gap: Option<f64>,
) -> savvy::Result<ReadTuning> {
    Ok(ReadTuning {
        read_concurrency: optional_usize(read_concurrency, "read_concurrency")?,
        chunk_size: optional_usize(chunk_size, "chunk_size")?,
        coalesce_gap: optional_usize(coalesce_gap, "coalesce_gap")?,
    })
}

fn write_tuning(
    write_concurrency: Option<f64>,
    chunk_size: Option<f64>,
) -> savvy::Result<WriteTuning> {
    Ok(WriteTuning {
        write_concurrency: optional_usize(write_concurrency, "write_concurrency")?,
        chunk_size: optional_usize(chunk_size, "chunk_size")?,
    })
}

fn read_requests_from_options(
    paths: StringSexp,
    offsets: Option<Sexp>,
    sizes: Option<Sexp>,
    ends: Option<Sexp>,
) -> savvy::Result<ReadPlan> {
    if sizes.is_some() && ends.is_some() {
        return Err(savvy::Error::new("use only one of size or end"));
    }
    let n_paths = paths.len();
    let offset_groups = parse_offset_groups(offsets, n_paths)?;
    let size_groups = parse_bound_groups(sizes, "size", &offset_groups)?;
    let end_groups = parse_bound_groups(ends, "end", &offset_groups)?;
    let ranges_per_path: Vec<usize> = offset_groups.iter().map(Vec::len).collect();
    let request_count = ranges_per_path.iter().sum();

    let mut requests = Vec::with_capacity(request_count);
    for (path_index, path) in paths.iter().enumerate() {
        let (normalized_path, path_error) = match normalize_user_path(path, false) {
            Ok(path) => (path, None),
            Err(error) => (path.to_string(), Some(error)),
        };
        for (range_index, offset_value) in offset_groups[path_index].iter().copied().enumerate() {
            let offset = checked_u64(offset_value, "offset")?;
            let size = if let Some(groups) = &size_groups {
                Some(checked_u64(groups[path_index][range_index], "size")?)
            } else if let Some(groups) = &end_groups {
                let end = checked_u64(groups[path_index][range_index], "end")?;
                if end < offset {
                    return Err(savvy::Error::new(
                        "end must be greater than or equal to offset",
                    ));
                }
                Some(end - offset)
            } else {
                None
            };
            requests.push(ReadRequest {
                path_index,
                range_index,
                path: normalized_path.clone(),
                offset,
                size,
                error: path_error.clone(),
            });
        }
    }

    Ok(ReadPlan {
        requests,
        shape: ReadShape {
            n_paths,
            ranges_per_path,
        },
    })
}

fn parse_offset_groups(value: Option<Sexp>, n_paths: usize) -> savvy::Result<Vec<Vec<f64>>> {
    let Some(value) = value else {
        return Ok(vec![vec![0.0]; n_paths]);
    };
    match value.into_typed() {
        TypedSexp::Null(_) => Ok(vec![vec![0.0]; n_paths]),
        TypedSexp::Real(real) => flat_offset_groups(real.as_slice(), n_paths),
        TypedSexp::Integer(int) => {
            let values: Vec<f64> = int.as_slice().iter().map(|v| *v as f64).collect();
            flat_offset_groups(&values, n_paths)
        }
        TypedSexp::List(list) => {
            let groups = numeric_list_groups(list, "offset", n_paths)?;
            validate_offset_groups(&groups)?;
            Ok(groups)
        }
        _ => Err(savvy::Error::new(
            "offset must be numeric or a list of numeric vectors",
        )),
    }
}

fn flat_offset_groups(values: &[f64], n_paths: usize) -> savvy::Result<Vec<Vec<f64>>> {
    if n_paths == 0 {
        return if values.is_empty() {
            Ok(Vec::new())
        } else {
            Err(savvy::Error::new("offset length must match path length"))
        };
    }
    if n_paths == 1 {
        if values.is_empty() {
            return Err(savvy::Error::new(
                "offset must contain at least one range per path",
            ));
        }
        return Ok(vec![values.to_vec()]);
    }
    if values.len() != n_paths {
        return Err(savvy::Error::new(
            "offset length must match path length or use a list of ranges",
        ));
    }
    Ok(values.iter().map(|value| vec![*value]).collect())
}

fn validate_offset_groups(groups: &[Vec<f64>]) -> savvy::Result<()> {
    for (i, group) in groups.iter().enumerate() {
        if group.is_empty() {
            return Err(savvy::Error::new(&format!(
                "offset for path {} must contain at least one range",
                i + 1
            )));
        }
    }
    Ok(())
}

fn parse_bound_groups(
    value: Option<Sexp>,
    name: &str,
    offset_groups: &[Vec<f64>],
) -> savvy::Result<Option<Vec<Vec<f64>>>> {
    let Some(value) = value else {
        return Ok(None);
    };
    match value.into_typed() {
        TypedSexp::Null(_) => Ok(None),
        TypedSexp::Real(real) => flat_bound_groups(real.as_slice(), name, offset_groups).map(Some),
        TypedSexp::Integer(int) => {
            let values: Vec<f64> = int.as_slice().iter().map(|v| *v as f64).collect();
            flat_bound_groups(&values, name, offset_groups).map(Some)
        }
        TypedSexp::List(list) => numeric_list_groups(list, name, offset_groups.len()).map(Some),
        _ => Err(savvy::Error::new(&format!(
            "{name} must be numeric or a list of numeric vectors"
        ))),
    }
    .and_then(|groups| {
        if let Some(groups) = &groups {
            validate_bound_groups(groups, name, offset_groups)?;
        }
        Ok(groups)
    })
}

fn flat_bound_groups(
    values: &[f64],
    name: &str,
    offset_groups: &[Vec<f64>],
) -> savvy::Result<Vec<Vec<f64>>> {
    let n_paths = offset_groups.len();
    if n_paths == 0 {
        return if values.is_empty() {
            Ok(Vec::new())
        } else {
            Err(savvy::Error::new(&format!(
                "{name} length must match path length"
            )))
        };
    }
    if n_paths == 1 {
        if values.len() != offset_groups[0].len() {
            return Err(savvy::Error::new(&format!(
                "{name} length must match offset length"
            )));
        }
        return Ok(vec![values.to_vec()]);
    }
    if offset_groups.iter().all(|group| group.len() == 1) && values.len() == n_paths {
        return Ok(values.iter().map(|value| vec![*value]).collect());
    }
    Err(savvy::Error::new(&format!(
        "{name} must match path length or be a list matching offset ranges"
    )))
}

fn numeric_list_groups(list: ListSexp, name: &str, n_paths: usize) -> savvy::Result<Vec<Vec<f64>>> {
    if list.len() != n_paths {
        return Err(savvy::Error::new(&format!(
            "{name} list length must match path length"
        )));
    }
    let mut groups = Vec::with_capacity(n_paths);
    for value in list.values_iter() {
        groups.push(numeric_arg(value, name)?);
    }
    Ok(groups)
}

fn validate_bound_groups(
    groups: &[Vec<f64>],
    name: &str,
    offset_groups: &[Vec<f64>],
) -> savvy::Result<()> {
    if groups.len() != offset_groups.len() {
        return Err(savvy::Error::new(&format!(
            "{name} list length must match path length"
        )));
    }
    for (i, (values, offsets)) in groups.iter().zip(offset_groups.iter()).enumerate() {
        if values.len() != offsets.len() {
            return Err(savvy::Error::new(&format!(
                "{name} length for path {} must match offset length",
                i + 1
            )));
        }
    }
    Ok(())
}

fn numeric_arg(value: Sexp, name: &str) -> savvy::Result<Vec<f64>> {
    match value.into_typed() {
        TypedSexp::Null(_) => Ok(Vec::new()),
        TypedSexp::Real(real) => Ok(real.as_slice().to_vec()),
        TypedSexp::Integer(int) => Ok(int.as_slice().iter().map(|v| *v as f64).collect()),
        _ => Err(savvy::Error::new(&format!("{name} must be numeric"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_concurrency_distinguishes_missing_from_zero() {
        assert_eq!(batch_concurrency_limit(None, 100).unwrap(), 16);
        assert_eq!(batch_concurrency_limit(None, 4).unwrap(), 4);
        assert_eq!(batch_concurrency_limit(None, 0).unwrap(), 1);
        assert_eq!(batch_concurrency_limit(Some(1.0), 100).unwrap(), 1);
        assert_eq!(batch_concurrency_limit(Some(32.0), 10).unwrap(), 10);
        assert!(batch_concurrency_limit(Some(0.0), 10).is_err());
        assert!(batch_concurrency_limit(Some(0.0), 0).is_err());
        assert!(batch_concurrency_limit(Some(-1.0), 10).is_err());
    }

    #[test]
    fn flat_offset_groups_preserve_range_shape_without_recycling() {
        assert_eq!(
            flat_offset_groups(&[0.0, 2.0], 1).unwrap(),
            vec![vec![0.0, 2.0]]
        );
        assert_eq!(
            flat_offset_groups(&[0.0, 4.0], 2).unwrap(),
            vec![vec![0.0], vec![4.0]]
        );
        assert!(flat_offset_groups(&[0.0], 2).is_err());
        assert!(flat_offset_groups(&[], 1).is_err());
        assert!(flat_offset_groups(&[0.0], 0).is_err());
        assert_eq!(flat_offset_groups(&[], 0).unwrap(), Vec::<Vec<f64>>::new());
        assert!(validate_offset_groups(&[vec![0.0], Vec::new()]).is_err());
    }

    #[test]
    fn flat_bound_groups_match_offset_shape() {
        let scalar_path_offsets = vec![vec![0.0, 2.0]];
        assert_eq!(
            flat_bound_groups(&[1.0, 3.0], "size", &scalar_path_offsets).unwrap(),
            vec![vec![1.0, 3.0]]
        );
        assert!(flat_bound_groups(&[1.0], "size", &scalar_path_offsets).is_err());

        let multi_path_offsets = vec![vec![0.0], vec![2.0]];
        assert_eq!(
            flat_bound_groups(&[1.0, 3.0], "size", &multi_path_offsets).unwrap(),
            vec![vec![1.0], vec![3.0]]
        );
        let nested_offsets = vec![vec![0.0, 2.0], vec![1.0]];
        assert!(flat_bound_groups(&[1.0, 2.0, 3.0], "size", &nested_offsets).is_err());
        assert!(
            validate_bound_groups(&vec![vec![1.0], vec![2.0]], "size", &nested_offsets).is_err()
        );
    }

    #[test]
    fn read_shape_auto_rules_match_r_contract() {
        let scalar = ReadShape {
            n_paths: 1,
            ranges_per_path: vec![1],
        };
        assert!(scalar.is_scalar());
        assert!(!scalar.auto_nested());

        let scalar_many_ranges = ReadShape {
            n_paths: 1,
            ranges_per_path: vec![2],
        };
        assert!(!scalar_many_ranges.is_scalar());
        assert!(!scalar_many_ranges.auto_nested());

        let many_paths_one_range_each = ReadShape {
            n_paths: 2,
            ranges_per_path: vec![1, 1],
        };
        assert!(!many_paths_one_range_each.auto_nested());

        let many_paths_nested_ranges = ReadShape {
            n_paths: 2,
            ranges_per_path: vec![2, 1],
        };
        assert!(many_paths_nested_ranges.auto_nested());
    }
}

fn append_named_config(
    out: &mut Vec<(String, String)>,
    list: ListSexp,
    context: &str,
) -> savvy::Result<()> {
    for (name, value) in list.iter() {
        if name.is_empty() {
            return Err(savvy::Error::new(&format!(
                "all {context} entries must be named"
            )));
        }
        out.push((name.to_string(), config_value_to_string(value, name)?));
    }
    Ok(())
}

fn header_pairs_from_list(headers: Option<ListSexp>) -> savvy::Result<Vec<(String, String)>> {
    let Some(headers) = headers else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (name, value) in headers.iter() {
        if name.is_empty() {
            return Err(savvy::Error::new("all headers entries must be named"));
        }
        let value = match value.into_typed() {
            TypedSexp::String(value) if value.len() == 1 => {
                value.iter().next().unwrap_or("").to_string()
            }
            _ => {
                return Err(savvy::Error::new(&format!(
                    "HTTP header {name} must be a scalar string"
                )));
            }
        };
        out.push((name.to_string(), value));
    }
    Ok(out)
}

fn checked_page_size(value: f64, name: &str) -> savvy::Result<usize> {
    let value = checked_u64(value, name)?;
    if value == 0 {
        return Err(savvy::Error::new(&format!(
            "{name} must be greater than zero"
        )));
    }
    usize::try_from(value).map_err(|_| savvy::Error::new(&format!("{name} is too large")))
}

fn checked_positive_usize(value: Option<f64>, name: &str) -> savvy::Result<Option<usize>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = checked_u64(value, name)?;
    if value == 0 {
        return Err(savvy::Error::new(&format!(
            "{name} must be greater than zero"
        )));
    }
    usize::try_from(value)
        .map(Some)
        .map_err(|_| savvy::Error::new(&format!("{name} is too large")))
}

fn checked_nonnegative_usize(value: Option<f64>, name: &str) -> savvy::Result<usize> {
    let Some(value) = value else {
        return Ok(0);
    };
    let value = checked_u64(value, name)?;
    usize::try_from(value).map_err(|_| savvy::Error::new(&format!("{name} is too large")))
}

fn checked_positive_duration(value: Option<f64>, name: &str) -> savvy::Result<Option<Duration>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value <= 0.0 {
        return Err(savvy::Error::new(&format!(
            "{name} must be greater than zero"
        )));
    }
    Duration::try_from_secs_f64(value)
        .map(Some)
        .map_err(|_| savvy::Error::new(&format!("{name} must be a finite number of seconds")))
}

fn apply_timeout_layer(
    op: Operator,
    request_timeout: Option<Duration>,
    io_timeout: Option<Duration>,
) -> Operator {
    if request_timeout.is_none() && io_timeout.is_none() {
        return op;
    }
    let mut layer = TimeoutLayer::new();
    if let Some(timeout) = request_timeout {
        layer = layer.with_timeout(timeout);
    }
    if let Some(timeout) = io_timeout {
        layer = layer.with_io_timeout(timeout);
    }
    op.layer(layer)
}

fn apply_concurrent_limit(op: Operator, max_inflight: Option<usize>) -> Operator {
    match max_inflight {
        Some(max) => op.layer(ConcurrentLimitLayer::new(max)),
        None => op,
    }
}

fn checked_limit(value: Option<f64>, name: &str) -> savvy::Result<Option<usize>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = checked_u64(value, name)?;
    usize::try_from(value)
        .map(Some)
        .map_err(|_| savvy::Error::new(&format!("{name} is too large")))
}

fn normalize_optional_start_after(value: Option<&str>) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let directory = value.trim().ends_with('/');
    let value = normalize_user_path(value, directory)?;
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

fn config_value_to_string(value: Sexp, name: &str) -> savvy::Result<String> {
    match value.into_typed() {
        TypedSexp::String(value) if value.len() == 1 => {
            Ok(value.iter().next().unwrap_or("").to_string())
        }
        TypedSexp::Integer(value) if value.len() == 1 => Ok(value.as_slice()[0].to_string()),
        TypedSexp::Real(value) if value.len() == 1 => Ok(value.as_slice()[0].to_string()),
        TypedSexp::Logical(value) if value.len() == 1 => {
            Ok(if value.iter().next().unwrap_or(false) {
                "true".to_string()
            } else {
                "false".to_string()
            })
        }
        _ => Err(savvy::Error::new(&format!(
            "config value {name} must be a scalar string, number, or logical"
        ))),
    }
}

fn payloads_from_sexp(data: Sexp, n: usize) -> savvy::Result<Vec<Buffer>> {
    if let Some(buffer) = buffer_from_opendal_bytes_sexp(&data)? {
        return if n == 1 {
            Ok(vec![buffer])
        } else {
            Err(savvy::Error::new(
                "data must be a list of raw vectors or OpendalBytes when path has length greater than 1",
            ))
        };
    }

    match data.into_typed() {
        TypedSexp::Raw(raw) if n == 1 => Ok(vec![raw.to_vec().into()]),
        TypedSexp::Raw(_) => Err(savvy::Error::new(
            "data must be a list of raw vectors or OpendalBytes when path has length greater than 1",
        )),
        TypedSexp::List(list) => {
            if list.len() != n {
                return Err(savvy::Error::new("data length must match path length"));
            }
            let mut out = Vec::with_capacity(n);
            for value in list.values_iter() {
                if let Some(buffer) = buffer_from_opendal_bytes_sexp(&value)? {
                    out.push(buffer);
                    continue;
                }
                match value.into_typed() {
                    TypedSexp::Raw(raw) => out.push(raw.to_vec().into()),
                    _ => {
                        return Err(savvy::Error::new(
                            "each data element must be raw or OpendalBytes",
                        ));
                    }
                }
            }
            Ok(out)
        }
        _ => Err(savvy::Error::new(
            "data must be a raw vector, OpendalBytes, or list of raw vectors/OpendalBytes",
        )),
    }
}
