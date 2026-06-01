library(Ropendal)

helper <- system.file("tinytest", "helper-ropendal.R", package = "Ropendal")
source(helper, local = TRUE)

if (!ropendal_service_tests_enabled("gdrive")) exit_file("set ROPENDAL_TEST_NETWORK=true and ROPENDAL_TEST_GDRIVE=true to run")

secret_json <- Sys.getenv("ROPENDAL_GDRIVE_SECRET_JSON", unset = "")
tokens_json <- Sys.getenv("ROPENDAL_GDRIVE_TOKENS_JSON", unset = "")
gdrive_root <- Sys.getenv("ROPENDAL_GDRIVE_ROOT", unset = Sys.getenv("ROPENDAL_TEST_GDRIVE_ROOT", unset = ""))
gdrive_file <- Sys.getenv("ROPENDAL_GDRIVE_FILE", unset = "map_catalog.txt")

missing <- ropendal_missing_env(c("ROPENDAL_GDRIVE_SECRET_JSON", "ROPENDAL_GDRIVE_ROOT"))
if (length(missing)) exit_file(paste("missing env vars:", paste(missing, collapse = ", ")))
if (!file.exists(secret_json)) exit_file("ROPENDAL_GDRIVE_SECRET_JSON does not exist")
if (!nzchar(tokens_json)) tokens_json <- file.path(dirname(secret_json), "tokens.json")
if (!file.exists(tokens_json)) exit_file("ROPENDAL_GDRIVE_TOKENS_JSON does not exist")

auth <- credentials_gdrive3(secret_json = secret_json, tokens_json = tokens_json)
fs <- opendal("gdrive", root = gdrive_root, auth = auth)

entries <- fs_ls(fs)
expect_false(is_error_value(entries))
paths <- vapply(entries, `[[`, character(1), "path")
expect_true(gdrive_file %in% paths)

stat <- fs_stat(fs, gdrive_file)
expect_false(is_error_value(stat))
expect_equal(stat$type, "file")
expect_true(stat$size > 0)

head <- fs_read(fs, gdrive_file, offset = 0, size = min(64, stat$size))
expect_true(is.raw(head))
expect_true(length(head) > 0)
