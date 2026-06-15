#ifndef ROPENDAL_H
#define ROPENDAL_H

/*
 * Public C API for Ropendal.
 *
 * This header is installed as inst/include/ropendal.h. It is a pure C interface:
 * it does not include R headers or R-specific object types, and does not require
 * C callers to use R's C API. Synchronous behavior is built by submitting an async
 * operation and waiting on the returned ropendal_aio_t.
 *
 * Allocation and ownership follow one rule: any function that writes a non-NULL
 * handle through an out parameter transfers one ownership reference to the
 * caller. ropendal_fs_open(), ropendal_fs_from_uri(), ropendal_store_open(),
 * ropendal_cv_alloc(), ropendal_monitor_create(), and the *_aio() submission
 * functions allocate such handles. The caller must eventually release each owned
 * reference with the matching release function: ropendal_fs_release(),
 * ropendal_store_release(), ropendal_cv_release(), ropendal_monitor_release(),
 * ropendal_bytes_release(), or ropendal_aio_release(). ropendal_fs_retain() and
 * ropendal_store_retain() create one additional reference that must also be
 * released.
 * Passing NULL to a release function is allowed and has no effect.
 *
 * Errors are also allocated values. When a function accepts ropendal_error_t
 * **err and returns a non-OK status, it may store a newly allocated error in
 * *err. The caller owns that error and must release it with
 * ropendal_error_release(). Passing err == NULL is valid when the caller only
 * needs the status code. Strings returned by ropendal_error_message(),
 * ropendal_error_kind(), ropendal_error_operation(), and ropendal_error_path()
 * are borrowed from the error object and become invalid after release.
 *
 * Input strings, config key/value arrays, and option structs are borrowed only
 * for the duration of the call that receives them. Ropendal copies the data it
 * needs before returning from a successful submission call. For write, replace,
 * and append submissions, src bytes are copied before the function returns, so
 * the caller may reuse or free src immediately after a successful call. For
 * read_into and readv_into submissions, destination buffers remain owned by the
 * caller and must stay valid and writable until the Aio reaches a terminal state
 * and the caller has inspected the byte count/result. readv_into destination
 * buffers must not overlap because requests may be filled concurrently.
 *
 * Result pointers returned by ropendal_aio_result_bytes(),
 * ropendal_aio_result_readv(), ropendal_aio_result_entry(),
 * ropendal_aio_result_entries(), and ropendal_monitor_read() are borrowed. They
 * remain valid only while the owning Aio or monitor remains alive, and monitor
 * event slices may be invalidated by a later monitor read.
 *
 * Every public option/result struct starts with struct_size. Callers should
 * zero-initialize the struct and set struct_size = sizeof(struct_value). Fields
 * left as zero or NULL mean "unset" unless the field documentation says
 * otherwise. This lets newer Ropendal versions detect older callers safely.
 *
 * Callbacks and notifications may run on worker threads. They MUST NOT call R's
 * C API unless the caller has independently arranged a safe handoff to R's main
 * thread. Callback aio arguments are borrowed pointers to the completed Aio and
 * do not transfer ownership. Registering a callback retains the Aio internally
 * until the callback has run, so callback userdata must remain valid until that
 * callback runs even if the caller releases its Aio handle earlier. Treat
 * callbacks as completion notifications only; inspect results via
 * ropendal_aio_wait(), ropendal_aio_poll(), and the result accessors.
 */

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct ropendal_fs ropendal_fs_t;
typedef struct ropendal_store ropendal_store_t;
typedef struct ropendal_aio ropendal_aio_t;
typedef struct ropendal_bytes ropendal_bytes_t;
typedef struct ropendal_error ropendal_error_t;
typedef struct ropendal_cv ropendal_cv_t;
typedef struct ropendal_monitor ropendal_monitor_t;

typedef enum ropendal_status {
  ROPENDAL_OK = 0,
  ROPENDAL_ERR = 1,
  ROPENDAL_INVALID_ARGUMENT = 2,
  ROPENDAL_UNSUPPORTED = 3,
  ROPENDAL_NOT_FOUND = 4,
  ROPENDAL_PERMISSION_DENIED = 5,
  ROPENDAL_CONDITION_NOT_MATCH = 6,
  ROPENDAL_CANCELLED = 7,
  ROPENDAL_TIMEOUT = 8
} ropendal_status_t;

typedef enum ropendal_aio_status {
  ROPENDAL_AIO_PENDING = 0,
  ROPENDAL_AIO_READY = 1,
  ROPENDAL_AIO_ERROR = 2,
  ROPENDAL_AIO_CANCELLED = 3
} ropendal_aio_status_t;

typedef enum ropendal_entry_mode {
  ROPENDAL_ENTRY_UNKNOWN = 0,
  ROPENDAL_ENTRY_FILE = 1,
  ROPENDAL_ENTRY_DIR = 2
} ropendal_entry_mode_t;

typedef enum ropendal_event_kind {
  ROPENDAL_EVENT_AIO_READY = 1,
  ROPENDAL_EVENT_AIO_ERROR = 2,
  ROPENDAL_EVENT_AIO_CANCELLED = 3,
  ROPENDAL_EVENT_FS_CLOSED = 4
} ropendal_event_kind_t;

typedef enum ropendal_store_cache_validate {
  ROPENDAL_STORE_CACHE_VALIDATE_LAST_MODIFIED_SIZE = 0,
  ROPENDAL_STORE_CACHE_VALIDATE_NONE = 1
} ropendal_store_cache_validate_t;

typedef void (*ropendal_aio_callback_t)(ropendal_aio_t *aio, void *userdata);

typedef struct ropendal_kv {
  size_t struct_size;
  const char *key;
  const char *value;
} ropendal_kv_t;

typedef struct ropendal_monitor_event {
  size_t struct_size;
  ropendal_event_kind_t kind;
  ropendal_aio_t *aio;
  uint64_t id;
} ropendal_monitor_event_t;

/* Byte-store options. Store keys are normalized relative to the store prefix. */
typedef struct ropendal_store_options {
  size_t struct_size;
  /* Optional directory prefix relative to the filesystem root. NULL/empty means root. */
  const char *prefix;
} ropendal_store_options_t;

typedef struct ropendal_store_cache_options {
  size_t struct_size;
  /* Zero/default validates cached objects by parent last-modified plus size. */
  ropendal_store_cache_validate_t validate;
} ropendal_store_cache_options_t;

typedef struct ropendal_store_block_cache_options {
  size_t struct_size;
  /* 0 means the default 8 MiB block size. */
  uint64_t block_size;
  /* Zero/default validates cached blocks by parent last-modified plus size. */
  ropendal_store_cache_validate_t validate;
} ropendal_store_block_cache_options_t;

typedef struct ropendal_store_read_options {
  size_t struct_size;
  const char *key;
  /* has_offset preserves explicit offset 0 versus unset for forward-compatible callers. */
  int has_offset;
  uint64_t offset;
  int has_size;
  uint64_t size;
  size_t part_concurrency;
  size_t chunk_size;
  size_t coalesce_gap;
  ropendal_aio_callback_t callback;
  void *userdata;
} ropendal_store_read_options_t;

typedef struct ropendal_store_write_options {
  size_t struct_size;
  const char *key;
  size_t part_concurrency;
  size_t chunk_size;
  ropendal_aio_callback_t callback;
  void *userdata;
} ropendal_store_write_options_t;

typedef struct ropendal_store_ls_options {
  size_t struct_size;
  const char *path;
  int recursive;
  size_t limit;
  const char *start_after;
  ropendal_aio_callback_t callback;
  void *userdata;
} ropendal_store_ls_options_t;

typedef struct ropendal_store_delete_options {
  size_t struct_size;
  const char *key;
  int recursive;
  ropendal_aio_callback_t callback;
  void *userdata;
} ropendal_store_delete_options_t;

typedef struct ropendal_read_options {
  size_t struct_size;
  const char *path;
  uint64_t offset;
  uint64_t size;
  int has_size;
  uint64_t content_length_hint;
  int has_content_length_hint;
  const char *version;
  const char *if_match;
  const char *if_none_match;
  /* Per-object chunk parallelism. Maps to OpenDAL ReadOptions.concurrent. */
  size_t part_concurrency;
  /* Requested chunk size in bytes. 0 means backend/OpenDAL default. */
  size_t chunk_size;
  /* Merge nearby byte ranges when the gap is <= this many bytes. 0 means unset. */
  size_t coalesce_gap;
  /* Streaming reader prefetch depth. 0 means strict back-pressure/default. */
  size_t prefetch;
  ropendal_aio_callback_t callback;
  void *userdata;
} ropendal_read_options_t;

typedef struct ropendal_read_request {
  size_t struct_size;
  const char *path;
  uint64_t offset;
  uint64_t size;
  int has_size;
  uint64_t content_length_hint;
  int has_content_length_hint;
  const char *version;
} ropendal_read_request_t;

typedef struct ropendal_read_into_request {
  size_t struct_size;
  const char *path;
  uint64_t offset;
  uint64_t size;
  int has_size;
  uint8_t *dst;
  size_t dst_len;
} ropendal_read_into_request_t;

typedef struct ropendal_readv_options {
  size_t struct_size;
  /* Number of independent range requests in flight. */
  size_t batch_concurrency;
  /* Per-object chunk fanout for large individual ranges. */
  size_t part_concurrency;
  size_t chunk_size;
  size_t coalesce_gap;
  /* Reserved; current result accessors report request order. */
  int preserve_order;
  ropendal_aio_callback_t callback;
  void *userdata;
} ropendal_readv_options_t;

typedef struct ropendal_readv_result {
  size_t struct_size;
  size_t index;
  ropendal_status_t status;
  size_t nread;
  const char *kind;
  const char *message;
  const char *path;
} ropendal_readv_result_t;

typedef struct ropendal_write_options {
  size_t struct_size;
  const char *path;
  /* Reserved conditional/content options. Non-NULL currently returns ROPENDAL_UNSUPPORTED. */
  const char *if_match;
  const char *if_none_match;
  const char *content_type;
  const char *content_encoding;
  const char *content_disposition;
  const char *cache_control;
  /* Multipart/write-task parallelism where supported. */
  size_t part_concurrency;
  /* Requested chunk/part size in bytes. 0 means backend/OpenDAL default. */
  size_t chunk_size;
  ropendal_aio_callback_t callback;
  void *userdata;
} ropendal_write_options_t;

typedef struct ropendal_ls_options {
  size_t struct_size;
  const char *path;
  int recursive;
  size_t limit;
  const char *start_after;
  int versions;
  int deleted;
  ropendal_aio_callback_t callback;
  void *userdata;
} ropendal_ls_options_t;

typedef struct ropendal_delete_options {
  size_t struct_size;
  const char *path;
  int recursive;
  const char *version;
  ropendal_aio_callback_t callback;
  void *userdata;
} ropendal_delete_options_t;

typedef struct ropendal_entry {
  size_t struct_size;
  const char *path;
  const char *name;
  ropendal_entry_mode_t mode;
  uint64_t content_length;
  int has_content_length;
  const char *etag;
  const char *content_type;
  const char *content_encoding;
  const char *last_modified;
  const char *version;
} ropendal_entry_t;

/*
 * API / filesystem handle lifecycle.
 *
 * ropendal_api_version() returns the C ABI version supported by the loaded
 * shared library. ropendal_fs_open() and ropendal_fs_from_uri() allocate a new
 * filesystem handle with reference count 1 on success and store it in *out. The
 * caller owns that reference. The scheme/config/uri strings are copied during
 * the call and do not need to outlive it. A filesystem handle may be released
 * after an Aio has been successfully submitted; the Aio keeps the native
 * operator/runtime alive for its own work.
 */
uint32_t ropendal_api_version(void);
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

/*
 * Byte-store lifecycle and async operations.
 *
 * A store is a lightweight key-to-bytes view over a filesystem plus a normalized
 * prefix. Store operations never serialize R objects and never call R's C API.
 * Write submits create-only semantics; replace overwrites or creates according
 * to the backend. read_into writes directly into caller-owned memory, which must
 * remain valid until the Aio reaches a terminal state. Returned entry paths from
 * store_ls_aio() are relative to the store prefix. ropendal_store_cache_open()
 * creates an explicit full-object cache adapter from an uncached parent store
 * plus an uncached cache store supplied by the caller; partial reads bypass that
 * cache. ropendal_store_block_cache_open() creates an explicit range-aware block
 * cache adapter over uncached stores; reads are assembled from fixed-size cached
 * byte blocks, and mutations through the adapter invalidate affected blocks.
 */
ropendal_status_t ropendal_store_open(ropendal_fs_t *fs,
                                      const ropendal_store_options_t *opts,
                                      ropendal_store_t **out,
                                      ropendal_error_t **err);
ropendal_status_t ropendal_store_cache_open(ropendal_store_t *parent,
                                            ropendal_store_t *cache,
                                            const ropendal_store_cache_options_t *opts,
                                            ropendal_store_t **out,
                                            ropendal_error_t **err);
ropendal_status_t ropendal_store_block_cache_open(ropendal_store_t *parent,
                                                  ropendal_store_t *cache,
                                                  const ropendal_store_block_cache_options_t *opts,
                                                  ropendal_store_t **out,
                                                  ropendal_error_t **err);
void ropendal_store_retain(ropendal_store_t *store);
void ropendal_store_release(ropendal_store_t *store);

ropendal_status_t ropendal_store_read_aio(ropendal_store_t *store,
                                          const ropendal_store_read_options_t *opts,
                                          ropendal_aio_t **out,
                                          ropendal_error_t **err);
ropendal_status_t ropendal_store_read_into_aio(ropendal_store_t *store,
                                               const ropendal_store_read_options_t *opts,
                                               uint8_t *dst,
                                               size_t dst_len,
                                               ropendal_aio_t **out,
                                               ropendal_error_t **err);
ropendal_status_t ropendal_store_write_aio(ropendal_store_t *store,
                                           const ropendal_store_write_options_t *opts,
                                           const uint8_t *src,
                                           size_t src_len,
                                           ropendal_aio_t **out,
                                           ropendal_error_t **err);
ropendal_status_t ropendal_store_replace_aio(ropendal_store_t *store,
                                             const ropendal_store_write_options_t *opts,
                                             const uint8_t *src,
                                             size_t src_len,
                                             ropendal_aio_t **out,
                                             ropendal_error_t **err);
ropendal_status_t ropendal_store_exists_aio(ropendal_store_t *store,
                                            const char *key,
                                            ropendal_aio_callback_t callback,
                                            void *userdata,
                                            ropendal_aio_t **out,
                                            ropendal_error_t **err);
ropendal_status_t ropendal_store_ls_aio(ropendal_store_t *store,
                                        const ropendal_store_ls_options_t *opts,
                                        ropendal_aio_t **out,
                                        ropendal_error_t **err);
ropendal_status_t ropendal_store_delete_aio(ropendal_store_t *store,
                                            const ropendal_store_delete_options_t *opts,
                                            ropendal_aio_t **out,
                                            ropendal_error_t **err);

/*
 * Native byte codecs.
 *
 * ropendal_codec_encode() and ropendal_codec_decode() are synchronous, pure-C
 * byte transforms for codecs that do not touch R's C API. The current built-in
 * codecs are "identity", "gzip", and "zlib". src is borrowed only for the
 * duration of the call and may be NULL only when src_len == 0. On success, *out
 * owns an immutable byte handle that must be released with
 * ropendal_bytes_release(). ropendal_bytes_data() returns a borrowed pointer
 * valid until that release; empty byte handles return NULL with length 0.
 */
ropendal_status_t ropendal_codec_encode(const char *codec,
                                        const uint8_t *src,
                                        size_t src_len,
                                        ropendal_bytes_t **out,
                                        ropendal_error_t **err);
ropendal_status_t ropendal_codec_decode(const char *codec,
                                        const uint8_t *src,
                                        size_t src_len,
                                        ropendal_bytes_t **out,
                                        ropendal_error_t **err);
const uint8_t *ropendal_bytes_data(const ropendal_bytes_t *bytes);
size_t ropendal_bytes_len(const ropendal_bytes_t *bytes);
void ropendal_bytes_release(ropendal_bytes_t *bytes);

/*
 * Async filesystem operations.
 *
 * Each successful *_aio() call stores a newly allocated Aio in *out. The caller
 * owns the Aio and must release it with ropendal_aio_release(), even after the
 * operation has completed, failed, or been cancelled. Submission functions copy
 * path strings and options before returning. Operations that are not implemented
 * for the current C API or backend return ROPENDAL_UNSUPPORTED and do not
 * allocate an Aio.
 */
ropendal_status_t ropendal_read_aio(ropendal_fs_t *fs,
                                    const ropendal_read_options_t *opts,
                                    ropendal_aio_t **out,
                                    ropendal_error_t **err);

/* Caller owns dst and must keep it valid until aio reaches a terminal state. */
ropendal_status_t ropendal_read_into_aio(ropendal_fs_t *fs,
                                         const ropendal_read_options_t *opts,
                                         uint8_t *dst,
                                         size_t dst_len,
                                         ropendal_aio_t **out,
                                         ropendal_error_t **err);

/*
 * Submit multiple independent range reads. Completion is a vector outcome:
 * ropendal_aio_result_bytes() returns successful read payloads concatenated in
 * request order, ropendal_aio_result_nread() returns the concatenated byte
 * count, and ropendal_aio_result_readv() returns one per-request result in the
 * same order. Byte offsets within the flattened payload are the cumulative
 * nread values of previous result entries. Per-request backend failures do not
 * fail the whole Aio; inspect each result status.
 */
ropendal_status_t ropendal_readv_aio(ropendal_fs_t *fs,
                                     const ropendal_read_request_t *requests,
                                     size_t n_requests,
                                     const ropendal_readv_options_t *opts,
                                     ropendal_aio_t **out,
                                     ropendal_error_t **err);

/*
 * Each request owns dst and must keep it valid and non-overlapping until aio
 * reaches a terminal state. ropendal_aio_result_nread() returns the total bytes
 * copied across successful requests; ropendal_aio_result_readv() returns
 * per-request status/byte-count details in request order. Per-request backend
 * failures do not fail the whole Aio; inspect each result status.
 */
ropendal_status_t ropendal_readv_into_aio(ropendal_fs_t *fs,
                                          const ropendal_read_into_request_t *requests,
                                          size_t n_requests,
                                          const ropendal_readv_options_t *opts,
                                          ropendal_aio_t **out,
                                          ropendal_error_t **err);

/* Create/write a new file/object. Existing target resolves to AlreadyExists. */
ropendal_status_t ropendal_write_aio(ropendal_fs_t *fs,
                                     const ropendal_write_options_t *opts,
                                     const uint8_t *src,
                                     size_t src_len,
                                     ropendal_aio_t **out,
                                     ropendal_error_t **err);

/* Replace an existing file/object or create it if the service profile documents that behavior. */
ropendal_status_t ropendal_replace_aio(ropendal_fs_t *fs,
                                       const ropendal_write_options_t *opts,
                                       const uint8_t *src,
                                       size_t src_len,
                                       ropendal_aio_t **out,
                                       ropendal_error_t **err);

/* Append is a distinct operation and is supported only when declared by the profile. */
ropendal_status_t ropendal_append_aio(ropendal_fs_t *fs,
                                      const ropendal_write_options_t *opts,
                                      const uint8_t *src,
                                      size_t src_len,
                                      ropendal_aio_t **out,
                                      ropendal_error_t **err);

ropendal_status_t ropendal_stat_aio(ropendal_fs_t *fs,
                                    const char *path,
                                    ropendal_aio_callback_t callback,
                                    void *userdata,
                                    ropendal_aio_t **out,
                                    ropendal_error_t **err);

ropendal_status_t ropendal_exists_aio(ropendal_fs_t *fs,
                                      const char *path,
                                      ropendal_aio_callback_t callback,
                                      void *userdata,
                                      ropendal_aio_t **out,
                                      ropendal_error_t **err);

ropendal_status_t ropendal_ls_aio(ropendal_fs_t *fs,
                                  const ropendal_ls_options_t *opts,
                                  ropendal_aio_t **out,
                                  ropendal_error_t **err);

ropendal_status_t ropendal_delete_aio(ropendal_fs_t *fs,
                                      const ropendal_delete_options_t *opts,
                                      ropendal_aio_t **out,
                                      ropendal_error_t **err);

ropendal_status_t ropendal_copy_aio(ropendal_fs_t *fs,
                                    const char *from,
                                    const char *to,
                                    ropendal_aio_callback_t callback,
                                    void *userdata,
                                    ropendal_aio_t **out,
                                    ropendal_error_t **err);

ropendal_status_t ropendal_rename_aio(ropendal_fs_t *fs,
                                      const char *from,
                                      const char *to,
                                      ropendal_aio_callback_t callback,
                                      void *userdata,
                                      ropendal_aio_t **out,
                                      ropendal_error_t **err);

ropendal_status_t ropendal_mkdir_aio(ropendal_fs_t *fs,
                                     const char *path,
                                     ropendal_aio_callback_t callback,
                                     void *userdata,
                                     ropendal_aio_t **out,
                                     ropendal_error_t **err);

/*
 * Condition variables / notifications for multi-aio coordination.
 *
 * ropendal_aio_notify() retains the Aio and CV until the Aio reaches a
 * terminal state, caches the result on the Aio, and signals the CV. A monitor
 * retains its CV and every Aio added to it until ropendal_monitor_release().
 * ropendal_monitor_read() drains queued completion events into a
 * monitor-owned snapshot valid until the next monitor read or monitor release.
 * monitor_release() waits for registered notification workers to exit before
 * releasing retained Aio references.
 */
ropendal_status_t ropendal_cv_alloc(ropendal_cv_t **out, ropendal_error_t **err);
void ropendal_cv_release(ropendal_cv_t *cv);
ropendal_status_t ropendal_cv_wait(ropendal_cv_t *cv, ropendal_error_t **err);
ropendal_status_t ropendal_cv_until(ropendal_cv_t *cv, int timeout_ms, ropendal_error_t **err);
uint64_t ropendal_cv_value(ropendal_cv_t *cv);
void ropendal_cv_reset(ropendal_cv_t *cv);
void ropendal_cv_signal(ropendal_cv_t *cv);

ropendal_status_t ropendal_aio_notify(ropendal_aio_t *aio,
                                      ropendal_cv_t *cv,
                                      uint64_t id,
                                      ropendal_error_t **err);

ropendal_status_t ropendal_monitor_create(ropendal_cv_t *cv,
                                          ropendal_monitor_t **out,
                                          ropendal_error_t **err);
ropendal_status_t ropendal_monitor_add_aio(ropendal_monitor_t *monitor,
                                           ropendal_aio_t *aio,
                                           uint64_t id,
                                           ropendal_error_t **err);
ropendal_status_t ropendal_monitor_read(ropendal_monitor_t *monitor,
                                        const ropendal_monitor_event_t **events,
                                        size_t *len,
                                        ropendal_error_t **err);
void ropendal_monitor_release(ropendal_monitor_t *monitor);

/*
 * Aio lifecycle / completion.
 *
 * poll is non-blocking. wait uses timeout_ms as follows: negative waits
 * indefinitely, zero polls without waiting, and positive values bound the wait
 * in milliseconds. On timeout, wait returns ROPENDAL_TIMEOUT without resolving
 * or cancelling the Aio. cancel requests cancellation, but remote I/O may already
 * have happened. Releasing an Aio drops
 * the handle and invalidates all borrowed result pointers from that Aio.
 */
ropendal_aio_status_t ropendal_aio_poll(ropendal_aio_t *aio);
ropendal_status_t ropendal_aio_wait(ropendal_aio_t *aio, int timeout_ms, ropendal_error_t **err);
void ropendal_aio_cancel(ropendal_aio_t *aio);
void ropendal_aio_release(ropendal_aio_t *aio);

/*
 * Result extraction.
 *
 * Result accessors wait for completion if needed. Byte and entry pointers are
 * borrowed from the Aio and remain valid until ropendal_aio_release(aio). For
 * read_into/readv_into operations, use ropendal_aio_result_nread() to learn how
 * many bytes were written into caller-owned buffers. For readv_aio(), bytes are
 * the flattened successful payloads described above and result_readv() supplies
 * the per-request lengths/statuses needed to split them.
 */
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
ropendal_status_t ropendal_aio_result_entries(ropendal_aio_t *aio,
                                              const ropendal_entry_t **entries,
                                              size_t *len,
                                              ropendal_error_t **err);
ropendal_status_t ropendal_aio_result_entry(ropendal_aio_t *aio,
                                            const ropendal_entry_t **entry,
                                            ropendal_error_t **err);

/*
 * Error inspection.
 *
 * Error strings are borrowed from err. Copy them if they must outlive
 * ropendal_error_release(err). Releasing NULL is allowed.
 */
const char *ropendal_error_message(const ropendal_error_t *err);
const char *ropendal_error_kind(const ropendal_error_t *err);
const char *ropendal_error_operation(const ropendal_error_t *err);
const char *ropendal_error_path(const ropendal_error_t *err);
void ropendal_error_release(ropendal_error_t *err);

#ifdef __cplusplus
}
#endif

#endif /* ROPENDAL_H */
