#!/usr/bin/env bash
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
WEBR_IMAGE=${WEBR_IMAGE:-ghcr.io/r-wasm/webr:main}

# Mirror the R-universe wasm build path closely enough for local debugging:
# build the source tarball, install rwasm inside the webR image, and ask rwasm
# to produce the package .tgz. Ropendal currently compiles a wasm load shim that
# returns explicit unsupported errors for OpenDAL-backed operations.
docker run --rm \
  -e RUSTUP_HOME=/tmp/rustup \
  -e CARGO_HOME=/tmp/cargo \
  -e RUSTUP_INIT_SKIP_PATH_CHECK=yes \
  -v "${ROOT_DIR}:/work/Ropendal" \
  -w /work/Ropendal \
  "${WEBR_IMAGE}" bash -lc '
set -eu
export PATH="/tmp/cargo/bin:$PATH"
rm -rf Ropendal.Rcheck ..Rcheck README.html
R CMD build --no-build-vignettes .
Rscript -e "if (!requireNamespace(\"pak\", quietly = TRUE)) install.packages(\"pak\", repos = \"https://repo.r-wasm.org/\")"
Rscript -e "pak::pak(\"r-wasm/rwasm\", ask = FALSE)"
Rscript -e "version <- read.dcf(\"DESCRIPTION\")[1, \"Version\"]; rwasm::build(sprintf(\"./Ropendal_%s.tar.gz\", version))"
'
