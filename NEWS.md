# Ropendal News

<!--
Update this file for user-facing changes before release.

Guidelines:
- Keep entries in reverse chronological order: newest release/development
  section first.
- Use a heading for each version, e.g. `## Ropendal 0.0.2`.
- Keep bullets concise and user-facing. Prefer `Added`, `Changed`, `Fixed`,
  `Deprecated`, `Removed`, or `Documentation` phrasing when useful.
- Mention API changes, behavior changes, new backends, important bug fixes,
  documentation improvements, and migration notes.
- Do not include secrets, local paths, CI-only noise, or exhaustive internal
  implementation details.
-->

## Ropendal 0.0.1.9000

- Development version after 0.0.1.

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
