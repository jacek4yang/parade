#!/usr/bin/env bash
# Local-only removal of Parade's own units and binaries. State is preserved by
# default. Nothing in Parade can invoke this remotely.
set -euo pipefail

LANGUAGE=${PARADE_LANG:-}
COMPONENT=${1:-}
PURGE=0
TTY_AVAILABLE=0
if [[ -c /dev/tty ]] && { exec 3<>/dev/tty; } 2>/dev/null; then TTY_AVAILABLE=1; fi
for argument in "$@"; do [[ $argument == --purge ]] && PURGE=1; done

choose_language() {
  if [[ $LANGUAGE == zh* ]]; then LANGUAGE='zh-CN'; return; fi
  if [[ $LANGUAGE == en* ]]; then LANGUAGE='en'; return; fi
  local choice="" default_choice=1
  [[ ${LC_ALL:-${LANG:-}} == zh* ]] && default_choice=2
  if (( TTY_AVAILABLE )); then
    printf '\nParade uninstaller language / 卸载程序语言\n  1) English\n  2) 简体中文\nSelect / 请选择 [%s]: ' "$default_choice" >&3
    IFS= read -r choice <&3 || choice=""
  fi
  if [[ ${choice:-$default_choice} == 2 ]]; then LANGUAGE='zh-CN'; else LANGUAGE='en'; fi
}

say() {
  local en=$1 zh=$2
  [[ $LANGUAGE == zh-CN ]] && printf '%s\n' "$zh" || printf '%s\n' "$en"
}

choose_language
[[ $(uname -s) == Linux ]] || { say 'Only Linux is supported.' '仅支持 Linux。' >&2; exit 1; }
[[ $EUID -eq 0 ]] || { say 'Run as root (normally with sudo).' '请以 root 身份运行（通常使用 sudo）。' >&2; exit 1; }
for required in systemctl rm grep; do
  command -v "$required" >/dev/null 2>&1 || {
    say "Required command is missing: $required" "缺少必需命令：$required" >&2
    exit 1
  }
done
if (( PURGE )); then
  for required in find id getent userdel groupdel; do
    command -v "$required" >/dev/null 2>&1 || {
      say "Required purge command is missing: $required" "缺少彻底清理所需命令：$required" >&2
      exit 1
    }
  done
fi
if [[ -z $COMPONENT ]]; then
  if (( ! TTY_AVAILABLE )); then
    say 'Specify hub, agent, or all.' '请指定 hub、agent 或 all。' >&2
    exit 1
  fi
  printf 'Component / 组件 [agent/hub/all]: ' >&3
  IFS= read -r COMPONENT <&3
fi
[[ $COMPONENT == agent || $COMPONENT == hub || $COMPONENT == all ]] || {
  say 'Component must be hub, agent, or all.' '组件必须是 hub、agent 或 all。' >&2
  exit 1
}
if (( TTY_AVAILABLE )); then
  say "This removes Parade ${COMPONENT} programs. State is preserved unless --purge is present." "这会删除 Parade ${COMPONENT} 程序；除非指定 --purge，否则保留状态。" >&3
  printf 'Type uninstall to continue / 输入 uninstall 继续: ' >&3
  confirmation=""
  IFS= read -r confirmation <&3 || confirmation=""
  [[ $confirmation == uninstall ]] || exit 1
elif [[ ${PARADE_CONFIRM_UNINSTALL:-} != uninstall ]]; then
  say 'Non-interactive removal requires PARADE_CONFIRM_UNINSTALL=uninstall.' '非交互卸载必须设置 PARADE_CONFIRM_UNINSTALL=uninstall。' >&2
  exit 1
fi

remove_agent() {
  local remove_user=0 remove_group=0
  if [[ -f /etc/parade/agent-account-owner ]]; then
    grep -qx 'user=installer' /etc/parade/agent-account-owner && remove_user=1
    grep -qx 'group=installer' /etc/parade/agent-account-owner && remove_group=1
  fi
  if systemctl cat parade-agent.service >/dev/null 2>&1 || [[ -e /etc/systemd/system/parade-agent.service ]]; then
    if ! systemctl disable --now parade-agent.service >/dev/null; then
      say 'Could not stop and disable the Agent; no files were removed.' '无法停止并禁用 Agent；尚未删除任何文件。' >&2
      exit 1
    fi
  fi
  if systemctl is-active --quiet parade-agent.service; then
    say 'The Agent is still active; no files were removed.' 'Agent 仍在运行；尚未删除任何文件。' >&2
    exit 1
  fi
  rm -f /etc/systemd/system/parade-agent.service /usr/local/bin/parade-agent
  if (( PURGE )); then
    rm -f /etc/parade/agent.toml
    if [[ -d /var/lib/parade-agent ]]; then find /var/lib/parade-agent -depth -delete; fi
    if (( remove_user )) && id parade >/dev/null 2>&1 && ! userdel parade; then
      say 'Failed to remove the parade user.' '无法删除 parade 用户。' >&2
      exit 1
    fi
    if (( remove_group )) && getent group parade >/dev/null 2>&1 && ! groupdel parade; then
      say 'Failed to remove the parade group.' '无法删除 parade 组。' >&2
      exit 1
    fi
    if (( ! remove_user || ! remove_group )); then
      say 'A pre-existing or unverified parade account/group was preserved.' '已保留预先存在或无法验证归属的 parade 用户/组。'
    fi
    rm -f /etc/parade/agent-account-owner
  fi
  say 'Agent program removed. Configuration and state were preserved unless --purge was used.' 'Agent 程序已删除；除非使用 --purge，否则配置和状态已保留。'
}

remove_hub() {
  local remove_user=0 remove_group=0
  if [[ -f /etc/parade/hub-account-owner ]]; then
    grep -qx 'user=installer' /etc/parade/hub-account-owner && remove_user=1
    grep -qx 'group=installer' /etc/parade/hub-account-owner && remove_group=1
  fi
  if systemctl cat parade-hub.service >/dev/null 2>&1 || [[ -e /etc/systemd/system/parade-hub.service ]]; then
    if ! systemctl disable --now parade-hub.service >/dev/null; then
      say 'Could not stop and disable the Hub; no files were removed.' '无法停止并禁用 Hub；尚未删除任何文件。' >&2
      exit 1
    fi
  fi
  if systemctl is-active --quiet parade-hub.service; then
    say 'The Hub is still active; no files were removed.' 'Hub 仍在运行；尚未删除任何文件。' >&2
    exit 1
  fi
  rm -f /etc/systemd/system/parade-hub.service /usr/local/bin/parade-hub
  if (( PURGE )); then
    rm -f /etc/parade/hub.toml
    if [[ -d /var/lib/parade ]]; then find /var/lib/parade -depth -delete; fi
    if [[ -d /var/lib/parade-dist ]]; then find /var/lib/parade-dist -depth -delete; fi
    if (( remove_user )) && id parade-hub >/dev/null 2>&1 && ! userdel parade-hub; then
      say 'Failed to remove the parade-hub user.' '无法删除 parade-hub 用户。' >&2
      exit 1
    fi
    if (( remove_group )) && getent group parade-hub >/dev/null 2>&1 && ! groupdel parade-hub; then
      say 'Failed to remove the parade-hub group.' '无法删除 parade-hub 组。' >&2
      exit 1
    fi
    if (( ! remove_user || ! remove_group )); then
      say 'A pre-existing or unverified parade-hub account/group was preserved.' '已保留预先存在或无法验证归属的 parade-hub 用户/组。'
    fi
    rm -f /etc/parade/hub-account-owner
  fi
  say 'Hub program removed. Database, configuration, audit history, and release tree were preserved unless --purge was used.' 'Hub 程序已删除；除非使用 --purge，否则数据库、配置、审计历史和发布目录已保留。'
}

if [[ $COMPONENT == agent || $COMPONENT == all ]]; then remove_agent; fi
if [[ $COMPONENT == hub || $COMPONENT == all ]]; then remove_hub; fi
systemctl daemon-reload
