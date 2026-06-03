library(Ropendal)

s3_provider <- credentials_s3(
  access_key_id = "access-key",
  secret_access_key = "secret-key",
  session_token = "session-token",
  region = "us-east-1"
)
expect_true(S7::S7_inherits(s3_provider, CredentialProvider))
expect_equal(credential_schemes(s3_provider), "s3")
s3_cfg <- credential_config(s3_provider, "s3")
expect_equal(names(s3_cfg), c("access_key_id", "secret_access_key", "session_token", "region"))
expect_equal(s3_cfg$access_key_id, "access-key")
expect_equal(s3_cfg$secret_access_key, "secret-key")
expect_equal(s3_cfg$session_token, "session-token")
expect_equal(s3_cfg$region, "us-east-1")
expect_error(credential_config(s3_provider, "gdrive"), "cannot be used")
expect_true(grepl("<redacted>", capture.output(print(s3_provider)), fixed = TRUE))
expect_error(credentials_s3("", "secret-key"), "access_key_id")
expect_error(credentials_s3("access-key", ""), "secret_access_key")

gcs_token_provider <- credentials_gcs(token = "gcs-token", scope = "scope-a")
expect_true(S7::S7_inherits(gcs_token_provider, CredentialProvider))
expect_equal(credential_schemes(gcs_token_provider), "gcs")
gcs_token_cfg <- credential_config(gcs_token_provider, "gcs")
expect_equal(gcs_token_cfg$token, "gcs-token")
expect_equal(gcs_token_cfg$scope, "scope-a")
expect_equal(gcs_token_cfg$disable_config_load, "true")
expect_equal(gcs_token_cfg$disable_vm_metadata, "true")
expect_equal(credential_summary(gcs_token_provider)$method, "token")
expect_true(grepl("<redacted>", capture.output(print(gcs_token_provider)), fixed = TRUE))

gcs_key_provider <- credentials_gcs(service_account_key = "{json}")
gcs_key_cfg <- credential_config(gcs_key_provider, "gcs")
expect_equal(names(gcs_key_cfg), c("credential", "disable_config_load", "disable_vm_metadata"))
expect_equal(gcs_key_cfg$credential, "{json}")
expect_equal(credential_summary(gcs_key_provider)$method, "service_account_key")

gcs_path_provider <- credentials_gcs(credential_path = "/tmp/gcs.json")
gcs_path_cfg <- credential_config(gcs_path_provider, "gcs")
expect_equal(gcs_path_cfg$credential_path, "/tmp/gcs.json")
expect_error(credential_config(gcs_path_provider, "s3"), "cannot be used")
expect_error(credentials_gcs(), "exactly one")
expect_error(credentials_gcs(token = "a", service_account_key = "b"), "exactly one")

az_key_provider <- credentials_azblob(
  account_name = "account",
  account_key = "account-key",
  endpoint = "https://example.blob.core.windows.net"
)
expect_true(S7::S7_inherits(az_key_provider, CredentialProvider))
expect_equal(credential_schemes(az_key_provider), "azblob")
az_key_cfg <- credential_config(az_key_provider, "azblob")
expect_equal(names(az_key_cfg), c("account_name", "account_key", "endpoint"))
expect_equal(az_key_cfg$account_name, "account")
expect_equal(az_key_cfg$account_key, "account-key")
expect_equal(az_key_cfg$endpoint, "https://example.blob.core.windows.net")
expect_equal(credential_summary(az_key_provider)$method, "account_key")
expect_true(grepl("<redacted>", capture.output(print(az_key_provider)), fixed = TRUE))

az_sas_provider <- credentials_azblob(sas_token = "sas-token")
az_sas_cfg <- credential_config(az_sas_provider, "azblob")
expect_equal(names(az_sas_cfg), "sas_token")
expect_equal(az_sas_cfg$sas_token, "sas-token")
expect_error(credential_config(az_sas_provider, "gcs"), "cannot be used")
expect_error(credentials_azblob(account_key = "key"), "account_name")
expect_error(credentials_azblob(account_name = "account"), "account_key or sas_token")
expect_error(credentials_azblob(account_name = "account", account_key = "key", sas_token = "sas"), "only one")

refresh_provider <- credentials_gdrive(
  refresh_token = "refresh-token",
  client_id = "client-id",
  client_secret = "client-secret"
)
expect_true(S7::S7_inherits(refresh_provider, CredentialProvider))
expect_equal(credential_schemes(refresh_provider), "gdrive")
refresh_cfg <- credential_config(refresh_provider, "gdrive")
expect_equal(names(refresh_cfg), c("refresh_token", "client_id", "client_secret"))
expect_equal(refresh_cfg$refresh_token, "refresh-token")
expect_error(credential_config(refresh_provider, "s3"), "cannot be used")
expect_true(grepl("<redacted>", capture.output(print(refresh_provider)), fixed = TRUE))

access_provider <- credentials_gdrive(access_token = "access-token")
expect_true(S7::S7_inherits(access_provider, CredentialProvider))
access_cfg <- credential_config(access_provider, "gdrive")
expect_equal(names(access_cfg), "access_token")
expect_equal(access_cfg$access_token, "access-token")

expect_error(credentials_gdrive(), "access_token or refresh_token")
expect_error(credentials_gdrive(access_token = "a", refresh_token = "r"), "only one")
expect_error(credentials_gdrive(refresh_token = "r"), "client_id is required")

root <- tempfile("ropendal-gdrive-credentials-")
dir.create(root)
secret_json <- file.path(root, "secret.json")
tokens_json <- file.path(root, "tokens.json")
writeLines('{"client_id":"json-client","client_secret":"json-secret"}', secret_json)
writeLines('[{"scopes":["https://www.googleapis.com/auth/drive"],"token":{"refresh_token":"json-refresh"}}]', tokens_json)
json_provider <- credentials_gdrive3(secret_json, tokens_json)
json_cfg <- credential_config(json_provider, "gdrive")
expect_equal(json_cfg$refresh_token, "json-refresh")
expect_equal(json_cfg$client_id, "json-client")
expect_equal(json_cfg$client_secret, "json-secret")
expect_true(startsWith(credential_summary(json_provider)$source, "gdrive3:"))
