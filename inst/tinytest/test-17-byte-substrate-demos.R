library(Ropendal)

helper <- system.file("tinytest", "helper-ropendal.R", package = "Ropendal")
source(helper, local = TRUE)

root <- ropendal_temp_root("ropendal-byte-substrate-demos-")
fs <- opendal("fs", root = root)

encode_i32 <- function(x) writeBin(as.integer(x), raw(), size = 4L, endian = "little")
decode_i32 <- function(x, n) readBin(x, integer(), n = n, size = 4L, endian = "little")
chunk_key <- function(i, j) sprintf("c/%d/%d", i - 1L, j - 1L)

write_toy_array <- function(store, x, chunk_dim = c(2L, 2L)) {
  stopifnot(is.matrix(x), length(chunk_dim) == 2L)
  nr <- nrow(x)
  nc <- ncol(x)
  meta <- paste0(
    "dim=", nr, ",", nc, "\n",
    "chunk_dim=", chunk_dim[[1L]], ",", chunk_dim[[2L]], "\n",
    "type=int32\n"
  )
  store_write(store, "zarr.json", charToRaw(meta))
  for (i0 in seq(1L, nr, by = chunk_dim[[1L]])) {
    for (j0 in seq(1L, nc, by = chunk_dim[[2L]])) {
      rows <- i0:min(i0 + chunk_dim[[1L]] - 1L, nr)
      cols <- j0:min(j0 + chunk_dim[[2L]] - 1L, nc)
      store_write(store, chunk_key((i0 - 1L) %/% chunk_dim[[1L]] + 1L, (j0 - 1L) %/% chunk_dim[[2L]] + 1L), encode_i32(as.vector(x[rows, cols])))
    }
  }
  invisible(store)
}

parse_toy_meta <- function(bytes) {
  lines <- strsplit(rawToChar(as.raw(bytes)), "\n", fixed = TRUE)[[1L]]
  pairs <- strsplit(lines[nzchar(lines)], "=", fixed = TRUE)
  values <- setNames(lapply(pairs, `[[`, 2L), vapply(pairs, `[[`, character(1), 1L))
  list(
    dim = as.integer(strsplit(values$dim, ",", fixed = TRUE)[[1L]]),
    chunk_dim = as.integer(strsplit(values$chunk_dim, ",", fixed = TRUE)[[1L]]),
    type = values$type
  )
}

read_toy_array <- function(store) {
  meta <- parse_toy_meta(store_read(store, "zarr.json"))
  out <- matrix(NA_integer_, nrow = meta$dim[[1L]], ncol = meta$dim[[2L]])
  keys <- character()
  positions <- list()
  for (i0 in seq(1L, meta$dim[[1L]], by = meta$chunk_dim[[1L]])) {
    for (j0 in seq(1L, meta$dim[[2L]], by = meta$chunk_dim[[2L]])) {
      keys <- c(keys, chunk_key((i0 - 1L) %/% meta$chunk_dim[[1L]] + 1L, (j0 - 1L) %/% meta$chunk_dim[[2L]] + 1L))
      positions[[length(keys)]] <- list(
        rows = i0:min(i0 + meta$chunk_dim[[1L]] - 1L, meta$dim[[1L]]),
        cols = j0:min(j0 + meta$chunk_dim[[2L]] - 1L, meta$dim[[2L]])
      )
    }
  }
  chunks <- store_read(store, keys, mode = "raw")
  for (i in seq_along(chunks)) {
    pos <- positions[[i]]
    out[pos$rows, pos$cols] <- matrix(decode_i32(chunks[[i]], length(pos$rows) * length(pos$cols)), nrow = length(pos$rows))
  }
  out
}

toy <- matrix(seq_len(16L), nrow = 4L)
store <- byte_store(fs, "array.zarr")
write_toy_array(store, toy)
expect_equal(read_toy_array(store), toy)

cached <- store_cache(store, file.path(root, "cache"), validate = "last_modified_size")
expect_equal(read_toy_array(cached), toy)
store_replace(store, "c/0/0", encode_i32(rep(99L, 4L)))
updated <- toy
updated[1:2, 1:2] <- 99L
expect_equal(read_toy_array(cached), updated)

header <- c("##fileformat=VCFv4.3\n", "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n")
records <- c(
  "chr1\t10\t.\tA\tG\t50\tPASS\t.\n",
  "chr1\t20\t.\tC\tT\t51\tPASS\t.\n",
  "chr2\t15\t.\tG\tA\t52\tPASS\t.\n",
  "chr1\t30\t.\tT\tC\t53\tPASS\t.\n"
)
fs_write(fs, "toy.vcf", charToRaw(paste0(c(header, records), collapse = "")))
record_sizes <- vapply(records, function(x) length(charToRaw(x)), integer(1))
idx <- data.frame(
  chrom = c("chr1", "chr1", "chr2", "chr1"),
  pos = c(10L, 20L, 15L, 30L),
  offset = length(charToRaw(paste0(header, collapse = ""))) + cumsum(c(0L, head(record_sizes, -1L))),
  size = record_sizes,
  id = paste0(c("chr1", "chr1", "chr2", "chr1"), ":", c(10L, 20L, 15L, 30L))
)
query <- idx[idx$chrom == "chr1" & idx$pos >= 20L, ]
req <- byte_ranges("toy.vcf", query$offset, size = query$size, id = query$id)
raw_records <- fs_read(fs, req)
expect_equal(names(raw_records), query$id)
fields <- strsplit(trimws(vapply(raw_records, rawToChar, character(1))), "\t", fixed = TRUE)
expect_equal(unname(vapply(fields, `[[`, character(1), 1L)), c("chr1", "chr1"))
expect_equal(unname(as.integer(vapply(fields, `[[`, character(1), 2L))), c(20L, 30L))

bytes_records <- collect_aio(fs_read_bytes_aio(fs, req))
expect_true(all(vapply(bytes_records, inherits, logical(1), "OpendalBytes")))
expect_equal(vapply(bytes_records, function(x) rawToChar(as.raw(x)), character(1)), vapply(raw_records, rawToChar, character(1)))
