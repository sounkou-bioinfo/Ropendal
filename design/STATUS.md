# Ropendal implementation and test checklist

Status labels:

- `contract`: API/design contract documented, no implementation yet
- `implemented`: implementation exists
- `tested-default`: covered by default non-network tinytest / R CMD check
- `tested-ci`: covered by CI-only tests (`ROPENDAL_TEST_CI=true`)
- `planned`: not started

## Current state summary

Ropendal currently has a working implementation through the Apache OpenDAL Rust crate and savvy. Rust code is split into modules, the public R layer is thin and delegates operation logic to generated Rust-backed methods, error values are classed in Rust, local byte operations plus read/write/listing/walking iterators and `OpendalBytes` byte handles are tested, native gzip/zlib byte codecs are wired as explicit raw-byte transforms for R and C consumers, public S3-compatible, HTTP, and Google Drive opt-in service tests pass, HTTP(S) filesystems support explicit request headers, and the pure C API has compiled roundtrip coverage for byte, read-vector, codec, metadata/existence, listing, and namespace operations against the installed native library.

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
| `opendal()` / `opendal_uri()` constructors | yes | yes | yes | partial | `opendal("fs", root=)`, local `opendal_uri("fs://...")`, and public S3 config tested; HTTP(S) `headers=` supported for explicit request headers |
| declarative `fs_capabilities()` | yes | yes | partial | partial | classed Rust-built capability values; local `fs` shape/read/list support tested by default; HTTP fixture and S3 public/MinIO service profile checks added |
| path normalization relative to root | yes | yes | yes | yes | escape above root errors |
| errors as values / `opendalErrorValue` | yes | yes | yes | yes | Rust assigns S3 classes; NotFound and AlreadyExists tested |
| vectorized `fs_read()` shape | yes | partial | partial | partial | scalar/range reads, strict mismatch, and read transfer tuning tested |
| strict length matching, no recycling | yes | partial | yes | yes | read/write mismatch tests added; batch write now uses bounded async execution |
| metadata as Rust/C-defined lists | yes | yes | yes | yes | stat/list tested |
| `fs_write()` / `fs_write_aio()` create-only | yes | yes | yes | yes | Rust checks existence before create; accepts batch/write transfer tuning; async local and MinIO tests |
| `fs_replace()` / `fs_replace_aio()` replacement | yes | yes | yes | yes | local sync/async and MinIO async tests |
| `fs_append()` / `fs_append_aio()` separate append op | yes | partial | no | yes | local `fs` roundtrip in installed C API; R async API wired; broader backend capability coverage remains service-dependent |
| `fs_read_iter()` chunked reads | yes | yes | yes | no | one path returns a seekable/tellable iterator; many paths return a list; local test |
| `OpendalBytes` / `fs_read_bytes()` / `fs_read_bytes_aio()` | yes | yes | yes | no | Rust-owned immutable byte handles; `as.raw()`/`length()` materialization; writes accept handles without rerouting through another R raw vector |
| `fs_write_iter()` chunked writes | yes | yes | yes | no | one path returns a tellable sink; many paths return a list; seek intentionally unsupported; local test |
| `fs_ls_iter()` / `fs_walk_iter()` streaming namespace traversal | yes | yes | yes | no | Rust-backed `OpendalLsIter` with `page_size`, `limit`, `start_after`, `*_next()`, and `*_collect()`; empty listing, paged/limited root listing, continuation filtering, and recursive local walk tested |
| `fs_stat()` / `fs_stats()` / `fs_exists()` and `_aio()` | yes | yes | yes | partial | metadata/existence sync plus async local, HTTP fixture, public S3, and MinIO tests; `fs_stats*` aliases vectorized `fs_stat*` |
| `fs_ls()` / `fs_ls_aio()` | yes | yes | yes | partial | root entry filtered for local `fs`; `limit`/`start_after` wired through OpenDAL list options and client filtering; sync and async listing tested for local, public S3, and MinIO; HTTP unsupported value tested sync and async |
| `fs_mkdir()` / `fs_delete()` / `fs_copy()` / `fs_rename()` and `_aio()` | yes | yes | yes | partial | direct sync methods tested; async local namespace tests; MinIO covers S3 copy/delete and unsupported/supported atomic rename outcomes sync and async |
| `serial_config(class, sfunc, ufunc)` / `serialize_raw()` / `deserialize_raw()` / `mode = "serial"` | yes | yes | yes | no | base serialization, custom class envelopes, `opt(fs, "serial")`, sync/Aio read/write materialization, vectorized serial writes, reset via `list()`, and partial-range rejection tested locally |
| `codec_config()` / `codec =` explicit native byte-codec layer | yes | partial | yes | yes | `identity`, `gzip`, and `zlib` native transforms; sync/Aio raw and serial+codec local roundtrips; C API byte-handle encode/decode roundtrip |
| explicit credential helpers | yes | partial | yes | yes | S7 `CredentialProvider` with S3, GCS, AzBlob, and Google Drive direct/gdrive3 providers, redacted print, Rust JSON parsing, and opt-in service test implemented; no hidden env/provider-chain lookup |

## Async R contracts

| Contract | Documented | Implemented | Tested default | Tested CI | Notes |
|---|---:|---:|---:|---:|---|
| generic `OpendalAio` native result future | yes | yes | yes | pending | bytes, unit, bool, metadata, entries, many, error, cancelled outcome family implemented |
| `_aio()` for metadata/namespace operations | yes | yes | yes | partial | stat/stats, exists, ls, mkdir, delete, copy, rename implemented; local plus HTTP/S3/MinIO service coverage added where supported |
| `_aio()` for writes | yes | yes | yes | partial | write, replace, append API implemented; local and MinIO cover write/replace; append remains backend-dependent |
| active bindings `$value` / `$data` / `$result` / `$state` / `$resolved` / `$error` | yes | yes | partial | pending | generated Aio wrappers are decorated with read-only active bindings; post-resolution behavior tested by default, deterministic pending behavior tested with delayed local HTTP fixture |
| `unresolved()` | yes | yes | partial | pending | no-arg sentinel plus `unresolved(aio)` / `unresolved(value)` predicate; deterministic pending predicates tested with delayed local HTTP fixture |
| `call_aio()` / `collect_aio()` | yes | yes | yes | pending | `call_aio()` waits/updates and returns aio invisibly, including delayed pending HTTP fixture Aio; `collect_aio()` returns value |
| `stop_aio()` / C Aio cancellation | yes | partial | no | yes | delayed HTTP fixture covers R pending-read cancellation; installed C roundtrip covers `ropendal_aio_cancel()`; broader race/service coverage still useful |
| condition variables `cv_*` | yes | R polling + native C | yes | yes | R `cv()`, `cv_value()`, `cv_reset()`, `cv_signal()`, `cv_wait()`, and `cv_until()` implemented; C cv lifecycle and `ropendal_aio_notify()` covered |
| `aio_monitor()` / `read_monitor()` | yes | yes | yes | partial | R monitor drains Aio completion events without materializing success payloads; C monitor queues native completion events |
| `race_aio()` | yes | yes | yes | no | returns first completion event plus Aio handle; timeout returns `unresolvedValue` |

## C API contracts

| Contract | Documented/header | Implemented library | Tested default | Tested CI | Notes |
|---|---:|---:|---:|---:|---|
| pure C header, no `R.h` / `SEXP` | yes | n/a | yes | yes | grep + compile check |
| `struct_size` in public structs | yes | n/a | no | yes | CI contract lint |
| exported C symbols retained in installed library | yes | yes | no | yes | C anchor file references public C API |
| opaque `ropendal_fs_t` / `ropendal_aio_t` | yes | yes | no | yes | roundtrip test |
| `ropendal_fs_open()` | yes | yes | no | yes | local `fs` roundtrip |
| `ropendal_fs_from_uri()` | yes | yes | no | yes | local `fs://` roundtrip |
| async `read_aio()` | yes | yes | no | yes | borrowed byte result roundtrip |
| async `read_into_aio()` | yes | yes | no | yes | caller buffer roundtrip |
| async `readv_aio()` | yes | yes | no | yes | installed-library roundtrip returns flattened borrowed bytes plus per-request success/failure details |
| async `readv_into_aio()` | yes | yes | no | yes | installed-library roundtrip fills multiple caller-owned range buffers and checks per-request success/failure result details |
| `write_aio()` create | yes | yes | no | yes | roundtrip |
| `replace_aio()` | yes | yes | no | yes | local `fs` roundtrip |
| `append_aio()` | yes | partial | no | yes | local `fs` roundtrip; broader backend capability coverage remains service-dependent |
| `stat_aio()` / `exists_aio()` / `ls_aio()` | yes | yes | no | yes | entry/bool/entries accessors exercised in installed-library roundtrip |
| `cv` primitives | yes | yes | no | yes | installed C roundtrip covers alloc/value/signal/reset/timed wait plus Aio notify |
| monitor primitives | yes | yes | no | yes | installed C roundtrip covers monitor create/add/read/release with retained Aio/CV lifetimes |
| per-request read-vector result details | yes | yes | no | yes | `ropendal_readv_result_t` plus `ropendal_aio_result_readv()` for both `readv_aio()` and `readv_into_aio()` |
| native byte codecs / byte handles | yes | yes | no | yes | `ropendal_codec_encode()`, `ropendal_codec_decode()`, `ropendal_bytes_data()`, `ropendal_bytes_len()`, and `ropendal_bytes_release()` roundtrip gzip bytes without R API |

## Next implementation milestones

1. Add prefetch, traversal fanout, and stronger continuation/backpressure semantics for namespace iterators where OpenDAL/service support warrants them; `limit` and `start_after` controls are implemented for listing collection and iterators.
2. Extend the `OpendalBytes` byte boundary with any needed ALTREP-style optimizations; C API byte-handle accessors are implemented.
3. Add remaining memory/backpressure limits and any additional service-level layers. `runtime_config(threads=)`, `layer_concurrent_limit(max=)`, per-call batch/read/write/chunk/coalesce tuning, async operations, active Aio bindings, read/write/listing/walking iterators, and `OpendalBytes` handles are now wired through Rust/OpenDAL.
4. Extend serializer/deserializer coverage and ergonomics where needed; `serial_config()`, `serialize_raw()`, `deserialize_raw()`, and `mode = "serial"` are implemented with R-thread-only hooks.
5. Extend native byte codecs beyond explicit `identity`/`gzip`/`zlib` where useful and add async/background codec composition only where it preserves the R-thread boundary.
6. Broaden native C API remote-service and cancellation-race coverage now that byte, metadata, namespace, codec, CV, and monitor primitives are implemented.
7. Finalize the S7 credential-provider contract, and decide whether to add an `s7contract` interface/trait layer for third-party providers.
8. Expand capability tests for additional service profiles and consider higher-level capability interfaces for consumers.
9. Add more service coverage for explicit cloud credential helpers and any provider-specific readers that remain opt-in and non-ambient.
10. Add richer service-level async tests for additional providers beyond current HTTP/S3/MinIO coverage.

## Deferred milestones

- Additional real-service integration tests beyond current public S3 and Google Drive secret-backed coverage.
- Provider-chain/credential-store plugins.
- Advanced codec auto-selection.
- Browser/webR/wasm constraints.
- Downstream C consumer example package.
