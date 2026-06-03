library(Ropendal)

header <- system.file("include", "ropendal.h", package = "Ropendal")
expect_true(nzchar(header) && file.exists(header))

header_text <- paste(readLines(header, warn = FALSE), collapse = "\n")

# The installed header is the public native contract. Keep this test focused on
# consumer-visible primitives rather than package metadata boilerplate.
expect_match(header_text, "ropendal_api_version")
expect_match(header_text, "ropendal_fs_open")
expect_match(header_text, "ropendal_fs_from_uri")
expect_false(grepl("#include <R", header_text, fixed = TRUE))
expect_false(grepl("SEXP", header_text, fixed = TRUE))
expect_match(header_text, "ropendal_read_aio")
expect_match(header_text, "ropendal_read_into_aio")
expect_match(header_text, "ropendal_readv_into_aio")
expect_match(header_text, "ropendal_aio_poll")
expect_match(header_text, "ropendal_aio_wait")
expect_match(header_text, "ropendal_aio_cancel")
expect_match(header_text, "ropendal_replace_aio")
expect_match(header_text, "ropendal_append_aio")
expect_match(header_text, "ropendal_codec_encode")
expect_match(header_text, "ropendal_bytes_release")
expect_match(header_text, "ropendal_readv_options")
