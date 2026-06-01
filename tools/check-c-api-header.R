#!/usr/bin/env Rscript

args <- commandArgs(trailingOnly = TRUE)
header <- if (length(args) >= 1L) args[[1L]] else file.path("inst", "include", "ropendal.h")
header <- normalizePath(header, winslash = "/", mustWork = TRUE)
include_dir <- dirname(header)

cc_config <- system2(file.path(R.home("bin"), "R"), c("CMD", "config", "CC"), stdout = TRUE, stderr = TRUE)
status <- attr(cc_config, "status")
if (!is.null(status) && status != 0L) {
  writeLines(cc_config)
  stop("R CMD config CC failed for C API header compile check test", call. = FALSE)
}
cc_config <- trimws(paste(cc_config, collapse = " "))
if (!nzchar(cc_config)) {
  stop("R CMD config CC returned an empty compiler command", call. = FALSE)
}
cc <- strsplit(cc_config, "[[:space:]]+")[[1L]]
cc_cmd <- cc[[1L]]
cc_args <- cc[-1L]

src <- tempfile("ropendal-header-compile-", fileext = ".c")
obj <- tempfile("ropendal-header-compile-", fileext = ".o")

writeLines(c(
  '#include "ropendal.h"',
  '',
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
  '}',
  ''
), src)

cmd <- c(cc_args, "-std=c99", "-Wall", "-Wextra", "-Werror", "-I", include_dir, "-c", src, "-o", obj)
out <- system2(cc_cmd, cmd, stdout = TRUE, stderr = TRUE)
status <- attr(out, "status")
if (is.null(status)) status <- 0L
if (status != 0L) {
  writeLines(out)
  stop(sprintf("C API header compile check failed with status %s", status), call. = FALSE)
}

cat(sprintf("C API header compile check ok: %s\n", header))
