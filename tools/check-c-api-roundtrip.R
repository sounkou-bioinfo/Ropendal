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
  '  aio = 0;',
  '  st = ropendal_exists_aio(fs, "a.bin", 0, 0, &aio, &err);',
  '  if (st != ROPENDAL_OK) return fail("exists submit failed", err);',
  '  st = ropendal_aio_wait(aio, -1, &err);',
  '  if (st != ROPENDAL_OK) return fail("exists wait failed", err);',
  '  int exists = 0;',
  '  st = ropendal_aio_result_bool(aio, &exists, &err);',
  '  if (st != ROPENDAL_OK) return fail("exists result failed", err);',
  '  if (exists != 1) return fail("exists result was false", 0);',
  '  ropendal_aio_release(aio);',
  '',
  '  aio = 0;',
  '  st = ropendal_stat_aio(fs, "a.bin", 0, 0, &aio, &err);',
  '  if (st != ROPENDAL_OK) return fail("stat submit failed", err);',
  '  st = ropendal_aio_wait(aio, -1, &err);',
  '  if (st != ROPENDAL_OK) return fail("stat wait failed", err);',
  '  const ropendal_entry_t *entry = 0;',
  '  st = ropendal_aio_result_entry(aio, &entry, &err);',
  '  if (st != ROPENDAL_OK) return fail("stat entry failed", err);',
  '  if (entry == 0 || strcmp(entry->path, "a.bin") != 0) return fail("unexpected stat entry", 0);',
  '  if (entry->has_content_length && entry->content_length != 4) return fail("unexpected stat size", 0);',
  '  ropendal_aio_release(aio);',
  '',
  '  aio = 0;',
  '  st = ropendal_mkdir_aio(fs, "dir", 0, 0, &aio, &err);',
  '  if (st != ROPENDAL_OK) return fail("mkdir submit failed", err);',
  '  st = ropendal_aio_wait(aio, -1, &err);',
  '  if (st != ROPENDAL_OK) return fail("mkdir wait failed", err);',
  '  ropendal_aio_release(aio);',
  '',
  '  aio = 0;',
  '  st = ropendal_copy_aio(fs, "a.bin", "dir/b.bin", 0, 0, &aio, &err);',
  '  if (st != ROPENDAL_OK) return fail("copy submit failed", err);',
  '  st = ropendal_aio_wait(aio, -1, &err);',
  '  if (st != ROPENDAL_OK) return fail("copy wait failed", err);',
  '  ropendal_aio_release(aio);',
  '',
  '  aio = 0;',
  '  st = ropendal_rename_aio(fs, "dir/b.bin", "dir/c.bin", 0, 0, &aio, &err);',
  '  if (st != ROPENDAL_OK) return fail("rename submit failed", err);',
  '  st = ropendal_aio_wait(aio, -1, &err);',
  '  if (st != ROPENDAL_OK) return fail("rename wait failed", err);',
  '  ropendal_aio_release(aio);',
  '',
  '  ropendal_ls_options_t l = {0};',
  '  l.struct_size = sizeof l;',
  '  l.path = "";',
  '  l.recursive = 1;',
  '  aio = 0;',
  '  st = ropendal_ls_aio(fs, &l, &aio, &err);',
  '  if (st != ROPENDAL_OK) return fail("ls submit failed", err);',
  '  st = ropendal_aio_wait(aio, -1, &err);',
  '  if (st != ROPENDAL_OK) return fail("ls wait failed", err);',
  '  const ropendal_entry_t *entries = 0;',
  '  size_t nentries = 0;',
  '  st = ropendal_aio_result_entries(aio, &entries, &nentries, &err);',
  '  if (st != ROPENDAL_OK) return fail("ls entries failed", err);',
  '  int found = 0;',
  '  for (size_t i = 0; i < nentries; ++i) {',
  '    if (strcmp(entries[i].path, "dir/c.bin") == 0) found = 1;',
  '  }',
  '  if (!found) return fail("listed entry not found", 0);',
  '  ropendal_aio_release(aio);',
  '',
  '  aio = 0;',
  '  ropendal_delete_options_t d = {0};',
  '  d.struct_size = sizeof d;',
  '  d.path = "dir/c.bin";',
  '  st = ropendal_delete_aio(fs, &d, &aio, &err);',
  '  if (st != ROPENDAL_OK) return fail("delete submit failed", err);',
  '  st = ropendal_aio_wait(aio, -1, &err);',
  '  if (st != ROPENDAL_OK) return fail("delete wait failed", err);',
  '  ropendal_aio_release(aio);',
  '',
  '  aio = 0;',
  '  st = ropendal_exists_aio(fs, "dir/c.bin", 0, 0, &aio, &err);',
  '  if (st != ROPENDAL_OK) return fail("post-delete exists submit failed", err);',
  '  st = ropendal_aio_wait(aio, -1, &err);',
  '  if (st != ROPENDAL_OK) return fail("post-delete exists wait failed", err);',
  '  exists = 1;',
  '  st = ropendal_aio_result_bool(aio, &exists, &err);',
  '  if (st != ROPENDAL_OK) return fail("post-delete exists result failed", err);',
  '  if (exists != 0) return fail("deleted file still exists", 0);',
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
  '',
  '  uint8_t dstv0[2] = {0, 0};',
  '  uint8_t dstv1[2] = {0, 0};',
  '  ropendal_read_into_request_t reqs[2] = {{0}};',
  '  reqs[0].struct_size = sizeof reqs[0];',
  '  reqs[0].path = "a.bin";',
  '  reqs[0].offset = 0;',
  '  reqs[0].size = 2;',
  '  reqs[0].has_size = 1;',
  '  reqs[0].dst = dstv0;',
  '  reqs[0].dst_len = sizeof dstv0;',
  '  reqs[1].struct_size = sizeof reqs[1];',
  '  reqs[1].path = "a.bin";',
  '  reqs[1].offset = 2;',
  '  reqs[1].size = 2;',
  '  reqs[1].has_size = 1;',
  '  reqs[1].dst = dstv1;',
  '  reqs[1].dst_len = sizeof dstv1;',
  '  ropendal_readv_options_t rv = {0};',
  '  rv.struct_size = sizeof rv;',
  '  rv.batch_concurrency = 2;',
  '  aio = 0;',
  '  st = ropendal_readv_into_aio(fs, reqs, 2, &rv, &aio, &err);',
  '  if (st != ROPENDAL_OK) return fail("readv submit failed", err);',
  '  st = ropendal_aio_wait(aio, -1, &err);',
  '  if (st != ROPENDAL_OK) return fail("readv wait failed", err);',
  '  nread = 0;',
  '  st = ropendal_aio_result_nread(aio, &nread, &err);',
  '  if (st != ROPENDAL_OK) return fail("readv nread failed", err);',
  '  if (nread != 4 || dstv0[0] != 1 || dstv0[1] != 2 || dstv1[0] != 3 || dstv1[1] != 4) return fail("unexpected readv bytes", 0);',
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
