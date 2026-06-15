#include <stdint.h>
#include "../inst/include/ropendal.h"

/*
 * Keep this file in sync with inst/include/ropendal.h.
 *
 * The savvy-generated src/init.c registers R's .Call entry points only. The
 * pure C API below is a separate downstream-native ABI implemented in Rust and
 * documented by inst/include/ropendal.h; most of these symbols are not called
 * by R's registration layer.
 *
 * When the Rust library is linked into Ropendal.so, link-time dead stripping can
 * otherwise decide that unreferenced extern "C" functions are unnecessary. This
 * anchor takes each public C API function's address from a C translation unit,
 * forcing the linker to retain the symbols in the installed shared library.
 * It is intentionally not R_RegisterCCallable(); downstream C users include the
 * header and link/load against the Ropendal native library directly.
 */

typedef void (*ropendal_any_fn)(void);

ropendal_any_fn ropendal_c_api_anchor[] = {
    (ropendal_any_fn)ropendal_api_version,
    (ropendal_any_fn)ropendal_fs_open,
    (ropendal_any_fn)ropendal_fs_from_uri,
    (ropendal_any_fn)ropendal_fs_retain,
    (ropendal_any_fn)ropendal_fs_release,
    (ropendal_any_fn)ropendal_store_open,
    (ropendal_any_fn)ropendal_store_cache_open,
    (ropendal_any_fn)ropendal_store_block_cache_open,
    (ropendal_any_fn)ropendal_store_retain,
    (ropendal_any_fn)ropendal_store_release,
    (ropendal_any_fn)ropendal_store_read_aio,
    (ropendal_any_fn)ropendal_store_read_into_aio,
    (ropendal_any_fn)ropendal_store_write_aio,
    (ropendal_any_fn)ropendal_store_replace_aio,
    (ropendal_any_fn)ropendal_store_exists_aio,
    (ropendal_any_fn)ropendal_store_ls_aio,
    (ropendal_any_fn)ropendal_store_delete_aio,
    (ropendal_any_fn)ropendal_codec_encode,
    (ropendal_any_fn)ropendal_codec_decode,
    (ropendal_any_fn)ropendal_bytes_data,
    (ropendal_any_fn)ropendal_bytes_len,
    (ropendal_any_fn)ropendal_bytes_release,
    (ropendal_any_fn)ropendal_read_aio,
    (ropendal_any_fn)ropendal_read_into_aio,
    (ropendal_any_fn)ropendal_readv_aio,
    (ropendal_any_fn)ropendal_readv_into_aio,
    (ropendal_any_fn)ropendal_write_aio,
    (ropendal_any_fn)ropendal_replace_aio,
    (ropendal_any_fn)ropendal_append_aio,
    (ropendal_any_fn)ropendal_stat_aio,
    (ropendal_any_fn)ropendal_exists_aio,
    (ropendal_any_fn)ropendal_ls_aio,
    (ropendal_any_fn)ropendal_delete_aio,
    (ropendal_any_fn)ropendal_copy_aio,
    (ropendal_any_fn)ropendal_rename_aio,
    (ropendal_any_fn)ropendal_mkdir_aio,
    (ropendal_any_fn)ropendal_cv_alloc,
    (ropendal_any_fn)ropendal_cv_release,
    (ropendal_any_fn)ropendal_cv_wait,
    (ropendal_any_fn)ropendal_cv_until,
    (ropendal_any_fn)ropendal_cv_value,
    (ropendal_any_fn)ropendal_cv_reset,
    (ropendal_any_fn)ropendal_cv_signal,
    (ropendal_any_fn)ropendal_aio_notify,
    (ropendal_any_fn)ropendal_monitor_create,
    (ropendal_any_fn)ropendal_monitor_add_aio,
    (ropendal_any_fn)ropendal_monitor_read,
    (ropendal_any_fn)ropendal_monitor_release,
    (ropendal_any_fn)ropendal_aio_poll,
    (ropendal_any_fn)ropendal_aio_wait,
    (ropendal_any_fn)ropendal_aio_cancel,
    (ropendal_any_fn)ropendal_aio_release,
    (ropendal_any_fn)ropendal_aio_result_bytes,
    (ropendal_any_fn)ropendal_aio_result_nread,
    (ropendal_any_fn)ropendal_aio_result_readv,
    (ropendal_any_fn)ropendal_aio_result_bool,
    (ropendal_any_fn)ropendal_aio_result_entries,
    (ropendal_any_fn)ropendal_aio_result_entry,
    (ropendal_any_fn)ropendal_error_message,
    (ropendal_any_fn)ropendal_error_kind,
    (ropendal_any_fn)ropendal_error_operation,
    (ropendal_any_fn)ropendal_error_path,
    (ropendal_any_fn)ropendal_error_release,
};

uintptr_t ropendal_c_api_anchor_len = sizeof(ropendal_c_api_anchor) / sizeof(ropendal_c_api_anchor[0]);
