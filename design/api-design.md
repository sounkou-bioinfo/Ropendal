# Ropendal API design

## Design center

Ropendal is an abstract filesystem interface for R backed by the Rust crate of Apache OpenDAL. The core value is not a convenience wrapper around one backend; it is an async, capability-aware remote filesystem operation substrate that higher-level R and native packages can build on. Byte I/O is the data plane, but metadata and namespace operations are first-class remote operations too.

The API borrows deliberately from `nanonext`:

- operations that may block have async variants returning Aio objects immediately
- Aio values are exposed through active bindings
- `call_aio()` waits and updates the Aio object
- `collect_aio()` returns the resolved value
- `unresolved()` is suitable for polling/control flow
- serializers are explicit R object-to-byte materializers

The `savvy-async-poc` pattern is useful for ALTREP-backed lazy vectors, but explicit Aio handles are the primary async abstraction.

## Core constitution

Ropendal's stable center is an async, capability-aware remote filesystem
operation substrate over OpenDAL. The core should stay as small as possible:

| Core value | Meaning |
|---|---|
| `OpendalFs` | capability-bearing OpenDAL operator handle plus runtime/config |
| `OpendalAio` | one in-flight or resolved async operation |
| `OpendalBytes` | immutable Rust-owned bytes exposed to R as a handle |
| `NativeResult` | bytes, metadata, entries, booleans, unit completion, errors, cancellation, or many results |
| Request/options | normalized operation description, ranges, concurrency, and transfer tuning |

Everything else is an adapter or materialization layer. R objects, `SEXP` values,
R closures, and serializer/deserializer hooks must not enter Tokio/background
work. Background work operates on owned bytes, metadata, entries, booleans,
unit completion, native codec state, paths, options, and errors only.

The package therefore uses three layers:

1. **Operation future**: Rust/OpenDAL owns async backend work and resolves to a
   native result such as bytes, metadata, entries, bool, unit, many, error, or
   cancellation.
2. **Materializer**: R-thread conversion between native results and R
   representations such as `raw`, `OpendalBytes`, metadata lists, entries,
   logical values, text, serialized R objects, matrix columns, or data frames.
3. **Adapter**: convenience surfaces such as iterators, Aio active bindings,
   future connection wrappers, event-loop integrations, or package-specific
   serializers.

Read/write are intentionally asymmetric at the R boundary. Reads can complete
into Rust-owned bytes first and materialize later. Writes from ordinary R values
must serialize or copy into stable owned bytes before background upload. The
`OpendalBytes` provides the copy-minimized R-facing byte handle: remote bytes can
be read into Rust-owned storage, passed around in R, converted to `raw`
explicitly via `as.raw()`, inspected with `length()`, or written back without
routing through another R raw-vector payload.

R connections are compatibility adapters, not the core abstraction. A future
connection API should wrap read/write iterators so `readBin()`, `writeBin()`,
`serialize(obj, con)`, and `unserialize(con)` can be useful, while preserving the
same operation-future, multipart-finalization, range-read, and provider-error
semantics.

Every operation that may perform backend I/O should have an `_aio()` form:
read/write, stat/exists, listing/walk, mkdir/delete, copy, rename, and append.
`fs_info()`, `fs_capabilities()`, and path normalization can remain synchronous
because they are local handle introspection.

## Avoid `list`

Do not export `list()` or expose `$list`. In R, `list` is too central and should not be overloaded for filesystem enumeration. Use:

- `fs_ls()` for listing/enumeration
- `fs_ls_aio()` for async listing
- `fs_ls(..., recursive = TRUE)` for collected recursive traversal
- `fs_walk_iter()` for streaming recursive traversal
- optional `$ls` sugar if a method table is added later

## R object model

### Filesystem handle

```r
fs <- opendal("s3", bucket = "bucket", root = "prefix", auth = credentials_s3())
fs <- opendal_uri("s3://bucket/prefix?region=us-east-1")
fs <- opendal("gdrive", root = "folder", auth = credentials_gdrive())
http_fs <- opendal("http", endpoint = "https://example.test/data", headers = list(Authorization = "Bearer ..."))
```

Class:

```r
class(fs)
# c("Ropendal::OpendalFs", "OpendalFs", "savvy_Ropendal__sealed")
```

Implementation: a Rust/savvy-generated object wrapping an external pointer. R state is minimal; Rust owns the OpenDAL operator, runtime, operation validation, and result shaping. Do not add ad hoc R-side `opendalFs` / `abstractFs` S3 classes; if a future abstract filesystem hierarchy is needed, make it a real S7 interface/class layer rather than mutating generated objects.

Required native state:

- external pointer to Rust `OpendalFs`
- service scheme
- redacted configuration summary
- optional default serialization config
- optional default operation options

### Aio handles

Aio objects are environments with active bindings, inspired by nanonext. An Aio is a generic remote operation future, not just a read handle.

```r
aio <- fs_stat_aio(fs, "x.bin")
aio
# < opendal aio | $value >

unresolved(aio)
aio$value        # unresolvedValue if pending, otherwise value or errorValue
call_aio(aio)    # wait, update object, return invisibly
collect_aio(aio) # wait and return value
stop_aio(aio)    # cancel if possible
```

Base generated class:

```r
class(aio)
# c("Ropendal::OpendalAio", "OpendalAio", "savvy_Ropendal__sealed")
```

Operation-specific Aio classes, if added later, must be Rust-backed/generated objects rather than R-side `class<-` mutation.

Active bindings:

| Binding | Meaning |
|---|---|
| `$value` | universal resolved value, success value, or error value |
| `$data` | alias for `$value` on value-returning operations such as read/stat/ls/exists |
| `$result` | alias for `$value` on unit/completion operations such as write/delete/copy/rename/mkdir |
| `$state` | pending, ready, resolved, error, or cancelled |
| `$resolved` | logical resolution state |
| `$error` | error value or `NULL` |

`collect_aio()` returns `$value`. Lists of Aios preserve length and names. Operation-specific Aio classes may still exist for printing/documentation, but the value contract is generic.

## Errors as values

Follow nanonext's resolution ergonomics: operations resolve to values, and filesystem/backend failures are represented as classed values rather than surprise conditions.

```r
x <- fs_read(fs, "missing.bin")
is_error_value(x)
error_kind(x)
# "NotFound"
error_path(x)
# "missing.bin"

inherits(x, "opendalNotFoundValue")
# TRUE
```

There is no `errors = "stop"` mode in the primitive API. Users who want condition-like programming can convert/check explicitly:

```r
# Future adapters can provide explicit opt-in throwing helpers, e.g.
# stop_for_error(x) or stop(as_error_condition(x)).
```

Rules:

- Aio active bindings (`$value`, `$data`, `$result`) never throw for backend failures; they return `unresolvedValue`, a success value, or `opendalErrorValue`.
- `collect_aio(aio)` returns the resolved value, including `opendalErrorValue` on failure.
- `call_aio(aio)` waits and returns the Aio invisibly; users inspect `$value`/`$data`/`$result`.
- Synchronous primitive functions also return `opendalErrorValue` for backend/filesystem failures.
- Vectorized operations preserve shape and place error values exactly where the corresponding success value would appear.
- Hard errors still throw: invalid R arguments, impossible result shapes, unsafe pointer misuse, serializer/deserializer exceptions, package internal bugs, and memory/allocation failures.

Error values are small classed lists constructed in Rust:

```r
structure(
  list(
    `__ropendal_error__` = TRUE,
    code = 4L,
    kind = "NotFound",
    message = "object not found",
    operation = "read",
    path = "missing.bin"
  ),
  class = c("opendalNotFoundValue", "opendalErrorValue", "errorValue", "list")
)
```

Kind-specific classes should mirror OpenDAL `ErrorKind` where possible:

- `opendalUnexpectedValue`
- `opendalUnsupportedValue`
- `opendalConfigInvalidValue`
- `opendalNotFoundValue`
- `opendalPermissionDeniedValue`
- `opendalIsADirectoryValue`
- `opendalNotADirectoryValue`
- `opendalAlreadyExistsValue`
- `opendalRateLimitedValue`
- `opendalIsSameFileValue`
- `opendalConditionNotMatchValue`
- `opendalRangeNotSatisfiedValue`

Current public helpers:

```r
is_error_value(x)
error_kind(x)
error_message(x)
error_path(x)
error_operation(x)
```

Future adapters may add explicit conversion helpers such as `as_error_condition()`
or `stop_for_error()`, but primitive functions should continue to resolve
backend/filesystem failures as values. The C API analog is `ropendal_status_t`
plus an optional `ropendal_error_t *` for structured inspection. Async C handles
resolve to ready/error/cancelled states instead of throwing through R.

## Core R API

Use package-prefixed generics. The R layer should be thin wrappers over `.Call()`. Avoid API bloat by making the core verbs vectorized instead of adding `_many`, `_range`, and `_ranges` variants to the public R surface.

```r
fs_info(x)
fs_capabilities(x)
fs_normalize_path(x, path, directory = FALSE)

fs_stat(x, path, batch_concurrency = NULL)
fs_stats(x, path, batch_concurrency = NULL)
fs_exists(x, path, batch_concurrency = NULL)
fs_ls(x, path = "", recursive = FALSE, limit = NULL, start_after = NULL)
fs_ls_iter(x, path = "", recursive = FALSE, page_size = 1000,
           limit = NULL, start_after = NULL, prefetch = 0)
fs_walk_iter(x, path = "", page_size = 1000,
             limit = NULL, start_after = NULL, prefetch = 0)

fs_read(
  x, path,
  offset = 0,
  size = NULL,
  end = NULL,
  result = c("auto", "flat", "nested"),
  batch_concurrency = NULL,
  read_concurrency = NULL,
  chunk_size = NULL,
  coalesce_gap = NULL,
  mode = c("raw", "serial", "text", "codec"),
  encoding = "UTF-8",
  serial_config = opt(x, "serial"),
  codec = opt(x, "codec")
)

fs_write(
  x, path, data,
  batch_concurrency = NULL,
  write_concurrency = NULL,
  chunk_size = NULL,
  mode = c("raw", "serial", "text", "codec"),
  encoding = "UTF-8",
  serial_config = opt(x, "serial"),
  codec = opt(x, "codec")
)
fs_replace(x, path, data, batch_concurrency = NULL,
           write_concurrency = NULL, chunk_size = NULL,
           mode = c("raw", "serial", "text", "codec"), ...)
fs_append(x, path, data, batch_concurrency = NULL,
          write_concurrency = NULL, chunk_size = NULL,
          mode = c("raw", "serial", "text", "codec"), ...)

fs_mkdir(x, path)
fs_delete(x, path, recursive = FALSE, batch_concurrency = NULL)
fs_copy(x, from, to)
fs_rename(x, from, to)
```

Write semantics:

- `fs_write()` creates a new file/object and returns `opendalAlreadyExistsValue` if the target exists.
- `fs_replace()` is the explicit replacement/overwrite operation.
- `fs_append()` is a separate append operation and is supported only when the declared profile supports append.
- Directory hierarchy creation is a directory operation, not part of file write semantics. Use `fs_mkdir()` / directory APIs.
- User-supplied `path` and `data` vectors/lists use strict length matching; no implicit recycling of payloads.

Async variants use the same vectorized arguments and return one Aio handle for the whole submitted operation. The synchronous forms should be blocking convenience wrappers over the same native operation pipeline, even if the implementation avoids literally constructing a public Aio object.

```r
fs_stat_aio(x, path, batch_concurrency = NULL)
fs_stats_aio(x, path, batch_concurrency = NULL)
fs_exists_aio(x, path, batch_concurrency = NULL)
fs_ls_aio(x, path = "", recursive = FALSE, limit = NULL, start_after = NULL)
fs_read_aio(x, path, offset = 0, size = NULL, end = NULL,
            result = c("auto", "flat", "nested"), batch_concurrency = NULL,
            read_concurrency = NULL, chunk_size = NULL, coalesce_gap = NULL,
            mode = c("raw", "serial", "text", "codec"), ...)
fs_write_aio(x, path, data, batch_concurrency = NULL,
             write_concurrency = NULL, chunk_size = NULL,
             mode = c("raw", "serial", "text", "codec"), ...)
fs_replace_aio(x, path, data, batch_concurrency = NULL,
               write_concurrency = NULL, chunk_size = NULL,
               mode = c("raw", "serial", "text", "codec"), ...)
fs_append_aio(x, path, data, batch_concurrency = NULL,
              write_concurrency = NULL, chunk_size = NULL,
              mode = c("raw", "serial", "text", "codec"), ...)
fs_mkdir_aio(x, path, batch_concurrency = NULL)
fs_delete_aio(x, path, recursive = FALSE, batch_concurrency = NULL)
fs_copy_aio(x, from, to, batch_concurrency = NULL)
fs_rename_aio(x, from, to, batch_concurrency = NULL)
```

Operation taxonomy:

| Plane | Operations | Result family |
|---|---|---|
| Data | read, write, replace, append | bytes or unit |
| Metadata | stat, exists | metadata or bool |
| Namespace | ls, walk, mkdir, delete, copy, rename | entries or unit |
| Materialization/adapters | raw, bytes handle, text, serial, codec, entries data frame, connections | R values over native results |

## Vectorized range reads

Range reads are first-class, but not as separate public verbs. They are the vectorized form of `fs_read()`.

Examples:

```r
# Whole object: scalar path, default offset/size.
x <- fs_read(fs, "x.bin")

# One range from one object.
x <- fs_read(fs, "x.bin", offset = 100, size = 50)

# Many ranges from one object: returns a list of raw vectors.
xs <- fs_read(fs, "x.bin", offset = c(0, 4096, 8192), size = c(512, 512, 512))

# Many paths, one full read each.
xs <- fs_read(fs, c("a.bin", "b.bin", "c.bin"))

# Many paths, with one or more ranges per path.
# `offset` and `size` are lists aligned with `path`.
xs <- fs_read(fs,
  path = c("a.bin", "b.bin"),
  offset = list(c(0, 4096), c(0)),
  size = list(c(100, 100), c(200))
)

# Fully flat request-table helpers may be added later.
# For now, use vector/list arguments and `result = "flat"` when a flat
# return shape is desired.
```

Semantics:

- no R recycling for user-supplied vectors/lists; lengths must match the required shape exactly, except for documented defaults
- offsets are zero-based bytes
- ranges are half-open: `[offset, end)`
- specify either `size` or `end`, not both
- `size` is a byte count; `end = offset + size`
- `size = NULL` and `end = NULL` means `offset..EOF`
- scalar path plus scalar range returns one raw vector in `result = "auto"`
- scalar path plus many numeric ranges returns a flat list in `result = "auto"`
- multiple paths with list `offset`/`size` return a nested list in `result = "auto"`: one element per path, then one raw vector per requested range
- flat request objects/data frames, if added later, return a flat list in `result = "auto"`: one raw vector per row/request
- `result = "flat"` always returns a flat list of raw vectors, one per expanded request
- `result = "nested"` always returns a list aligned with input paths, with per-path lists of range results
- `coalesce_gap` may merge nearby backend requests internally, but the returned object is split into the originally requested ranges
- for partial ranges, `mode = "serial"` should be rejected unless the caller explicitly opts into reading a complete serialized object; byte range reads are fundamentally `mode = "raw"`

A small request-table constructor can exist later for clarity, but it should feed `fs_read()` rather than create another verb. Directory paths are for `fs_ls(recursive = TRUE)` and `fs_walk_iter()`; byte ranges apply to file reads.

The native C API remains more explicit (`read_aio`, `read_into_aio`, `readv_into_aio`) because C callers need direct buffer ownership and cannot rely on R vector/list recycling.

## Listing and walking

Listing is a first-class remote operation. On object stores and Google Drive-like
services it can be paginated, rate-limited, recursive, and much larger than a
single materialized R list. Provide both collectable and streaming forms:

```r
aio <- fs_ls_aio(fs, "prefix/", recursive = TRUE)
entries <- collect_aio(aio)

it <- fs_ls_iter(fs, "prefix/", recursive = TRUE, page_size = 1000, prefetch = 2000)
page <- ls_iter_next(it)       # list(done = FALSE, entries = list(...), cursor = "last/path")
entries <- ls_iter_collect(it) # collect remaining pages

walk <- fs_walk_iter(fs, "prefix/", page_size = 1000, prefetch = 2000)
page <- walk_iter_next(walk)
```

Semantics:

- `fs_ls()` / `fs_ls_aio()` collect a finite listing into an entries value or
  list of entries values.
- `fs_ls_iter()` pages a listing and provides caller-driven backpressure.
- `fs_walk_iter()` recursively traverses and streams pages.
- Current iterator pages return `list(done, entries, cursor)` and expose `*_next()` plus `*_collect()`.
- `cursor` is the last yielded root-relative path. It can be used as a best-effort `start_after` marker for lexically ordered listings, but it is not an opaque backend continuation token and exact restart semantics are backend/order dependent.
- `page_size` bounds the number of R entries yielded per iterator page and is also passed as an OpenDAL backend request-size hint where possible.
- `limit` bounds materialized results for collectable listing APIs and total entries yielded by listing/walk iterators.
- `start_after` is a root-relative continuation marker passed through OpenDAL list options and also enforced by client-side filtering.
- `prefetch` is an explicit bounded entry-buffer depth for iterators. `0` disables background prefetch; positive values may start Rust/Tokio read-ahead at iterator construction without calling R.
- Future `batch_concurrency` controls many independent roots/prefixes.
- Future `list_concurrency` controls recursive traversal fanout where implemented.

A lightweight `opendalEntries` value can wrap returned entries with
`as.list()`/`as.data.frame()` adapters. Primitive errors remain per-root/per-page
error values rather than conditions.

## Path and metadata semantics

All operation paths are relative to the filesystem root configured at construction time. Ropendal normalizes logical paths before dispatch:

- collapse repeated slashes
- remove `.` components
- resolve `..` components without allowing escape above root
- normalize directory paths to trailing `/` for directory APIs
- return directory entries with trailing `/`; file entries do not have trailing `/`

Callers are responsible for requesting semantically correct paths. Ropendal path identity is the normalized OpenDAL logical path.

Metadata shape is defined at the Rust/C layer and returned to R as generic lists. Required common fields where available:

```r
list(
  path = "dir/file.bin",
  type = "file",
  size = 1234,
  etag = "...",
  last_modified = as.POSIXct(...),
  version = "..."
)
```

Profiles may add fields. The primitive API returns lists because metadata is service-dependent; tabular helpers such as `as.data.frame()` can be layered on top.

## Declarative capabilities and adapters

The user-facing API should be uniform. Users should not choose between backend-native operations and adapter-composed operations. Ropendal controls the implementation.

Ropendal should declare an effective capability set for each filesystem handle:

```r
fs_capabilities(fs)
```

This is the Ropendal capability contract, not just a raw dump of backend flags. It can include operations implemented by OpenDAL directly, OpenDAL layers, or Ropendal Rust/C adapters. If Ropendal declares `stat = TRUE` for a service, `fs_stat()` must work through the adapter. If it cannot be made to work with honest semantics, declare `stat = FALSE` and return `opendalUnsupportedValue` when called.

Suggested capability table columns:

```r
operation        # "read", "read_range", "write", "stat", "ls", "copy", ...
supported        # TRUE/FALSE
implementation   # "native", "opendal_layer", "ropendal_adapter"
semantics        # short stable label: "atomic", "best_effort", "eventual", etc.
notes            # human-readable diagnostics, no secrets
```

Principles:

- No public knobs for selecting internal composition strategy.
- Internal emulation/composition belongs in Rust/C adapters and is selected declaratively by the capability profile.
- If an operation is unsupported by the declared profile, return a meaningful error value such as `opendalUnsupportedValue`.
- Never silently weaken requested conditions, version IDs, metadata requirements, or consistency semantics. If the adapter cannot honor the request, return an error value.
- Warnings may be emitted for diagnostics when an adapter has surprising cost or weaker backend semantics, but warnings are not part of control flow.
- `fs_info()` should expose the service, root, layers/adapters, and redacted configuration summary so users can understand which profile they are using.

Example policy:

| Operation | Declarative rule |
|---|---|
| `fs_exists()` | Ropendal helper defined via `stat`; supported if the profile supports enough metadata/error semantics to distinguish not-found from other failures. |
| `fs_read()` whole object | Supported if profile declares `read`. |
| `fs_read()` byte ranges | Supported if profile declares `read_range`; implementation may be native or adapter-owned, but R API is identical. |
| `fs_write()` | Supported if profile declares `write`. |
| `fs_stat()` | Supported if profile declares `stat`; adapters must provide the documented metadata shape. |
| `fs_ls()` / `fs_walk_iter()` | Supported if profile declares listing/recursive traversal support; adapter may perform pagination or iterative traversal internally. |
| `fs_delete()` | Supported if profile declares `delete`; recursive delete requires `delete_recursive`. |
| `fs_copy()` | Supported if profile declares `copy`; if implemented by adapter composition, that is not a user option. |
| `fs_rename()` | Supported if profile declares `rename`; if non-atomic, the capability table must say so. |
| `fs_mkdir()` | Supported if profile declares `mkdir`; directory-marker semantics must be documented per profile. |

This keeps the public API ergonomic and primitive-oriented while still allowing sophisticated service-specific adapters in Rust/C.

## Concurrency and performance controls

Concurrency and deadlines belong in the API, but controls must be explicit about *which* resource is being controlled. There are several different levels:

| Level | R option | Applies to | Meaning |
|---|---|---|---|
| Runtime workers | `runtime_config(threads = )` | an `OpendalFs` handle | Tokio worker threads used to drive async work. This is not a per-read fanout knob. |
| Global in-flight limit | `layer_concurrent_limit(max = )` | an `OpendalFs` handle | Throttle total backend requests for rate limits, politeness, or memory control. |
| Operation/I/O deadlines | `layer_timeout(request_timeout = , io_timeout = )` | an `OpendalFs` handle | Service-wide backend non-streaming operation and streaming I/O timeouts in seconds; listing iteration is governed by `io_timeout`; if one side is omitted, OpenDAL's timeout-layer default applies. |
| Batch concurrency | `batch_concurrency = ` | many paths/ranges | Number of independent operations in flight, e.g. many small reads/stat calls. |
| Per-object read/write concurrency | `read_concurrency = ` / `write_concurrency = ` | one large object | Number of chunks/parts fetched or uploaded concurrently for a single object where OpenDAL/backend supports it. |

Use `NULL` to mean "do not override; let Ropendal/OpenDAL choose". Use `1L` to force serial execution at that level. Use values `> 1L` for explicit fanout.

Examples:

```r
# One large object: split into 8 chunk reads if supported by the backend.
fs_read(fs, "big.bin", read_concurrency = 8, chunk_size = 8 * 1024^2)

# Many independent objects: at most 32 object reads in flight.
fs_read(fs, paths, batch_concurrency = 32, read_concurrency = 1)

# Indexed/range-heavy readers: many ranges, with nearby ranges coalesced.
fs_read(fs,
  path = index$path,
  offset = as.list(index$offset),
  size = as.list(index$size),
  result = "flat",
  batch_concurrency = 64,
  read_concurrency = 4,
  chunk_size = 4 * 1024^2,
  coalesce_gap = 64 * 1024
)

# Service-wide throttle/deadlines to avoid hammering or hanging on an API.
fs <- opendal("gdrive", auth = credentials_gdrive(),
              layers = list(
                layer_concurrent_limit(8),
                layer_timeout(request_timeout = 30, io_timeout = 10)
              ))
```

Guidance:

- Local filesystems usually do not benefit from high `read_concurrency`; object stores often do.
- `batch_concurrency` helps many small files; `read_concurrency` helps one large file.
- `chunk_size` controls memory/request size tradeoffs.
- `coalesce_gap` trades extra bytes transferred for fewer range requests.
- `prefetch` is for streaming readers: higher values can improve throughput but buffer more memory.
- Backend rate limits matter. Google Drive and similar APIs should default to conservative global limits and let users raise them.

## Operation options

Read options:

```r
fs_read(
  fs, "x.bin",
  offset = 0,
  size = NULL,
  content_length_hint = NULL,
  version = NULL,
  if_match = NULL,
  if_none_match = NULL,
  if_modified_since = NULL,
  if_unmodified_since = NULL,
  read_concurrency = NULL,
  chunk_size = NULL,
  coalesce_gap = NULL,
  prefetch = NULL,
  mode = "raw",
  codec = NULL
)
```

Write options:

```r
fs_write(
  fs, "x.bin", data,
  append = FALSE,
  if_match = NULL,
  if_none_match = NULL,
  if_not_exists = FALSE,
  content_type = NULL,
  content_encoding = NULL,
  content_disposition = NULL,
  cache_control = NULL,
  user_metadata = NULL,
  write_concurrency = NULL,
  chunk_size = NULL,
  mode = "raw",
  codec = NULL
)
```

List options:

```r
fs_ls(fs, path = "", recursive = FALSE, limit = NULL, start_after = NULL,
      versions = FALSE, deleted = FALSE)
```

Delete options:

```r
fs_delete(fs, path, recursive = FALSE, version = NULL)
```

## Serialization, deserialization, and codecs

The filesystem core moves bytes. Object support is a materialization layer above bytes and must be symmetric: every custom serializer has a matching deserializer. Keep the names precise:

- **serializers** convert R objects to/from bytes and may touch the R API;
- **codecs** transform bytes to/from bytes and can be native/background-safe when implemented without R callbacks.

### Native modes

```r
fs_write(fs, "x.bin", raw_vec, mode = "raw")
fs_read(fs, "x.bin", mode = "raw")

fs_write(fs, "x.txt", "hello", mode = "text")
fs_read(fs, "x.txt", mode = "text", encoding = "UTF-8")

fs_write(fs, "x.rds", object, mode = "serial")
fs_read(fs, "x.rds", mode = "serial")
```

`mode = "serial"` defaults to base R `serialize()` / `unserialize()` if no configured custom hook matches.

### nanonext-like serialization config

Mirror the relevant nanonext ergonomics at the user level: `serial_config()` registers both serialize and unserialize hooks.

```r
cfg <- serial_config(
  class = "ArrowTabular",
  sfunc = function(x) arrow::write_ipc_stream(x, raw()),
  ufunc = function(x) arrow::read_ipc_stream(x)
)

opt(fs, "serial") <- cfg
```

Multiple classes use parallel vectors/lists, as in nanonext:

```r
cfg <- serial_config(
  class = c("torch_tensor", "ArrowTabular"),
  sfunc = list(serialize_torch, serialize_arrow),
  ufunc = list(unserialize_torch, unserialize_arrow)
)
```

Rules:

- `sfunc(object)` must return bytes (`raw` initially; later also `OpendalBytes` if useful).
- `ufunc(raw_or_bytes)` is the deserializer and returns an R object.
- R serializer and deserializer closures run only on the R thread.
- `fs_write_aio(..., mode = "serial")` serializes on the R thread before submitting the async upload.
- For vectorized serial writes, serialize one item on the R thread and submit its upload before serializing the next, so later R-thread serialization can overlap earlier background byte uploads without moving R objects into worker tasks.
- `fs_read_aio(..., mode = "serial")` downloads asynchronously; `$data`/`collect_aio()` deserializes on the R thread after bytes resolve.
- Passing `list()` to `opt(fs, "serial") <- list()` removes custom hooks, matching nanonext.

Implementation: for `mode = "serial"`, Ropendal uses an R serialization envelope with custom-hook payload bytes, not bare codec bytes. This lets the deserializer be chosen from the serialized stream, provided the same `serial_config()` is available at read time.

Expose explicit helpers for debugging and for users who want the conversion layer without I/O:

```r
serialize_raw(x, config = opt(fs, "serial"))
deserialize_raw(raw, config = opt(fs, "serial"))
```

### Named codecs

Named codecs are different from `serial_config()`: they are storage-format byte transforms with explicit read-side selection by name, content type, extension, or sniffing. Built-in/native codecs should not call the R API and may run inside Rust async/background work. R-closure codecs, if ever allowed, must be treated like serializers and run on the R thread only.

```r
codec <- codec_config("gzip")

opt(fs, "codec") <- codec

fs_write(fs, "x.bin.gz", raw_vec, mode = "raw", codec = "gzip")
fs_read(fs, "x.bin.gz", mode = "raw", codec = "gzip")
```

Initial built-in native codecs are `identity`, `gzip`, and `zlib`. `mode =
"codec"` is a raw-byte alias that requires a codec. Non-identity codec reads
currently require complete-object reads; byte ranges remain `mode = "raw"`
without a codec unless a future codec explicitly supports partial decode.

Core codec selection should be explicit to avoid hidden deserializer surprises:

- `mode = "serial"` follows nanonext-like `serial_config(class, sfunc, ufunc)` semantics.
- `codec =` applies a bytes-to-bytes transform before write or after read; it does not decide R object classes.
- No automatic extension/content-type/sniff selection in the core API initially. Those can be adapter/plugin policy later.
- Do not silently fall back to R serialization for codecs; use `mode = "serial"` for base R serialization semantics.
- Native codecs can be shared with the C API because they are R-free byte transforms.

For one-off reads, allow an explicit deserializer without registering a codec:

```r
fs_read(fs, "external-format.bin", mode = "raw", deserialize = decode_external)
fs_read_aio(fs, "external-format.bin", mode = "raw", deserialize = decode_external)
```

In that case I/O is still async for `_aio`; only `deserialize(raw)` runs on the R thread when collected.

## Auth and credentials

Core constructors should be explicit and as stateless as possible: no hidden env-variable lookup, provider chain, profile search, or credential store access. Users provide credentials/config directly or by explicitly calling helper functions that return credential objects.

```r
credentials_s3(
  access_key_id,
  secret_access_key,
  session_token = NULL,
  region = NULL
)

credentials_gcs(
  token = "",
  service_account_key = "",
  credential_path = "",
  scope = "",
  source = "direct"
)

credentials_azblob(
  account_name = "",
  account_key = "",
  sas_token = "",
  endpoint = "",
  source = "direct"
)

credentials_gdrive(
  access_token = NULL,
  refresh_token = NULL,
  client_id = NULL,
  client_secret = NULL
)
```

Credential helper/plugins may provide explicit readers such as `credentials_from_env()`, `credentials_from_file()`, provider chains, stores, or gargle integration, but the core API should not invoke them implicitly.

HTTP(S) filesystems can also take explicit request headers:

```r
fs <- opendal(
  "http",
  endpoint = "https://example.test/data",
  root = "/",
  headers = list(Authorization = "Bearer ...", `X-Api-Key` = "...")
)
```

Header values are treated as credential-bearing configuration: they are passed to the Rust/OpenDAL HTTP client layer, not printed in filesystem summaries, and not stored in package metadata or fixtures. They are currently restricted to OpenDAL `http`/`https` filesystem handles to avoid accidentally altering signed object-store requests.

Google Drive maps directly to OpenDAL `gdrive` config:

- `root`
- `access_token`
- `refresh_token`
- `client_id`
- `client_secret`

OpenDAL requires either:

- `access_token`, or
- `refresh_token` + `client_id` + `client_secret`

Potential explicit helper for users of gargle without a hard dependency:

```r
credentials_gdrive_gargle <- function(token = gargle::token_fetch(...)) {
  credentials_gdrive(access_token = token$credentials$access_token)
}
```

If implemented, put gargle in `Suggests`, never `Imports`.

## Layers and runtime options

OpenDAL layers should be configured at construction time.

```r
fs <- opendal(
  "s3",
  bucket = "bucket",
  auth = credentials_s3(...),
  layers = list(
    # layer_retry(max_times = 5), # future if exposed explicitly
    layer_timeout(request_timeout = 60, io_timeout = 10),
    layer_concurrent_limit(128)
  ),
  runtime = runtime_config(threads = 4)
)
```

Layer helper objects are R lists/classes converted to Rust config before operator construction.

## Notification, condition variables, and monitors

Nanonext's condition-variable and monitor design is useful for Ropendal, but the filesystem package should adapt the concept rather than copy socket/pipe terminology directly. The primitive event source is Aio completion, not a network pipe.

R exports:

```r
cv <- cv()
cv_wait(cv)              # block until signalled
cv_until(cv, msec = 10)  # bounded wait
cv_value(cv)
cv_reset(cv)
cv_signal(cv)

# `_aio()` calls can signal a cv on completion.
aio <- fs_read_aio(fs, path, cv = cv)
cv_until(cv, 100)
collect_aio(aio)

# Monitor many Aios and drain completion events.
mon <- aio_monitor(list(a = aio1, b = aio2), cv = cv)
cv_wait(cv)
read_monitor(mon)
# data.frame(id/name, event = "ready"/"error"/"cancelled", index, ...)
```

Design rules:

- Signalling happens from Rust/Tokio without calling the R API.
- The signal increments an atomic/mutex-protected counter, like nanonext `cv`.
- `stop_aio()` mirrors nanonext ergonomics where possible: it requests cancellation for the Aio handle and resolves the handle to a classed cancellation error value if cancellation wins, but it cannot guarantee that a backend request or remote write did not already happen.
- `race_aio(aios, cv)` can be implemented on top of the same primitive.
- Monitors are for completion notifications and event draining; they do not materialize R values in the background.
- Avoid exporting `pipe_notify()` unless Ropendal later has a real connection/stream concept. Use `aio_notify()` or `aio_monitor()` instead.
- If aliases `wait()`/`until()` are considered, keep canonical names `cv_wait()`/`cv_until()` to avoid ambiguity.

C exports should mirror this for native consumers that manage many operations:

```c
ropendal_cv_alloc(&cv, &err);
ropendal_aio_notify(aio, cv, id, &err);
ropendal_cv_until(cv, 100, &err);

ropendal_monitor_create(cv, &mon, &err);
ropendal_monitor_add_aio(mon, aio, id, &err);
ropendal_monitor_read(mon, &events, &len, &err);
ropendal_monitor_release(mon);
ropendal_cv_release(cv);
```

This gives C callers a lightweight completion queue without forcing one callback per request, while preserving the simpler `ropendal_aio_wait()` path for one-off operations.

## C API design

The native API is pure C and async-first. It does not include R headers and should not require C callers to touch R's C API. Synchronous native operations are implemented by submitting an Aio and waiting.

### Ownership rules

- `ropendal_fs_t` is an opaque retained handle around the Rust filesystem. It must be released.
- `ropendal_aio_t` is an opaque task handle. It owns any Rust result buffers until released.
- `ropendal_bytes_t` is an opaque immutable byte handle for synchronous native byte transforms such as codecs. It must be released.
- Callbacks may run on Rust/Tokio worker threads and must not call R's C API unless the caller independently arranges safe handoff to R's main thread.
- For `read_into_aio`, the caller owns `dst` and must keep it valid until the Aio reaches a terminal state.
- Returned buffer pointers from `ropendal_aio_result_bytes()` are owned by the Aio and valid until `ropendal_aio_release()`.
- Returned metadata/entry pointers are also owned by the Aio and valid until `ropendal_aio_release()`. The Aio must own C-compatible stable storage for strings and entry arrays.

### C types

See `inst/include/ropendal.h` for the public header. Public option/result structs start with `size_t struct_size` so the ABI can grow without silently misreading older caller memory.

Core status enum:

```c
typedef enum {
  ROPENDAL_AIO_PENDING = 0,
  ROPENDAL_AIO_READY = 1,
  ROPENDAL_AIO_ERROR = 2,
  ROPENDAL_AIO_CANCELLED = 3
} ropendal_aio_status_t;
```

Core async functions:

```c
ropendal_status_t ropendal_fs_open(const char *scheme,
                                   const ropendal_kv_t *config,
                                   size_t config_len,
                                   ropendal_fs_t **out,
                                   ropendal_error_t **err);
ropendal_status_t ropendal_fs_from_uri(const char *uri,
                                       ropendal_fs_t **out,
                                       ropendal_error_t **err);
void ropendal_fs_retain(ropendal_fs_t *fs);
void ropendal_fs_release(ropendal_fs_t *fs);

ropendal_status_t ropendal_codec_encode(const char *codec,
                                        const uint8_t *src, size_t src_len,
                                        ropendal_bytes_t **out,
                                        ropendal_error_t **err);
ropendal_status_t ropendal_codec_decode(const char *codec,
                                        const uint8_t *src, size_t src_len,
                                        ropendal_bytes_t **out,
                                        ropendal_error_t **err);
const uint8_t *ropendal_bytes_data(const ropendal_bytes_t *bytes);
size_t ropendal_bytes_len(const ropendal_bytes_t *bytes);
void ropendal_bytes_release(ropendal_bytes_t *bytes);

ropendal_status_t ropendal_read_aio(ropendal_fs_t *fs,
                                    const ropendal_read_options_t *opts,
                                    ropendal_aio_t **out,
                                    ropendal_error_t **err);
ropendal_status_t ropendal_read_into_aio(ropendal_fs_t *fs,
                                         const ropendal_read_options_t *opts,
                                         uint8_t *dst, size_t dst_len,
                                         ropendal_aio_t **out,
                                         ropendal_error_t **err);
ropendal_status_t ropendal_write_aio(ropendal_fs_t *fs,
                                     const ropendal_write_options_t *opts,
                                     const uint8_t *src, size_t src_len,
                                     ropendal_aio_t **out,
                                     ropendal_error_t **err);
ropendal_status_t ropendal_replace_aio(ropendal_fs_t *fs,
                                       const ropendal_write_options_t *opts,
                                       const uint8_t *src, size_t src_len,
                                       ropendal_aio_t **out,
                                       ropendal_error_t **err);
ropendal_status_t ropendal_append_aio(ropendal_fs_t *fs,
                                      const ropendal_write_options_t *opts,
                                      const uint8_t *src, size_t src_len,
                                      ropendal_aio_t **out,
                                      ropendal_error_t **err);
ropendal_status_t ropendal_stat_aio(ropendal_fs_t *fs, const char *path,
                                    ropendal_aio_callback_t callback,
                                    void *userdata,
                                    ropendal_aio_t **out,
                                    ropendal_error_t **err);
ropendal_status_t ropendal_exists_aio(ropendal_fs_t *fs, const char *path,
                                      ropendal_aio_callback_t callback,
                                      void *userdata,
                                      ropendal_aio_t **out,
                                      ropendal_error_t **err);
ropendal_status_t ropendal_ls_aio(ropendal_fs_t *fs,
                                  const ropendal_ls_options_t *opts,
                                  ropendal_aio_t **out,
                                  ropendal_error_t **err);
ropendal_status_t ropendal_mkdir_aio(ropendal_fs_t *fs, const char *path,
                                     ropendal_aio_callback_t callback,
                                     void *userdata,
                                     ropendal_aio_t **out,
                                     ropendal_error_t **err);
ropendal_status_t ropendal_delete_aio(ropendal_fs_t *fs,
                                      const ropendal_delete_options_t *opts,
                                      ropendal_aio_t **out,
                                      ropendal_error_t **err);
ropendal_status_t ropendal_copy_aio(ropendal_fs_t *fs, const char *from,
                                    const char *to,
                                    ropendal_aio_callback_t callback,
                                    void *userdata,
                                    ropendal_aio_t **out,
                                    ropendal_error_t **err);
ropendal_status_t ropendal_rename_aio(ropendal_fs_t *fs, const char *from,
                                      const char *to,
                                      ropendal_aio_callback_t callback,
                                      void *userdata,
                                      ropendal_aio_t **out,
                                      ropendal_error_t **err);

ropendal_aio_status_t ropendal_aio_poll(ropendal_aio_t *aio);
ropendal_status_t ropendal_aio_wait(ropendal_aio_t *aio, int timeout_ms,
                                    ropendal_error_t **err);
void ropendal_aio_cancel(ropendal_aio_t *aio);
void ropendal_aio_release(ropendal_aio_t *aio);

ropendal_status_t ropendal_aio_result_bytes(ropendal_aio_t *aio,
                                            const uint8_t **data,
                                            size_t *len,
                                            ropendal_error_t **err);
ropendal_status_t ropendal_aio_result_nread(ropendal_aio_t *aio,
                                            size_t *nread,
                                            ropendal_error_t **err);
ropendal_status_t ropendal_aio_result_readv(ropendal_aio_t *aio,
                                            const ropendal_readv_result_t **results,
                                            size_t *len,
                                            ropendal_error_t **err);
ropendal_status_t ropendal_aio_result_bool(ropendal_aio_t *aio,
                                           int *value,
                                           ropendal_error_t **err);
ropendal_status_t ropendal_aio_result_entry(ropendal_aio_t *aio,
                                            const ropendal_entry_t **entry,
                                            ropendal_error_t **err);
ropendal_status_t ropendal_aio_result_entries(ropendal_aio_t *aio,
                                              const ropendal_entry_t **entries,
                                              size_t *len,
                                              ropendal_error_t **err);
```

Vectorized range reads:

```c
ropendal_status_t ropendal_readv_aio(ropendal_fs_t *fs,
                                     const ropendal_read_request_t *requests,
                                     size_t n_requests,
                                     const ropendal_readv_options_t *opts,
                                     ropendal_aio_t **out,
                                     ropendal_error_t **err);

ropendal_status_t ropendal_readv_into_aio(ropendal_fs_t *fs,
                                          const ropendal_read_into_request_t *requests,
                                          size_t n_requests,
                                          const ropendal_readv_options_t *opts,
                                          ropendal_aio_t **out,
                                          ropendal_error_t **err);
```

`ropendal_read_options_t` carries the same performance knobs as R:

- `part_concurrency`: per-object chunk fanout
- `chunk_size`: requested chunk size in bytes
- `coalesce_gap`: merge nearby ranges to reduce backend calls
- `prefetch`: streaming-reader buffer depth
- `content_length_hint`: avoid extra stat requests when the caller already knows object size

`readv`/`readv_into` also take `batch_concurrency`, which is independent of `part_concurrency`. This separation matters for native consumers: one package may need 64 independent range reads in flight but only 2 chunks per object, while another may need 1 object with 16 chunk reads.

This is the important path for high-performance consumers: indexed file readers, genomics tools, Arrow/DuckDB bridges, Zarr-like arrays, etc. For `read_into` and `readv_into`, the caller owns the destination buffers and must keep them valid until the Aio reaches a terminal state. `readv_into` destination buffers must not overlap because requests may be filled concurrently. Ropendal writes bytes into those buffers to avoid R raw-vector copies. `readv_aio()` is the borrowed-result counterpart: successful range payloads are flattened in request order and borrowed through `ropendal_aio_result_bytes()`, while `ropendal_aio_result_readv()` supplies per-request statuses and lengths needed to split the byte stream. `ropendal_aio_result_nread()` reports total successful bytes for all read/readv shapes, and per-request backend failures are represented in `ropendal_aio_result_readv()` rather than failing the whole vector Aio.

## Package build features

OpenDAL feature set should include Google Drive in addition to the object-store/file/http defaults:

```toml
opendal = { version = "0.57", default-features = false, features = [
  "blocking",
  "executors-tokio",
  "reqwest-rustls-tls",
  "services-fs",
  "services-http",
  "services-s3",
  "services-gcs",
  "services-azblob",
  "services-gdrive",
  "layers-retry",
  "layers-timeout",
  "layers-concurrent-limit"
] }
```

Call `opendal::init_default_registry()` explicitly during package initialization or before construction because static-linked bindings cannot rely on service registry constructors.
