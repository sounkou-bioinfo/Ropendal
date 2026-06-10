# Ropendal News

## Ropendal 0.0.1.9000

- Development version after 0.0.1.
- Documentation: expanded README examples for batched I/O, async Aio handles,
  serializers/codecs, Arrow IPC streams via nanoarrow, lower-level iterators,
  S3-compatible MinIO benchmarking, and native C API scheduling.
- Documentation: aligned design notes, roxygen docs, and vignettes with the
  current async, credential, serializer, and native C API behavior.
- Documentation: documented R-universe Linux binary repository setup.
- Build: added a webR/wasm package-load fallback and local rwasm build helper.
- Reads: `fs_read()` / `fs_read_bytes()` now support multiple ranges from one
  path and list-of-ranges shapes across multiple paths, while preserving ordered
  flat or nested results. `byte_ranges()` adds a request-object form for
  index-heavy readers.
- Byte stores: `byte_store()` and `store_*()` helpers provide a small
  store-relative key-to-bytes adapter for Zarr-like chunk layouts. `store_read()`
  is now bytes-first for `OpendalBytes`/ALTREP-compatible flows, async `store_*_aio()`
  wrappers are available, and `store_cache()` adds an explicit local full-object
  cache for chunk-key stores.
- API: R-side `batch_concurrency = 0` is now rejected; use `NULL` for the
  default. Capability rows now include range/concurrency/recursive semantics.
- C API: write tuning options are wired through, unsupported write header
  options reject explicitly, `ropendal_aio_wait()` honors its timeout, and
  prefix-scoped byte stores now expose async read/write/read-into/list/delete
  operations for downstream native code. The C ABI version is now 2.

## Ropendal 0.0.1

- Initial public release.
- Added byte-first filesystem primitives backed by Apache OpenDAL, including
  read, write/create, replace, append, stat, listing, copy, rename, delete, and
  directory creation where supported by the backend.
- Added Aio-based async operations and controls, including `fs_*_aio()`,
  `call_aio()`, `collect_aio()`, unresolved sentinels, condition variables, and
  monitor helpers.
- Added explicit serializer/deserializer and codec layers for materializing R
  objects and transforming raw bytes.
- Added an async-first native C API in `inst/include/ropendal.h` for downstream
  native packages that need direct byte I/O into caller-owned buffers.
- Added local filesystem, HTTP(S), S3-compatible, and Google Drive examples and
  test coverage.
