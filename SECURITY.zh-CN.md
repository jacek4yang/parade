# 安全策略与运维

[English](SECURITY.md) | 简体中文

## 产品边界

Parade 只观察被监测 Linux 主机。它不会远程执行命令，控制进程、服务、软件包、
防火墙、用户或文件，重启/重装机器，调用厂商写 API，或者自动修复发现。任何能
执行上述操作的代码路径都应作为严重漏洞报告。

## 安全部署

1. 使用随附 unit，以专用 `parade-hub` 用户运行 Hub。
2. 保持 Hub 绑定 `127.0.0.1:8008`，只通过受维护的 TLS 反向代理开放。完整域名
   HTTPS 验证后再启用 HSTS。
3. 使用 `parade-hub hash-password` 生成唯一管理员哈希；明文密码不能进入 TOML、
   Shell 历史、Git 或服务环境。
4. `trusted_proxies` 只配置直接代理地址，不能为方便而信任宽泛网段。
5. 限制 `/etc/parade/hub.toml`、`/var/lib/parade`、备份与发布暂存目录。数据库
   包含敏感遥测和有效 session 哈希。
6. 构建启用 TLS 的 Agent，暂存离线签名的 `SHA256SUMS`、分离签名和固定公钥，
   再从已认证界面复制完整注册命令。Token 15 分钟后过期且只使用一次。重新注册
   失败不会提前覆盖既有二进制；在 token 兑换前失败时会恢复此前活跃服务。
7. 安装后的 Agent 以 `parade` 用户运行，无 capabilities、无监听端口。可选日志
   可见性只能在评估额外暴露后，通过狭窄只读组授予。

Hub 会拒绝空密码/非 Argon2id、远程明文公网 URL、不安全 URL 字符和无效时效
阈值。

## 凭据事件响应

- 注册 token 疑似在使用前泄漏：丢弃并签发新 token；已用或过期 token 不能
  重用。
- Agent 私钥疑似失陷：重新注册轮换，或删除/墓碑该 server。删除会持久撤销
  活跃身份和未使用 token。
- 管理员 session 疑似被盗：先保留审计证据，再在 SQLite 中撤销 session，或
  轮换控制台密码哈希并使 session 行失效。
- Hub 疑似失陷：将数据库、发布 manifest、Agent 注册及界面全部遥测视为不可信；
  从已知镜像重建，并逐台轮换 Agent。

不要把 token、Cookie、私钥、原始敏感报告或生产配置粘贴到 issue。报告应包含
版本、威胁场景、最小复现和建议披露窗口。

## 能力限制

Ed25519 认证只能证明哪一个已注册 Agent 发送了数据，不能证明其内核说了真话。
root/内核失陷可以伪造或隐藏全部观察事实。Parade 不能成为唯一事件响应证据源。
厂商用量可能与接口计数器不同，Linux 也不能重建开始观察前的流量。

## 备份与审计

使用 SQLite 在线备份功能；或者先停 Hub，再复制数据库与 WAL/SHM 状态。备份应
加密、测试恢复并保留文件所有权。操作员修改记录在 `audit_events`；流量种子和
修正是仅追加记录，常规保留任务不会删除这些证据。

完整部署步骤见[简体中文运维指南](docs/zh-CN/OPERATIONS.md)，安全排查见
[简体中文故障排查](docs/troubleshooting.zh-CN.md)。
