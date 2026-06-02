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
listing_page <- ls_iter_next(fs_ls_iter(fs))
expect_true(is_error_value(listing_page))
expect_equal(error_kind(listing_page), "Unsupported")
walk_entries <- walk_iter_collect(fs_walk_iter(fs))
expect_true(is_error_value(walk_entries))
expect_equal(error_kind(walk_entries), "Unsupported")

header_fixture <- Ropendal:::OpendalHttpFixture$start(
  root,
  list(Authorization = "Bearer ropendal-test", `X-Ropendal-Test` = "fixture")
)
header_fs <- opendal(
  "http",
  endpoint = header_fixture$endpoint(),
  root = "/",
  headers = list(Authorization = "Bearer ropendal-test", `X-Ropendal-Test` = "fixture")
)
expect_equal(fs_read(header_fs, "data.bin"), bytes)
expect_equal(fs_read(header_fs, "data.bin", offset = 1, size = 2), as.raw(c(2, 3)))
expect_equal(fs_stat(header_fs, "data.bin")$size, length(bytes))
header_uri_fs <- opendal_uri(
  header_fixture$endpoint(),
  headers = list(Authorization = "Bearer ropendal-test", `X-Ropendal-Test` = "fixture")
)
expect_equal(fs_read(header_uri_fs, "data.bin"), bytes)
missing_header_fs <- opendal("http", endpoint = header_fixture$endpoint(), root = "/")
missing_header <- fs_read(missing_header_fs, "data.bin")
expect_true(is_error_value(missing_header))

pending_fixture <- Ropendal:::OpendalHttpFixture$start(root, delay_ms = 1000)
tryCatch(
  {
    pending_fs <- opendal("http", endpoint = pending_fixture$endpoint(), root = "/")
    pending_aio <- fs_read_aio(pending_fs, "data.bin")
    expect_equal(poll_aio(pending_aio), "pending")
    expect_equal(pending_aio$state, "pending")
    expect_false(pending_aio$resolved)
    expect_true(unresolved(pending_aio))
    pending_value <- pending_aio$value
    expect_true(unresolved(pending_value))
    expect_true(is.null(pending_aio$error))
    expect_true(identical(call_aio(pending_aio), pending_aio))
    expect_equal(pending_aio$state, "resolved")
    expect_true(pending_aio$resolved)
    expect_false(unresolved(pending_aio))
    expect_equal(pending_aio$value, bytes)
  },
  finally = expect_true(identical(pending_fixture$stop(), TRUE))
)

expect_true(identical(header_fixture$stop(), TRUE))
expect_true(identical(fixture$stop(), TRUE))
