#' Byte store adapter
#'
#' A small key-to-bytes adapter over an `OpendalFs` prefix. It is intended as a
#' substrate for Zarr-like chunk layouts and other index-driven readers that want
#' store-relative keys without leaving Ropendal's byte-first filesystem API.
#'
#' `store_read()` returns `OpendalBytes` by default so callers can stay below R
#' raw-vector materialization until they explicitly ask for `mode = "raw"` or
#' `as.raw()`. The store layer is intentionally byte-only: serializers,
#' deserializers, codecs, and format parsers belong above it.
#'
#' `store_cache()` wraps a byte store in an explicit local full-object cache. It
#' is useful for chunked layouts where each key is already a small block. Partial
#' byte-range reads and non-default read shaping bypass this first cache layer.
#' `store_block_cache()` wraps a byte store in an explicit fixed-size block cache
#' for scalar complete or range reads with `result = "auto"` while keeping
#' format semantics above the store layer. Vectorized ranges, vectorized keys,
#' and non-auto result shapes bypass this cache adapter and read from the parent
#' store directly.
#'
#' @name byte-store
#' @aliases byte_store store_cache store_block_cache store_cache_clear store_read store_read_aio
#'   store_write store_write_aio store_replace store_replace_aio store_exists
#'   store_exists_aio store_list store_list_aio store_delete store_delete_aio
#' @param fs Ropendal filesystem handle.
#' @param prefix Root-relative filesystem prefix used as the store root.
#' @param store Byte store object returned by `byte_store()`, `store_cache()`,
#'   or `store_block_cache()`.
#' @param cache_dir Local directory used by `store_cache()` or
#'   `store_block_cache()`.
#' @param block_size Fixed block size in bytes for `store_block_cache()`.
#' @param validate Cache validation strategy. `"last_modified_size"` compares
#'   parent object size and modification time before using a cached value.
#'   `"none"` skips modification-time checks; block caches still stat the
#'   parent to bound ranges and refresh when cached block metadata has a
#'   different object size.
#' @param key Store-relative object key or keys.
#' @param path Store-relative directory path for `store_list()`.
#' @param data Raw vector, `OpendalBytes`, or list of raw vectors/byte handles.
#' @param mode Read materialization: `OpendalBytes` handles or raw vectors.
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
#' @return `byte_store()`, `store_cache()`, and `store_block_cache()` return
#'   byte-store objects. Store operations return the corresponding filesystem values with paths rewritten
#'   relative to the store root for listings.
#' @examples
#' root <- tempfile("ropendal-store-")
#' dir.create(root)
#' fs <- opendal("fs", root = root)
#' store <- byte_store(fs, "array.zarr")
#' store_write(store, "zarr.json", charToRaw("{}"))
#' as.raw(store_read(store, "zarr.json"))
#' cached <- store_cache(store, tempfile("ropendal-cache-"), validate = "none")
#' as.raw(store_read(cached, "zarr.json"))
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
  cache_store <- .ropendal_cache_store_for(store, cache_dir, "store-cache")
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

#' @rdname byte-store
#' @export
store_block_cache <- function(store, cache_dir = tools::R_user_dir("Ropendal", "cache"),
                              block_size = 8 * 1024^2,
                              validate = c("last_modified_size", "none")) {
  store <- .ropendal_check_store(store)
  if (.ropendal_is_cached_store(store)) stop("store is already cached", call. = FALSE)
  validate <- match.arg(validate)
  block_size <- .ropendal_check_positive_byte_count(block_size, "block_size")
  cache_store <- .ropendal_cache_store_for(store, cache_dir, "store-block-cache")
  structure(
    list(
      fs = store$fs,
      prefix = store$prefix,
      parent = store,
      cache_store = cache_store,
      cache_dir = cache_dir,
      validate = validate,
      block_size = block_size
    ),
    class = c("ropendalBlockCachedByteStore", "ropendalByteStore")
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

#' @export
print.ropendalBlockCachedByteStore <- function(x, ...) {
  cat(
    "<opendal block-cached byte store> prefix=", sQuote(x$prefix),
    " block_size=", format(x$block_size, scientific = FALSE),
    " validate=", x$validate, "\n",
    sep = ""
  )
  invisible(x)
}

#' @rdname byte-store
#' @export
store_cache_clear <- function(store) {
  store <- .ropendal_check_cached_store(store)
  if (.ropendal_is_block_cached_store(store)) {
    invisible(store_delete(store$cache_store, "blocks", recursive = TRUE))
    invisible(store_delete(store$cache_store, "block_meta", recursive = TRUE))
  } else {
    invisible(store_delete(store$cache_store, "objects", recursive = TRUE))
    invisible(store_delete(store$cache_store, "meta", recursive = TRUE))
  }
  invisible(TRUE)
}

#' @rdname byte-store
#' @export
store_read <- function(store, key, mode = c("bytes", "raw"),
                       offset = NULL, size = NULL, end = NULL,
                       result = c("auto", "flat", "nested"),
                       batch_concurrency = NULL,
                       read_concurrency = NULL,
                       chunk_size = NULL,
                       coalesce_gap = NULL) {
  store <- .ropendal_check_store(store)
  mode <- match.arg(mode)
  result <- match.arg(result)
  if (.ropendal_is_block_cached_store(store)) {
    return(.ropendal_block_cached_store_read(
      store, key, mode = mode, offset = offset, size = size, end = end,
      result = result, batch_concurrency = batch_concurrency,
      read_concurrency = read_concurrency, chunk_size = chunk_size,
      coalesce_gap = coalesce_gap
    ))
  }
  if (.ropendal_is_full_cached_store(store)) {
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
store_read_aio <- function(store, key, mode = c("bytes", "raw"),
                           offset = NULL, size = NULL, end = NULL,
                           result = c("auto", "flat", "nested"),
                           batch_concurrency = NULL,
                           read_concurrency = NULL,
                           chunk_size = NULL,
                           coalesce_gap = NULL) {
  store <- .ropendal_check_store(store)
  mode <- match.arg(mode)
  result <- match.arg(result)
  if (.ropendal_is_block_cached_store(store)) {
    return(.ropendal_block_cached_store_read_aio(
      store, key, mode = mode, offset = offset, size = size, end = end,
      result = result, batch_concurrency = batch_concurrency,
      read_concurrency = read_concurrency, chunk_size = chunk_size,
      coalesce_gap = coalesce_gap
    ))
  }
  if (.ropendal_is_full_cached_store(store)) {
    return(.ropendal_cached_store_read_aio(
      store, key, mode = mode, offset = offset, size = size, end = end,
      result = result, batch_concurrency = batch_concurrency,
      read_concurrency = read_concurrency, chunk_size = chunk_size,
      coalesce_gap = coalesce_gap
    ))
  }
  path <- .ropendal_store_path(store, key, object = TRUE)
  if (identical(mode, "bytes")) {
    return(fs_read_bytes_aio(
      store$fs, path, offset = offset, size = size, end = end,
      result = result, batch_concurrency = batch_concurrency,
      read_concurrency = read_concurrency, chunk_size = chunk_size,
      coalesce_gap = coalesce_gap
    ))
  }
  fs_read_aio(
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
    .ropendal_cached_store_invalidate_any(store, key)
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
store_write_aio <- function(store, key, data, batch_concurrency = NULL,
                            write_concurrency = NULL, chunk_size = NULL) {
  store <- .ropendal_check_store(store)
  if (.ropendal_is_cached_store(store)) {
    .ropendal_cached_store_invalidate_any(store, key)
    return(store_write_aio(
      store$parent, key, data,
      batch_concurrency = batch_concurrency,
      write_concurrency = write_concurrency,
      chunk_size = chunk_size
    ))
  }
  fs_write_aio(
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
    .ropendal_cached_store_invalidate_any(store, key)
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
store_replace_aio <- function(store, key, data, batch_concurrency = NULL,
                              write_concurrency = NULL, chunk_size = NULL) {
  store <- .ropendal_check_store(store)
  if (.ropendal_is_cached_store(store)) {
    .ropendal_cached_store_invalidate_any(store, key)
    return(store_replace_aio(
      store$parent, key, data,
      batch_concurrency = batch_concurrency,
      write_concurrency = write_concurrency,
      chunk_size = chunk_size
    ))
  }
  fs_replace_aio(
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
store_exists_aio <- function(store, key, batch_concurrency = NULL) {
  store <- .ropendal_check_store(store)
  if (.ropendal_is_cached_store(store)) {
    return(store_exists_aio(store$parent, key, batch_concurrency = batch_concurrency))
  }
  fs_exists_aio(
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
store_list_aio <- function(store, path = "", recursive = FALSE, limit = NULL, start_after = NULL) {
  store <- .ropendal_check_store(store)
  if (.ropendal_is_cached_store(store)) {
    return(store_list_aio(store$parent, path, recursive = recursive, limit = limit, start_after = start_after))
  }
  path <- .ropendal_store_path(store, path, directory = TRUE, allow_empty = TRUE)
  start_after <- if (is.null(start_after)) {
    NULL
  } else {
    .ropendal_store_path(store, start_after, allow_empty = FALSE)
  }
  opendal_aio_with_bindings(
    fs_ls_aio(store$fs, path, recursive = recursive, limit = limit, start_after = start_after),
    materializer = function(entries) .ropendal_store_relative_entries(store, entries)
  )
}

#' @rdname byte-store
#' @export
store_delete <- function(store, key, recursive = FALSE, batch_concurrency = NULL) {
  store <- .ropendal_check_store(store)
  if (.ropendal_is_cached_store(store)) {
    value <- store_delete(store$parent, key, recursive = recursive, batch_concurrency = batch_concurrency)
    if (recursive) store_cache_clear(store) else .ropendal_cached_store_invalidate_any(store, key)
    return(value)
  }
  fs_delete(
    store$fs,
    .ropendal_store_path(store, key, directory = recursive, object = !recursive),
    recursive = recursive,
    batch_concurrency = batch_concurrency
  )
}

#' @rdname byte-store
#' @export
store_delete_aio <- function(store, key, recursive = FALSE, batch_concurrency = NULL) {
  store <- .ropendal_check_store(store)
  if (.ropendal_is_cached_store(store)) {
    if (recursive) store_cache_clear(store) else .ropendal_cached_store_invalidate_any(store, key)
    return(store_delete_aio(store$parent, key, recursive = recursive, batch_concurrency = batch_concurrency))
  }
  fs_delete_aio(
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

.ropendal_is_full_cached_store <- function(store) inherits(store, "ropendalCachedByteStore")

.ropendal_is_block_cached_store <- function(store) inherits(store, "ropendalBlockCachedByteStore")

.ropendal_is_cached_store <- function(store) {
  .ropendal_is_full_cached_store(store) || .ropendal_is_block_cached_store(store)
}

.ropendal_check_cached_store <- function(store) {
  store <- .ropendal_check_store(store)
  if (!.ropendal_is_cached_store(store)) stop("store must come from store_cache() or store_block_cache()", call. = FALSE)
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

.ropendal_cache_store_for <- function(store, cache_dir, namespace_prefix) {
  if (!is.character(cache_dir) || length(cache_dir) != 1L || is.na(cache_dir) || !nzchar(cache_dir)) {
    stop("cache_dir must be a non-empty scalar string", call. = FALSE)
  }
  dir.create(cache_dir, recursive = TRUE, showWarnings = FALSE)
  if (!dir.exists(cache_dir)) stop("cache_dir could not be created", call. = FALSE)

  info <- fs_info(store$fs)
  namespace <- .ropendal_store_key_hex(paste(info$scheme, info$root, store$prefix, sep = "\n"))
  cache_fs <- opendal("fs", root = cache_dir)
  byte_store(cache_fs, paste0(namespace_prefix, "/", namespace))
}

.ropendal_check_positive_byte_count <- function(value, name) {
  value <- .ropendal_check_byte_count(value, name)
  if (value <= 0) stop(name, " must be positive", call. = FALSE)
  value
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

.ropendal_block_cached_store_read <- function(store, key, mode, offset, size, end, result,
                                           batch_concurrency, read_concurrency,
                                           chunk_size, coalesce_gap) {
  if (!.ropendal_block_cache_scalar_shape(key, offset, size, end, result)) {
    return(store_read(
      store$parent, key, mode = mode, offset = offset, size = size, end = end,
      result = result, batch_concurrency = batch_concurrency,
      read_concurrency = read_concurrency, chunk_size = chunk_size,
      coalesce_gap = coalesce_gap
    ))
  }
  plan <- .ropendal_block_cache_plan(store, key, offset, size, end)
  if (is_error_value(plan)) return(plan)
  if (!length(plan$blocks)) return(.ropendal_block_cache_assemble(plan, list(), mode))

  valid <- vapply(plan$blocks, .ropendal_block_cached_store_block_valid, logical(1), store = store, key = key, meta = plan$meta)
  if (all(valid)) {
    blocks <- lapply(plan$blocks, function(block) {
      store_read(store$cache_store, .ropendal_block_cache_object_key(block, store, key), mode = "bytes")
    })
    err <- .ropendal_first_error(blocks)
    if (!is.null(err) || !.ropendal_block_cache_lengths_ok(plan, blocks)) {
      .ropendal_block_cached_store_invalidate(store, key)
      blocks <- .ropendal_block_cache_fetch_parent_blocks(store, key, plan)
    }
  } else {
    if (identical(store$validate, "none")) .ropendal_block_cached_store_invalidate(store, key)
    blocks <- .ropendal_block_cache_fetch_parent_blocks(store, key, plan)
  }
  err <- .ropendal_first_error(blocks)
  if (!is.null(err)) return(err)
  if (!.ropendal_block_cache_lengths_ok(plan, blocks)) {
    return(store_read(
      store$parent, key, mode = mode, offset = offset, size = size, end = end,
      result = result, batch_concurrency = batch_concurrency,
      read_concurrency = read_concurrency, chunk_size = chunk_size,
      coalesce_gap = coalesce_gap
    ))
  }
  .ropendal_block_cache_assemble(plan, blocks, mode)
}

.ropendal_block_cached_store_read_aio <- function(store, key, mode, offset, size, end, result,
                                               batch_concurrency, read_concurrency,
                                               chunk_size, coalesce_gap) {
  if (!.ropendal_block_cache_scalar_shape(key, offset, size, end, result)) {
    return(store_read_aio(
      store$parent, key, mode = mode, offset = offset, size = size, end = end,
      result = result, batch_concurrency = batch_concurrency,
      read_concurrency = read_concurrency, chunk_size = chunk_size,
      coalesce_gap = coalesce_gap
    ))
  }
  plan <- .ropendal_block_cache_plan(store, key, offset, size, end)
  if (is_error_value(plan)) {
    return(store_read_aio(
      store$parent, key, mode = mode, offset = offset, size = size, end = end,
      result = result, batch_concurrency = batch_concurrency,
      read_concurrency = read_concurrency, chunk_size = chunk_size,
      coalesce_gap = coalesce_gap
    ))
  }
  if (!length(plan$blocks)) {
    return(store_read_aio(
      store$parent, key, mode = mode, offset = plan$start, size = 0,
      result = result, batch_concurrency = batch_concurrency,
      read_concurrency = read_concurrency, chunk_size = chunk_size,
      coalesce_gap = coalesce_gap
    ))
  }

  valid <- vapply(plan$blocks, .ropendal_block_cached_store_block_valid, logical(1), store = store, key = key, meta = plan$meta)
  if (all(valid)) {
    cache_keys <- vapply(plan$blocks, .ropendal_block_cache_object_key, character(1), store = store, key = key)
    aio <- if (length(cache_keys) == 1L) {
      store_read_aio(
        store$cache_store, cache_keys[[1L]], mode = "bytes",
        batch_concurrency = batch_concurrency,
        read_concurrency = read_concurrency, chunk_size = chunk_size,
        coalesce_gap = coalesce_gap
      )
    } else {
      store_read_aio(
        store$cache_store, cache_keys, mode = "bytes", result = "flat",
        batch_concurrency = batch_concurrency,
        read_concurrency = read_concurrency, chunk_size = chunk_size,
        coalesce_gap = coalesce_gap
      )
    }
    return(opendal_aio_with_bindings(aio, materializer = function(value) {
      blocks <- .ropendal_block_cache_value_list(value, length(cache_keys))
      err <- .ropendal_first_error(blocks)
      if (!is.null(err)) {
        return(.ropendal_block_cached_store_read(
          store, key, mode = mode, offset = offset, size = size, end = end,
          result = result, batch_concurrency = batch_concurrency,
          read_concurrency = read_concurrency, chunk_size = chunk_size,
          coalesce_gap = coalesce_gap
        ))
      }
      if (!.ropendal_block_cache_lengths_ok(plan, blocks)) {
        return(.ropendal_block_cached_store_read(
          store, key, mode = mode, offset = offset, size = size, end = end,
          result = result, batch_concurrency = batch_concurrency,
          read_concurrency = read_concurrency, chunk_size = chunk_size,
          coalesce_gap = coalesce_gap
        ))
      }
      .ropendal_block_cache_assemble(plan, blocks, mode)
    }))
  }

  if (identical(store$validate, "none")) .ropendal_block_cached_store_invalidate(store, key)
  offsets <- plan$blocks * store$block_size
  sizes <- vapply(plan$blocks, .ropendal_block_cache_expected_len, numeric(1), store = store, meta = plan$meta)
  aio <- if (length(plan$blocks) == 1L) {
    store_read_aio(
      store$parent, key, mode = "bytes", offset = offsets[[1L]], size = sizes[[1L]],
      batch_concurrency = batch_concurrency,
      read_concurrency = read_concurrency, chunk_size = chunk_size,
      coalesce_gap = coalesce_gap
    )
  } else {
    store_read_aio(
      store$parent, rep(key, length(plan$blocks)), mode = "bytes", offset = offsets,
      size = sizes, result = "flat", batch_concurrency = batch_concurrency,
      read_concurrency = read_concurrency, chunk_size = chunk_size,
      coalesce_gap = coalesce_gap
    )
  }
  opendal_aio_with_bindings(aio, materializer = function(value) {
    blocks <- .ropendal_block_cache_value_list(value, length(plan$blocks))
    err <- .ropendal_first_error(blocks)
    if (!is.null(err)) return(err)
    if (!.ropendal_block_cache_lengths_ok(plan, blocks)) {
      return(.ropendal_block_cached_store_read(
        store, key, mode = mode, offset = offset, size = size, end = end,
        result = result, batch_concurrency = batch_concurrency,
        read_concurrency = read_concurrency, chunk_size = chunk_size,
        coalesce_gap = coalesce_gap
      ))
    }
    for (i in seq_along(plan$blocks)) {
      .ropendal_block_cache_fill_block(store, key, plan$meta, plan$blocks[[i]], blocks[[i]])
    }
    .ropendal_block_cache_assemble(plan, blocks, mode)
  })
}

.ropendal_block_cache_scalar_shape <- function(key, offset, size, end, result) {
  length(key) == 1L && identical(result, "auto") &&
    !is.list(offset) && !is.list(size) && !is.list(end) &&
    (is.null(offset) || length(offset) == 1L) &&
    (is.null(size) || length(size) == 1L) &&
    (is.null(end) || length(end) == 1L)
}

.ropendal_block_cache_plan <- function(store, key, offset, size, end) {
  meta <- .ropendal_parent_store_meta(store$parent, key)
  if (is_error_value(meta)) return(meta)
  start <- if (is.null(offset)) 0 else .ropendal_check_byte_count(offset, "offset")
  if (!is.null(size) && !is.null(end)) stop("use only one of size or end", call. = FALSE)
  requested_end <- if (!is.null(end)) {
    .ropendal_check_byte_count(end, "end")
  } else if (!is.null(size)) {
    start + .ropendal_check_byte_count(size, "size")
  } else {
    meta$size
  }
  requested_end <- min(meta$size, max(start, requested_end))
  blocks <- if (requested_end <= start) {
    numeric(0)
  } else {
    seq.int(start %/% store$block_size, (requested_end - 1) %/% store$block_size)
  }
  list(meta = meta, start = start, end = requested_end, blocks = blocks, block_size = store$block_size)
}

.ropendal_block_cache_fetch_parent_blocks <- function(store, key, plan) {
  lapply(plan$blocks, function(block) {
    block_start <- block * store$block_size
    block_size <- .ropendal_block_cache_expected_len(block, store, plan$meta)
    value <- store_read(store$parent, key, mode = "bytes", offset = block_start, size = block_size)
    if (is_error_value(value)) return(value)
    if (!inherits(value, "OpendalBytes") || length(value) != block_size) return(value)
    .ropendal_block_cache_fill_block(store, key, plan$meta, block, value)
    value
  })
}

.ropendal_block_cached_store_block_valid <- function(block, store, key, meta) {
  cache_key <- .ropendal_block_cache_object_key(block, store, key)
  if (!isTRUE(store_exists(store$cache_store, cache_key))) return(FALSE)
  meta_key <- .ropendal_block_cache_meta_key(block, store, key)
  cached <- tryCatch(deserialize_raw(store_read(store$cache_store, meta_key)), error = function(e) NULL)
  if (!is.list(cached) || !identical(meta$size, cached$size)) {
    .ropendal_block_cached_store_invalidate(store, key)
    return(FALSE)
  }
  if (identical(store$validate, "none")) return(TRUE)
  if (!identical(meta$last_modified, cached$last_modified)) {
    .ropendal_block_cached_store_invalidate(store, key)
    return(FALSE)
  }
  TRUE
}

.ropendal_block_cache_fill_block <- function(store, key, meta, block, value) {
  if (is_error_value(value)) return(invisible(FALSE))
  invisible(store_replace(store$cache_store, .ropendal_block_cache_object_key(block, store, key), value))
  invisible(store_replace(store$cache_store, .ropendal_block_cache_meta_key(block, store, key), serialize(meta, NULL)))
  invisible(TRUE)
}

.ropendal_block_cache_assemble <- function(plan, blocks, mode) {
  if (!length(plan$blocks)) {
    return(if (identical(mode, "bytes")) opendal_bytes_from_raw(raw(0)) else raw(0))
  }
  pieces <- vector("list", length(blocks))
  for (i in seq_along(blocks)) {
    block <- plan$blocks[[i]]
    block_start <- block * plan$block_size
    copy_start <- max(plan$start, block_start) - block_start
    copy_end <- min(plan$end, block_start + length(blocks[[i]])) - block_start
    pieces[[i]] <- opendal_bytes_slice(blocks[[i]], offset = copy_start, size = max(0, copy_end - copy_start))
  }
  if (length(pieces) == 1L) {
    return(if (identical(mode, "bytes")) pieces[[1L]] else as.raw(pieces[[1L]]))
  }
  raw_value <- do.call(c, lapply(pieces, as.raw))
  if (identical(mode, "bytes")) opendal_bytes_from_raw(raw_value) else raw_value
}

.ropendal_block_cache_lengths_ok <- function(plan, blocks) {
  if (length(blocks) != length(plan$blocks)) return(FALSE)
  for (i in seq_along(blocks)) {
    expected <- max(0, min(plan$block_size, plan$meta$size - plan$blocks[[i]] * plan$block_size))
    if (!inherits(blocks[[i]], "OpendalBytes") || length(blocks[[i]]) != expected) return(FALSE)
  }
  TRUE
}

.ropendal_block_cache_expected_len <- function(block, store, meta) {
  max(0, min(store$block_size, meta$size - block * store$block_size))
}

.ropendal_block_cache_object_key <- function(block, store, key) {
  paste0("blocks/", format(store$block_size, scientific = FALSE, trim = TRUE), "/", .ropendal_store_key_hex(key), "/", sprintf("%016.0f", block), ".bin")
}

.ropendal_block_cache_meta_key <- function(block, store, key) {
  paste0("block_meta/", format(store$block_size, scientific = FALSE, trim = TRUE), "/", .ropendal_store_key_hex(key), "/", sprintf("%016.0f", block), ".rds")
}

.ropendal_block_cache_value_list <- function(value, n) {
  if (is_error_value(value) || n == 1L) list(value) else value
}

.ropendal_first_error <- function(values) {
  if (is_error_value(values)) return(values)
  for (value in values) if (is_error_value(value)) return(value)
  NULL
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

.ropendal_cached_store_read_aio <- function(store, key, mode, offset, size, end, result,
                                            batch_concurrency, read_concurrency,
                                            chunk_size, coalesce_gap) {
  if (!.ropendal_is_complete_store_read(offset, size, end) || !identical(result, "auto")) {
    return(store_read_aio(
      store$parent, key, mode = mode, offset = offset, size = size, end = end,
      result = result, batch_concurrency = batch_concurrency,
      read_concurrency = read_concurrency, chunk_size = chunk_size,
      coalesce_gap = coalesce_gap
    ))
  }
  .ropendal_store_path(store$parent, key, object = TRUE)
  cache_keys <- vapply(key, .ropendal_cache_object_key, character(1), USE.NAMES = FALSE)
  meta_keys <- vapply(key, .ropendal_cache_meta_key, character(1), USE.NAMES = FALSE)
  hits <- vapply(
    seq_along(key),
    function(i) isTRUE(store_exists(store$cache_store, cache_keys[[i]])) &&
      .ropendal_cached_store_valid(store, key[[i]], meta_keys[[i]]),
    logical(1)
  )
  if (all(hits)) {
    return(store_read_aio(
      store$cache_store, cache_keys, mode = mode,
      batch_concurrency = batch_concurrency,
      read_concurrency = read_concurrency, chunk_size = chunk_size,
      coalesce_gap = coalesce_gap
    ))
  }

  paths <- .ropendal_store_path(store$parent, key, object = TRUE)
  aio <- fs_read_bytes_aio(
    store$parent$fs, paths, result = result,
    batch_concurrency = batch_concurrency,
    read_concurrency = read_concurrency, chunk_size = chunk_size,
    coalesce_gap = coalesce_gap
  )
  opendal_aio_with_bindings(
    aio,
    materializer = function(value) {
      .ropendal_cached_store_fill(store, key, cache_keys, meta_keys, value)
      .ropendal_store_materialize_mode(value, mode)
    }
  )
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

.ropendal_block_cached_store_invalidate <- function(store, key) {
  if (!is.character(key) || !length(key) || anyNA(key)) return(invisible(FALSE))
  for (one in key) {
    if (!nzchar(one) || endsWith(one, "/")) next
    key_hex <- .ropendal_store_key_hex(one)
    block_prefix <- paste0("blocks/", format(store$block_size, scientific = FALSE, trim = TRUE), "/", key_hex)
    meta_prefix <- paste0("block_meta/", format(store$block_size, scientific = FALSE, trim = TRUE), "/", key_hex)
    invisible(store_delete(store$cache_store, block_prefix, recursive = TRUE))
    invisible(store_delete(store$cache_store, meta_prefix, recursive = TRUE))
  }
  invisible(TRUE)
}

.ropendal_cached_store_invalidate_any <- function(store, key) {
  if (.ropendal_is_block_cached_store(store)) {
    .ropendal_block_cached_store_invalidate(store, key)
  } else {
    .ropendal_cached_store_invalidate(store, key)
  }
}

.ropendal_cached_store_fill <- function(store, key, cache_keys, meta_keys, value) {
  if (is_error_value(value)) return(invisible(FALSE))
  values <- if (length(key) == 1L) list(value) else value
  if (!is.list(values) || length(values) != length(key)) return(invisible(FALSE))

  for (i in seq_along(key)) {
    one <- values[[i]]
    if (is_error_value(one)) next
    invisible(store_replace(store$cache_store, cache_keys[[i]], one))
    meta <- .ropendal_parent_store_meta(store$parent, key[[i]])
    if (!is_error_value(meta)) {
      invisible(store_replace(store$cache_store, meta_keys[[i]], serialize(meta, NULL)))
    }
  }
  invisible(TRUE)
}

.ropendal_store_materialize_mode <- function(value, mode) {
  if (!identical(mode, "raw")) return(value)
  .ropendal_store_as_raw_tree(value)
}

.ropendal_store_as_raw_tree <- function(value) {
  if (is_error_value(value)) return(value)
  if (is.raw(value)) return(value)
  if (inherits(value, "OpendalBytes")) return(as.raw(value))
  if (is.list(value)) return(lapply(value, .ropendal_store_as_raw_tree))
  value
}

.ropendal_cache_object_key <- function(key) paste0("objects/", .ropendal_store_key_hex(key))

.ropendal_cache_meta_key <- function(key) paste0("meta/", .ropendal_store_key_hex(key), ".rds")

.ropendal_store_key_hex <- function(key) {
  paste(sprintf("%02x", as.integer(charToRaw(enc2utf8(key)))), collapse = "")
}
