
<!-- README.md is generated from README.Rmd. Please edit this file -->

# Ropendal

<!-- badges: start -->

[![R-CMD-check](https://github.com/sounkou-bioinfo/Ropendal/actions/workflows/r-package.yml/badge.svg)](https://github.com/sounkou-bioinfo/Ropendal/actions/workflows/r-package.yml)
[![R-universe](https://sounkou-bioinfo.r-universe.dev/badges/Ropendal)](https://sounkou-bioinfo.r-universe.dev/Ropendal)
[![Lifecycle:
experimental](https://img.shields.io/badge/lifecycle-experimental-orange.svg)](https://lifecycle.r-lib.org/articles/stages.html#experimental)
<!-- badges: end -->

Ropendal: **Abstract Filesystem Access Interface for R** via the Rust
crate [`opendal`](https://crates.io/crates/opendal), using
[`savvy`](https://github.com/yutannihilation/savvy) for R/Rust FFI. We
keep the bottom layer byte-first: filesystem operations move raw bytes,
and serializers or codecs explicitly materialize those bytes into R
objects.

Async work uses Aio handles inspired by
[`nanonext`](https://nanonext.r-lib.org/): we issue work with
`fs_*_aio()`, wait with `call_aio()`, and retrieve the payload (or an
`opendalErrorValue`) with `collect_aio()`.

We expose the same principles to native packages through a pure C API in
`inst/include/ropendal.h`: submit async work, wait on an Aio, then read
borrowed results or fill caller-owned buffers without routing data
through R raw vectors.

Backends are configured explicitly. We pass HTTP headers to
`opendal("http")` with `headers`, and we pass credential provider
objects such as `credentials_s3()`, `credentials_gcs()`,
`credentials_azblob()`, and `credentials_gdrive()` with `auth`.

## Installation

Ropendal is available from R-universe:

``` r
install.packages(
  "Ropendal",
  repos = c("https://sounkou-bioinfo.r-universe.dev", "https://cloud.r-project.org")
)
```

Source builds can still use explicit OpenDAL feature flags when you need
custom provider wiring.

``` r
# Keep only local filesystem, HTTP, S3-compatible, and Google Drive support.
install.packages(
  "Ropendal",
  repos = c("https://sounkou-bioinfo.r-universe.dev", "https://cloud.r-project.org"),
  type = "source",
  configure.args = "--without-default-rust-features --with-rust-features=fs,http,s3,gdrive"
)

# Add the current cloud-service feature group explicitly.
install.packages(
  "Ropendal",
  repos = c("https://sounkou-bioinfo.r-universe.dev", "https://cloud.r-project.org"),
  type = "source",
  configure.args = "--enable-cloud"
)
```

## Quick start

``` r
library(Ropendal)

root <- file.path(tempdir(), "ropendal-readme")
unlink(root, recursive = TRUE)
dir.create(root, recursive = TRUE)

fs <- opendal("fs", root = root)

fs_write(fs, "note.txt", charToRaw("hello ropendal\n"))
#> [1] TRUE
raw <- fs_read(fs, "note.txt")
rawToChar(raw)
#> [1] "hello ropendal\n"

stat <- fs_stat(fs, "note.txt")
stat[c("path", "type", "size")]
#> $path
#> [1] "note.txt"
#> 
#> $type
#> [1] "file"
#> 
#> $size
#> [1] 15

entries <- fs_ls(fs)
vapply(entries, `[[`, character(1), "path")
#> [1] "note.txt"
```

## Async reads and explicit wait

We get an Aio handle, wait for it, and collect the result when needed.
`fs_read()` is vectorized too: we can pass several paths with
`batch_concurrency` when we want one call to launch multiple reads at
the same time.

``` r
aio <- fs_read_aio(fs, "note.txt")
call_aio(aio)
collect_aio(aio)
#>  [1] 68 65 6c 6c 6f 20 72 6f 70 65 6e 64 61 6c 0a
```

## Tour of the API across backends

The same `fs_*` calls apply to HTTP(S), S3, and Google Drive handles.

### HTTP/HTTPS read example

We define the headers once, require them on the local fixture, and pass
the same headers to the HTTP filesystem handle.

``` r
root <- tempfile("ropendal-http-readme-")
dir.create(root, recursive = TRUE)
writeLines("hello http example", file.path(root, "hello.txt"))

headers <- list(
  Authorization = "Bearer ropendal-readme",
  `X-Ropendal-Example` = "headers"
)

fixture <- OpendalHttpFixture$start(root, required_headers = headers)
http_fs <- opendal("http", endpoint = fixture$endpoint(), root = "/", headers = headers)

rawToChar(fs_read(http_fs, "hello.txt"))
#> [1] "hello http example\n"
fs_stat(http_fs, "hello.txt")[c("path", "type", "size")]
#> $path
#> [1] "hello.txt"
#> 
#> $type
#> [1] "file"
#> 
#> $size
#> [1] 19

fixture$stop()
#> [1] TRUE
```

### S3-compatible store (local MinIO)

We set up a temporary MinIO instance behind the scenes, then check that
the S3-compatible store supports the same byte API.

We write, read, stat, and list objects with the same `fs_*` functions.

``` r
fs_write(s3_fs, "notes/a.txt", charToRaw("hello s3-compatible store\n"))
#> [1] TRUE
fs_write(s3_fs, "notes/b.txt", charToRaw("another object\n"))
#> [1] TRUE

rawToChar(fs_read(s3_fs, "notes/a.txt"))
#> [1] "hello s3-compatible store\n"
fs_stat(s3_fs, "notes/a.txt")[c("path", "type", "size")]
#> $path
#> [1] "notes/a.txt"
#> 
#> $type
#> [1] "file"
#> 
#> $size
#> [1] 26
vapply(fs_ls(s3_fs, "notes/"), `[[`, character(1), "path")
#> [1] "notes/a.txt" "notes/b.txt"
```

The async path returns an Aio handle; we wait and collect when we need
the payload.

``` r
aio <- fs_read_aio(s3_fs, "notes/b.txt")
call_aio(aio)
rawToChar(collect_aio(aio))
#> [1] "another object\n"
```

For a small local comparison, we use a larger object to compare
Ropendal’s default path with its chunked/concurrent path. `paws.storage`
is included as a single-GET baseline.

``` r
restore_aws_env <- readme_set_aws_env(minio)
paws_s3 <- paws.storage::s3(
  endpoint = minio$endpoint,
  region = minio$region,
  config = list(s3_force_path_style = TRUE)
)

payload <- as.raw(sample.int(256L, 8L * 1024L * 1024L, replace = TRUE) - 1L)
bench_key <- "bench/payload.bin"
read_chunk <- 1024 * 1024
write_chunk <- 5 * 1024 * 1024
```

We first compare upload/replace paths: the default Ropendal call,
Ropendal with explicit write chunking/concurrency, and
`paws.storage::put_object()`.

``` r
bench::mark(
  ropendal_replace = fs_replace(s3_fs, bench_key, payload),
  ropendal_replace_concurrent = fs_replace(
    s3_fs,
    bench_key,
    payload,
    write_concurrency = 4,
    chunk_size = write_chunk
  ),
  paws_put = {
    paws_s3$put_object(Bucket = minio$bucket, Key = bench_key, Body = payload)
    TRUE
  },
  iterations = 3,
  check = FALSE
)[, c("expression", "min", "median", "itr/sec", "mem_alloc", "n_gc")]
#> # A tibble: 3 × 5
#>   expression                       min   median `itr/sec` mem_alloc
#>   <bch:expr>                  <bch:tm> <bch:tm>     <dbl> <bch:byt>
#> 1 ropendal_replace              19.8ms   21.1ms      47.5    10.5KB
#> 2 ropendal_replace_concurrent   20.7ms   20.8ms      45.3        0B
#> 3 paws_put                      79.1ms   82.8ms      12.2    11.7MB
```

Then we compare download paths. The Ropendal rows separate default
reads, chunked/concurrent reads, Aio reads, and Aio plus
chunked/concurrent reads; `paws_get` remains the single-GET baseline.

``` r
bench::mark(
  ropendal_read = fs_read(s3_fs, bench_key),
  ropendal_read_concurrent = fs_read(
    s3_fs,
    bench_key,
    read_concurrency = 4,
    chunk_size = read_chunk
  ),
  ropendal_read_aio = collect_aio(fs_read_aio(s3_fs, bench_key)),
  ropendal_read_aio_concurrent = collect_aio(fs_read_aio(
    s3_fs,
    bench_key,
    read_concurrency = 4,
    chunk_size = read_chunk
  )),
  paws_get = paws_s3$get_object(Bucket = minio$bucket, Key = bench_key)$Body,
  iterations = 3,
  check = FALSE
)[, c("expression", "min", "median", "itr/sec", "mem_alloc", "n_gc")]
#> # A tibble: 5 × 5
#>   expression                        min   median `itr/sec` mem_alloc
#>   <bch:expr>                   <bch:tm> <bch:tm>     <dbl> <bch:byt>
#> 1 ropendal_read                  6.02ms   6.02ms     166.        8MB
#> 2 ropendal_read_concurrent       5.04ms    6.3ms     164.        8MB
#> 3 ropendal_read_aio              6.94ms   7.25ms     138.        8MB
#> 4 ropendal_read_aio_concurrent   7.08ms   7.54ms     133.        8MB
#> 5 paws_get                      12.42ms  14.53ms      68.8    8.29MB
```

### Google Drive read example (credentials explicit)

For Google Drive, we pass a credential provider object through `auth`
and keep secret material outside the filesystem handle printout.

``` r
secret_json <- Sys.getenv("ROPENDAL_GDRIVE_SECRET_JSON")
tokens_json <- Sys.getenv("ROPENDAL_GDRIVE_TOKENS_JSON", unset = file.path(dirname(secret_json), "tokens.json"))
gdrive_root <- Sys.getenv("ROPENDAL_GDRIVE_ROOT", unset = "Ropendal")
gdrive_file <- Sys.getenv("ROPENDAL_GDRIVE_FILE", unset = "map_catalog.txt")

drive_fs <- opendal(
  "gdrive",
  root = gdrive_root,
  auth = credentials_gdrive3(
    secret_json = secret_json,
    tokens_json = tokens_json,
    scope = "https://www.googleapis.com/auth/drive"
  )
)
drive_head <- rawToChar(fs_read(drive_fs, gdrive_file, size = 64))
drive_head
```

## Native C API roundtrip

The native API is for other R packages that want OpenDAL-backed byte I/O
without calling R while async work is running. A downstream package can
declare `LinkingTo: Ropendal`, include `<ropendal.h>`, submit async
work, wait on Aio handles, and read borrowed results or fill
caller-owned buffers.

We exercise that installed C API in-process with
[`Rtinycc`](https://github.com/sounkou-bioinfo/Rtinycc). The C code
submits async work, waits on Aio handles, checks metadata, lists
entries, and reads into a caller-owned buffer.

``` r
root <- tempfile("ropendal-c-api-readme-")
dir.create(root, recursive = TRUE)

ropendal_lib <- list.files(
  system.file("libs", package = "Ropendal"),
  pattern = paste0("Ropendal", .Platform$dynlib.ext, "$"),
  recursive = TRUE,
  full.names = TRUE
)

c_api_code <- '
#include <stdint.h>
#include <string.h>
#include "ropendal.h"

static int cleanup(ropendal_fs_t *fs, ropendal_aio_t *aio, ropendal_error_t *err) {
  if (aio) ropendal_aio_release(aio);
  if (fs) ropendal_fs_release(fs);
  if (err) ropendal_error_release(err);
  return -1;
}

int ropendal_c_api_roundtrip(const char *root) {
  ropendal_error_t *err = 0;
  ropendal_fs_t *fs = 0;
  ropendal_aio_t *aio = 0;
  ropendal_kv_t cfg = { sizeof(ropendal_kv_t), "root", root };

  ropendal_status_t st = ropendal_fs_open("fs", &cfg, 1, &fs, &err);
  if (st != ROPENDAL_OK) return cleanup(fs, aio, err);

  const uint8_t payload[] = "hello native api\\n";
  const size_t payload_len = sizeof(payload) - 1;
  ropendal_write_options_t w = {0};
  w.struct_size = sizeof(w);
  w.path = "native.txt";

  st = ropendal_write_aio(fs, &w, payload, payload_len, &aio, &err);
  if (st != ROPENDAL_OK) return cleanup(fs, aio, err);
  st = ropendal_aio_wait(aio, -1, &err);
  if (st != ROPENDAL_OK) return cleanup(fs, aio, err);
  ropendal_aio_release(aio);
  aio = 0;

  st = ropendal_stat_aio(fs, "native.txt", 0, 0, &aio, &err);
  if (st != ROPENDAL_OK) return cleanup(fs, aio, err);
  st = ropendal_aio_wait(aio, -1, &err);
  if (st != ROPENDAL_OK) return cleanup(fs, aio, err);
  const ropendal_entry_t *entry = 0;
  st = ropendal_aio_result_entry(aio, &entry, &err);
  if (st != ROPENDAL_OK || !entry || !entry->has_content_length || entry->content_length != payload_len) {
    return cleanup(fs, aio, err);
  }
  ropendal_aio_release(aio);
  aio = 0;

  ropendal_ls_options_t ls = {0};
  ls.struct_size = sizeof(ls);
  ls.path = "";
  st = ropendal_ls_aio(fs, &ls, &aio, &err);
  if (st != ROPENDAL_OK) return cleanup(fs, aio, err);
  st = ropendal_aio_wait(aio, -1, &err);
  if (st != ROPENDAL_OK) return cleanup(fs, aio, err);
  const ropendal_entry_t *entries = 0;
  size_t nentries = 0;
  st = ropendal_aio_result_entries(aio, &entries, &nentries, &err);
  if (st != ROPENDAL_OK || nentries == 0) return cleanup(fs, aio, err);
  ropendal_aio_release(aio);
  aio = 0;

  uint8_t dst[64] = {0};
  ropendal_read_options_t r = {0};
  r.struct_size = sizeof(r);
  r.path = "native.txt";
  st = ropendal_read_into_aio(fs, &r, dst, sizeof(dst), &aio, &err);
  if (st != ROPENDAL_OK) return cleanup(fs, aio, err);
  st = ropendal_aio_wait(aio, -1, &err);
  if (st != ROPENDAL_OK) return cleanup(fs, aio, err);
  size_t nread = 0;
  st = ropendal_aio_result_nread(aio, &nread, &err);
  if (st != ROPENDAL_OK || nread != payload_len || memcmp(dst, payload, payload_len) != 0) {
    return cleanup(fs, aio, err);
  }

  ropendal_aio_release(aio);
  ropendal_fs_release(fs);
  return (int)nread;
}
'

ffi <- Rtinycc::tcc_ffi() |>
  Rtinycc::tcc_include(system.file("include", package = "Ropendal")) |>
  Rtinycc::tcc_library(ropendal_lib[[1]]) |>
  Rtinycc::tcc_source(c_api_code) |>
  Rtinycc::tcc_bind(
    ropendal_c_api_roundtrip = list(args = list("cstring"), returns = "i32")
  ) |>
  Rtinycc::tcc_compile()

ffi$ropendal_c_api_roundtrip(root)
#> [1] 17
```

## Development

Common targets:

``` bash
make --no-print-directory help
Common development targets:
  make rd              regenerate savvy wrappers, roxygen docs, and NAMESPACE
  make test-fast       install current source and run non-network tinytest
  make test-http       run opt-in local HTTP fixture tests
  make test-s3         run opt-in public read-only S3-compatible tests
  make test-s3-minio   start local MinIO and run writable S3-compatible tests
  make test-gdrive     run opt-in Google Drive tests using local gdrive3 JSON defaults
  make test-ci         run C API checks and CI-only tinytest
  make rdm             render README.md from README.Rmd
  make bench-minio-paws render development MinIO benchmark
  make check           build and run R CMD check --as-cran --no-manual
```
