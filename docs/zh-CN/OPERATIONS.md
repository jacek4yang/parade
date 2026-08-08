# Parade 简体中文全生命周期运维指南

[English](../operations.md) | 简体中文 | [文档索引](../index.zh-CN.md)

本文覆盖从获取发布物到最终退役的完整流程。命令中的域名、路径和版本必须替换成你的真实值。任何步骤都不能突破 Parade 的只读边界。

## 1. 先确认安全边界

被监测 VPS 永远只是观察目标。Parade 不提供以下能力：

- 任意命令、Shell、脚本、模板、任务、插件或上传代码执行；
- 进程终止/暂停/调整，服务启动/停止/重启；
- 软件包、用户、SSH 密钥、sudo、cron、systemd、内核、DNS、路由或防火墙修改；
- 任意文件读取、下载、上传、编辑、删除、权限或所有权修改；
- 重启、关机、重装、厂商写入或自动修复。

Agent 没有监听端口，只主动连接 Hub。Hub 的临时详情请求是闭合枚举、短 TTL、严格大小限制、双方自动到期并记录审计；没有用户命令、路径或表达式字段。

Agent 身份签名不是远程证明。root/内核已失陷的主机能伪造或隐藏遥测。没有发现问题不能证明主机安全。

## 2. 规划拓扑：公网与 NAT

推荐结构：

```text
浏览器 -> HTTPS 反向代理 -> 127.0.0.1:8008 Parade Hub
                                      ^
                                      | 仅出站 HTTPS
                   公网 Agent / NAT Agent / CGNAT Agent
```

- NAT/CGNAT Agent：无需端口映射，只需 DNS、准确时间和到 Hub HTTPS/443 的出站连接。
- 公网 IP Agent：同样不开放 Parade 端口；不要为 Parade 添加入站防火墙规则。
- NAT 后的 Hub：必须由运维人员在 Parade 外提供所有 Agent 可达的 HTTPS origin，例如固定端口映射、反向代理或已有 VPN。Parade 不自动配置 UPnP/NAT-PMP、路由、VPN、隧道或代理。
- 多台 Agent 可能共享一个出口 IP，出口也可能变化。Agent 凭据和 server ID 才是身份，IP 不是身份。

Overview 的“已观察到的上报拓扑”只显示最近经过验证的 Agent→Hub 上报边，并把相同来源归类为“可能共享 NAT/代理出口”。它不扫描主机、不推断 Agent 之间连接、不显示原始来源 IP，也不证明公网入站可达。

## 3. 前置条件

运行时：

- Linux VPS；
- Hub 一行安装支持 x86_64、aarch64；
- Agent 发行树支持 x86_64/aarch64/armv7 的 musl，及 x86_64/aarch64/armv7/riscv64gc 的 GNU 回退；
- systemd；
- Hub 生产环境有 DNS 和 HTTPS 反向代理；
- `curl`、OpenSSL、SHA-256 工具及常见系统账号工具。

从源码构建还需要稳定版 Rust、Node.js 22/npm，以及跨架构时使用的 `cross`。这些不是 Parade 运行时外部服务。

## 4. 使用 GitHub 已签名 Release 安装 Hub

当 Releases 页面已经发布版本时：

```bash
curl -fsSL https://github.com/jacek4yang/parade/releases/latest/download/parade-install.sh | sudo bash -s -- hub
```

交互流程：

1. 通过控制终端选择 English 或简体中文；
2. 自动检测 Linux 和 CPU 架构；
3. 下载发布清单、签名、公钥、Hub 和 Agent 发行树；
4. 显示发布公钥 SHA-256，首次交互信任需明确输入 `yes`；
5. 验证 Ed25519 清单签名、每个制品 SHA-256 与 Hub `--version`；
6. 输入规范的 Hub origin 和至少 12 字符管理员密码；
7. 运行 `parade-hub check-config` 后才安装并启动服务。

自动化安装必须预先从可信渠道得到发布公钥摘要。有人值守时，无回显读取密码，
避免将密码字面量写进 Shell 历史：

```bash
read -rsp 'Hub 管理员密码：' PARADE_ADMIN_PASSWORD
printf '\n'
export PARADE_ADMIN_PASSWORD
export PARADE_LANG=zh-CN
export PARADE_VERSION=v0.1.0
export PARADE_PUBLIC_URL=https://parade.example.com
export PARADE_RELEASE_KEY_SHA256='64位十六进制摘要'
sudo --preserve-env=PARADE_LANG,PARADE_VERSION,PARADE_PUBLIC_URL,PARADE_ADMIN_PASSWORD,PARADE_RELEASE_KEY_SHA256 \
  bash parade-install.sh hub
unset PARADE_ADMIN_PASSWORD
```

完全无人值守时，应由隔离 runner 的 secret manager 注入
`PARADE_ADMIN_PASSWORD` 环境变量；它仍会短暂对 root/执行账号的进程检查可见，
结束后必须清除。安全要求：不要盲目信任 `curl | bash`。高保障环境应按照
[生产部署指南](../deployment.zh-CN.md#高保障-release-验证)固定一个明确标签，
通过独立可信渠道核对公钥摘要，阅读脚本，再执行。Release 缺少签名 Secret 时
工作流会失败，不会发布未签名替代品。

## 5. 从源码构建并离线签名

```bash
git clone https://github.com/jacek4yang/parade.git
cd parade
cd frontend
npm ci
npm run build
cd ..
cargo build --release --workspace --all-features --locked
```

必须先构建前端，再构建 Hub，否则 Hub 会嵌入旧资源。

在仓库之外创建并保护一次性信任根：

```bash
umask 077
openssl genpkey -algorithm Ed25519 -out /secure/offline/parade-release.key
parade_agent_stage=$(mktemp -d)
PARADE_RELEASE_SIGNING_KEY=/secure/offline/parade-release.key \
  scripts/build-agents.sh --dist "$parade_agent_stage"
sha256sum "$parade_agent_stage/release-public.pem"
```

私钥不能进入 Git、Hub 配置、发行目录、日志或工单。创建 `parade-hub` 账号后，
先审阅用户可写的暂存树，再只把公钥、分离签名、校验和与目标二进制复制到 root
拥有且 Hub 组只读的 `/var/lib/parade-dist`，并按本地策略删除暂存目录。完整多架构
树需要 `cross`；没有 `cross` 时不能宣传脚本已跳过的目标。Hub 只配置公钥摘要。

服务账号存在后，把已审阅暂存树复制到正式目录：

```bash
sudo install -d -o root -g parade-hub -m 0750 /var/lib/parade-dist
sudo cp -a "$parade_agent_stage"/. /var/lib/parade-dist/
sudo chown -R root:parade-hub /var/lib/parade-dist
sudo chmod -R u=rwX,g=rX,o= /var/lib/parade-dist
```

自动 Release 由 `.github/workflows/release.yml` 在 `v*` 标签触发。仓库管理员需要将同一 Ed25519 私钥 PEM 的 base64 值配置为 GitHub Actions Secret `PARADE_RELEASE_SIGNING_KEY_B64`。标签必须与 Cargo workspace 版本完全一致，例如 `v0.1.0`。

## 6. 手动初始化 Hub

生成 Argon2id 密码哈希：

```bash
read -rsp 'Hub 管理员密码：' parade_admin_password
printf '\n'
printf '%s\n' "$parade_admin_password" | target/release/parade-hub hash-password
unset parade_admin_password
```

创建专用用户与路径，复制 `config/hub.toml`，填写：

- `listen = "127.0.0.1:8008"`；
- `database_path = "/var/lib/parade/parade.sqlite3"`；
- 无路径、查询、凭据的规范 `public_url`，远程地址必须 HTTPS；
- root 拥有的 `dist_dir`；
- `release_public_key_sha256`；
- 只包含直接反向代理 IP 的 `trusted_proxies`；
- 真实 `$argon2id$...` 密码哈希。

校验和安装：

```bash
target/release/parade-hub check-config /etc/parade/hub.toml
sudo install -o root -g root -m 0755 target/release/parade-hub /usr/local/bin/
sudo install -o root -g root -m 0644 systemd/parade-hub.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now parade-hub
systemctl is-active parade-hub
ss -ltn | grep '127.0.0.1:8008'
```

## 7. HTTPS 反向代理

以 `nginx/parade.conf` 为基础：

1. Hub 保持只监听回环地址；
2. 反向代理连接 Hub 的直接 IP 才能加入 `trusted_proxies`；
3. 完整验证 HTTPS 后再启用 HSTS；
4. 修改后先执行 `nginx -t`，成功才 reload；
5. 不要从不可信 `Host` 或 `X-Forwarded-*` 自动生成公网地址。

如果反向代理不在本机，需要同时修改 Hub 监听与网络隔离；这属于 Hub 部署面，不得开放 Agent 端口。

## 8. 首次登录与多主机注册

打开 `public_url`，界面会首次跟随浏览器语言。右上角可以在 English/简体中文之间切换，选择保存在浏览器本地。

对主机 A、B、C 分别执行：

1. 在“设置”输入唯一 server ID（`[A-Za-z0-9._-]+`，最长 64）与名称；
2. 创建记录并取得 15 分钟、单次、只绑定该记录的命令；
3. 在对应 VPS 上完整复制执行；
4. 安装器选择语言、自动检测架构、验证固定公钥/清单/二进制、自检并注册；
5. Agent 作为 `parade` 非特权用户启动，仅写 `/var/lib/parade-agent`；
6. 约一个正常上报周期后在 Fleet 验证首份签名数据。

不要把 A 的命令用于 B/C。一个 Agent 被攻破不能冒充另一个 server ID。共享 NAT 下注册使用较宽的共享出口限额与每令牌独立重试限额，因此合法批量部署不会因旧的 30 台共享限制被误阻断。

验证 Agent 无监听端口：

```bash
systemctl is-active parade-agent
sudo -u parade /usr/local/bin/parade-agent check-config /etc/parade/agent.toml
ss -ltnup
```

最后一条需要人工确认没有 `parade-agent` 监听项，不能把“没有观察到”当成公网不可达证明。

## 9. 厂商流量计费全场景

先设置：IANA 时区、账期日期 1–31、本地边界时间、接口策略和计费模式。等首个可靠 checkpoint 后，从厂商面板读取同一时刻的本周期用量。

闭合模式：

| 模式 | 厂商种子输入 | 计算方式 | 限额 |
| --- | --- | --- | --- |
| 入站+出站 | 一个合计值 | 合计种子 + 后续 RX + TX | 一个总限额 |
| 仅入站 | RX 值 | RX 种子 + 后续 RX | 一个入站计费限额 |
| 仅出站 | TX 值 | TX 种子 + 后续 TX | 一个出站计费限额 |
| 较大方向 | RX 与 TX 两个值 | `max(当前 RX, 当前 TX)` | 一个总限额 |
| 分别计费 | RX 与 TX 两个值 | 两个方向独立 | RX/TX 独立限额 |

较大方向不能只填合计值。例如种子 RX=100、TX=90，之后 TX 增加 20，正确结果是 110；`100 + max(0,20)` 会错误得到 120。Parade 因此强制两个方向种子并拒绝猜测。

保存前确认页必须显示：厂商输入、精确 Agent checkpoint、当前周期和保存后的公式。种子不可修改。错误只能追加带原因的正/负调整；分别计费模式必须选择方向。所有修改进入审计。

月度边界自动建立零种子新周期，不修改 Linux 计数器。日期 29–31 在短月取最后一天；DST 缺口/歧义按确定规则处理。跨边界离线且无法精确拆分时显示 `estimated`，不能伪装为精确值。

NAT 场景要特别注意：厂商配额可能属于共享公网网关，而 Agent 只能观察本机所选接口；NAT 开销、其他租户或其他主机流量无法归属。此时厂商种子仍是权威起点，但后续本机增量与厂商账单可能不同，界面会保留此限制。

## 10. 日常观察

- Fleet：在线/陈旧/离线/待注册、分组、系统、覆盖度、最后上报；
- Resources：CPU、负载、内存/Swap、PSI、硬盘/inode；
- Processes：有界 Top-N、UID/PID/PPID、CPU/内存、cgroup/unit、可写路径/已删除可执行文件证据；
- Network：选定接口 RX/TX、错误/丢包、监听地址和端口；
- Security：规则 ID/版本、严重度、置信度、首次/最近、次数、证据、原因、人工核验和覆盖限制；
- Events/Audit：可用性、身份、计费、租约和 Hub 元数据变化；
- 临时详情：闭合只读 profile，最多 10 分钟、单次正文最多 256 KiB，可提前结束并显示真实响应字节。

不要因覆盖不完整自动提权。确需认证日志时，只由本机管理员评估并授予狭窄的只读日志组。

## 11. 资源、日志与保留

默认硬边界：

- Agent `MemoryHigh=96M`、`MemoryMax=128M`、`TasksMax=64`；
- Hub `MemoryHigh=256M`、`MemoryMax=512M`、`TasksMax=128`；
- Agent 一个待发报告，接口 64、异常 32、同启动基线 256；
- 每个 procfs 文件最多读取 1 MiB；
- 报告正文最多 256 KiB，限流器最多 10,000 个键；
- SQLite WAL `journal_size_limit=16 MiB`，自动 checkpoint；
- 每分钟每表最多清理 10,000 行。

Agent 仍每 10 秒采样，但普通原始计数器最多每 60 秒耐久 checkpoint 一次；boot/计数器 reset/新计数段、待发报告、回执、身份和流量策略变化仍立即落盘，防止再次崩溃时重放整个新计数段而重复计费。同一次 Linux boot 内普通进程崩溃可从当前内核计数器补回；若崩溃后主机又重启，跨窗口会如实标记为部分/不确定，而不是伪造精确值。

保留窗口：资源 30 天、流量汇总 90 天、进程/端口变化 7 天加每台最新值、事件 180 天、普通消息 1 天、原始流量 checkpoint 400 天（种子绑定和每台最新值例外）。过期 token、session、非活动租约也会清理。

持久证据：身份/撤销、墓碑、审计、安全发现、周期、种子、调整。这些记录因安全与计费要求不会被常规保留任务删除，因此数据库会随真实的低频管理历史缓慢增长。定期备份、监控 `/var/lib/parade`，按组织政策导出/归档；不要宣称绝对零增长。

服务只设置日志速率上限。journald 总容量属于全机策略，可由主机管理员在 `/etc/systemd/journald.conf` 设置 `SystemMaxUse` 等配额并按发行版流程重启 journald；Parade 安装器不会修改全局日志策略。

## 12. Agent 凭据轮换与升级

对同一个服务器记录签发新的注册 token，并在原 VPS 重跑完整注册命令。成功注册会绑定新身份并撤销旧身份；本机持久的单调流量累计不会因重新注册清零。旧二进制不会在注册成功前被替换。令牌兑换前失败时，安装器会尝试恢复之前活跃的服务；一旦开始兑换，Hub 可能已提交新身份但响应在网络中丢失，此时安装器会让旧服务保持停止，避免用已撤销凭据做错误回滚。请签发新的单次 token 后重跑。Hub 与 Agent 可以安全同机安装：共享的 `/etc/parade` 仅由 root 管理并允许目录穿越，各组件配置仍只对各自服务组可读。

协议版本发生不兼容变更时 Hub/Agent 必须配套升级或重新注册；Hub 不会猜测旧报文布局。

## 13. Hub 升级与回滚

升级前：

1. 做一致性 SQLite 备份；
2. 记录 Hub 版本和 `schema_migrations` 最大版本；
3. 备份 Hub 配置与 Agent 发布清单；
4. 停止 Hub，替换已验证二进制，启动并检查日志；
5. 验证登录、Fleet、一个签名报告、流量周期与 Audit。

迁移在监听前以事务执行。Migration 4 将旧合计种子迁移为 `sum`，旧调整迁移为 `billed`，总量不变。Parade 不承诺 schema 向下迁移；真正回滚需要恢复升级前完整备份，不能手工降低版本号。

## 14. 备份与恢复

在线备份使用 SQLite `.backup` 或等价 API：

```bash
sqlite3 /var/lib/parade/parade.sqlite3 ".backup '/secure/backup/parade.sqlite3'"
```

离线备份先停止 Hub，再把主数据库与存在的 `-wal`、`-shm` 作为同一集合复制。配置和 `/var/lib/parade-dist` 清单另行备份。备份应加密并保留权限。

恢复先在隔离 Hub 验证：SQLite integrity、迁移、管理员登录、Fleet、Agent 报告和流量周期。恢复旧身份数据库会让 replay cursor 倒退；必须审查该时间窗口，完整性存疑时逐台轮换 Agent。

## 15. 撤销、删除与退役

- 未使用 token 泄漏：不要继续使用，签发新 token；旧 token 15 分钟后过期。
- Agent 私钥泄漏：重新注册轮换；无法信任时删除服务器记录。
- 删除服务器必须填写原因，生成永久 tombstone，并撤销身份和所有未使用 token。
- 当前没有 tombstone 恢复流程；退役后复用物理主机应采用新 server ID。

本地一行卸载：

```bash
curl -fsSL https://github.com/jacek4yang/parade/releases/latest/download/parade-uninstall.sh | sudo bash -s -- agent
curl -fsSL https://github.com/jacek4yang/parade/releases/latest/download/parade-uninstall.sh | sudo bash -s -- hub
```

便捷命令通过 GitHub HTTPS 信任 root 级脚本；高保障流程应按
[生产部署指南](../deployment.zh-CN.md#本机卸载)固定标签、验证已签名校验和并
审阅后本机执行。默认保留状态。只有已完成证据与备份评估后才添加 `--purge`。
非交互卸载还必须设置 `PARADE_CONFIRM_UNINSTALL=uninstall`。Hub 退役前保留
SQLite 和审计；Agent 本地 `/var/lib/parade-agent` 是否删除取决于身份/流量调查
需求。没有任何远程卸载路径。

## 16. 故障排查

- Hub 不启动：运行 `parade-hub check-config`；检查 Argon2id、规范 HTTPS origin、公钥 pin、数据库权限和 `journalctl -u parade-hub -e`。
- 没有注册命令：检查 `dist_dir` 的 `SHA256SUMS`、签名、公钥和目标架构文件是否完整且 Hub 可读。
- 安装校验失败：绝不跳过校验。重新构建/签名/暂存，重新签发一次性命令。
- Agent 不启动：运行 `sudo -u parade parade-agent check-config /etc/parade/agent.toml`，检查时间、DNS、HTTPS、权限和 journal。
- identity/sequence 被拒绝：检查是否恢复了旧 Hub 备份、复制了状态或重复注册；必要时人工审查并轮换。
- Traffic 等待 checkpoint：等首份签名上报；种子必须绑定精确 checkpoint。
- 跨边界为 `estimated`：这是无法精确拆分的诚实状态，可依据厂商面板追加审计修正。
- NAT Agent 不上线：只检查出站 DNS/HTTPS/时间和 Hub 可达性，不开放 Agent 入站端口。
- 覆盖不完整：先阅读 coverage 原因，不要自动增加权限。

## 17. 上线验收与开发验证

主机验收：

```bash
parade-hub --version
parade-agent --version
systemctl is-active parade-hub
systemctl is-active parade-agent
ss -ltn
```

确认 Hub 仅在计划地址监听、Agent 无监听、HTTPS 登录有效、首份签名报告进入 Fleet、Traffic 公式与 Audit 可见。

仓库验证：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

cd frontend
npm run format:check
npm run lint
npm run typecheck
npm test
npm run build
npm run test:e2e
cd ..

bash -n hub/assets/install.sh scripts/*.sh
shellcheck hub/assets/install.sh scripts/*.sh
bash scripts/test-installer-integrity.sh
npm --prefix frontend audit --audit-level=high
cargo audit
```

带宽回归测试名为 `default_monthly_bandwidth_is_below_target`；1,000 Agent 合成数据库负载测试名为 `synthetic_fleet_load_1000_agents_is_bounded`。只有实际成功执行的命令才能标为通过，精确结果记录在 `PLANS.md` 和草稿 PR 中。

完整性能门禁：

```bash
PARADE_PERF_REPORT=/tmp/parade-performance.txt scripts/performance-gate.sh
```

它使用锁定依赖的正常 Release 二进制，记录 Git SHA、二进制 SHA、内核、架构、Rust 版本、CPU governor 和 `perf_event_paranoid`，检查二进制体积与依赖特性，执行带宽/千节点测试，并对临时本地 Hub 采样 RSS、FD、线程与初始数据库大小。它只控制本机临时测试 Hub，绝不在被监测 VPS 上运行压力、kill、cgroup 或网络扰动操作。需采集 DWARF/`perf` 热点时应使用隔离的取证构建；正式性能数字仍来自普通发布二进制。

## 18. 已知限制

- 上报拓扑不是网络地图、链路质量探测、自动路由或分布式存储；当前架构仍是一个 Hub/SQLite 加每台 Agent 的有界本地状态。
- Hub 无法仅凭来源 IP 可靠判断公网、NAT、代理或 VPN，也不能证明两个 Agent 直连。
- 厂商 API 不在本里程碑；共享 NAT 网关配额可能无法按单机准确归属。
- root/内核失陷主机可伪造遥测。
- 持久安全/计费证据在无限操作下会缓慢增长；常规资源、日志型遥测和队列是有界的。
- systemd 内存上限是安全默认值。超大 Fleet 若确有测量证据需要提高 Hub 上限，应使用本地 systemd override，并保留明确容量记录；Parade 不自动修改。
