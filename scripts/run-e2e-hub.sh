#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
E2E_DIR=$(mktemp -d)
HUB_PID=""
cleanup() {
  if [[ -n $HUB_PID ]] && kill -0 "$HUB_PID" >/dev/null 2>&1; then
    if ! kill "$HUB_PID" >/dev/null 2>&1; then
      printf 'warning: could not stop the temporary e2e Hub process %s\n' "$HUB_PID" >&2
    fi
    if ! wait "$HUB_PID" 2>/dev/null; then
      # A signal exit is expected after the explicit termination above.
      :
    fi
  fi
  rm -rf -- "$E2E_DIR"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
PASSWORD_HASH=$(printf '%s\n' 'correct horse battery staple' | target/debug/parade-hub hash-password)
cat > "$E2E_DIR/hub.toml" <<CONFIG
[hub]
listen = "127.0.0.1:8008"
database_path = "$E2E_DIR/parade.sqlite3"
public_url = "http://127.0.0.1:8008"
dist_dir = "$E2E_DIR/dist"
release_public_key_sha256 = "0000000000000000000000000000000000000000000000000000000000000000"
trusted_proxies = []
secure_cookies = false
session_hours = 1
stale_after_secs = 600
offline_after_secs = 1800

[dashboard]
title = "Parade"
password_hash = "$PASSWORD_HASH"
CONFIG
target/debug/parade-hub "$E2E_DIR/hub.toml" &
HUB_PID=$!
wait "$HUB_PID"
