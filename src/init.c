
// clang-format sorts includes unless SortIncludes: Never. However, the ordering
// does matter here. So, we need to disable clang-format for safety.

// clang-format off
#include <stdint.h>
#include <Rinternals.h>
#include <R_ext/Parse.h>
// clang-format on

#include "rust/api.h"

static uintptr_t TAGGED_POINTER_MASK = (uintptr_t)1;

SEXP handle_result(SEXP res_) {
    uintptr_t res = (uintptr_t)res_;

    // An error is indicated by tag.
    if ((res & TAGGED_POINTER_MASK) == 1) {
        // Remove tag
        SEXP res_aligned = (SEXP)(res & ~TAGGED_POINTER_MASK);

        // Currently, there are two types of error cases:
        //
        //   1. Error from Rust code
        //   2. Error from R's C API, which is caught by R_UnwindProtect()
        //
        if (TYPEOF(res_aligned) == CHARSXP) {
            // In case 1, the result is an error message that can be passed to
            // Rf_errorcall() directly.
            Rf_errorcall(R_NilValue, "%s", CHAR(res_aligned));
        } else {
            // In case 2, the result is the token to restart the
            // cleanup process on R's side.
            R_ContinueUnwind(res_aligned);
        }
    }

    return (SEXP)res;
}

SEXP savvy_opendal_bytes_as_raw__impl(SEXP c_arg__bytes) {
    SEXP res = savvy_opendal_bytes_as_raw__ffi(c_arg__bytes);
    return handle_result(res);
}

SEXP savvy_opendal_bytes_from_raw__impl(SEXP c_arg__data) {
    SEXP res = savvy_opendal_bytes_from_raw__ffi(c_arg__data);
    return handle_result(res);
}

SEXP savvy_opendal_bytes_len__impl(SEXP c_arg__bytes) {
    SEXP res = savvy_opendal_bytes_len__ffi(c_arg__bytes);
    return handle_result(res);
}

SEXP savvy_opendal_bytes_slice__impl(SEXP c_arg__bytes, SEXP c_arg__offset, SEXP c_arg__size) {
    SEXP res = savvy_opendal_bytes_slice__ffi(c_arg__bytes, c_arg__offset, c_arg__size);
    return handle_result(res);
}

SEXP savvy_opendal_codec_decode__impl(SEXP c_arg__name, SEXP c_arg__data) {
    SEXP res = savvy_opendal_codec_decode__ffi(c_arg__name, c_arg__data);
    return handle_result(res);
}

SEXP savvy_opendal_codec_encode__impl(SEXP c_arg__name, SEXP c_arg__data) {
    SEXP res = savvy_opendal_codec_encode__ffi(c_arg__name, c_arg__data);
    return handle_result(res);
}

SEXP savvy_OpendalAio_cancel__impl(SEXP self__) {
    SEXP res = savvy_OpendalAio_cancel__ffi(self__);
    return handle_result(res);
}

SEXP savvy_OpendalAio_collect__impl(SEXP self__) {
    SEXP res = savvy_OpendalAio_collect__ffi(self__);
    return handle_result(res);
}

SEXP savvy_OpendalAio_error_value__impl(SEXP self__) {
    SEXP res = savvy_OpendalAio_error_value__ffi(self__);
    return handle_result(res);
}

SEXP savvy_OpendalAio_poll__impl(SEXP self__) {
    SEXP res = savvy_OpendalAio_poll__ffi(self__);
    return handle_result(res);
}

SEXP savvy_OpendalAio_state_name__impl(SEXP self__) {
    SEXP res = savvy_OpendalAio_state_name__ffi(self__);
    return handle_result(res);
}


SEXP savvy_OpendalCredentialProvider_azblob__impl(SEXP c_arg__account_name, SEXP c_arg__account_key, SEXP c_arg__sas_token, SEXP c_arg__endpoint, SEXP c_arg__source) {
    SEXP res = savvy_OpendalCredentialProvider_azblob__ffi(c_arg__account_name, c_arg__account_key, c_arg__sas_token, c_arg__endpoint, c_arg__source);
    return handle_result(res);
}

SEXP savvy_OpendalCredentialProvider_config__impl(SEXP self__, SEXP c_arg__service) {
    SEXP res = savvy_OpendalCredentialProvider_config__ffi(self__, c_arg__service);
    return handle_result(res);
}

SEXP savvy_OpendalCredentialProvider_gcs__impl(SEXP c_arg__token, SEXP c_arg__service_account_key, SEXP c_arg__credential_path, SEXP c_arg__scope, SEXP c_arg__source) {
    SEXP res = savvy_OpendalCredentialProvider_gcs__ffi(c_arg__token, c_arg__service_account_key, c_arg__credential_path, c_arg__scope, c_arg__source);
    return handle_result(res);
}

SEXP savvy_OpendalCredentialProvider_gdrive__impl(SEXP c_arg__access_token, SEXP c_arg__refresh_token, SEXP c_arg__client_id, SEXP c_arg__client_secret, SEXP c_arg__source) {
    SEXP res = savvy_OpendalCredentialProvider_gdrive__ffi(c_arg__access_token, c_arg__refresh_token, c_arg__client_id, c_arg__client_secret, c_arg__source);
    return handle_result(res);
}

SEXP savvy_OpendalCredentialProvider_gdrive3__impl(SEXP c_arg__secret_json, SEXP c_arg__tokens_json, SEXP c_arg__scope) {
    SEXP res = savvy_OpendalCredentialProvider_gdrive3__ffi(c_arg__secret_json, c_arg__tokens_json, c_arg__scope);
    return handle_result(res);
}

SEXP savvy_OpendalCredentialProvider_s3__impl(SEXP c_arg__access_key_id, SEXP c_arg__secret_access_key, SEXP c_arg__session_token, SEXP c_arg__region, SEXP c_arg__source) {
    SEXP res = savvy_OpendalCredentialProvider_s3__ffi(c_arg__access_key_id, c_arg__secret_access_key, c_arg__session_token, c_arg__region, c_arg__source);
    return handle_result(res);
}

SEXP savvy_OpendalCredentialProvider_schemes__impl(SEXP self__) {
    SEXP res = savvy_OpendalCredentialProvider_schemes__ffi(self__);
    return handle_result(res);
}

SEXP savvy_OpendalCredentialProvider_summary__impl(SEXP self__) {
    SEXP res = savvy_OpendalCredentialProvider_summary__ffi(self__);
    return handle_result(res);
}

SEXP savvy_OpendalFs_append__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__data, SEXP c_arg__batch_concurrency, SEXP c_arg__write_concurrency, SEXP c_arg__chunk_size) {
    SEXP res = savvy_OpendalFs_append__ffi(self__, c_arg__path, c_arg__data, c_arg__batch_concurrency, c_arg__write_concurrency, c_arg__chunk_size);
    return handle_result(res);
}

SEXP savvy_OpendalFs_append_aio__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__data, SEXP c_arg__batch_concurrency, SEXP c_arg__write_concurrency, SEXP c_arg__chunk_size) {
    SEXP res = savvy_OpendalFs_append_aio__ffi(self__, c_arg__path, c_arg__data, c_arg__batch_concurrency, c_arg__write_concurrency, c_arg__chunk_size);
    return handle_result(res);
}

SEXP savvy_OpendalFs_capabilities__impl(SEXP self__) {
    SEXP res = savvy_OpendalFs_capabilities__ffi(self__);
    return handle_result(res);
}

SEXP savvy_OpendalFs_copy__impl(SEXP self__, SEXP c_arg__from, SEXP c_arg__to) {
    SEXP res = savvy_OpendalFs_copy__ffi(self__, c_arg__from, c_arg__to);
    return handle_result(res);
}

SEXP savvy_OpendalFs_copy_aio__impl(SEXP self__, SEXP c_arg__from, SEXP c_arg__to, SEXP c_arg__batch_concurrency) {
    SEXP res = savvy_OpendalFs_copy_aio__ffi(self__, c_arg__from, c_arg__to, c_arg__batch_concurrency);
    return handle_result(res);
}

SEXP savvy_OpendalFs_delete__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__recursive, SEXP c_arg__batch_concurrency) {
    SEXP res = savvy_OpendalFs_delete__ffi(self__, c_arg__path, c_arg__recursive, c_arg__batch_concurrency);
    return handle_result(res);
}

SEXP savvy_OpendalFs_delete_aio__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__recursive, SEXP c_arg__batch_concurrency) {
    SEXP res = savvy_OpendalFs_delete_aio__ffi(self__, c_arg__path, c_arg__recursive, c_arg__batch_concurrency);
    return handle_result(res);
}

SEXP savvy_OpendalFs_exists__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__batch_concurrency) {
    SEXP res = savvy_OpendalFs_exists__ffi(self__, c_arg__path, c_arg__batch_concurrency);
    return handle_result(res);
}

SEXP savvy_OpendalFs_exists_aio__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__batch_concurrency) {
    SEXP res = savvy_OpendalFs_exists_aio__ffi(self__, c_arg__path, c_arg__batch_concurrency);
    return handle_result(res);
}

SEXP savvy_OpendalFs_from_uri__impl(SEXP c_arg__uri, SEXP c_arg__headers, SEXP c_arg__runtime_threads, SEXP c_arg__max_inflight, SEXP c_arg__timeout_seconds, SEXP c_arg__io_timeout_seconds) {
    SEXP res = savvy_OpendalFs_from_uri__ffi(c_arg__uri, c_arg__headers, c_arg__runtime_threads, c_arg__max_inflight, c_arg__timeout_seconds, c_arg__io_timeout_seconds);
    return handle_result(res);
}

SEXP savvy_OpendalFs_info__impl(SEXP self__) {
    SEXP res = savvy_OpendalFs_info__ffi(self__);
    return handle_result(res);
}

SEXP savvy_OpendalFs_ls__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__recursive, SEXP c_arg__limit, SEXP c_arg__start_after) {
    SEXP res = savvy_OpendalFs_ls__ffi(self__, c_arg__path, c_arg__recursive, c_arg__limit, c_arg__start_after);
    return handle_result(res);
}

SEXP savvy_OpendalFs_ls_aio__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__recursive, SEXP c_arg__limit, SEXP c_arg__start_after) {
    SEXP res = savvy_OpendalFs_ls_aio__ffi(self__, c_arg__path, c_arg__recursive, c_arg__limit, c_arg__start_after);
    return handle_result(res);
}

SEXP savvy_OpendalFs_ls_iter__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__recursive, SEXP c_arg__page_size, SEXP c_arg__limit, SEXP c_arg__start_after, SEXP c_arg__prefetch) {
    SEXP res = savvy_OpendalFs_ls_iter__ffi(self__, c_arg__path, c_arg__recursive, c_arg__page_size, c_arg__limit, c_arg__start_after, c_arg__prefetch);
    return handle_result(res);
}

SEXP savvy_OpendalFs_mkdir__impl(SEXP self__, SEXP c_arg__path) {
    SEXP res = savvy_OpendalFs_mkdir__ffi(self__, c_arg__path);
    return handle_result(res);
}

SEXP savvy_OpendalFs_mkdir_aio__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__batch_concurrency) {
    SEXP res = savvy_OpendalFs_mkdir_aio__ffi(self__, c_arg__path, c_arg__batch_concurrency);
    return handle_result(res);
}

SEXP savvy_OpendalFs_normalize_path__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__directory) {
    SEXP res = savvy_OpendalFs_normalize_path__ffi(self__, c_arg__path, c_arg__directory);
    return handle_result(res);
}

SEXP savvy_OpendalFs_open__impl(SEXP c_arg__scheme, SEXP c_arg__dots, SEXP c_arg__config, SEXP c_arg__root, SEXP c_arg__auth_config, SEXP c_arg__headers, SEXP c_arg__runtime_threads, SEXP c_arg__max_inflight, SEXP c_arg__timeout_seconds, SEXP c_arg__io_timeout_seconds) {
    SEXP res = savvy_OpendalFs_open__ffi(c_arg__scheme, c_arg__dots, c_arg__config, c_arg__root, c_arg__auth_config, c_arg__headers, c_arg__runtime_threads, c_arg__max_inflight, c_arg__timeout_seconds, c_arg__io_timeout_seconds);
    return handle_result(res);
}

SEXP savvy_OpendalFs_read__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__offset, SEXP c_arg__size, SEXP c_arg__end, SEXP c_arg__result, SEXP c_arg__batch_concurrency, SEXP c_arg__read_concurrency, SEXP c_arg__chunk_size, SEXP c_arg__coalesce_gap) {
    SEXP res = savvy_OpendalFs_read__ffi(self__, c_arg__path, c_arg__offset, c_arg__size, c_arg__end, c_arg__result, c_arg__batch_concurrency, c_arg__read_concurrency, c_arg__chunk_size, c_arg__coalesce_gap);
    return handle_result(res);
}

SEXP savvy_OpendalFs_read_aio__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__offset, SEXP c_arg__size, SEXP c_arg__end, SEXP c_arg__result, SEXP c_arg__batch_concurrency, SEXP c_arg__read_concurrency, SEXP c_arg__chunk_size, SEXP c_arg__coalesce_gap) {
    SEXP res = savvy_OpendalFs_read_aio__ffi(self__, c_arg__path, c_arg__offset, c_arg__size, c_arg__end, c_arg__result, c_arg__batch_concurrency, c_arg__read_concurrency, c_arg__chunk_size, c_arg__coalesce_gap);
    return handle_result(res);
}

SEXP savvy_OpendalFs_read_bytes__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__offset, SEXP c_arg__size, SEXP c_arg__end, SEXP c_arg__result, SEXP c_arg__batch_concurrency, SEXP c_arg__read_concurrency, SEXP c_arg__chunk_size, SEXP c_arg__coalesce_gap) {
    SEXP res = savvy_OpendalFs_read_bytes__ffi(self__, c_arg__path, c_arg__offset, c_arg__size, c_arg__end, c_arg__result, c_arg__batch_concurrency, c_arg__read_concurrency, c_arg__chunk_size, c_arg__coalesce_gap);
    return handle_result(res);
}

SEXP savvy_OpendalFs_read_bytes_aio__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__offset, SEXP c_arg__size, SEXP c_arg__end, SEXP c_arg__result, SEXP c_arg__batch_concurrency, SEXP c_arg__read_concurrency, SEXP c_arg__chunk_size, SEXP c_arg__coalesce_gap) {
    SEXP res = savvy_OpendalFs_read_bytes_aio__ffi(self__, c_arg__path, c_arg__offset, c_arg__size, c_arg__end, c_arg__result, c_arg__batch_concurrency, c_arg__read_concurrency, c_arg__chunk_size, c_arg__coalesce_gap);
    return handle_result(res);
}

SEXP savvy_OpendalFs_read_iter__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__chunk_size, SEXP c_arg__offset, SEXP c_arg__size, SEXP c_arg__read_concurrency, SEXP c_arg__coalesce_gap) {
    SEXP res = savvy_OpendalFs_read_iter__ffi(self__, c_arg__path, c_arg__chunk_size, c_arg__offset, c_arg__size, c_arg__read_concurrency, c_arg__coalesce_gap);
    return handle_result(res);
}

SEXP savvy_OpendalFs_rename__impl(SEXP self__, SEXP c_arg__from, SEXP c_arg__to) {
    SEXP res = savvy_OpendalFs_rename__ffi(self__, c_arg__from, c_arg__to);
    return handle_result(res);
}

SEXP savvy_OpendalFs_rename_aio__impl(SEXP self__, SEXP c_arg__from, SEXP c_arg__to, SEXP c_arg__batch_concurrency) {
    SEXP res = savvy_OpendalFs_rename_aio__ffi(self__, c_arg__from, c_arg__to, c_arg__batch_concurrency);
    return handle_result(res);
}

SEXP savvy_OpendalFs_replace__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__data, SEXP c_arg__batch_concurrency, SEXP c_arg__write_concurrency, SEXP c_arg__chunk_size) {
    SEXP res = savvy_OpendalFs_replace__ffi(self__, c_arg__path, c_arg__data, c_arg__batch_concurrency, c_arg__write_concurrency, c_arg__chunk_size);
    return handle_result(res);
}

SEXP savvy_OpendalFs_replace_aio__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__data, SEXP c_arg__batch_concurrency, SEXP c_arg__write_concurrency, SEXP c_arg__chunk_size) {
    SEXP res = savvy_OpendalFs_replace_aio__ffi(self__, c_arg__path, c_arg__data, c_arg__batch_concurrency, c_arg__write_concurrency, c_arg__chunk_size);
    return handle_result(res);
}

SEXP savvy_OpendalFs_stat__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__batch_concurrency) {
    SEXP res = savvy_OpendalFs_stat__ffi(self__, c_arg__path, c_arg__batch_concurrency);
    return handle_result(res);
}

SEXP savvy_OpendalFs_stat_aio__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__batch_concurrency) {
    SEXP res = savvy_OpendalFs_stat_aio__ffi(self__, c_arg__path, c_arg__batch_concurrency);
    return handle_result(res);
}

SEXP savvy_OpendalFs_walk_iter__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__page_size, SEXP c_arg__limit, SEXP c_arg__start_after, SEXP c_arg__prefetch) {
    SEXP res = savvy_OpendalFs_walk_iter__ffi(self__, c_arg__path, c_arg__page_size, c_arg__limit, c_arg__start_after, c_arg__prefetch);
    return handle_result(res);
}

SEXP savvy_OpendalFs_write__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__data, SEXP c_arg__batch_concurrency, SEXP c_arg__write_concurrency, SEXP c_arg__chunk_size) {
    SEXP res = savvy_OpendalFs_write__ffi(self__, c_arg__path, c_arg__data, c_arg__batch_concurrency, c_arg__write_concurrency, c_arg__chunk_size);
    return handle_result(res);
}

SEXP savvy_OpendalFs_write_aio__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__data, SEXP c_arg__batch_concurrency, SEXP c_arg__write_concurrency, SEXP c_arg__chunk_size) {
    SEXP res = savvy_OpendalFs_write_aio__ffi(self__, c_arg__path, c_arg__data, c_arg__batch_concurrency, c_arg__write_concurrency, c_arg__chunk_size);
    return handle_result(res);
}

SEXP savvy_OpendalFs_write_iter__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__create, SEXP c_arg__append, SEXP c_arg__write_concurrency, SEXP c_arg__chunk_size) {
    SEXP res = savvy_OpendalFs_write_iter__ffi(self__, c_arg__path, c_arg__create, c_arg__append, c_arg__write_concurrency, c_arg__chunk_size);
    return handle_result(res);
}

SEXP savvy_OpendalHttpFixture_endpoint__impl(SEXP self__) {
    SEXP res = savvy_OpendalHttpFixture_endpoint__ffi(self__);
    return handle_result(res);
}

SEXP savvy_OpendalHttpFixture_root__impl(SEXP self__) {
    SEXP res = savvy_OpendalHttpFixture_root__ffi(self__);
    return handle_result(res);
}

SEXP savvy_OpendalHttpFixture_start__impl(SEXP c_arg__root, SEXP c_arg__required_headers, SEXP c_arg__delay_ms) {
    SEXP res = savvy_OpendalHttpFixture_start__ffi(c_arg__root, c_arg__required_headers, c_arg__delay_ms);
    return handle_result(res);
}

SEXP savvy_OpendalHttpFixture_stop__impl(SEXP self__) {
    SEXP res = savvy_OpendalHttpFixture_stop__ffi(self__);
    return handle_result(res);
}

SEXP savvy_OpendalLsIter_collect__impl(SEXP self__) {
    SEXP res = savvy_OpendalLsIter_collect__ffi(self__);
    return handle_result(res);
}

SEXP savvy_OpendalLsIter_next__impl(SEXP self__) {
    SEXP res = savvy_OpendalLsIter_next__ffi(self__);
    return handle_result(res);
}

SEXP savvy_OpendalReadIter_collect__impl(SEXP self__) {
    SEXP res = savvy_OpendalReadIter_collect__ffi(self__);
    return handle_result(res);
}

SEXP savvy_OpendalReadIter_next__impl(SEXP self__) {
    SEXP res = savvy_OpendalReadIter_next__ffi(self__);
    return handle_result(res);
}

SEXP savvy_OpendalReadIter_seek__impl(SEXP self__, SEXP c_arg__offset, SEXP c_arg__whence) {
    SEXP res = savvy_OpendalReadIter_seek__ffi(self__, c_arg__offset, c_arg__whence);
    return handle_result(res);
}

SEXP savvy_OpendalReadIter_tell__impl(SEXP self__) {
    SEXP res = savvy_OpendalReadIter_tell__ffi(self__);
    return handle_result(res);
}

SEXP savvy_OpendalWriteIter_close__impl(SEXP self__) {
    SEXP res = savvy_OpendalWriteIter_close__ffi(self__);
    return handle_result(res);
}

SEXP savvy_OpendalWriteIter_tell__impl(SEXP self__) {
    SEXP res = savvy_OpendalWriteIter_tell__ffi(self__);
    return handle_result(res);
}

SEXP savvy_OpendalWriteIter_write__impl(SEXP self__, SEXP c_arg__data) {
    SEXP res = savvy_OpendalWriteIter_write__ffi(self__, c_arg__data);
    return handle_result(res);
}


static const R_CallMethodDef CallEntries[] = {
    {"savvy_opendal_bytes_as_raw__impl", (DL_FUNC) &savvy_opendal_bytes_as_raw__impl, 1},
    {"savvy_opendal_bytes_from_raw__impl", (DL_FUNC) &savvy_opendal_bytes_from_raw__impl, 1},
    {"savvy_opendal_bytes_len__impl", (DL_FUNC) &savvy_opendal_bytes_len__impl, 1},
    {"savvy_opendal_bytes_slice__impl", (DL_FUNC) &savvy_opendal_bytes_slice__impl, 3},
    {"savvy_opendal_codec_decode__impl", (DL_FUNC) &savvy_opendal_codec_decode__impl, 2},
    {"savvy_opendal_codec_encode__impl", (DL_FUNC) &savvy_opendal_codec_encode__impl, 2},
    {"savvy_OpendalAio_cancel__impl", (DL_FUNC) &savvy_OpendalAio_cancel__impl, 1},
    {"savvy_OpendalAio_collect__impl", (DL_FUNC) &savvy_OpendalAio_collect__impl, 1},
    {"savvy_OpendalAio_error_value__impl", (DL_FUNC) &savvy_OpendalAio_error_value__impl, 1},
    {"savvy_OpendalAio_poll__impl", (DL_FUNC) &savvy_OpendalAio_poll__impl, 1},
    {"savvy_OpendalAio_state_name__impl", (DL_FUNC) &savvy_OpendalAio_state_name__impl, 1},

    {"savvy_OpendalCredentialProvider_azblob__impl", (DL_FUNC) &savvy_OpendalCredentialProvider_azblob__impl, 5},
    {"savvy_OpendalCredentialProvider_config__impl", (DL_FUNC) &savvy_OpendalCredentialProvider_config__impl, 2},
    {"savvy_OpendalCredentialProvider_gcs__impl", (DL_FUNC) &savvy_OpendalCredentialProvider_gcs__impl, 5},
    {"savvy_OpendalCredentialProvider_gdrive__impl", (DL_FUNC) &savvy_OpendalCredentialProvider_gdrive__impl, 5},
    {"savvy_OpendalCredentialProvider_gdrive3__impl", (DL_FUNC) &savvy_OpendalCredentialProvider_gdrive3__impl, 3},
    {"savvy_OpendalCredentialProvider_s3__impl", (DL_FUNC) &savvy_OpendalCredentialProvider_s3__impl, 5},
    {"savvy_OpendalCredentialProvider_schemes__impl", (DL_FUNC) &savvy_OpendalCredentialProvider_schemes__impl, 1},
    {"savvy_OpendalCredentialProvider_summary__impl", (DL_FUNC) &savvy_OpendalCredentialProvider_summary__impl, 1},
    {"savvy_OpendalFs_append__impl", (DL_FUNC) &savvy_OpendalFs_append__impl, 6},
    {"savvy_OpendalFs_append_aio__impl", (DL_FUNC) &savvy_OpendalFs_append_aio__impl, 6},
    {"savvy_OpendalFs_capabilities__impl", (DL_FUNC) &savvy_OpendalFs_capabilities__impl, 1},
    {"savvy_OpendalFs_copy__impl", (DL_FUNC) &savvy_OpendalFs_copy__impl, 3},
    {"savvy_OpendalFs_copy_aio__impl", (DL_FUNC) &savvy_OpendalFs_copy_aio__impl, 4},
    {"savvy_OpendalFs_delete__impl", (DL_FUNC) &savvy_OpendalFs_delete__impl, 4},
    {"savvy_OpendalFs_delete_aio__impl", (DL_FUNC) &savvy_OpendalFs_delete_aio__impl, 4},
    {"savvy_OpendalFs_exists__impl", (DL_FUNC) &savvy_OpendalFs_exists__impl, 3},
    {"savvy_OpendalFs_exists_aio__impl", (DL_FUNC) &savvy_OpendalFs_exists_aio__impl, 3},
    {"savvy_OpendalFs_from_uri__impl", (DL_FUNC) &savvy_OpendalFs_from_uri__impl, 6},
    {"savvy_OpendalFs_info__impl", (DL_FUNC) &savvy_OpendalFs_info__impl, 1},
    {"savvy_OpendalFs_ls__impl", (DL_FUNC) &savvy_OpendalFs_ls__impl, 5},
    {"savvy_OpendalFs_ls_aio__impl", (DL_FUNC) &savvy_OpendalFs_ls_aio__impl, 5},
    {"savvy_OpendalFs_ls_iter__impl", (DL_FUNC) &savvy_OpendalFs_ls_iter__impl, 7},
    {"savvy_OpendalFs_mkdir__impl", (DL_FUNC) &savvy_OpendalFs_mkdir__impl, 2},
    {"savvy_OpendalFs_mkdir_aio__impl", (DL_FUNC) &savvy_OpendalFs_mkdir_aio__impl, 3},
    {"savvy_OpendalFs_normalize_path__impl", (DL_FUNC) &savvy_OpendalFs_normalize_path__impl, 3},
    {"savvy_OpendalFs_open__impl", (DL_FUNC) &savvy_OpendalFs_open__impl, 10},
    {"savvy_OpendalFs_read__impl", (DL_FUNC) &savvy_OpendalFs_read__impl, 10},
    {"savvy_OpendalFs_read_aio__impl", (DL_FUNC) &savvy_OpendalFs_read_aio__impl, 10},
    {"savvy_OpendalFs_read_bytes__impl", (DL_FUNC) &savvy_OpendalFs_read_bytes__impl, 10},
    {"savvy_OpendalFs_read_bytes_aio__impl", (DL_FUNC) &savvy_OpendalFs_read_bytes_aio__impl, 10},
    {"savvy_OpendalFs_read_iter__impl", (DL_FUNC) &savvy_OpendalFs_read_iter__impl, 7},
    {"savvy_OpendalFs_rename__impl", (DL_FUNC) &savvy_OpendalFs_rename__impl, 3},
    {"savvy_OpendalFs_rename_aio__impl", (DL_FUNC) &savvy_OpendalFs_rename_aio__impl, 4},
    {"savvy_OpendalFs_replace__impl", (DL_FUNC) &savvy_OpendalFs_replace__impl, 6},
    {"savvy_OpendalFs_replace_aio__impl", (DL_FUNC) &savvy_OpendalFs_replace_aio__impl, 6},
    {"savvy_OpendalFs_stat__impl", (DL_FUNC) &savvy_OpendalFs_stat__impl, 3},
    {"savvy_OpendalFs_stat_aio__impl", (DL_FUNC) &savvy_OpendalFs_stat_aio__impl, 3},
    {"savvy_OpendalFs_walk_iter__impl", (DL_FUNC) &savvy_OpendalFs_walk_iter__impl, 6},
    {"savvy_OpendalFs_write__impl", (DL_FUNC) &savvy_OpendalFs_write__impl, 6},
    {"savvy_OpendalFs_write_aio__impl", (DL_FUNC) &savvy_OpendalFs_write_aio__impl, 6},
    {"savvy_OpendalFs_write_iter__impl", (DL_FUNC) &savvy_OpendalFs_write_iter__impl, 6},
    {"savvy_OpendalHttpFixture_endpoint__impl", (DL_FUNC) &savvy_OpendalHttpFixture_endpoint__impl, 1},
    {"savvy_OpendalHttpFixture_root__impl", (DL_FUNC) &savvy_OpendalHttpFixture_root__impl, 1},
    {"savvy_OpendalHttpFixture_start__impl", (DL_FUNC) &savvy_OpendalHttpFixture_start__impl, 3},
    {"savvy_OpendalHttpFixture_stop__impl", (DL_FUNC) &savvy_OpendalHttpFixture_stop__impl, 1},
    {"savvy_OpendalLsIter_collect__impl", (DL_FUNC) &savvy_OpendalLsIter_collect__impl, 1},
    {"savvy_OpendalLsIter_next__impl", (DL_FUNC) &savvy_OpendalLsIter_next__impl, 1},
    {"savvy_OpendalReadIter_collect__impl", (DL_FUNC) &savvy_OpendalReadIter_collect__impl, 1},
    {"savvy_OpendalReadIter_next__impl", (DL_FUNC) &savvy_OpendalReadIter_next__impl, 1},
    {"savvy_OpendalReadIter_seek__impl", (DL_FUNC) &savvy_OpendalReadIter_seek__impl, 3},
    {"savvy_OpendalReadIter_tell__impl", (DL_FUNC) &savvy_OpendalReadIter_tell__impl, 1},
    {"savvy_OpendalWriteIter_close__impl", (DL_FUNC) &savvy_OpendalWriteIter_close__impl, 1},
    {"savvy_OpendalWriteIter_tell__impl", (DL_FUNC) &savvy_OpendalWriteIter_tell__impl, 1},
    {"savvy_OpendalWriteIter_write__impl", (DL_FUNC) &savvy_OpendalWriteIter_write__impl, 2},
    {NULL, NULL, 0}
};

void R_init_Ropendal(DllInfo *dll) {
    R_registerRoutines(dll, NULL, CallEntries, NULL, NULL);
    R_useDynamicSymbols(dll, FALSE);

    // Functions for initialization, if any.

}
