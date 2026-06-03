#!/usr/bin/env Rscript

# Development-only read stress benchmark for a large Google Drive object versus
# a local MinIO S3-compatible object. It reads only ROPENDAL_BENCH_BYTES from the
# selected file, so it is suitable for a huge Drive object such as a multi-GB
# archive in KidneyGWAS without downloading the whole object.
#
# Common invocation:
#   ROPENDAL_BENCH_BYTES=256MiB \
#   ROPENDAL_BENCH_CONCURRENCY=1,2,4,8 \
#   ROPENDAL_BENCH_GDRIVE_ROOT=KidneyGWAS \
#   ROPENDAL_BENCH_GDRIVE_FILE=KidneyPROJECTFILES.zip \
#   Rscript benchmarks/gdrive-minio-read-stress.R

suppressPackageStartupMessages({
  library(Ropendal)
})

parse_bytes <- function(x, default) {
  if (!nzchar(x)) return(default)
  x <- trimws(x)
  m <- regexec("^([0-9.]+)\\s*([KMGT]?i?B?|[kmgt]i?b?)?$", x)
  hit <- regmatches(x, m)[[1]]
  if (!length(hit)) stop("invalid byte size: ", x, call. = FALSE)
  n <- as.numeric(hit[[2]])
  suffix <- toupper(hit[[3]])
  if (!nzchar(suffix)) suffix <- "B"
  mult <- switch(suffix,
    "B" = 1,
    "K" = 1000, "KB" = 1000, "KI" = 1024, "KIB" = 1024,
    "M" = 1000^2, "MB" = 1000^2, "MI" = 1024^2, "MIB" = 1024^2,
    "G" = 1000^3, "GB" = 1000^3, "GI" = 1024^3, "GIB" = 1024^3,
    "T" = 1000^4, "TB" = 1000^4, "TI" = 1024^4, "TIB" = 1024^4,
    stop("invalid byte suffix: ", suffix, call. = FALSE)
  )
  as.numeric(n * mult)
}

fmt_bytes <- function(x) {
  units <- c("B", "KiB", "MiB", "GiB", "TiB")
  i <- 1L
  while (x >= 1024 && i < length(units)) {
    x <- x / 1024
    i <- i + 1L
  }
  sprintf("%.2f %s", x, units[[i]])
}

parse_ints <- function(x, default) {
  if (!nzchar(x)) return(default)
  as.integer(strsplit(gsub("\\s+", "", x), ",", fixed = FALSE)[[1]])
}

first_existing_gdrive_secret <- function() {
  root <- path.expand("~/.config/gdrive3")
  if (!dir.exists(root)) return("")
  hits <- Sys.glob(file.path(root, "*", "secret.json"))
  if (length(hits)) hits[[1]] else ""
}

find_gdrive_tokens <- function(secret) {
  explicit <- Sys.getenv("ROPENDAL_GDRIVE_TOKENS_JSON", unset = "")
  if (nzchar(explicit)) return(explicit)
  if (!nzchar(secret)) return("")
  file.path(dirname(secret), "tokens.json")
}

is_bad <- function(x) is_error_value(x)

stop_if_bad <- function(x, context) {
  if (is_bad(x)) stop(context, ": ", format(x), call. = FALSE)
  invisible(x)
}

payload_len <- function(x) {
  stop_if_bad(x, "read failed")
  if (inherits(x, "OpendalBytes") || is.raw(x)) return(length(x))
  if (is.list(x)) return(sum(vapply(x, payload_len, numeric(1))))
  stop("unexpected read result type: ", paste(class(x), collapse = "/"), call. = FALSE)
}

bench_read <- function(service, mode, concurrency, bytes, expr) {
  invisible(gc())
  start <- proc.time()[["elapsed"]]
  value <- eval.parent(substitute(expr))
  elapsed <- proc.time()[["elapsed"]] - start
  n <- payload_len(value)
  rm(value)
  invisible(gc())
  data.frame(
    service = service,
    mode = mode,
    concurrency = concurrency,
    bytes = n,
    elapsed_sec = elapsed,
    mib_per_sec = n / 1024^2 / elapsed,
    stringsAsFactors = FALSE
  )
}

read_prefix <- function(fs, path, total_bytes, concurrency, chunk_size) {
  fs_read_bytes(
    fs,
    path,
    offset = 0,
    size = total_bytes,
    read_concurrency = as.numeric(concurrency),
    chunk_size = as.numeric(chunk_size)
  )
}

read_ranges <- function(fs, path, total_bytes, concurrency, chunk_size) {
  n <- ceiling(total_bytes / chunk_size)
  offsets <- seq(0, by = chunk_size, length.out = n)
  sizes <- rep(chunk_size, n)
  sizes[[n]] <- total_bytes - sum(sizes[-n])
  fs_read_bytes(
    fs,
    rep(path, n),
    offset = offsets,
    size = sizes,
    result = "flat",
    batch_concurrency = as.numeric(concurrency)
  )
}

choose_gdrive_file <- function(fs, limit) {
  entries <- fs_ls(fs, recursive = TRUE, limit = limit)
  stop_if_bad(entries, "gdrive list failed")
  paths <- vapply(entries, `[[`, character(1), "path")
  types <- vapply(entries, function(x) if (is.null(x$type)) NA_character_ else x$type, character(1))
  paths <- paths[is.na(types) | types == "file"]
  if (!length(paths)) stop("no files found under Google Drive root", call. = FALSE)
  sizes <- vapply(paths, function(path) {
    st <- fs_stat(fs, path)
    if (is_bad(st)) return(NA_real_)
    st$size
  }, numeric(1))
  paths[[which.max(sizes)]]
}

free_port <- function() {
  code <- paste(
    "import socket",
    "s=socket.socket()",
    "s.bind(('127.0.0.1',0))",
    "print(s.getsockname()[1])",
    "s.close()",
    sep = ";"
  )
  as.integer(system2("python3", c("-c", shQuote(code)), stdout = TRUE))
}

start_minio <- function() {
  if (!requireNamespace("processx", quietly = TRUE)) {
    stop("processx is required for the MinIO part of this benchmark", call. = FALSE)
  }
  minio_bin <- Sys.which("minio")
  mc_bin <- Sys.which("mc")
  if (!nzchar(minio_bin)) stop("minio binary is required", call. = FALSE)
  if (!nzchar(mc_bin)) stop("mc binary is required", call. = FALSE)

  data_dir <- tempfile("ropendal-minio-stress-data-")
  dir.create(data_dir, recursive = TRUE)
  log_file <- tempfile("ropendal-minio-stress-log-")
  port <- free_port()
  console_port <- free_port()
  while (identical(console_port, port)) console_port <- free_port()
  endpoint <- sprintf("http://127.0.0.1:%d", port)
  access_key <- "minioadmin"
  secret_key <- "minioadmin"

  env <- Sys.getenv()
  env[["MINIO_ROOT_USER"]] <- access_key
  env[["MINIO_ROOT_PASSWORD"]] <- secret_key
  proc <- processx::process$new(
    minio_bin,
    c(
      "server", data_dir,
      "--address", sprintf("127.0.0.1:%d", port),
      "--console-address", sprintf("127.0.0.1:%d", console_port)
    ),
    env = env,
    stdout = log_file,
    stderr = log_file
  )

  alias <- paste0("ropendal-stress-", Sys.getpid())
  ready <- FALSE
  for (i in seq_len(120)) {
    ok <- processx::run(mc_bin, c("alias", "set", alias, endpoint, access_key, secret_key), error_on_status = FALSE)$status == 0
    if (ok) {
      ready <- TRUE
      break
    }
    if (!proc$is_alive()) stop("minio exited before becoming ready; see ", log_file, call. = FALSE)
    Sys.sleep(0.25)
  }
  if (!ready) stop("minio did not become ready; see ", log_file, call. = FALSE)

  bucket <- "ropendal-stress"
  processx::run(mc_bin, c("mb", "--ignore-existing", paste0(alias, "/", bucket)), error_on_status = TRUE)

  list(
    proc = proc,
    data_dir = data_dir,
    log_file = log_file,
    alias = alias,
    endpoint = endpoint,
    bucket = bucket,
    access_key = access_key,
    secret_key = secret_key,
    region = "us-east-1"
  )
}

cleanup_minio <- function(minio) {
  if (is.null(minio)) return(invisible())
  mc_bin <- Sys.which("mc")
  if (nzchar(mc_bin)) try(processx::run(mc_bin, c("alias", "remove", minio$alias), error_on_status = FALSE), silent = TRUE)
  try(minio$proc$kill(), silent = TRUE)
  unlink(minio$data_dir, recursive = TRUE)
  unlink(minio$log_file)
  invisible()
}

total_bytes <- parse_bytes(Sys.getenv("ROPENDAL_BENCH_BYTES", unset = ""), 512 * 1024^2)
chunk_size <- parse_bytes(Sys.getenv("ROPENDAL_BENCH_CHUNK_SIZE", unset = ""), 16 * 1024^2)
concurrency <- parse_ints(Sys.getenv("ROPENDAL_BENCH_CONCURRENCY", unset = ""), c(1L, 2L, 4L, 8L))
runtime_threads <- as.integer(Sys.getenv("ROPENDAL_BENCH_RUNTIME_THREADS", unset = max(4L, max(concurrency))))
max_inflight <- as.integer(Sys.getenv("ROPENDAL_BENCH_MAX_INFLIGHT", unset = max(16L, max(concurrency) * 2L)))
run_minio <- !identical(tolower(Sys.getenv("ROPENDAL_BENCH_MINIO", unset = "true")), "false")

message("Stress bytes: ", fmt_bytes(total_bytes))
message("Chunk size:   ", fmt_bytes(chunk_size))
message("Concurrency:  ", paste(concurrency, collapse = ", "))

results <- list()

secret_json <- Sys.getenv("ROPENDAL_GDRIVE_SECRET_JSON", unset = first_existing_gdrive_secret())
tokens_json <- find_gdrive_tokens(secret_json)
gdrive_root <- Sys.getenv("ROPENDAL_BENCH_GDRIVE_ROOT", unset = Sys.getenv("ROPENDAL_GDRIVE_ROOT", unset = "KidneyGWAS"))
gdrive_file <- Sys.getenv("ROPENDAL_BENCH_GDRIVE_FILE", unset = Sys.getenv("ROPENDAL_GDRIVE_FILE", unset = ""))

if (nzchar(secret_json) && file.exists(secret_json) && nzchar(tokens_json) && file.exists(tokens_json)) {
  message("Google Drive root: ", gdrive_root)
  auth <- credentials_gdrive3(secret_json = secret_json, tokens_json = tokens_json)
  gfs <- opendal("gdrive", root = gdrive_root, auth = auth, runtime_threads = runtime_threads, max_inflight = max_inflight)
  if (!nzchar(gdrive_file)) {
    gdrive_file <- choose_gdrive_file(gfs, as.integer(Sys.getenv("ROPENDAL_BENCH_GDRIVE_LS_LIMIT", unset = 500L)))
  }
  gstat <- fs_stat(gfs, gdrive_file)
  stop_if_bad(gstat, "gdrive stat failed")
  if (gstat$size < total_bytes) stop("selected Google Drive file is smaller than requested read size", call. = FALSE)
  message("Google Drive file: ", gdrive_file)
  message("Google Drive file size: ", fmt_bytes(gstat$size))
  invisible(payload_len(fs_read_bytes(gfs, gdrive_file, offset = 0, size = min(1024^2, total_bytes))))
  for (cc in concurrency) {
    message("gdrive prefix read_concurrency=", cc)
    results[[length(results) + 1L]] <- bench_read("gdrive", "prefix", cc, total_bytes, read_prefix(gfs, gdrive_file, total_bytes, cc, chunk_size))
  }
  for (cc in concurrency) {
    message("gdrive ranges batch_concurrency=", cc)
    results[[length(results) + 1L]] <- bench_read("gdrive", "ranges", cc, total_bytes, read_ranges(gfs, gdrive_file, total_bytes, cc, chunk_size))
  }
} else {
  message("Skipping Google Drive: set ROPENDAL_GDRIVE_SECRET_JSON and ROPENDAL_GDRIVE_TOKENS_JSON")
}

if (run_minio) {
  minio <- NULL
  minio <- start_minio()
  on.exit(cleanup_minio(minio), add = TRUE)
  auth <- credentials_s3(
    access_key_id = minio$access_key,
    secret_access_key = minio$secret_key,
    region = minio$region,
    source = "minio-stress"
  )
  mfs <- opendal(
    "s3",
    endpoint = minio$endpoint,
    bucket = minio$bucket,
    root = paste0("stress-", Sys.getpid()),
    region = minio$region,
    disable_config_load = TRUE,
    disable_ec2_metadata = TRUE,
    auth = auth,
    runtime_threads = runtime_threads,
    max_inflight = max_inflight
  )
  key <- "payload.bin"
  message("Preparing MinIO payload: ", fmt_bytes(total_bytes))
  payload <- rep(as.raw(0:255), length.out = total_bytes)
  stop_if_bad(fs_replace(mfs, key, payload, write_concurrency = as.numeric(max(concurrency)), chunk_size = as.numeric(max(5 * 1024^2, chunk_size))), "minio payload upload failed")
  rm(payload)
  invisible(gc())
  invisible(payload_len(fs_read_bytes(mfs, key, offset = 0, size = min(1024^2, total_bytes))))
  for (cc in concurrency) {
    message("minio prefix read_concurrency=", cc)
    results[[length(results) + 1L]] <- bench_read("minio", "prefix", cc, total_bytes, read_prefix(mfs, key, total_bytes, cc, chunk_size))
  }
  for (cc in concurrency) {
    message("minio ranges batch_concurrency=", cc)
    results[[length(results) + 1L]] <- bench_read("minio", "ranges", cc, total_bytes, read_ranges(mfs, key, total_bytes, cc, chunk_size))
  }
  cleanup_minio(minio)
  minio <- NULL
}

out <- do.call(rbind, results)
out$bytes_read <- vapply(out$bytes, fmt_bytes, character(1))
out <- out[, c("service", "mode", "concurrency", "bytes_read", "elapsed_sec", "mib_per_sec")]
out$elapsed_sec <- round(out$elapsed_sec, 3)
out$mib_per_sec <- round(out$mib_per_sec, 2)
print(out, row.names = FALSE)

outfile <- Sys.getenv("ROPENDAL_BENCH_OUT", unset = "")
if (nzchar(outfile)) {
  utils::write.csv(out, outfile, row.names = FALSE)
  message("Wrote ", outfile)
}
