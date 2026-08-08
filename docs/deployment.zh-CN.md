# 生产部署

[English](deployment.md) | 简体中文

本文把快速上手扩展为生产部署。Parade 始终只是只读观察者：本机安装器只管理
Parade 自己的文件和服务；Hub 与 Agent 都不能操作另一台主机。

## 支持的拓扑

```text
操作员 -> HTTPS 反向代理 -> 127.0.0.1:8008 Parade Hub -> SQLite/WAL
                                      ^
                                      | 仅出站 HTTPS
                          公网 / NAT / CGNAT Linux Agent
```

- 公网 IP Agent 不开放 Parade 端口；
- NAT/CGNAT Agent 只需 DNS、准确时间和出站 HTTPS；
- NAT 后的 Hub 需要运维人员提供外部可达的稳定 HTTPS 地址；
- 多台 Agent 可以共享或改变出口地址，身份依据是 server-bound 凭据，不是 IP；
- 界面拓扑只是有界的已验证 Agent→Hub 上报证据，不是扫描、Peer Mesh、转发、
  路由优化或分布式数据库。

## 高保障 Release 验证

一行命令最初信任 GitHub HTTPS。更高保障的引导应先选定一个明确且已发布的标签，
从这个不可变标签下载全部文件，不要立即执行：

```bash
tag=v0.1.0
release_base="https://github.com/jacek4yang/parade/releases/download/${tag}"
curl -fLO "${release_base}/parade-install.sh"
curl -fLO "${release_base}/SHA256SUMS.release"
curl -fLO "${release_base}/SHA256SUMS.release.sig"
curl -fLO "${release_base}/release-public.pem"
```

通过独立可信渠道取得 `release-public.pem` 的预期 SHA-256，核对后验证 Ed25519
分离签名，再验证并审阅安装脚本：

```bash
sha256sum release-public.pem
openssl pkeyutl -verify -pubin -inkey release-public.pem -rawin \
  -in SHA256SUMS.release -sigfile SHA256SUMS.release.sig
sha256sum --check --ignore-missing SHA256SUMS.release
less parade-install.sh
```

把 `v0.1.0` 替换为准备安装的确切 Release。不要混用不同 Release 的文件，任何
一步失败都应停止。运行已审阅的本地脚本时同时固定同一标签和公钥摘要，避免后续
二进制下载跟随已经切换的 `latest`：

```bash
tag=v0.1.0
sudo env \
  PARADE_VERSION="${tag}" \
  PARADE_RELEASE_KEY_SHA256='<可信的64位十六进制摘要>' \
  bash parade-install.sh hub
```

首个公开签名 Release 必须先由仓库所有者配置
`PARADE_RELEASE_SIGNING_KEY_B64`，再发布版本一致且可从默认分支到达的 `v*`
标签。签名材料或必要架构缺失时，工作流会失败关闭。

## Hub 安装与 HTTPS

交互安装器只询问语言、规范公网 URL 和管理员密码，以专用账号安装 Hub，校验
配置并启动加固的 systemd unit。它不会安装或配置反向代理。

审阅并参考 [`nginx/parade.conf`](../nginx/parade.conf)：

1. Hub 保持监听 `127.0.0.1:8008`；
2. 在 Parade 之外申请并续期有效证书；
3. `trusted_proxies` 只写直接代理 IP；
4. 本机 reload 前运行 `nginx -t`；
5. 通过 HTTPS 验证认证、Cookie 和跳转；
6. 只有整个域名都确认 HTTPS-only 后才启用 HSTS。

若反向代理不在本机，要明确隔离 Hub→代理链路。不能把 Hub 的修改接口暴露在
明文传输上。

## 注册多台主机

对主机 A、B、C 以及后续每台主机分别执行：

1. 登录 Hub，创建一条 server 记录；
2. 复制它独有的 15 分钟单次命令；
3. 只在匹配的 VPS 上运行；
4. 验证 `parade-agent` active 且没有监听 socket；
5. 等待首份签名上报并检查采集覆盖；
6. 只有可靠检查点出现后才设置账期和厂商种子。

不要共享 token，也不要把 `/var/lib/parade-agent` 复制到另一台机器。每台 Agent
都有独立私钥与序列状态；一台 Agent 失陷不能认证成另一台。

Hub 与 Agent 可以安装在同一台 VPS。`/etc/parade` 是 root 拥有的目录穿越层，
每个组件配置只允许对应服务组读取，不能改成共享的 Fleet 密钥。

## 自动化安装 Hub

自动化必须显式给出所有输入和信任值。有人值守的自动化流程可以无回显读取密码，
避免把密码字面量写入 Shell 历史；只为安装器导出，完成后立即清除：

```bash
read -rsp 'Hub 管理员密码：' PARADE_ADMIN_PASSWORD
printf '\n'
export PARADE_ADMIN_PASSWORD
export PARADE_LANG=zh-CN
export PARADE_VERSION=v0.1.0
export PARADE_PUBLIC_URL=https://parade.example.com
export PARADE_RELEASE_KEY_SHA256='<可信的64位十六进制摘要>'
sudo --preserve-env=PARADE_LANG,PARADE_VERSION,PARADE_PUBLIC_URL,PARADE_ADMIN_PASSWORD,PARADE_RELEASE_KEY_SHA256 \
  bash parade-install.sh hub
unset PARADE_ADMIN_PASSWORD
```

完全无人值守时，让专用 runner 的 secret manager 把
`PARADE_ADMIN_PASSWORD` 注入进程环境，不要拼成带明文密码的命令。环境秘密仍会
短暂对 root 和执行账号的进程检查可见，因此应隔离 runner，并在结束后清除。
不要把真实值写入 Git、Shell 历史、示例、截图或 issue。Agent 仍然逐台使用
单次 token，不存在全 Fleet 共享 bearer token。

## 上线验收

在适用的本地主机运行：

```bash
parade-hub --version
parade-agent --version
systemctl is-active parade-hub
systemctl is-active parade-agent
sudo -u parade /usr/local/bin/parade-agent check-config /etc/parade/agent.toml
ss -ltnup
```

确认 Hub 只在计划地址监听、Agent 没有监听、HTTPS 登录成功、签名上报进入
Fleet、Traffic 显示检查点/公式，且 Audit 记录操作员变更。不支持的采集必须
显示 unavailable/partial，不能静默当作零或安全。

## 备份与恢复

使用 SQLite 在线备份 API/`.backup`，或者先停 Hub，再把数据库与存在的
WAL/SHM 一起复制。备份应加密、保留所有权，并包含 Hub 配置和已签名 Agent
发布树；恢复必须先在隔离 Hub 测试。

恢复旧身份/重放数据库可能让 Hub cursor 落后于 Agent。应审查该时间段；完整性
不确定时逐台轮换身份。Schema 迁移是事务性的，但没有自动降级；回滚必须恢复
升级前的完整备份。

## 升级与凭据轮换

Hub 升级前记录版本和迁移级别，做一致性备份，验证所有新发布物，只替换 Parade
自己的 Hub 二进制；启动后检查登录、上报、计费和审计。

Agent 升级或轮换时，为同一 server 签发新命令并在本机重跑。安装器会在 token
兑换前暂存二进制、unit 和身份。如果兑换后响应丢失，旧身份可能已经撤销；安装
器会让它保持停止，而不是做错误回滚。此时签发新 token 再试。

## 本机卸载

Release 卸载器只在管理员执行它的当前主机生效，默认保留状态：

```bash
curl -fsSL https://github.com/jacek4yang/parade/releases/latest/download/parade-uninstall.sh | sudo bash -s -- agent
curl -fsSL https://github.com/jacek4yang/parade/releases/latest/download/parade-uninstall.sh | sudo bash -s -- hub
```

这些便捷命令通过 GitHub HTTPS 信任并以 root 执行脚本；卸载器与安装器不同，
之后没有其他下载制品可由它认证。高保障环境应固定一个标签，先验证并审阅脚本，
再在本机运行：

```bash
tag=v0.1.0
release_base="https://github.com/jacek4yang/parade/releases/download/${tag}"
curl -fLO "${release_base}/parade-uninstall.sh"
curl -fLO "${release_base}/SHA256SUMS.release"
curl -fLO "${release_base}/SHA256SUMS.release.sig"
curl -fLO "${release_base}/release-public.pem"
sha256sum release-public.pem
openssl pkeyutl -verify -pubin -inkey release-public.pem -rawin \
  -in SHA256SUMS.release -sigfile SHA256SUMS.release.sig
sha256sum --check --ignore-missing SHA256SUMS.release
less parade-uninstall.sh
sudo bash parade-uninstall.sh agent
```

通过与安装相同的独立渠道核对公钥摘要；只有在目标确为 Hub 主机时才把 `agent`
换成 `hub`。

只有确定身份、计费和审计证据不再需要并已备份后才使用 `--purge`。非交互卸载
还必须设置 `PARADE_CONFIRM_UNINSTALL=uninstall`。Parade 没有远程卸载路径。

## 继续阅读

- [完整生命周期运维](zh-CN/OPERATIONS.md)
- [厂商流量计费](traffic-accounting.zh-CN.md)
- [资源与保留预算](resource-budgets.zh-CN.md)
- [故障排查](troubleshooting.zh-CN.md)
- [安全策略](../SECURITY.zh-CN.md)
- [迁移与回滚（英文）](../MIGRATION.md)
