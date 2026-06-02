use std::sync::Arc;

use futures::{StreamExt, stream};
use opendal::options::{DeleteOptions, ListOptions};
use opendal::{Buffer, Metadata, Operator};
use savvy::savvy;
use savvy::{ListSexp, OwnedListSexp, Sexp, StringSexp, TypedSexp};

use crate::aio::{AioOutcome, OpendalAio};
use crate::common::{NativeFs, build_runtime, init_registry};
use crate::error::{kind_code, op_error_list, unsupported_value};
use crate::io_iter::{OpendalReadIter, OpendalWriteIter, checked_chunk_size, normalize_iter_path};
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
    ) -> savvy::Result<Self> {
        let mut pairs = Vec::new();
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
        Self::open_from_pairs(scheme, pairs)
    }

    /// Open an OpenDAL filesystem from a URI.
    /// @export
    fn from_uri(uri: &str) -> savvy::Result<Self> {
        init_registry();
        let op = Operator::from_uri(uri)
            .map_err(|e| savvy::Error::new(&format!("cannot open OpenDAL URI: {e}")))?;
        let info = op.info();
        let native = NativeFs {
            op,
            runtime: build_runtime()?,
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
            ("stat", caps.stat),
            ("read", caps.read),
            ("write", caps.write),
            ("replace", caps.write),
            ("append", caps.write && caps.write_can_append),
            ("mkdir", caps.create_dir),
            ("delete", caps.delete),
            ("copy", caps.copy),
            ("rename", caps.rename),
            ("ls", caps.list),
        ];
        let mut operations = OwnedListSexp::new(ops.len(), true)?;
        for (i, (name, supported)) in ops.iter().enumerate() {
            let mut one = OwnedListSexp::new(3, true)?;
            one.set_name_and_value(0, "supported", bool_scalar(*supported)?)?;
            one.set_name_and_value(
                1,
                "implementation",
                str_scalar(if *supported { "opendal" } else { "unsupported" })?,
            )?;
            one.set_name_and_value(2, "notes", str_scalar("")?)?;
            operations.set_name_and_value(i, name, one)?;
        }
        let mut out = OwnedListSexp::new(3, true)?;
        out.set_name_and_value(0, "scheme", str_scalar(&self.inner.scheme)?)?;
        out.set_name_and_value(1, "root", str_scalar(&self.inner.root)?)?;
        out.set_name_and_value(2, "operations", operations)?;
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
        let requests = read_requests_from_options(path, offset, size, end)?;
        let tuning = read_tuning(read_concurrency, chunk_size, coalesce_gap)?;
        self.read_requests(requests, result, batch_concurrency.unwrap_or(0.0), tuning)
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
        let requests = read_requests_from_options(path, offset, size, end)?;
        let tuning = read_tuning(read_concurrency, chunk_size, coalesce_gap)?;
        let n = requests.len();
        let concurrency = batch_concurrency_limit(batch_concurrency.unwrap_or(0.0), n)?;
        let op = self.inner.op.clone();
        let result = result.to_string();
        let handle = self.inner.runtime.spawn(async move {
            if n == 1 && result == "auto" {
                return read_request_outcome(op, requests.into_iter().next().unwrap(), tuning)
                    .await;
            }
            let mut values = stream::iter(requests.into_iter().enumerate())
                .map(|(i, req)| {
                    let op = op.clone();
                    async move { (i, read_request_outcome(op, req, tuning).await) }
                })
                .buffer_unordered(concurrency)
                .collect::<Vec<_>>()
                .await;
            values.sort_by_key(|(i, _)| *i);
            AioOutcome::Many(values.into_iter().map(|(_, value)| value).collect())
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
        self.write_many_common(
            path,
            data,
            true,
            false,
            "write",
            batch_concurrency.unwrap_or(0.0),
            tuning,
        )
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
            batch_concurrency.unwrap_or(0.0),
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
        self.write_many_common(
            path,
            data,
            false,
            true,
            "append",
            batch_concurrency.unwrap_or(0.0),
            tuning,
        )
    }

    /// Return metadata for path(s).
    /// @export
    fn stat(&self, path: StringSexp, batch_concurrency: Option<f64>) -> savvy::Result<savvy::Sexp> {
        let n = path.len();
        if n == 0 {
            return OwnedListSexp::new(0, false)?.into();
        }
        if n == 1 {
            return self.stat_one(path.iter().next().unwrap_or(""));
        }

        let concurrency = batch_concurrency_limit(batch_concurrency.unwrap_or(0.0), n)?;
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

    /// Check whether path(s) exist.
    /// @export
    fn exists(
        &self,
        path: StringSexp,
        batch_concurrency: Option<f64>,
    ) -> savvy::Result<savvy::Sexp> {
        let n = path.len();
        if n == 0 {
            return OwnedListSexp::new(0, false)?.into();
        }
        if n == 1 {
            return self.exists_one(path.iter().next().unwrap_or(""));
        }

        let concurrency = batch_concurrency_limit(batch_concurrency.unwrap_or(0.0), n)?;
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
        if n == 0 {
            return OwnedListSexp::new(0, false)?.into();
        }
        if n == 1 {
            return self.delete_one(path.iter().next().unwrap_or(""), recursive);
        }

        let concurrency = batch_concurrency_limit(batch_concurrency.unwrap_or(0.0), n)?;
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

    /// List entries under a directory.
    /// @export
    fn ls(&self, path: &str, recursive: bool) -> savvy::Result<savvy::Sexp> {
        let path = match normalize_user_path(path, true) {
            Ok(p) => p,
            Err(e) => return crate::error::error_list("InvalidArgument", 14, &e, "ls", path),
        };
        let op = self.inner.op.clone();
        let path_for_error = path.clone();
        let mut opts = ListOptions::default();
        opts.recursive = recursive;
        match self
            .inner
            .runtime
            .block_on(async move { op.list_options(&path, opts).await })
        {
            Ok(entries) => {
                let entries = entries
                    .iter()
                    .filter(|entry| entry.path() != "/" && !entry.path().is_empty())
                    .collect::<Vec<_>>();
                let mut out = OwnedListSexp::new(entries.len(), false)?;
                for (i, entry) in entries.iter().enumerate() {
                    out.set_value(i, metadata_list(entry.path(), entry.metadata())?)?;
                }
                out.into()
            }
            Err(e) => op_error_list(e, "ls", &path_for_error),
        }
    }
}

impl OpendalFs {
    fn open_from_pairs(scheme: &str, config: Vec<(String, String)>) -> savvy::Result<Self> {
        init_registry();
        let op = Operator::via_iter(scheme, config)
            .map_err(|e| savvy::Error::new(&format!("cannot open OpenDAL operator: {e}")))?;
        let info = op.info();
        let native = NativeFs {
            op,
            runtime: build_runtime()?,
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
        requests: Vec<ReadRequest>,
        result: &str,
        batch_concurrency: f64,
        tuning: ReadTuning,
    ) -> savvy::Result<savvy::Sexp> {
        let n = requests.len();
        if n == 0 {
            return OwnedListSexp::new(0, false)?.into();
        }
        if n == 1 && result == "auto" {
            let req = requests.into_iter().next().unwrap();
            return read_value_to_sexp(self.inner.runtime.block_on(read_request(
                self.inner.op.clone(),
                req,
                tuning,
            )));
        }

        let concurrency = batch_concurrency_limit(batch_concurrency, n)?;
        let op = self.inner.op.clone();
        let mut values = self.inner.runtime.block_on(async move {
            stream::iter(requests.into_iter().enumerate())
                .map(|(i, req)| {
                    let op = op.clone();
                    async move { (i, read_request(op, req, tuning).await) }
                })
                .buffer_unordered(concurrency)
                .collect::<Vec<_>>()
                .await
        });
        values.sort_by_key(|(i, _)| *i);

        let mut out = OwnedListSexp::new(n, false)?;
        for (i, value) in values.into_iter() {
            out.set_value(i, read_value_to_sexp(value)?)?;
        }
        out.into()
    }

    fn write_many_common(
        &self,
        paths: StringSexp,
        data: Sexp,
        create_only: bool,
        append: bool,
        operation: &str,
        batch_concurrency: f64,
        tuning: WriteTuning,
    ) -> savvy::Result<savvy::Sexp> {
        let n = paths.len();
        let payloads = payloads_from_sexp(data, n)?;
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

        let concurrency = batch_concurrency_limit(batch_concurrency, n)?;
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

struct ReadRequest {
    path: String,
    offset: u64,
    size: Option<u64>,
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
    let path_for_error = req.path.clone();
    match read_bytes_with(op, req.path, req.offset, req.size, tuning).await {
        Ok(bytes) => AioOutcome::Bytes(bytes),
        Err(e) => {
            let kind = e.kind();
            AioOutcome::Error {
                kind: kind.into_static().to_string(),
                code: kind_code(kind),
                message: e.to_string(),
                operation: "read".to_string(),
                path: path_for_error,
            }
        }
    }
}

async fn read_request(op: Operator, req: ReadRequest, tuning: ReadTuning) -> ReadValue {
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
    match value {
        ReadValue::Bytes(bytes) => buffer_to_raw_sexp(bytes).map(|x| x.into()),
        ReadValue::Error {
            kind,
            code,
            message,
            path,
        } => crate::error::error_list(&kind, code, &message, "read", &path),
    }
}

fn batch_concurrency_limit(value: f64, n: usize) -> savvy::Result<usize> {
    if n == 0 {
        return Ok(1);
    }
    if value == 0.0 {
        return Ok(n.min(16));
    }
    let checked = checked_u64(value, "batch_concurrency")?;
    if checked == 0 {
        Ok(n.min(16))
    } else {
        Ok((checked as usize).max(1).min(n))
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
) -> savvy::Result<Vec<ReadRequest>> {
    if sizes.is_some() && ends.is_some() {
        return Err(savvy::Error::new("use only one of size or end"));
    }
    let n = paths.len();
    let offset_values = expand_numeric_arg(offsets, n, 0.0, "offset")?;
    let size_values = optional_numeric_arg(sizes, "size")?;
    let end_values = optional_numeric_arg(ends, "end")?;
    if let Some(values) = &size_values {
        if values.len() != n {
            return Err(savvy::Error::new("size length must match path length"));
        }
    }
    if let Some(values) = &end_values {
        if values.len() != n {
            return Err(savvy::Error::new("end length must match path length"));
        }
    }

    let mut out = Vec::with_capacity(n);
    for (i, path) in paths.iter().enumerate() {
        let path = match normalize_user_path(path, false) {
            Ok(p) => p,
            Err(e) => return Err(savvy::Error::new(&e)),
        };
        let offset = checked_u64(offset_values[i], "offset")?;
        let size = if let Some(values) = &size_values {
            Some(checked_u64(values[i], "size")?)
        } else if let Some(values) = &end_values {
            Some(checked_u64(values[i] - offset_values[i], "size")?)
        } else {
            None
        };
        out.push(ReadRequest { path, offset, size });
    }
    Ok(out)
}

fn expand_numeric_arg(
    value: Option<Sexp>,
    n: usize,
    default: f64,
    name: &str,
) -> savvy::Result<Vec<f64>> {
    let Some(value) = value else {
        return Ok(vec![default; n]);
    };
    let values = numeric_arg(value, name)?;
    if values.len() != n {
        return Err(savvy::Error::new(&format!(
            "{name} length must match path length"
        )));
    }
    Ok(values)
}

fn optional_numeric_arg(value: Option<Sexp>, name: &str) -> savvy::Result<Option<Vec<f64>>> {
    value.map(|value| numeric_arg(value, name)).transpose()
}

fn numeric_arg(value: Sexp, name: &str) -> savvy::Result<Vec<f64>> {
    match value.into_typed() {
        TypedSexp::Null(_) => Ok(Vec::new()),
        TypedSexp::Real(real) => Ok(real.as_slice().to_vec()),
        TypedSexp::Integer(int) => Ok(int.as_slice().iter().map(|v| *v as f64).collect()),
        _ => Err(savvy::Error::new(&format!("{name} must be numeric"))),
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
    match data.into_typed() {
        TypedSexp::Raw(raw) if n == 1 => Ok(vec![raw.to_vec().into()]),
        TypedSexp::Raw(_) => Err(savvy::Error::new(
            "data must be a list of raw vectors when path has length greater than 1",
        )),
        TypedSexp::List(list) => {
            if list.len() != n {
                return Err(savvy::Error::new("data length must match path length"));
            }
            let mut out = Vec::with_capacity(n);
            for value in list.values_iter() {
                match value.into_typed() {
                    TypedSexp::Raw(raw) => out.push(raw.to_vec().into()),
                    _ => return Err(savvy::Error::new("each data element must be raw")),
                }
            }
            Ok(out)
        }
        _ => Err(savvy::Error::new(
            "data must be a raw vector or list of raw vectors",
        )),
    }
}
