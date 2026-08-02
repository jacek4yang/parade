#!/usr/bin/env bash
# Parade Agent installer. The authenticated Hub UI emits the enrollment token
# and a pinned SHA-256 digest for the release manifest as environment values.
set -euo pipefail

HUB_BASE="{{HUB_BASE}}"
TOKEN="${PARADE_ENROLL_TOKEN:-}"
MANIFEST_SHA256="${PARADE_MANIFEST_SHA256:-}"
RELEASE_KEY_SHA256="${PARADE_RELEASE_KEY_SHA256:-}"
BIN_PATH=/usr/local/bin/parade-agent
CFG_DIR=/etc/parade
CFG_PATH=$CFG_DIR/agent.toml
STATE_DIR=/var/lib/parade-agent
STATE_PATH=$STATE_DIR/state.json
SERVICE_PATH=/etc/systemd/system/parade-agent.service

die() { printf 'parade-agent installer: %s\n' "$*" >&2; exit 1; }
say() { printf 'parade-agent installer: %s\n' "$*"; }

[[ -n $TOKEN ]] || die "PARADE_ENROLL_TOKEN is required; copy the complete command from Parade"
[[ $MANIFEST_SHA256 =~ ^[0-9a-f]{64}$ ]] || die "missing or malformed pinned manifest digest"
[[ $RELEASE_KEY_SHA256 =~ ^[0-9a-f]{64}$ ]] || die "missing or malformed pinned release-key digest"
if [[ $HUB_BASE != https://* && ! $HUB_BASE =~ ^http://(localhost|127\.0\.0\.1|\[::1\])(:[0-9]+)?$ ]]; then
  die "Hub downloads require HTTPS (except exact loopback development)"
fi
for command in curl sha256sum awk install uname openssl; do
  command -v "$command" >/dev/null 2>&1 || die "required command not found: $command"
done

case "$(uname -s)" in Linux) ;; *) die "only Linux is supported" ;; esac
case "$(uname -m)" in
  x86_64|amd64) ARCH=x86_64 ;;
  aarch64|arm64) ARCH=aarch64 ;;
  armv7l|armv7|armhf) ARCH=armv7 ;;
  riscv64) ARCH=riscv64gc ;;
  *) die "unsupported architecture: $(uname -m)" ;;
esac

TMP_DIR=$(mktemp -d)
WAS_ACTIVE=0
INSTALL_SUCCEEDED=0
cleanup() {
  status=$?
  rm -rf -- "$TMP_DIR"
  if (( status != 0 && INSTALL_SUCCEEDED == 0 && WAS_ACTIVE == 1 )); then
    if ! systemctl start parade-agent.service >/dev/null 2>&1; then
      printf 'parade-agent installer: warning: failed to restart the previous Agent service\n' >&2
    fi
  fi
  return "$status"
}
trap cleanup EXIT
CURL=(curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 --connect-timeout 10 --max-time 120 --retry 3)
if [[ $HUB_BASE == http://* ]]; then CURL=(curl --fail --silent --show-error --location --connect-timeout 10 --max-time 120 --retry 3); fi

say "downloading and verifying the pinned release manifest"
"${CURL[@]}" "$HUB_BASE/dist/SHA256SUMS" --output "$TMP_DIR/SHA256SUMS"
"${CURL[@]}" "$HUB_BASE/dist/SHA256SUMS.sig" --output "$TMP_DIR/SHA256SUMS.sig"
"${CURL[@]}" "$HUB_BASE/dist/release-public.pem" --output "$TMP_DIR/release-public.pem"
printf '%s  %s\n' "$RELEASE_KEY_SHA256" "$TMP_DIR/release-public.pem" | sha256sum --check --status - \
  || die "release public-key pin verification failed"
printf '%s  %s\n' "$MANIFEST_SHA256" "$TMP_DIR/SHA256SUMS" | sha256sum --check --status - \
  || die "release manifest verification failed"
openssl pkeyutl -verify -pubin -inkey "$TMP_DIR/release-public.pem" -rawin \
  -in "$TMP_DIR/SHA256SUMS" -sigfile "$TMP_DIR/SHA256SUMS.sig" >/dev/null \
  || die "offline release signature verification failed"

RELATIVE_PATH=""
for triple in "${ARCH}-unknown-linux-musl" "${ARCH}-unknown-linux-gnu"; do
  candidate="$triple/parade-agent"
  if awk -v path="$candidate" '$2 == path && $1 ~ /^[0-9a-f]{64}$/ { found=1 } END { exit !found }' "$TMP_DIR/SHA256SUMS"; then
    RELATIVE_PATH=$candidate
    break
  fi
done
[[ -n $RELATIVE_PATH ]] || die "no signed release artifact is published for $(uname -m)"
AGENT_SHA256=$(awk -v path="$RELATIVE_PATH" '$2 == path { print $1; exit }' "$TMP_DIR/SHA256SUMS")
"${CURL[@]}" "$HUB_BASE/dist/$RELATIVE_PATH" --output "$TMP_DIR/parade-agent"
printf '%s  %s\n' "$AGENT_SHA256" "$TMP_DIR/parade-agent" | sha256sum --check --status - \
  || die "Agent artifact verification failed"
chmod 0755 "$TMP_DIR/parade-agent"
VERSION=$("$TMP_DIR/parade-agent" --version) || die "downloaded Agent self-test failed"
[[ $VERSION == parade-agent\ * ]] || die "downloaded file is not a Parade Agent"
say "verified $VERSION"

[[ $EUID -eq 0 ]] || die "artifact verified; run the complete command as root (use sudo)"

if ! getent group parade >/dev/null 2>&1; then groupadd --system parade; fi
if ! id parade >/dev/null 2>&1; then
  useradd --system --gid parade --home-dir "$STATE_DIR" --shell /usr/sbin/nologin parade
fi
install -d -o root -g parade -m 0750 "$CFG_DIR"
install -d -o parade -g parade -m 0700 "$STATE_DIR"

if command -v systemctl >/dev/null 2>&1 && [[ -d /run/systemd/system ]]; then
  if systemctl is-active --quiet parade-agent.service; then
    WAS_ACTIVE=1
    systemctl stop parade-agent.service >/dev/null \
      || die "could not stop the existing Agent service safely"
  fi
fi
say "redeeming the short-lived, single-use enrollment token"
PARADE_ENROLL_TOKEN=$TOKEN "$TMP_DIR/parade-agent" enroll \
  --hub "$HUB_BASE" --config "$TMP_DIR/agent.toml" --state "$STATE_PATH"
install -o root -g root -m 0755 "$TMP_DIR/parade-agent" "$BIN_PATH.new"
mv -f -- "$BIN_PATH.new" "$BIN_PATH"
install -o root -g parade -m 0640 "$TMP_DIR/agent.toml" "$CFG_PATH.new"
mv -f -- "$CFG_PATH.new" "$CFG_PATH"
chown parade:parade "$STATE_PATH"
chmod 0600 "$STATE_PATH"

if ! command -v systemctl >/dev/null 2>&1 || [[ ! -d /run/systemd/system ]]; then
  die "systemd is required for the supported one-command install"
fi
install -o root -g root -m 0644 /dev/stdin "$SERVICE_PATH" <<'UNIT'
[Unit]
Description=Parade read-only observability Agent
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=parade
Group=parade
ExecStart=/usr/local/bin/parade-agent /etc/parade/agent.toml
Restart=on-failure
RestartSec=10s
UMask=0077
NoNewPrivileges=yes
CapabilityBoundingSet=
AmbientCapabilities=
PrivateTmp=yes
PrivateDevices=yes
ProtectSystem=strict
ProtectHome=yes
ProtectKernelTunables=yes
ProtectKernelModules=yes
ProtectKernelLogs=yes
ProtectControlGroups=yes
RestrictNamespaces=yes
RestrictRealtime=yes
LockPersonality=yes
MemoryDenyWriteExecute=yes
RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6
SystemCallArchitectures=native
ReadWritePaths=/var/lib/parade-agent

[Install]
WantedBy=multi-user.target
UNIT

runuser -u parade -- "$BIN_PATH" check-config "$CFG_PATH" >/dev/null \
  || die "installed Agent cannot read its configuration or state"
systemctl daemon-reload
systemctl enable --now parade-agent.service >/dev/null
if ! systemctl is-active --quiet parade-agent.service; then
  if ! journalctl --no-pager -u parade-agent.service -n 30 >&2; then
    say "journal output was unavailable"
  fi
  die "Agent service failed to start"
fi
INSTALL_SUCCEEDED=1
say "$VERSION installed; the unprivileged outbound-only service is active"
