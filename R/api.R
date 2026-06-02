#' Ropendal filesystem API
#'
#' Byte-oriented filesystem operations backed by Apache OpenDAL. The current
#' implementation includes local `fs`, HTTP, S3-compatible, and Google Drive
#' handles; raw byte operations; metadata and listing where supported; error
#' values; read Aio handles; and a pure C API.
#'
#' @name Ropendal-api
#' @aliases CredentialProvider opendal opendal_uri credentials_s3 credentials_gdrive
#'   credentials_gdrive3 credential_schemes credential_config credential_summary
#'   fs_info fs_capabilities fs_normalize_path fs_read fs_read_aio fs_read_iter
#'   read_iter_next read_iter_collect fs_seek fs_tell fs_write fs_write_aio
#'   fs_replace fs_replace_aio fs_append fs_append_aio fs_write_iter
#'   write_iter_write write_iter_close fs_stat fs_stat_aio fs_stats
#'   fs_stats_aio fs_exists fs_exists_aio fs_ls fs_ls_aio fs_mkdir fs_mkdir_aio fs_delete
#'   fs_delete_aio fs_copy fs_copy_aio fs_rename fs_rename_aio collect_aio
#'   call_aio stop_aio poll_aio unresolved is_error_value
#'   error_kind error_message error_operation error_path
#' @param scheme OpenDAL service scheme.
#' @param ... Named service configuration entries.
#' @param root Root path/prefix for the service.
#' @param config Named list of service configuration entries.
#' @param auth Explicit credential provider.
#' @param uri OpenDAL URI.
#' @param access_key_id,secret_access_key S3-compatible access key fields.
#' @param session_token Optional S3-compatible session token.
#' @param region Optional S3-compatible signing region.
#' @param access_token Google Drive access token, mutually exclusive with
#'   `refresh_token`.
#' @param refresh_token Google Drive refresh token.
#' @param client_id,client_secret OAuth client fields required with
#'   `refresh_token`.
#' @param source Redacted source label for a credential provider.
#' @param secret_json Path to a JSON file containing Google Drive `client_id`
#'   and `client_secret`.
#' @param tokens_json Path to a token JSON file containing a refresh token.
#' @param scope Scope used to choose a refresh token from `tokens_json`.
#' @param provider Credential provider object.
#' @param service OpenDAL service scheme for credential materialization.
#' @param fs Ropendal filesystem handle.
#' @param path Root-relative path or paths.
#' @param directory Whether to normalize as a directory path.
#' @param offset Zero-based byte offset.
#' @param size Number of bytes to read, or `NULL` to read to EOF.
#' @param end Exclusive byte end offset.
#' @param result Requested result shape.
#' @param batch_concurrency Optional maximum number of independent paths/ranges
#'   to process concurrently.
#' @param read_concurrency Optional per-object OpenDAL read concurrency for large
#'   reads where the backend supports chunked/concurrent reads.
#' @param write_concurrency Optional per-object OpenDAL write concurrency for
#'   large writes where the backend supports multipart/concurrent writes.
#' @param chunk_size Optional read/write chunk size in bytes for OpenDAL's
#'   per-object transfer planning.
#' @param coalesce_gap Optional byte gap for coalescing nearby read ranges.
#' @param data Raw vector, or list of raw vectors for multiple paths.
#' @param iter Read or write iterator handle.
#' @param whence Seek origin for read iterators: iterator `start`, `current`, or `end`.
#' @param create Whether a write iterator should create only and fail if the target exists.
#' @param append Whether a write iterator should append rather than replace.
#' @param recursive Whether to recurse for operations that support it.
#' @param from,to Source and destination paths.
#' @param aio Aio handle.
#' @param x Object to inspect.
#' @usage
#' opendal(scheme = "fs", ..., root = NULL, config = list(), auth = NULL)
#' opendal_uri(uri)
#' credentials_s3(access_key_id, secret_access_key,
#'                session_token = "", region = "", source = "direct")
#' credentials_gdrive(access_token = "", refresh_token = "",
#'                    client_id = "", client_secret = "", source = "direct")
#' credentials_gdrive3(secret_json,
#'                     tokens_json = file.path(dirname(secret_json), "tokens.json"),
#'                     scope = "https://www.googleapis.com/auth/drive")
#' credential_schemes(provider)
#' credential_config(provider, service)
#' credential_summary(provider)
#' fs_info(fs)
#' fs_capabilities(fs)
#' fs_normalize_path(fs, path, directory = FALSE)
#' fs_read(fs, path, offset = 0, size = NULL, end = NULL,
#'         result = c("auto", "flat", "nested"), batch_concurrency = NULL,
#'         read_concurrency = NULL, chunk_size = NULL, coalesce_gap = NULL)
#' fs_read_aio(fs, path, offset = 0, size = NULL, end = NULL,
#'             result = c("auto", "flat", "nested"), batch_concurrency = NULL,
#'             read_concurrency = NULL, chunk_size = NULL, coalesce_gap = NULL)
#' fs_read_iter(fs, path, chunk_size, offset = 0, size = NULL,
#'              read_concurrency = NULL, coalesce_gap = NULL)
#' read_iter_next(iter)
#' read_iter_collect(iter)
#' fs_tell(iter)
#' fs_seek(iter, offset, whence = c("start", "current", "end"))
#' fs_write(fs, path, data, batch_concurrency = NULL,
#'          write_concurrency = NULL, chunk_size = NULL)
#' fs_write_aio(fs, path, data, batch_concurrency = NULL,
#'              write_concurrency = NULL, chunk_size = NULL)
#' fs_replace(fs, path, data, batch_concurrency = NULL,
#'            write_concurrency = NULL, chunk_size = NULL)
#' fs_replace_aio(fs, path, data, batch_concurrency = NULL,
#'                write_concurrency = NULL, chunk_size = NULL)
#' fs_append(fs, path, data, batch_concurrency = NULL,
#'           write_concurrency = NULL, chunk_size = NULL)
#' fs_append_aio(fs, path, data, batch_concurrency = NULL,
#'               write_concurrency = NULL, chunk_size = NULL)
#' fs_write_iter(fs, path, create = TRUE, append = FALSE,
#'               write_concurrency = NULL, chunk_size = NULL)
#' write_iter_write(iter, data)
#' write_iter_close(iter)
#' fs_stat(fs, path, batch_concurrency = NULL)
#' fs_stat_aio(fs, path, batch_concurrency = NULL)
#' fs_stats(fs, path, batch_concurrency = NULL)
#' fs_stats_aio(fs, path, batch_concurrency = NULL)
#' fs_exists(fs, path, batch_concurrency = NULL)
#' fs_exists_aio(fs, path, batch_concurrency = NULL)
#' fs_ls(fs, path = "", recursive = FALSE)
#' fs_ls_aio(fs, path = "", recursive = FALSE)
#' fs_mkdir(fs, path)
#' fs_mkdir_aio(fs, path, batch_concurrency = NULL)
#' fs_delete(fs, path, recursive = FALSE, batch_concurrency = NULL)
#' fs_delete_aio(fs, path, recursive = FALSE, batch_concurrency = NULL)
#' fs_copy(fs, from, to)
#' fs_copy_aio(fs, from, to, batch_concurrency = NULL)
#' fs_rename(fs, from, to)
#' fs_rename_aio(fs, from, to, batch_concurrency = NULL)
#' collect_aio(aio)
#' call_aio(aio)
#' stop_aio(aio)
#' poll_aio(aio)
#' unresolved(x = NULL)
#' is_error_value(x)
#' error_kind(x)
#' error_message(x)
#' error_operation(x)
#' error_path(x)
#' @details
#' Filesystem failures are returned as classed error values. Invalid arguments
#' and internal runtime failures may signal R errors. Credential helpers return
#' classed providers with redacted printing; pass them with `auth =`.
#'
#' Aio handles expose read-only active bindings. `$value` returns an
#' `unresolvedValue` while the operation is pending and the resolved value or
#' error value afterwards. `$data` and `$result` are aliases for `$value`, while
#' `$state`, `$resolved`, and `$error` expose readiness and error inspection.
#' `collect_aio()` waits and returns the value. `call_aio()` waits, updates the
#' Aio, and returns the Aio invisibly. `unresolved()` constructs the unresolved
#' sentinel; `unresolved(aio)` and `unresolved(value)` are predicates.
#' @return Filesystem handles, raw vectors, metadata lists, logical results,
#'   Aio handles, or classed error values depending on the operation.
NULL

#' Credential provider interface.
#' @export
#' @noRd
CredentialProvider <- S7::new_class(
  "CredentialProvider",
  package = "Ropendal",
  properties = list(
    native = S7::class_any,
    schemes = S7::class_character
  ),
  validator = function(self) {
    if (!length(self@schemes) || any(!nzchar(self@schemes))) {
      "@schemes must contain non-empty service scheme names"
    }
  }
)

#' @export
#' @noRd
credential_schemes <- S7::new_generic(
  "credential_schemes",
  "provider",
  function(provider) S7::S7_dispatch()
)

#' @export
#' @noRd
credential_config <- S7::new_generic(
  "credential_config",
  "provider",
  function(provider, service) S7::S7_dispatch()
)

#' @export
#' @noRd
credential_summary <- S7::new_generic(
  "credential_summary",
  "provider",
  function(provider) S7::S7_dispatch()
)

.onLoad <- function(libname, pkgname) {
  if (!identical(pkgname, "Ropendal")) return(invisible())

  S7::method(credential_schemes, CredentialProvider) <- function(provider) {
    provider@native$schemes()
  }

  S7::method(credential_config, CredentialProvider) <- function(provider, service) {
    provider@native$config(service)
  }

  S7::method(credential_summary, CredentialProvider) <- function(provider) {
    provider@native$summary()
  }

  S7::method(print, CredentialProvider) <- print_credential_provider

  invisible()
}

#' @export
#' @noRd
opendal <- function(scheme = "fs", ..., root = NULL, config = list(), auth = NULL) {
  OpendalFs$open(
    scheme,
    list(...),
    config,
    root,
    if (is.null(auth)) NULL else credential_config(auth, scheme)
  )
}

#' @export
#' @noRd
opendal_uri <- function(uri) OpendalFs$from_uri(uri)

#' @export
#' @noRd
credentials_s3 <- function(access_key_id, secret_access_key,
                           session_token = "", region = "",
                           source = "direct") {
  CredentialProvider(
    native = OpendalCredentialProvider$s3(
      access_key_id,
      secret_access_key,
      session_token,
      region,
      source
    ),
    schemes = "s3"
  )
}

#' @export
#' @noRd
credentials_gdrive <- function(access_token = "", refresh_token = "",
                               client_id = "", client_secret = "",
                               source = "direct") {
  CredentialProvider(
    native = OpendalCredentialProvider$gdrive(
      access_token,
      refresh_token,
      client_id,
      client_secret,
      source
    ),
    schemes = "gdrive"
  )
}

#' @export
#' @noRd
credentials_gdrive3 <- function(secret_json,
                                tokens_json = file.path(dirname(secret_json), "tokens.json"),
                                scope = "https://www.googleapis.com/auth/drive") {
  CredentialProvider(
    native = OpendalCredentialProvider$gdrive3(secret_json, tokens_json, scope),
    schemes = "gdrive"
  )
}

print_credential_provider <- function(x, ...) {
  summary <- credential_summary(x)
  cat("<opendal credential provider>", paste(credential_schemes(x), collapse = ","), summary$method, "source=", summary$source, "secrets=<redacted>\n")
  invisible(x)
}

#' @export
#' @noRd
is_error_value <- function(x) inherits(x, "opendalErrorValue")

#' @export
#' @noRd
error_kind <- function(x) {
  if (!is_error_value(x)) stop("not an opendal error value", call. = FALSE)
  x$kind
}

#' @export
#' @noRd
error_message <- function(x) {
  if (!is_error_value(x)) stop("not an opendal error value", call. = FALSE)
  x$message
}

#' @export
#' @noRd
error_operation <- function(x) {
  if (!is_error_value(x)) stop("not an opendal error value", call. = FALSE)
  x$operation
}

#' @export
#' @noRd
error_path <- function(x) {
  if (!is_error_value(x)) stop("not an opendal error value", call. = FALSE)
  x$path
}

#' @export
#' @noRd
print.opendalErrorValue <- function(x, ...) {
  cat("<opendal error ", x$kind, "> ", x$message, "\n", sep = "")
  invisible(x)
}

#' @export
#' @noRd
fs_info <- function(fs) fs$info()

#' @export
#' @noRd
fs_capabilities <- function(fs) fs$capabilities()

#' @export
#' @noRd
fs_normalize_path <- function(fs, path, directory = FALSE) {
  fs$normalize_path(path, directory)
}

#' @export
#' @noRd
fs_read <- function(fs, path, offset = 0, size = NULL, end = NULL,
                    result = c("auto", "flat", "nested"),
                    batch_concurrency = NULL,
                    read_concurrency = NULL,
                    chunk_size = NULL,
                    coalesce_gap = NULL) {
  fs$read(
    path,
    offset,
    size,
    end,
    match.arg(result),
    batch_concurrency,
    read_concurrency,
    chunk_size,
    coalesce_gap
  )
}

#' @export
#' @noRd
fs_read_aio <- function(fs, path, offset = 0, size = NULL, end = NULL,
                        result = c("auto", "flat", "nested"),
                        batch_concurrency = NULL,
                        read_concurrency = NULL,
                        chunk_size = NULL,
                        coalesce_gap = NULL) {
  opendal_aio_with_bindings(fs$read_aio(
    path,
    offset,
    size,
    end,
    match.arg(result),
    batch_concurrency,
    read_concurrency,
    chunk_size,
    coalesce_gap
  ))
}

#' @export
#' @noRd
fs_write <- function(fs, path, data, batch_concurrency = NULL,
                     write_concurrency = NULL, chunk_size = NULL) {
  fs$write(path, data, batch_concurrency, write_concurrency, chunk_size)
}

#' @export
#' @noRd
fs_write_aio <- function(fs, path, data, batch_concurrency = NULL,
                         write_concurrency = NULL, chunk_size = NULL) {
  opendal_aio_with_bindings(
    fs$write_aio(path, data, batch_concurrency, write_concurrency, chunk_size)
  )
}

#' @export
#' @noRd
fs_replace <- function(fs, path, data, batch_concurrency = NULL,
                       write_concurrency = NULL, chunk_size = NULL) {
  fs$replace(path, data, batch_concurrency, write_concurrency, chunk_size)
}

#' @export
#' @noRd
fs_replace_aio <- function(fs, path, data, batch_concurrency = NULL,
                           write_concurrency = NULL, chunk_size = NULL) {
  opendal_aio_with_bindings(
    fs$replace_aio(path, data, batch_concurrency, write_concurrency, chunk_size)
  )
}

#' @export
#' @noRd
fs_append <- function(fs, path, data, batch_concurrency = NULL,
                      write_concurrency = NULL, chunk_size = NULL) {
  fs$append(path, data, batch_concurrency, write_concurrency, chunk_size)
}

#' @export
#' @noRd
fs_append_aio <- function(fs, path, data, batch_concurrency = NULL,
                          write_concurrency = NULL, chunk_size = NULL) {
  opendal_aio_with_bindings(
    fs$append_aio(path, data, batch_concurrency, write_concurrency, chunk_size)
  )
}

#' @export
#' @noRd
fs_read_iter <- function(fs, path, chunk_size, offset = 0, size = NULL,
                         read_concurrency = NULL, coalesce_gap = NULL) {
  n <- length(path)
  if (n == 0L) return(list())
  one <- function(i) {
    fs$read_iter(
      path[[i]],
      scalar_or_at(chunk_size, i, n, "chunk_size"),
      scalar_or_at(offset, i, n, "offset"),
      null_or_scalar_or_at(size, i, n, "size"),
      read_concurrency,
      coalesce_gap
    )
  }
  if (n == 1L) one(1L) else lapply(seq_len(n), one)
}

#' @export
#' @noRd
read_iter_next <- function(iter) iter[["next"]]()

#' @export
#' @noRd
read_iter_collect <- function(iter) iter$collect()

#' @export
#' @noRd
fs_tell <- function(iter) {
  if (is.null(iter$tell)) stop("object does not support fs_tell()", call. = FALSE)
  iter$tell()
}

#' @export
#' @noRd
fs_seek <- function(iter, offset, whence = c("start", "current", "end")) {
  if (is.null(iter$seek)) stop("object does not support fs_seek()", call. = FALSE)
  iter$seek(offset, match.arg(whence))
}

#' @export
#' @noRd
fs_write_iter <- function(fs, path, create = TRUE, append = FALSE,
                          write_concurrency = NULL, chunk_size = NULL) {
  n <- length(path)
  if (n == 0L) return(list())
  one <- function(i) {
    fs$write_iter(
      path[[i]],
      scalar_or_at(create, i, n, "create"),
      scalar_or_at(append, i, n, "append"),
      write_concurrency,
      chunk_size
    )
  }
  if (n == 1L) one(1L) else lapply(seq_len(n), one)
}

#' @export
#' @noRd
write_iter_write <- function(iter, data) iter$write(data)

#' @export
#' @noRd
write_iter_close <- function(iter) iter$close()

scalar_or_at <- function(x, i, n, name) {
  if (length(x) == 1L) return(x[[1L]])
  if (length(x) == n) return(x[[i]])
  stop(name, " must have length 1 or match path length", call. = FALSE)
}

null_or_scalar_or_at <- function(x, i, n, name) {
  if (is.null(x)) return(NULL)
  scalar_or_at(x, i, n, name)
}

#' @export
#' @noRd
fs_stat <- function(fs, path, batch_concurrency = NULL) {
  fs$stat(path, batch_concurrency)
}

#' @export
#' @noRd
fs_stat_aio <- function(fs, path, batch_concurrency = NULL) {
  opendal_aio_with_bindings(fs$stat_aio(path, batch_concurrency))
}

#' @export
#' @noRd
fs_stats <- function(fs, path, batch_concurrency = NULL) {
  fs_stat(fs, path, batch_concurrency)
}

#' @export
#' @noRd
fs_stats_aio <- function(fs, path, batch_concurrency = NULL) {
  fs_stat_aio(fs, path, batch_concurrency)
}

#' @export
#' @noRd
fs_exists <- function(fs, path, batch_concurrency = NULL) {
  fs$exists(path, batch_concurrency)
}

#' @export
#' @noRd
fs_exists_aio <- function(fs, path, batch_concurrency = NULL) {
  opendal_aio_with_bindings(fs$exists_aio(path, batch_concurrency))
}

#' @export
#' @noRd
fs_ls <- function(fs, path = "", recursive = FALSE) {
  fs$ls(path, recursive)
}

#' @export
#' @noRd
fs_ls_aio <- function(fs, path = "", recursive = FALSE) {
  opendal_aio_with_bindings(fs$ls_aio(path, recursive))
}

#' @export
#' @noRd
fs_mkdir <- function(fs, path) {
  fs$mkdir(path)
}

#' @export
#' @noRd
fs_mkdir_aio <- function(fs, path, batch_concurrency = NULL) {
  opendal_aio_with_bindings(fs$mkdir_aio(path, batch_concurrency))
}

#' @export
#' @noRd
fs_delete <- function(fs, path, recursive = FALSE, batch_concurrency = NULL) {
  fs$delete(path, recursive, batch_concurrency)
}

#' @export
#' @noRd
fs_delete_aio <- function(fs, path, recursive = FALSE, batch_concurrency = NULL) {
  opendal_aio_with_bindings(fs$delete_aio(path, recursive, batch_concurrency))
}

#' @export
#' @noRd
fs_copy <- function(fs, from, to) fs$copy(from, to)

#' @export
#' @noRd
fs_copy_aio <- function(fs, from, to, batch_concurrency = NULL) {
  opendal_aio_with_bindings(fs$copy_aio(from, to, batch_concurrency))
}

#' @export
#' @noRd
fs_rename <- function(fs, from, to) fs$rename(from, to)

#' @export
#' @noRd
fs_rename_aio <- function(fs, from, to, batch_concurrency = NULL) {
  opendal_aio_with_bindings(fs$rename_aio(from, to, batch_concurrency))
}

opendal_aio_with_bindings <- function(aio) {
  for (name in c("value", "data", "result", "state", "resolved", "error")) {
    if (!exists(name, envir = aio, inherits = FALSE)) {
      makeActiveBinding(name, opendal_aio_binding(aio, name), aio)
    }
  }
  aio
}

opendal_aio_binding <- function(aio, name) {
  force(aio)
  force(name)
  function(value) {
    if (nargs()) stop("$", name, " is read-only", call. = FALSE)
    switch(name,
      value = opendal_aio_value(aio),
      data = opendal_aio_value(aio),
      result = opendal_aio_value(aio),
      state = aio$state_name(),
      resolved = opendal_aio_resolved(aio),
      error = aio$error_value()
    )
  }
}

opendal_aio_value <- function(aio) {
  if (identical(aio$state_name(), "pending")) unresolved() else aio$collect()
}

opendal_aio_resolved <- function(aio) {
  !identical(aio$state_name(), "pending")
}

savvy_wrap_OpendalAio_generated <- .savvy_wrap_OpendalAio
.savvy_wrap_OpendalAio <- function(ptr) {
  opendal_aio_with_bindings(savvy_wrap_OpendalAio_generated(ptr))
}

#' @export
#' @noRd
unresolved <- function(x = NULL) {
  if (missing(x)) return(structure(NA, class = "unresolvedValue"))
  if (inherits(x, "OpendalAio")) return(!opendal_aio_resolved(x))
  inherits(x, "unresolvedValue")
}

#' @export
#' @noRd
collect_aio <- function(aio) {
  aio$collect()
}

#' @export
#' @noRd
call_aio <- function(aio) {
  collect_aio(aio)
  invisible(aio)
}

#' @export
#' @noRd
stop_aio <- function(aio) {
  aio$cancel()
}

#' @export
#' @noRd
poll_aio <- function(aio) {
  aio$poll()
}

#' @export
#' @noRd
print.OpendalFs <- function(x, ...) {
  info <- fs_info(x)
  cat("<opendal filesystem>", info$scheme, info$root, "\n")
  invisible(x)
}

#' @export
#' @noRd
print.OpendalAio <- function(x, ...) {
  cat("<opendal aio>", poll_aio(x), "\n")
  invisible(x)
}

#' @export
#' @noRd
print.OpendalReadIter <- function(x, ...) {
  cat("<opendal read iterator>\n")
  invisible(x)
}

#' @export
#' @noRd
print.OpendalWriteIter <- function(x, ...) {
  cat("<opendal write iterator>\n")
  invisible(x)
}
