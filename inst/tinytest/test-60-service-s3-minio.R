library(Ropendal)

helper <- system.file("tinytest", "helper-ropendal.R", package = "Ropendal")
source(helper, local = TRUE)

if (!ropendal_service_tests_enabled("s3_minio")) exit_file("set ROPENDAL_TEST_NETWORK=true and ROPENDAL_TEST_S3_MINIO=true to run")

required <- c(
  "ROPENDAL_S3_MINIO_ENDPOINT",
  "ROPENDAL_S3_MINIO_BUCKET",
  "ROPENDAL_S3_MINIO_REGION",
  "ROPENDAL_S3_MINIO_ACCESS_KEY_ID",
  "ROPENDAL_S3_MINIO_SECRET_ACCESS_KEY",
  "ROPENDAL_S3_MINIO_ROOT"
)
missing <- ropendal_missing_env(required)
if (length(missing)) exit_file(paste("missing env vars:", paste(missing, collapse = ", ")))

auth <- credentials_s3(
  access_key_id = Sys.getenv("ROPENDAL_S3_MINIO_ACCESS_KEY_ID"),
  secret_access_key = Sys.getenv("ROPENDAL_S3_MINIO_SECRET_ACCESS_KEY"),
  region = Sys.getenv("ROPENDAL_S3_MINIO_REGION"),
  source = "minio-test"
)

fs <- opendal(
  "s3",
  endpoint = Sys.getenv("ROPENDAL_S3_MINIO_ENDPOINT"),
  bucket = Sys.getenv("ROPENDAL_S3_MINIO_BUCKET"),
  root = Sys.getenv("ROPENDAL_S3_MINIO_ROOT"),
  disable_config_load = TRUE,
  disable_ec2_metadata = TRUE,
  auth = auth
)

expect_equal(fs_info(fs)$scheme, "s3")
expect_equal(fs_info(fs)$name, Sys.getenv("ROPENDAL_S3_MINIO_BUCKET"))
minio_caps <- fs_capabilities(fs)
expect_true(inherits(minio_caps, "opendalCapabilityValue"))
expect_true(minio_caps$operations$read$supported)
expect_true(minio_caps$operations$write$supported)
expect_true(minio_caps$operations$delete$supported)
expect_true(minio_caps$operations$copy$supported)
expect_true(minio_caps$operations$ls$supported)

bytes <- as.raw(c(1, 2, 3, 4, 5, 6))
expect_true(identical(fs_write(fs, "a.bin", bytes), TRUE))
expect_equal(fs_read(fs, "a.bin"), bytes)
expect_equal(fs_read(fs, "a.bin", offset = 2, size = 3), as.raw(c(3, 4, 5)))

again <- fs_write(fs, "a.bin", as.raw(9))
expect_true(is_error_value(again))
expect_equal(error_kind(again), "AlreadyExists")

stat <- fs_stat(fs, "a.bin")
expect_false(is_error_value(stat))
expect_equal(stat$type, "file")
expect_equal(stat$size, length(bytes))

expect_true(identical(fs_replace(fs, "a.bin", as.raw(c(9, 8))), TRUE))
expect_equal(fs_read(fs, "a.bin"), as.raw(c(9, 8)))

expect_true(identical(fs_copy(fs, "a.bin", "copy.bin"), TRUE))
expect_equal(fs_read(fs, "copy.bin"), as.raw(c(9, 8)))

# S3-compatible stores generally support copy and delete but not atomic rename.
# Ropendal should surface that as an error value rather than silently emulating
# weaker semantics.
renamed <- fs_rename(fs, "copy.bin", "renamed.bin")
if (is_error_value(renamed)) {
  expect_equal(error_kind(renamed), "Unsupported")
  expect_equal(fs_read(fs, "copy.bin"), as.raw(c(9, 8)))
  cleanup_path <- "copy.bin"
} else {
  expect_true(identical(renamed, TRUE))
  expect_equal(fs_read(fs, "renamed.bin"), as.raw(c(9, 8)))
  cleanup_path <- "renamed.bin"
}

listing <- fs_ls(fs)
expect_false(is_error_value(listing))
paths <- vapply(listing, `[[`, character(1), "path")
expect_true("a.bin" %in% paths)
expect_true(cleanup_path %in% paths)

expect_false(fs_exists(fs, "missing.bin"))
expect_true(fs_exists(fs, "a.bin"))

async_bytes <- as.raw(c(10, 11, 12, 13))
expect_true(identical(collect_aio(fs_write_aio(fs, "async.bin", async_bytes)), TRUE))
expect_equal(collect_aio(fs_read_aio(fs, "async.bin", offset = 1, size = 2)), as.raw(c(11, 12)))
async_stat <- collect_aio(fs_stat_aio(fs, "async.bin"))
expect_false(is_error_value(async_stat))
expect_equal(async_stat$type, "file")
expect_equal(async_stat$size, length(async_bytes))
expect_true(collect_aio(fs_exists_aio(fs, "async.bin")))
expect_false(collect_aio(fs_exists_aio(fs, "missing-async.bin")))
existing_async <- collect_aio(fs_write_aio(fs, "async.bin", as.raw(1)))
expect_true(is_error_value(existing_async))
expect_equal(error_kind(existing_async), "AlreadyExists")
expect_true(identical(collect_aio(fs_replace_aio(fs, "async.bin", as.raw(c(20, 21)))), TRUE))
expect_equal(collect_aio(fs_read_aio(fs, "async.bin")), as.raw(c(20, 21)))
expect_true(identical(collect_aio(fs_copy_aio(fs, "async.bin", "async-copy.bin")), TRUE))
expect_equal(fs_read(fs, "async-copy.bin"), as.raw(c(20, 21)))
async_renamed <- collect_aio(fs_rename_aio(fs, "async-copy.bin", "async-renamed.bin"))
if (is_error_value(async_renamed)) {
  expect_equal(error_kind(async_renamed), "Unsupported")
  expect_equal(fs_read(fs, "async-copy.bin"), as.raw(c(20, 21)))
  async_cleanup_path <- "async-copy.bin"
} else {
  expect_true(identical(async_renamed, TRUE))
  expect_equal(fs_read(fs, "async-renamed.bin"), as.raw(c(20, 21)))
  async_cleanup_path <- "async-renamed.bin"
}
async_listing <- collect_aio(fs_ls_aio(fs))
expect_false(is_error_value(async_listing))
async_paths <- vapply(async_listing, `[[`, character(1), "path")
expect_true("async.bin" %in% async_paths)
expect_true(async_cleanup_path %in% async_paths)
expect_true(identical(collect_aio(fs_delete_aio(fs, async_cleanup_path)), TRUE))
expect_true(identical(collect_aio(fs_delete_aio(fs, "async.bin")), TRUE))
expect_false(collect_aio(fs_exists_aio(fs, async_cleanup_path)))
expect_false(collect_aio(fs_exists_aio(fs, "async.bin")))

expect_true(identical(fs_delete(fs, cleanup_path), TRUE))
expect_true(identical(fs_delete(fs, "a.bin"), TRUE))
expect_false(fs_exists(fs, cleanup_path))
expect_false(fs_exists(fs, "a.bin"))
