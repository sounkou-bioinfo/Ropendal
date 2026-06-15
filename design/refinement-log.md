# Refinement log and open API questions

This project should preserve primitive orientation while allowing API details to evolve. When an ambiguity appears, record it here with a provisional resolution instead of burying it in conversation.

Status labels:

- `resolved`: decision accepted for now
- `provisional`: usable but may change while the package is experimental
- `implemented`: current code implements the decision; remaining work is refinement or broader coverage
- `open`: needs design/testing before implementation

## Resolved / provisional decisions

### R listing name

Status: `resolved`

Do not export `list()` or `$list`. Use `fs_ls()` for collected enumeration, `fs_ls(..., recursive = TRUE)` for collected recursive traversal, and `fs_walk_iter()` for streaming recursive traversal.

### Range reads

Status: `provisional`

Do not add public `fs_read_range()` / `fs_readv()` initially. Make `fs_read()` vectorized over paths and per-path range lists. Keep the C API explicit with `read_aio`, `read_into_aio`, and `readv_into_aio` because native callers need buffer ownership.

### Result shape

Status: `resolved`

Replace ambiguous `simplify = TRUE` with:

```r
result = c("auto", "flat", "nested")
```

The public `fs_read()` / `fs_read_aio()` shape contract is implemented and covered by tinytest through the direct `path`, `offset`, `size`, and `end` arguments. A separate public request-table helper remains optional future API rather than a current exported contract.

### Errors

Status: `provisional`

Use errors as values, nanonext-like. Aio active bindings and synchronous primitives return `unresolvedValue`, success value, or `opendalErrorValue`; they do not throw for backend/filesystem failures. There is no `errors = "stop"` option. Hard errors still throw for invalid R arguments, serializer/deserializer exceptions, unsafe pointer misuse, internal bugs, or allocation failures.

### CI-only tests

Status: `resolved`

Use env-gated tests for API contract linting and non-CRAN checks:

```sh
ROPENDAL_TEST_CI=true
```

Network/service tests remain separate:

```sh
ROPENDAL_TEST_NETWORK=true
ROPENDAL_TEST_GDRIVE=true
```

### Streaming namespace iterators

Status: `provisional`

`fs_ls_iter()` and `fs_walk_iter()` use a Rust-backed `OpendalLsIter` that streams entries from OpenDAL and materializes only the requested R page on the R thread. `ls_iter_next()` / `walk_iter_next()` return `list(done, entries, cursor)`, where `cursor` is the last yielded root-relative path. It is useful as a best-effort `start_after` marker for lexically ordered listings, but it is not an opaque backend continuation token and does not by itself guarantee exact restart semantics on unordered traversals. `*_collect()` collects remaining entries. `page_size` bounds R entries yielded per page and is passed as an OpenDAL request-size hint, `limit` bounds total yielded/materialized entries, and `start_after` provides a root-relative continuation marker with backend and client-side filtering. `prefetch` is an explicit entry-buffer depth: `0` keeps strict demand-driven iteration, while positive values may start a Rust/Tokio task at iterator construction that reads ahead into a bounded channel without calling R. Traversal fanout and opaque backend continuation tokens remain future refinements.

### HTTP(S) explicit headers

Status: `resolved`

OpenDAL's HTTP service exposes built-in auth fields but not arbitrary headers in service config. Ropendal supports `headers = list(...)` on `opendal("http", ...)` and `opendal_uri("http(s)://...", headers = ...)` by installing an OpenDAL HTTP client layer that injects validated headers per request. Header support is intentionally restricted to HTTP(S) filesystem handles so it does not interfere with signed object-store requests. Header values are credential-bearing data and must not be printed or committed in metadata/fixtures.

## Additional decisions and open questions

### Exact `opendalErrorValue` representation

Status: `implemented`

Current error values are small classed lists constructed in Rust:

- base classes: `opendalErrorValue`, `errorValue`, `list`
- kind-specific class such as `opendalNotFoundValue`
- fields: `__ropendal_error__`, `code`, `kind`, `message`, `operation`, and `path`

The code mapping follows OpenDAL `ErrorKind` for the currently surfaced kinds, with `Cancelled` added for cancelled Aio handles. Future service/profile work may add redacted service/status/context fields, but the current public helpers (`is_error_value()`, `error_kind()`, `error_message()`, `error_operation()`, and `error_path()`) should remain stable.

### Partial failures in vectorized reads

Status: `implemented`

Shape preservation is required and implemented. Each failed element becomes an `opendalErrorValue` in the corresponding output position. Current vectorized execution continues expanded requests and returns per-element success or error values rather than failing the whole operation. A future fail-fast policy would need an explicit design reason and should not silently change the primitive result shape.

### Directory inputs to `fs_read()`

Status: `resolved`

Byte ranges apply to file reads only. Directory traversal belongs to `fs_ls()` / `fs_ls(..., recursive = TRUE)` / `fs_walk_iter()`.

### Text mode and ranges

Status: `resolved`

Partial text ranges can split multibyte characters. Ropendal rejects partial
`mode = "text"` reads by default and directs callers to `mode = "raw"` for byte
ranges. Text mode is a complete-object materialization layer with an explicit
`encoding` boundary. Because R character strings cannot carry embedded NUL bytes
before decoding, encodings whose probe bytes contain NULs are rejected; use raw
mode for UTF-16/UTF-32-like storage formats unless a future byte-safe decoder is
added. Writes accept scalar strings per path; vectorized text writes use a
character vector or list of scalar strings matching path length.

### Serialization mode and ranges

Status: `resolved`

Reject partial `mode = "serial"` by default. A serialized object must be read as a complete object unless a codec explicitly supports partial decode.

### Credential model and precedence

Status: `provisional`

Core API is explicit and as stateless as possible. No hidden env-variable/provider-chain lookup in core constructors. Users provide credentials through explicit arguments or explicit credential/helper objects. Current built-in providers cover S3-compatible access keys, GCS token/service-account material, AzBlob account keys/SAS tokens, and Google Drive direct/gdrive3 credentials. Reader helpers may load env vars, profiles, stores, provider chains, gargle tokens, etc., but calling those helpers is the user's responsibility.

This explicit model should serve as the base for plugins that implement provider chains or credential stores.

### Monitor naming

Status: `provisional`

Use `cv_*`, `aio_notify()`, `aio_monitor()`, `read_monitor()`. Avoid `pipe_notify()` unless a future Ropendal concept really has pipes/connections.

### Declarative capabilities and adapters

Status: `provisional`

The public API is uniform. Ropendal declares an effective capability profile for each filesystem handle and implements that profile in Rust/C using OpenDAL operations, layers, or Ropendal adapters.

If Ropendal declares an operation supported, the corresponding R/C primitive should work with documented semantics. If it cannot be supported honestly, declare it unsupported and return `opendalUnsupportedValue` when called.

`fs_capabilities(fs)` should report the declared Ropendal contract, with implementation diagnostics such as `native`, `opendal_layer`, or `ropendal_adapter`, plus semantic notes. Internal composition is not a user option.

Do not silently weaken conditions, version IDs, metadata requirements, or consistency semantics. If the adapter cannot honor a request, return an error value.

### Runtime ownership

Status: `implemented`

Current implementation uses one Tokio runtime per `OpendalFs` handle, configured by `runtime_config(threads = )`, and stores it with the OpenDAL operator in an `Arc` so submitted Aio work can outlive the R filesystem wrapper. Service-wide request throttling remains a layer concern (`layer_concurrent_limit()`), not a runtime-thread knob. A shared runtime can be reconsidered later only if startup/resource costs justify changing the internal ownership model without changing the public API.

### C ABI versioning

Status: `provisional`

All public option/result structs should start with `size_t struct_size`. C tests should enforce this. Need define behavior for unknown/larger/smaller struct sizes.

### Path normalization and path identity

Status: `resolved`

All operation paths are relative to the configured root. Callers are responsible for requesting the correct logical path, and Ropendal normalizes path strings before dispatch:

- collapse repeated slashes
- remove `.` components
- resolve `..` components without allowing escape above root
- normalize directory paths to trailing `/` for directory-returning/listing APIs
- return directory entries with trailing `/`; file entries do not have trailing `/`

Path identity is the normalized OpenDAL logical path. `fs_read()` on a directory-like path resolves to an `opendalIsADirectoryValue` or service-specific error value.

### Metadata schema

Status: `resolved`

Metadata shape is defined at the Rust/C layer and returned to R as generic lists, not forced into a data frame. Required common fields where available:

- `path`
- `type`
- `size`
- `etag`
- `last_modified`
- `version`

Profiles may add fields. R helpers can later provide `as.data.frame()` for tabular consumers, but the primitive API returns lists because metadata is service-dependent.

### Write semantics

Status: `resolved`

No overwrite by default. `fs_write()` creates a file/object and returns `opendalAlreadyExistsValue` if the target exists. Append is a distinct operation (`fs_append()`), supported only when the declared profile supports append. Replacement/overwrite is also distinct (`fs_replace()`) rather than hidden behind an overwrite flag.

Directory hierarchy creation is not part of file write semantics. Use `fs_mkdir()` / directory APIs for directories. Whether parent creation is supported is part of the declared profile.

### Vectorized write shape

Status: `resolved`

No recycling. Strict length matching for user-supplied vectors/lists. Defaults may expand internally, but explicit `path`, `data`, `offset`, and `size` vectors/lists must match the expected shape exactly. Length mismatches are hard R argument errors. Mixed success/backend failure returns preserve shape with `opendalErrorValue` elements.

### Codec and serializer selection

Status: `provisional`

Use nanonext-like semantics for the core serializer API:

```r
serial_config(class, sfunc, ufunc)
```

`mode = "serial"` uses base R serialization with custom serialize/unserialize hooks. `sfunc` and `ufunc` are paired, and both R closures run only on the R thread. Current implementation stores custom-hook payloads in an explicit Ropendal serialization envelope so read-side deserialization can select the matching `ufunc` from `serial_config()`.

The optional storage-format codec layer (`codec_config()`) starts explicit-only. Later extension/content-type selection can be added as adapter policy, but core behavior should avoid hidden deserializer surprises.

### Async cancellation semantics

Status: `provisional`

Mirror nanonext ergonomics where possible: `stop_aio()` requests cancellation for an Aio and the handle resolves to a cancellation error value, e.g. `opendalCancelledValue`, if cancellation wins. The current R helper marks the handle cancelled immediately after requesting abort; callers that need to observe final state should inspect `$state`, `$error`, or collect the Aio.

There are no guarantees about backend side effects unless the backend/profile primitive provides them. A remote request may already have completed or a write may already have been committed. For C `read_into_aio()` and `readv_into_aio()`, caller-owned buffers must remain valid until the Aio reaches a terminal state.

### Timeout and waiting semantics

Status: `implemented`

Operation timeouts are operation/backend behavior and resolve as error values if the package/profile/layer defines them. `layer_timeout(request_timeout=, io_timeout=)` exposes OpenDAL's timeout layer explicitly at filesystem construction; `request_timeout` covers non-streaming operations, `io_timeout` covers read/write streams and listing iteration, and if only one timeout is supplied the omitted side uses OpenDAL's timeout-layer default. Wait timeouts are user convenience: `cv_until()` / `aio_wait(timeout)` control waiting and do not by themselves guarantee cancellation. Cancellation remains subject to the backend/profile semantics described above.

### Warning policy

Status: `provisional`

Adapters may warn for whatever profile-specific reason is helpful, especially weaker consistency semantics, expensive adapter paths, or partial text decoding risk. Warnings are diagnostics only and must never be required for control flow. Backend/filesystem failures still resolve to error values.

### Native C result model for vector reads

Status: `implemented`

Native C byte reads are buffer-oriented to avoid copies. For `read_into_aio()` and `readv_into_aio()`, the caller provides destination buffers and owns them. Ropendal fills those buffers asynchronously. The caller must keep buffers valid until the Aio reaches a terminal state.

Current result inspection details:

- `ropendal_readv_aio()` returns successful borrowed bytes flattened in request order through `ropendal_aio_result_bytes()`.
- `ropendal_readv_into_aio()` fills caller-owned buffers and reports total bytes through `ropendal_aio_result_nread()`.
- `ropendal_aio_result_readv()` returns one `ropendal_readv_result_t` per request with status, byte count, and borrowed error details.
- Result arrays and borrowed byte/entry pointers are owned by the Aio and valid until `ropendal_aio_release()`.

### Native C API distribution

Status: `resolved`

`R_RegisterCCallable()` is not the goal. Ropendal should expose a pure C API and compile/provide a library so C consumers do not need to touch R's C API. The installed header must not include R headers. R-specific bridging can exist separately for the R package internals, but not as the core native consumer contract.

### Thread safety and lifetime

Status: `open`

Need define whether `ropendal_fs_t`, `ropendal_aio_t`, `ropendal_cv_t`, and `ropendal_monitor_t` are thread-safe, and exactly how retain/release interacts with R GC and package unload.

### Service profiles and adapter registry

Status: `open`

Need design the internal registry that maps a service/config to a declared Ropendal capability profile. This is where service-specific Rust/C adapters live, including Google Drive ergonomics.

### Auth refresh and secret lifetime

Status: `provisional`

Core passes explicit credentials/config to Rust/OpenDAL. Refresh behavior should be delegated to OpenDAL/service support where available, e.g. Google Drive refresh-token handling, rather than reinvented in R. Secrets must be redacted in errors, `print()`, `str()`, README, tests, and profile diagnostics.

### R/Rust responsibility boundary

Status: `provisional`

Keep `R/api.R` as a very thin public surface. R should define user-facing names,
S7/S3 generics and interfaces, documentation, optional local credential-source
objects, materialization closures, and README/test orchestration. Rust should own
filesystem operation semantics, operation-call argument validation, strict vector
length checks, result shaping, S3 classes and fields for returned native values,
capability values, and error-value construction.

The current implementation has moved operation loops and error construction into
Rust/savvy methods. Remaining R helpers are primarily interface adapters, option
normalizers, serializer/deserializer and codec orchestration, Aio active-binding
materialization, printing, and R-side wait helpers. Hard R errors remain
appropriate only for R-side interface construction, explicit environment/file
helper setup, serializer/deserializer failures, and generic dispatch failures.

### Credential provider interface

Status: `implemented`

A credential provider is a small explicit object/protocol that can materialize a
service-specific OpenDAL config at filesystem construction time and can present a
redacted summary for diagnostics. It is not an implicit global provider chain and
not just an unclassed list of secrets.

Current behavior:

- `CredentialProvider` is an S7 class wrapping a Rust-backed provider object and
  supported schemes.
- `credential_config(provider, service)` returns named scalar config values for
  Rust/OpenDAL, with secrets only in the returned construction payload.
- `credential_summary(provider)` returns a redacted classed value for printing,
  capability diagnostics, and logs.
- built-in helpers cover S3, GCS, AzBlob, Google Drive direct credentials, and
  local `gdrive3` JSON/token-file credentials.

A future `s7contract` interface/trait layer may still be useful for third-party
providers, but the initial explicit provider protocol is implemented.

### Capability values and interfaces

Status: `provisional`

`fs_capabilities()` returns a classed capability value built in Rust rather than
an R-assembled plain list. R-level interfaces such as `ReadableFs`, `WritableFs`,
or `ListableFs` can be useful for consumers, but support is a runtime property
of the handle/profile and must ultimately be enforced by Rust operation methods
returning `opendalUnsupportedValue` where appropriate.

### Serializer, codec, and R API thread boundary

Status: `provisional`

Do not move `SEXP`, savvy `FunctionSexp`, R closures, or arbitrary R objects into
Tokio/background tasks. Savvy-generated entrypoints run on the R thread; savvy's
`Send`/`Sync` implementations for narrow items such as ALTREP class descriptors
are not a license to call the R API concurrently. ALTREP callbacks are also R VM
callbacks, not a general background scheduler.

Split conversion into two layers:

- **Serializers** are R object <-> raw-byte conversions. `serial_config(class,
  sfunc, ufunc)` remains nanonext-like: `sfunc(object)` returns raw bytes and
  `ufunc(raw)` returns an R object. These functions may touch R and must run only
  on the R thread.
- **Codecs** are raw-byte <-> raw-byte transforms. Built-in/native codecs may run
  in Rust async/background work and can be exposed consistently to the C API.
  R-closure codecs, if ever allowed, must be treated like serializers and run on
  the R thread only.

Async semantics should be future/phase based around bytes:

- `fs_write(..., mode = "serial")` serializes on the R thread before submitting
  the async upload. For vectorized writes, serialize one item on the R thread and
  submit its upload before serializing the next, so uploads may overlap later
  serialization without background tasks touching R.
- `fs_read_aio(..., mode = "serial")` downloads bytes asynchronously. Collection
  waits for bytes, then deserializes on the R thread before returning the final R
  object. Polling may distinguish pending I/O from ready-bytes/pending-decode if
  needed.
- The C API remains byte-first. Native consumers own object serialization unless
  they opt into pure byte codecs; the C API must not mention `SEXP` or R-specific
  serializer objects.

### Byte materialization, R connections, and read/write asymmetry

Status: `provisional`

There are three distinct layers that must not be conflated:

1. **Byte future**: Rust/OpenDAL owns in-flight I/O and resolves to owned bytes,
   or writes from bytes whose lifetime is independent of R's heap.
2. **Materializer**: conversion between bytes and an R representation. Returning
   an R `raw` vector, extracting bytes from a matrix column, or running
   `serialize()` all touch the R API and must run on the R thread.
3. **Adapter**: convenience surfaces such as iterators, Aio collection helpers,
   and possible R connection wrappers.

Read and write are inherently asymmetric at the R boundary. Reads can complete
into Rust-owned bytes first and delay materialization into an R `raw` vector or R
object until collection/access. Writes that start from ordinary R objects must
first serialize or copy into stable owned bytes before background upload. A truly
borrowed async write from R memory would require an explicit lifetime contract
and is not safe for ordinary R vectors because of mutation and GC interaction.

The R-facing byte object is now explicit: immutable `OpendalBytes` values are
R-GC-managed external byte holders around OpenDAL/Rust `Buffer` storage. The R
object owns reachability and finalization through an external pointer; the byte
storage is not an R `RAWSXP` payload unless materialized. `length()` is cheap,
`as.raw()` currently materializes to an R raw vector, and the handle can be
passed back to Ropendal writes without routing through another R raw-vector
payload. A future ALTREP raw facade is feasible by storing the external holder in
ALTREP `data1`; `Elt`/`Get_region` can copy requested bytes without full
materialization, and read-only `Dataptr()`/`Dataptr_or_null()` can expose a stable
contiguous pointer when the underlying buffer shape permits it. Writable
`Dataptr()` must materialize into an R-owned `RAWSXP` cache in `data2`. The main
constraint is writable/raw-vector pointer semantics, not holder lifetime. If a
user passes a normal R `raw`, matrix, character vector, or arbitrary object, the
package still has to touch the R API on the R thread to copy or serialize.

R's connection API is an adapter candidate, not the core abstraction. `readBin()`
and `writeBin()` operate through a connection's synchronous callbacks; a
non-blocking connection returns what is currently available, but it does not make
R API calls safe on background threads. Ropendal can later expose read/write
connections backed by `fs_read_iter()` and `fs_write_iter()` for compatibility,
with `seek` mapped to range reads where possible and write `close()` mapped to
multipart/finalize. The core API should remain byte futures plus iterators/Aio so
provider errors, object-store range semantics, multipart finalization, and C API
ownership stay explicit.

### Async metadata and namespace operations

Status: `implemented for R API; C stat/exists/ls result parity implemented for local roundtrip`

The async-first contract applies to every operation that may perform backend I/O,
not only byte reads/writes. On S3, Google Drive, HTTP-like remote services, and
other object stores, `stat`, `exists`, `ls`, recursive `walk`, `delete`, `copy`,
`rename`, `mkdir`, and append/write completion are network operations with
latency, pagination, rate limits, and backend-specific consistency semantics.

Revise the core wording from a byte future to a native operation future. Bytes
are one result family among several:

- data plane: read/write/replace/append -> bytes or unit completion;
- metadata plane: stat/exists -> metadata or bool;
- namespace plane: ls/walk/mkdir/delete/copy/rename -> entries or unit
  completion;
- materialization/adapters: raw vectors, `OpendalBytes`, text, serial R objects,
  entries data frames, and R connections.

Every backend-I/O operation should have an `_aio()` form. Synchronous R functions
should be blocking convenience wrappers over the same native operation pipeline.
Only local handle introspection such as `fs_info()`, `fs_capabilities()`, and path
normalization can stay sync-only.

Listing needs both collectable and streaming forms. `fs_ls_aio()` can collect a
finite listing, but `fs_ls_iter()` and `fs_walk_iter()` are needed for paginated
or very large listings with backpressure. Listing controls should distinguish
many-prefix batch concurrency, recursive traversal fanout, page size, bounded prefetch,
limits, and continuation/resume points.

The generic Aio outcome should therefore include bytes, unit, bool, metadata,
entries, many, error, and cancelled states. R materialization of metadata/entries
still happens on the R thread. The C API mirrors the single-result metadata path
with R-free bool, entry, and entries accessors; returned pointers are owned by
the Aio until release.

Implementation note: the R API now has `fs_stat_aio()`, `fs_stats_aio()`,
`fs_exists_aio()`, `fs_ls_aio()`, `fs_mkdir_aio()`, `fs_delete_aio()`,
`fs_copy_aio()`, `fs_rename_aio()`, `fs_write_aio()`, `fs_replace_aio()`, and
`fs_append_aio()` over a generic `AioOutcome` that can materialize bytes, unit,
bool, metadata, entries, many, errors, and cancellation. The C API now exposes
async stat/exists/list/delete/copy/rename/mkdir operations plus bool/entry/entries
result accessors, `readv_into_aio()` for multiple caller-owned range buffers,
and `readv_aio()` for borrowed flattened byte results. `ropendal_aio_result_readv()`
exposes per-request status/byte-count/error details for both read-vector shapes;
`ropendal_aio_result_bytes()` returns successful `readv_aio()` payloads concatenated
in request order.

### Aio active binding contract

Status: `implemented for R Aio wrappers`

Generated `OpendalAio` environments are decorated with read-only active
bindings:

- `$value`: universal value binding; returns `unresolvedValue` while pending and
  the resolved value/error after readiness or collection;
- `$data`: alias for `$value` for read/stat/list/exists-like operations;
- `$result`: alias for `$value` for unit/completion operations;
- `$state`: native state string (`pending`, `ready`, `resolved`, `error`, or
  `cancelled`) without materializing success payloads;
- `$resolved`: logical predicate over state;
- `$error`: error value if the Aio has resolved as an error, otherwise `NULL`.

`call_aio()` now matches the intended nanonext-like shape: it waits/updates and
returns the Aio invisibly. `collect_aio()` remains the value-returning helper.
`unresolved()` remains the sentinel constructor when called with no argument and
also acts as a predicate for Aios or returned values.

### R Aio wait helpers and polling monitors

Status: `implemented provisionally in R`

R now exports `collect_aio_()`, `call_aio_()`, `cv()`, `cv_value()`,
`cv_reset()`, `cv_signal()`, `cv_wait()`, `cv_until()`, `aio_monitor()`,
`read_monitor()`, and `race_aio()`. The current R monitor is deliberately thin
and polling-based: it never materializes successful Aio values in the
background, and it uses `$error` only to classify ready/error/cancelled events.
This gives users nanonext-like wait/race ergonomics while keeping background
work in Rust/OpenDAL/Tokio and avoiding R API calls from worker threads.

This remains a provisional R-side bridge even though the native notification
ownership model is now implemented for C consumers. C `ropendal_aio_notify()` and
monitor event queues retain Aio/CV handles safely, but the exported R
`aio_monitor()` remains polling-based so completion classification never requires
background tasks to call the R API.

### Explicit native byte codecs

Status: `implemented`

`codec_config()` is now an explicit native byte-transform config for raw-byte
storage codecs. The first implemented codecs are `identity`, `gzip`, and `zlib`.
`codec =` can be passed directly to read/write/replace/append sync and Aio
helpers, or installed as `opt(fs, "codec")`. `mode = "codec"` is a raw-byte
alias that requires an explicit codec.

Codec transforms are deliberately separate from `serial_config()`: serializers
turn R objects into raw vectors on the R thread, then an optional codec transforms
those bytes before upload; reads decode bytes first and only then deserialize for
`mode = "serial"`. Non-identity codec reads currently require complete objects;
byte ranges remain raw-mode operations unless a future codec explicitly supports
partial decode.

The Rust implementation is R-free, and the public C API now exposes borrowed
byte-handle helpers through `ropendal_codec_encode()`, `ropendal_codec_decode()`,
`ropendal_bytes_data()`, `ropendal_bytes_len()`, and `ropendal_bytes_release()`.
C callers can roundtrip gzip/zlib bytes without using R's C API or R raw vectors.

### Native C monitor lifetime contract

Status: `provisional`

The C notification layer now distinguishes one-shot `ropendal_aio_notify()` from
monitor-based event queues. `ropendal_aio_notify(aio, cv, id, ...)` retains the
Aio and condition variable until the Aio reaches a terminal state, caches the
result on the Aio, then signals the CV. `ropendal_monitor_add_aio()` retains the
Aio until `ropendal_monitor_release()` so `ropendal_monitor_read()` can return
completion events that borrow the original Aio pointer while the monitor is
alive. Monitor release waits for registered notification workers to exit before
releasing retained Aio/CV references. This keeps the C API R-free and avoids
callbacks touching R while still allowing downstream native packages to block on
condition variables and drain completion events.

### Missing/default sentinel handling

Status: `implemented for R batch concurrency and read offsets; ongoing review item`

R-facing `NULL`/missing defaults and explicit user-supplied values must remain
distinct when they carry different API meaning. `batch_concurrency = NULL` means
use Ropendal's bounded default, while `batch_concurrency = 0` is an invalid R
argument; the C API keeps zero-as-unset only for documented zero-initialized
option structs. Read wrappers also distinguish a missing `offset` from an
explicit scalar `offset = 0`, so vectorized reads can default each path to
`0..EOF` without introducing general scalar recycling for user-supplied range
vectors.

When adding new options, avoid using `unwrap_or()` as a shortcut until the API
semantics of missing, zero, false, and empty-string values have been reviewed.

### `byte_ranges()` request objects

Status: `implemented`

Range-heavy R readers can now build a `byte_ranges(path, offset, size/end, id)`
request object and pass it as the `path` argument to `fs_read()`,
`fs_read_aio()`, `fs_read_bytes()`, or `fs_read_bytes_aio()`. The request is an
R-side convenience layer only: wrappers unpack it into the same vector/list
`path`, `offset`, `size`, and `end` shapes consumed by the Rust read planner.
This preserves one read surface while making index/table-driven formats such as
Zarr chunks, BGZF/Tabix, FASTA+FAI, and tiled binary indexes easier to express.

`byte_ranges()` defaults to flat results for row-oriented index tables, but
callers can request nested or auto shaping explicitly. Optional `id` values name
flat list results when lengths match; they are metadata for R consumers and do
not affect OpenDAL paths or backend requests.

### R byte store adapter

Status: `implemented for synchronous and Aio R byte operations; native C byte-store and cache operations implemented`

`byte_store(fs, prefix)` is an R-side prefix adapter for Zarr-like key-to-bytes
layouts. Store keys are normalized before they are joined to the store prefix so
`../` cannot escape the store root. Object operations reject empty keys and
trailing slash directory keys; recursive delete and listing use explicit
directory normalization.

`store_write()` preserves the filesystem create-only contract and returns an
`opendalErrorValue` such as `AlreadyExists` when the key already exists.
`store_replace()` is the overwrite path. `store_read()` is byte-only and now
returns `OpendalBytes` by default, with `mode = "raw"` as an explicit
materialization request; serial/text/codec materialization remains on the
lower-level `fs_read()` API for now. `store_list()` rewrites returned entry
paths to store-relative paths and filters the store-root marker when a backend
returns it. `store_*_aio()` wrappers are available for read/write/replace,
exists/list/delete; R-side cached async reads fill cache entries during main
thread collection rather than from background tasks.

The native C API exposes the same lower substrate as an opaque `ropendal_store_t`
opened from an existing `ropendal_fs_t` plus optional prefix. Its async read,
read-into, write, replace, exists, list, and delete functions copy submission
strings/options up front, keep destination buffers caller-owned until Aio
completion, and return listing paths relative to the store prefix. `ropendal_store_cache_open()`
wraps an uncached parent store plus an uncached cache store as a native full-object
cache adapter; partial reads bypass it, validation can use last-modified plus size
or trust cached objects, and recursive deletes clear cached objects for that cache
store. Pure-R vignettes now demonstrate a toy chunked-array reader and a VCF-like
range reader built above these primitives without moving format semantics into
the store layer. C zero values remain documented as unset, while
`ropendal_store_read_options_t.has_offset` and `has_size` make valid zero
offsets/sizes explicit for downstream structs.

### Byte-store full-object cache

Status: `implemented for synchronous and Aio R byte stores`

`store_cache(store, cache_dir, validate)` wraps a `byte_store()` in an explicit
local cache rooted in an OpenDAL `fs` store. The cache stores complete key
payloads under a namespaced cache prefix derived from the parent filesystem
scheme/root and byte-store prefix. This matches Zarr-like chunk stores where
keys are already small blocks and avoids an implicit whole-object cache on
ordinary `fs_read()` calls.

The first cache layer caches only complete `store_read()` and `store_read_aio()`
calls with default `result = "auto"`. Partial byte ranges and non-default read shaping delegate to
the parent store so callers cannot accidentally assemble large hidden downloads
or observe cache-specific shape changes. `validate = "last_modified_size"`
compares parent size and modification time before using a cached object;
`validate = "none"` deliberately trusts cached objects until `store_write()`,
`store_replace()`, `store_delete()`, or `store_cache_clear()` invalidates them.
Range-aware block caching, eviction, and service-specific validators such as
ETag/version are future work.

### Native C Aio callback arguments

Status: `implemented`

C Aio callbacks now receive the completed `ropendal_aio_t *` instead of a null
placeholder. The callback pointer is borrowed and does not transfer ownership;
the implementation retains the Aio for the callback worker, waits until the task
is terminal without blocking `ropendal_aio_poll()` or finite-timeout waits, caches
the result with the same finish path as `ropendal_aio_wait()`, invokes the
callback, and then releases the temporary reference. Cancellation requests abort
the Tokio task but do not cache a terminal cancellation value until the task has
quiesced, so callbacks/monitors do not report completion while caller-owned
buffers might still be touched by an aborting read-into task. Callback userdata
must remain valid until the callback runs, even if the caller releases its public
Aio handle earlier. Callbacks may run on worker threads and remain completion
notifications only: downstream callers must not call R's C API from them and
should inspect results via the normal Aio wait/poll/result accessors or hand off
to their own main-thread scheduler.

### Native C fixed-size block cache adapter

Status: `implemented for the native byte-store C API`

`ropendal_store_block_cache_open(parent, cache, opts, out, err)` adds the first
range-aware cache layer below R. Like `ropendal_store_cache_open()`, it accepts
uncached parent and cache stores and returns another opaque `ropendal_store_t`.
Unlike the full-object cache, reads through this adapter are split into fixed-size
byte blocks (default 8 MiB when the C option struct is zero-initialized) and
assembled back into the requested range. Complete reads are also served through
blocks after a parent stat determines object length.

The block cache remains deliberately explicit: callers choose the cache store,
block size, and validation strategy. `LAST_MODIFIED_SIZE` compares each cached
block against the current parent object metadata; `NONE` skips modification-time
checks but still refreshes when cached block metadata has a different object
size. Writes/replaces/deletes submitted through the block-cache
adapter mutate the parent store and invalidate the affected key's block entries;
recursive deletes conservatively clear the adapter's block namespace. This is
intentionally conservative while the higher R cache interface and eviction policy
are still being designed. There is no hidden cache beneath ordinary `fs_read()`.

### OpendalBytes slicing before ALTREP

Status: `implemented as a Rust-backed handle primitive`

`opendal_bytes_slice(x, offset, size/end)` returns another immutable
`OpendalBytes` handle for a subrange of an existing byte handle. The slice is
created in Rust from OpenDAL's `Buffer::slice()` and does not first materialize
the complete payload into an R raw vector. This is a stepping stone toward the
future ALTREP raw facade: callers can keep range composition in the byte-handle
layer, while `as.raw()` remains the explicit materialization boundary.

### R-facing fixed-size store block cache

Status: `implemented for scalar store reads`

`store_block_cache(store, cache_dir, block_size, validate)` adds an explicit
R-level block-cache wrapper over `byte_store()`. The cache is byte-only and uses
a local OpenDAL `fs` cache store. Scalar complete or byte-range reads with
`result = "auto"` are split into fixed-size blocks, cached as `OpendalBytes`,
and assembled back into either `OpendalBytes` or raw results.
Vectorized/non-auto shapes currently fall through to the parent store rather
than adding hidden policy. If any block in a requested range is missing,
stale, or has a corrupt payload length, the R and native adapters refill the
whole requested block set rather than assembling mixed cache/parent versions.

Validation mirrors the native cache choices: `last_modified_size` refreshes when
parent size/mtime changes, while `none` skips modification-time checks but still
uses the parent stat needed to bound ranges and refreshes cached block entries
whose stored object size differs from the current size. A cached block metadata
mismatch invalidates that object's whole block namespace so old blocks cannot be
resurrected after size cycles. In `none` mode, partial cache misses and corrupt
payload lengths also invalidate the key before refill to avoid assembling mixed
object versions. Cached payload lengths are checked before assembly; corrupted
or stale-length blocks are refetched as a whole requested block set rather than
silently assembled. Mutations through the wrapper invalidate the
affected object's block namespace; recursive deletes clear the block cache.
Async R reads submit the planned cache-hit or parent-fill read as an Aio and
perform cache fill/materialization on the R main thread after collection. Rare
repair fallbacks for cache payloads that disappear or become corrupt between
planning and collection may perform a synchronous parent refill on the R thread;
the native C block-cache adapter keeps that repair path inside the native Aio.
