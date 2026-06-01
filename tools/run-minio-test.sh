#!/usr/bin/env bash
set -euo pipefail

if ! command -v minio >/dev/null 2>&1; then
  echo "minio binary is required for make test-s3-minio" >&2
  exit 127
fi
if ! command -v mc >/dev/null 2>&1; then
  echo "mc binary is required for make test-s3-minio" >&2
  exit 127
fi

pick_port() {
  python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
}

PORT="${ROPENDAL_MINIO_PORT:-$(pick_port)}"
CONSOLE_PORT="${ROPENDAL_MINIO_CONSOLE_PORT:-$(pick_port)}"
DATA_DIR="${ROPENDAL_MINIO_DATA_DIR:-$(mktemp -d -t ropendal-minio-data-XXXXXX)}"
LOG_FILE="${ROPENDAL_MINIO_LOG_FILE:-$(mktemp -t ropendal-minio-log-XXXXXX)}"
ALIAS="ropendal-local-$$"
BUCKET="${ROPENDAL_S3_MINIO_BUCKET:-ropendal-test}"
ACCESS_KEY="${ROPENDAL_S3_MINIO_ACCESS_KEY_ID:-minioadmin}"
SECRET_KEY="${ROPENDAL_S3_MINIO_SECRET_ACCESS_KEY:-minioadmin}"
REGION="${ROPENDAL_S3_MINIO_REGION:-us-east-1}"
ROOT="${ROPENDAL_S3_MINIO_ROOT:-ropendal-$(date +%s)-$$}"
ENDPOINT="http://127.0.0.1:${PORT}"
MINIO_PID=""

cleanup() {
  set +e
  if [ -n "$MINIO_PID" ]; then
    kill "$MINIO_PID" >/dev/null 2>&1 || true
    wait "$MINIO_PID" >/dev/null 2>&1 || true
  fi
  mc alias remove "$ALIAS" >/dev/null 2>&1 || true
  if [ -z "${ROPENDAL_MINIO_DATA_DIR:-}" ]; then
    rm -rf "$DATA_DIR"
  fi
  if [ -z "${ROPENDAL_MINIO_LOG_FILE:-}" ]; then
    rm -f "$LOG_FILE"
  else
    echo "minio log: $LOG_FILE" >&2
  fi
}
trap cleanup EXIT

MINIO_ROOT_USER="$ACCESS_KEY" \
MINIO_ROOT_PASSWORD="$SECRET_KEY" \
minio server "$DATA_DIR" \
  --address "127.0.0.1:${PORT}" \
  --console-address "127.0.0.1:${CONSOLE_PORT}" \
  >"$LOG_FILE" 2>&1 &
MINIO_PID=$!

ready=false
for _ in $(seq 1 60); do
  if mc alias set "$ALIAS" "$ENDPOINT" "$ACCESS_KEY" "$SECRET_KEY" >/dev/null 2>&1; then
    ready=true
    break
  fi
  if ! kill -0 "$MINIO_PID" >/dev/null 2>&1; then
    echo "minio exited before becoming ready" >&2
    tail -n 80 "$LOG_FILE" >&2 || true
    exit 1
  fi
  sleep 0.5
done

if [ "$ready" != true ]; then
  echo "minio did not become ready" >&2
  tail -n 80 "$LOG_FILE" >&2 || true
  exit 1
fi

mc mb --ignore-existing "$ALIAS/$BUCKET" >/dev/null

ROPENDAL_TEST_NETWORK=true \
ROPENDAL_TEST_S3_MINIO=true \
ROPENDAL_S3_MINIO_ENDPOINT="$ENDPOINT" \
ROPENDAL_S3_MINIO_BUCKET="$BUCKET" \
ROPENDAL_S3_MINIO_REGION="$REGION" \
ROPENDAL_S3_MINIO_ACCESS_KEY_ID="$ACCESS_KEY" \
ROPENDAL_S3_MINIO_SECRET_ACCESS_KEY="$SECRET_KEY" \
ROPENDAL_S3_MINIO_ROOT="$ROOT" \
R -e "tinytest::test_package('Ropendal', testdir = 'inst/tinytest')"
