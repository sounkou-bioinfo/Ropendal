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
#'   fs_info fs_capabilities fs_normalize_path fs_read fs_read_aio fs_write
#'   fs_replace fs_append fs_stat fs_exists fs_ls fs_mkdir fs_delete fs_copy
#'   fs_rename collect_aio call_aio stop_aio poll_aio unresolved is_error_value
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
#' @param batch_concurrency Optional maximum number of concurrent batch
#'   operations.
#' @param data Raw vector, or list of raw vectors for multiple paths.
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
#'         result = c("auto", "flat", "nested"), batch_concurrency = NULL)
#' fs_read_aio(fs, path, offset = 0, size = NULL, end = NULL,
#'             result = c("auto", "flat", "nested"), batch_concurrency = NULL)
#' fs_write(fs, path, data)
#' fs_replace(fs, path, data)
#' fs_append(fs, path, data)
#' fs_stat(fs, path, batch_concurrency = NULL)
#' fs_exists(fs, path, batch_concurrency = NULL)
#' fs_ls(fs, path = "", recursive = FALSE)
#' fs_mkdir(fs, path)
#' fs_delete(fs, path, recursive = FALSE, batch_concurrency = NULL)
#' fs_copy(fs, from, to)
#' fs_rename(fs, from, to)
#' collect_aio(aio)
#' call_aio(aio)
#' stop_aio(aio)
#' poll_aio(aio)
#' unresolved()
#' is_error_value(x)
#' error_kind(x)
#' error_message(x)
#' error_operation(x)
#' error_path(x)
#' @details
#' Filesystem failures are returned as classed error values. Invalid arguments
#' and internal runtime failures may signal R errors. Credential helpers return
#' classed providers with redacted printing; pass them with `auth =`.
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
                    batch_concurrency = NULL) {
  fs$read(path, offset, size, end, match.arg(result), batch_concurrency)
}

#' @export
#' @noRd
fs_read_aio <- function(fs, path, offset = 0, size = NULL, end = NULL,
                        result = c("auto", "flat", "nested"),
                        batch_concurrency = NULL) {
  fs$read_aio(path, offset, size, end, match.arg(result), batch_concurrency)
}

#' @export
#' @noRd
fs_write <- function(fs, path, data) fs$write(path, data)

#' @export
#' @noRd
fs_replace <- function(fs, path, data) fs$replace(path, data)

#' @export
#' @noRd
fs_append <- function(fs, path, data) fs$append(path, data)

#' @export
#' @noRd
fs_stat <- function(fs, path, batch_concurrency = NULL) {
  fs$stat(path, batch_concurrency)
}

#' @export
#' @noRd
fs_exists <- function(fs, path, batch_concurrency = NULL) {
  fs$exists(path, batch_concurrency)
}

#' @export
#' @noRd
fs_ls <- function(fs, path = "", recursive = FALSE) {
  fs$ls(path, recursive)
}

#' @export
#' @noRd
fs_mkdir <- function(fs, path) {
  fs$mkdir(path)
}

#' @export
#' @noRd
fs_delete <- function(fs, path, recursive = FALSE, batch_concurrency = NULL) {
  fs$delete(path, recursive, batch_concurrency)
}

#' @export
#' @noRd
fs_copy <- function(fs, from, to) fs$copy(from, to)

#' @export
#' @noRd
fs_rename <- function(fs, from, to) fs$rename(from, to)

#' @export
#' @noRd
unresolved <- function() structure(NA, class = "unresolvedValue")

#' @export
#' @noRd
collect_aio <- function(aio) {
  aio$collect()
}

#' @export
#' @noRd
call_aio <- function(aio) collect_aio(aio)

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
