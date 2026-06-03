
<!-- README.md is generated from README.Rmd. Please edit that file -->

# Ropendal

Ropendal is an abstract remote filesystem operation interface for R
backed by the Rust crate of [Apache
OpenDAL](https://opendal.apache.org/). The R/Rust interface is built
with the [`savvy`](https://github.com/yutannihilation/savvy) crate,
which generates the R wrappers and native registration used by the
package.

The central abstraction is an `OpendalFs` handle: a root-relative,
byte-addressable filesystem over local files, HTTP endpoints,
S3-compatible services, Google Drive, and other OpenDAL-backed providers
as support is added. Operations move raw bytes and byte ranges, but
metadata and namespace operations such as stat, exists, listing, delete,
copy, rename, and mkdir are remote I/O too; backend failures resolve to
classed error values; credentials are explicit rather than discovered
through hidden ambient provider chains.

Aio handles are a core part of the interface. Reads, writes, metadata
checks, listings, and namespace mutations can return nanonext-like
handles that callers poll, collect, or cancel explicitly, while
background Rust tasks never call R APIs. The package also exposes
immutable `OpendalBytes` handles, chunked read/write iterators, paged
listing/walking iterators for streaming-style transfer and namespace
traversal, and installs `inst/include/ropendal.h`, a pure C API for
downstream native packages that want async byte, metadata, and namespace
operations.

See `design/api-design.md` for API design notes and `design/STATUS.md`
for the implementation/test checklist.

## Core concepts

- `OpendalFs` is a filesystem handle rooted at one local or remote
  location; all paths are normalized relative to that root.
- The core layer moves bytes. Text materialization, R object
  serialization, and native byte codecs are explicit layers above bytes.
- Backend failures are returned as classed values such as
  `opendalErrorValue`, so ordinary filesystem failures can be inspected
  without surprise throws.
- Every remote I/O family has Aio forms where practical; callers
  explicitly wait, collect, race, or cancel `OpendalAio` handles.
- Credentials and request headers are explicit inputs to constructors,
  not hidden provider-chain lookup.

## Installation

Source installs require a Rust toolchain. The default source build
enables the currently wired OpenDAL service features for local
filesystems, HTTP, S3-compatible storage, Google Cloud Storage, Azure
Blob, and Google Drive. Since the core is backed by the OpenDAL Rust
crate, Ropendal can grow to support additional OpenDAL services; adding
a service still needs Ropendal-side configuration, credential, and test
coverage.

``` r
# Keep only local filesystem, HTTP, S3-compatible, and Google Drive support.
install.packages(
  "Ropendal_0.0.0.9000.tar.gz",
  repos = NULL,
  type = "source",
  configure.args = "--without-default-rust-features --with-rust-features=fs,http,s3,gdrive"
)

# Add the current cloud-service feature group explicitly.
install.packages(
  "Ropendal_0.0.0.9000.tar.gz",
  repos = NULL,
  type = "source",
  configure.args = "--enable-cloud"
)
```

Equivalent environment-variable control is also supported for source
builds. From R, set the variable before installing; child
`R CMD INSTALL` processes inherit it.

``` r
Sys.setenv(SAVVY_FEATURES = "fs http s3 gdrive")
install.packages("Ropendal_0.0.0.9000.tar.gz", repos = NULL, type = "source")
```

From a shell, use the same variable inline with `R CMD INSTALL`.

``` bash
SAVVY_FEATURES="fs http s3 gdrive" R CMD INSTALL Ropendal_0.0.0.9000.tar.gz
```

## Getting started

Start with a local filesystem handle. The same API shape is used for
remote services: paths are root-relative, reads and writes move bytes,
metadata and listing are filesystem operations, backend failures are
values, and Aio helpers wait on background Rust work.

### Local filesystem

Create a root-relative filesystem handle. The same `fs` object shape is
used for local files, S3-compatible buckets, HTTP roots, Google Drive
folders, and other providers.

``` r
library(Ropendal)

root <- file.path(tempdir(), "ropendal-readme-example")
unlink(root, recursive = TRUE)
dir.create(root, recursive = TRUE)

fs <- opendal("fs", root = root)
```

Byte operations are explicit. `fs_write()` is create-only, `fs_read()`
returns a raw vector, and byte ranges avoid downloading the whole
object.

``` r
fs_write(fs, "data.bin", as.raw(c(1, 2, 3, 4)))
#> [1] TRUE
fs_read(fs, "data.bin")
#> [1] 01 02 03 04
fs_read(fs, "data.bin", offset = 1, size = 2)
#> [1] 02 03
```

Metadata, byte handles, and listings are ordinary filesystem operations
too. `OpendalBytes` keeps bytes Rust-owned until `as.raw()` materializes
them.

``` r
fs_stat(fs, "data.bin")[c("path", "type", "size")]
#> $path
#> [1] "data.bin"
#> 
#> $type
#> [1] "file"
#> 
#> $size
#> [1] 4
bytes_handle <- fs_read_bytes(fs, "data.bin")
length(bytes_handle)
#> [1] 4
as.raw(bytes_handle)
#> [1] 01 02 03 04
vapply(fs_ls(fs), `[[`, character(1), "path")
#> [1] "data.bin"
```

Paged listing iterators let callers drain namespaces incrementally.

``` r
it <- fs_ls_iter(fs, page_size = 1)
page <- ls_iter_next(it)
page$done
#> [1] FALSE
page$cursor
#> [1] "data.bin"
vapply(page$entries, `[[`, character(1), "path")
#> [1] "data.bin"
```

Backend failures are values. A second create-only write returns an
`opendalErrorValue` instead of overwriting `data.bin`.

``` r
err <- fs_write(fs, "data.bin", as.raw(9))
is_error_value(err)
#> [1] TRUE
error_kind(err)
#> [1] "AlreadyExists"
```

Aio variants submit the same I/O to background Rust tasks and are
collected explicitly from R.

``` r
aio <- fs_read_aio(fs, "data.bin")
call_aio(aio)
aio$value
#> [1] 01 02 03 04

stat <- collect_aio(fs_stat_aio(fs, "data.bin"))
stat$path
#> [1] "data.bin"
stat$size
#> [1] 4
collect_aio(fs_exists_aio(fs, "data.bin"))
#> [1] TRUE
entry <- collect_aio(fs_ls_aio(fs))[[1]]
entry$path
#> [1] "data.bin"
entry$type
#> [1] "file"
collect_aio(fs_replace_aio(fs, "data.bin", as.raw(c(4, 5, 6))))
#> [1] TRUE
fs_read(fs, "data.bin")
#> [1] 04 05 06
```

Monitors let callers wait for one or more Aio completions and drain
completion events without immediately materializing every success
payload.

``` r
read_aio <- fs_read_aio(fs, "data.bin")
monitor_cv <- cv()
monitor <- aio_monitor(list(read = read_aio), cv = monitor_cv)
cv_until(monitor_cv, 1000)
#> [1] TRUE
read_monitor(monitor)
#>   index name event    state
#> 1     1 read ready resolved
collect_aio(read_aio)
#> [1] 04 05 06
```

## Service examples

### Public S3-compatible endpoint

Public S3-compatible endpoints can be opened without credentials by
explicitly skipping request signing and disabling ambient config
loading. Private S3-compatible services should use an explicit
credential provider such as
`credentials_s3(access_key_id, secret_access_key)` rather than hidden
ambient credential lookup. This example uses the public 1000 Genomes AWS
S3 bucket to range-read the beginning of a large reference FASTA and
inspect a small VCF object without downloading an entire genome-scale
file.

``` r
# Public AWS S3 bucket — no credentials needed.
s3fs <- opendal(
  "s3",
  endpoint = "https://s3.amazonaws.com",
  bucket = "1000genomes",
  root = "/",
  region = "us-east-1",
  skip_signature = TRUE,
  disable_config_load = TRUE
)

fasta_path <- "technical/reference/GRCh38_reference_genome/GRCh38_full_analysis_set_plus_decoy_hla.fa"
fasta_head <- rawToChar(fs_read(s3fs, fasta_path, offset = 0, size = 80))
cat(fasta_head, "\n", sep = "")
#> >chr1  AC:CM000663.2  gi:568336023  LN:248956422  rl:Chromosome  M5:6aef897c3d6f

fasta_stat <- fs_stat(s3fs, fasta_path)
data.frame(path = fasta_stat$path, type = fasta_stat$type, size = fasta_stat$size)
#>                                                                                     path
#> 1 technical/reference/GRCh38_reference_genome/GRCh38_full_analysis_set_plus_decoy_hla.fa
#>   type       size
#> 1 file 3263683042

vcf_path <- "phase3/integrated_sv_map/ALL.autosomes.pindel.20130502.complexindex.low_coverage.genotypes.vcf.gz"
vcf_stat <- fs_stat(s3fs, vcf_path)
data.frame(path = vcf_stat$path, type = vcf_stat$type, size = vcf_stat$size)
#>                                                                                                path
#> 1 phase3/integrated_sv_map/ALL.autosomes.pindel.20130502.complexindex.low_coverage.genotypes.vcf.gz
#>   type   size
#> 1 file 405302

vcf_head_gz <- fs_read(s3fs, vcf_path, offset = 0, size = 16384)
con <- gzcon(rawConnection(vcf_head_gz))
vcf_header <- readLines(con, n = 3)
close(con)
cat(vcf_header, sep = "\n")
#> ##fileformat=VCFv4.0
#> ##FILTER=<ID=PASS,Description="All filters passed">
#> ##fileDate=20140627
```

### HTTP endpoint

HTTP endpoints are useful for read-only byte access too. Full reads and
byte ranges work when the server returns byte-range responses. For
authenticated HTTP(S) endpoints, pass explicit request headers such as
`headers = list(Authorization = "Bearer ...")`; Ropendal does not print
these header values.

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

### Google Drive

Google Drive handles use explicit credentials. The Makefile defaults
point at a local `gdrive3` account directory for the JSON files. Run
`make ROPENDAL_README_GDRIVE=true rdm` to execute this README chunk
locally.

``` r
gdrive_fs <- opendal(
  "gdrive",
  root = "Ropendal",
  auth = credentials_gdrive3(
    secret_json = Sys.getenv("ROPENDAL_GDRIVE_SECRET_JSON"),
    tokens_json = Sys.getenv("ROPENDAL_GDRIVE_TOKENS_JSON")
  )
)

fs_stat(gdrive_fs, "map_catalog.txt")[c("path", "type", "size")]

catalog_head <- rawToChar(fs_read(gdrive_fs, "map_catalog.txt", offset = 0, size = 80))
catalog_head
```

## API at a glance

The tables below are generated from progress data frames in this README
source.

<details>
<summary>
Supported operations
</summary>

| operation    | surface             | status            | notes                                                                        |
|:-------------|:--------------------|:------------------|:-----------------------------------------------------------------------------|
| read         | sync, Aio, iterator | implemented       | range reads, batch concurrency, iterators, seek/tell, per-object read tuning |
| write        | sync, Aio, iterator | implemented       | create/replace, batch concurrency, iterators, tell, per-object write tuning  |
| stat/exists  | sync, Aio           | implemented       | metadata and existence values                                                |
| list         | sync, Aio, iterator | implemented       | collectable or paged where provider supports listing                         |
| mkdir/delete | sync, Aio           | implemented       | root-relative path normalization                                             |
| copy         | sync, Aio           | implemented       | direct provider copy                                                         |
| rename       | sync, Aio           | backend-dependent | no silent S3-style emulation of atomic rename                                |
| append       | sync, Aio, iterator | backend-dependent | returns unsupported where OpenDAL reports no append capability               |

</details>
<details>
<summary>
Aio interface
</summary>

| abstraction                   | status                                                                                          | role                                                                                                 |
|:------------------------------|:------------------------------------------------------------------------------------------------|:-----------------------------------------------------------------------------------------------------|
| OpendalAio                    | implemented for bytes, metadata, entries, bools, and unit completions                           | nanonext-like handle for background Rust work                                                        |
| active bindings               | implemented                                                                                     | $value/$data/\$result plus $state/$resolved/\$error                                                  |
| poll_aio()                    | implemented                                                                                     | non-blocking readiness check                                                                         |
| collect_aio()/call_aio()      | implemented                                                                                     | collect returns value; call waits/updates and returns the Aio invisibly                              |
| cv()/aio_monitor()/race_aio() | implemented as R polling helpers                                                                | condition-variable style wait helpers and completion event draining                                  |
| stop_aio()                    | implemented with delayed HTTP cancellation coverage                                             | explicit cancellation request                                                                        |
| native C Aio                  | implemented for byte, read-vector, metadata, entry/list, bool, unit, CV, and monitor operations | downstream native packages can submit async filesystem operations and drain completion notifications |

</details>
<details>
<summary>
Concurrency controls
</summary>

| control                            | status      | scope                                                           |
|:-----------------------------------|:------------|:----------------------------------------------------------------|
| runtime_config(threads=)           | implemented | Tokio worker threads for one filesystem handle                  |
| layer_concurrent_limit(max=)       | implemented | service-wide in-flight backend operation throttle               |
| batch_concurrency                  | implemented | many independent paths/ranges in one call                       |
| read_concurrency/write_concurrency | implemented | per-object chunk/part transfer fanout where OpenDAL supports it |

</details>
<details>
<summary>
Read operations
</summary>

| function_or_path                                          | status      | tuning                                                             |
|:----------------------------------------------------------|:------------|:-------------------------------------------------------------------|
| fs_read()                                                 | implemented | batch_concurrency, read_concurrency, chunk_size, coalesce_gap      |
| fs_read_aio()                                             | implemented | same as fs_read()                                                  |
| fs_read_bytes()/fs_read_bytes_aio()                       | implemented | Rust-owned OpendalBytes handles; explicit as.raw() materialization |
| fs_read_iter()                                            | implemented | one path returns a handle; many paths return a list of handles     |
| read_iter_next()                                          | implemented | next chunk as raw bytes                                            |
| read_iter_collect()                                       | implemented | remaining chunks into one raw vector                               |
| fs_seek()/fs_tell()                                       | implemented | read iterator position within its read window                      |
| C read_aio()/read_into_aio()/readv_aio()/readv_into_aio() | implemented | borrowed result bytes or caller-owned output buffers               |

</details>
<details>
<summary>
Write operations
</summary>

| function_or_path              | status            | tuning                                                                        |
|:------------------------------|:------------------|:------------------------------------------------------------------------------|
| fs_write()/fs_write_aio()     | implemented       | batch_concurrency, write_concurrency, chunk_size; accepts raw or OpendalBytes |
| fs_replace()/fs_replace_aio() | implemented       | same as fs_write(); accepts raw or OpendalBytes                               |
| fs_append()/fs_append_aio()   | backend-dependent | same where append is supported; accepts raw or OpendalBytes                   |
| fs_write_iter()               | implemented       | one path returns a sink; many paths return a list of sinks                    |
| write_iter_write()            | implemented       | submit one raw chunk                                                          |
| write_iter_close()            | implemented       | finalize multipart/streaming write                                            |
| fs_tell()                     | implemented       | bytes submitted to write sink; seek is intentionally unsupported              |
| C write/replace/append Aio    | implemented       | caller-owned input buffer                                                     |

</details>
<details>
<summary>
Supported providers
</summary>

| provider               | status                                      | credentials                                         |
|:-----------------------|:--------------------------------------------|:----------------------------------------------------|
| fs                     | implemented/tested                          | none                                                |
| http                   | implemented/tested read-only                | optional explicit headers                           |
| s3-compatible          | implemented/tested with public S3 and MinIO | explicit credentials_s3() or unsigned public config |
| gdrive                 | implemented/opt-in tested                   | explicit credentials_gdrive()/credentials_gdrive3() |
| gcs                    | feature wired                               | explicit credentials_gcs()                          |
| azblob                 | feature wired                               | explicit credentials_azblob()                       |
| other OpenDAL services | extension path                              | needs config, credential, and tests                 |

</details>
<details>
<summary>
Serializers and codecs
</summary>

| layer                   | status      | notes                                                                                                   |
|:------------------------|:------------|:--------------------------------------------------------------------------------------------------------|
| raw bytes               | implemented | core storage contract uses R raw vectors and OpendalBytes handles                                       |
| text                    | implemented | mode = text plus encoding=; complete-object reads; NUL-producing encodings rejected                     |
| R serialize/unserialize | implemented | mode = serial plus serial_config(), serialize_raw(), and deserialize_raw(); R hooks run on the R thread |
| codecs/compression      | implemented | explicit codec_config()/codec= native byte transforms; gzip and zlib wired for R and C                  |

</details>

## Related work

Apache OpenDAL supplies the Rust backend abstraction that Ropendal binds
into R. The R [`objectstore`](https://github.com/Bisaloo/objectstore)
package is related work for object-store access, and its
[`store_filesystem.R`](https://github.com/Bisaloo/objectstore/blob/main/R/store_filesystem.R)
implementation is a useful reference for filesystem-like surfaces in R.
Ropendal takes a more opinionated byte-filesystem stance, with
root-relative path normalization, explicit credentials,
errors-as-values, async handles, and an exported native C API as part of
the core contract.

The async API is inspired by
[`nanonext`](https://github.com/shikokuchuo/nanonext)’s Aio model:
operations return handles, callers can poll or wait explicitly, and
notification primitives are designed so background workers never call R
APIs.

## Development

Common development and opt-in service test targets are listed by
`make help`.

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
