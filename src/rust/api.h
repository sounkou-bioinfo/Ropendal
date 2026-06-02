// methods and associated functions for OpendalAio
SEXP savvy_OpendalAio_cancel__ffi(SEXP self__);
SEXP savvy_OpendalAio_collect__ffi(SEXP self__);
SEXP savvy_OpendalAio_poll__ffi(SEXP self__);

// methods and associated functions for OpendalCredentialProvider
SEXP savvy_OpendalCredentialProvider_config__ffi(SEXP self__, SEXP c_arg__service);
SEXP savvy_OpendalCredentialProvider_gdrive__ffi(SEXP c_arg__access_token, SEXP c_arg__refresh_token, SEXP c_arg__client_id, SEXP c_arg__client_secret, SEXP c_arg__source);
SEXP savvy_OpendalCredentialProvider_gdrive3__ffi(SEXP c_arg__secret_json, SEXP c_arg__tokens_json, SEXP c_arg__scope);
SEXP savvy_OpendalCredentialProvider_s3__ffi(SEXP c_arg__access_key_id, SEXP c_arg__secret_access_key, SEXP c_arg__session_token, SEXP c_arg__region, SEXP c_arg__source);
SEXP savvy_OpendalCredentialProvider_schemes__ffi(SEXP self__);
SEXP savvy_OpendalCredentialProvider_summary__ffi(SEXP self__);

// methods and associated functions for OpendalFs
SEXP savvy_OpendalFs_append__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__data, SEXP c_arg__batch_concurrency, SEXP c_arg__write_concurrency, SEXP c_arg__chunk_size);
SEXP savvy_OpendalFs_capabilities__ffi(SEXP self__);
SEXP savvy_OpendalFs_copy__ffi(SEXP self__, SEXP c_arg__from, SEXP c_arg__to);
SEXP savvy_OpendalFs_delete__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__recursive, SEXP c_arg__batch_concurrency);
SEXP savvy_OpendalFs_exists__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__batch_concurrency);
SEXP savvy_OpendalFs_from_uri__ffi(SEXP c_arg__uri);
SEXP savvy_OpendalFs_info__ffi(SEXP self__);
SEXP savvy_OpendalFs_ls__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__recursive);
SEXP savvy_OpendalFs_mkdir__ffi(SEXP self__, SEXP c_arg__path);
SEXP savvy_OpendalFs_normalize_path__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__directory);
SEXP savvy_OpendalFs_open__ffi(SEXP c_arg__scheme, SEXP c_arg__dots, SEXP c_arg__config, SEXP c_arg__root, SEXP c_arg__auth_config);
SEXP savvy_OpendalFs_read__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__offset, SEXP c_arg__size, SEXP c_arg__end, SEXP c_arg__result, SEXP c_arg__batch_concurrency, SEXP c_arg__read_concurrency, SEXP c_arg__chunk_size, SEXP c_arg__coalesce_gap);
SEXP savvy_OpendalFs_read_aio__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__offset, SEXP c_arg__size, SEXP c_arg__end, SEXP c_arg__result, SEXP c_arg__batch_concurrency, SEXP c_arg__read_concurrency, SEXP c_arg__chunk_size, SEXP c_arg__coalesce_gap);
SEXP savvy_OpendalFs_rename__ffi(SEXP self__, SEXP c_arg__from, SEXP c_arg__to);
SEXP savvy_OpendalFs_replace__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__data, SEXP c_arg__batch_concurrency, SEXP c_arg__write_concurrency, SEXP c_arg__chunk_size);
SEXP savvy_OpendalFs_stat__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__batch_concurrency);
SEXP savvy_OpendalFs_write__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__data, SEXP c_arg__batch_concurrency, SEXP c_arg__write_concurrency, SEXP c_arg__chunk_size);

// methods and associated functions for OpendalHttpFixture
SEXP savvy_OpendalHttpFixture_endpoint__ffi(SEXP self__);
SEXP savvy_OpendalHttpFixture_root__ffi(SEXP self__);
SEXP savvy_OpendalHttpFixture_start__ffi(SEXP c_arg__root);
SEXP savvy_OpendalHttpFixture_stop__ffi(SEXP self__);
