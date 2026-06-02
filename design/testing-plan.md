# Ropendal testing plan

## Test framework

Use `tinytest` only. The package test entry point is `tests/tinytest.R`, and package tests live under `inst/tinytest/`.

Shared helpers live in `inst/tinytest/helper-ropendal.R` and must remain dependency-light: base R plus `tinytest` only.

Primary commands:

```sh
make test-fast      # install current source, run non-network tinytest suite
make test-local     # same intent; reserved for local filesystem coverage
make test-network       # opt into network/service tests via env vars
make test-http          # opt into local HTTP fixture tests via internal Rust fixture
make test-s3            # opt into public read-only S3-compatible endpoint tests
make test-s3-minio      # start local MinIO and run writable S3-compatible tests
make test-c-api-header  # compile the source C API header as pure C
make test-ci            # run CI-only non-network API contract tests
make test-gdrive        # opt into Google Drive tests via env vars
make test           # build, install, run tinytest
```

## Principles

1. Tests should exercise the public R API when possible.
2. Rust implementation details should be tested through R or C ABI boundaries, not by duplicating Rust internals in R tests.
3. Network tests are opt-in and must never run by default on CRAN or normal local checks.
4. Secrets must never be printed, snapshotted, or stored in fixtures.
5. Async tests must never require background Rust tasks to touch R APIs.
6. Range-read shape and ordering are contract tests, not implementation details.
7. C API tests should compile a tiny downstream consumer/header compile check without R headers. The core native consumer contract is pure C, not `R_RegisterCCallable()`.

## Test groups

### 00 package contract

Files: `test-00-*`

Current coverage:

- package loads
- `tinytest` is in `Suggests`
- installed C header exists
- async-first C API symbols are present in the header

Future coverage:

- expected R symbols exported
- pkgdown/reference ordering source remains valid
- README generated from `README.Rmd`

### 10 pure R API contracts

Files: `test-10-*`

No OpenDAL backend required. These tests should cover:

- path/range request normalization
- `read_requests()` construction
- strict vector/list length matching rules; no R recycling
- `result = "auto"`, `"flat"`, and `"nested"` shape contracts
- `opendalErrorValue` construction for filesystem/backend failures
- redacted printing for credentials
- `serial_config()` validation and planned `codec_config()` validation

### 20 local filesystem backend

Files: `test-20-fs-*`

Always run by default. Use `tempfile()` roots only. Coverage:

- construct `opendal("fs", root = tempdir)`
- write/read raw bytes
- `OpendalBytes` read handles, `as.raw()`/`length()`, Aio collection, and write/replace/append acceptance
- read scalar ranges
- read multiple ranges from one file
- read many paths with list-of-ranges
- stat metadata
- `fs_ls()` output columns and ordering rules
- `fs_ls_iter()` empty-listing, paged listing, and collect behavior
- `fs_walk_iter()` recursive traversal pages/collection
- mkdir/delete/copy/rename
- declarative capability profiles: supported operations, implementation source, and unsupported-operation error values

### 30 async Aio behavior

Files: `test-30-aio-*`

Run on local filesystem by default. Coverage:

- each `_aio` function returns immediately with expected class
- `unresolved()` behavior before/after completion
- `call_aio()` waits and caches result
- `collect_aio()` returns result with the same shape as sync function
- `stop_aio()` cancellation behavior
- errors captured as structured values by default, nanonext-like
- list/vectorized Aio result shape and names

### 40 serialization and codecs

Files: `test-40-serial-*`

Default tests should use only base R objects and toy custom classes. Coverage:

- base `mode = "serial"` roundtrip (implemented locally)
- custom `serial_config(class, sfunc, ufunc)` roundtrip (implemented locally)
- multiple class hooks
- `opt(fs, "serial") <- list()` removes hooks (implemented locally)
- `mode = "codec"` explicit codec roundtrip
- deserializer runs at collect/materialization time for async reads (implemented locally)
- partial range + `mode = "serial"` errors clearly (implemented locally)

### 50 C ABI / downstream consumer

Files/tools: `tools/check-c-api-header.R`, `tools/check-c-api-roundtrip.R`, and CI-only `test-90-ci-native-api-contract.R`.

Coverage:

- pure C header compiles without R headers or `SEXP`
- `ropendal_api_version()` and exported C symbols remain link-visible through the installed native library
- pure C `ropendal_fs_open()` lifecycle contract
- `ropendal_write_aio()` and `ropendal_read_into_aio()` fill caller-owned buffers
- `ropendal_exists_aio()` plus `ropendal_aio_result_bool()`
- `ropendal_stat_aio()` plus `ropendal_aio_result_entry()`
- `ropendal_ls_aio()` plus `ropendal_aio_result_entries()`
- namespace mutations `ropendal_mkdir_aio()`, `ropendal_copy_aio()`, `ropendal_rename_aio()`, and `ropendal_delete_aio()`
- still planned: `ropendal_fs_from_uri()` fixture coverage, `ropendal_readv_into_aio()`, per-request `readv` result/error reporting, and cancellation safety tests

### 90 CI-only API contract tests

Files: `test-90-*`

These tests are installed with the package but exit immediately unless `ROPENDAL_TEST_CI=true`. They are for API contract linting and non-CRAN checks that should not run by default during CRAN-like checks, even if a generic `CI=true` env var is present.

Coverage:

- pure C header contract: no R headers and no `SEXP`
- C header ABI hygiene such as `struct_size` fields
- source and installed header compile check compilation
- exported R symbol presence once implemented
- generated wrapper drift checks if they can be run without network or secrets

### 60 service integrations

Files: `test-60-service-*`

Opt-in only. All service tests require `ROPENDAL_TEST_NETWORK=true` plus a service-specific flag.

Suggested flags:

```sh
ROPENDAL_TEST_NETWORK=true
ROPENDAL_TEST_S3=true
ROPENDAL_TEST_GCS=true
ROPENDAL_TEST_AZBLOB=true
ROPENDAL_TEST_GDRIVE=true
```

Service env vars:

- Public S3-compatible read-only endpoint: `ROPENDAL_TEST_S3=true`; `make test-s3` passes `ROPENDAL_S3_PUBLIC_ENDPOINT`, `ROPENDAL_S3_PUBLIC_BUCKET`, `ROPENDAL_S3_PUBLIC_ROOT`, `ROPENDAL_S3_PUBLIC_REGION`, `ROPENDAL_S3_PUBLIC_FILE`, `ROPENDAL_S3_PUBLIC_RANGE_FILE`, and `ROPENDAL_S3_PUBLIC_LIST_PATH`. Defaults use EMBL-EBI's public IDR object store.
- Local writable S3-compatible endpoint: `ROPENDAL_TEST_S3_MINIO=true`; `make test-s3-minio` starts MinIO via `tools/run-minio-test.sh`, creates a bucket, and passes `ROPENDAL_S3_MINIO_ENDPOINT`, `ROPENDAL_S3_MINIO_BUCKET`, `ROPENDAL_S3_MINIO_REGION`, `ROPENDAL_S3_MINIO_ACCESS_KEY_ID`, `ROPENDAL_S3_MINIO_SECRET_ACCESS_KEY`, and `ROPENDAL_S3_MINIO_ROOT`. This target is suitable for CI because it needs no cloud credentials.
- External S3 credentials, if added later: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, optional `AWS_SESSION_TOKEN`, `AWS_REGION`, `ROPENDAL_TEST_S3_BUCKET`, `ROPENDAL_TEST_S3_PREFIX`.
- GDrive: `make test-gdrive` passes `ROPENDAL_GDRIVE_SECRET_JSON`, `ROPENDAL_GDRIVE_TOKENS_JSON`, `ROPENDAL_GDRIVE_ROOT`, and `ROPENDAL_GDRIVE_FILE`; direct token/provider-chain helpers are intentionally not part of hidden core lookup.
- HTTP: `ROPENDAL_TEST_HTTP=true`; local fixture tests use Ropendal's internal non-blocking Rust HTTP fixture, including required request-header checks

Coverage:

- auth construction redacts secrets
- public S3-compatible read/stat/list/range reads
- HTTP fixture read/stat/range reads, unsupported listing value, explicit `headers=` authentication path, and delayed-response pending Aio state
- local MinIO write/read/stat/list/copy/delete in an isolated prefix
- unsupported S3-compatible atomic rename returns an error value rather than silent emulation
- range reads
- concurrency options accepted and bounded
- service-specific rate-limit defaults are conservative

## Environment and skip helpers

Use predicates from `helper-ropendal.R`. tinytest's `exit_file()` must be called directly at top level in the test file and unqualified:

```r
source(system.file("tinytest", "helper-ropendal.R", package = "Ropendal"), local = TRUE)
if (!ropendal_service_tests_enabled("gdrive")) exit_file("set ROPENDAL_TEST_NETWORK=true and ROPENDAL_TEST_GDRIVE=true")
missing <- ropendal_missing_env(c("ROPENDAL_GDRIVE_SECRET_JSON", "ROPENDAL_GDRIVE_ROOT"))
if (length(missing)) exit_file(paste("missing env vars:", paste(missing, collapse = ", ")))
```

Network tests must gate before constructing credentials.

## Shape contract for `fs_read()`

The R consumer contract to test and preserve:

```r
fs_read(fs, "x")
# raw

fs_read(fs, "x", offset = c(0, 10), size = c(2, 2))
# list(raw, raw)

fs_read(fs, c("x", "y"))
# list(x = raw, y = raw) when names are available or path names are unique

fs_read(fs,
  path = c("x", "y"),
  offset = list(c(0, 10), c(5)),
  size = list(c(2, 2), c(3)),
  result = "nested"
)
# list(x = list(raw, raw), y = list(raw))

fs_read(fs, read_requests(c("x", "x", "y"), c(0, 10, 5), c(2, 2, 3)),
        result = "flat")
# list(raw, raw, raw)
```

## C API contract to test later

Public option/result structs contain `struct_size` for ABI extensibility. Future tests should check that downstream C code initializes structs with `sizeof(struct)` and that older struct sizes are accepted or rejected intentionally.

Remaining important cases:

- many async range reads into caller buffers
- cancellation before completion
- timeout wait
- per-request failure in a vector read
- release order: aio before fs and fs before aio
- monitor/notification flow using `ropendal_cv_*` and `ropendal_monitor_*`
