library(Ropendal)

helper <- system.file("tinytest", "helper-ropendal.R", package = "Ropendal")
source(helper, local = TRUE)

root <- ropendal_temp_root()
fs <- opendal("fs", root = root)
store <- byte_store(fs, "array.zarr")

expect_true(inherits(store, "ropendalByteStore"))
expect_error(byte_store(list()), "OpendalFs")
expect_error(byte_store(fs, c("a", "b")), "prefix")

expect_true(identical(store_write(store, "zarr.json", charToRaw("{}")), TRUE))
json_bytes <- store_read(store, "zarr.json")
expect_true(inherits(json_bytes, "OpendalBytes"))
expect_equal(as.raw(json_bytes), charToRaw("{}"))
expect_equal(store_read(store, "zarr.json", mode = "raw"), charToRaw("{}"))
expect_equal(fs_read(fs, "array.zarr/zarr.json"), charToRaw("{}"))

again <- store_write(store, "zarr.json", charToRaw("[]"))
expect_true(is_error_value(again))
expect_equal(error_kind(again), "AlreadyExists")
expect_true(identical(store_replace(store, "zarr.json", charToRaw("[]")), TRUE))
expect_equal(as.raw(store_read(store, "zarr.json")), charToRaw("[]"))
expect_equal(collect_aio(store_read_aio(store, "zarr.json", mode = "raw")), charToRaw("[]"))

expect_true(identical(
  store_write(
    store,
    c("c/0/0", "c/0/1"),
    list(as.raw(c(1, 2, 3, 4)), as.raw(c(5, 6, 7, 8))),
    batch_concurrency = 2
  ),
  list(TRUE, TRUE)
))
expect_equal(
  lapply(store_read(store, c("c/0/0", "c/0/1"), batch_concurrency = 2), as.raw),
  list(as.raw(c(1, 2, 3, 4)), as.raw(c(5, 6, 7, 8)))
)
expect_equal(store_read(store, c("c/0/0", "c/0/1"), mode = "raw", batch_concurrency = 2), list(as.raw(c(1, 2, 3, 4)), as.raw(c(5, 6, 7, 8))))
expect_equal(as.raw(store_read(store, "c/0/0", offset = 1, size = 2)), as.raw(c(2, 3)))
chunk_bytes <- store_read(store, "c/0/1")
expect_true(inherits(chunk_bytes, "OpendalBytes"))
expect_equal(as.raw(chunk_bytes), as.raw(c(5, 6, 7, 8)))
expect_equal(as.raw(collect_aio(store_read_aio(store, "c/0/1"))), as.raw(c(5, 6, 7, 8)))
expect_equal(collect_aio(store_read_aio(store, "c/0/1", mode = "raw")), as.raw(c(5, 6, 7, 8)))

exists_many <- store_exists(store, c("zarr.json", "missing"), batch_concurrency = 2)
expect_equal(unlist(exists_many, use.names = FALSE), c(TRUE, FALSE))

entries <- store_list(store, recursive = TRUE)
paths <- sort(vapply(entries, `[[`, character(1), "path"))
expect_true("zarr.json" %in% paths)
expect_true("c/0/0" %in% paths)
expect_false(any(paths == "array.zarr/"))
expect_false(any(startsWith(paths, "array.zarr/")))

c_entries <- store_list(store, "c/", recursive = TRUE)
c_paths <- sort(vapply(c_entries, `[[`, character(1), "path"))
expect_true("c/0/0" %in% c_paths)
expect_true("c/0/1" %in% c_paths)
start_entries <- store_list(store, recursive = TRUE, start_after = "c/0/0")
start_paths <- vapply(start_entries, `[[`, character(1), "path")
expect_false("c/0/0" %in% start_paths)

expect_true(identical(collect_aio(store_write_aio(store, "c/aio", as.raw(42))), TRUE))
expect_equal(as.raw(collect_aio(store_read_aio(store, "c/aio"))), as.raw(42))
expect_true(identical(collect_aio(store_replace_aio(store, "c/aio", as.raw(43))), TRUE))
expect_equal(collect_aio(store_exists_aio(store, "c/aio")), TRUE)
aio_entries <- collect_aio(store_list_aio(store, "c/", recursive = TRUE))
expect_true("c/aio" %in% vapply(aio_entries, `[[`, character(1), "path"))
expect_true(identical(collect_aio(store_delete_aio(store, "c/aio")), TRUE))
expect_false(store_exists(store, "c/aio"))

expect_true(identical(store_delete(store, "c/0/0"), TRUE))
expect_false(store_exists(store, "c/0/0"))
expect_true(identical(store_delete(store, "c/", recursive = TRUE), TRUE))
expect_false(store_exists(store, "c/0/1"))

expect_true(identical(fs_write(fs, "outside.bin", as.raw(99)), TRUE))
expect_error(store_read(store, "../outside.bin"), "escape")
expect_error(store_write(store, "dir/", as.raw(1)), "object key")
expect_error(store_read(store, ""), "empty")
expect_equal(as.raw(store_read(store, "sub/../zarr.json")), charToRaw("[]"))

root_store <- byte_store(fs)
expect_true(identical(store_write(root_store, "root-key.bin", as.raw(11)), TRUE))
expect_equal(as.raw(store_read(root_store, "root-key.bin")), as.raw(11))
