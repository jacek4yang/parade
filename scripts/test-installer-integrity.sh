#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
TEST_DIR=$(mktemp -d)
cleanup() { rm -rf -- "$TEST_DIR"; }
trap cleanup EXIT
mkdir -p "$TEST_DIR/bin" "$TEST_DIR/source"
printf 'manifest contents\n' > "$TEST_DIR/source/SHA256SUMS"
printf 'not a real signature' > "$TEST_DIR/source/SHA256SUMS.sig"
printf 'test public key\n' > "$TEST_DIR/source/release-public.pem"

cat > "$TEST_DIR/bin/curl" <<'CURL'
#!/usr/bin/env bash
set -euo pipefail
url=""
output=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --output) output=$2; shift 2 ;;
    http://*|https://*) url=$1; shift ;;
    *) shift ;;
  esac
done
relative=${url#*/dist/}
[[ $relative != "$url" ]] || relative=${url##*/}
printf '%s\n' "$relative" >> "$FAKE_CURL_LOG"
cp "$FAKE_RELEASE_SOURCE/$relative" "$output"
CURL
chmod 0755 "$TEST_DIR/bin/curl"
cat > "$TEST_DIR/bin/uname" <<'UNAME'
#!/usr/bin/env bash
case "${1:-}" in
  -s) printf 'Linux\n' ;;
  -m) printf '%s\n' "${FAKE_UNAME_M:-x86_64}" ;;
  *) /usr/bin/uname "$@" ;;
esac
UNAME
chmod 0755 "$TEST_DIR/bin/uname"
sed 's#HUB_BASE="{{HUB_BASE}}"#HUB_BASE="http://127.0.0.1:8008"#' \
  hub/assets/install.sh > "$TEST_DIR/install.sh"

KEY_SHA=$(sha256sum "$TEST_DIR/source/release-public.pem" | awk '{print $1}')
set +e
OUTPUT=$(PATH="$TEST_DIR/bin:$PATH" FAKE_RELEASE_SOURCE="$TEST_DIR/source" \
  FAKE_CURL_LOG="$TEST_DIR/curl.log" \
  PARADE_ENROLL_TOKEN=test \
  PARADE_MANIFEST_SHA256=0000000000000000000000000000000000000000000000000000000000000000 \
  PARADE_RELEASE_KEY_SHA256="$KEY_SHA" \
  bash "$TEST_DIR/install.sh" 2>&1)
STATUS=$?
set -e
[[ $STATUS -ne 0 ]] || { echo "installer accepted a bad manifest digest" >&2; exit 1; }
[[ $OUTPUT == *"release manifest verification failed"* ]] \
  || { printf 'unexpected installer output: %s\n' "$OUTPUT" >&2; exit 1; }

mkdir -p "$TEST_DIR/source/armv7-unknown-linux-musleabihf"
cat > "$TEST_DIR/source/armv7-unknown-linux-musleabihf/parade-agent" <<'AGENT'
#!/usr/bin/env bash
printf 'parade-agent 1.0.0\n'
AGENT
chmod 0755 "$TEST_DIR/source/armv7-unknown-linux-musleabihf/parade-agent"
(
  cd "$TEST_DIR/source"
  sha256sum armv7-unknown-linux-musleabihf/parade-agent > SHA256SUMS
  openssl genpkey -algorithm Ed25519 -out release-private.pem >/dev/null 2>&1
  openssl pkey -in release-private.pem -pubout -out release-public.pem
  openssl pkeyutl -sign -inkey release-private.pem -rawin \
    -in SHA256SUMS -out SHA256SUMS.sig
)
MANIFEST_SHA=$(sha256sum "$TEST_DIR/source/SHA256SUMS" | awk '{print $1}')
KEY_SHA=$(sha256sum "$TEST_DIR/source/release-public.pem" | awk '{print $1}')
: > "$TEST_DIR/curl.log"
set +e
OUTPUT=$(PATH="$TEST_DIR/bin:$PATH" FAKE_RELEASE_SOURCE="$TEST_DIR/source" \
  FAKE_CURL_LOG="$TEST_DIR/curl.log" FAKE_UNAME_M=armv7l PARADE_LANG=en \
  PARADE_ENROLL_TOKEN=test PARADE_MANIFEST_SHA256="$MANIFEST_SHA" \
  PARADE_RELEASE_KEY_SHA256="$KEY_SHA" bash "$TEST_DIR/install.sh" 2>&1)
STATUS=$?
set -e
[[ $STATUS -ne 0 ]] || { echo "non-root installer unexpectedly completed" >&2; exit 1; }
[[ $OUTPUT == *"verified parade-agent 1.0.0"* ]] \
  || { printf 'armv7 artifact was not selected and verified: %s\n' "$OUTPUT" >&2; exit 1; }
grep -qx 'armv7-unknown-linux-musleabihf/parade-agent' "$TEST_DIR/curl.log" \
  || { echo "installer did not request the signed armv7 hard-float artifact" >&2; exit 1; }

ENROLL_LINE=$(awk 'index($0,"PARADE_ENROLL_TOKEN=$TOKEN") && index($0,"enroll") { print NR; exit }' hub/assets/install.sh)
STAGE_LINE=$(awk '/install .*parade-agent.*BIN_PATH\.new/ { print NR; exit }' hub/assets/install.sh)
COMMIT_LINE=$(awk '/mv -f -- "\$BIN_PATH\.new" "\$BIN_PATH"/ { print NR; exit }' hub/assets/install.sh)
STAGED_STATE_LINE=$(awk 'index($0,"--staged-state") { print NR; exit }' hub/assets/install.sh)
MAYBE_REDEEMED_LINE=$(awk 'index($0,"TOKEN_MAY_BE_REDEEMED=1") { print NR; exit }' hub/assets/install.sh)
[[ -n $STAGE_LINE && -n $ENROLL_LINE && -n $COMMIT_LINE ]] \
  || { echo "installer staging, enrollment, or commit step is missing" >&2; exit 1; }
[[ $STAGE_LINE -lt $ENROLL_LINE && $ENROLL_LINE -lt $COMMIT_LINE ]] \
  || { echo "installer must stage before enrollment and commit only after enrollment succeeds" >&2; exit 1; }
[[ -n $STAGED_STATE_LINE && $STAGED_STATE_LINE -ge $ENROLL_LINE ]] \
  || { echo "installer must enroll into staged state without overwriting the live identity" >&2; exit 1; }
[[ -n $MAYBE_REDEEMED_LINE && $MAYBE_REDEEMED_LINE -lt $ENROLL_LINE ]] \
  || { echo "installer must forbid false rollback before redemption can commit" >&2; exit 1; }
printf 'installer rejected a mismatched pinned manifest before execution\n'
printf 'installer selected and verified the signed armv7 hard-float artifact\n'
printf 'installer stages first and commits binary and identity only after enrollment succeeds\n'
