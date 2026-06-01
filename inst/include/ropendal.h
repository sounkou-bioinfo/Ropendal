#ifndef ROPENDAL_H
#define ROPENDAL_H

/*
 * Public C API draft for Ropendal.
 *
 * The API is pure C and async-first. It does not include R headers and does not
 * require callers to touch R's C API. Convenience synchronous behavior should be
 * built by submitting an aio and waiting on it.
 *
 * Callbacks may be invoked from worker threads. They MUST NOT call R's C API
 * unless the caller has independently arranged safe handoff to R's main thread.
 */

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct ropendal_fs ropendal_fs_t;
typedef struct ropendal_aio ropendal_aio_t;
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
  int preserve_order;
  ropendal_aio_callback_t callback;
  void *userdata;
} ropendal_readv_options_t;

typedef struct ropendal_write_options {
  size_t struct_size;
  const char *path;
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

/* API / handle lifecycle */
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

/* Async filesystem operations */
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

ropendal_status_t ropendal_readv_aio(ropendal_fs_t *fs,
                                     const ropendal_read_request_t *requests,
                                     size_t n_requests,
                                     const ropendal_readv_options_t *opts,
                                     ropendal_aio_t **out,
                                     ropendal_error_t **err);

/* Each request owns dst and must keep it valid until aio reaches a terminal state. */
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

/* Condition variables / notifications for multi-aio coordination. */
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

/* Aio lifecycle / completion */
ropendal_aio_status_t ropendal_aio_poll(ropendal_aio_t *aio);
ropendal_status_t ropendal_aio_wait(ropendal_aio_t *aio, int timeout_ms, ropendal_error_t **err);
void ropendal_aio_cancel(ropendal_aio_t *aio);
void ropendal_aio_release(ropendal_aio_t *aio);

/* Result extraction. Returned pointers are owned by the aio unless documented otherwise. */
ropendal_status_t ropendal_aio_result_bytes(ropendal_aio_t *aio,
                                            const uint8_t **data,
                                            size_t *len,
                                            ropendal_error_t **err);
ropendal_status_t ropendal_aio_result_nread(ropendal_aio_t *aio,
                                            size_t *nread,
                                            ropendal_error_t **err);
ropendal_status_t ropendal_aio_result_entries(ropendal_aio_t *aio,
                                              const ropendal_entry_t **entries,
                                              size_t *len,
                                              ropendal_error_t **err);
ropendal_status_t ropendal_aio_result_entry(ropendal_aio_t *aio,
                                            const ropendal_entry_t **entry,
                                            ropendal_error_t **err);

/* Error inspection */
const char *ropendal_error_message(const ropendal_error_t *err);
const char *ropendal_error_kind(const ropendal_error_t *err);
const char *ropendal_error_operation(const ropendal_error_t *err);
const char *ropendal_error_path(const ropendal_error_t *err);
void ropendal_error_release(ropendal_error_t *err);

#ifdef __cplusplus
}
#endif

#endif /* ROPENDAL_H */
