#' Ropendal filesystem API
#'
#' Byte-oriented filesystem operations backed by Apache OpenDAL. The current
#' implementation includes local `fs`, HTTP, S3-compatible, and Google Drive
#' handles; raw byte operations; metadata and listing where supported; error
#' values; read Aio handles; and a pure C API.
#'
#' @name Ropendal-api
#' @aliases CredentialProvider OpendalBytes opendal opendal_uri credentials_s3 credentials_gcs
#'   credentials_azblob credentials_gdrive credentials_gdrive3 credential_schemes
#'   credential_config credential_summary runtime_config layer_concurrent_limit
#'   opt opt<- serial_config codec_config serialize_raw deserialize_raw
#'   fs_info fs_capabilities fs_normalize_path fs_read fs_read_aio
#'   fs_read_bytes fs_read_bytes_aio as.raw.OpendalBytes length.OpendalBytes
#'   fs_read_iter read_iter_next read_iter_collect fs_ls_iter fs_walk_iter ls_iter_next
#'   ls_iter_collect walk_iter_next walk_iter_collect fs_seek fs_tell fs_write fs_write_aio
#'   fs_replace fs_replace_aio fs_append fs_append_aio fs_write_iter
#'   write_iter_write write_iter_close fs_stat fs_stat_aio fs_stats
#'   fs_stats_aio fs_exists fs_exists_aio fs_ls fs_ls_aio fs_mkdir fs_mkdir_aio fs_delete
#'   fs_delete_aio fs_copy fs_copy_aio fs_rename fs_rename_aio collect_aio
#'   collect_aio_ call_aio call_aio_ stop_aio poll_aio cv cv_value
#'   cv_reset cv_signal cv_wait cv_until aio_monitor read_monitor race_aio
#'   unresolved is_error_value
#'   error_kind error_message error_operation error_path
#' @export OpendalBytes
#' @param scheme OpenDAL service scheme.
#' @param ... Named service configuration entries.
#' @param root Root path/prefix for the service.
#' @param config For `opendal()`, named list of service configuration entries.
#'   For `serialize_raw()` / `deserialize_raw()`, serialization config.
#' @param auth Explicit credential provider.
#' @param headers Named scalar character list/vector of HTTP headers for
#'   `http`/`https` filesystems; values may contain credentials and are not
#'   printed by Ropendal.
#' @param runtime Runtime configuration from `runtime_config()`.
#' @param layers List of explicit filesystem layers, currently including
#'   `layer_concurrent_limit()`.
#' @param threads Number of Tokio worker threads for a filesystem handle.
#' @param max Maximum total in-flight backend operations for a filesystem handle.
#' @param uri OpenDAL URI.
#' @param access_key_id,secret_access_key S3-compatible access key fields.
#' @param session_token Optional S3-compatible session token.
#' @param region Optional S3-compatible signing region.
#' @param token Google Cloud Storage OAuth2 bearer token.
#' @param service_account_key Google Cloud Storage service-account JSON string.
#' @param credential_path Explicit path to Google Cloud Storage credential JSON.
#' @param scope Optional Google Cloud Storage OAuth scope.
#' @param account_name Azure Blob Storage account name.
#' @param account_key Azure Blob Storage account key.
#' @param sas_token Azure Blob Storage SAS token.
#' @param endpoint Optional Azure Blob Storage endpoint.
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
#' @param name Option name.
#' @param value Option value.
#' @param class Class name or names matched for custom serialization.
#' @param sfunc,ufunc Serializer and deserializer functions for
#'   `serial_config()`. R-closure codecs are not supported; codec transforms
#'   are native byte transforms.
#' @param mode Materialization mode: raw bytes, text strings, serialized R
#'   objects, or raw bytes passed through an explicit codec.
#' @param encoding Text encoding for `mode = "text"` reads and writes.
#'   Encodings whose encoded bytes contain NUL bytes are rejected; use raw mode
#'   for those storage formats.
#' @param serial_config Serialization config, normally `opt(fs, "serial")`.
#' @param codec Optional native byte codec name or `codec_config()` object,
#'   normally `opt(fs, "codec")`.
#' @param batch_concurrency Optional maximum number of independent paths/ranges
#'   to process concurrently.
#' @param read_concurrency Optional per-object OpenDAL read concurrency for large
#'   reads where the backend supports chunked/concurrent reads.
#' @param write_concurrency Optional per-object OpenDAL write concurrency for
#'   large writes where the backend supports multipart/concurrent writes.
#' @param chunk_size Optional read/write chunk size in bytes for OpenDAL's
#'   per-object transfer planning.
#' @param coalesce_gap Optional byte gap for coalescing nearby read ranges.
#' @param data Raw vector, `OpendalBytes` handle, or list mixing raw vectors
#'   and `OpendalBytes` handles for multiple paths in raw mode; arbitrary R
#'   object or list of objects in serial mode.
#' @param iter Read, write, listing, or walking iterator handle.
#' @param whence Seek origin for read iterators: iterator `start`, `current`, or `end`.
#' @param create Whether a write iterator should create only and fail if the target exists.
#' @param append Whether a write iterator should append rather than replace.
#' @param recursive Whether to recurse for operations that support it.
#' @param limit Optional maximum number of listing entries to materialize or
#'   return across an iterator.
#' @param start_after Optional root-relative listing continuation marker;
#'   entries less than or equal to this path are skipped where supported.
#'   Iterator `$cursor` values are last yielded paths and can be used as
#'   best-effort `start_after` markers for lexically ordered listings; they are
#'   not opaque backend continuation tokens.
#' @param page_size Maximum number of entries returned by one iterator page.
#' @param from,to Source and destination paths.
#' @param aio Aio handle.
#' @param cv Condition variable object from `cv()`.
#' @param monitor Monitor handle from `aio_monitor()`.
#' @param msec Milliseconds to wait for `cv_until()`.
#' @param timeout Optional milliseconds to wait for `race_aio()`.
#' @param interval Poll interval in seconds for R-thread wait helpers.
#' @param x Object to inspect.
#' @usage
#' opendal(scheme = "fs", ..., root = NULL, config = list(), auth = NULL,
#'         headers = NULL, runtime = runtime_config(), layers = list())
#' opendal_uri(uri, headers = NULL, runtime = runtime_config(), layers = list())
#' credentials_s3(access_key_id, secret_access_key,
#'                session_token = "", region = "", source = "direct")
#' credentials_gcs(token = "", service_account_key = "",
#'                 credential_path = "", scope = "", source = "direct")
#' credentials_azblob(account_name = "", account_key = "", sas_token = "",
#'                    endpoint = "", source = "direct")
#' credentials_gdrive(access_token = "", refresh_token = "",
#'                    client_id = "", client_secret = "", source = "direct")
#' credentials_gdrive3(secret_json,
#'                     tokens_json = file.path(dirname(secret_json), "tokens.json"),
#'                     scope = "https://www.googleapis.com/auth/drive")
#' credential_schemes(provider)
#' credential_config(provider, service)
#' credential_summary(provider)
#' runtime_config(threads = NULL)
#' layer_concurrent_limit(max)
#' fs_info(fs)
#' fs_capabilities(fs)
#' fs_normalize_path(fs, path, directory = FALSE)
#' opt(fs, name)
#' opt(fs, name) <- value
#' serial_config(class, sfunc, ufunc)
#' codec_config(name, class = "raw", sfunc = NULL, ufunc = NULL)
#' serialize_raw(x, config = list())
#' deserialize_raw(x, config = list())
#' fs_read(fs, path, offset = 0, size = NULL, end = NULL,
#'         result = c("auto", "flat", "nested"), batch_concurrency = NULL,
#'         read_concurrency = NULL, chunk_size = NULL, coalesce_gap = NULL,
#'         mode = c("raw", "serial", "text", "codec"), encoding = "UTF-8",
#'         serial_config = opt(fs, "serial"), codec = opt(fs, "codec"))
#' fs_read_aio(fs, path, offset = 0, size = NULL, end = NULL,
#'             result = c("auto", "flat", "nested"), batch_concurrency = NULL,
#'             read_concurrency = NULL, chunk_size = NULL, coalesce_gap = NULL,
#'             mode = c("raw", "serial", "text", "codec"), encoding = "UTF-8",
#'             serial_config = opt(fs, "serial"), codec = opt(fs, "codec"))
#' fs_read_bytes(fs, path, offset = 0, size = NULL, end = NULL,
#'               result = c("auto", "flat", "nested"), batch_concurrency = NULL,
#'               read_concurrency = NULL, chunk_size = NULL, coalesce_gap = NULL)
#' fs_read_bytes_aio(fs, path, offset = 0, size = NULL, end = NULL,
#'                   result = c("auto", "flat", "nested"), batch_concurrency = NULL,
#'                   read_concurrency = NULL, chunk_size = NULL, coalesce_gap = NULL)
#' \method{as.raw}{OpendalBytes}(x)
#' \method{length}{OpendalBytes}(x)
#' fs_read_iter(fs, path, chunk_size, offset = 0, size = NULL,
#'              read_concurrency = NULL, coalesce_gap = NULL)
#' read_iter_next(iter)
#' read_iter_collect(iter)
#' fs_tell(iter)
#' fs_seek(iter, offset, whence = c("start", "current", "end"))
#' fs_write(fs, path, data, batch_concurrency = NULL,
#'          write_concurrency = NULL, chunk_size = NULL,
#'          mode = c("raw", "serial", "text", "codec"), encoding = "UTF-8",
#'          serial_config = opt(fs, "serial"), codec = opt(fs, "codec"))
#' fs_write_aio(fs, path, data, batch_concurrency = NULL,
#'              write_concurrency = NULL, chunk_size = NULL,
#'              mode = c("raw", "serial", "text", "codec"), encoding = "UTF-8",
#'              serial_config = opt(fs, "serial"), codec = opt(fs, "codec"))
#' fs_replace(fs, path, data, batch_concurrency = NULL,
#'            write_concurrency = NULL, chunk_size = NULL,
#'            mode = c("raw", "serial", "text", "codec"), encoding = "UTF-8",
#'            serial_config = opt(fs, "serial"), codec = opt(fs, "codec"))
#' fs_replace_aio(fs, path, data, batch_concurrency = NULL,
#'                write_concurrency = NULL, chunk_size = NULL,
#'                mode = c("raw", "serial", "text", "codec"), encoding = "UTF-8",
#'                serial_config = opt(fs, "serial"), codec = opt(fs, "codec"))
#' fs_append(fs, path, data, batch_concurrency = NULL,
#'           write_concurrency = NULL, chunk_size = NULL,
#'           mode = c("raw", "serial", "text", "codec"), encoding = "UTF-8",
#'           serial_config = opt(fs, "serial"), codec = opt(fs, "codec"))
#' fs_append_aio(fs, path, data, batch_concurrency = NULL,
#'               write_concurrency = NULL, chunk_size = NULL,
#'               mode = c("raw", "serial", "text", "codec"), encoding = "UTF-8",
#'               serial_config = opt(fs, "serial"), codec = opt(fs, "codec"))
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
#' fs_ls(fs, path = "", recursive = FALSE, limit = NULL, start_after = NULL)
#' fs_ls_aio(fs, path = "", recursive = FALSE,
#'           limit = NULL, start_after = NULL)
#' fs_ls_iter(fs, path = "", recursive = FALSE, page_size = 1000,
#'            limit = NULL, start_after = NULL)
#' fs_walk_iter(fs, path = "", page_size = 1000,
#'              limit = NULL, start_after = NULL)
#' ls_iter_next(iter)
#' ls_iter_collect(iter)
#' walk_iter_next(iter)
#' walk_iter_collect(iter)
#' fs_mkdir(fs, path)
#' fs_mkdir_aio(fs, path, batch_concurrency = NULL)
#' fs_delete(fs, path, recursive = FALSE, batch_concurrency = NULL)
#' fs_delete_aio(fs, path, recursive = FALSE, batch_concurrency = NULL)
#' fs_copy(fs, from, to)
#' fs_copy_aio(fs, from, to, batch_concurrency = NULL)
#' fs_rename(fs, from, to)
#' fs_rename_aio(fs, from, to, batch_concurrency = NULL)
#' collect_aio(aio)
#' collect_aio_(aio)
#' call_aio(aio)
#' call_aio_(aio)
#' stop_aio(aio)
#' poll_aio(aio)
#' cv()
#' cv_value(cv)
#' cv_reset(cv)
#' cv_signal(cv)
#' cv_wait(cv, interval = 0.001)
#' cv_until(cv, msec = 0, interval = 0.001)
#' aio_monitor(aio, cv = cv())
#' read_monitor(monitor)
#' race_aio(aio, timeout = NULL, interval = 0.001)
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
#' @return Filesystem handles, raw vectors, `OpendalBytes` handles,
#'   deserialized R objects, metadata lists, logical results, Aio handles, or
#'   classed error values depending on the operation and mode.
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
opendal <- function(scheme = "fs", ..., root = NULL, config = list(), auth = NULL,
                    headers = NULL, runtime = runtime_config(), layers = list()) {
  if (!is.null(headers)) headers <- as.list(headers)
  OpendalFs$open(
    scheme,
    list(...),
    config,
    root,
    if (is.null(auth)) NULL else credential_config(auth, scheme),
    headers,
    .ropendal_runtime_threads(runtime),
    .ropendal_layer_max_inflight(layers)
  )
}

#' @export
#' @noRd
opendal_uri <- function(uri, headers = NULL, runtime = runtime_config(), layers = list()) {
  if (!is.null(headers)) headers <- as.list(headers)
  OpendalFs$from_uri(
    uri,
    headers,
    .ropendal_runtime_threads(runtime),
    .ropendal_layer_max_inflight(layers)
  )
}

#' @export
#' @noRd
runtime_config <- function(threads = NULL) {
  structure(list(threads = threads), class = "ropendalRuntimeConfig")
}

#' @export
#' @noRd
layer_concurrent_limit <- function(max) {
  if (missing(max)) stop("max is required", call. = FALSE)
  structure(
    list(type = "concurrent_limit", max = max),
    class = c("ropendalConcurrentLimitLayer", "ropendalLayerConfig")
  )
}

.ropendal_runtime_threads <- function(runtime) {
  if (is.null(runtime)) return(NULL)
  if (!inherits(runtime, "ropendalRuntimeConfig")) {
    stop("runtime must be created by runtime_config()", call. = FALSE)
  }
  runtime$threads
}

.ropendal_layer_max_inflight <- function(layers) {
  if (is.null(layers)) return(NULL)
  if (inherits(layers, "ropendalLayerConfig")) layers <- list(layers)
  if (!is.list(layers)) stop("layers must be a list of layer config objects", call. = FALSE)
  max_inflight <- NULL
  for (layer in layers) {
    if (!inherits(layer, "ropendalLayerConfig")) {
      stop("layers must contain only layer config objects", call. = FALSE)
    }
    if (identical(layer$type, "concurrent_limit")) {
      if (!is.null(max_inflight)) stop("only one layer_concurrent_limit() is allowed", call. = FALSE)
      max_inflight <- layer$max
    } else {
      stop("unsupported layer config", call. = FALSE)
    }
  }
  max_inflight
}

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
credentials_gcs <- function(token = "", service_account_key = "",
                            credential_path = "", scope = "",
                            source = "direct") {
  CredentialProvider(
    native = OpendalCredentialProvider$gcs(
      token,
      service_account_key,
      credential_path,
      scope,
      source
    ),
    schemes = "gcs"
  )
}

#' @export
#' @noRd
credentials_azblob <- function(account_name = "", account_key = "", sas_token = "",
                               endpoint = "", source = "direct") {
  CredentialProvider(
    native = OpendalCredentialProvider$azblob(
      account_name,
      account_key,
      sas_token,
      endpoint,
      source
    ),
    schemes = "azblob"
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
print.opendalCapabilityValue <- function(x, ...) {
  ops <- names(x$operations)
  supported <- vapply(x$operations, `[[`, logical(1), "supported")
  cat("<opendal capabilities>", x$scheme, x$root, "\n")
  if (length(ops)) {
    cat("  supported: ", paste(ops[supported], collapse = ", "), "\n", sep = "")
    unsupported <- ops[!supported]
    if (length(unsupported)) cat("  unsupported: ", paste(unsupported, collapse = ", "), "\n", sep = "")
  }
  invisible(x)
}

#' @export
#' @noRd
fs_normalize_path <- function(fs, path, directory = FALSE) {
  fs$normalize_path(path, directory)
}

.ropendal_serial_magic <- "Ropendal.serial.v1"
.ropendal_options_name <- ".ropendal_options"

#' @export
#' @noRd
serial_config <- function(class, sfunc, ufunc) {
  if (!is.character(class) || !length(class) || anyNA(class) || any(!nzchar(class))) {
    stop("class must be a non-empty character vector", call. = FALSE)
  }
  sfunc <- .ropendal_function_list(sfunc, length(class), "sfunc")
  ufunc <- .ropendal_function_list(ufunc, length(class), "ufunc")
  structure(
    list(class = class, sfunc = sfunc, ufunc = ufunc),
    class = "ropendalSerialConfig"
  )
}

.ropendal_native_codecs <- c("identity", "gzip", "zlib")

#' @export
#' @noRd
codec_config <- function(name, class = "raw", sfunc = NULL, ufunc = NULL) {
  name <- .ropendal_normalize_codec_name(name)
  if (!is.character(class) || length(class) != 1L || is.na(class) || !identical(class, "raw")) {
    stop("class must be \"raw\" for native byte codecs", call. = FALSE)
  }
  if (!is.null(sfunc) || !is.null(ufunc)) {
    stop("R-closure codecs are not supported; use serial_config() for R object materialization", call. = FALSE)
  }
  structure(
    list(name = name, class = class, native = TRUE),
    class = "ropendalCodecConfig"
  )
}

.ropendal_normalize_codec_name <- function(name) {
  if (!is.character(name) || length(name) != 1L || is.na(name) || !nzchar(name)) {
    stop("codec name must be a non-empty scalar string", call. = FALSE)
  }
  name <- tolower(name)
  name <- switch(name,
    none = "identity",
    raw = "identity",
    gz = "gzip",
    name
  )
  if (!(name %in% .ropendal_native_codecs)) {
    stop(
      "unsupported codec ", sQuote(name),
      "; supported codecs are ", paste(.ropendal_native_codecs, collapse = ", "),
      call. = FALSE
    )
  }
  name
}

.ropendal_function_list <- function(value, n, name) {
  if (is.function(value)) value <- list(value)
  if (!is.list(value) || length(value) != n || !all(vapply(value, is.function, logical(1)))) {
    stop(name, " must be a function or list of functions matching class", call. = FALSE)
  }
  value
}

.ropendal_normalize_serial_config <- function(config) {
  if (is.null(config) || (is.list(config) && !length(config))) return(list())
  if (!inherits(config, "ropendalSerialConfig")) {
    stop("serial config must come from serial_config() or be list()", call. = FALSE)
  }
  config
}

.ropendal_normalize_codec_config <- function(config, required = FALSE) {
  empty <- is.null(config) || (is.list(config) && !inherits(config, "ropendalCodecConfig") && !length(config))
  if (empty) {
    if (required) stop("mode = \"codec\" requires a codec", call. = FALSE)
    return(NULL)
  }
  if (inherits(config, "ropendalCodecConfig")) return(config$name)
  if (is.character(config)) return(.ropendal_normalize_codec_name(config))
  stop("codec must be NULL, list(), a codec name, or codec_config()", call. = FALSE)
}

.ropendal_codec_for_mode <- function(codec, mode) {
  .ropendal_normalize_codec_config(codec, required = identical(mode, "codec"))
}

.ropendal_check_fs <- function(fs) {
  if (!inherits(fs, "OpendalFs")) stop("fs must be an OpendalFs", call. = FALSE)
}

.ropendal_options <- function(fs) {
  .ropendal_check_fs(fs)
  if (exists(.ropendal_options_name, envir = fs, inherits = FALSE)) {
    get(.ropendal_options_name, envir = fs, inherits = FALSE)
  } else {
    list()
  }
}

.ropendal_set_options <- function(fs, opts) {
  assign(.ropendal_options_name, opts, envir = fs)
  invisible(fs)
}

#' @export
#' @noRd
opt <- function(fs, name) {
  name <- match.arg(name, c("serial", "codec"))
  opts <- .ropendal_options(fs)
  switch(name,
    serial = if (is.null(opts$serial)) list() else opts$serial,
    codec = if (is.null(opts$codec)) list() else opts$codec
  )
}

#' @export
#' @noRd
`opt<-` <- function(fs, name, value) {
  name <- match.arg(name, c("serial", "codec"))
  opts <- .ropendal_options(fs)
  switch(name,
    serial = {
      opts$serial <- .ropendal_normalize_serial_config(value)
    },
    codec = {
      codec_name <- .ropendal_normalize_codec_config(value)
      opts$codec <- if (is.null(codec_name)) list() else codec_config(codec_name)
    }
  )
  .ropendal_set_options(fs, opts)
}

.ropendal_payload_to_raw <- function(value, name) {
  if (inherits(value, "OpendalBytes")) value <- as.raw(value)
  if (!is.raw(value)) stop(name, " must be a raw vector or OpendalBytes", call. = FALSE)
  value
}

.ropendal_serial_match <- function(x, config) {
  if (!inherits(config, "ropendalSerialConfig")) return(NA_integer_)
  match(TRUE, vapply(config$class, function(cls) inherits(x, cls), logical(1)), nomatch = NA_integer_)
}

#' @export
#' @noRd
serialize_raw <- function(x, config = list()) {
  config <- .ropendal_normalize_serial_config(config)
  i <- .ropendal_serial_match(x, config)
  if (is.na(i)) return(serialize(x, NULL))

  payload <- config$sfunc[[i]](x)
  payload <- .ropendal_payload_to_raw(payload, "sfunc result")
  serialize(
    structure(
      list(
        magic = .ropendal_serial_magic,
        version = 1L,
        class = config$class[[i]],
        payload = payload
      ),
      class = "ropendalSerialEnvelope"
    ),
    NULL
  )
}

.ropendal_is_serial_envelope <- function(x) {
  inherits(x, "ropendalSerialEnvelope") &&
    is.list(x) &&
    identical(x$magic, .ropendal_serial_magic) &&
    identical(x$version, 1L) &&
    is.character(x$class) && length(x$class) == 1L &&
    is.raw(x$payload)
}

#' @export
#' @noRd
deserialize_raw <- function(x, config = list()) {
  x <- .ropendal_payload_to_raw(x, "x")
  value <- unserialize(x)
  if (!.ropendal_is_serial_envelope(value)) return(value)

  config <- .ropendal_normalize_serial_config(config)
  if (!inherits(config, "ropendalSerialConfig")) {
    stop("serialized payload requires a matching serial_config()", call. = FALSE)
  }
  i <- match(value$class, config$class, nomatch = NA_integer_)
  if (is.na(i)) {
    stop("serialized payload requires a deserializer for class ", value$class, call. = FALSE)
  }
  config$ufunc[[i]](value$payload)
}

.ropendal_deserialize_tree <- function(x, config) {
  if (is_error_value(x)) return(x)
  if (is.raw(x) || inherits(x, "OpendalBytes")) return(deserialize_raw(x, config))
  if (is.list(x)) return(lapply(x, .ropendal_deserialize_tree, config = config))
  stop("serial reads must resolve to raw bytes or error values", call. = FALSE)
}

.ropendal_serial_data <- function(path, data, config) {
  config <- .ropendal_normalize_serial_config(config)
  n <- length(path)
  if (n <= 1L) return(serialize_raw(data, config))
  if (!is.list(data) || is_error_value(data) || length(data) != n) {
    stop("data must be a list matching path length for vectorized serial writes", call. = FALSE)
  }
  lapply(data, serialize_raw, config = config)
}

.ropendal_is_partial_read <- function(offset, size, end) {
  if (!is.null(size) || !is.null(end)) return(TRUE)
  values <- unlist(offset, recursive = TRUE, use.names = FALSE)
  length(values) && (anyNA(values) || any(values != 0))
}

.ropendal_check_complete_serial_read <- function(offset, size, end) {
  if (.ropendal_is_partial_read(offset, size, end)) {
    stop("mode = \"serial\" requires complete-object reads; use mode = \"raw\" for byte ranges", call. = FALSE)
  }
}

.ropendal_check_complete_text_read <- function(offset, size, end) {
  if (.ropendal_is_partial_read(offset, size, end)) {
    stop("mode = \"text\" requires complete-object reads; use mode = \"raw\" for byte ranges", call. = FALSE)
  }
}

.ropendal_check_text_encoding <- function(encoding) {
  if (!is.character(encoding) || length(encoding) != 1L || is.na(encoding) || !nzchar(encoding)) {
    stop("encoding must be a non-empty scalar string", call. = FALSE)
  }
  probe <- tryCatch(
    iconv("A", from = "UTF-8", to = encoding, toRaw = TRUE)[[1]],
    error = function(e) NULL
  )
  if (is.null(probe)) stop("unsupported text encoding ", sQuote(encoding), call. = FALSE)
  if (any(probe == as.raw(0))) {
    stop("text encoding ", sQuote(encoding), " produces embedded NUL bytes; use raw mode", call. = FALSE)
  }
  encoding
}

.ropendal_text_to_raw_one <- function(value, encoding) {
  if (!is.character(value) || length(value) != 1L || is.na(value)) {
    stop("text data must be a non-missing scalar string", call. = FALSE)
  }
  bytes <- iconv(value, from = "", to = encoding, toRaw = TRUE)[[1]]
  if (is.null(bytes)) stop("text data could not be encoded as ", encoding, call. = FALSE)
  bytes
}

.ropendal_text_from_raw_one <- function(value, encoding) {
  bytes <- .ropendal_payload_to_raw(value, "text bytes")
  if (any(bytes == as.raw(0))) stop("text bytes cannot contain NUL bytes", call. = FALSE)
  text <- tryCatch(
    rawToChar(bytes, multiple = FALSE),
    error = function(e) stop("text bytes cannot be materialized as an R string", call. = FALSE)
  )
  text <- iconv(text, from = encoding, to = "UTF-8")
  if (is.na(text)) stop("text bytes could not be decoded as ", encoding, call. = FALSE)
  Encoding(text) <- "UTF-8"
  text
}

.ropendal_text_from_raw_tree <- function(value, encoding) {
  if (is_error_value(value)) return(value)
  if (is.raw(value) || inherits(value, "OpendalBytes")) return(.ropendal_text_from_raw_one(value, encoding))
  if (is.list(value)) return(lapply(value, .ropendal_text_from_raw_tree, encoding = encoding))
  stop("text reads must resolve to raw bytes or error values", call. = FALSE)
}

.ropendal_text_data <- function(path, data, encoding) {
  encoding <- .ropendal_check_text_encoding(encoding)
  n <- length(path)
  if (n <= 1L) return(.ropendal_text_to_raw_one(data, encoding))
  if (is.character(data)) {
    if (length(data) != n) stop("data length must match path length for vectorized text writes", call. = FALSE)
    return(lapply(data, .ropendal_text_to_raw_one, encoding = encoding))
  }
  if (is.list(data) && !is_error_value(data)) {
    if (length(data) != n) stop("data length must match path length for vectorized text writes", call. = FALSE)
    return(lapply(data, .ropendal_text_to_raw_one, encoding = encoding))
  }
  stop("data must be a character vector or list of scalar strings for text writes", call. = FALSE)
}

.ropendal_check_complete_codec_read <- function(codec, offset, size, end) {
  if (!is.null(codec) && !identical(codec, "identity") && .ropendal_is_partial_read(offset, size, end)) {
    stop("codec reads require complete-object reads; use mode = \"raw\" without codec for byte ranges", call. = FALSE)
  }
}

.ropendal_codec_encode_one <- function(value, codec) {
  if (inherits(value, "OpendalBytes")) value <- as.raw(value)
  if (!is.raw(value)) {
    stop("codec input must be a raw vector or OpendalBytes", call. = FALSE)
  }
  opendal_codec_encode(codec, value)
}

.ropendal_codec_decode_one <- function(value, codec) {
  if (inherits(value, "OpendalBytes")) value <- as.raw(value)
  if (!is.raw(value)) {
    stop("codec input must be a raw vector or OpendalBytes", call. = FALSE)
  }
  opendal_codec_decode(codec, value)
}

.ropendal_codec_encode_tree <- function(value, codec) {
  if (is.null(codec)) return(opendal_bytes_unwrap(value))
  if (is_error_value(value)) return(value)
  if (is.raw(value) || inherits(value, "OpendalBytes")) return(.ropendal_codec_encode_one(value, codec))
  if (is.list(value)) return(lapply(value, .ropendal_codec_encode_tree, codec = codec))
  stop("codec writes require raw bytes, OpendalBytes, or lists of bytes", call. = FALSE)
}

.ropendal_codec_decode_tree <- function(value, codec) {
  if (is.null(codec)) return(value)
  if (is_error_value(value)) return(value)
  if (is.raw(value) || inherits(value, "OpendalBytes")) return(.ropendal_codec_decode_one(value, codec))
  if (is.list(value)) return(lapply(value, .ropendal_codec_decode_tree, codec = codec))
  stop("codec reads must resolve to raw bytes or error values", call. = FALSE)
}

.ropendal_materialize_read <- function(value, mode, serial_config, codec, encoding) {
  value <- .ropendal_codec_decode_tree(value, codec)
  if (identical(mode, "serial")) value <- .ropendal_deserialize_tree(value, serial_config)
  if (identical(mode, "text")) value <- .ropendal_text_from_raw_tree(value, encoding)
  value
}

.ropendal_prepare_write_data <- function(path, data, mode, serial_config, codec, encoding) {
  if (identical(mode, "serial")) data <- .ropendal_serial_data(path, data, serial_config)
  if (identical(mode, "text")) data <- .ropendal_text_data(path, data, encoding)
  .ropendal_codec_encode_tree(data, codec)
}

#' @export
#' @noRd
fs_read <- function(fs, path, offset = 0, size = NULL, end = NULL,
                    result = c("auto", "flat", "nested"),
                    batch_concurrency = NULL,
                    read_concurrency = NULL,
                    chunk_size = NULL,
                    coalesce_gap = NULL,
                    mode = c("raw", "serial", "text", "codec"),
                    encoding = "UTF-8",
                    serial_config = opt(fs, "serial"),
                    codec = opt(fs, "codec")) {
  mode <- match.arg(mode)
  result <- match.arg(result)
  codec <- .ropendal_codec_for_mode(codec, mode)
  if (identical(mode, "serial")) .ropendal_check_complete_serial_read(offset, size, end)
  if (identical(mode, "text")) {
    encoding <- .ropendal_check_text_encoding(encoding)
    .ropendal_check_complete_text_read(offset, size, end)
  }
  .ropendal_check_complete_codec_read(codec, offset, size, end)
  value <- fs$read(
    path,
    offset,
    size,
    end,
    result,
    batch_concurrency,
    read_concurrency,
    chunk_size,
    coalesce_gap
  )
  .ropendal_materialize_read(value, mode, serial_config, codec, encoding)
}

#' @export
#' @noRd
fs_read_aio <- function(fs, path, offset = 0, size = NULL, end = NULL,
                        result = c("auto", "flat", "nested"),
                        batch_concurrency = NULL,
                        read_concurrency = NULL,
                        chunk_size = NULL,
                        coalesce_gap = NULL,
                        mode = c("raw", "serial", "text", "codec"),
                        encoding = "UTF-8",
                        serial_config = opt(fs, "serial"),
                        codec = opt(fs, "codec")) {
  mode <- match.arg(mode)
  result <- match.arg(result)
  codec <- .ropendal_codec_for_mode(codec, mode)
  if (identical(mode, "serial")) .ropendal_check_complete_serial_read(offset, size, end)
  if (identical(mode, "text")) {
    encoding <- .ropendal_check_text_encoding(encoding)
    .ropendal_check_complete_text_read(offset, size, end)
  }
  .ropendal_check_complete_codec_read(codec, offset, size, end)
  materializer <- function(value) .ropendal_materialize_read(value, mode, serial_config, codec, encoding)
  opendal_aio_with_bindings(fs$read_aio(
    path,
    offset,
    size,
    end,
    result,
    batch_concurrency,
    read_concurrency,
    chunk_size,
    coalesce_gap
  ), materializer = materializer)
}

#' @export
#' @noRd
fs_read_bytes <- function(fs, path, offset = 0, size = NULL, end = NULL,
                          result = c("auto", "flat", "nested"),
                          batch_concurrency = NULL,
                          read_concurrency = NULL,
                          chunk_size = NULL,
                          coalesce_gap = NULL) {
  opendal_bytes_wrap(fs$read_bytes(
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
fs_read_bytes_aio <- function(fs, path, offset = 0, size = NULL, end = NULL,
                              result = c("auto", "flat", "nested"),
                              batch_concurrency = NULL,
                              read_concurrency = NULL,
                              chunk_size = NULL,
                              coalesce_gap = NULL) {
  opendal_aio_with_bindings(fs$read_bytes_aio(
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
as.raw.OpendalBytes <- function(x) opendal_bytes_as_raw(x)

#' @export
#' @noRd
length.OpendalBytes <- function(x) opendal_bytes_len(x)

opendal_bytes_wrap <- function(x) {
  if (inherits(x, "OpendalBytes") && identical(typeof(x), "externalptr")) {
    return(.savvy_wrap_OpendalBytes(x))
  }
  if (is.list(x) && !is_error_value(x)) {
    return(lapply(x, opendal_bytes_wrap))
  }
  x
}

opendal_bytes_unwrap <- function(x) {
  if (inherits(x, "OpendalBytes") && is.environment(x)) return(x$.ptr)
  if (is.list(x) && !is_error_value(x)) return(lapply(x, opendal_bytes_unwrap))
  x
}

#' @export
#' @noRd
fs_write <- function(fs, path, data, batch_concurrency = NULL,
                     write_concurrency = NULL, chunk_size = NULL,
                     mode = c("raw", "serial", "text", "codec"),
                     encoding = "UTF-8",
                     serial_config = opt(fs, "serial"),
                     codec = opt(fs, "codec")) {
  mode <- match.arg(mode)
  codec <- .ropendal_codec_for_mode(codec, mode)
  data <- .ropendal_prepare_write_data(path, data, mode, serial_config, codec, encoding)
  fs$write(path, data, batch_concurrency, write_concurrency, chunk_size)
}

#' @export
#' @noRd
fs_write_aio <- function(fs, path, data, batch_concurrency = NULL,
                         write_concurrency = NULL, chunk_size = NULL,
                         mode = c("raw", "serial", "text", "codec"),
                         encoding = "UTF-8",
                         serial_config = opt(fs, "serial"),
                         codec = opt(fs, "codec")) {
  mode <- match.arg(mode)
  codec <- .ropendal_codec_for_mode(codec, mode)
  data <- .ropendal_prepare_write_data(path, data, mode, serial_config, codec, encoding)
  opendal_aio_with_bindings(
    fs$write_aio(path, data, batch_concurrency, write_concurrency, chunk_size)
  )
}

#' @export
#' @noRd
fs_replace <- function(fs, path, data, batch_concurrency = NULL,
                       write_concurrency = NULL, chunk_size = NULL,
                       mode = c("raw", "serial", "text", "codec"),
                       encoding = "UTF-8",
                       serial_config = opt(fs, "serial"),
                       codec = opt(fs, "codec")) {
  mode <- match.arg(mode)
  codec <- .ropendal_codec_for_mode(codec, mode)
  data <- .ropendal_prepare_write_data(path, data, mode, serial_config, codec, encoding)
  fs$replace(path, data, batch_concurrency, write_concurrency, chunk_size)
}

#' @export
#' @noRd
fs_replace_aio <- function(fs, path, data, batch_concurrency = NULL,
                           write_concurrency = NULL, chunk_size = NULL,
                           mode = c("raw", "serial", "text", "codec"),
                           encoding = "UTF-8",
                           serial_config = opt(fs, "serial"),
                           codec = opt(fs, "codec")) {
  mode <- match.arg(mode)
  codec <- .ropendal_codec_for_mode(codec, mode)
  data <- .ropendal_prepare_write_data(path, data, mode, serial_config, codec, encoding)
  opendal_aio_with_bindings(
    fs$replace_aio(path, data, batch_concurrency, write_concurrency, chunk_size)
  )
}

#' @export
#' @noRd
fs_append <- function(fs, path, data, batch_concurrency = NULL,
                      write_concurrency = NULL, chunk_size = NULL,
                      mode = c("raw", "serial", "text", "codec"),
                      encoding = "UTF-8",
                      serial_config = opt(fs, "serial"),
                      codec = opt(fs, "codec")) {
  mode <- match.arg(mode)
  codec <- .ropendal_codec_for_mode(codec, mode)
  data <- .ropendal_prepare_write_data(path, data, mode, serial_config, codec, encoding)
  fs$append(path, data, batch_concurrency, write_concurrency, chunk_size)
}

#' @export
#' @noRd
fs_append_aio <- function(fs, path, data, batch_concurrency = NULL,
                          write_concurrency = NULL, chunk_size = NULL,
                          mode = c("raw", "serial", "text", "codec"),
                          encoding = "UTF-8",
                          serial_config = opt(fs, "serial"),
                          codec = opt(fs, "codec")) {
  mode <- match.arg(mode)
  codec <- .ropendal_codec_for_mode(codec, mode)
  data <- .ropendal_prepare_write_data(path, data, mode, serial_config, codec, encoding)
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
fs_ls <- function(fs, path = "", recursive = FALSE, limit = NULL, start_after = NULL) {
  fs$ls(path, recursive, limit, start_after)
}

#' @export
#' @noRd
fs_ls_aio <- function(fs, path = "", recursive = FALSE, limit = NULL, start_after = NULL) {
  opendal_aio_with_bindings(fs$ls_aio(path, recursive, limit, start_after))
}

#' @export
#' @noRd
fs_ls_iter <- function(fs, path = "", recursive = FALSE, page_size = 1000,
                       limit = NULL, start_after = NULL) {
  fs$ls_iter(path, recursive, page_size, limit, start_after)
}

#' @export
#' @noRd
fs_walk_iter <- function(fs, path = "", page_size = 1000,
                         limit = NULL, start_after = NULL) {
  fs$walk_iter(path, page_size, limit, start_after)
}

#' @export
#' @noRd
ls_iter_next <- function(iter) iter[["next"]]()

#' @export
#' @noRd
ls_iter_collect <- function(iter) iter$collect()

#' @export
#' @noRd
walk_iter_next <- function(iter) iter[["next"]]()

#' @export
#' @noRd
walk_iter_collect <- function(iter) iter$collect()

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

opendal_aio_with_bindings <- function(aio, materializer = identity) {
  assign(".ropendal_materializer", materializer, envir = aio)
  assign(".ropendal_materialized_ready", FALSE, envir = aio)
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
  if (identical(aio$state_name(), "pending")) unresolved() else opendal_aio_collect_value(aio)
}

opendal_aio_collect_value <- function(aio) {
  if (exists(".ropendal_materialized_ready", envir = aio, inherits = FALSE) &&
    isTRUE(get(".ropendal_materialized_ready", envir = aio, inherits = FALSE))) {
    return(get(".ropendal_materialized_value", envir = aio, inherits = FALSE))
  }
  value <- opendal_bytes_wrap(aio$collect())
  materializer <- if (exists(".ropendal_materializer", envir = aio, inherits = FALSE)) {
    get(".ropendal_materializer", envir = aio, inherits = FALSE)
  } else {
    identity
  }
  value <- materializer(value)
  assign(".ropendal_materialized_value", value, envir = aio)
  assign(".ropendal_materialized_ready", TRUE, envir = aio)
  value
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
  opendal_aio_collect_value(aio)
}

#' @export
#' @noRd
collect_aio_ <- function(aio) {
  if (inherits(aio, "OpendalAio")) return(collect_aio(aio))
  lapply(aio, collect_aio)
}

#' @export
#' @noRd
call_aio <- function(aio) {
  collect_aio(aio)
  invisible(aio)
}

#' @export
#' @noRd
call_aio_ <- function(aio) {
  if (inherits(aio, "OpendalAio")) return(call_aio(aio))
  lapply(aio, call_aio)
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
cv <- function() {
  out <- new.env(parent = emptyenv())
  out$.value <- 0L
  out$.monitors <- list()
  class(out) <- "opendalCv"
  out
}

opendal_assert_cv <- function(x) {
  if (!inherits(x, "opendalCv")) stop("cv must come from cv()", call. = FALSE)
}

opendal_cv_bump <- function(x) {
  x$.value <- x$.value + 1L
  invisible(x)
}

#' @export
#' @noRd
cv_value <- function(cv) {
  opendal_assert_cv(cv)
  cv$.value
}

#' @export
#' @noRd
cv_reset <- function(cv) {
  opendal_assert_cv(cv)
  cv$.value <- 0L
  invisible(cv)
}

#' @export
#' @noRd
cv_signal <- function(cv) {
  opendal_assert_cv(cv)
  opendal_cv_bump(cv)
}

opendal_empty_monitor_events <- function() {
  data.frame(
    index = integer(),
    name = character(),
    event = character(),
    state = character(),
    stringsAsFactors = FALSE
  )
}

opendal_aio_event <- function(aio) {
  state <- aio$state
  if (identical(state, "pending")) return(NULL)
  if (identical(state, "cancelled")) return(c(event = "cancelled", state = state))
  err <- aio$error
  if (is_error_value(err)) {
    event <- if (identical(error_kind(err), "Cancelled")) "cancelled" else "error"
    return(c(event = event, state = aio$state))
  }
  c(event = "ready", state = aio$state)
}

opendal_monitor_scan <- function(monitor) {
  if (!inherits(monitor, "opendalAioMonitor")) return(FALSE)
  found <- opendal_empty_monitor_events()
  for (i in seq_along(monitor$.aios)) {
    if (monitor$.emitted[[i]]) next
    event <- opendal_aio_event(monitor$.aios[[i]])
    if (is.null(event)) next
    monitor$.emitted[[i]] <- TRUE
    found <- rbind(found, data.frame(
      index = i,
      name = monitor$.names[[i]],
      event = unname(event[["event"]]),
      state = unname(event[["state"]]),
      stringsAsFactors = FALSE
    ))
  }
  if (!nrow(found)) return(FALSE)
  monitor$.events <- rbind(monitor$.events, found)
  TRUE
}

opendal_cv_scan <- function(cv) {
  opendal_assert_cv(cv)
  if (!length(cv$.monitors)) return(invisible(FALSE))
  signalled <- FALSE
  for (monitor in cv$.monitors) {
    signalled <- opendal_monitor_scan(monitor) || signalled
  }
  if (signalled) opendal_cv_bump(cv)
  invisible(signalled)
}

#' @export
#' @noRd
cv_until <- function(cv, msec = 0, interval = 0.001) {
  opendal_assert_cv(cv)
  if (!is.numeric(msec) || length(msec) != 1L || is.na(msec) || msec < 0) {
    stop("msec must be a non-negative number", call. = FALSE)
  }
  if (!is.numeric(interval) || length(interval) != 1L || is.na(interval) || interval < 0) {
    stop("interval must be a non-negative number of seconds", call. = FALSE)
  }
  if (cv$.value > 0L) return(TRUE)
  start <- cv$.value
  deadline <- Sys.time() + as.difftime(msec / 1000, units = "secs")
  repeat {
    opendal_cv_scan(cv)
    if (!identical(cv$.value, start)) return(TRUE)
    remaining <- as.numeric(difftime(deadline, Sys.time(), units = "secs"))
    if (remaining <= 0) return(FALSE)
    Sys.sleep(min(interval, remaining))
  }
}

#' @export
#' @noRd
cv_wait <- function(cv, interval = 0.001) {
  opendal_assert_cv(cv)
  if (cv$.value > 0L) return(TRUE)
  start <- cv$.value
  repeat {
    opendal_cv_scan(cv)
    if (!identical(cv$.value, start)) return(TRUE)
    Sys.sleep(interval)
  }
}

#' @export
#' @noRd
aio_monitor <- function(aio, cv = cv()) {
  opendal_assert_cv(cv)
  aios <- if (inherits(aio, "OpendalAio")) list(aio) else as.list(aio)
  if (!length(aios) || !all(vapply(aios, inherits, logical(1), "OpendalAio"))) {
    stop("aio must be an OpendalAio or a list of OpendalAio objects", call. = FALSE)
  }
  out <- new.env(parent = emptyenv())
  out$.aios <- aios
  out$.names <- names(aios)
  if (is.null(out$.names)) out$.names <- rep("", length(aios))
  out$.emitted <- rep(FALSE, length(aios))
  out$.events <- opendal_empty_monitor_events()
  out$cv <- cv
  class(out) <- "opendalAioMonitor"
  cv$.monitors[[length(cv$.monitors) + 1L]] <- out
  out
}

#' @export
#' @noRd
read_monitor <- function(monitor) {
  if (!inherits(monitor, "opendalAioMonitor")) {
    stop("monitor must come from aio_monitor()", call. = FALSE)
  }
  opendal_monitor_scan(monitor)
  events <- monitor$.events
  monitor$.events <- opendal_empty_monitor_events()
  events
}

#' @export
#' @noRd
race_aio <- function(aio, timeout = NULL, interval = 0.001) {
  aios <- if (inherits(aio, "OpendalAio")) list(aio) else as.list(aio)
  cv <- cv()
  monitor <- aio_monitor(aios, cv = cv)
  ready <- if (is.null(timeout)) cv_wait(cv, interval = interval) else cv_until(cv, timeout, interval = interval)
  if (!ready) return(unresolved())
  events <- read_monitor(monitor)
  first <- events[["index"]][[1L]]
  structure(
    list(index = first, name = events[["name"]][[1L]], event = events[["event"]][[1L]], aio = aios[[first]]),
    class = "opendalAioRace"
  )
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
print.OpendalBytes <- function(x, ...) {
  cat("<opendal bytes>", length(x), "bytes\n")
  invisible(x)
}

#' @export
#' @noRd
print.OpendalLsIter <- function(x, ...) {
  cat("<opendal listing iterator>\n")
  invisible(x)
}

#' @export
#' @noRd
print.OpendalWriteIter <- function(x, ...) {
  cat("<opendal write iterator>\n")
  invisible(x)
}
