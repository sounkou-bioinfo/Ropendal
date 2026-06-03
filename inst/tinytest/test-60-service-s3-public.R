library(Ropendal)

helper <- system.file("tinytest", "helper-ropendal.R", package = "Ropendal")
source(helper, local = TRUE)

if (!ropendal_service_tests_enabled("s3")) exit_file("set ROPENDAL_TEST_NETWORK=true and ROPENDAL_TEST_S3=true to run")

endpoint <- Sys.getenv("ROPENDAL_S3_PUBLIC_ENDPOINT", unset = "https://uk1s3.embassy.ebi.ac.uk")
bucket <- Sys.getenv("ROPENDAL_S3_PUBLIC_BUCKET", unset = "idr")
root <- Sys.getenv("ROPENDAL_S3_PUBLIC_ROOT", unset = "/zarr/v0.4/idr0062A/6001240.zarr")
region <- Sys.getenv("ROPENDAL_S3_PUBLIC_REGION", unset = "us-east-1")
object <- Sys.getenv("ROPENDAL_S3_PUBLIC_FILE", unset = "0/.zarray")
range_object <- Sys.getenv("ROPENDAL_S3_PUBLIC_RANGE_FILE", unset = "0/0/0/0/0")
list_path <- Sys.getenv("ROPENDAL_S3_PUBLIC_LIST_PATH", unset = "0/")

fs <- opendal(
  "s3",
  endpoint = endpoint,
  bucket = bucket,
  root = root,
  region = region,
  skip_signature = TRUE,
  disable_config_load = TRUE
)

info <- fs_info(fs)
expect_equal(info$scheme, "s3")
expect_equal(info$name, bucket)
public_s3_caps <- fs_capabilities(fs)
expect_true(inherits(public_s3_caps, "opendalCapabilityValue"))
expect_true(public_s3_caps$operations$read$supported)
expect_true(public_s3_caps$operations$stat$supported)
expect_true(public_s3_caps$operations$ls$supported)

zarray <- rawToChar(fs_read(fs, object))
expect_true(grepl('"dtype": "<u2"', zarray, fixed = TRUE))

stat <- fs_stat(fs, object)
expect_false(is_error_value(stat))
expect_equal(stat$path, object)
expect_equal(stat$type, "file")
expect_equal(stat$size, 417)

chunk_head <- fs_read(fs, range_object, offset = 0, size = 16)
expect_true(is.raw(chunk_head))
expect_equal(length(chunk_head), 16L)

listing <- fs_ls(fs)
expect_false(is_error_value(listing))
paths <- vapply(listing, `[[`, character(1), "path")
expect_true(list_path %in% paths)

async_chunk_head <- collect_aio(fs_read_aio(fs, range_object, offset = 0, size = 16))
expect_true(is.raw(async_chunk_head))
expect_equal(length(async_chunk_head), 16L)
async_stat <- collect_aio(fs_stat_aio(fs, object))
expect_false(is_error_value(async_stat))
expect_equal(async_stat$path, object)
expect_equal(async_stat$type, "file")
expect_equal(async_stat$size, 417)
expect_true(collect_aio(fs_exists_aio(fs, object)))
expect_false(collect_aio(fs_exists_aio(fs, "missing-ropendal-public-test-object")))
async_listing <- collect_aio(fs_ls_aio(fs))
expect_false(is_error_value(async_listing))
async_paths <- vapply(async_listing, `[[`, character(1), "path")
expect_true(list_path %in% async_paths)
