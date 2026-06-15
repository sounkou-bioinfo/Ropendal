library(Ropendal)

helper <- system.file("tinytest", "helper-ropendal.R", package = "Ropendal")
source(helper, local = TRUE)

root <- ropendal_temp_root()
fs <- opendal("fs", root = root)

expect_true(inherits(runtime_config(threads = 1), "ropendalRuntimeConfig"))
expect_true(inherits(layer_concurrent_limit(1), "ropendalConcurrentLimitLayer"))
expect_true(inherits(layer_timeout(request_timeout = 1, io_timeout = 1), "ropendalTimeoutLayer"))
limited_root <- ropendal_temp_root()
limited_fs <- opendal(
  "fs",
  root = limited_root,
  runtime = runtime_config(threads = 1),
  layers = list(layer_concurrent_limit(1), layer_timeout(request_timeout = 1, io_timeout = 1))
)
expect_true(inherits(limited_fs, "OpendalFs"))
expect_true(identical(fs_write(limited_fs, "limited.bin", as.raw(1)), TRUE))
expect_equal(fs_read(limited_fs, "limited.bin"), as.raw(1))
request_timeout_fs <- opendal("fs", root = ropendal_temp_root(), layers = list(layer_timeout(request_timeout = 1)))
expect_true(identical(fs_write(request_timeout_fs, "request-timeout.bin", as.raw(1)), TRUE))
io_timeout_fs <- opendal("fs", root = ropendal_temp_root(), layers = list(layer_timeout(io_timeout = 1)))
expect_true(identical(fs_write(io_timeout_fs, "io-timeout.bin", as.raw(1)), TRUE))
expect_error(opendal("fs", root = ropendal_temp_root(), runtime = list()), "runtime_config")
expect_error(opendal("fs", root = ropendal_temp_root(), layers = list(list())), "layer config")
expect_error(opendal("fs", root = ropendal_temp_root(), runtime = runtime_config(0)), "greater than zero")
expect_error(opendal("fs", root = ropendal_temp_root(), layers = list(layer_concurrent_limit(0))), "greater than zero")
expect_error(layer_timeout(), "timeout")
expect_error(opendal("fs", root = ropendal_temp_root(), layers = list(layer_timeout(request_timeout = 0))), "greater than zero")
expect_error(
  opendal(
    "fs",
    root = ropendal_temp_root(),
    layers = list(layer_concurrent_limit(1), layer_concurrent_limit(2))
  ),
  "only one"
)
expect_error(
  opendal(
    "fs",
    root = ropendal_temp_root(),
    layers = list(layer_timeout(request_timeout = 1), layer_timeout(io_timeout = 1))
  ),
  "only one"
)

expect_true(inherits(fs, "OpendalFs"))
expect_equal(fs_info(fs)$scheme, "fs")
fs_caps <- fs_capabilities(fs)
expect_true(inherits(fs_caps, "opendalCapabilityValue"))
expect_true(inherits(fs_caps$operations, "opendalCapabilityOperations"))
expect_equal(fs_caps$scheme, "fs")
expect_true(inherits(fs_caps$operations$read, "opendalCapabilityOperation"))
expect_true(fs_caps$operations$read$supported)
expect_equal(fs_caps$operations$read$implementation, "opendal")
expect_true(fs_caps$operations$read_range$supported)
expect_equal(fs_caps$operations$read_range$semantics, "native")
expect_true(fs_caps$operations$ls$supported)
uri_root <- ropendal_temp_root()
fs_uri <- opendal_uri(paste0("fs://", uri_root))
expect_true(inherits(fs_uri, "OpendalFs"))
expect_equal(fs_info(fs_uri)$scheme, "fs")
expect_true(identical(fs_write(fs_uri, "uri.bin", as.raw(7)), TRUE))
expect_equal(fs_read(fs_uri, "uri.bin"), as.raw(7))
expect_equal(fs_normalize_path(fs, "/a//b/../c"), "a/c")
expect_error(fs_normalize_path(fs, "../escape"))

bytes <- as.raw(c(1, 2, 3, 4))
expect_true(identical(fs_write(fs, "a.bin", bytes), TRUE))
expect_equal(fs_read(fs, "a.bin"), bytes)
expect_equal(fs_read(fs, "a.bin", offset = 1, size = 2), as.raw(c(2, 3)))
expect_equal(
  fs_read(fs, "a.bin", offset = c(0, 2), size = c(2, 2), result = "flat"),
  list(as.raw(c(1, 2)), as.raw(c(3, 4)))
)
expect_equal(
  fs_read(fs, "a.bin", offset = c(0, 2), size = c(1, 2)),
  list(as.raw(1), as.raw(c(3, 4)))
)
expect_equal(
  fs_read(fs, "a.bin", offset = c(0, 2), end = c(1, 4), result = "flat"),
  list(as.raw(1), as.raw(c(3, 4)))
)
range_nested <- fs_read(fs, "a.bin", offset = c(0, 2), size = c(1, 1), result = "nested")
expect_equal(length(range_nested), 1)
expect_equal(range_nested[[1]], list(as.raw(1), as.raw(3)))
expect_true(identical(fs_write(fs, "ranges-b.bin", as.raw(c(10, 11, 12))), TRUE))
expect_equal(
  fs_read(fs, c("a.bin", "ranges-b.bin"), result = "flat"),
  list(bytes, as.raw(c(10, 11, 12)))
)
range_nested_many <- fs_read(
  fs,
  c("a.bin", "ranges-b.bin"),
  offset = list(c(0, 2), c(1)),
  size = list(c(1, 2), c(2))
)
expect_equal(length(range_nested_many), 2)
expect_equal(range_nested_many[[1]], list(as.raw(1), as.raw(c(3, 4))))
expect_equal(range_nested_many[[2]], list(as.raw(c(11, 12))))
range_end_many <- fs_read(
  fs,
  c("a.bin", "ranges-b.bin"),
  offset = list(c(0, 2), c(1)),
  end = list(c(1, 4), c(3))
)
expect_equal(range_end_many[[1]], list(as.raw(1), as.raw(c(3, 4))))
expect_equal(range_end_many[[2]], list(as.raw(c(11, 12))))
bytes_ranges <- fs_read_bytes(fs, "a.bin", offset = c(0, 2), size = c(1, 1), result = "flat")
expect_true(all(vapply(bytes_ranges, inherits, logical(1), "OpendalBytes")))
expect_equal(as.raw(bytes_ranges[[2]]), as.raw(3))
bytes_ranges_aio <- collect_aio(fs_read_bytes_aio(fs, "a.bin", offset = c(1, 3), size = c(1, 1), result = "flat"))
expect_equal(as.raw(bytes_ranges_aio[[1]]), as.raw(2))
expect_equal(as.raw(bytes_ranges_aio[[2]]), as.raw(4))
raw_ranges_aio <- collect_aio(fs_read_aio(fs, c("a.bin", "ranges-b.bin"), offset = list(c(1), c(0, 2)), size = list(c(2), c(1, 1))))
expect_equal(raw_ranges_aio[[1]], list(as.raw(c(2, 3))))
expect_equal(raw_ranges_aio[[2]], list(as.raw(10), as.raw(12)))
range_req <- byte_ranges(
  path = c("a.bin", "a.bin", "ranges-b.bin"),
  offset = c(0, 2, 1),
  size = c(1, 2, 2),
  id = c("a:first", "a:tail", "b:middle")
)
expect_equal(
  fs_read(fs, range_req),
  structure(
    list(as.raw(1), as.raw(c(3, 4)), as.raw(c(11, 12))),
    names = c("a:first", "a:tail", "b:middle")
  )
)
range_req_aio <- collect_aio(fs_read_aio(fs, range_req))
expect_equal(names(range_req_aio), c("a:first", "a:tail", "b:middle"))
expect_equal(range_req_aio[[2]], as.raw(c(3, 4)))
range_req_bytes <- fs_read_bytes(fs, range_req)
expect_equal(names(range_req_bytes), c("a:first", "a:tail", "b:middle"))
expect_true(all(vapply(range_req_bytes, inherits, logical(1), "OpendalBytes")))
expect_equal(as.raw(range_req_bytes[[3]]), as.raw(c(11, 12)))
range_req_bytes_aio <- collect_aio(fs_read_bytes_aio(fs, range_req))
expect_equal(names(range_req_bytes_aio), c("a:first", "a:tail", "b:middle"))
expect_equal(as.raw(range_req_bytes_aio[[1]]), as.raw(1))
range_req_nested <- byte_ranges(
  path = c("a.bin", "ranges-b.bin"),
  offset = list(c(0, 2), c(1)),
  end = list(c(1, 4), c(3)),
  id = list(c("a:first", "a:tail"), "b:middle"),
  result = "nested"
)
expect_equal(names(fs_read(fs, range_req_nested)[[1]]), c("a:first", "a:tail"))
expect_error(fs_read(fs, range_req, offset = 0), "byte_ranges")
expect_error(fs_read(fs, range_req, size = 1), "byte_ranges")
expect_error(byte_ranges("a.bin", 0, size = 1, end = 1), "use only one")
expect_error(fs_read(fs, byte_ranges(c("a.bin", "ranges-b.bin"), c(0, 1, 2), size = c(1, 1, 1))), "offset length")
expect_error(fs_read(fs, range_req, mode = "serial"), "complete-object reads")
expect_error(fs_read(fs, range_req, mode = "text"), "complete-object reads")
expect_equal(
  fs_read(
    fs,
    "a.bin",
    read_concurrency = 1,
    chunk_size = 2,
    coalesce_gap = 1
  ),
  bytes
)

again <- fs_write(fs, "a.bin", as.raw(9))
expect_true(is_error_value(again))
expect_equal(error_kind(again), "AlreadyExists")

expect_true(identical(
  fs_replace(
    fs,
    "a.bin",
    as.raw(c(9, 8)),
    write_concurrency = 1,
    chunk_size = 2
  ),
  TRUE
))
expect_equal(fs_read(fs, "a.bin"), as.raw(c(9, 8)))

bytes_handle <- fs_read_bytes(fs, "a.bin")
expect_true(inherits(bytes_handle, "OpendalBytes"))
expect_equal(length(bytes_handle), 2)
expect_equal(as.raw(bytes_handle), as.raw(c(9, 8)))
bytes_slice <- opendal_bytes_slice(bytes_handle, offset = 1, size = 1)
expect_true(inherits(bytes_slice, "OpendalBytes"))
expect_equal(length(bytes_slice), 1)
expect_equal(as.raw(bytes_slice), as.raw(8))
bytes_slice_end <- opendal_bytes_slice(bytes_handle, offset = 0, end = 1)
expect_equal(as.raw(bytes_slice_end), as.raw(9))
bytes_slice_clamped <- opendal_bytes_slice(bytes_handle, offset = 1, size = 99)
expect_equal(as.raw(bytes_slice_clamped), as.raw(8))
bytes_slice_empty <- opendal_bytes_slice(bytes_handle, offset = 99)
expect_equal(as.raw(bytes_slice_empty), raw(0))
expect_error(opendal_bytes_slice(bytes_handle, offset = 0, size = 1, end = 1), "use only one")
expect_error(opendal_bytes_slice(bytes_handle, offset = -1), "non-negative")
expect_true(identical(fs_write(fs, "bytes-create.bin", bytes_handle), TRUE))
expect_equal(fs_read(fs, "bytes-create.bin"), as.raw(c(9, 8)))
expect_true(identical(fs_replace(fs, "bytes-replace.bin", bytes_handle), TRUE))
expect_equal(fs_read(fs, "bytes-replace.bin"), as.raw(c(9, 8)))
append_bytes <- fs_append(fs, "bytes-replace.bin", bytes_handle)
if (!is_error_value(append_bytes)) {
  expect_true(identical(append_bytes, TRUE))
  expect_equal(fs_read(fs, "bytes-replace.bin"), as.raw(c(9, 8, 9, 8)))
}
bytes_aio <- collect_aio(fs_read_bytes_aio(fs, "a.bin"))
expect_true(inherits(bytes_aio, "OpendalBytes"))
expect_equal(as.raw(bytes_aio), as.raw(c(9, 8)))
bytes_range <- fs_read_bytes(fs, "a.bin", offset = 1, size = 1)
expect_equal(as.raw(bytes_range), as.raw(8))
bytes_end <- fs_read_bytes(fs, "a.bin", offset = 0, end = 1)
expect_equal(as.raw(bytes_end), as.raw(9))
bytes_many <- fs_read_bytes(fs, c("a.bin", "bytes-create.bin"), offset = c(0, 0), result = "flat")
expect_equal(length(bytes_many), 2)
expect_true(all(vapply(bytes_many, inherits, logical(1), "OpendalBytes")))
expect_equal(as.raw(bytes_many[[2]]), as.raw(c(9, 8)))
bytes_nested <- fs_read_bytes(fs, c("a.bin", "bytes-create.bin"), offset = c(0, 0), result = "nested")
expect_equal(length(bytes_nested), 2)
expect_equal(length(bytes_nested[[1]]), 1)
expect_true(inherits(bytes_nested[[1]][[1]], "OpendalBytes"))
expect_equal(as.raw(bytes_nested[[1]][[1]]), as.raw(c(9, 8)))
gc_bytes_nested <- tryCatch(
  {
    gctorture(TRUE)
    fs_read_bytes(fs, c("a.bin", "bytes-create.bin"), offset = c(0, 0), result = "nested")
  },
  finally = gctorture(FALSE)
)
expect_equal(as.raw(gc_bytes_nested[[1]][[1]]), as.raw(c(9, 8)))
raw_nested <- fs_read(fs, c("a.bin", "bytes-create.bin"), offset = c(0, 0), result = "nested")
expect_equal(raw_nested[[2]][[1]], as.raw(c(9, 8)))
bytes_many_aio <- collect_aio(fs_read_bytes_aio(fs, c("a.bin", "bytes-create.bin"), offset = c(0, 0), result = "flat"))
expect_equal(length(bytes_many_aio), 2)
expect_true(all(vapply(bytes_many_aio, inherits, logical(1), "OpendalBytes")))
bytes_nested_aio <- collect_aio(fs_read_bytes_aio(fs, c("a.bin", "bytes-create.bin"), offset = c(0, 0), result = "nested"))
expect_equal(as.raw(bytes_nested_aio[[2]][[1]]), as.raw(c(9, 8)))
missing_bytes <- fs_read_bytes(fs, "missing-bytes.bin")
expect_true(is_error_value(missing_bytes))
expect_equal(error_kind(missing_bytes), "NotFound")
missing_bytes_aio <- collect_aio(fs_read_bytes_aio(fs, "missing-bytes.bin"))
expect_true(is_error_value(missing_bytes_aio))
expect_equal(error_kind(missing_bytes_aio), "NotFound")
expect_true(identical(fs_replace(fs, c("bytes-list-a.bin", "bytes-list-b.bin"), list(bytes_handle, bytes_aio)), list(TRUE, TRUE)))
expect_equal(fs_read(fs, "bytes-list-b.bin"), as.raw(c(9, 8)))
expect_true(identical(collect_aio(fs_write_aio(fs, "bytes-aio-create.bin", bytes_handle)), TRUE))
expect_equal(fs_read(fs, "bytes-aio-create.bin"), as.raw(c(9, 8)))
expect_true(identical(collect_aio(fs_replace_aio(fs, "bytes-aio-replace.bin", bytes_handle)), TRUE))
expect_equal(fs_read(fs, "bytes-aio-replace.bin"), as.raw(c(9, 8)))
append_bytes_aio <- collect_aio(fs_append_aio(fs, "bytes-aio-replace.bin", bytes_handle))
if (!is_error_value(append_bytes_aio)) {
  expect_true(identical(append_bytes_aio, TRUE))
  expect_equal(fs_read(fs, "bytes-aio-replace.bin"), as.raw(c(9, 8, 9, 8)))
}
bad_root <- ropendal_temp_root()
bad_fs <- opendal("fs", root = bad_root)
bad_bytes <- bad_fs$.ptr
class(bad_bytes) <- c("Ropendal::OpendalBytes", "OpendalBytes", "savvy_Ropendal__sealed")
expect_error(as.raw(bad_bytes), "invalid OpendalBytes pointer")
expect_error(fs_write(fs, "bad-bytes.bin", bad_bytes), "invalid OpendalBytes pointer")

base_obj <- list(alpha = 1:3, beta = "text")
base_raw <- serialize_raw(base_obj)
expect_true(is.raw(base_raw))
expect_equal(deserialize_raw(base_raw), base_obj)
expect_true(identical(fs_write(fs, "serial-base.rds", base_obj, mode = "serial"), TRUE))
expect_equal(fs_read(fs, "serial-base.rds", mode = "serial"), base_obj)
expect_error(fs_read(fs, "serial-base.rds", size = 1, mode = "serial"), "complete-object")
expect_error(fs_read_aio(fs, "serial-base.rds", size = 1, mode = "serial"), "complete-object")
expect_error(fs_read_aio(fs, "serial-base.rds", end = 1, mode = "serial"), "complete-object")

toy <- structure(list(value = 42L), class = "ropendalToy")
toy_config <- serial_config(
  "ropendalToy",
  sfunc = function(x) serialize(x$value, NULL),
  ufunc = function(x) structure(list(value = unserialize(x)), class = "ropendalToy")
)
expect_true(inherits(toy_config, "ropendalSerialConfig"))
toy_raw <- serialize_raw(toy, toy_config)
expect_equal(deserialize_raw(toy_raw, toy_config), toy)
expect_error(deserialize_raw(toy_raw), "matching serial_config")
opt(fs, "serial") <- toy_config
expect_true(inherits(opt(fs, "serial"), "ropendalSerialConfig"))
expect_true(identical(fs_write(fs, "serial-toy.rds", toy, mode = "serial"), TRUE))
expect_equal(fs_read(fs, "serial-toy.rds", mode = "serial"), toy)
serial_aio <- fs_read_aio(fs, "serial-toy.rds", mode = "serial")
expect_equal(collect_aio(serial_aio), toy)
expect_equal(serial_aio$value, toy)
expect_true(identical(collect_aio(fs_write_aio(fs, "serial-aio.rds", toy, mode = "serial")), TRUE))
expect_equal(fs_read(fs, "serial-aio.rds", mode = "serial"), toy)
expect_true(identical(fs_replace(fs, c("serial-v1.rds", "serial-v2.rds"), list(toy, toy), mode = "serial"), list(TRUE, TRUE)))
serial_many <- fs_read(fs, c("serial-v1.rds", "serial-v2.rds"), offset = c(0, 0), result = "flat", mode = "serial")
expect_equal(serial_many[[1]], toy)
expect_equal(serial_many[[2]], toy)
opt(fs, "serial") <- list()
expect_equal(opt(fs, "serial"), list())
expect_error(fs_read(fs, "serial-toy.rds", mode = "serial"), "matching serial_config")

text_value <- "café π"
expect_true(identical(fs_write(fs, "text-utf8.txt", text_value, mode = "text"), TRUE))
expect_equal(fs_read(fs, "text-utf8.txt", mode = "text"), text_value)
expect_equal(fs_read(fs, "text-utf8.txt", mode = "raw"), charToRaw(enc2utf8(text_value)))
expect_error(fs_read(fs, "text-utf8.txt", size = 1, mode = "text"), "complete-object")
expect_error(fs_write(fs, "text-invalid.txt", c("a", "b"), mode = "text"), "scalar string")
expect_error(fs_write(fs, "text-na.txt", NA_character_, mode = "text"), "non-missing")
expect_true(identical(fs_replace(fs, c("text-a.txt", "text-b.txt"), c("alpha", "beta"), mode = "text"), list(TRUE, TRUE)))
expect_equal(fs_read(fs, c("text-a.txt", "text-b.txt"), offset = c(0, 0), mode = "text", result = "flat"), list("alpha", "beta"))
text_aio <- fs_read_aio(fs, "text-utf8.txt", mode = "text")
expect_equal(collect_aio(text_aio), text_value)
expect_equal(text_aio$value, text_value)
expect_true(identical(collect_aio(fs_write_aio(fs, "text-aio.txt", "async text", mode = "text")), TRUE))
expect_equal(fs_read(fs, "text-aio.txt", mode = "text"), "async text")
latin1_text <- "café"
expect_true(identical(fs_replace(fs, "text-latin1.txt", latin1_text, mode = "text", encoding = "latin1"), TRUE))
expect_equal(fs_read(fs, "text-latin1.txt", mode = "text", encoding = "latin1"), latin1_text)
expect_equal(as.integer(fs_read(fs, "text-latin1.txt")), c(99L, 97L, 102L, 233L))
expect_error(fs_replace(fs, "text-bad-encoding.txt", "€", mode = "text", encoding = "latin1"), "could not be encoded")
expect_error(fs_write(fs, "text-utf16.txt", "A", mode = "text", encoding = "UTF-16LE"), "embedded NUL")
expect_error(fs_read(fs, "missing-text.txt", mode = "text", encoding = "not-an-encoding"), "unsupported text encoding")
expect_error(fs_read_aio(fs, "missing-text.txt", mode = "text", encoding = "not-an-encoding"), "unsupported text encoding")
expect_true(identical(fs_replace(fs, "text-nul.txt", as.raw(c(0x41, 0x00))), TRUE))
expect_error(fs_read(fs, "text-nul.txt", mode = "text"), "NUL bytes")
expect_error(collect_aio(fs_read_aio(fs, "text-nul.txt", mode = "text")), "NUL bytes")

codec_payload <- charToRaw(paste(rep("codec payload", 20), collapse = "|"))
gzip_codec <- codec_config("gzip")
expect_true(inherits(gzip_codec, "ropendalCodecConfig"))
expect_equal(codec_config("gz")$name, "gzip")
expect_error(codec_config("brotli"), "unsupported codec")
expect_error(codec_config("gzip", sfunc = identity), "R-closure codecs")
expect_equal(opt(fs, "codec"), list())
opt(fs, "codec") <- gzip_codec
expect_true(inherits(opt(fs, "codec"), "ropendalCodecConfig"))
expect_true(identical(fs_write(fs, "codec-gzip.bin", codec_payload, mode = "codec"), TRUE))
expect_equal(fs_read(fs, "codec-gzip.bin", mode = "codec"), codec_payload)
expect_equal(fs_read(fs, "codec-gzip.bin", codec = "gzip"), codec_payload)
stored_gzip <- fs_read(fs, "codec-gzip.bin", codec = list())
expect_equal(stored_gzip[1:2], as.raw(c(0x1f, 0x8b)))
expect_error(fs_read(fs, "codec-gzip.bin", offset = 1, codec = "gzip"), "complete-object")
expect_true(identical(fs_replace(fs, "text-codec.txt.gz", text_value, mode = "text", codec = "gzip"), TRUE))
expect_equal(fs_read(fs, "text-codec.txt.gz", mode = "text", codec = "gzip"), text_value)
text_codec_aio <- fs_read_aio(fs, "text-codec.txt.gz", mode = "text", codec = "gzip")
expect_equal(collect_aio(text_codec_aio), text_value)
expect_equal(text_codec_aio$value, text_value)
expect_true(identical(collect_aio(fs_write_aio(fs, "text-codec-aio.txt.gz", text_value, mode = "text", codec = "gzip")), TRUE))
expect_equal(fs_read(fs, "text-codec-aio.txt.gz", mode = "text", codec = "gzip"), text_value)
expect_true(identical(fs_replace(fs, "serial-codec.rds", base_obj, mode = "serial", codec = "gzip"), TRUE))
expect_equal(fs_read(fs, "serial-codec.rds", mode = "serial", codec = "gzip"), base_obj)
expect_true(identical(collect_aio(fs_write_aio(fs, "codec-aio.bin", codec_payload, mode = "codec")), TRUE))
expect_equal(collect_aio(fs_read_aio(fs, "codec-aio.bin", mode = "codec")), codec_payload)
opt(fs, "codec") <- list()
expect_equal(opt(fs, "codec"), list())
expect_error(fs_read(fs, "codec-gzip.bin", mode = "codec"), "requires a codec")
expect_equal(fs_read(fs, "codec-gzip.bin", codec = gzip_codec), codec_payload)
expect_true(identical(fs_replace(fs, "codec-bytes.bin", bytes_handle, codec = "gzip"), TRUE))
expect_equal(fs_read(fs, "codec-bytes.bin", codec = "gzip"), as.raw(c(9, 8)))

stat <- fs_stat(fs, "a.bin")
expect_equal(stat$path, "a.bin")
expect_equal(stat$type, "file")
expect_equal(stat$size, 2)

entries <- fs_ls(fs)
expect_true(any(vapply(entries, function(x) identical(x$path, "a.bin"), logical(1))))
expect_false(any(vapply(entries, function(x) identical(x$path, "/"), logical(1))))

missing <- fs_read(fs, "missing.bin")
expect_true(is_error_value(missing))
expect_equal(error_kind(missing), "NotFound")
expect_false(fs_exists(fs, "missing.bin"))
expect_true(fs_exists(fs, "a.bin"))

expect_true(identical(fs_mkdir(fs, "empty"), TRUE))
expect_true(ls_iter_next(fs_ls_iter(fs, "empty", page_size = 2))$done)

expect_true(identical(fs_mkdir(fs, "dir"), TRUE))
expect_true(identical(fs_copy(fs, "a.bin", "dir/b.bin"), TRUE))
expect_equal(fs_read(fs, "dir/b.bin"), as.raw(c(9, 8)))
expect_true(identical(fs_rename(fs, "dir/b.bin", "dir/c.bin"), TRUE))
expect_equal(fs_read(fs, "dir/c.bin"), as.raw(c(9, 8)))

ls_iter <- fs_ls_iter(fs, "", page_size = 1)
expect_true(inherits(ls_iter, "OpendalLsIter"))
pages <- list()
repeat {
  page <- ls_iter_next(ls_iter)
  if (page$done) break
  expect_true(length(page$entries) > 0)
  expect_true(length(page$entries) <= 1)
  page_paths <- vapply(page$entries, `[[`, "", "path")
  expect_equal(page$cursor, page_paths[[length(page_paths)]])
  pages[[length(pages) + 1L]] <- page
}
ls_iter_paths <- unlist(lapply(pages, function(page) vapply(page$entries, `[[`, "", "path")))
expect_true("a.bin" %in% ls_iter_paths)
expect_true("dir/" %in% ls_iter_paths)
expect_equal(ls_iter_next(ls_iter)$cursor, ls_iter_paths[[length(ls_iter_paths)]])
expect_true(ls_iter_next(ls_iter)$done)

walk_page <- walk_iter_next(fs_walk_iter(fs, "", page_size = 1))
if (!walk_page$done) {
  walk_page_paths <- vapply(walk_page$entries, `[[`, "", "path")
  expect_equal(walk_page$cursor, walk_page_paths[[length(walk_page_paths)]])
}
walk_iter <- fs_walk_iter(fs, "", page_size = 2)
walk_entries <- walk_iter_collect(walk_iter)
walk_paths <- vapply(walk_entries, `[[`, "", "path")
expect_true("dir/c.bin" %in% walk_paths)
expect_true(walk_iter_next(walk_iter)$done)

ls_iter_collect_paths <- vapply(ls_iter_collect(fs_ls_iter(fs)), `[[`, "", "path")
expect_true("a.bin" %in% ls_iter_collect_paths)

limited_entries <- fs_ls(fs, limit = 1)
expect_true(length(limited_entries) <= 1)
expect_equal(fs_ls(fs, limit = 0), list())
after_entries <- fs_ls(fs, start_after = "a.bin")
after_paths <- vapply(after_entries, `[[`, "", "path")
expect_false("a.bin" %in% after_paths)
expect_true(all(after_paths > "a.bin"))
limited_aio_entries <- collect_aio(fs_ls_aio(fs, limit = 2))
expect_true(length(limited_aio_entries) <= 2)
after_aio_paths <- vapply(collect_aio(fs_ls_aio(fs, start_after = "a.bin")), `[[`, "", "path")
expect_false("a.bin" %in% after_aio_paths)
expect_true(all(after_aio_paths > "a.bin"))
limited_iter_entries <- ls_iter_collect(fs_ls_iter(fs, page_size = 1, limit = 2))
expect_true(length(limited_iter_entries) <= 2)
expect_equal(ls_iter_collect(fs_ls_iter(fs, limit = 0)), list())
iter_after_paths <- vapply(
  ls_iter_collect(fs_ls_iter(fs, start_after = "a.bin")),
  `[[`,
  "",
  "path"
)
expect_false("a.bin" %in% iter_after_paths)
expect_true(all(iter_after_paths > "a.bin"))
first_cursor <- pages[[1L]]$cursor
cursor_resume_paths <- vapply(
  ls_iter_collect(fs_ls_iter(fs, start_after = first_cursor)),
  `[[`,
  "",
  "path"
)
expect_true(all(cursor_resume_paths > first_cursor))
prefetch_iter <- fs_ls_iter(fs, page_size = 1, limit = 2, prefetch = 4)
prefetch_page <- ls_iter_next(prefetch_iter)
prefetch_page_paths <- character()
if (!prefetch_page$done) {
  prefetch_page_paths <- vapply(prefetch_page$entries, `[[`, "", "path")
  expect_true(length(prefetch_page_paths) <= 1)
  expect_equal(prefetch_page$cursor, prefetch_page_paths[[length(prefetch_page_paths)]])
}
prefetch_rest <- ls_iter_collect(prefetch_iter)
prefetch_rest_paths <- vapply(prefetch_rest, `[[`, "", "path")
expect_true(length(c(prefetch_page_paths, prefetch_rest_paths)) <= 2)
expect_equal(ls_iter_collect(fs_ls_iter(fs, limit = 0, prefetch = 2)), list())
prefetch_after_paths <- vapply(
  ls_iter_collect(fs_ls_iter(fs, start_after = "a.bin", prefetch = 2)),
  `[[`,
  "",
  "path"
)
expect_true(all(prefetch_after_paths > "a.bin"))
expect_error(fs_ls_iter(fs, prefetch = -1), "prefetch")
expect_error(fs_walk_iter(fs, prefetch = -1), "prefetch")
walk_limited_entries <- walk_iter_collect(fs_walk_iter(fs, page_size = 1, limit = 1))
expect_true(length(walk_limited_entries) <= 1)
walk_prefetch_entries <- walk_iter_collect(fs_walk_iter(fs, page_size = 1, limit = 2, prefetch = 3))
expect_true(length(walk_prefetch_entries) <= 2)
single_thread_root <- tempfile("ropendal-prefetch-single-")
dir.create(single_thread_root)
single_thread_fs <- opendal("fs", root = single_thread_root, runtime = runtime_config(threads = 1))
expect_true(identical(fs_write(single_thread_fs, c("one.txt", "two.txt"), list(charToRaw("1"), charToRaw("2"))), list(TRUE, TRUE)))
single_thread_prefetch <- ls_iter_collect(fs_ls_iter(single_thread_fs, page_size = 1, prefetch = 2))
expect_equal(sort(vapply(single_thread_prefetch, `[[`, "", "path")), c("one.txt", "two.txt"))

expect_true(identical(fs_delete(fs, "dir/c.bin"), TRUE))

expect_error(fs_read(fs, c("a.bin", "a.bin"), offset = 0))
expect_error(fs_read(fs, "a.bin", batch_concurrency = 0), "batch_concurrency")
expect_error(fs_read(fs, "a.bin", offset = numeric()), "offset")
expect_error(fs_read(fs, "a.bin", offset = list(numeric())), "offset")
expect_error(fs_read(fs, "a.bin", offset = numeric(), mode = "serial"), "offset")
expect_error(fs_write(fs, c("x", "y"), as.raw(1)))
expect_error(fs_read(fs, "a.bin", read_concurrency = -1))
expect_error(fs_write(fs, "bad.bin", as.raw(1), write_concurrency = -1))

expect_true(identical(fs_replace(fs, "iter-read.bin", as.raw(1:6)), TRUE))
riter <- fs_read_iter(fs, "iter-read.bin", chunk_size = 2)
expect_true(inherits(riter, "OpendalReadIter"))
expect_equal(fs_tell(riter), 0)
chunk <- read_iter_next(riter)
expect_false(chunk$done)
expect_equal(chunk$data, as.raw(c(1, 2)))
expect_equal(fs_tell(riter), 2)
expect_equal(fs_seek(riter, -1, "current"), 1)
expect_equal(read_iter_next(riter)$data, as.raw(c(2, 3)))
expect_equal(fs_seek(riter, -2, "end"), 4)
expect_equal(read_iter_collect(riter), as.raw(c(5, 6)))
expect_true(read_iter_next(riter)$done)
expect_equal(fs_seek(riter, 0), 0)
expect_equal(read_iter_next(riter)$data, as.raw(c(1, 2)))

riters <- fs_read_iter(fs, c("iter-read.bin", "iter-read.bin"), chunk_size = 3)
expect_equal(length(riters), 2)
expect_equal(read_iter_next(riters[[2]])$data, as.raw(c(1, 2, 3)))

witer <- fs_write_iter(fs, "iter-write.bin", chunk_size = 2)
expect_true(inherits(witer, "OpendalWriteIter"))
expect_equal(fs_tell(witer), 0)
expect_true(identical(write_iter_write(witer, as.raw(c(10, 11))), TRUE))
expect_equal(fs_tell(witer), 2)
expect_true(identical(write_iter_write(witer, as.raw(c(12, 13))), TRUE))
expect_equal(fs_tell(witer), 4)
expect_true(identical(write_iter_close(witer), TRUE))
expect_equal(fs_read(fs, "iter-write.bin"), as.raw(c(10, 11, 12, 13)))
expect_true(identical(write_iter_close(witer), TRUE))
expect_error(fs_seek(witer, 0))

witers <- fs_write_iter(fs, c("iter-write-a.bin", "iter-write-b.bin"), chunk_size = 2)
expect_equal(length(witers), 2)
expect_true(identical(write_iter_write(witers[[1]], as.raw(21)), TRUE))
expect_true(identical(write_iter_close(witers[[1]]), TRUE))
expect_true(identical(write_iter_write(witers[[2]], as.raw(22)), TRUE))
expect_true(identical(write_iter_close(witers[[2]]), TRUE))
expect_equal(fs_read(fs, "iter-write-a.bin"), as.raw(21))
expect_equal(fs_read(fs, "iter-write-b.bin"), as.raw(22))

expect_error(fs_write_iter(fs, "bad-iter.bin", create = TRUE, append = TRUE))

aio <- fs_read_aio(fs, "a.bin")
expect_true(inherits(aio, "OpendalAio"))
expect_true(bindingIsActive("value", aio))
expect_true(bindingIsActive("data", aio))
expect_true(bindingIsActive("result", aio))
expect_true(is.logical(aio$resolved))
expect_true(identical(call_aio(aio), aio))
expect_false(unresolved(aio))
expect_equal(aio$value, as.raw(c(9, 8)))
expect_equal(aio$data, as.raw(c(9, 8)))
expect_true(is.null(aio$error))
expect_equal(aio$state, "resolved")
expect_true(unresolved(unresolved()))
direct_aio <- fs$read_aio("a.bin")
expect_true(bindingIsActive("value", direct_aio))
expect_equal(collect_aio(direct_aio), as.raw(c(9, 8)))

expect_equal(collect_aio_(list(fs_read_aio(fs, "a.bin")))[[1]], as.raw(c(9, 8)))
call_many <- list(fs_read_aio(fs, "a.bin"), fs_stat_aio(fs, "a.bin"))
expect_true(identical(call_aio_(call_many), call_many))
expect_equal(call_many[[1]]$value, as.raw(c(9, 8)))

monitor_cv <- cv()
expect_equal(cv_value(monitor_cv), 0L)
expect_false(cv_until(monitor_cv, 1))
cv_signal(monitor_cv)
expect_equal(cv_value(monitor_cv), 1L)
expect_true(cv_until(monitor_cv, 1))
cv_reset(monitor_cv)
expect_equal(cv_value(monitor_cv), 0L)
monitor <- aio_monitor(list(read = fs_read_aio(fs, "a.bin")), cv = monitor_cv)
expect_true(cv_until(monitor_cv, 100))
events <- read_monitor(monitor)
expect_equal(nrow(events), 1L)
expect_equal(events$name, "read")
expect_equal(events$event, "ready")
expect_equal(nrow(read_monitor(monitor)), 0L)
race <- race_aio(list(read = fs_read_aio(fs, "a.bin")), timeout = 100)
expect_equal(race$index, 1L)
expect_equal(race$name, "read")
expect_equal(race$event, "ready")
expect_true(inherits(race$aio, "OpendalAio"))

write_aio <- fs_write_aio(fs, "aio-write.bin", as.raw(c(31, 32)))
expect_true(bindingIsActive("result", write_aio))
expect_true(identical(collect_aio(write_aio), TRUE))
expect_true(identical(write_aio$result, TRUE))
expect_equal(fs_read(fs, "aio-write.bin"), as.raw(c(31, 32)))
expect_true(identical(collect_aio(fs_replace_aio(fs, "aio-write.bin", as.raw(33))), TRUE))
expect_equal(collect_aio(fs_read_aio(fs, "aio-write.bin")), as.raw(33))

err_aio <- fs_read_aio(fs, "missing-aio-error.bin")
err_value <- collect_aio(err_aio)
expect_true(is_error_value(err_value))
expect_true(is_error_value(err_aio$value))
expect_true(is_error_value(err_aio$error))
expect_equal(err_aio$state, "error")

stat_aio <- collect_aio(fs_stat_aio(fs, "aio-write.bin"))
expect_equal(stat_aio$path, "aio-write.bin")
expect_equal(stat_aio$size, 1)
expect_equal(fs_stats(fs, "aio-write.bin")$size, 1)
expect_equal(collect_aio(fs_stats_aio(fs, "aio-write.bin"))$size, 1)
expect_true(collect_aio(fs_exists_aio(fs, "aio-write.bin")))
expect_false(collect_aio(fs_exists_aio(fs, "nope.bin")))

entries_aio <- collect_aio(fs_ls_aio(fs))
expect_true(any(vapply(entries_aio, function(x) identical(x$path, "aio-write.bin"), logical(1))))

stats_aio <- collect_aio(fs_stat_aio(fs, c("a.bin", "aio-write.bin"), batch_concurrency = 2))
expect_equal(length(stats_aio), 2)
expect_equal(stats_aio[[1]]$path, "a.bin")
expect_equal(stats_aio[[2]]$path, "aio-write.bin")
exists_aio <- collect_aio(fs_exists_aio(fs, c("a.bin", "missing-aio.bin"), batch_concurrency = 2))
expect_equal(exists_aio, list(TRUE, FALSE))

expect_true(identical(collect_aio(fs_mkdir_aio(fs, "aio-dir")), TRUE))
expect_true(identical(collect_aio(fs_copy_aio(fs, "aio-write.bin", "aio-dir/copied.bin")), TRUE))
expect_equal(fs_read(fs, "aio-dir/copied.bin"), as.raw(33))
expect_true(identical(collect_aio(fs_rename_aio(fs, "aio-dir/copied.bin", "aio-dir/renamed.bin")), TRUE))
expect_equal(fs_read(fs, "aio-dir/renamed.bin"), as.raw(33))
expect_true(identical(collect_aio(fs_delete_aio(fs, "aio-dir/renamed.bin")), TRUE))
