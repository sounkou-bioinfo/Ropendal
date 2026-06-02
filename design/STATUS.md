# Ropendal implementation and test checklist

Status labels:

- `contract`: API/design contract documented, no implementation yet
- `implemented`: implementation exists
- `tested-default`: covered by default non-network tinytest / R CMD check
- `tested-ci`: covered by CI-only tests (`ROPENDAL_TEST_CI=true`)
- `planned`: not started

## Current state summary

Ropendal currently has a working implementation through the Apache OpenDAL Rust crate and savvy. Rust code is split into modules, the public R layer is thin and delegates operation logic to generated Rust-backed methods, error values are classed in Rust, local byte operations plus read/write/listing/walking iterators and `OpendalBytes` byte handles are tested, public S3-compatible, HTTP, and Google Drive opt-in service tests pass, HTTP(S) filesystems support explicit request headers, and the pure C API has compiled roundtrip coverage for byte, metadata/existence, listing, and namespace operations against the installed native library.

## CI matrix

| Area | Command | Default/CRAN | CI-only | Status |
|---|---|---:|---:|---|
| R package build | `R CMD build .` | yes | yes | implemented/tested |
| Default tinytest | `make test-fast` | yes | yes | implemented/tested-default |
| C API source-header compile check | `make test-c-api-header` | no | yes | implemented/tested-ci |
| Installed C API roundtrip | `make test-c-api-roundtrip` | no | yes | implemented/tested-ci |
| Installed C API contract lint | `make test-ci` | no | yes | implemented/tested-ci |
| R CMD check | `R CMD check --no-manual` | yes | yes | implemented in workflow |
| Network/service tests | `make test-network` | no | opt-in | implemented for S3/HTTP/GDrive; broader services planned |
| Local HTTP fixture tests | `make test-http` | no | opt-in | implemented with internal Rust fixture, including required-header authentication coverage |
| Public S3-compatible tests | `make test-s3` | no | opt-in network | implemented with EMBL-EBI IDR public object store |
| Local MinIO S3-compatible tests | `make test-s3-minio` | no | CI/local service | implemented; starts MinIO with `minio`/`mc`; wired into GitHub Actions |
| Google Drive tests | `make test-gdrive` | no | opt-in with secrets | implemented with explicit env paths |

GitHub Actions jobs:

- `R API tinytest (non-network)`
- `C API contract and CI-only tests`
- `Local MinIO S3-compatible tests`
- `R CMD check (no CI-only/network tests)`

## Repository infrastructure

| Item | Implemented | Tested default | Tested CI | Notes |
|---|---:|---:|---:|---|
| `tests/tinytest.R` entrypoint | yes | yes | yes | runs installed package tests |
| `inst/tinytest/helper-ropendal.R` | yes | yes | yes | env gates, temp helpers |
| CI-only gating via `ROPENDAL_TEST_CI` | yes | yes | yes | default tests skip CI-only files |
| Network/service gating | yes | no | partial | public S3, local MinIO, HTTP fixture, and Google Drive tests exist; other cloud service tests planned |
| Makefile test targets | yes | yes | yes | includes R tests, C API tests, and development benchmark rendering |
| Development MinIO benchmark | yes | n/a | n/a | `benchmarks/minio-paws.Rmd` rendered to GitHub Markdown; ignored from package build |
| GitHub Actions workflow | yes | no local | defined | separate R/C/MinIO/check jobs |
| savvy/roxygen generated wrappers and namespace | yes | yes | yes | `R/000-wrappers.R`, `src/init.c`, `src/rust/api.h`, `NAMESPACE` via `make rd` |
| Rust modules | yes | yes | yes | `aio`, `common`, `error`, `fs`, `http_fixture`, `http_headers`, `io_iter`, `metadata`, `ops`, `path`, `r_values`, `c_api/*` |
| configure / Makevars templates | yes | yes | yes | package tarball installs from generated `src/Makevars`; source builds support Cargo feature selection via `SAVVY_FEATURES` and `--with-rust-features` |

## R API contracts

| Contract | Documented | Implemented | Tested default | Tested CI | Notes |
|---|---:|---:|---:|---:|---|
| Rust-backed filesystem handle `OpendalFs` | yes | yes | yes | yes | local `fs`; no ad hoc `opendalFs` / `abstractFs` mutation |
| `opendal()` / `opendal_uri()` constructors | yes | yes | partial | partial | `opendal("fs", root=)` and public S3 config tested; `opendal_uri()` still needs coverage; HTTP(S) `headers=` supported for explicit request headers |
| declarative `fs_capabilities()` | yes | yes | no | no | shape exists; needs stronger tests |
| path normalization relative to root | yes | yes | yes | yes | escape above root errors |
| errors as values / `opendalErrorValue` | yes | yes | yes | yes | Rust assigns S3 classes; NotFound and AlreadyExists tested |
| vectorized `fs_read()` shape | yes | partial | partial | partial | scalar/range reads, strict mismatch, and read transfer tuning tested |
| strict length matching, no recycling | yes | partial | yes | yes | read/write mismatch tests added; batch write now uses bounded async execution |
| metadata as Rust/C-defined lists | yes | yes | yes | yes | stat/list tested |
| `fs_write()` / `fs_write_aio()` create-only | yes | yes | yes | pending | Rust checks existence before create; accepts batch/write transfer tuning; async local test added |
| `fs_replace()` / `fs_replace_aio()` replacement | yes | yes | yes | pending | local sync and async tests |
| `fs_append()` / `fs_append_aio()` separate append op | yes | partial | no | no | returns unsupported if profile lacks append; async API wired |
| `fs_read_iter()` chunked reads | yes | yes | yes | no | one path returns a seekable/tellable iterator; many paths return a list; local test |
| `OpendalBytes` / `fs_read_bytes()` / `fs_read_bytes_aio()` | yes | yes | yes | no | Rust-owned immutable byte handles; `as.raw()`/`length()` materialization; writes accept handles without rerouting through another R raw vector |
| `fs_write_iter()` chunked writes | yes | yes | yes | no | one path returns a tellable sink; many paths return a list; seek intentionally unsupported; local test |
| `fs_ls_iter()` / `fs_walk_iter()` streaming namespace traversal | yes | yes | yes | no | Rust-backed `OpendalLsIter` with `page_size`, `*_next()`, and `*_collect()`; empty listing, paged root listing, and recursive local walk tested |
| `fs_stat()` / `fs_stats()` / `fs_exists()` and `_aio()` | yes | yes | yes | pending | metadata/existence sync plus async local tests; `fs_stats*` aliases vectorized `fs_stat*`; remote services should use async-first path |
| `fs_ls()` / `fs_ls_aio()` | yes | yes | yes | pending | root entry filtered for local `fs`; public S3 and MinIO listing tested for sync; async local test added; HTTP currently unsupported by OpenDAL |
| `fs_mkdir()` / `fs_delete()` / `fs_copy()` / `fs_rename()` and `_aio()` | yes | yes | yes | pending | direct sync methods tested; async local namespace tests added; MinIO covers S3 copy/delete and unsupported atomic rename error value |
| `serial_config(class, sfunc, ufunc)` / `serialize_raw()` / `deserialize_raw()` / `mode = "serial"` | yes | yes | yes | no | base serialization, custom class envelopes, `opt(fs, "serial")`, sync/Aio read/write materialization, vectorized serial writes, reset via `list()`, and partial-range rejection tested locally |
| `codec_config()` explicit codec layer | provisional | no | no | no | planned; README progress table added |
| explicit credential helpers | yes | partial | yes | yes | S7 `CredentialProvider` with Google Drive direct/gdrive3 providers, redacted print, Rust JSON parsing, and opt-in service test implemented; no hidden env/provider-chain lookup |

## Async R contracts

| Contract | Documented | Implemented | Tested default | Tested CI | Notes |
|---|---:|---:|---:|---:|---|
| generic `OpendalAio` native result future | yes | yes | yes | pending | bytes, unit, bool, metadata, entries, many, error, cancelled outcome family implemented |
| `_aio()` for metadata/namespace operations | yes | yes | yes | pending | stat/stats, exists, ls, mkdir, delete, copy, rename implemented and locally tested |
| `_aio()` for writes | yes | yes | yes | pending | write, replace, append API implemented; local tests cover write/replace |
| active bindings `$value` / `$data` / `$result` / `$state` / `$resolved` / `$error` | yes | yes | partial | pending | generated Aio wrappers are decorated with read-only active bindings; post-resolution behavior tested by default, deterministic pending behavior tested with delayed local HTTP fixture |
| `unresolved()` | yes | yes | partial | pending | no-arg sentinel plus `unresolved(aio)` / `unresolved(value)` predicate; deterministic pending predicates tested with delayed local HTTP fixture |
| `call_aio()` / `collect_aio()` | yes | yes | yes | pending | `call_aio()` waits/updates and returns aio invisibly, including delayed pending HTTP fixture Aio; `collect_aio()` returns value |
| `stop_aio()` cancellation | yes | partial | no | no | cancel path exists; needs race tests |
| condition variables `cv_*` | yes | C-only partial | no | header only | planned for R |
| `aio_monitor()` / `read_monitor()` | yes | no | no | no | planned |
| `race_aio()` | yes | no | no | no | planned |

## C API contracts

| Contract | Documented/header | Implemented library | Tested default | Tested CI | Notes |
|---|---:|---:|---:|---:|---|
| pure C header, no `R.h` / `SEXP` | yes | n/a | yes | yes | grep + compile check |
| `struct_size` in public structs | yes | n/a | no | yes | CI contract lint |
| exported C symbols retained in installed library | yes | yes | no | yes | C anchor file references public C API |
| opaque `ropendal_fs_t` / `ropendal_aio_t` | yes | yes | no | yes | roundtrip test |
| `ropendal_fs_open()` | yes | yes | no | yes | local `fs` roundtrip |
| `ropendal_fs_from_uri()` | yes | yes | no | symbol | not exercised yet |
| async `read_aio()` | yes | yes | no | symbol | result bytes not exercised yet |
| async `read_into_aio()` | yes | yes | no | yes | caller buffer roundtrip |
| async `readv_into_aio()` | yes | unsupported stub | no | symbol | planned |
| `write_aio()` create | yes | yes | no | yes | roundtrip |
| `replace_aio()` | yes | yes | no | symbol | planned test |
| `append_aio()` | yes | partial | no | symbol | backend capability dependent |
| `stat_aio()` / `exists_aio()` / `ls_aio()` | yes | yes | no | yes | entry/bool/entries accessors exercised in installed-library roundtrip |
| `cv` primitives | yes | partial | no | symbol | basic alloc/wait/signal exists |
| monitor primitives | yes | unsupported stub | no | symbol | planned |
| per-request `readv` result details | provisional | no | no | no | still needs final structs |

## Next implementation milestones

1. Add prefetch, traversal fanout, limits, and stronger continuation/backpressure semantics for namespace iterators where OpenDAL/service support warrants them.
2. Extend the `OpendalBytes` byte boundary with any needed ALTREP-style optimizations or C API byte-handle accessors.
3. Add service-level concurrency layers and memory/backpressure limits. Per-call batch/read/write/chunk/coalesce tuning, async operations, active Aio bindings, read/write/listing/walking iterators, and `OpendalBytes` handles are now wired through Rust/OpenDAL.
4. Extend serializer/deserializer coverage and ergonomics where needed; `serial_config()`, `serialize_raw()`, `deserialize_raw()`, and `mode = "serial"` are implemented with R-thread-only hooks.
5. Implement native byte codecs as R-free byte transforms where useful, keeping them separable from serializers and shareable with the C API.
6. Bring native C API parity up to the async operation contract: `readv_into_aio()`, per-request result details, and broader service tests.
7. Finalize the S7 credential-provider contract, and decide whether to add an `s7contract` interface/trait layer for third-party providers.
8. Expand capability tests by service profile and return classed capability values.
9. Expand credential helpers beyond Google Drive and add more service coverage.
10. Add richer service-level async tests for metadata/namespace Aios on opt-in remote backends.

## Deferred milestones

- Additional real-service integration tests beyond current public S3 and Google Drive secret-backed coverage.
- Provider-chain/credential-store plugins.
- Advanced codec auto-selection.
- Browser/webR/wasm constraints.
- Downstream C consumer example package.
