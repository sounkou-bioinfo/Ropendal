
<!-- README.md is generated from README.Rmd. Please edit this file -->

# Ropendal

<!-- badges: start -->

[![R-CMD-check](https://github.com/sounkou-bioinfo/Ropendal/actions/workflows/r-package.yml/badge.svg)](https://github.com/sounkou-bioinfo/Ropendal/actions/workflows/r-package.yml)
[![R-universe](https://sounkou-bioinfo.r-universe.dev/badges/Ropendal)](https://sounkou-bioinfo.r-universe.dev/Ropendal)
[![Lifecycle:
experimental](https://img.shields.io/badge/lifecycle-experimental-orange.svg)](https://lifecycle.r-lib.org/articles/stages.html#experimental)
<!-- badges: end -->

Ropendal: **Abstract Filesystem Access for R** via [Apache
OpenDAL](https://opendal.apache.org/). The package is byte-first:
operations move raw bytes, then explicitly materialize into R objects
through modes and serializers.

The async API is inspired by `nanonext`-style Aio handles: issue async
work with `fs_*_aio()`, then `call_aio()` to wait for completion and
update the Aio object, and `collect_aio()` to retrieve the result (or an
`opendalErrorValue`).

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

``` r
aio <- fs_read_aio(fs, "note.txt")
call_aio(aio)
collect_aio(aio)
#>  [1] 68 65 6c 6c 6f 20 72 6f 70 65 6e 64 61 6c 0a
```

## Tour of the API across backends

The same `fs_*` calls apply to HTTP(S), S3, and Google Drive handles.

### HTTP/HTTPS read example (uses the package-provided HTTP fixture server)

``` r
# Uses the fixture HTTP server provided by this package for a reproducible example.
root <- tempfile("ropendal-http-readme-")
dir.create(root, recursive = TRUE)
writeLines("hello http example", file.path(root, "hello.txt"))

fixture <- OpendalHttpFixture$start(root)
http_fs <- opendal("http", endpoint = fixture$endpoint(), root = "/")
http_head <- rawToChar(fs_read(http_fs, "hello.txt"))
http_head
fixture$stop()
```

### Public S3-compatible read example (VCF header)

``` r
if (!identical(Sys.getenv("ROPENDAL_README_S3", ""), "true")) {
  cat("Set ROPENDAL_README_S3=true and network access to run this public S3 example.\n")
} else {
  s3_fs <- opendal(
    "s3",
    endpoint = Sys.getenv("ROPENDAL_S3_PUBLIC_ENDPOINT", "https://uk1s3.embassy.ebi.ac.uk"),
    bucket = Sys.getenv("ROPENDAL_S3_PUBLIC_BUCKET", "idr"),
    root = Sys.getenv("ROPENDAL_S3_PUBLIC_ROOT", "/zarr/v0.4/idr0062A/6001240.zarr"),
    region = Sys.getenv("ROPENDAL_S3_PUBLIC_REGION", "us-east-1"),
    skip_signature = TRUE,
    disable_config_load = TRUE
  )

  vcf_gz <- fs_read(
    s3_fs,
    "phase3/integrated_sv_map/ALL.autosomes.pindel.20130502.complexindex.low_coverage.genotypes.vcf.gz",
    size = 16384
  )
  con <- gzcon(rawConnection(vcf_gz))
  vcf_header <- readLines(con, n = 5)
  close(con)
  cat(vcf_header, sep = "\n")
  cat("\n")
}
#> Set ROPENDAL_README_S3=true and network access to run this public S3 example.
```

### Google Drive read example (credentials explicit)

``` r
if (!identical(Sys.getenv("ROPENDAL_README_GDRIVE", ""), "true")) {
  cat("Set ROPENDAL_README_GDRIVE=true with credentials to run this Google Drive example.\n")
} else {
  secret_json <- Sys.getenv("ROPENDAL_GDRIVE_SECRET_JSON")
  tokens_json <- Sys.getenv("ROPENDAL_GDRIVE_TOKENS_JSON", unset = file.path(dirname(secret_json), "tokens.json"))
  if (!nzchar(secret_json) || !nzchar(tokens_json) || !file.exists(secret_json) || !file.exists(tokens_json)) {
    cat("Set valid ROPENDAL_GDRIVE_SECRET_JSON and ROPENDAL_GDRIVE_TOKENS_JSON paths to run this example.\n")
  } else {
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
    cat(drive_head, "\n")
  }
}
#> Set ROPENDAL_README_GDRIVE=true with credentials to run this Google Drive example.
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
