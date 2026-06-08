#' Byte store adapter
#'
#' A small key-to-bytes adapter over an `OpendalFs` prefix. It is intended as a
#' substrate for Zarr-like chunk layouts and other index-driven readers that want
#' store-relative keys without leaving Ropendal's byte-first filesystem API.
#'
#' @name byte-store
#' @aliases byte_store store_read store_write store_replace store_exists store_list store_delete
#' @param fs Ropendal filesystem handle.
#' @param prefix Root-relative filesystem prefix used as the store root.
#' @param store Byte store object returned by `byte_store()`.
#' @param key Store-relative object key or keys.
#' @param path Store-relative directory path for `store_list()`.
#' @param data Raw vector, `OpendalBytes`, or list of raw vectors/byte handles.
#' @param mode Read materialization: raw vectors or `OpendalBytes` handles.
#' @param offset,size,end Optional byte-range controls forwarded to the read
#'   operation.
#' @param result Requested read result shape.
#' @param batch_concurrency Optional maximum number of keys to process
#'   concurrently.
#' @param read_concurrency,write_concurrency Optional per-object OpenDAL transfer
#'   concurrency.
#' @param chunk_size Optional transfer chunk size in bytes.
#' @param coalesce_gap Optional byte gap for coalescing nearby read ranges.
#' @param recursive Whether to list or delete recursively.
#' @param limit Optional maximum listing entries.
#' @param start_after Optional store-relative listing continuation marker.
#' @return `byte_store()` returns a `ropendalByteStore`. Store operations return
#'   the corresponding filesystem values with paths rewritten relative to the
#'   store root for listings.
#' @examples
#' root <- tempfile("ropendal-store-")
#' dir.create(root)
#' fs <- opendal("fs", root = root)
#' store <- byte_store(fs, "array.zarr")
#' store_write(store, "zarr.json", charToRaw("{}"))
#' store_read(store, "zarr.json")
#' @export
byte_store <- function(fs, prefix = "") {
  .ropendal_check_fs(fs)
  structure(
    list(fs = fs, prefix = .ropendal_store_prefix(fs, prefix)),
    class = "ropendalByteStore"
  )
}

#' @export
print.ropendalByteStore <- function(x, ...) {
  cat("<opendal byte store> prefix=", sQuote(x$prefix), "\n", sep = "")
  invisible(x)
}

#' @rdname byte-store
#' @export
store_read <- function(store, key, mode = c("raw", "bytes"),
                       offset = NULL, size = NULL, end = NULL,
                       result = c("auto", "flat", "nested"),
                       batch_concurrency = NULL,
                       read_concurrency = NULL,
                       chunk_size = NULL,
                       coalesce_gap = NULL) {
  store <- .ropendal_check_store(store)
  path <- .ropendal_store_path(store, key, object = TRUE)
  mode <- match.arg(mode)
  if (identical(mode, "bytes")) {
    return(fs_read_bytes(
      store$fs, path, offset = offset, size = size, end = end,
      result = match.arg(result), batch_concurrency = batch_concurrency,
      read_concurrency = read_concurrency, chunk_size = chunk_size,
      coalesce_gap = coalesce_gap
    ))
  }
  fs_read(
    store$fs, path, offset = offset, size = size, end = end,
    result = match.arg(result), batch_concurrency = batch_concurrency,
    read_concurrency = read_concurrency, chunk_size = chunk_size,
    coalesce_gap = coalesce_gap
  )
}

#' @rdname byte-store
#' @export
store_write <- function(store, key, data, batch_concurrency = NULL,
                        write_concurrency = NULL, chunk_size = NULL) {
  store <- .ropendal_check_store(store)
  fs_write(
    store$fs,
    .ropendal_store_path(store, key, object = TRUE),
    data,
    batch_concurrency = batch_concurrency,
    write_concurrency = write_concurrency,
    chunk_size = chunk_size
  )
}

#' @rdname byte-store
#' @export
store_replace <- function(store, key, data, batch_concurrency = NULL,
                          write_concurrency = NULL, chunk_size = NULL) {
  store <- .ropendal_check_store(store)
  fs_replace(
    store$fs,
    .ropendal_store_path(store, key, object = TRUE),
    data,
    batch_concurrency = batch_concurrency,
    write_concurrency = write_concurrency,
    chunk_size = chunk_size
  )
}

#' @rdname byte-store
#' @export
store_exists <- function(store, key, batch_concurrency = NULL) {
  store <- .ropendal_check_store(store)
  fs_exists(
    store$fs,
    .ropendal_store_path(store, key, object = TRUE),
    batch_concurrency = batch_concurrency
  )
}

#' @rdname byte-store
#' @export
store_list <- function(store, path = "", recursive = FALSE, limit = NULL, start_after = NULL) {
  store <- .ropendal_check_store(store)
  path <- .ropendal_store_path(store, path, directory = TRUE, allow_empty = TRUE)
  start_after <- if (is.null(start_after)) {
    NULL
  } else {
    .ropendal_store_path(store, start_after, allow_empty = FALSE)
  }
  entries <- fs_ls(store$fs, path, recursive = recursive, limit = limit, start_after = start_after)
  .ropendal_store_relative_entries(store, entries)
}

#' @rdname byte-store
#' @export
store_delete <- function(store, key, recursive = FALSE, batch_concurrency = NULL) {
  store <- .ropendal_check_store(store)
  fs_delete(
    store$fs,
    .ropendal_store_path(store, key, directory = recursive, object = !recursive),
    recursive = recursive,
    batch_concurrency = batch_concurrency
  )
}

.ropendal_check_store <- function(store) {
  if (!inherits(store, "ropendalByteStore")) stop("store must come from byte_store()", call. = FALSE)
  .ropendal_check_fs(store$fs)
  store
}

.ropendal_store_prefix <- function(fs, prefix) {
  if (is.null(prefix)) prefix <- ""
  if (!is.character(prefix) || length(prefix) != 1L || is.na(prefix)) {
    stop("prefix must be a non-missing scalar string", call. = FALSE)
  }
  if (!nzchar(prefix)) return("")
  fs_normalize_path(fs, prefix, directory = TRUE)
}

.ropendal_store_path <- function(store, key, directory = FALSE, object = FALSE, allow_empty = FALSE) {
  if (!is.character(key) || anyNA(key)) stop("key must be a non-missing character vector", call. = FALSE)
  if (!length(key)) stop("key must contain at least one value", call. = FALSE)
  if (!allow_empty && any(!nzchar(key))) stop("key must not be empty", call. = FALSE)
  if (object && any(endsWith(key, "/"))) stop("key must be an object key, not a directory", call. = FALSE)

  normalized <- vapply(
    key,
    function(one) fs_normalize_path(store$fs, one, directory = directory),
    character(1),
    USE.NAMES = FALSE
  )
  if (!allow_empty && any(!nzchar(normalized))) stop("key must not be empty", call. = FALSE)
  if (object && any(endsWith(normalized, "/"))) stop("key must be an object key, not a directory", call. = FALSE)
  paste0(store$prefix, normalized)
}

.ropendal_store_relative_entries <- function(store, entries) {
  if (!length(entries)) return(entries)
  prefix <- store$prefix
  keep <- rep(TRUE, length(entries))
  out <- entries
  for (i in seq_along(out)) {
    path <- out[[i]]$path
    if (nzchar(prefix)) {
      if (!startsWith(path, prefix)) {
        keep[[i]] <- FALSE
        next
      }
      path <- substring(path, nchar(prefix) + 1L)
    }
    if (!nzchar(path)) {
      keep[[i]] <- FALSE
      next
    }
    out[[i]]$path <- path
  }
  out[keep]
}
