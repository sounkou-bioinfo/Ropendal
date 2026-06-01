## Shared tinytest helpers for Ropendal.
## Keep helpers dependency-light: base R + tinytest only.
##
## tinytest's exit_file() only works reliably as a direct top-level call in the
## test file, and should be called unqualified. Therefore helpers expose
## predicates; test files should gate with:
##
## if (!ropendal_ci_tests_enabled()) exit_file("set ROPENDAL_TEST_CI=true to run")

ropendal_env_flag <- function(name, default = FALSE) {
  value <- Sys.getenv(name, unset = NA_character_)
  if (is.na(value) || !nzchar(value)) {
    return(isTRUE(default))
  }
  tolower(value) %in% c("1", "true", "yes", "y", "on")
}

ropendal_ci_tests_enabled <- function() {
  ropendal_env_flag("ROPENDAL_TEST_CI")
}

ropendal_network_tests_enabled <- function() {
  ropendal_env_flag("ROPENDAL_TEST_NETWORK")
}

ropendal_service_tests_enabled <- function(service) {
  ropendal_network_tests_enabled() &&
    ropendal_env_flag(paste0("ROPENDAL_TEST_", toupper(service)))
}

ropendal_missing_env <- function(vars) {
  vars[!nzchar(Sys.getenv(vars, unset = ""))]
}

ropendal_temp_root <- function(prefix = "ropendal-") {
  root <- tempfile(prefix)
  dir.create(root, recursive = TRUE, showWarnings = FALSE)
  normalizePath(root, winslash = "/", mustWork = TRUE)
}

ropendal_bytes <- function(n = 256L) {
  as.raw((seq_len(n) - 1L) %% 256L)
}
