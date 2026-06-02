library(Ropendal)

helper <- system.file("tinytest", "helper-ropendal.R", package = "Ropendal")
source(helper, local = TRUE)
if (!ropendal_ci_tests_enabled()) exit_file("native API contract lint; set ROPENDAL_TEST_CI=true to run")

header <- system.file("include", "ropendal.h", package = "Ropendal")
include_dir <- dirname(header)
header_text <- paste(readLines(header, warn = FALSE), collapse = "\n")

# CI-only because this is static API lint rather than user-level package behavior.
# It should not run in CRAN checks by default.
public_structs <- gregexpr("typedef struct ropendal_[A-Za-z0-9_]+ \\{", header_text)[[1L]]
expect_true(length(public_structs) >= 1L && public_structs[[1L]] > 0L)
expect_match(header_text, "size_t struct_size;")

# Pure C contract: native consumers should not need R headers or SEXP.
expect_false(grepl("#include <R", header_text, fixed = TRUE))
expect_false(grepl("SEXP", header_text, fixed = TRUE))
expect_match(header_text, "ropendal_fs_open")
expect_match(header_text, "ropendal_fs_from_uri")
expect_match(header_text, "ropendal_replace_aio")
expect_match(header_text, "ropendal_append_aio")
expect_match(header_text, "ropendal_exists_aio")
expect_match(header_text, "ropendal_aio_result_bool")

# Notification/monitor primitives inspired by nanonext condition variables.
expect_match(header_text, "ropendal_cv_alloc")
expect_match(header_text, "ropendal_cv_wait")
expect_match(header_text, "ropendal_aio_notify")
expect_match(header_text, "ropendal_monitor_create")
expect_match(header_text, "ropendal_monitor_read")

# Header compile check. Compile only; do not link against implementation symbols.
cc_candidates <- c(Sys.getenv("CC", unset = ""), Sys.which(c("cc", "gcc", "clang")))
cc_candidates <- cc_candidates[nzchar(cc_candidates)]
expect_true(length(cc_candidates) > 0L)

if (length(cc_candidates) > 0L) {
  cc <- cc_candidates[[1L]]
  src <- tempfile("ropendal-header-compile-", fileext = ".c")
  obj <- tempfile("ropendal-header-compile-", fileext = ".o")
  writeLines(c(
    '#include "ropendal.h"',
    'void ropendal_header_compile(void) {',
    '  ropendal_kv_t kv = {0};',
    '  ropendal_read_options_t read_opts = {0};',
    '  ropendal_read_request_t read_req = {0};',
    '  ropendal_read_into_request_t read_into_req = {0};',
    '  ropendal_readv_options_t readv_opts = {0};',
    '  ropendal_write_options_t write_opts = {0};',
    '  ropendal_ls_options_t ls_opts = {0};',
    '  ropendal_delete_options_t delete_opts = {0};',
    '  ropendal_entry_t entry = {0};',
    '  ropendal_monitor_event_t event = {0};',
    '  kv.struct_size = sizeof kv;',
    '  read_opts.struct_size = sizeof read_opts;',
    '  read_req.struct_size = sizeof read_req;',
    '  read_into_req.struct_size = sizeof read_into_req;',
    '  readv_opts.struct_size = sizeof readv_opts;',
    '  write_opts.struct_size = sizeof write_opts;',
    '  ls_opts.struct_size = sizeof ls_opts;',
    '  delete_opts.struct_size = sizeof delete_opts;',
    '  entry.struct_size = sizeof entry;',
    '  event.struct_size = sizeof event;',
    '  (void)kv; (void)read_opts; (void)read_req; (void)read_into_req;',
    '  (void)readv_opts; (void)write_opts; (void)ls_opts; (void)delete_opts;',
    '  (void)entry; (void)event;',
    '}'
  ), src)
  out <- system2(cc, c("-std=c99", "-Wall", "-Wextra", "-Werror", "-I", include_dir,
                       "-c", src, "-o", obj), stdout = TRUE, stderr = TRUE)
  status <- attr(out, "status")
  if (is.null(status)) status <- 0L
  if (status != 0L) writeLines(out)
  expect_equal(status, 0L)
}
