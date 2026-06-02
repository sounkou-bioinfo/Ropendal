library(Ropendal)

helper <- system.file("tinytest", "helper-ropendal.R", package = "Ropendal")
source(helper, local = TRUE)

root <- ropendal_temp_root()
fs <- opendal("fs", root = root)

expect_true(inherits(fs, "OpendalFs"))
expect_equal(fs_info(fs)$scheme, "fs")
expect_equal(fs_normalize_path(fs, "/a//b/../c"), "a/c")
expect_error(fs_normalize_path(fs, "../escape"))

bytes <- as.raw(c(1, 2, 3, 4))
expect_true(identical(fs_write(fs, "a.bin", bytes), TRUE))
expect_equal(fs_read(fs, "a.bin"), bytes)
expect_equal(fs_read(fs, "a.bin", offset = 1, size = 2), as.raw(c(2, 3)))
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
  pages[[length(pages) + 1L]] <- page
}
ls_iter_paths <- unlist(lapply(pages, function(page) vapply(page$entries, `[[`, "", "path")))
expect_true("a.bin" %in% ls_iter_paths)
expect_true("dir/" %in% ls_iter_paths)
expect_true(ls_iter_next(ls_iter)$done)

walk_iter <- fs_walk_iter(fs, "", page_size = 2)
walk_entries <- walk_iter_collect(walk_iter)
walk_paths <- vapply(walk_entries, `[[`, "", "path")
expect_true("dir/c.bin" %in% walk_paths)
expect_true(walk_iter_next(walk_iter)$done)

ls_iter_collect_paths <- vapply(ls_iter_collect(fs_ls_iter(fs)), `[[`, "", "path")
expect_true("a.bin" %in% ls_iter_collect_paths)

expect_true(identical(fs_delete(fs, "dir/c.bin"), TRUE))

expect_error(fs_read(fs, c("a.bin", "a.bin"), offset = 0))
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
