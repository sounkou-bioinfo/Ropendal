SEXP savvy_opendal_bytes_as_raw__ffi(SEXP c_arg__bytes);
SEXP savvy_opendal_bytes_len__ffi(SEXP c_arg__bytes);
SEXP savvy_opendal_codec_decode__ffi(SEXP c_arg__name, SEXP c_arg__data);
SEXP savvy_opendal_codec_encode__ffi(SEXP c_arg__name, SEXP c_arg__data);

// methods and associated functions for OpendalAio
SEXP savvy_OpendalAio_cancel__ffi(SEXP self__);
SEXP savvy_OpendalAio_collect__ffi(SEXP self__);
SEXP savvy_OpendalAio_error_value__ffi(SEXP self__);
SEXP savvy_OpendalAio_poll__ffi(SEXP self__);
SEXP savvy_OpendalAio_state_name__ffi(SEXP self__);

// methods and associated functions for OpendalCredentialProvider
SEXP savvy_OpendalCredentialProvider_azblob__ffi(SEXP c_arg__account_name, SEXP c_arg__account_key, SEXP c_arg__sas_token, SEXP c_arg__endpoint, SEXP c_arg__source);
SEXP savvy_OpendalCredentialProvider_config__ffi(SEXP self__, SEXP c_arg__service);
SEXP savvy_OpendalCredentialProvider_gcs__ffi(SEXP c_arg__token, SEXP c_arg__service_account_key, SEXP c_arg__credential_path, SEXP c_arg__scope, SEXP c_arg__source);
SEXP savvy_OpendalCredentialProvider_gdrive__ffi(SEXP c_arg__access_token, SEXP c_arg__refresh_token, SEXP c_arg__client_id, SEXP c_arg__client_secret, SEXP c_arg__source);
SEXP savvy_OpendalCredentialProvider_gdrive3__ffi(SEXP c_arg__secret_json, SEXP c_arg__tokens_json, SEXP c_arg__scope);
SEXP savvy_OpendalCredentialProvider_s3__ffi(SEXP c_arg__access_key_id, SEXP c_arg__secret_access_key, SEXP c_arg__session_token, SEXP c_arg__region, SEXP c_arg__source);
SEXP savvy_OpendalCredentialProvider_schemes__ffi(SEXP self__);
SEXP savvy_OpendalCredentialProvider_summary__ffi(SEXP self__);

// methods and associated functions for OpendalFs
SEXP savvy_OpendalFs_append__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__data, SEXP c_arg__batch_concurrency, SEXP c_arg__write_concurrency, SEXP c_arg__chunk_size);
SEXP savvy_OpendalFs_append_aio__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__data, SEXP c_arg__batch_concurrency, SEXP c_arg__write_concurrency, SEXP c_arg__chunk_size);
SEXP savvy_OpendalFs_capabilities__ffi(SEXP self__);
SEXP savvy_OpendalFs_copy__ffi(SEXP self__, SEXP c_arg__from, SEXP c_arg__to);
SEXP savvy_OpendalFs_copy_aio__ffi(SEXP self__, SEXP c_arg__from, SEXP c_arg__to, SEXP c_arg__batch_concurrency);
SEXP savvy_OpendalFs_delete__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__recursive, SEXP c_arg__batch_concurrency);
SEXP savvy_OpendalFs_delete_aio__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__recursive, SEXP c_arg__batch_concurrency);
SEXP savvy_OpendalFs_exists__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__batch_concurrency);
SEXP savvy_OpendalFs_exists_aio__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__batch_concurrency);
SEXP savvy_OpendalFs_from_uri__ffi(SEXP c_arg__uri, SEXP c_arg__headers, SEXP c_arg__runtime_threads, SEXP c_arg__max_inflight);
SEXP savvy_OpendalFs_info__ffi(SEXP self__);
SEXP savvy_OpendalFs_ls__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__recursive, SEXP c_arg__limit, SEXP c_arg__start_after);
SEXP savvy_OpendalFs_ls_aio__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__recursive, SEXP c_arg__limit, SEXP c_arg__start_after);
SEXP savvy_OpendalFs_ls_iter__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__recursive, SEXP c_arg__page_size, SEXP c_arg__limit, SEXP c_arg__start_after, SEXP c_arg__prefetch);
SEXP savvy_OpendalFs_mkdir__ffi(SEXP self__, SEXP c_arg__path);
SEXP savvy_OpendalFs_mkdir_aio__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__batch_concurrency);
SEXP savvy_OpendalFs_normalize_path__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__directory);
SEXP savvy_OpendalFs_open__ffi(SEXP c_arg__scheme, SEXP c_arg__dots, SEXP c_arg__config, SEXP c_arg__root, SEXP c_arg__auth_config, SEXP c_arg__headers, SEXP c_arg__runtime_threads, SEXP c_arg__max_inflight);
SEXP savvy_OpendalFs_read__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__offset, SEXP c_arg__size, SEXP c_arg__end, SEXP c_arg__result, SEXP c_arg__batch_concurrency, SEXP c_arg__read_concurrency, SEXP c_arg__chunk_size, SEXP c_arg__coalesce_gap);
SEXP savvy_OpendalFs_read_aio__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__offset, SEXP c_arg__size, SEXP c_arg__end, SEXP c_arg__result, SEXP c_arg__batch_concurrency, SEXP c_arg__read_concurrency, SEXP c_arg__chunk_size, SEXP c_arg__coalesce_gap);
SEXP savvy_OpendalFs_read_bytes__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__offset, SEXP c_arg__size, SEXP c_arg__end, SEXP c_arg__result, SEXP c_arg__batch_concurrency, SEXP c_arg__read_concurrency, SEXP c_arg__chunk_size, SEXP c_arg__coalesce_gap);
SEXP savvy_OpendalFs_read_bytes_aio__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__offset, SEXP c_arg__size, SEXP c_arg__end, SEXP c_arg__result, SEXP c_arg__batch_concurrency, SEXP c_arg__read_concurrency, SEXP c_arg__chunk_size, SEXP c_arg__coalesce_gap);
SEXP savvy_OpendalFs_read_iter__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__chunk_size, SEXP c_arg__offset, SEXP c_arg__size, SEXP c_arg__read_concurrency, SEXP c_arg__coalesce_gap);
SEXP savvy_OpendalFs_rename__ffi(SEXP self__, SEXP c_arg__from, SEXP c_arg__to);
SEXP savvy_OpendalFs_rename_aio__ffi(SEXP self__, SEXP c_arg__from, SEXP c_arg__to, SEXP c_arg__batch_concurrency);
SEXP savvy_OpendalFs_replace__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__data, SEXP c_arg__batch_concurrency, SEXP c_arg__write_concurrency, SEXP c_arg__chunk_size);
SEXP savvy_OpendalFs_replace_aio__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__data, SEXP c_arg__batch_concurrency, SEXP c_arg__write_concurrency, SEXP c_arg__chunk_size);
SEXP savvy_OpendalFs_stat__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__batch_concurrency);
SEXP savvy_OpendalFs_stat_aio__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__batch_concurrency);
SEXP savvy_OpendalFs_walk_iter__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__page_size, SEXP c_arg__limit, SEXP c_arg__start_after, SEXP c_arg__prefetch);
SEXP savvy_OpendalFs_write__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__data, SEXP c_arg__batch_concurrency, SEXP c_arg__write_concurrency, SEXP c_arg__chunk_size);
SEXP savvy_OpendalFs_write_aio__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__data, SEXP c_arg__batch_concurrency, SEXP c_arg__write_concurrency, SEXP c_arg__chunk_size);
SEXP savvy_OpendalFs_write_iter__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__create, SEXP c_arg__append, SEXP c_arg__write_concurrency, SEXP c_arg__chunk_size);

// methods and associated functions for OpendalHttpFixture
SEXP savvy_OpendalHttpFixture_endpoint__ffi(SEXP self__);
SEXP savvy_OpendalHttpFixture_root__ffi(SEXP self__);
SEXP savvy_OpendalHttpFixture_start__ffi(SEXP c_arg__root, SEXP c_arg__required_headers, SEXP c_arg__delay_ms);
SEXP savvy_OpendalHttpFixture_stop__ffi(SEXP self__);

// methods and associated functions for OpendalLsIter
SEXP savvy_OpendalLsIter_collect__ffi(SEXP self__);
SEXP savvy_OpendalLsIter_next__ffi(SEXP self__);

// methods and associated functions for OpendalReadIter
SEXP savvy_OpendalReadIter_collect__ffi(SEXP self__);
SEXP savvy_OpendalReadIter_next__ffi(SEXP self__);
SEXP savvy_OpendalReadIter_seek__ffi(SEXP self__, SEXP c_arg__offset, SEXP c_arg__whence);
SEXP savvy_OpendalReadIter_tell__ffi(SEXP self__);

// methods and associated functions for OpendalWriteIter
SEXP savvy_OpendalWriteIter_close__ffi(SEXP self__);
SEXP savvy_OpendalWriteIter_tell__ffi(SEXP self__);
SEXP savvy_OpendalWriteIter_write__ffi(SEXP self__, SEXP c_arg__data);
