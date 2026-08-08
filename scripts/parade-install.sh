#!/usr/bin/env bash
# Installs a signed Parade Hub release on a Linux VPS. Agent installation uses
# the unique one-line enrollment command emitted by the authenticated Hub.
set -euo pipefail

REPOSITORY=${PARADE_GITHUB_REPOSITORY:-jacek4yang/parade}
VERSION=${PARADE_VERSION:-latest}
COMPONENT=${1:-hub}
LANGUAGE=${PARADE_LANG:-}
TMP_DIR=""
TTY_AVAILABLE=0
if [[ -c /dev/tty ]] && { exec 3<>/dev/tty; } 2>/dev/null; then TTY_AVAILABLE=1; fi

choose_language() {
  if [[ $LANGUAGE == zh || $LANGUAGE == zh-CN || $LANGUAGE == zh_* ]]; then
    LANGUAGE=zh-CN
    return
  fi
  if [[ $LANGUAGE == en || $LANGUAGE == en_* ]]; then
    LANGUAGE=en
    return
  fi
  local default_choice=1 choice=""
  [[ ${LC_ALL:-${LANG:-}} == zh* ]] && default_choice=2
  if (( TTY_AVAILABLE )); then
    printf '\nParade installer language / 安装程序语言\n  1) English\n  2) 简体中文\nSelect / 请选择 [%s]: ' "$default_choice" >&3
    IFS= read -r choice <&3 || choice=""
  fi
  [[ ${choice:-$default_choice} == 2 ]] && LANGUAGE=zh-CN || LANGUAGE=en
}

msg() {
  local key=$1
  shift || true
  if [[ $LANGUAGE == zh-CN ]]; then
    case $key in
      linux_only) printf '仅支持 Linux VPS。' ;;
      root) printf '请以 root 身份运行（通常使用 sudo）。' ;;
      component) printf '公开的一行安装程序只安装 Hub；Agent 必须使用 Hub 中每台服务器唯一的注册命令。' ;;
      missing) printf '缺少必需命令：%s' "$1" ;;
      systemd) printf '当前支持的一行安装流程需要 systemd。' ;;
      arch) printf 'Hub 暂不支持此架构：%s' "$1" ;;
      download) printf '正在下载 %s 的签名发布制品…' "$1" ;;
      fingerprint) printf '发布公钥 SHA-256：%s' "$1" ;;
      trust) printf '这是首次信任。是否信任此发布公钥？输入 yes 继续：' ;;
      pin_required) printf '非交互安装必须设置 PARADE_RELEASE_KEY_SHA256 为受信任的 64 位十六进制公钥摘要。' ;;
      key_failed) printf '发布公钥摘要校验失败。' ;;
      signature_failed) printf '发布清单的 Ed25519 签名校验失败。' ;;
      artifact_failed) printf '发布制品校验失败：%s' "$1" ;;
      public_url) printf 'Hub 公网地址（生产环境使用 HTTPS；仅本机测试可用 http://127.0.0.1:8008）' ;;
      password) printf '管理员密码（至少 12 个字符）' ;;
      password_again) printf '再次输入管理员密码' ;;
      password_mismatch) printf '两次输入的密码不一致。' ;;
      password_short) printf '管理员密码至少需要 12 个字符。' ;;
      config_failed) printf 'Hub 地址或生成的配置无效。' ;;
      existing) printf '检测到现有 Hub 或遗留安装文件；为保护数据库和配置，安装程序拒绝覆盖。请按照运维文档先备份并执行升级流程。' ;;
      installing) printf '正在安装非特权 Hub 服务和只读 Agent 发布目录…' ;;
      active) printf 'Parade Hub %s 已安装且服务正常。' "$1" ;;
      next) printf '下一步：为该地址配置 HTTPS 反向代理，登录 Hub，在“设置”中为每台 VPS 创建独立服务器记录，并分别执行其一次性注册命令。' ;;
      failed) printf '安装失败：%s' "$1" ;;
    esac
  else
    case $key in
      linux_only) printf 'Only Linux VPS hosts are supported.' ;;
      root) printf 'Run as root (normally with sudo).' ;;
      component) printf 'The public one-line installer installs the Hub only; Agents require the unique per-server enrollment command from the Hub.' ;;
      missing) printf 'Required command is missing: %s' "$1" ;;
      systemd) printf 'systemd is required for the supported one-line installation.' ;;
      arch) printf 'This Hub architecture is not published: %s' "$1" ;;
      download) printf 'Downloading signed release artifacts for %s…' "$1" ;;
      fingerprint) printf 'Release public-key SHA-256: %s' "$1" ;;
      trust) printf 'This is trust on first use. Trust this release key? Type yes to continue: ' ;;
      pin_required) printf 'Non-interactive installation requires PARADE_RELEASE_KEY_SHA256 with the trusted 64-character hexadecimal key digest.' ;;
      key_failed) printf 'Release public-key digest verification failed.' ;;
      signature_failed) printf 'The Ed25519 release-manifest signature is invalid.' ;;
      artifact_failed) printf 'Release artifact verification failed: %s' "$1" ;;
      public_url) printf 'Hub public origin (HTTPS in production; http://127.0.0.1:8008 for local-only evaluation)' ;;
      password) printf 'Administrator password (at least 12 characters)' ;;
      password_again) printf 'Administrator password again' ;;
      password_mismatch) printf 'The passwords do not match.' ;;
      password_short) printf 'The administrator password must be at least 12 characters.' ;;
      config_failed) printf 'The Hub origin or generated configuration is invalid.' ;;
      existing) printf 'An existing Hub or partial installation was detected. To protect its database and configuration, this installer refuses to overwrite it; back up and use the documented upgrade procedure.' ;;
      installing) printf 'Installing the unprivileged Hub and read-only Agent release tree…' ;;
      active) printf 'Parade Hub %s is installed and active.' "$1" ;;
      next) printf 'Next: configure an HTTPS reverse proxy for this origin, sign in, create one independent server record per VPS in Settings, and run each unique one-use enrollment command on its matching VPS.' ;;
      failed) printf 'Installation failed: %s' "$1" ;;
    esac
  fi
}

die() {
  msg failed "$(msg "$@")" >&2
  printf '\n' >&2
  exit 1
}

say() {
  msg "$@"
  printf '\n'
}

prompt() {
  local label=$1 default_value=$2 value=""
  if (( ! TTY_AVAILABLE )); then
    printf '%s' "$default_value"
    return
  fi
  printf '%s [%s]: ' "$label" "$default_value" >&3
  IFS= read -r value <&3 || value=""
  printf '%s' "${value:-$default_value}"
}

choose_language
[[ $(uname -s) == Linux ]] || die linux_only
[[ $EUID -eq 0 ]] || die root
[[ $COMPONENT == hub ]] || die component
for required in curl sha256sum awk install uname openssl tar getent groupadd useradd id systemctl find grep cat mktemp chown chmod; do
  command -v "$required" >/dev/null 2>&1 || die missing "$required"
done
[[ -d /run/systemd/system ]] || die systemd
if systemctl is-active --quiet parade-hub.service 2>/dev/null \
  || [[ -e /usr/local/bin/parade-hub || -e /etc/parade/hub.toml \
    || -e /etc/systemd/system/parade-hub.service || -e /var/lib/parade \
    || -e /var/lib/parade-dist ]]; then
  die existing
fi
if id parade-hub >/dev/null 2>&1 || getent group parade-hub >/dev/null 2>&1; then
  die existing
fi

case $(uname -m) in
  x86_64|amd64) TARGET=x86_64-unknown-linux-musl ;;
  aarch64|arm64) TARGET=aarch64-unknown-linux-musl ;;
  *) die arch "$(uname -m)" ;;
esac

if [[ $VERSION == latest ]]; then
  RELEASE_BASE="https://github.com/${REPOSITORY}/releases/latest/download"
else
  RELEASE_BASE="https://github.com/${REPOSITORY}/releases/download/${VERSION}"
fi
TMP_DIR=$(mktemp -d /tmp/parade-hub-install.XXXXXX)
case $TMP_DIR in /tmp/parade-hub-install.*) ;; *) die artifact_failed temp-directory ;; esac
cleanup() {
  local status=$?
  if [[ -n $TMP_DIR && -d $TMP_DIR ]]; then
    find "$TMP_DIR" -depth -delete
  fi
  return "$status"
}
trap cleanup EXIT

say download "$TARGET"
CURL=(curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 --connect-timeout 10 --max-time 300 --retry 3)
download_artifact() {
  local artifact=$1 max_bytes=$2
  "${CURL[@]}" --max-filesize "$max_bytes" "${RELEASE_BASE}/${artifact}" --output "${TMP_DIR}/${artifact}"
}
download_artifact SHA256SUMS.release 1048576
download_artifact SHA256SUMS.release.sig 1024
download_artifact release-public.pem 65536
download_artifact "parade-hub-${TARGET}" 134217728
download_artifact parade-agent-dist.tar.gz 268435456

KEY_DIGEST=$(sha256sum "$TMP_DIR/release-public.pem" | awk '{print $1}')
say fingerprint "$KEY_DIGEST"
if [[ -n ${PARADE_RELEASE_KEY_SHA256:-} ]]; then
  [[ $PARADE_RELEASE_KEY_SHA256 =~ ^[0-9a-f]{64}$ ]] || die key_failed
  [[ $KEY_DIGEST == "$PARADE_RELEASE_KEY_SHA256" ]] || die key_failed
elif (( TTY_AVAILABLE )); then
  msg trust >&3
  trust=""
  IFS= read -r trust <&3 || trust=""
  [[ $trust == yes ]] || die key_failed
else
  die pin_required
fi
openssl pkeyutl -verify -pubin -inkey "$TMP_DIR/release-public.pem" -rawin \
  -in "$TMP_DIR/SHA256SUMS.release" -sigfile "$TMP_DIR/SHA256SUMS.release.sig" >/dev/null \
  || die signature_failed

verify_artifact() {
  local name=$1 expected
  expected=$(awk -v path="$name" '$2 == path && $1 ~ /^[0-9a-f]{64}$/ { print $1; exit }' "$TMP_DIR/SHA256SUMS.release")
  [[ -n $expected ]] || die artifact_failed "$name"
  printf '%s  %s\n' "$expected" "$TMP_DIR/$name" | sha256sum --check --status - \
    || die artifact_failed "$name"
}
verify_artifact "parade-hub-${TARGET}"
verify_artifact parade-agent-dist.tar.gz
tar -tzf "$TMP_DIR/parade-agent-dist.tar.gz" >"$TMP_DIR/archive.list" \
  || die artifact_failed parade-agent-dist.tar.gz
if grep -Eq '(^/|(^|/)\.\.(/|$))' "$TMP_DIR/archive.list"; then
  die artifact_failed parade-agent-dist.tar.gz
fi
tar -tvzf "$TMP_DIR/parade-agent-dist.tar.gz" >"$TMP_DIR/archive.verbose" \
  || die artifact_failed parade-agent-dist.tar.gz
if awk 'substr($1,1,1)!="-" && substr($1,1,1)!="d" {found=1} END {exit !found}' \
  "$TMP_DIR/archive.verbose"; then
  die artifact_failed parade-agent-dist.tar.gz
fi
ARCHIVE_BYTES=$(awk '{total += $3} END {print total + 0}' "$TMP_DIR/archive.verbose")
[[ $ARCHIVE_BYTES =~ ^[0-9]+$ && $ARCHIVE_BYTES -le 536870912 ]] \
  || die artifact_failed parade-agent-dist.tar.gz
chmod 0755 "$TMP_DIR/parade-hub-${TARGET}"
HUB_VERSION=$("$TMP_DIR/parade-hub-${TARGET}" --version) || die artifact_failed parade-hub
[[ $HUB_VERSION == parade-hub\ * ]] || die artifact_failed parade-hub

PUBLIC_URL=${PARADE_PUBLIC_URL:-}
if [[ -z $PUBLIC_URL ]]; then
  PUBLIC_URL=$(prompt "$(msg public_url)" "http://127.0.0.1:8008")
fi
SECURE_COOKIES=true
if [[ $PUBLIC_URL == http://* ]]; then SECURE_COOKIES=false; fi
ADMIN_PASSWORD=${PARADE_ADMIN_PASSWORD:-}
if [[ -z $ADMIN_PASSWORD ]]; then
  (( TTY_AVAILABLE )) || die password_short
  msg password >&3
  printf ': ' >&3
  IFS= read -r -s ADMIN_PASSWORD <&3 || ADMIN_PASSWORD=""
  printf '\n' >&3
  msg password_again >&3
  printf ': ' >&3
  confirmation=""
  IFS= read -r -s confirmation <&3 || confirmation=""
  printf '\n' >&3
  [[ $ADMIN_PASSWORD == "$confirmation" ]] || die password_mismatch
fi
[[ ${#ADMIN_PASSWORD} -ge 12 ]] || die password_short
PASSWORD_HASH=$(printf '%s\n' "$ADMIN_PASSWORD" | "$TMP_DIR/parade-hub-${TARGET}" hash-password)
unset ADMIN_PASSWORD confirmation

say installing
getent group parade-hub >/dev/null 2>&1 || groupadd --system parade-hub
id parade-hub >/dev/null 2>&1 || useradd --system --gid parade-hub --home-dir /var/lib/parade --shell /usr/sbin/nologin parade-hub
install -d -o root -g root -m 0755 /etc/parade
install -d -o parade-hub -g parade-hub -m 0700 /var/lib/parade
install -d -o root -g parade-hub -m 0750 /var/lib/parade-dist
printf 'user=installer\ngroup=installer\n' >"$TMP_DIR/hub-account-owner"
install -o root -g root -m 0600 "$TMP_DIR/hub-account-owner" \
  /etc/parade/hub-account-owner
tar --extract --gzip --file "$TMP_DIR/parade-agent-dist.tar.gz" \
  --directory /var/lib/parade-dist --no-same-owner --no-same-permissions
chown -R root:parade-hub /var/lib/parade-dist
chmod -R u=rwX,g=rX,o= /var/lib/parade-dist
cat >"$TMP_DIR/hub.toml" <<CONFIG
[hub]
listen = "127.0.0.1:8008"
database_path = "/var/lib/parade/parade.sqlite3"
public_url = "${PUBLIC_URL}"
dist_dir = "/var/lib/parade-dist"
release_public_key_sha256 = "${KEY_DIGEST}"
trusted_proxies = ["127.0.0.1", "::1"]
secure_cookies = ${SECURE_COOKIES}
session_hours = 12
stale_after_secs = 600
offline_after_secs = 1800

[dashboard]
title = "Parade"
password_hash = "${PASSWORD_HASH}"
CONFIG
"$TMP_DIR/parade-hub-${TARGET}" check-config "$TMP_DIR/hub.toml" >/dev/null \
  || die config_failed
install -o root -g root -m 0755 "$TMP_DIR/parade-hub-${TARGET}" /usr/local/bin/parade-hub
install -o root -g parade-hub -m 0640 "$TMP_DIR/hub.toml" /etc/parade/hub.toml
install -o root -g root -m 0644 /dev/stdin /etc/systemd/system/parade-hub.service <<'UNIT'
[Unit]
Description=Parade fleet observability Hub
After=network-online.target
Wants=network-online.target
StartLimitIntervalSec=5min
StartLimitBurst=10

[Service]
Type=simple
User=parade-hub
Group=parade-hub
ExecStart=/usr/local/bin/parade-hub /etc/parade/hub.toml
Restart=on-failure
RestartSec=5s
MemoryHigh=256M
MemoryMax=512M
TasksMax=128
LimitNOFILE=8192
LogRateLimitIntervalSec=30s
LogRateLimitBurst=500
StateDirectory=parade
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
ReadWritePaths=/var/lib/parade
ReadOnlyPaths=/var/lib/parade-dist

[Install]
WantedBy=multi-user.target
UNIT
systemctl daemon-reload
systemctl enable --now parade-hub.service >/dev/null
systemctl is-active --quiet parade-hub.service || die artifact_failed parade-hub.service
say active "$HUB_VERSION"
say next
say fingerprint "$KEY_DIGEST"
