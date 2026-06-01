#!/usr/bin/env Rscript

pkg_root <- system.file(package = "Ropendal")
if (!nzchar(pkg_root)) stop("Ropendal is not installed", call. = FALSE)
header <- system.file("include", "ropendal.h", package = "Ropendal")
lib_dir <- system.file("libs", package = "Ropendal")
lib <- file.path(lib_dir, paste0("Ropendal", .Platform$dynlib.ext))
if (!file.exists(header)) stop("installed header not found", call. = FALSE)
if (!file.exists(lib)) stop("installed native library not found", call. = FALSE)

cc_candidates <- c(Sys.getenv("CC", unset = ""), Sys.which(c("cc", "gcc", "clang")))
cc_candidates <- cc_candidates[nzchar(cc_candidates)]
if (!length(cc_candidates)) stop("no C compiler found", call. = FALSE)
cc <- cc_candidates[[1L]]

src <- tempfile("ropendal-c-api-roundtrip-", fileext = ".c")
exe <- tempfile("ropendal-c-api-roundtrip-")
root <- tempfile("ropendal-c-api-root-")
dir.create(root, recursive = TRUE)

writeLines(c(
  '#include "ropendal.h"',
  '#include <stdint.h>',
  '#include <stdio.h>',
  '#include <string.h>',
  '',
  'static int fail(const char *msg, const ropendal_error_t *err) {',
  '  if (err != 0) {',
  '    const char *kind = ropendal_error_kind(err);',
  '    const char *text = ropendal_error_message(err);',
  '    fprintf(stderr, "%s: %s %s\\n", msg, kind ? kind : "", text ? text : "");',
  '  } else {',
  '    fprintf(stderr, "%s\\n", msg);',
  '  }',
  '  return 1;',
  '}',
  '',
  'int main(int argc, char **argv) {',
  '  if (argc != 2) return fail("missing root", 0);',
  '  ropendal_fs_t *fs = 0;',
  '  ropendal_aio_t *aio = 0;',
  '  ropendal_error_t *err = 0;',
  '  ropendal_kv_t kv = {0};',
  '  kv.struct_size = sizeof kv;',
  '  kv.key = "root";',
  '  kv.value = argv[1];',
  '  int st = ropendal_fs_open("fs", &kv, 1, &fs, &err);',
  '  if (st != ROPENDAL_OK) return fail("open failed", err);',
  '',
  '  const uint8_t src[4] = {1, 2, 3, 4};',
  '  ropendal_write_options_t w = {0};',
  '  w.struct_size = sizeof w;',
  '  w.path = "a.bin";',
  '  st = ropendal_write_aio(fs, &w, src, sizeof src, &aio, &err);',
  '  if (st != ROPENDAL_OK) return fail("write submit failed", err);',
  '  st = ropendal_aio_wait(aio, -1, &err);',
  '  if (st != ROPENDAL_OK) return fail("write wait failed", err);',
  '  ropendal_aio_release(aio);',
  '',
  '  uint8_t dst[4] = {0, 0, 0, 0};',
  '  ropendal_read_options_t r = {0};',
  '  r.struct_size = sizeof r;',
  '  r.path = "a.bin";',
  '  r.offset = 1;',
  '  r.size = 2;',
  '  r.has_size = 1;',
  '  aio = 0;',
  '  st = ropendal_read_into_aio(fs, &r, dst, sizeof dst, &aio, &err);',
  '  if (st != ROPENDAL_OK) return fail("read submit failed", err);',
  '  st = ropendal_aio_wait(aio, -1, &err);',
  '  if (st != ROPENDAL_OK) return fail("read wait failed", err);',
  '  size_t nread = 0;',
  '  st = ropendal_aio_result_nread(aio, &nread, &err);',
  '  if (st != ROPENDAL_OK) return fail("nread failed", err);',
  '  if (nread != 2 || dst[0] != 2 || dst[1] != 3) return fail("unexpected bytes", 0);',
  '  ropendal_aio_release(aio);',
  '  ropendal_fs_release(fs);',
  '  return 0;',
  '}',
  ''
), src)

cmd <- c(
  "-std=c99", "-Wall", "-Wextra", "-Werror",
  "-I", dirname(header), src, lib,
  paste0("-Wl,-rpath,", lib_dir), paste0("-Wl,-rpath,", R.home("lib")),
  "-o", exe
)
out <- system2(cc, cmd, stdout = TRUE, stderr = TRUE)
status <- attr(out, "status")
if (is.null(status)) status <- 0L
if (status != 0L) {
  writeLines(out)
  stop(sprintf("C API roundtrip compile failed with status %s", status), call. = FALSE)
}

env <- Sys.getenv("LD_LIBRARY_PATH", unset = "")
ld <- paste(c(lib_dir, R.home("lib"), env[nzchar(env)]), collapse = ":")
out <- system2(exe, root, stdout = TRUE, stderr = TRUE, env = paste0("LD_LIBRARY_PATH=", ld))
status <- attr(out, "status")
if (is.null(status)) status <- 0L
if (status != 0L) {
  writeLines(out)
  stop(sprintf("C API roundtrip failed with status %s", status), call. = FALSE)
}
cat("C API roundtrip ok\n")
