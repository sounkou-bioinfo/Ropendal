#include <stddef.h>
#include <stdint.h>
#include <Rinternals.h>
#include "rust/api.h"
#include "../inst/include/ropendal.h"

/*
 * webR/wasm compatibility shim.
 *
 * Ropendal's real implementation is Rust/OpenDAL/Tokio and is linked on native
 * platforms. The current OpenDAL dependency graph pulls reqwest/hyper/mio even
 * for a minimal fs feature set, and mio's net/event backend does not support
 * wasm32-unknown-emscripten. R-universe still attempts a webR build, so this
 * file provides a loadable package with explicit unsupported errors instead of
 * failing during installation. It is compiled only by src/Makevars.in when the
 * configure script detects the wasm32-unknown-emscripten target.
 *
 * Do not include this file in native builds. Native builds must link the real
 * Rust static library and the public C API symbols anchored by capi_anchor.c.
 */

static SEXP ropendal_wasm_savvy_error(void) {
    SEXP msg = Rf_mkChar("Ropendal's Rust/OpenDAL backend is not available in the current webR/wasm build");
    return (SEXP)(((uintptr_t)msg) | (uintptr_t)1);
}

SEXP savvy_opendal_bytes_as_raw__ffi(SEXP c_arg__bytes) {
    (void)c_arg__bytes;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_opendal_bytes_len__ffi(SEXP c_arg__bytes) {
    (void)c_arg__bytes;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_opendal_codec_decode__ffi(SEXP c_arg__name, SEXP c_arg__data) {
    (void)c_arg__name;
    (void)c_arg__data;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_opendal_codec_encode__ffi(SEXP c_arg__name, SEXP c_arg__data) {
    (void)c_arg__name;
    (void)c_arg__data;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalAio_cancel__ffi(SEXP self__) {
    (void)self__;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalAio_collect__ffi(SEXP self__) {
    (void)self__;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalAio_error_value__ffi(SEXP self__) {
    (void)self__;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalAio_poll__ffi(SEXP self__) {
    (void)self__;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalAio_state_name__ffi(SEXP self__) {
    (void)self__;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalCredentialProvider_azblob__ffi(SEXP c_arg__account_name, SEXP c_arg__account_key, SEXP c_arg__sas_token, SEXP c_arg__endpoint, SEXP c_arg__source) {
    (void)c_arg__account_name;
    (void)c_arg__account_key;
    (void)c_arg__sas_token;
    (void)c_arg__endpoint;
    (void)c_arg__source;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalCredentialProvider_config__ffi(SEXP self__, SEXP c_arg__service) {
    (void)self__;
    (void)c_arg__service;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalCredentialProvider_gcs__ffi(SEXP c_arg__token, SEXP c_arg__service_account_key, SEXP c_arg__credential_path, SEXP c_arg__scope, SEXP c_arg__source) {
    (void)c_arg__token;
    (void)c_arg__service_account_key;
    (void)c_arg__credential_path;
    (void)c_arg__scope;
    (void)c_arg__source;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalCredentialProvider_gdrive__ffi(SEXP c_arg__access_token, SEXP c_arg__refresh_token, SEXP c_arg__client_id, SEXP c_arg__client_secret, SEXP c_arg__source) {
    (void)c_arg__access_token;
    (void)c_arg__refresh_token;
    (void)c_arg__client_id;
    (void)c_arg__client_secret;
    (void)c_arg__source;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalCredentialProvider_gdrive3__ffi(SEXP c_arg__secret_json, SEXP c_arg__tokens_json, SEXP c_arg__scope) {
    (void)c_arg__secret_json;
    (void)c_arg__tokens_json;
    (void)c_arg__scope;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalCredentialProvider_s3__ffi(SEXP c_arg__access_key_id, SEXP c_arg__secret_access_key, SEXP c_arg__session_token, SEXP c_arg__region, SEXP c_arg__source) {
    (void)c_arg__access_key_id;
    (void)c_arg__secret_access_key;
    (void)c_arg__session_token;
    (void)c_arg__region;
    (void)c_arg__source;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalCredentialProvider_schemes__ffi(SEXP self__) {
    (void)self__;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalCredentialProvider_summary__ffi(SEXP self__) {
    (void)self__;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_append__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__data, SEXP c_arg__batch_concurrency, SEXP c_arg__write_concurrency, SEXP c_arg__chunk_size) {
    (void)self__;
    (void)c_arg__path;
    (void)c_arg__data;
    (void)c_arg__batch_concurrency;
    (void)c_arg__write_concurrency;
    (void)c_arg__chunk_size;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_append_aio__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__data, SEXP c_arg__batch_concurrency, SEXP c_arg__write_concurrency, SEXP c_arg__chunk_size) {
    (void)self__;
    (void)c_arg__path;
    (void)c_arg__data;
    (void)c_arg__batch_concurrency;
    (void)c_arg__write_concurrency;
    (void)c_arg__chunk_size;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_capabilities__ffi(SEXP self__) {
    (void)self__;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_copy__ffi(SEXP self__, SEXP c_arg__from, SEXP c_arg__to) {
    (void)self__;
    (void)c_arg__from;
    (void)c_arg__to;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_copy_aio__ffi(SEXP self__, SEXP c_arg__from, SEXP c_arg__to, SEXP c_arg__batch_concurrency) {
    (void)self__;
    (void)c_arg__from;
    (void)c_arg__to;
    (void)c_arg__batch_concurrency;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_delete__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__recursive, SEXP c_arg__batch_concurrency) {
    (void)self__;
    (void)c_arg__path;
    (void)c_arg__recursive;
    (void)c_arg__batch_concurrency;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_delete_aio__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__recursive, SEXP c_arg__batch_concurrency) {
    (void)self__;
    (void)c_arg__path;
    (void)c_arg__recursive;
    (void)c_arg__batch_concurrency;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_exists__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__batch_concurrency) {
    (void)self__;
    (void)c_arg__path;
    (void)c_arg__batch_concurrency;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_exists_aio__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__batch_concurrency) {
    (void)self__;
    (void)c_arg__path;
    (void)c_arg__batch_concurrency;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_from_uri__ffi(SEXP c_arg__uri, SEXP c_arg__headers, SEXP c_arg__runtime_threads, SEXP c_arg__max_inflight, SEXP c_arg__timeout_seconds, SEXP c_arg__io_timeout_seconds) {
    (void)c_arg__uri;
    (void)c_arg__headers;
    (void)c_arg__runtime_threads;
    (void)c_arg__max_inflight;
    (void)c_arg__timeout_seconds;
    (void)c_arg__io_timeout_seconds;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_info__ffi(SEXP self__) {
    (void)self__;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_ls__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__recursive, SEXP c_arg__limit, SEXP c_arg__start_after) {
    (void)self__;
    (void)c_arg__path;
    (void)c_arg__recursive;
    (void)c_arg__limit;
    (void)c_arg__start_after;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_ls_aio__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__recursive, SEXP c_arg__limit, SEXP c_arg__start_after) {
    (void)self__;
    (void)c_arg__path;
    (void)c_arg__recursive;
    (void)c_arg__limit;
    (void)c_arg__start_after;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_ls_iter__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__recursive, SEXP c_arg__page_size, SEXP c_arg__limit, SEXP c_arg__start_after, SEXP c_arg__prefetch) {
    (void)self__;
    (void)c_arg__path;
    (void)c_arg__recursive;
    (void)c_arg__page_size;
    (void)c_arg__limit;
    (void)c_arg__start_after;
    (void)c_arg__prefetch;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_mkdir__ffi(SEXP self__, SEXP c_arg__path) {
    (void)self__;
    (void)c_arg__path;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_mkdir_aio__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__batch_concurrency) {
    (void)self__;
    (void)c_arg__path;
    (void)c_arg__batch_concurrency;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_normalize_path__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__directory) {
    (void)self__;
    (void)c_arg__path;
    (void)c_arg__directory;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_open__ffi(SEXP c_arg__scheme, SEXP c_arg__dots, SEXP c_arg__config, SEXP c_arg__root, SEXP c_arg__auth_config, SEXP c_arg__headers, SEXP c_arg__runtime_threads, SEXP c_arg__max_inflight, SEXP c_arg__timeout_seconds, SEXP c_arg__io_timeout_seconds) {
    (void)c_arg__scheme;
    (void)c_arg__dots;
    (void)c_arg__config;
    (void)c_arg__root;
    (void)c_arg__auth_config;
    (void)c_arg__headers;
    (void)c_arg__runtime_threads;
    (void)c_arg__max_inflight;
    (void)c_arg__timeout_seconds;
    (void)c_arg__io_timeout_seconds;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_read__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__offset, SEXP c_arg__size, SEXP c_arg__end, SEXP c_arg__result, SEXP c_arg__batch_concurrency, SEXP c_arg__read_concurrency, SEXP c_arg__chunk_size, SEXP c_arg__coalesce_gap) {
    (void)self__;
    (void)c_arg__path;
    (void)c_arg__offset;
    (void)c_arg__size;
    (void)c_arg__end;
    (void)c_arg__result;
    (void)c_arg__batch_concurrency;
    (void)c_arg__read_concurrency;
    (void)c_arg__chunk_size;
    (void)c_arg__coalesce_gap;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_read_aio__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__offset, SEXP c_arg__size, SEXP c_arg__end, SEXP c_arg__result, SEXP c_arg__batch_concurrency, SEXP c_arg__read_concurrency, SEXP c_arg__chunk_size, SEXP c_arg__coalesce_gap) {
    (void)self__;
    (void)c_arg__path;
    (void)c_arg__offset;
    (void)c_arg__size;
    (void)c_arg__end;
    (void)c_arg__result;
    (void)c_arg__batch_concurrency;
    (void)c_arg__read_concurrency;
    (void)c_arg__chunk_size;
    (void)c_arg__coalesce_gap;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_read_bytes__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__offset, SEXP c_arg__size, SEXP c_arg__end, SEXP c_arg__result, SEXP c_arg__batch_concurrency, SEXP c_arg__read_concurrency, SEXP c_arg__chunk_size, SEXP c_arg__coalesce_gap) {
    (void)self__;
    (void)c_arg__path;
    (void)c_arg__offset;
    (void)c_arg__size;
    (void)c_arg__end;
    (void)c_arg__result;
    (void)c_arg__batch_concurrency;
    (void)c_arg__read_concurrency;
    (void)c_arg__chunk_size;
    (void)c_arg__coalesce_gap;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_read_bytes_aio__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__offset, SEXP c_arg__size, SEXP c_arg__end, SEXP c_arg__result, SEXP c_arg__batch_concurrency, SEXP c_arg__read_concurrency, SEXP c_arg__chunk_size, SEXP c_arg__coalesce_gap) {
    (void)self__;
    (void)c_arg__path;
    (void)c_arg__offset;
    (void)c_arg__size;
    (void)c_arg__end;
    (void)c_arg__result;
    (void)c_arg__batch_concurrency;
    (void)c_arg__read_concurrency;
    (void)c_arg__chunk_size;
    (void)c_arg__coalesce_gap;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_read_iter__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__chunk_size, SEXP c_arg__offset, SEXP c_arg__size, SEXP c_arg__read_concurrency, SEXP c_arg__coalesce_gap) {
    (void)self__;
    (void)c_arg__path;
    (void)c_arg__chunk_size;
    (void)c_arg__offset;
    (void)c_arg__size;
    (void)c_arg__read_concurrency;
    (void)c_arg__coalesce_gap;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_rename__ffi(SEXP self__, SEXP c_arg__from, SEXP c_arg__to) {
    (void)self__;
    (void)c_arg__from;
    (void)c_arg__to;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_rename_aio__ffi(SEXP self__, SEXP c_arg__from, SEXP c_arg__to, SEXP c_arg__batch_concurrency) {
    (void)self__;
    (void)c_arg__from;
    (void)c_arg__to;
    (void)c_arg__batch_concurrency;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_replace__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__data, SEXP c_arg__batch_concurrency, SEXP c_arg__write_concurrency, SEXP c_arg__chunk_size) {
    (void)self__;
    (void)c_arg__path;
    (void)c_arg__data;
    (void)c_arg__batch_concurrency;
    (void)c_arg__write_concurrency;
    (void)c_arg__chunk_size;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_replace_aio__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__data, SEXP c_arg__batch_concurrency, SEXP c_arg__write_concurrency, SEXP c_arg__chunk_size) {
    (void)self__;
    (void)c_arg__path;
    (void)c_arg__data;
    (void)c_arg__batch_concurrency;
    (void)c_arg__write_concurrency;
    (void)c_arg__chunk_size;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_stat__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__batch_concurrency) {
    (void)self__;
    (void)c_arg__path;
    (void)c_arg__batch_concurrency;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_stat_aio__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__batch_concurrency) {
    (void)self__;
    (void)c_arg__path;
    (void)c_arg__batch_concurrency;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_walk_iter__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__page_size, SEXP c_arg__limit, SEXP c_arg__start_after, SEXP c_arg__prefetch) {
    (void)self__;
    (void)c_arg__path;
    (void)c_arg__page_size;
    (void)c_arg__limit;
    (void)c_arg__start_after;
    (void)c_arg__prefetch;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_write__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__data, SEXP c_arg__batch_concurrency, SEXP c_arg__write_concurrency, SEXP c_arg__chunk_size) {
    (void)self__;
    (void)c_arg__path;
    (void)c_arg__data;
    (void)c_arg__batch_concurrency;
    (void)c_arg__write_concurrency;
    (void)c_arg__chunk_size;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_write_aio__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__data, SEXP c_arg__batch_concurrency, SEXP c_arg__write_concurrency, SEXP c_arg__chunk_size) {
    (void)self__;
    (void)c_arg__path;
    (void)c_arg__data;
    (void)c_arg__batch_concurrency;
    (void)c_arg__write_concurrency;
    (void)c_arg__chunk_size;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalFs_write_iter__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__create, SEXP c_arg__append, SEXP c_arg__write_concurrency, SEXP c_arg__chunk_size) {
    (void)self__;
    (void)c_arg__path;
    (void)c_arg__create;
    (void)c_arg__append;
    (void)c_arg__write_concurrency;
    (void)c_arg__chunk_size;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalHttpFixture_endpoint__ffi(SEXP self__) {
    (void)self__;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalHttpFixture_root__ffi(SEXP self__) {
    (void)self__;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalHttpFixture_start__ffi(SEXP c_arg__root, SEXP c_arg__required_headers, SEXP c_arg__delay_ms) {
    (void)c_arg__root;
    (void)c_arg__required_headers;
    (void)c_arg__delay_ms;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalHttpFixture_stop__ffi(SEXP self__) {
    (void)self__;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalLsIter_collect__ffi(SEXP self__) {
    (void)self__;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalLsIter_next__ffi(SEXP self__) {
    (void)self__;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalReadIter_collect__ffi(SEXP self__) {
    (void)self__;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalReadIter_next__ffi(SEXP self__) {
    (void)self__;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalReadIter_seek__ffi(SEXP self__, SEXP c_arg__offset, SEXP c_arg__whence) {
    (void)self__;
    (void)c_arg__offset;
    (void)c_arg__whence;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalReadIter_tell__ffi(SEXP self__) {
    (void)self__;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalWriteIter_close__ffi(SEXP self__) {
    (void)self__;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalWriteIter_tell__ffi(SEXP self__) {
    (void)self__;
    return ropendal_wasm_savvy_error();
}

SEXP savvy_OpendalWriteIter_write__ffi(SEXP self__, SEXP c_arg__data) {
    (void)self__;
    (void)c_arg__data;
    return ropendal_wasm_savvy_error();
}

uint32_t ropendal_api_version(void) { return 4; }

static ropendal_status_t ropendal_wasm_unsupported(void) { return ROPENDAL_UNSUPPORTED; }

ropendal_status_t ropendal_fs_open(const char *scheme, const ropendal_kv_t *config,
                                   size_t config_len, ropendal_fs_t **out,
                                   ropendal_error_t **err) {
    (void)scheme; (void)config; (void)config_len; (void)err;
    if (out) *out = NULL;
    return ropendal_wasm_unsupported();
}

ropendal_status_t ropendal_fs_from_uri(const char *uri, ropendal_fs_t **out,
                                       ropendal_error_t **err) {
    (void)uri; (void)err;
    if (out) *out = NULL;
    return ropendal_wasm_unsupported();
}

void ropendal_fs_retain(ropendal_fs_t *fs) { (void)fs; }
void ropendal_fs_release(ropendal_fs_t *fs) { (void)fs; }

ropendal_status_t ropendal_store_open(ropendal_fs_t *fs, const ropendal_store_options_t *opts,
                                      ropendal_store_t **out, ropendal_error_t **err) {
    (void)fs; (void)opts; (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_store_cache_open(ropendal_store_t *parent, ropendal_store_t *cache,
                                            const ropendal_store_cache_options_t *opts,
                                            ropendal_store_t **out, ropendal_error_t **err) {
    (void)parent; (void)cache; (void)opts; (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_store_block_cache_open(ropendal_store_t *parent, ropendal_store_t *cache,
                                                  const ropendal_store_block_cache_options_t *opts,
                                                  ropendal_store_t **out, ropendal_error_t **err) {
    (void)parent; (void)cache; (void)opts; (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}
void ropendal_store_retain(ropendal_store_t *store) { (void)store; }
void ropendal_store_release(ropendal_store_t *store) { (void)store; }
ropendal_status_t ropendal_store_read_aio(ropendal_store_t *store, const ropendal_store_read_options_t *opts,
                                          ropendal_aio_t **out, ropendal_error_t **err) {
    (void)store; (void)opts; (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_store_read_into_aio(ropendal_store_t *store, const ropendal_store_read_options_t *opts,
                                               uint8_t *dst, size_t dst_len, ropendal_aio_t **out,
                                               ropendal_error_t **err) {
    (void)store; (void)opts; (void)dst; (void)dst_len; (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_store_write_aio(ropendal_store_t *store, const ropendal_store_write_options_t *opts,
                                           const uint8_t *src, size_t src_len, ropendal_aio_t **out,
                                           ropendal_error_t **err) {
    (void)store; (void)opts; (void)src; (void)src_len; (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_store_replace_aio(ropendal_store_t *store, const ropendal_store_write_options_t *opts,
                                             const uint8_t *src, size_t src_len, ropendal_aio_t **out,
                                             ropendal_error_t **err) {
    (void)store; (void)opts; (void)src; (void)src_len; (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_store_exists_aio(ropendal_store_t *store, const char *key,
                                            ropendal_aio_callback_t callback, void *userdata,
                                            ropendal_aio_t **out, ropendal_error_t **err) {
    (void)store; (void)key; (void)callback; (void)userdata; (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_store_ls_aio(ropendal_store_t *store, const ropendal_store_ls_options_t *opts,
                                        ropendal_aio_t **out, ropendal_error_t **err) {
    (void)store; (void)opts; (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_store_delete_aio(ropendal_store_t *store, const ropendal_store_delete_options_t *opts,
                                            ropendal_aio_t **out, ropendal_error_t **err) {
    (void)store; (void)opts; (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}

ropendal_status_t ropendal_codec_encode(const char *codec, const uint8_t *src,
                                        size_t src_len, ropendal_bytes_t **out,
                                        ropendal_error_t **err) {
    (void)codec; (void)src; (void)src_len; (void)err;
    if (out) *out = NULL;
    return ropendal_wasm_unsupported();
}

ropendal_status_t ropendal_codec_decode(const char *codec, const uint8_t *src,
                                        size_t src_len, ropendal_bytes_t **out,
                                        ropendal_error_t **err) {
    (void)codec; (void)src; (void)src_len; (void)err;
    if (out) *out = NULL;
    return ropendal_wasm_unsupported();
}

const uint8_t *ropendal_bytes_data(const ropendal_bytes_t *bytes) { (void)bytes; return NULL; }
size_t ropendal_bytes_len(const ropendal_bytes_t *bytes) { (void)bytes; return 0; }
void ropendal_bytes_release(ropendal_bytes_t *bytes) { (void)bytes; }

ropendal_status_t ropendal_read_aio(ropendal_fs_t *fs, const ropendal_read_options_t *opts,
                                    ropendal_aio_t **out, ropendal_error_t **err) {
    (void)fs; (void)opts; (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_read_into_aio(ropendal_fs_t *fs, const ropendal_read_options_t *opts,
                                         uint8_t *dst, size_t dst_len, ropendal_aio_t **out,
                                         ropendal_error_t **err) {
    (void)fs; (void)opts; (void)dst; (void)dst_len; (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_readv_aio(ropendal_fs_t *fs, const ropendal_read_request_t *requests,
                                     size_t n_requests, const ropendal_readv_options_t *opts,
                                     ropendal_aio_t **out, ropendal_error_t **err) {
    (void)fs; (void)requests; (void)n_requests; (void)opts; (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_readv_into_aio(ropendal_fs_t *fs, const ropendal_read_into_request_t *requests,
                                          size_t n_requests, const ropendal_readv_options_t *opts,
                                          ropendal_aio_t **out, ropendal_error_t **err) {
    (void)fs; (void)requests; (void)n_requests; (void)opts; (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_write_aio(ropendal_fs_t *fs, const ropendal_write_options_t *opts,
                                     const uint8_t *src, size_t src_len, ropendal_aio_t **out,
                                     ropendal_error_t **err) {
    (void)fs; (void)opts; (void)src; (void)src_len; (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_replace_aio(ropendal_fs_t *fs, const ropendal_write_options_t *opts,
                                       const uint8_t *src, size_t src_len, ropendal_aio_t **out,
                                       ropendal_error_t **err) {
    (void)fs; (void)opts; (void)src; (void)src_len; (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_append_aio(ropendal_fs_t *fs, const ropendal_write_options_t *opts,
                                      const uint8_t *src, size_t src_len, ropendal_aio_t **out,
                                      ropendal_error_t **err) {
    (void)fs; (void)opts; (void)src; (void)src_len; (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_stat_aio(ropendal_fs_t *fs, const char *path,
                                    ropendal_aio_callback_t callback, void *userdata,
                                    ropendal_aio_t **out, ropendal_error_t **err) {
    (void)fs; (void)path; (void)callback; (void)userdata; (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_exists_aio(ropendal_fs_t *fs, const char *path,
                                      ropendal_aio_callback_t callback, void *userdata,
                                      ropendal_aio_t **out, ropendal_error_t **err) {
    (void)fs; (void)path; (void)callback; (void)userdata; (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_ls_aio(ropendal_fs_t *fs, const ropendal_ls_options_t *opts,
                                  ropendal_aio_t **out, ropendal_error_t **err) {
    (void)fs; (void)opts; (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_delete_aio(ropendal_fs_t *fs, const ropendal_delete_options_t *opts,
                                      ropendal_aio_t **out, ropendal_error_t **err) {
    (void)fs; (void)opts; (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_copy_aio(ropendal_fs_t *fs, const char *from, const char *to,
                                    ropendal_aio_callback_t callback, void *userdata,
                                    ropendal_aio_t **out, ropendal_error_t **err) {
    (void)fs; (void)from; (void)to; (void)callback; (void)userdata; (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_rename_aio(ropendal_fs_t *fs, const char *from, const char *to,
                                      ropendal_aio_callback_t callback, void *userdata,
                                      ropendal_aio_t **out, ropendal_error_t **err) {
    (void)fs; (void)from; (void)to; (void)callback; (void)userdata; (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_mkdir_aio(ropendal_fs_t *fs, const char *path,
                                     ropendal_aio_callback_t callback, void *userdata,
                                     ropendal_aio_t **out, ropendal_error_t **err) {
    (void)fs; (void)path; (void)callback; (void)userdata; (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}

ropendal_status_t ropendal_cv_alloc(ropendal_cv_t **out, ropendal_error_t **err) {
    (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}
void ropendal_cv_release(ropendal_cv_t *cv) { (void)cv; }
ropendal_status_t ropendal_cv_wait(ropendal_cv_t *cv, ropendal_error_t **err) { (void)cv; (void)err; return ropendal_wasm_unsupported(); }
ropendal_status_t ropendal_cv_until(ropendal_cv_t *cv, int timeout_ms, ropendal_error_t **err) { (void)cv; (void)timeout_ms; (void)err; return ropendal_wasm_unsupported(); }
uint64_t ropendal_cv_value(ropendal_cv_t *cv) { (void)cv; return 0; }
void ropendal_cv_reset(ropendal_cv_t *cv) { (void)cv; }
void ropendal_cv_signal(ropendal_cv_t *cv) { (void)cv; }

ropendal_status_t ropendal_aio_notify(ropendal_aio_t *aio, ropendal_cv_t *cv,
                                      uint64_t id, ropendal_error_t **err) {
    (void)aio; (void)cv; (void)id; (void)err; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_monitor_create(ropendal_cv_t *cv, ropendal_monitor_t **out,
                                          ropendal_error_t **err) {
    (void)cv; (void)err; if (out) *out = NULL; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_monitor_add_aio(ropendal_monitor_t *monitor, ropendal_aio_t *aio,
                                           uint64_t id, ropendal_error_t **err) {
    (void)monitor; (void)aio; (void)id; (void)err; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_monitor_read(ropendal_monitor_t *monitor,
                                        const ropendal_monitor_event_t **events,
                                        size_t *len, ropendal_error_t **err) {
    (void)monitor; (void)err; if (events) *events = NULL; if (len) *len = 0; return ropendal_wasm_unsupported();
}
void ropendal_monitor_release(ropendal_monitor_t *monitor) { (void)monitor; }

ropendal_aio_status_t ropendal_aio_poll(ropendal_aio_t *aio) { (void)aio; return ROPENDAL_AIO_ERROR; }
ropendal_status_t ropendal_aio_wait(ropendal_aio_t *aio, int timeout_ms, ropendal_error_t **err) { (void)aio; (void)timeout_ms; (void)err; return ropendal_wasm_unsupported(); }
void ropendal_aio_cancel(ropendal_aio_t *aio) { (void)aio; }
void ropendal_aio_release(ropendal_aio_t *aio) { (void)aio; }

ropendal_status_t ropendal_aio_result_bytes(ropendal_aio_t *aio, const uint8_t **data,
                                            size_t *len, ropendal_error_t **err) {
    (void)aio; (void)err; if (data) *data = NULL; if (len) *len = 0; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_aio_result_nread(ropendal_aio_t *aio, size_t *nread,
                                            ropendal_error_t **err) {
    (void)aio; (void)err; if (nread) *nread = 0; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_aio_result_readv(ropendal_aio_t *aio,
                                            const ropendal_readv_result_t **results,
                                            size_t *len, ropendal_error_t **err) {
    (void)aio; (void)err; if (results) *results = NULL; if (len) *len = 0; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_aio_result_bool(ropendal_aio_t *aio, int *value,
                                           ropendal_error_t **err) {
    (void)aio; (void)err; if (value) *value = 0; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_aio_result_entries(ropendal_aio_t *aio,
                                              const ropendal_entry_t **entries,
                                              size_t *len, ropendal_error_t **err) {
    (void)aio; (void)err; if (entries) *entries = NULL; if (len) *len = 0; return ropendal_wasm_unsupported();
}
ropendal_status_t ropendal_aio_result_entry(ropendal_aio_t *aio,
                                            const ropendal_entry_t **entry,
                                            ropendal_error_t **err) {
    (void)aio; (void)err; if (entry) *entry = NULL; return ropendal_wasm_unsupported();
}

const char *ropendal_error_message(const ropendal_error_t *err) {
    (void)err; return "Ropendal's Rust/OpenDAL backend is not available in the current webR/wasm build";
}
const char *ropendal_error_kind(const ropendal_error_t *err) { (void)err; return "Unsupported"; }
const char *ropendal_error_operation(const ropendal_error_t *err) { (void)err; return "wasm"; }
const char *ropendal_error_path(const ropendal_error_t *err) { (void)err; return ""; }
void ropendal_error_release(ropendal_error_t *err) { (void)err; }
