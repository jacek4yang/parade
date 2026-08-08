#!/usr/bin/env bash
# Deterministic, Linux-only resource and bandwidth regression gate.
#
# This is adapted from the measurement discipline in the read-only reference
# workspace at ~/work/kimi-rust-reality-performance: record the exact binary,
# machine and Git identity; preserve raw samples; reject regressions with fixed
# ceilings. It never runs against or controls a monitored VPS.
set -euo pipefail

cd "$(dirname "$0")/.."

REPORT_PATH=${PARADE_PERF_REPORT:-"/tmp/parade-performance-${$}.txt"}
SKIP_BUILD=${PARADE_PERF_SKIP_BUILD:-0}
AGENT_SIZE_LIMIT=$((5 * 1024 * 1024))
HUB_SIZE_LIMIT=$((16 * 1024 * 1024))
HUB_IDLE_RSS_LIMIT_KIB=$((128 * 1024))

TMP_DIR=$(mktemp -d)
HUB_PID=""
cleanup() {
  if [[ -n $HUB_PID ]] && kill -0 "$HUB_PID" 2>/dev/null; then
    kill "$HUB_PID"
    wait "$HUB_PID" 2>/dev/null || :
  fi
  rm -rf -- "$TMP_DIR"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

exec > >(tee "$REPORT_PATH") 2>&1

printf 'PARADE_PERFORMANCE_V1\n'
printf 'timestamp_utc=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
printf 'git_commit=%s\n' "$(git rev-parse HEAD)"
printf 'git_dirty_files=%s\n' "$(git status --porcelain=v1 | wc -l)"
printf 'kernel=%s\n' "$(uname -srmo)"
printf 'architecture=%s\n' "$(uname -m)"
printf 'rustc=%s\n' "$(rustc --version)"
printf 'cargo=%s\n' "$(cargo --version)"
if [[ -r /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor ]]; then
  printf 'cpu_governor=%s\n' "$(</sys/devices/system/cpu/cpu0/cpufreq/scaling_governor)"
fi
if [[ -r /proc/sys/kernel/perf_event_paranoid ]]; then
  printf 'perf_event_paranoid=%s\n' "$(</proc/sys/kernel/perf_event_paranoid)"
fi

if [[ $SKIP_BUILD != 1 ]]; then
  cargo build --release --workspace --all-features --locked
fi
for binary in target/release/parade-agent target/release/parade-hub; do
  [[ -x $binary ]] || {
    printf 'missing release binary: %s\n' "$binary" >&2
    exit 1
  }
done

AGENT_SIZE=$(stat --format='%s' target/release/parade-agent)
HUB_SIZE=$(stat --format='%s' target/release/parade-hub)
printf 'agent_binary_bytes=%s\n' "$AGENT_SIZE"
printf 'hub_binary_bytes=%s\n' "$HUB_SIZE"
printf 'agent_binary_sha256=%s\n' "$(sha256sum target/release/parade-agent | awk '{print $1}')"
printf 'hub_binary_sha256=%s\n' "$(sha256sum target/release/parade-hub | awk '{print $1}')"
((AGENT_SIZE <= AGENT_SIZE_LIMIT)) || {
  printf 'Agent binary exceeds %s-byte ceiling\n' "$AGENT_SIZE_LIMIT" >&2
  exit 1
}
((HUB_SIZE <= HUB_SIZE_LIMIT)) || {
  printf 'Hub binary exceeds %s-byte ceiling\n' "$HUB_SIZE_LIMIT" >&2
  exit 1
}

cargo tree -p parade-hub -e normal >"$TMP_DIR/hub-tree.txt"
cargo tree -p parade-agent -e normal >"$TMP_DIR/agent-tree.txt"
if grep -Eq '(tokio-tungstenite|tungstenite|flate2|miniz_oxide) v' \
  "$TMP_DIR/hub-tree.txt" "$TMP_DIR/agent-tree.txt"; then
  printf 'unused WebSocket or gzip dependency re-entered the runtime graph\n' >&2
  exit 1
fi
printf 'dependency_feature_gate=passed\n'

cargo test -p parade-agent --all-features \
  tests::default_monthly_bandwidth_is_below_target -- --nocapture
FLEET_TEST_LOG="$TMP_DIR/fleet-load-test.txt"
PARADE_FLEET_LOAD_WALL_CLOCK_GATE=1 \
  cargo test -p parade-hub --all-features --bin parade-hub \
  db::tests::synthetic_fleet_load_1000_agents_is_bounded -- \
  --exact --nocapture 2>&1 | tee "$FLEET_TEST_LOG"
if ! grep -Fq 'PARADE_FLEET_LOAD_WALL_CLOCK_GATE=passed' "$FLEET_TEST_LOG"; then
  printf '1,000-Agent wall-clock assertion did not execute\n' >&2
  exit 1
fi

PASSWORD_HASH=$(printf '%s\n' 'temporary performance password' \
  | target/release/parade-hub hash-password)
mkdir -p "$TMP_DIR/dist"
cat >"$TMP_DIR/hub.toml" <<CONFIG
[hub]
listen = "127.0.0.1:0"
database_path = "$TMP_DIR/parade.sqlite3"
public_url = "http://127.0.0.1:8008"
dist_dir = "$TMP_DIR/dist"
release_public_key_sha256 = "0000000000000000000000000000000000000000000000000000000000000000"
trusted_proxies = []
secure_cookies = false
session_hours = 1
stale_after_secs = 600
offline_after_secs = 1800

[dashboard]
title = "Parade performance gate"
password_hash = "$PASSWORD_HASH"
CONFIG
target/release/parade-hub "$TMP_DIR/hub.toml" >"$TMP_DIR/hub.log" 2>&1 &
HUB_PID=$!

MAX_RSS_KIB=0
MAX_FDS=0
MAX_THREADS=0
for _sample in 1 2 3 4 5; do
  sleep 1
  kill -0 "$HUB_PID" 2>/dev/null || {
    printf 'temporary Hub exited during idle measurement\n' >&2
    sed -n '1,80p' "$TMP_DIR/hub.log" >&2
    exit 1
  }
  RSS_KIB=$(awk '/^VmRSS:/ {print $2}' "/proc/$HUB_PID/status")
  THREADS=$(awk '/^Threads:/ {print $2}' "/proc/$HUB_PID/status")
  FDS=$(find "/proc/$HUB_PID/fd" -mindepth 1 -maxdepth 1 -printf '.' | wc -c)
  ((RSS_KIB > MAX_RSS_KIB)) && MAX_RSS_KIB=$RSS_KIB
  ((FDS > MAX_FDS)) && MAX_FDS=$FDS
  ((THREADS > MAX_THREADS)) && MAX_THREADS=$THREADS
done
DB_BYTES=$(stat --format='%s' "$TMP_DIR/parade.sqlite3")
printf 'hub_idle_peak_rss_kib=%s\n' "$MAX_RSS_KIB"
printf 'hub_idle_peak_fds=%s\n' "$MAX_FDS"
printf 'hub_idle_peak_threads=%s\n' "$MAX_THREADS"
printf 'hub_idle_database_bytes=%s\n' "$DB_BYTES"
((MAX_RSS_KIB <= HUB_IDLE_RSS_LIMIT_KIB)) || {
  printf 'idle Hub RSS exceeds %s-KiB ceiling\n' "$HUB_IDLE_RSS_LIMIT_KIB" >&2
  exit 1
}

printf 'performance_gate=passed\n'
printf 'report_path=%s\n' "$REPORT_PATH"
