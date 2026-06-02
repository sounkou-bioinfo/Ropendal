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
