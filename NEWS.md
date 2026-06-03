# Ropendal News

## Ropendal 0.0.1.9000

- Development version after 0.0.1.
- Documentation: expanded README examples for batched I/O, async Aio handles,
  serializers/codecs, Arrow IPC streams via nanoarrow, lower-level iterators,
  S3-compatible MinIO benchmarking, and native C API scheduling.

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
