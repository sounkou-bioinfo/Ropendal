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

again <- fs_write(fs, "a.bin", as.raw(9))
expect_true(is_error_value(again))
expect_equal(error_kind(again), "AlreadyExists")

expect_true(identical(fs_replace(fs, "a.bin", as.raw(c(9, 8))), TRUE))
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

expect_true(identical(fs_mkdir(fs, "dir"), TRUE))
expect_true(identical(fs_copy(fs, "a.bin", "dir/b.bin"), TRUE))
expect_equal(fs_read(fs, "dir/b.bin"), as.raw(c(9, 8)))
expect_true(identical(fs_rename(fs, "dir/b.bin", "dir/c.bin"), TRUE))
expect_equal(fs_read(fs, "dir/c.bin"), as.raw(c(9, 8)))
expect_true(identical(fs_delete(fs, "dir/c.bin"), TRUE))

expect_error(fs_read(fs, c("a.bin", "a.bin"), offset = 0))
expect_error(fs_write(fs, c("x", "y"), as.raw(1)))

aio <- fs_read_aio(fs, "a.bin")
expect_true(inherits(aio, "OpendalAio"))
expect_equal(call_aio(aio), as.raw(c(9, 8)))
