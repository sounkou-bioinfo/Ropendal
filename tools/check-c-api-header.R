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
  '  ropendal_store_options_t store_opts = {0};',
  '  ropendal_store_cache_options_t store_cache_opts = {0};',
  '  ropendal_store_block_cache_options_t store_block_cache_opts = {0};',
  '  ropendal_store_read_options_t store_read_opts = {0};',
  '  ropendal_store_write_options_t store_write_opts = {0};',
  '  ropendal_store_ls_options_t store_ls_opts = {0};',
  '  ropendal_store_delete_options_t store_delete_opts = {0};',
  '  ropendal_read_options_t read_opts = {0};',
  '  ropendal_read_request_t read_req = {0};',
  '  ropendal_read_into_request_t read_into_req = {0};',
  '  ropendal_readv_options_t readv_opts = {0};',
  '  ropendal_readv_result_t readv_result = {0};',
  '  ropendal_write_options_t write_opts = {0};',
  '  ropendal_ls_options_t ls_opts = {0};',
  '  ropendal_delete_options_t delete_opts = {0};',
  '  ropendal_entry_t entry = {0};',
  '  ropendal_monitor_event_t event = {0};',
  '  ropendal_bytes_t *bytes = 0;',
  '  const uint8_t *byte_ptr = ropendal_bytes_data(bytes);',
  '  size_t byte_len = ropendal_bytes_len(bytes);',
  '  ropendal_status_t (*codec_fn)(const char *, const uint8_t *, size_t, ropendal_bytes_t **, ropendal_error_t **) = ropendal_codec_encode;',
  '  ropendal_store_t *store = 0;',
  '  ropendal_status_t (*store_open_fn)(ropendal_fs_t *, const ropendal_store_options_t *, ropendal_store_t **, ropendal_error_t **) = ropendal_store_open;',
  '  ropendal_status_t (*store_cache_open_fn)(ropendal_store_t *, ropendal_store_t *, const ropendal_store_cache_options_t *, ropendal_store_t **, ropendal_error_t **) = ropendal_store_cache_open;',
  '  ropendal_status_t (*store_block_cache_open_fn)(ropendal_store_t *, ropendal_store_t *, const ropendal_store_block_cache_options_t *, ropendal_store_t **, ropendal_error_t **) = ropendal_store_block_cache_open;',
  '  kv.struct_size = sizeof kv;',
  '  store_opts.struct_size = sizeof store_opts;',
  '  store_cache_opts.struct_size = sizeof store_cache_opts;',
  '  store_cache_opts.validate = ROPENDAL_STORE_CACHE_VALIDATE_NONE;',
  '  store_block_cache_opts.struct_size = sizeof store_block_cache_opts;',
  '  store_block_cache_opts.block_size = 4096;',
  '  store_block_cache_opts.validate = ROPENDAL_STORE_CACHE_VALIDATE_NONE;',
  '  store_read_opts.struct_size = sizeof store_read_opts;',
  '  store_write_opts.struct_size = sizeof store_write_opts;',
  '  store_ls_opts.struct_size = sizeof store_ls_opts;',
  '  store_delete_opts.struct_size = sizeof store_delete_opts;',
  '  read_opts.struct_size = sizeof read_opts;',
  '  read_req.struct_size = sizeof read_req;',
  '  read_into_req.struct_size = sizeof read_into_req;',
  '  readv_opts.struct_size = sizeof readv_opts;',
  '  readv_result.struct_size = sizeof readv_result;',
  '  write_opts.struct_size = sizeof write_opts;',
  '  ls_opts.struct_size = sizeof ls_opts;',
  '  delete_opts.struct_size = sizeof delete_opts;',
  '  entry.struct_size = sizeof entry;',
  '  event.struct_size = sizeof event;',
  '  (void)kv; (void)store; (void)store_opts; (void)store_cache_opts; (void)store_block_cache_opts; (void)store_read_opts;',
  '  (void)store_write_opts; (void)store_ls_opts; (void)store_delete_opts;',
  '  (void)store_open_fn; (void)store_cache_open_fn; (void)store_block_cache_open_fn;',
  '  (void)read_opts; (void)read_req; (void)read_into_req;',
  '  (void)readv_opts; (void)readv_result; (void)write_opts; (void)ls_opts; (void)delete_opts;',
  '  (void)entry; (void)event; (void)byte_ptr; (void)byte_len; (void)codec_fn;',
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
