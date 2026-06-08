PKGNAME := $(shell sed -n 's/Package: *\([^ ]*\)/\1/p' DESCRIPTION 2>/dev/null)
PKGVERS := $(shell sed -n 's/Version: *\([^ ]*\)/\1/p' DESCRIPTION 2>/dev/null)
ROPENDAL_GDRIVE_ACCOUNT ?= sounkoutoure@gmail.com
ROPENDAL_GDRIVE3_DIR ?= $(HOME)/.config/gdrive3/$(ROPENDAL_GDRIVE_ACCOUNT)
ROPENDAL_GDRIVE_SECRET_JSON ?= $(ROPENDAL_GDRIVE3_DIR)/secret.json
ROPENDAL_GDRIVE_TOKENS_JSON ?= $(ROPENDAL_GDRIVE3_DIR)/tokens.json
ROPENDAL_GDRIVE_ROOT ?= Ropendal
ROPENDAL_GDRIVE_FILE ?= map_catalog.txt
ROPENDAL_S3_PUBLIC_ENDPOINT ?= https://uk1s3.embassy.ebi.ac.uk
ROPENDAL_S3_PUBLIC_BUCKET ?= idr
ROPENDAL_S3_PUBLIC_ROOT ?= /zarr/v0.4/idr0062A/6001240.zarr
ROPENDAL_S3_PUBLIC_REGION ?= us-east-1
ROPENDAL_S3_PUBLIC_FILE ?= 0/.zarray
ROPENDAL_S3_PUBLIC_RANGE_FILE ?= 0/0/0/0/0
ROPENDAL_S3_PUBLIC_LIST_PATH ?= 0/
ROPENDAL_S3_MINIO_BUCKET ?= ropendal-test
ROPENDAL_S3_MINIO_REGION ?= us-east-1
ROPENDAL_BENCH_BYTES ?= 256MiB
ROPENDAL_BENCH_CHUNK_SIZE ?= 16MiB
ROPENDAL_BENCH_CONCURRENCY ?= 1,2,4,8
ROPENDAL_BENCH_GDRIVE_ROOT ?= KidneyGWAS
ROPENDAL_BENCH_GDRIVE_FILE ?= KidneyPROJECTFILES.zip

all: check

help:
	@printf '%s\n' \
	  'Common development targets:' \
	  '  make rd              regenerate savvy wrappers, roxygen docs, and NAMESPACE' \
	  '  make test-fast       install current source and run non-network tinytest' \
	  '  make test-rust       run Rust unit tests in src/rust' \
	  '  make test-http       run opt-in local HTTP fixture tests' \
	  '  make test-s3         run opt-in public read-only S3-compatible tests' \
	  '  make test-s3-minio   start local MinIO and run writable S3-compatible tests' \
	  '  make test-gdrive     run opt-in Google Drive tests using local gdrive3 JSON defaults' \
	  '  make test-ci         run C API checks and CI-only tinytest' \
	  '  make rdm             render README.md from README.Rmd' \
	  '  make bench-minio-paws render development MinIO benchmark' \
	  '  make bench-gdrive-minio-stress run Google Drive vs MinIO read stress benchmark' \
	  '  make test-webr       build webR/wasm package via rwasm Docker image' \
	  '  make check           build and run R CMD check --as-cran --no-manual'

rd:
	R -e 'if (requireNamespace("savvy", quietly = TRUE) && file.exists("src/rust/Cargo.toml")) savvy::savvy_update(); if (requireNamespace("roxygen2", quietly = TRUE)) roxygen2::roxygenize(load_code = "source") else stop("roxygen2 is required")'

build: install_deps
	R CMD build .

check: build
	R CMD check --as-cran --no-manual $(PKGNAME)_$(PKGVERS).tar.gz

install_deps:
	R \
	-e 'if (!requireNamespace("remotes", quietly = TRUE)) install.packages("remotes")' \
	-e 'remotes::install_deps(dependencies = TRUE)'

install: build
	R CMD INSTALL $(PKGNAME)_$(PKGVERS).tar.gz

install2:
	R CMD INSTALL --no-configure .

install3:
	R CMD INSTALL .

clean:
	@rm -rf $(PKGNAME)_$(PKGVERS).tar.gz $(PKGNAME).Rcheck ..Rcheck
	@rm -rf src/rust/target

dev-install:
	R CMD INSTALL --preclean .

test1:
	R -e "tinytest::test_package('$(PKGNAME)', testdir = 'inst/tinytest', ncpu = 1L)"

test2:
	R -e "tinytest::test_package('$(PKGNAME)', testdir = 'inst/tinytest', ncpu = 2L)"

test0:
	R -e "tinytest::test_package('$(PKGNAME)', testdir = 'inst/tinytest')"

test-fast: dev-install
	ROPENDAL_TEST_NETWORK=false R -e "tinytest::test_package('$(PKGNAME)', testdir = 'inst/tinytest')"

test-rust:
	cargo test --manifest-path=src/rust/Cargo.toml

test-local: test-fast

test-network: dev-install
	ROPENDAL_TEST_NETWORK=true R -e "tinytest::test_package('$(PKGNAME)', testdir = 'inst/tinytest')"

test-http: dev-install
	ROPENDAL_TEST_NETWORK=true ROPENDAL_TEST_HTTP=true R -e "tinytest::test_package('$(PKGNAME)', testdir = 'inst/tinytest')"

test-s3: dev-install
	ROPENDAL_TEST_NETWORK=true ROPENDAL_TEST_S3=true \
	ROPENDAL_S3_PUBLIC_ENDPOINT="$(ROPENDAL_S3_PUBLIC_ENDPOINT)" \
	ROPENDAL_S3_PUBLIC_BUCKET="$(ROPENDAL_S3_PUBLIC_BUCKET)" \
	ROPENDAL_S3_PUBLIC_ROOT="$(ROPENDAL_S3_PUBLIC_ROOT)" \
	ROPENDAL_S3_PUBLIC_REGION="$(ROPENDAL_S3_PUBLIC_REGION)" \
	ROPENDAL_S3_PUBLIC_FILE="$(ROPENDAL_S3_PUBLIC_FILE)" \
	ROPENDAL_S3_PUBLIC_RANGE_FILE="$(ROPENDAL_S3_PUBLIC_RANGE_FILE)" \
	ROPENDAL_S3_PUBLIC_LIST_PATH="$(ROPENDAL_S3_PUBLIC_LIST_PATH)" \
	R -e "tinytest::test_package('$(PKGNAME)', testdir = 'inst/tinytest')"

test-s3-minio: dev-install
	ROPENDAL_S3_MINIO_BUCKET="$(ROPENDAL_S3_MINIO_BUCKET)" \
	ROPENDAL_S3_MINIO_REGION="$(ROPENDAL_S3_MINIO_REGION)" \
	tools/run-minio-test.sh

test-c-api-header:
	Rscript tools/check-c-api-header.R inst/include/ropendal.h

test-c-api-roundtrip:
	Rscript tools/check-c-api-roundtrip.R

test-ci: dev-install test-c-api-header test-c-api-roundtrip
	ROPENDAL_TEST_CI=true R -e "tinytest::test_package('$(PKGNAME)', testdir = 'inst/tinytest')"

ci: test-fast test-ci

test-gdrive: dev-install
	ROPENDAL_TEST_NETWORK=true ROPENDAL_TEST_GDRIVE=true \
	ROPENDAL_GDRIVE_SECRET_JSON="$(ROPENDAL_GDRIVE_SECRET_JSON)" \
	ROPENDAL_GDRIVE_TOKENS_JSON="$(ROPENDAL_GDRIVE_TOKENS_JSON)" \
	ROPENDAL_GDRIVE_ROOT="$(ROPENDAL_GDRIVE_ROOT)" \
	ROPENDAL_GDRIVE_FILE="$(ROPENDAL_GDRIVE_FILE)" \
	R -e "tinytest::test_package('$(PKGNAME)', testdir = 'inst/tinytest')"

test: install
	R -e "tinytest::test_package('$(PKGNAME)', testdir = 'inst/tinytest')"

rdm:
	ROPENDAL_GDRIVE_SECRET_JSON="$(ROPENDAL_GDRIVE_SECRET_JSON)" \
	ROPENDAL_GDRIVE_TOKENS_JSON="$(ROPENDAL_GDRIVE_TOKENS_JSON)" \
	ROPENDAL_GDRIVE_ROOT="$(ROPENDAL_GDRIVE_ROOT)" \
	ROPENDAL_GDRIVE_FILE="$(ROPENDAL_GDRIVE_FILE)" \
	R -e "rmarkdown::render('README.Rmd')"

bench-minio-paws: dev-install
	R -e "rmarkdown::render('benchmarks/minio-paws.Rmd')"

bench-gdrive-minio-stress: dev-install
	ROPENDAL_GDRIVE_SECRET_JSON="$(ROPENDAL_GDRIVE_SECRET_JSON)" \
	ROPENDAL_GDRIVE_TOKENS_JSON="$(ROPENDAL_GDRIVE_TOKENS_JSON)" \
	ROPENDAL_BENCH_BYTES="$(ROPENDAL_BENCH_BYTES)" \
	ROPENDAL_BENCH_CHUNK_SIZE="$(ROPENDAL_BENCH_CHUNK_SIZE)" \
	ROPENDAL_BENCH_CONCURRENCY="$(ROPENDAL_BENCH_CONCURRENCY)" \
	ROPENDAL_BENCH_GDRIVE_ROOT="$(ROPENDAL_BENCH_GDRIVE_ROOT)" \
	ROPENDAL_BENCH_GDRIVE_FILE="$(ROPENDAL_BENCH_GDRIVE_FILE)" \
	Rscript benchmarks/gdrive-minio-read-stress.R

test-webr:
	tools/webr-build.sh

site:
	R -e "pkgdown::build_site()"

.PHONY: all help rd build check install_deps install install2 install3 clean dev-install test1 test2 test0 test-fast test-rust test-local test-network test-http test-s3 test-s3-minio test-c-api-header test-c-api-roundtrip test-ci ci test-gdrive test rdm bench-minio-paws bench-gdrive-minio-stress test-webr site
