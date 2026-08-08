# 故障排查

[English](troubleshooting.md) | 简体中文

诊断 Parade 时不能削弱信任边界。不得通过关闭 TLS、签名/校验和、重放检查、
大小限制或专用服务账号来“修复”安装，也不能开放 Agent 端口或加入远程命令。

## Hub 无法启动

先只读查看本机状态和日志，再校验确切配置：

```bash
systemctl status parade-hub --no-pager
journalctl -u parade-hub -e --no-pager
/usr/local/bin/parade-hub check-config /etc/parade/hub.toml
```

检查：

- `password_hash` 是真实 Argon2id，空密码不会被接受；
- 远程 `public_url` 使用 HTTPS，且没有路径、查询或凭据；
- `trusted_proxies` 只含直接代理地址；
- 数据库目录允许 `parade-hub` 写入；
- 发布目录由 root 拥有，但 Hub 组可读；
- 配置的发布公钥摘要与暂存公钥一致。

迁移在监听前执行。迁移失败时保留数据库/WAL/SHM，恢复完整的已知良好备份；
不要手工修改 schema 版本行。

## HTTPS 登录、跳转或客户端地址错误

确认 Hub 仍只监听计划地址，并在本机测试反向代理配置：

```bash
ss -ltnp
nginx -t
```

确认 `public_url` 与浏览器 origin 完全相同，直接代理地址已明确受信任。不能为让
转发头“能用”而信任整个大网段。完整 HTTPS 验证前不要启用 HSTS。

## 没有 Agent 注册命令

只有暂存 Agent 发布树完整且清单签名有效时，Hub 才签发命令。检查 `dist_dir`
中的 manifest、分离签名、公钥、目标架构文件以及 Hub 读取权限。重新完整构建、
签名和暂存；不能绕过代码校验，也不能手工拼接命令。

## 安装器拒绝 Release

公钥 pin、Ed25519 签名、manifest digest、checksum、archive 或 self-test 失败时
必须停止。确认所有文件来自同一个 tag，通过独立渠道核对公钥摘要；若固定清单已
变化，应重新签发注册命令。

若重新注册在 token 开始兑换后失败，即使响应丢失，旧凭据也可能已经撤销。安装器
会故意不重启该身份。请签发新的单次 token，再重跑已审阅的安装器。

## Agent 已安装但不上报

在这台 Agent 主机做只读检查：

```bash
systemctl status parade-agent --no-pager
journalctl -u parade-agent -e --no-pager
sudo -u parade /usr/local/bin/parade-agent check-config /etc/parade/agent.toml
timedatectl status
getent hosts parade.example.com
ss -ltnup
```

检查出站 DNS/HTTPS、时间、配置权限、Hub URL、身份/状态所有权，以及 server
是否已撤销或 tombstone。NAT/CGNAT 主机不需要入站规则；应确认没有
`parade-agent` 监听，而不是开放端口。

Agent 只保留一个待发签名报告。长时间中断后，经认证的 stale marker 会让它只
退休该 envelope，保留单调流量状态，再发送当前有界证据。若持续出现 identity/
sequence 拒绝，通常意味着复制/回滚状态或恢复了旧 Hub；轮换前应先审查身份和
重放窗口。

## 数据陈旧、部分、不支持或缺失

以界面显示的 collector coverage 原因为准。常见原因包括内核/架构差异、procfs
字段不存在、无权读取 allowlist 日志源、counter reset 或上报缺口。Parade 会
显示 unavailable，不会编造零。

不能自动把 Agent 改成 root。确需认证日志时，只能由本机管理员评估诸如
`systemd-journal` 的狭窄只读组；这会扩大暴露面，并非必需。

## Traffic 等待检查点

创建厂商种子前等待一个可靠签名汇总。种子必须绑定精确 checkpoint。确认至少
一个预期接口被实际选中，并检查 anomaly/partial 原因。

## Parade 与厂商数值不同

对照计费模式、时区/边界、种子检查点、实际选中接口、方向组成和修正。厂商可能
计算开销、舍入、共享 NAT、私网流量或不同方向。应添加带原因的仅追加修正，不能
修改原始种子或 Linux 计数器。

边界显示 `estimated` 表示 Agent 离线或计数器无法精确拆分，这是诚实的不确定
证据，不是临时 UI 错误。

## 磁盘或内存高于预期

先只读查看本机服务资源和文件大小，不要删除证据：

```bash
systemctl show parade-hub -p MemoryCurrent -p TasksCurrent
systemctl show parade-agent -p MemoryCurrent -p TasksCurrent
du -h /var/lib/parade
ls -lh /var/lib/parade/parade.sqlite3*
```

普通汇总、队列和 WAL 有边界/保留期。身份、墓碑、审计、发现和流量周期历史是
有意持久化的。任何本地归档策略前都应先备份并审查。Parade 不修改全机 journald
配额。

性能门禁只能在隔离开发机/CI 上运行，不能在被监测 VPS 上运行。详见
[资源预算](resource-budgets.zh-CN.md)。

## UI 显示旧资源或错误状态

静态 JS/CSS 使用强 ETag 和 revalidation；HTML/API 为 `no-store`。先正常刷新
浏览器，检查 HTTP 状态与 Content-Type，并确认先完成生产前端构建、再构建 Hub。
不要添加 CDN、外部脚本、tracker 或宽松内容类型。

## 报告问题时保留什么

可以提供 Parade 版本、架构、已脱敏的配置结构、精确错误、时间戳/时区、采集
覆盖和最小复现。不得提供注册 token、Authorization header、Cookie、私钥、
密码、完整生产配置、原始敏感报告或任意主机文件。疑似漏洞遵循
[简体中文安全策略](../SECURITY.zh-CN.md)。
