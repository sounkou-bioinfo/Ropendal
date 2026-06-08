# Opinionated abstract filesystem access for R

Ropendal is an opinionated abstract filesystem access layer for R, backed by
Apache OpenDAL. It should borrow the useful storage-abstraction idea from
systems like fsspec, but not copy Python ergonomics. The package should remain
R-native: verb-first, byte-first, async-aware, capability-aware, and friendly to
native R package authors through both ALTREP raw vectors and a stable pure C API.

## Positioning

Ropendal is not:

- a Python fsspec clone;
- a cloud SDK wrapper;
- an R6 object store with `$read()` / `$write()` as the primary surface;
- a format library for Zarr, Arrow, BAM, ZIP, or Parquet.

Ropendal is:

- a byte-oriented filesystem substrate;
- a common operation contract across local, HTTP, object-store, and adapter
  filesystems;
- an async substrate modeled closer to nanonext than to Python event-loop
  ergonomics;
- a C-facing integration layer through both R vectors/ALTREP and a pure C ABI;
- a foundation on which compatibility layers such as R connections, ALTREP raw
  vectors, ZIP filesystems, byte stores, caches, and domain-specific readers can
  be layered.

A useful one-line description:

> Opinionated abstract filesystem access for R: byte-first, async-capable,
> capability-aware, and native-package friendly.

## API style: prefer `function(object, ...)`

The primary API should remain:

```r
fs_read(fs, "x.bin")
fs_write(fs, "x.bin", raw)
fs_ls(fs, "prefix/")
fs_stat(fs, "x.bin")
fs_read_aio(fs, "x.bin")
```

not:

```r
fs$read("x.bin")
fs$write("x.bin", raw)
fs$list("prefix/")
```

Rationale:

1. **R dispatch works on verbs.** `fs_read(object, ...)` can become or remain a
   generic over `OpendalFs`, ZIP adapters, cache adapters, byte stores, and
   future S7 interfaces.
2. **Pipes work naturally.**

   ```r
   opendal("s3", bucket = "lab-data", root = "project", auth = credentials_s3(...)) |>
     fs_read("samples/HG001.bam", offset = offsets, size = sizes, result = "flat")
   ```

3. **Vectorization is clearer.** Ropendal's core verbs are shape-preserving
   vectorized operations over paths, ranges, and payloads. This is more natural
   in R as `fs_read(fs, path, offset, size, ...)`.
4. **Capabilities are runtime properties.** A handle may support different
   operations depending on backend, adapter, auth, object state, or declared
   semantics. `fs_capabilities(fs)` should report the effective contract.
5. **Adapters compose.** `fs_zip(fs, "archive.zip")`, `fs_cache(fs, ...)`, and
   `byte_store(fs, prefix = ...)` can all feed the same verb surface.

## Core layers

Keep the architecture layered:

```text
Core:
  OpendalFs
  OpendalAio
  OpendalBytes
  fs_* verbs
  monitor/CV primitives
  ALTREP raw facade
  pure C API

Compatibility adapters:
  R connections
  byte/chunk store
  cache layers
  ZIP/archive filesystem adapters

Domain adapters:
  Zarr arrays
  Arrow/Parquet readers
  Bioconductor range readers
  geospatial/image readers
```

The core must remain byte-first. R objects, R closures, and SEXP values should
not enter background Tokio/OpenDAL work. Background operations should operate on
owned bytes, metadata, entries, paths, options, booleans, errors, and C-owned
buffers.

## Bytes, ALTREP, and C consumers

The goal is to avoid materializing an R `raw` vector until a caller actually
requires one. `OpendalBytes` should remain the storage/lifetime owner and the
ALTREP raw vector should be a first-class R-vector facade over it.

```r
bytes <- fs_read_bytes(fs, "x.bin")
length(bytes)
fs_write(fs, "copy.bin", bytes)
raw <- as.raw(bytes)              # today: materializes; future: may be ALTREP
```

The right distinction is not "ALTREP for R, C API for true C". The right
distinction is:

1. **ALTREP raw path:** for C consumers that accept a `SEXP` raw vector and use
   the R vector API. These consumers can iterate without forcing a full raw
   materialization if they use `RAW_GET_REGION()`, element access, or compact-
   representation-friendly APIs. Ropendal's ALTREP `data1` can store/protect the
   `OpendalBytes` external holder; `Get_region` can memcpy only the requested
   byte window into the caller's buffer. Full materialization is delayed until a
   pointer-requesting or mutation-requiring access path makes it necessary.
2. **Pure C ABI path:** for downstream packages that explicitly depend on
   Ropendal and want direct async, monitors, readv/read-into, caller-owned
   buffers, or an R-independent ABI boundary. This path is not "more true" than
   ALTREP; it is a different integration contract.

Therefore, ALTREP should be a core C-facing compatibility feature, not a thin
R-only wrapper.

### Proposed ALTREP behavior

The ALTREP raw class should store an external pointer or protected object in
`data1` that owns the `OpendalBytes` lifetime. Optional `data2` can store a
materialized raw cache, range index, or flags.

Suggested behavior:

```text
Length(x):
  return ropendal_bytes_len(data1)

Elt(x, i):
  copy one byte from OpendalBytes; no full materialization

Get_region(x, i, n, buf):
  copy requested region from OpendalBytes into buf; no full materialization

Dataptr_or_null(x):
  return a stable read-only pointer only when the held bytes are contiguous and
  the external holder in `data1` keeps them alive; otherwise return NULL

Dataptr(x, writable):
  if writable:
    materialize into an R-owned RAWSXP stored in `data2` and return that pointer
  else if a stable contiguous read-only pointer is available:
    return that pointer while `data1` keeps the holder alive
  else:
    materialize into the R-owned RAWSXP cache and return that pointer
```

The important distinction is `Dataptr(writable = TRUE)`: writable raw-vector
access must never point into OpenDAL/Rust `Buffer` storage. Read-only pointer
access can be valid when the buffer is contiguous and the ALTREP object keeps the
external byte holder alive through `data1`; non-contiguous buffers should keep
using `Elt`/`Get_region` or materialize. C code can always violate contracts by
writing through a read-only pointer, but that is not a valid Ropendal/R vector
API use. This still gives `RAW_GET_REGION()`-style consumers efficient block
iteration with bounded copying and no full raw-vector allocation, while code
that requests writable memory crosses the materialization boundary.

### Pure C ABI still matters

The pure C API remains important for packages that do not want to pass through R
vectors at all:

```c
ropendal_read_into_aio(fs, &opts, dst, dst_len, &aio, &err);
ropendal_aio_wait(aio, -1, &err);
ropendal_aio_result_nread(aio, &nread, &err);
```

That is complementary to ALTREP, not a replacement for it. A native package that
already consumes raw `SEXP` values should be able to benefit from the ALTREP
facade. A native package that wants async scheduling, monitors, readv, or
caller-owned buffers can use the C ABI.

Possible C-side additions only if needed after profiling:

```c
ropendal_status_t ropendal_bytes_copy_region(
  const ropendal_bytes_t *bytes,
  size_t offset,
  size_t len,
  uint8_t *dst,
  size_t *nread,
  ropendal_error_t **err
);

ropendal_status_t ropendal_bytes_slice(
  const ropendal_bytes_t *bytes,
  size_t offset,
  size_t len,
  ropendal_bytes_t **out,
  ropendal_error_t **err
);
```

## R connection adapter

R connections are important for compatibility, but they are not the core
abstraction. `readBin()` and `writeBin()` necessarily interact with R's
connection machinery and R vectors. They should be offered as adapters over
Ropendal iterators.

Proposed shape:

```r
con <- fs_connection(
  fs,
  "x.bin",
  open = "rb",
  block_size = 8 * 1024^2,
  max_blocks = 4,
  readahead = 2,
  read_concurrency = NULL,
  cache = c("none", "memory", "file")
)

x <- readBin(con, "raw", n = 65536)
close(con)
```

Writing:

```r
con <- fs_connection(
  fs,
  "out.bin",
  open = "wb",
  chunk_size = 8 * 1024^2,
  write_concurrency = 4
)

writeBin(payload, con)
close(con) # finalize write sink / multipart upload
```

Primitive Ropendal functions should keep returning `opendalErrorValue` for
backend failures. Connection callbacks should signal R conditions, because base
R connection consumers expect bytes or errors, not structured value objects.

## Buffering model

Keep three concepts separate.

### 1. Transfer tuning

Already part of the core operation API:

```r
fs_read(
  fs,
  "big.bin",
  offset = offsets,
  size = sizes,
  batch_concurrency = 32,
  read_concurrency = 4,
  chunk_size = 8 * 1024^2,
  coalesce_gap = 64 * 1024
)
```

This controls execution of one operation. It is not a persistent cache.

### 2. Iterator / connection buffering

Used for sequential reads, `readBin()`, and `seek()` on connection adapters.

Suggested connection options:

```r
fs_connection(
  fs,
  "x.bin",
  open = "rb",
  block_size = 8 * 1024^2,
  max_blocks = 4,
  readahead = 2,
  cache = "memory"
)
```

### 3. Persistent block cache

A future filesystem adapter/layer, with explicit invalidation semantics:

```r
cached <- fs_cache(
  fs,
  cache_dir = tools::R_user_dir("Ropendal", "cache"),
  block_size = 8 * 1024^2,
  validate = c("etag", "version", "last_modified_size", "none")
)
```

Do not add implicit full-object downloads for huge remote objects. Hidden
downloads should require explicit cache settings.

## Monitoring system

The monitoring system is a core Ropendal differentiator and should be documented
as such. Ropendal should model this more on nanonext-style Aio, condition
variables, and monitors than on Python fsspec.

The separation should be:

```text
Aio      owns one operation and its eventual result
CV       wakes waiters
Monitor  queues completion events
collect  explicitly materializes or returns the resolved payload
```

Example:

```r
aios <- list(
  a = fs_read_aio(fs, "a.bin"),
  b = fs_read_aio(fs, "b.bin"),
  c = fs_stat_aio(fs, "c.bin")
)

gate <- cv()
mon <- aio_monitor(aios, cv = gate)

repeat {
  cv_until(gate, 100)
  events <- read_monitor(mon)

  if (length(events)) {
    # update progress, schedule follow-up work, inspect state
  }

  if (all(vapply(aios, function(x) x$resolved, logical(1)))) break
}

values <- lapply(aios, collect_aio)
```

Important rule: monitor events should not materialize large payloads. The Aio
owns the result. The user or native package explicitly collects.

## Range reads and bioinformatics

The current `fs_read()` range design is correct and should remain central.

Examples:

```r
fs_read(fs, "x.bin", offset = 100, size = 50)

fs_read(
  fs,
  "x.bin",
  offset = c(0, 4096, 8192),
  size = c(512, 512, 512),
  result = "flat"
)

fs_read(
  fs,
  path = c("a.bin", "b.bin"),
  offset = list(c(0, 4096), c(0)),
  size = list(c(100, 100), c(200)),
  result = "nested"
)
```

Add a small request object for range-heavy formats:

```r
req <- byte_ranges(
  path = index$path,
  offset = index$offset,
  size = index$size,
  id = index$chunk_id
)

chunks <- fs_read(fs, req, mode = "bytes", result = "flat")
```

This is especially useful for BAM/BAI, CRAM/CRAI, Tabix-indexed VCF/BED/GFF,
BigWig/BigBed, BGZF, FASTA + FAI, tiled arrays, and custom binary indexes.

The C API should remain explicit for range vectors:

```c
ropendal_readv_into_aio(fs, requests, nrequests, buffers, nbuffers, &aio, &err);
```

## ZIP as a filesystem adapter

ZIP support should be first-class, including ZIP over HTTP/S3/GCS/Azure/GDrive,
but ZIP should be modeled as a filesystem adapter over a parent filesystem, not
as a codec.

Preferred R shape:

```r
s3 <- opendal("s3", bucket = "datasets", root = "project", auth = credentials_s3(...))
zfs <- fs_zip(s3, "runs/run-001.zip")

fs_ls(zfs)
fs_stat(zfs, "qc/summary.tsv")
fs_read(zfs, "qc/summary.tsv")
```

Pipe-friendly:

```r
opendal("s3", bucket = "datasets", root = "project", auth = credentials_s3(...)) |>
  fs_zip("runs/run-001.zip") |>
  fs_read("qc/summary.tsv")
```

Do not make the primary interface a Python-like URL chain. URI sugar can come
later, but object composition should be primary.

### ZIP index

Remote ZIP needs an index loaded from the end-of-central-directory and central
directory records. Opening should be lazy by default, with explicit async index
loading when desired.

```r
zfs <- fs_zip(fs, "archive.zip", index = "lazy")

idx <- fs_zip_index(fs, "archive.zip")
zfs <- fs_zip(fs, "archive.zip", index = idx)

idx_aio <- fs_zip_index_aio(fs, "archive.zip")
idx <- collect_aio(idx_aio)
zfs <- fs_zip(fs, "archive.zip", index = idx)
```

Index cache keys should include archive path, size, etag/version if available,
last-modified if available, central-directory offset/size, ZIP64 status, and
entry table digest.

### ZIP capabilities

Start read-only.

| Operation | Initial support | Semantics |
|---|---:|---|
| `fs_ls()` | yes | central-directory index |
| `fs_stat()` | yes | central-directory metadata |
| `fs_exists()` | yes | index lookup |
| `fs_read()` | yes | stored direct range or deflate decode |
| `fs_read_bytes()` | yes | immutable bytes |
| `fs_read_iter()` | partial | direct for stored entries; streaming decode for deflate |
| `fs_write()` | no | archive rewrite required |
| `fs_replace()` | no | archive rewrite required |
| `fs_append()` | no | unsafe or backend-dependent |
| `fs_delete()` | no | archive rewrite required |
| `fs_rename()` | no | archive rewrite required |

Unsupported compression methods, encrypted entries, multi-disk archives, and
central-directory encryption should return `opendalUnsupportedValue`.

### ZIP C API

Expose ZIP as another filesystem-compatible adapter:

```c
typedef struct ropendal_zip_options {
  size_t struct_size;
  const char *archive_path;
  const char *root;
  size_t index_cache_bytes;
  size_t block_size;
  int strict_crc;
} ropendal_zip_options_t;

ropendal_status_t ropendal_fs_zip(
  ropendal_fs_t *parent,
  const ropendal_zip_options_t *opts,
  ropendal_fs_t **out,
  ropendal_error_t **err
);
```

Native consumers then use existing read/stat/list/Aio/monitor APIs against the
returned `ropendal_fs_t`.

## Byte store / chunk store

Ropendal should expose a R-native key-to-bytes store for Zarr-like layouts, but
avoid Python mapper ergonomics as the primary API.

Preferred shape:

```r
store <- byte_store(fs, prefix = "array.zarr")

store_read(store, "zarr.json")
store_write(store, "c/0/0", chunk_raw)
store_exists(store, "c/0/0")
store_list(store, "c/", recursive = TRUE)
store_delete(store, "c/0/0")
```

Vectorized:

```r
keys <- sprintf("c/0/%d", 0:99)

chunks <- store_read(
  store,
  keys,
  mode = "bytes",
  batch_concurrency = 32
)

store_write(
  store,
  keys,
  chunks,
  batch_concurrency = 32
)
```

ZIP composition should work naturally:

```r
store <- opendal("s3", bucket = "datasets", root = "arrays", auth = credentials_s3(...)) |>
  fs_zip("array.zarr.zip") |>
  byte_store(prefix = "array.zarr")

meta <- store_read(store, "zarr.json")
chunk <- store_read(store, "c/0/0", mode = "bytes")
```

Core Ropendal should not implement full Zarr array semantics. It should provide
the filesystem/byte-store substrate; Zarr, Arrow, Bioconductor, or geospatial
adapters can live on top.

## Suggested milestones

1. Document the opinionated architecture and keep `function(object, ...)`.
2. Stabilize `OpendalBytes` lifetime, ALTREP ownership, and C-level byte rules.
3. Make `as.raw.OpendalBytes()` eligible to return a first-class ALTREP raw
   integration layer, with `Get_region` avoiding full materialization.
4. Add `byte_ranges()` request objects feeding `fs_read()`.
5. Add `fs_connection()` backed by read/write iterators.
6. Add read-only `fs_zip()` over any range-readable parent filesystem.
7. Add `ropendal_fs_zip()` to the C API.
8. Add `byte_store()` / `chunk_store()` with vectorized store operations.
9. Add explicit memory/file block cache adapter once invalidation is designed.
10. Build targeted integrations: BioC range readers, Arrow/Parquet, and Zarr-like
    chunk stores.
