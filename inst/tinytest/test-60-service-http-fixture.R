library(Ropendal)

helper <- system.file("tinytest", "helper-ropendal.R", package = "Ropendal")
source(helper, local = TRUE)

if (!ropendal_service_tests_enabled("http")) exit_file("set ROPENDAL_TEST_NETWORK=true and ROPENDAL_TEST_HTTP=true to run")

root <- ropendal_temp_root("ropendal-http-")
bytes <- as.raw(c(1, 2, 3, 4, 5, 6))
writeBin(bytes, file.path(root, "data.bin"))
dir.create(file.path(root, "dir"))
writeBin(as.raw(c(9, 8)), file.path(root, "dir", "nested.bin"))

fixture <- Ropendal:::OpendalHttpFixture$start(root)

fs <- opendal("http", endpoint = fixture$endpoint(), root = "/")
expect_equal(fs_read(fs, "data.bin"), bytes)
expect_equal(fs_read(fs, "data.bin", offset = 2, size = 3), as.raw(c(3, 4, 5)))

stat <- fs_stat(fs, "data.bin")
expect_equal(stat$type, "file")
expect_equal(stat$size, length(bytes))

dir_index <- rawToChar(fs_read(fs, "dir/"))
expect_true(grepl("nested.bin", dir_index, fixed = TRUE))

# The fixture can serve directory indexes, but OpenDAL's generic HTTP service
# exposes read/stat, not a lister. A Ropendal adapter can add listing later by
# parsing an explicit index format.
listing <- fs_ls(fs)
expect_true(is_error_value(listing))
expect_equal(error_kind(listing), "Unsupported")

expect_true(identical(fixture$stop(), TRUE))
