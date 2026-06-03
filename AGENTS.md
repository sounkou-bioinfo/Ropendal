# Ropendal agent notes

## Project goal

Build an R package exposing Apache OpenDAL as an abstract filesystem for R. The package should provide:

- byte-first filesystem primitives backed by OpenDAL (`read`, create/write, replace, append, stat, directory enumeration, copy, rename, delete, mkdir)
- nanonext-like asynchronous R objects (`*Aio`) with active bindings, `call_aio()`, `collect_aio()`, `unresolved()`, `stop_aio()`, and race/wait helpers
- pluggable serializers/deserializers that turn R objects into raw vectors and raw vectors back into R objects; keep both sides explicit in the API
- a pure C API that is async-first and permits native packages to read/write directly into caller-owned buffers without routing through R raw vectors or R's C API
- ergonomic credential/auth helpers, including Google Drive support

## Local references

Reference checkouts must not be committed. If needed, keep them outside the repository or in an ignored local `.sync/` directory.

## API principles

1. The core abstraction is a filesystem handle plus normalized paths relative to its root.
2. The bottom layer moves bytes. Object serialization is a configurable layer above bytes.
3. Avoid exporting `list()` or `$list`; use `ls`, `dir`, `walk`, or package-prefixed names such as `fs_ls()`.
4. Prefer S3 objects/generics and external pointers over R6. R code should be very thin.
5. Async work must run in Rust/OpenDAL/Tokio. Background tasks must not call the R API.
6. R closures for serializers/deserializers run only on the R main thread before async upload or after async download completion. `serial_config(class, sfunc, ufunc)` must expose both nanonext-like sides; `ufunc` is the deserializer.
7. The C API is pure C and async-first. Synchronous behavior should be implemented by waiting on an async handle.
8. Prefer errors-as-values for primitive resolution ergonomics, nanonext-like: Aios and sync primitives resolve to values or `opendalErrorValue`, not surprise throws. Hard errors may still throw for invalid arguments, serializer/deserializer exceptions, unsafe pointer misuse, internal bugs, or allocation failures.
9. Expose declarative capabilities explicitly. Do not add user-facing knobs for internal composition strategy. If Ropendal declares a capability, implement it in Rust/C via OpenDAL, layers, or adapters; otherwise return `opendalUnsupportedValue`.
10. Keep concurrency knobs explicit and separated: runtime threads, global in-flight limit, batch concurrency, and per-object read/write chunk concurrency are different controls.
11. Secrets must be redacted in print/format and should not be stored in package metadata, README outputs, or test fixtures.
12. When an API ambiguity is discovered, update `design/refinement-log.md` with status and provisional resolution.
13. When implementation or test coverage changes, update `design/STATUS.md`.
14. Avoid bogus internal R helpers named like `.something()` when they only hide one line, duplicate Rust validation, or create fake abstraction. Prefer direct public wrappers, S7 classes/generics for real R-level interfaces, and Rust/savvy methods for operation logic.

## Naming conventions

- Filesystem handle class: Rust-backed generated `OpendalFs`; do not add ad hoc `opendalFs` / `abstractFs` S3 classes
- Async object base class: Rust-backed generated `OpendalAio`
- Async operation-specific classes must come from Rust-backed/generated objects, not R-side `class<-` mutation
- Unresolved sentinel class: `unresolvedValue`
- Error scalar/class for non-throwing async results: `opendalErrorValue`

Preferred public R functions:

- constructors: `opendal()`, `opendal_uri()`
- byte operations: vectorized `fs_read()`, `fs_write()` create, `fs_replace()` overwrite/replace, `fs_append()` append; do not add public `fs_read_range()`/`fs_readv()` unless a lower-level primitive proves necessary
- async operations: vectorized `fs_read_aio()`, `fs_write_aio()`, `fs_stat_aio()`, `fs_ls_aio()`
- async controls: `call_aio()`, `call_aio_()`, `collect_aio()`, `collect_aio_()`, `unresolved()`, `stop_aio()`, `race_aio()`, `cv()`, `cv_wait()`, `cv_until()`, `aio_monitor()`
- config: `opt()`, `opt<-`, `serial_config(class, sfunc, ufunc)`, `codec_config(name, class, sfunc, ufunc)`, `credentials_*()` / `auth_*()`

## Build/development workflow

Follow the R package workflow. Use the Makefile once present:

- `make rd` for roxygen2 documentation, generated `NAMESPACE`, and savvy wrapper refresh; do not hand-edit generated Rd/wrapper/namespace files
- `make rdm` to render `README.Rmd` to `README.md`; never edit `README.md` manually
- `make dev-install` for fast local install
- `make test-fast` for quick non-network tinytest iteration
- `make test` for build/install/tinytest
- `make test-network` for opt-in network tests
- `make test-http` for opt-in local HTTP fixture tests; uses the internal Rust HTTP fixture
- `make test-gdrive` for opt-in Google Drive tests
- `make site` for pkgdown

Tinytest infrastructure lives in `tests/tinytest.R` and `inst/tinytest/`. Keep helper code in `inst/tinytest/helper-ropendal.R`. See `design/testing-plan.md` for the test matrix and required env vars. See `design/STATUS.md` for the implementation/test checklist.

Keep `NAMESPACE`, generated wrappers, Rd files, and `README.md` synchronized with sources. Do not hand-edit generated files when roxygen2, savvy, or R Markdown owns them. Edit `README.Rmd`, then run `make rdm`.

Update `NEWS.md` for user-facing changes. Keep newest release/development sections first, write concise user-facing bullets, and do not include secrets, local paths, CI-only noise, or exhaustive internal implementation details.
