#' Byte store adapter
#'
#' A small key-to-bytes adapter over an `OpendalFs` prefix. It is intended as a
#' substrate for Zarr-like chunk layouts and other index-driven readers that want
#' store-relative keys without leaving Ropendal's byte-first filesystem API.
#'
#' `store_cache()` wraps a byte store in an explicit local full-object cache. It
#' is useful for chunked layouts where each key is already a small block. Partial
#' byte-range reads and non-default read shaping bypass this first cache layer;
#' future block caches can add range-aware eviction and readahead.
#'
#' @name byte-store
#' @aliases byte_store store_cache store_cache_clear store_read store_write
#'   store_replace store_exists store_list store_delete
#' @param fs Ropendal filesystem handle.
#' @param prefix Root-relative filesystem prefix used as the store root.
#' @param store Byte store object returned by `byte_store()` or `store_cache()`.
#' @param cache_dir Local directory used by `store_cache()`.
#' @param validate Cache validation strategy. `"last_modified_size"` compares
#'   parent object size and modification time before using a cached value;
#'   `"none"` trusts cached objects until explicit invalidation.
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
#' @return `byte_store()` and `store_cache()` return byte-store objects. Store
#'   operations return the corresponding filesystem values with paths rewritten
#'   relative to the store root for listings.
#' @examples
#' root <- tempfile("ropendal-store-")
#' dir.create(root)
#' fs <- opendal("fs", root = root)
#' store <- byte_store(fs, "array.zarr")
#' store_write(store, "zarr.json", charToRaw("{}"))
#' store_read(store, "zarr.json")
#' cached <- store_cache(store, tempfile("ropendal-cache-"), validate = "none")
#' store_read(cached, "zarr.json")
#' @export
byte_store <- function(fs, prefix = "") {
  .ropendal_check_fs(fs)
  structure(
    list(fs = fs, prefix = .ropendal_store_prefix(fs, prefix)),
    class = "ropendalByteStore"
  )
}

#' @rdname byte-store
#' @export
store_cache <- function(store, cache_dir = tools::R_user_dir("Ropendal", "cache"),
                        validate = c("last_modified_size", "none")) {
  store <- .ropendal_check_store(store)
  if (.ropendal_is_cached_store(store)) stop("store is already cached", call. = FALSE)
  validate <- match.arg(validate)
  if (!is.character(cache_dir) || length(cache_dir) != 1L || is.na(cache_dir) || !nzchar(cache_dir)) {
    stop("cache_dir must be a non-empty scalar string", call. = FALSE)
  }
  dir.create(cache_dir, recursive = TRUE, showWarnings = FALSE)
  if (!dir.exists(cache_dir)) stop("cache_dir could not be created", call. = FALSE)

  info <- fs_info(store$fs)
  namespace <- .ropendal_store_key_hex(paste(info$scheme, info$root, store$prefix, sep = "\n"))
  cache_fs <- opendal("fs", root = cache_dir)
  cache_store <- byte_store(cache_fs, paste0("store-cache/", namespace))
  structure(
    list(
      fs = store$fs,
      prefix = store$prefix,
      parent = store,
      cache_store = cache_store,
      cache_dir = cache_dir,
      validate = validate
    ),
    class = c("ropendalCachedByteStore", "ropendalByteStore")
  )
}

#' @export
print.ropendalByteStore <- function(x, ...) {
  cat("<opendal byte store> prefix=", sQuote(x$prefix), "\n", sep = "")
  invisible(x)
}

#' @export
print.ropendalCachedByteStore <- function(x, ...) {
  cat(
    "<opendal cached byte store> prefix=", sQuote(x$prefix),
    " validate=", x$validate, "\n",
    sep = ""
  )
  invisible(x)
}

#' @rdname byte-store
#' @export
store_cache_clear <- function(store) {
  store <- .ropendal_check_cached_store(store)
  invisible(store_delete(store$cache_store, "objects", recursive = TRUE))
  invisible(store_delete(store$cache_store, "meta", recursive = TRUE))
  invisible(TRUE)
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
  mode <- match.arg(mode)
  result <- match.arg(result)
  if (.ropendal_is_cached_store(store)) {
    return(.ropendal_cached_store_read(
      store, key, mode = mode, offset = offset, size = size, end = end,
      result = result, batch_concurrency = batch_concurrency,
      read_concurrency = read_concurrency, chunk_size = chunk_size,
      coalesce_gap = coalesce_gap
    ))
  }
  path <- .ropendal_store_path(store, key, object = TRUE)
  if (identical(mode, "bytes")) {
    return(fs_read_bytes(
      store$fs, path, offset = offset, size = size, end = end,
      result = result, batch_concurrency = batch_concurrency,
      read_concurrency = read_concurrency, chunk_size = chunk_size,
      coalesce_gap = coalesce_gap
    ))
  }
  fs_read(
    store$fs, path, offset = offset, size = size, end = end,
    result = result, batch_concurrency = batch_concurrency,
    read_concurrency = read_concurrency, chunk_size = chunk_size,
    coalesce_gap = coalesce_gap
  )
}

#' @rdname byte-store
#' @export
store_write <- function(store, key, data, batch_concurrency = NULL,
                        write_concurrency = NULL, chunk_size = NULL) {
  store <- .ropendal_check_store(store)
  if (.ropendal_is_cached_store(store)) {
    value <- store_write(
      store$parent, key, data,
      batch_concurrency = batch_concurrency,
      write_concurrency = write_concurrency,
      chunk_size = chunk_size
    )
    .ropendal_cached_store_invalidate(store, key)
    return(value)
  }
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
  if (.ropendal_is_cached_store(store)) {
    value <- store_replace(
      store$parent, key, data,
      batch_concurrency = batch_concurrency,
      write_concurrency = write_concurrency,
      chunk_size = chunk_size
    )
    .ropendal_cached_store_invalidate(store, key)
    return(value)
  }
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
  if (.ropendal_is_cached_store(store)) {
    return(store_exists(store$parent, key, batch_concurrency = batch_concurrency))
  }
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
  if (.ropendal_is_cached_store(store)) {
    return(store_list(store$parent, path, recursive = recursive, limit = limit, start_after = start_after))
  }
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
  if (.ropendal_is_cached_store(store)) {
    value <- store_delete(store$parent, key, recursive = recursive, batch_concurrency = batch_concurrency)
    if (recursive) store_cache_clear(store) else .ropendal_cached_store_invalidate(store, key)
    return(value)
  }
  fs_delete(
    store$fs,
    .ropendal_store_path(store, key, directory = recursive, object = !recursive),
    recursive = recursive,
    batch_concurrency = batch_concurrency
  )
}

.ropendal_check_store <- function(store) {
  if (!inherits(store, "ropendalByteStore")) stop("store must come from byte_store()", call. = FALSE)
  if (!.ropendal_is_cached_store(store)) .ropendal_check_fs(store$fs)
  store
}

.ropendal_is_cached_store <- function(store) inherits(store, "ropendalCachedByteStore")

.ropendal_check_cached_store <- function(store) {
  store <- .ropendal_check_store(store)
  if (!.ropendal_is_cached_store(store)) stop("store must come from store_cache()", call. = FALSE)
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

.ropendal_cached_store_read <- function(store, key, mode, offset, size, end, result,
                                        batch_concurrency, read_concurrency,
                                        chunk_size, coalesce_gap) {
  if (!.ropendal_is_complete_store_read(offset, size, end) || !identical(result, "auto")) {
    return(store_read(
      store$parent, key, mode = mode, offset = offset, size = size, end = end,
      result = result, batch_concurrency = batch_concurrency,
      read_concurrency = read_concurrency, chunk_size = chunk_size,
      coalesce_gap = coalesce_gap
    ))
  }
  .ropendal_store_path(store$parent, key, object = TRUE)
  values <- lapply(key, .ropendal_cached_store_read_one, store = store, mode = mode)
  if (length(values) == 1L) values[[1L]] else values
}

.ropendal_is_complete_store_read <- function(offset, size, end) {
  is.null(offset) && is.null(size) && is.null(end)
}

.ropendal_cached_store_read_one <- function(key, store, mode) {
  cache_key <- .ropendal_cache_object_key(key)
  meta_key <- .ropendal_cache_meta_key(key)
  cache_exists <- isTRUE(store_exists(store$cache_store, cache_key))

  if (cache_exists && .ropendal_cached_store_valid(store, key, meta_key)) {
    return(store_read(store$cache_store, cache_key, mode = mode))
  }

  value <- store_read(store$parent, key, mode = "raw")
  if (is_error_value(value)) return(value)

  invisible(store_replace(store$cache_store, cache_key, value))
  meta <- .ropendal_parent_store_meta(store$parent, key)
  if (!is_error_value(meta)) {
    invisible(store_replace(store$cache_store, meta_key, serialize(meta, NULL)))
  }
  if (identical(mode, "bytes")) store_read(store$cache_store, cache_key, mode = "bytes") else value
}

.ropendal_cached_store_valid <- function(store, key, meta_key) {
  if (identical(store$validate, "none")) return(TRUE)

  current <- .ropendal_parent_store_meta(store$parent, key)
  if (is_error_value(current)) return(FALSE)
  cached <- tryCatch(deserialize_raw(store_read(store$cache_store, meta_key)), error = function(e) NULL)
  is.list(cached) && identical(current$size, cached$size) &&
    identical(current$last_modified, cached$last_modified)
}

.ropendal_parent_store_meta <- function(store, key) {
  stat <- fs_stat(store$fs, .ropendal_store_path(store, key, object = TRUE))
  if (is_error_value(stat)) return(stat)
  list(size = stat$size, last_modified = stat$last_modified)
}

.ropendal_cached_store_invalidate <- function(store, key) {
  if (!is.character(key) || !length(key) || anyNA(key)) return(invisible(FALSE))
  for (one in key) {
    if (!nzchar(one) || endsWith(one, "/")) next
    cache_key <- .ropendal_cache_object_key(one)
    meta_key <- .ropendal_cache_meta_key(one)
    if (isTRUE(store_exists(store$cache_store, cache_key))) invisible(store_delete(store$cache_store, cache_key))
    if (isTRUE(store_exists(store$cache_store, meta_key))) invisible(store_delete(store$cache_store, meta_key))
  }
  invisible(TRUE)
}

.ropendal_cache_object_key <- function(key) paste0("objects/", .ropendal_store_key_hex(key))

.ropendal_cache_meta_key <- function(key) paste0("meta/", .ropendal_store_key_hex(key), ".rds")

.ropendal_store_key_hex <- function(key) {
  paste(sprintf("%02x", as.integer(charToRaw(enc2utf8(key)))), collapse = "")
}
