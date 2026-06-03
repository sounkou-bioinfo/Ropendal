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
http_caps <- fs_capabilities(fs)
expect_true(inherits(http_caps, "opendalCapabilityValue"))
expect_true(http_caps$operations$read$supported)
expect_true(http_caps$operations$stat$supported)
expect_false(http_caps$operations$ls$supported)
expect_equal(http_caps$operations$ls$implementation, "unsupported")
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

async_stat <- collect_aio(fs_stat_aio(fs, "data.bin"))
expect_false(is_error_value(async_stat))
expect_equal(async_stat$type, "file")
expect_equal(async_stat$size, length(bytes))
expect_true(collect_aio(fs_exists_aio(fs, "data.bin")))
expect_false(collect_aio(fs_exists_aio(fs, "missing.bin")))
async_listing <- collect_aio(fs_ls_aio(fs))
expect_true(is_error_value(async_listing))
expect_equal(error_kind(async_listing), "Unsupported")

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

cancel_fixture <- Ropendal:::OpendalHttpFixture$start(root, delay_ms = 1000)
tryCatch(
  {
    cancel_fs <- opendal("http", endpoint = cancel_fixture$endpoint(), root = "/")
    cancel_aio <- fs_read_aio(cancel_fs, "data.bin")
    expect_equal(cancel_aio$state, "pending")
    expect_true(unresolved(cancel_aio))
    expect_true(identical(stop_aio(cancel_aio), TRUE))
    expect_equal(cancel_aio$state, "cancelled")
    expect_true(cancel_aio$resolved)
    expect_false(unresolved(cancel_aio))
    cancel_error <- cancel_aio$error
    expect_true(is_error_value(cancel_error))
    expect_equal(error_kind(cancel_error), "Cancelled")
    cancel_value <- collect_aio(cancel_aio)
    expect_true(is_error_value(cancel_value))
    expect_equal(error_kind(cancel_value), "Cancelled")
  },
  finally = expect_true(identical(cancel_fixture$stop(), TRUE))
)

timeout_fixture <- Ropendal:::OpendalHttpFixture$start(root, delay_ms = 1000)
tryCatch(
  {
    read_timeout_fs <- opendal(
      "http",
      endpoint = timeout_fixture$endpoint(),
      root = "/",
      layers = list(layer_timeout(io_timeout = 0.01))
    )
    read_timeout <- fs_read(read_timeout_fs, "data.bin")
    expect_true(is_error_value(read_timeout))
    expect_true(grepl("timeout", error_message(read_timeout), ignore.case = TRUE))
    stat_timeout_fs <- opendal(
      "http",
      endpoint = timeout_fixture$endpoint(),
      root = "/",
      layers = list(layer_timeout(request_timeout = 0.01))
    )
    stat_timeout <- fs_stat(stat_timeout_fs, "data.bin")
    expect_true(is_error_value(stat_timeout))
    expect_true(grepl("timeout", error_message(stat_timeout), ignore.case = TRUE))
  },
  finally = expect_true(identical(timeout_fixture$stop(), TRUE))
)

expect_true(identical(header_fixture$stop(), TRUE))
expect_true(identical(fixture$stop(), TRUE))
