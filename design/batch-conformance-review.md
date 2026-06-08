# Batch conformance review

This review checks the batch and range-read implementation against the
opinionated Ropendal design center: R-native verbs, byte-first operation futures,
shape-preserving vectorized results, bounded concurrency, explicit separation of
batch concurrency from per-object transfer concurrency, and a simple async-first
C API. Items marked implemented were addressed after the original review.

## Current alignment

The current implementation is directionally aligned with the design:

- `fs_read()`, `fs_read_bytes()`, `fs_read_aio()`, and `fs_read_bytes_aio()` all
  parse R arguments before background work and submit owned native requests to
  Tokio/OpenDAL.
- Batch read execution uses bounded unordered concurrency internally and restores
  request order before materialization.
- Write batching serializes or copies R payloads into stable native bytes before
  submission, so R objects do not enter background work.
- The C `readv` and `readv_into` path separates `batch_concurrency` from
  per-object `part_concurrency`, `chunk_size`, and `coalesce_gap`.
- `readv_into` lets native consumers fill caller-owned buffers without routing
  through an R raw vector.

## Review items

### 1. R range batching

Status: `implemented for numeric many-ranges and list-of-ranges shapes`

The design says range reads are the vectorized form of `fs_read()` and should
support both many ranges from one object and many ranges per path:

```r
fs_read(fs, "x.bin", offset = c(0, 4096), size = c(512, 512))
fs_read(fs, c("a.bin", "b.bin"), offset = list(c(0, 4096), c(0)), size = list(c(100, 100), c(200)))
```

The parser supports one range per path, many ranges for a scalar path, and
list-of-range arguments aligned with multiple paths. The executor remains flat
and ordered, while materialization reshapes for `result = "auto"`, `"flat"`, and
`"nested"`.

Internal representation:

```rust
struct ReadRequest {
    path_index: usize,
    range_index: usize,
    path: String,
    offset: u64,
    size: Option<u64>,
    error: Option<String>,
}

struct ReadShape {
    n_paths: usize,
    ranges_per_path: Vec<usize>,
}
```

The executor stays flat and ordered. The materializer reshapes from `ReadShape`
for `result = "auto"`, `"flat"`, and `"nested"`.

### 2. R `batch_concurrency = 0` should not mean default

Status: `implemented for R API; C zero-as-unset retained`

For the R API, `NULL` means default, `1L` forces serial execution, and values
greater than one request explicit fanout. Explicit zero is an argument error. The
C API keeps zero-as-unset because public C option structs are zero-initialized by
convention.

Implemented split:

```rust
fn batch_concurrency_limit(value: Option<f64>, n: usize) -> savvy::Result<usize>;
fn c_batch_concurrency(value: usize, n: usize) -> usize;
```

### 3. Vectorized read path errors should preserve shape

Status: `implemented for normalized read requests after shape parsing`

Metadata and namespace batch operations already tend to place per-element invalid
path errors into the corresponding result slot. Vectorized read should follow the
same shape-preserving rule once the input shape is known. Hard R errors should be
reserved for impossible argument shapes, type errors, negative sizes, both `size`
and `end`, unsafe pointer misuse, serializer exceptions, and internal bugs.

### 4. C write tuning

Status: `implemented for part_concurrency/chunk_size; unsupported write headers reject explicitly`

The public C `ropendal_write_options_t` exposes `part_concurrency` and
`chunk_size`, and these now pass into the tuning-aware write path. Conditional
write headers and content headers are still not implemented; non-null fields
return `ROPENDAL_UNSUPPORTED` rather than being silently ignored.

### 5. C callback semantics need tightening

The C callback type receives a `ropendal_aio_t *`, but several submission paths
notify with `NULL`. Either pass the actual Aio pointer consistently or de-emphasize
callbacks in favor of `ropendal_aio_notify()` and `ropendal_monitor_add_aio()`.
The monitor/CV API should remain the canonical many-Aio completion path.

### 6. `ropendal_aio_wait(timeout_ms)` should honor timeout

Status: `implemented with polling wait semantics`

The C wait signature exposes a timeout. Implemented semantics:

- `timeout_ms < 0`: wait indefinitely;
- `timeout_ms == 0`: poll/no wait;
- `timeout_ms > 0`: bounded wait.

### 7. Capability rows need range and concurrency semantics

Status: `implemented for core range/concurrency/recursive rows`

`fs_capabilities()` exposes `read_range`, `read_concurrent`,
`write_concurrent`, `list_recursive`, `delete_recursive`, and `append`, with
implementation and semantics labels. Range-heavy adapters such as ZIP, Zarr-like
stores, BAM/Tabix readers, and archive readers need this declarative contract.

## Remaining patch order

1. Tighten C callback semantics: either pass the actual Aio pointer consistently
   or document/de-emphasize callbacks in favor of `ropendal_aio_notify()` and
   monitors.
2. Add more C tests for timeout behavior and unsupported write header rejection.
3. Expand service capability tests as new providers or adapters are added.

## Test cases to add

```r
expect_equal(
  fs_read(fs, "a.bin", offset = c(0, 2), size = c(2, 2), result = "flat"),
  list(as.raw(c(1, 2)), as.raw(c(3, 4)))
)

nested <- fs_read(
  fs,
  c("a.bin", "b.bin"),
  offset = list(c(0, 2), c(1)),
  size = list(c(1, 1), c(2)),
  result = "nested"
)
expect_equal(length(nested), 2)
expect_equal(length(nested[[1]]), 2)
expect_equal(length(nested[[2]]), 1)

bytes_nested <- fs_read_bytes(
  fs,
  c("a.bin", "b.bin"),
  offset = list(c(0, 2), c(1)),
  size = list(c(1, 1), c(2)),
  result = "nested"
)
expect_true(inherits(bytes_nested[[1]][[1]], "OpendalBytes"))

expect_error(
  fs_read(fs, "a.bin", batch_concurrency = 0),
  "batch_concurrency"
)
```

The goal is not more public API surface. The goal is to make the existing
R-native verb surface fully express many-path and many-range workloads while
keeping the C readv/readv_into path as the explicit native counterpart.
