
<!-- README.md is generated from README.Rmd. Please edit that file -->

# Ropendal

Ropendal is a byte-oriented abstract filesystem interface for R backed
by the Rust crate of [Apache OpenDAL](https://opendal.apache.org/).

Ropendal currently exposes local filesystems, HTTP endpoints,
S3-compatible services, and Google Drive as byte-addressable filesystem
handles with explicit backend configuration and explicit credentials.

It supports raw byte reads, range reads, writes, replacements, appends
where supported, metadata, listing, path normalization, classed error
values, read Aio handles, and a pure C API for direct byte access. The
API is designed around filesystem primitives over bytes, byte ranges,
and metadata; nanonext-like async Aio objects, condition variables, and
monitors; errors-as-values resolution; explicit raw-vector serializers
and deserializers; an async-first C API for native packages; and
credential helpers including Google Drive and S3-compatible providers.

See `design/api-design.md` for the current API design notes and
`design/STATUS.md` for the implementation/test checklist.

## Local filesystem example

``` r
library(Ropendal)

root <- file.path(tempdir(), "ropendal-readme-example")
unlink(root, recursive = TRUE)
dir.create(root, recursive = TRUE)

fs <- opendal("fs", root = root)

fs_write(fs, "data.bin", as.raw(c(1, 2, 3, 4)))
#> [1] TRUE
fs_read(fs, "data.bin")
#> [1] 01 02 03 04
fs_read(fs, "data.bin", offset = 1, size = 2)
#> [1] 02 03

fs_stat(fs, "data.bin")[c("path", "type", "size")]
#> $path
#> [1] "data.bin"
#> 
#> $type
#> [1] "file"
#> 
#> $size
#> [1] 4
vapply(fs_ls(fs), `[[`, character(1), "path")
#> [1] "data.bin"
```

## Public S3-compatible endpoint example

Public S3-compatible endpoints can be opened without credentials by
explicitly skipping request signing and disabling ambient config
loading. Private S3-compatible services should use an explicit
credential provider such as
`credentials_s3(access_key_id, secret_access_key)` rather than hidden
ambient credential lookup. This example uses a public IDR OME-Zarr
object store hosted by EMBL-EBI.

``` r
# Public HTTPS/S3-compatible endpoint — no credentials needed.
s3fs <- opendal(
  "s3",
  endpoint = "https://uk1s3.embassy.ebi.ac.uk",
  bucket = "idr",
  root = "/zarr/v0.4/idr0062A/6001240.zarr",
  region = "us-east-1",
  skip_signature = TRUE,
  disable_config_load = TRUE
)

zarray <- rawToChar(fs_read(s3fs, "0/.zarray"))
grepl('"dtype": "<u2"', zarray, fixed = TRUE)
#> [1] TRUE

fs_stat(s3fs, "0/.zarray")[c("path", "type", "size")]
#> $path
#> [1] "0/.zarray"
#>
#> $type
#> [1] "file"
#>
#> $size
#> [1] 417

chunk_head <- fs_read(s3fs, "0/0/0/0/0", offset = 0, size = 16)
length(chunk_head)
#> [1] 16
```

HTTP endpoints are useful for read-only byte access too. Full reads and
byte ranges work when the server returns byte-range responses.

``` r
http_fs <- opendal(
  "http",
  endpoint = "https://uk1s3.embassy.ebi.ac.uk/idr/zarr/v0.4/idr0062A/6001240.zarr",
  root = "/"
)

zarray_head <- rawToChar(fs_read(http_fs, "0/.zarray", offset = 0, size = 40))
grepl('"chunks"', zarray_head, fixed = TRUE)
#> [1] TRUE

fs_stat(http_fs, "0/.zarray")[c("path", "type", "size")]
#> $path
#> [1] "0/.zarray"
#> 
#> $type
#> [1] "file"
#> 
#> $size
#> [1] 417

chunk_head <- fs_read(http_fs, "0/0/0/0/0", offset = 0, size = 16)
length(chunk_head)
#> [1] 16
```

## Google Drive example

Google Drive handles use explicit credentials. The README render can opt
into a real Google Drive read by setting credential paths in the
environment; `make rdm` sets local defaults for a `gdrive3` account
directory when present.

``` r
gdrive_fs <- opendal(
  "gdrive",
  root = Sys.getenv("ROPENDAL_GDRIVE_ROOT", "Ropendal"),
  auth = credentials_gdrive3(
    secret_json = Sys.getenv("ROPENDAL_GDRIVE_SECRET_JSON"),
    tokens_json = Sys.getenv("ROPENDAL_GDRIVE_TOKENS_JSON")
  )
)

gdrive_path <- Sys.getenv("ROPENDAL_GDRIVE_FILE", "map_catalog.txt")
fs_stat(gdrive_fs, gdrive_path)[c("path", "type", "size")]
#> $path
#> [1] "map_catalog.txt"
#> 
#> $type
#> [1] "file"
#> 
#> $size
#> [1] 547

catalog_head <- fs_read(gdrive_fs, gdrive_path, offset = 0, size = 80)
length(catalog_head)
#> [1] 80
```

Filesystem failures are returned as values.

``` r
err <- fs_write(fs, "data.bin", as.raw(9))
is_error_value(err)
#> [1] TRUE
error_kind(err)
#> [1] "AlreadyExists"
```

Read operations can also return Aio handles.

``` r
aio <- fs_read_aio(fs, "data.bin")
poll_aio(aio)
#> [1] "pending"
call_aio(aio)
#> [1] 01 02 03 04
```

## Development

Common development and opt-in service test targets are listed by
`make help`.

``` bash
make help
make[1]: Entering directory '/root/Ropendal'
Common development targets:
  make rd              regenerate savvy wrappers, roxygen docs, and NAMESPACE
  make test-fast       install current source and run non-network tinytest
  make test-http       run opt-in local HTTP fixture tests
  make test-s3         run opt-in public read-only S3-compatible tests
  make test-s3-minio   start local MinIO and run writable S3-compatible tests
  make test-gdrive     run opt-in Google Drive tests using explicit env paths
  make test-ci         run C API checks and CI-only tinytest
  make rdm             render README.md from README.Rmd
  make check           build and run R CMD check --as-cran --no-manual
make[1]: Leaving directory '/root/Ropendal'
```
