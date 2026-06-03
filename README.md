
<!-- README.md is generated from README.Rmd. Please edit this file -->

# Ropendal

Ropendal is an abstract filesystem interface for R backed by the Rust
crate of [Apache OpenDAL](https://opendal.apache.org/). The package is
byte-first: operations move raw bytes, then explicitly materialize into
R objects through modes and serializers.

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
raw
#>  [1] 68 65 6c 6c 6f 20 72 6f 70 65 6e 64 61 6c 0a

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

``` r
aio <- fs_read_aio(fs, "note.txt")
call_aio(aio)
collect_aio(aio)
#>  [1] 68 65 6c 6c 6f 20 72 6f 70 65 6e 64 61 6c 0a
```

## Documentation and concepts

The README now points to full vignette guidance.

- <https://sounkou-bioinfo.github.io/Ropendal/articles/getting-started.html>
- <https://sounkou-bioinfo.github.io/Ropendal/articles/abstract-filesystem.html>
- <https://sounkou-bioinfo.github.io/Ropendal/articles/async-aio.html>
- <https://sounkou-bioinfo.github.io/Ropendal/articles/serializers.html>
- <https://sounkou-bioinfo.github.io/Ropendal/articles/credentials.html>
- <https://sounkou-bioinfo.github.io/Ropendal/articles/native-c-api.html>
- <https://sounkou-bioinfo.github.io/Ropendal/articles/internals.html>

More implementation notes and completion status are tracked in
`design/STATUS.md`.

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
  make rdm             render README.md from README.Rmd; GDrive execution stays opt-in
  make bench-minio-paws render development MinIO benchmark
  make check           build and run R CMD check --as-cran --no-manual
```
