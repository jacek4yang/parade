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
ACCOUNT_MARKER=$CFG_DIR/agent-account-owner
LANGUAGE=${PARADE_LANG:-}
TTY_AVAILABLE=0
if [[ -c /dev/tty ]] && { exec 3<>/dev/tty; } 2>/dev/null; then TTY_AVAILABLE=1; fi

choose_language() {
  if [[ $LANGUAGE == zh* ]]; then LANGUAGE=zh-CN; return; fi
  if [[ $LANGUAGE == en* ]]; then LANGUAGE=en; return; fi
  local choice="" default_choice=1
  [[ ${LC_ALL:-${LANG:-}} == zh* ]] && default_choice=2
  if (( TTY_AVAILABLE )); then
    printf '\nParade Agent installer language / Agent 安装语言\n  1) English\n  2) 简体中文\nSelect / 请选择 [%s]: ' "$default_choice" >&3
    IFS= read -r choice <&3 || choice=""
  fi
  [[ ${choice:-$default_choice} == 2 ]] && LANGUAGE=zh-CN || LANGUAGE=en
}

translate() {
  local value=$1
  if [[ $LANGUAGE != zh-CN ]]; then printf '%s' "$value"; return; fi
  case $value in
    'PARADE_ENROLL_TOKEN is required; copy the complete command from Parade') printf '缺少 PARADE_ENROLL_TOKEN；请从 Parade 完整复制注册命令' ;;
    'missing or malformed pinned manifest digest') printf '固定的发布清单摘要缺失或格式错误' ;;
    'missing or malformed pinned release-key digest') printf '固定的发布公钥摘要缺失或格式错误' ;;
    'Hub downloads require HTTPS (except exact loopback development)') printf 'Hub 下载必须使用 HTTPS（仅精确回环开发地址除外）' ;;
    'only Linux is supported') printf '仅支持 Linux VPS' ;;
    'release public-key pin verification failed') printf '发布公钥固定校验失败' ;;
    'release manifest verification failed') printf '发布清单校验失败' ;;
    'offline release signature verification failed') printf '离线发布签名校验失败' ;;
    'Agent artifact verification failed') printf 'Agent 发布制品校验失败' ;;
    'downloaded Agent self-test failed') printf '下载的 Agent 自检失败' ;;
    'downloaded file is not a Parade Agent') printf '下载的文件不是 Parade Agent' ;;
    'artifact verified; run the complete command as root (use sudo)') printf '制品已验证；请以 root 运行完整命令（使用 sudo）' ;;
    'could not stop the existing Agent service safely') printf '无法安全停止现有 Agent 服务' ;;
    'systemd is required for the supported one-command install') printf '受支持的一行安装需要 systemd' ;;
    'installed Agent cannot read its configuration or state') printf '安装后的 Agent 无法读取配置或状态' ;;
    'staged Agent configuration or identity is invalid') printf '暂存的 Agent 配置或身份无效' ;;
    'an unrelated parade account or group already exists') printf '已存在无法验证归属的 parade 用户或组；为避免权限泄露，安装已拒绝' ;;
    'Agent service failed to start') printf 'Agent 服务启动失败' ;;
    'downloading and verifying the pinned release manifest') printf '正在下载并验证固定的发布清单' ;;
    'redeeming the short-lived, single-use enrollment token') printf '正在兑换短期、单次使用的注册令牌' ;;
    'journal output was unavailable') printf '无法读取 journal 日志' ;;
    required\ command\ not\ found:*) printf '缺少必需命令：%s' "${value#*: }" ;;
    unsupported\ architecture:*) printf '不支持的架构：%s' "${value#*: }" ;;
    no\ signed\ release\ artifact*) printf '没有为当前架构发布已签名制品：%s' "${value##* }" ;;
    verified\ *) printf '已验证 %s' "${value#verified }" ;;
    *' installed; the unprivileged outbound-only service is active') printf '%s 已安装；非特权、仅出站服务已运行' "${value% installed*}" ;;
    *) printf '%s' "$value" ;;
  esac
}

die() { printf 'parade-agent installer: %s\n' "$(translate "$*")" >&2; exit 1; }
say() { printf 'parade-agent installer: %s\n' "$(translate "$*")"; }

choose_language
[[ -n $TOKEN ]] || die "PARADE_ENROLL_TOKEN is required; copy the complete command from Parade"
[[ $MANIFEST_SHA256 =~ ^[0-9a-f]{64}$ ]] || die "missing or malformed pinned manifest digest"
[[ $RELEASE_KEY_SHA256 =~ ^[0-9a-f]{64}$ ]] || die "missing or malformed pinned release-key digest"
if [[ $HUB_BASE != https://* && ! $HUB_BASE =~ ^http://(localhost|127\.0\.0\.1|\[::1\])(:[0-9]+)?$ ]]; then
  die "Hub downloads require HTTPS (except exact loopback development)"
fi
for command in curl sha256sum awk install uname openssl find mktemp; do
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

TMP_DIR=$(mktemp -d /tmp/parade-agent-install.XXXXXX)
case $TMP_DIR in /tmp/parade-agent-install.*) ;; *) die "unsafe temporary directory" ;; esac
WAS_ACTIVE=0
INSTALL_SUCCEEDED=0
TOKEN_MAY_BE_REDEEMED=0
cleanup() {
  status=$?
  if [[ -d $TMP_DIR ]]; then find "$TMP_DIR" -depth -delete; fi
  if (( status != 0 && INSTALL_SUCCEEDED == 0 && WAS_ACTIVE == 1 && TOKEN_MAY_BE_REDEEMED == 0 )); then
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
"${CURL[@]}" --max-filesize 1048576 "$HUB_BASE/dist/SHA256SUMS" --output "$TMP_DIR/SHA256SUMS"
"${CURL[@]}" --max-filesize 1024 "$HUB_BASE/dist/SHA256SUMS.sig" --output "$TMP_DIR/SHA256SUMS.sig"
"${CURL[@]}" --max-filesize 65536 "$HUB_BASE/dist/release-public.pem" --output "$TMP_DIR/release-public.pem"
printf '%s  %s\n' "$RELEASE_KEY_SHA256" "$TMP_DIR/release-public.pem" | sha256sum --check --status - \
  || die "release public-key pin verification failed"
printf '%s  %s\n' "$MANIFEST_SHA256" "$TMP_DIR/SHA256SUMS" | sha256sum --check --status - \
  || die "release manifest verification failed"
openssl pkeyutl -verify -pubin -inkey "$TMP_DIR/release-public.pem" -rawin \
  -in "$TMP_DIR/SHA256SUMS" -sigfile "$TMP_DIR/SHA256SUMS.sig" >/dev/null \
  || die "offline release signature verification failed"

RELATIVE_PATH=""
case "$ARCH" in
  armv7) TARGET_CANDIDATES=(armv7-unknown-linux-musleabihf armv7-unknown-linux-gnueabihf) ;;
  *) TARGET_CANDIDATES=("${ARCH}-unknown-linux-musl" "${ARCH}-unknown-linux-gnu") ;;
esac
for triple in "${TARGET_CANDIDATES[@]}"; do
  candidate="$triple/parade-agent"
  if awk -v path="$candidate" '$2 == path && $1 ~ /^[0-9a-f]{64}$/ { found=1 } END { exit !found }' "$TMP_DIR/SHA256SUMS"; then
    RELATIVE_PATH=$candidate
    break
  fi
done
[[ -n $RELATIVE_PATH ]] || die "no signed release artifact is published for $(uname -m)"
AGENT_SHA256=$(awk -v path="$RELATIVE_PATH" '$2 == path { print $1; exit }' "$TMP_DIR/SHA256SUMS")
"${CURL[@]}" --max-filesize 67108864 "$HUB_BASE/dist/$RELATIVE_PATH" --output "$TMP_DIR/parade-agent"
printf '%s  %s\n' "$AGENT_SHA256" "$TMP_DIR/parade-agent" | sha256sum --check --status - \
  || die "Agent artifact verification failed"
chmod 0755 "$TMP_DIR/parade-agent"
VERSION=$("$TMP_DIR/parade-agent" --version) || die "downloaded Agent self-test failed"
[[ $VERSION == parade-agent\ * ]] || die "downloaded file is not a Parade Agent"
say "verified $VERSION"

[[ $EUID -eq 0 ]] || die "artifact verified; run the complete command as root (use sudo)"
for command in getent groupadd useradd runuser systemctl; do
  command -v "$command" >/dev/null 2>&1 || die "required command not found: $command"
done

CREATED_GROUP=0
CREATED_USER=0
if [[ -f $ACCOUNT_MARKER ]]; then
  grep -qx 'group=installer' "$ACCOUNT_MARKER" && CREATED_GROUP=1
  grep -qx 'user=installer' "$ACCOUNT_MARKER" && CREATED_USER=1
fi
if { getent group parade >/dev/null 2>&1 && (( CREATED_GROUP == 0 )); } \
  || { id parade >/dev/null 2>&1 && (( CREATED_USER == 0 )); }; then
  die "an unrelated parade account or group already exists"
fi
if ! getent group parade >/dev/null 2>&1; then
  groupadd --system parade
  CREATED_GROUP=1
fi
if ! id parade >/dev/null 2>&1; then
  useradd --system --gid parade --home-dir "$STATE_DIR" --shell /usr/sbin/nologin parade
  CREATED_USER=1
fi
install -d -o root -g root -m 0755 "$CFG_DIR"
install -d -o parade -g parade -m 0700 "$STATE_DIR"
{
  (( CREATED_USER )) && printf 'user=installer\n' || printf 'user=preexisting\n'
  (( CREATED_GROUP )) && printf 'group=installer\n' || printf 'group=preexisting\n'
} >"$TMP_DIR/account-owner"
install -o root -g root -m 0600 "$TMP_DIR/account-owner" "$ACCOUNT_MARKER"

if [[ ! -d /run/systemd/system ]]; then
  die "systemd is required for the supported one-command install"
fi
install -o root -g root -m 0644 /dev/stdin "$TMP_DIR/parade-agent.service" <<'UNIT'
[Unit]
Description=Parade read-only observability Agent
After=network-online.target
Wants=network-online.target
StartLimitIntervalSec=5min
StartLimitBurst=10

[Service]
Type=simple
User=parade
Group=parade
ExecStart=/usr/local/bin/parade-agent /etc/parade/agent.toml
Restart=on-failure
RestartSec=10s
MemoryHigh=96M
MemoryMax=128M
TasksMax=64
LimitNOFILE=1024
LogRateLimitIntervalSec=30s
LogRateLimitBurst=200
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
# Stage every static artifact before consuming the single-use token. The old
# service remains untouched until all downloads, checks and destination writes
# that can be prepared safely have succeeded.
install -o root -g root -m 0755 "$TMP_DIR/parade-agent" "$BIN_PATH.new"
install -o root -g root -m 0644 "$TMP_DIR/parade-agent.service" "$SERVICE_PATH.new"

if command -v systemctl >/dev/null 2>&1 && [[ -d /run/systemd/system ]]; then
  if systemctl is-active --quiet parade-agent.service; then
    WAS_ACTIVE=1
    systemctl stop parade-agent.service >/dev/null \
      || die "could not stop the existing Agent service safely"
  fi
fi
say "redeeming the short-lived, single-use enrollment token"
# The Hub may commit redemption even if the response is lost. From this point
# onward restarting the old, now possibly revoked credential would be a false
# rollback, so failure deliberately leaves the old service stopped for an
# operator-visible retry with a fresh enrollment token.
TOKEN_MAY_BE_REDEEMED=1
PARADE_ENROLL_TOKEN=$TOKEN "$TMP_DIR/parade-agent" enroll \
  --hub "$HUB_BASE" --config "$TMP_DIR/agent.toml" --state "$STATE_PATH" \
  --staged-state "$TMP_DIR/state.json"
"$TMP_DIR/parade-agent" check-config "$TMP_DIR/agent.toml" "$TMP_DIR/state.json" >/dev/null \
  || die "staged Agent configuration or identity is invalid"
install -o root -g parade -m 0640 "$TMP_DIR/agent.toml" "$CFG_PATH.new"
install -o parade -g parade -m 0600 "$TMP_DIR/state.json" "$STATE_PATH.new"
mv -f -- "$STATE_PATH.new" "$STATE_PATH"
mv -f -- "$CFG_PATH.new" "$CFG_PATH"
mv -f -- "$BIN_PATH.new" "$BIN_PATH"
mv -f -- "$SERVICE_PATH.new" "$SERVICE_PATH"

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
