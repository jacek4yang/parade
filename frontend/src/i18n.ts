export type Locale = "en" | "zh-CN";

export const LOCALE_STORAGE_KEY = "parade-locale";

const zhCN: Record<string, string> = {
  Overview: "概览",
  Fleet: "服务器群",
  Security: "安全",
  Traffic: "流量",
  CPU: "CPU",
  Findings: "安全发现",
  average: "平均值",
  "active evidence": "活跃证据",
  Events: "事件",
  Settings: "设置",
  Resources: "资源",
  Processes: "进程",
  Network: "网络",
  Inventory: "清单",
  "Server detail": "服务器详情",
  "Fleet observability": "服务器群可观测性",
  "Skip to content": "跳到主要内容",
  "Primary navigation": "主导航",
  "Open navigation": "打开导航",
  "Fleet status summary": "服务器群状态摘要",
  "Server sections": "服务器栏目",
  "Read-only monitoring": "只读监测",
  "Read-only target": "只读目标",
  "No remote control or remediation": "不提供远程控制或自动修复",
  "Authenticated Hub session": "Hub 会话已认证",
  "Toggle display density": "切换显示密度",
  "Toggle color theme": "切换颜色主题",
  Language: "语言",
  English: "English",
  "Simplified Chinese": "简体中文",
  Comfortable: "舒适",
  Compact: "紧凑",
  Light: "浅色",
  Dark: "深色",
  Online: "在线",
  online: "在线",
  Stale: "数据陈旧",
  stale: "数据陈旧",
  Offline: "离线",
  offline: "离线",
  Pending: "待处理",
  pending: "待处理",
  Revoked: "已撤销",
  revoked: "已撤销",
  open: "进行中",
  critical: "严重",
  warning: "警告",
  error: "错误",
  identity: "身份",
  finding: "安全发现",
  audit: "审计",
  high: "完整",
  partial: "部分",
  unsupported: "不支持",
  unavailable: "不可用",
  "Enrollment pending": "等待注册",
  "{count} total": "共 {count} 台",
  "Reports delayed": "报告延迟",
  "Needs review": "需要检查",
  "Not yet reporting": "尚未上报",
  "Active finding evidence": "活跃安全证据",
  "Current provider traffic": "当前服务商流量",
  "Awaiting checkpoint": "等待检查点",
  "No accounting claim yet": "尚未形成核算结论",
  "Evidence, not a security score": "这是证据，不是安全评分",
  "Evidence for review, never a security score":
    "供人工审查的证据，不是安全评分",
  "Resource pressure": "资源压力",
  "CPU, memory or disk above review threshold": "CPU、内存或磁盘超过审查阈值",
  "{count} active evidence finding(s)": "{count} 条活跃证据发现",
  "Sustained CPU pressure": "持续 CPU 压力",
  "Memory or disk pressure": "内存或磁盘压力",
  "Traffic accounting uncertainty": "流量核算存在不确定性",
  "Attention queue": "待处理队列",
  "Freshness and collection gaps, ordered for review":
    "按优先级排列的数据时效与采集缺口",
  "Partial telemetry coverage": "遥测覆盖不完整",
  "Reporting gap": "存在上报缺口",
  "Nothing urgent in the current telemetry": "当前遥测中没有紧急项目",
  "This does not prove that monitored hosts are safe.":
    "这并不能证明受监测主机是安全的。",
  "Fleet distribution": "服务器群分布",
  "Current reporting state": "当前上报状态",
  "Telemetry trust": "遥测可信边界",
  "Evidence has explicit limits": "证据存在明确的能力边界",
  "Parade observes selected host-local telemetry. A sufficiently privileged attacker can falsify it. No absence of findings is proof that a host is safe.":
    "Parade 只观察选定的主机本地遥测。具有足够权限的攻击者可以伪造这些数据；没有发现问题并不能证明主机安全。",
  "Review coverage gaps →": "检查覆盖缺口 →",
  "Observed reporting topology": "已观察到的上报拓扑",
  "Verified outbound Agent reports to this Hub":
    "Agent 向此 Hub 发出的已验证出站报告",
  "Reporting paths": "上报路径",
  "Receives authenticated HTTPS reports": "接收经过身份验证的 HTTPS 报告",
  shared_observed_source: "观察到共享出口（可能为 NAT 或代理）",
  private_observed_source: "观察到私网来源",
  special_observed_source: "观察到特殊用途或共享地址来源",
  internet_scope_source: "观察到互联网范围来源",
  loopback_or_proxy_boundary: "观察到本机或代理边界",
  "{count} Agents share this observed source":
    "{count} 个 Agent 共享此观察来源",
  "Showing {shown} of {total} paths; attention-first and bounded.":
    "显示 {total} 条路径中的 {shown} 条；优先展示需关注项并保持有界。",
  "This is not a mesh or reachability scan. Shared or private sources may indicate NAT, a proxy, VPN, or routing policy; even an internet-scope source does not prove inbound reachability.":
    "这不是网状网络或可达性扫描。共享或私网来源可能来自 NAT、代理、VPN 或路由策略；即使观察到互联网范围来源，也不能证明该主机可从公网入站访问。",
  "{count} registered servers": "已登记 {count} 台服务器",
  "Search fleet": "搜索服务器群",
  "Search name, ID or group": "搜索名称、ID 或分组",
  "Showing {start}–{end}": "显示第 {start}–{end} 项",
  Status: "状态",
  Server: "服务器",
  Group: "分组",
  System: "系统",
  Coverage: "覆盖范围",
  "Last report": "最近报告",
  "Awaiting inventory": "等待清单",
  "No matching servers": "没有匹配的服务器",
  "Change the search query or add a server in Settings.":
    "请修改搜索条件，或在“设置”中添加服务器。",
  Previous: "上一页",
  Next: "下一页",
  "Security center": "安全中心",
  "Evidence-based findings are grouped by server and rule.":
    "基于证据的发现按服务器和规则分组。",
  "Fleet traffic": "服务器群流量",
  "Cycles with different boundaries remain labeled separately.":
    "具有不同边界的计费周期会分别标记。",
  "Fleet events": "服务器群事件",
  "Availability, security, traffic and audit evidence.":
    "可用性、安全、流量和审计证据。",
  "Open cycles": "进行中的周期",
  "Calendar instances": "日历周期实例",
  "Manually seeded": "已手动设种子",
  "Checkpoint-tied provider entries": "与检查点绑定的服务商数据",
  Uncertain: "存在不确定性",
  "Partial or estimated": "部分或估算数据",
  "day {day} at {time}": "每月 {day} 日 {time}",
  "{count} server(s)": "{count} 台服务器",
  Servers: "服务器",
  "Open cycle accounting": "查看周期核算",
  "{coverage} coverage": "覆盖范围：{coverage}",
  "No servers yet": "尚无服务器",
  "Create a server record in Settings, then enroll its Agent.":
    "请先在“设置”中创建服务器记录，再注册其 Agent。",
  "Inventory pending": "等待清单",
  "kernel unknown": "内核未知",
  "architecture unknown": "架构未知",
  "Last report {value}": "最近报告：{value}",
  "Coverage {value}": "覆盖范围：{value}",
  "Health summary": "健康摘要",
  "Latest accepted signed rollup": "最新接受的签名汇总",
  Availability: "可用性",
  Agent: "Agent",
  "Inventory fingerprint": "清单指纹",
  "Unavailable data is never treated as a healthy zero":
    "不可用数据绝不会被当作健康的零值",
  "Privacy-preserving processes": "保护隐私的进程信息",
  "Network and listening ports": "网络与监听端口",
  "Security evidence": "安全证据",
  "Static facts are sent on enrollment or content change":
    "静态信息仅在注册或内容变化时发送",
  "Operating system": "操作系统",
  Kernel: "内核",
  Architecture: "架构",
  "Observation mode": "观察模式",
  "Unprivileged, outbound-only": "非特权、仅出站",
  Unsupported: "不支持",
  "Enable a typed read-only detail profile for 10 minutes? This increases bandwidth and expires automatically.":
    "要启用 10 分钟的类型化只读详细观察吗？这会增加带宽占用，并自动到期。",
  "Lease {id} active until {time}.": "观察租约 {id} 有效至 {time}。",
  "Lease cancelled. The Agent returns to normal mode on its next outbound acknowledgement.":
    "观察租约已取消；Agent 在下一次出站确认后返回普通模式。",
  "Snapshot {value}": "快照：{value}",
  "not available": "不可用",
  "Read-only detail active · {time} remaining":
    "只读详细观察已启用 · 剩余 {time}",
  "Normal mode minimizes bandwidth": "普通模式将带宽占用降至最低",
  "{count} response(s), {bytes} measured body bytes. Automatic expiry is enforced by both Hub and Agent.":
    "已收到 {count} 个响应，实测正文 {bytes}。Hub 与 Agent 均强制自动到期。",
  "Bounded process/socket snapshots use a closed profile, add at most 256 KiB per response, and expire within 10 minutes.":
    "有界进程/套接字快照使用封闭观察配置，每次响应最多增加 256 KiB，并在 10 分钟内到期。",
  "End detail early": "提前结束详细观察",
  "Request temporary live detail": "请求临时实时详情",
  "Provider dashboard current-cycle usage": "服务商面板中的本周期已用流量",
  "Awaiting the first traffic checkpoint": "等待首个流量检查点",
  "A manual provider seed must be tied to a reliable Agent checkpoint.":
    "手动服务商种子必须绑定到可靠的 Agent 检查点。",
  "Cycle total": "周期总量",
  "Provider-billed cycle total": "服务商计费周期总量",
  "Inbound provider usage": "服务商入站用量",
  "Outbound provider usage": "服务商出站用量",
  "{percent} of inbound limit": "已用入站限额的 {percent}",
  "{percent} of outbound limit": "已用出站限额的 {percent}",
  "No inbound limit configured": "未配置入站限额",
  "No outbound limit configured": "未配置出站限额",
  "{percent} of limit": "已用限额的 {percent}",
  "No limit configured": "未配置限额",
  "Manual seed": "手动种子",
  "Provider dashboard": "服务商面板",
  "Parade observed": "Parade 观察值",
  "Since seed checkpoint": "自种子检查点起",
  Adjustments: "调整",
  "Negative correction": "负向修正",
  "Audited correction": "经审计修正",
  Projection: "预计用量",
  "Inbound projection": "预计入站用量",
  "Outbound projection": "预计出站用量",
  "Insufficient history": "历史数据不足",
  "Observed rate through cycle end": "按当前观察速率推算至周期结束",
  "Observed inbound rate through cycle end": "按当前入站观察速率推算至周期结束",
  "Observed outbound rate through cycle end":
    "按当前出站观察速率推算至周期结束",
  "Billing mode": "计费模式",
  "Closed provider accounting profile": "闭合的服务商计费配置",
  "Inbound + outbound sum": "入站与出站相加",
  "Inbound only": "仅入站计费",
  "Outbound only": "仅出站计费",
  "Larger direction": "按较大方向计费",
  "Separate inbound and outbound": "入站与出站分别计费",
  sum: "入站与出站相加",
  inbound_only: "仅入站计费",
  outbound_only: "仅出站计费",
  max_direction: "按较大方向计费",
  separate_directions: "入站与出站分别计费",
  "Transparent accounting": "透明流量核算",
  "Seed + observed + adjustments = current total":
    "种子 + 观察值 + 调整 = 当前总量",
  "Inbound and outbound remain independently auditable":
    "入站与出站保持独立且可审计",
  "directional sum (not a provider-billed total)":
    "方向合计（不是服务商计费总量）",
  "Provider directional totals": "服务商方向用量",
  "Not billed": "不计费",
  "Current cycle": "当前周期",
  Source: "来源",
  "Observed inbound": "观察到的入站流量",
  "Observed outbound": "观察到的出站流量",
  Confidence: "置信度",
  "Selected interfaces": "选定接口",
  Policy: "策略",
  "No accounting interface selected": "未选择任何计费接口",
  Uncertainty: "不确定性",
  "The latest checkpoint is incomplete or the cycle boundary could not be split exactly.":
    "最新检查点不完整，或周期边界无法被精确拆分。",
  "interface counters unavailable": "网络接口计数器不可用",
  "no selected accounting interface": "没有选中的计费接口",
  "No manual seed entered": "尚未输入手动种子",
  "Manual entry": "手动输入",
  "manual seed": "手动种子",
  "locally observed": "本地观察值",
  adjustments: "调整",
  "cycle total": "周期总量",
  Cycle: "周期",
  "Observed direction": "观察方向",
  inbound: "入站",
  outbound: "出站",
  "Observation window": "观察窗口",
  "Provider seed source": "服务商种子来源",
  "Parade measures selected Linux interface bytes. Provider billing can differ due to overhead, direction weighting, rounding and private traffic policy.":
    "Parade 测量选定 Linux 接口的字节数。服务商账单可能因协议开销、方向权重、舍入和私网流量政策而不同。",
  "Enter current provider usage": "输入服务商当前已用流量",
  "Creates one immutable primary seed at the latest checkpoint":
    "在最新检查点创建一个不可变的主种子",
  "Save the billing-cycle rule before entering a seed for the newly selected mode.":
    "请先保存计费周期规则，再按新选择的模式录入种子。",
  "Provider-used amount": "服务商已用流量",
  "Current provider-used traffic": "服务商当前已用流量",
  "Current provider-used amount": "服务商当前已用流量",
  "Current provider-used combined traffic": "服务商当前已用双向合计流量",
  "Current provider-used inbound traffic": "服务商当前已用入站流量",
  "Current provider-used outbound traffic": "服务商当前已用出站流量",
  "Traffic unit": "流量单位",
  "Effective checkpoint": "生效检查点",
  "Source note": "来源备注",
  "Confirm immutable seed": "确认不可变种子",
  "Exact Agent checkpoint": "精确 Agent 检查点",
  "Provider entry": "服务商输入值",
  "Agent checkpoint": "Agent 检查点",
  "Cycle boundary": "周期边界",
  "Result after saving: {bytes} + future selected-interface traffic":
    "保存后结果：{bytes} + 后续选定接口流量",
  "Confirm and save seed": "确认并保存种子",
  "Preview seed": "预览种子",
  "Mistakes are corrected with an append-only audited adjustment; history is never silently rewritten.":
    "错误只能通过仅追加、可审计的调整来修正；历史绝不会被静默重写。",
  "Append an audited adjustment": "追加经审计的调整",
  "Corrections preserve the original seed and full history":
    "修正会保留原始种子与完整历史",
  "Signed correction (GiB)": "带符号修正值（GiB）",
  "Adjustment direction": "调整方向",
  Inbound: "入站",
  Outbound: "出站",
  Reason: "原因",
  "Append adjustment": "追加调整",
  "Billing-cycle rule": "计费周期规则",
  "IANA timezone, calendar anchor, and optional provider limit":
    "IANA 时区、日历锚点与可选服务商限额",
  "IANA timezone": "IANA 时区",
  "Anchor day": "锚点日期",
  "Local anchor time": "本地锚点时间",
  "Traffic limit (GiB, optional)": "流量限额（GiB，可选）",
  "Provider billing mode": "服务商计费模式",
  "Inbound limit (GiB, optional)": "入站限额（GiB，可选）",
  "Outbound limit (GiB, optional)": "出站限额（GiB，可选）",
  "Selected interfaces (comma-separated; blank = automatic)":
    "选定接口（逗号分隔；留空表示自动）",
  "Excluded interfaces (comma-separated)": "排除接口（逗号分隔）",
  "Save cycle rule": "保存周期规则",
  "Billing-cycle history": "计费周期历史",
  "Latest 24 cycles with immutable seeds and append-only corrections":
    "最近 24 个周期，包含不可变种子与仅追加修正",
  "Provider total": "服务商计费总量",
  "Automatic zero rollover": "周期自动归零",
  "{count} audited corrections": "{count} 条经审计修正",
  "Only the first 50 corrections are shown": "仅显示前 50 条修正",
  "No earlier billing cycles": "暂无更早的计费周期",
  "The first completed rollover will appear here; the current cycle remains visible above.":
    "首次完成周期切换后会显示在此；当前周期仍展示在上方。",
  billed: "计费总量",
  "Interface auto-selection follows the default route and excludes loopback, container, bridge, veth, and tunnel devices. Current selected identities remain visible above.":
    "接口自动选择遵循默认路由，并排除回环、容器、网桥、veth 和隧道设备；当前选择结果始终在上方可见。",
  "Created {id}. This single-use enrollment command expires {time}.":
    "已创建 {id}。这条一次性注册命令将于 {time} 到期。",
  "Created {id}.": "已创建 {id}。",
  "Enrollment for {id} expires {time}.": "{id} 的注册命令将于 {time} 到期。",
  "Server {id} was created, but enrollment could not be issued: {error}":
    "服务器 {id} 已创建，但无法签发注册命令：{error}",
  "Agent enrollment": "Agent 注册",
  "Create a server record before enrolling one independent identity":
    "先创建服务器记录，再注册一个独立身份",
  "Server ID": "服务器 ID",
  "Display name": "显示名称",
  "Create server record": "创建服务器记录",
  "Issue or rotate enrollment": "签发或轮换注册身份",
  "One server-bound command; previous identity is revoked only after successful enrollment":
    "每条命令只绑定一台服务器；只有新身份成功注册后才撤销旧身份",
  "Existing server ID": "现有服务器 ID",
  "Issue single-use command": "签发一次性命令",
  "Retire a server record": "退役服务器记录",
  "Hub-only revocation and durable tombstone; no command is sent to the VPS":
    "只在 Hub 撤销并生成持久墓碑；不会向 VPS 发送任何命令",
  "Server ID to retire": "要退役的服务器 ID",
  "Retirement reason": "退役原因",
  "Create tombstone and revoke identity": "创建墓碑并撤销身份",
  "Permanently tombstone this server ID and revoke its Agent identity? The monitored VPS is not modified.":
    "永久为此服务器 ID 创建墓碑并撤销 Agent 身份？受监测 VPS 不会被修改。",
  "Server {id} was tombstoned.": "服务器 {id} 已生成墓碑。",
  "Security defaults": "安全默认值",
  "Hub metadata may change; monitored hosts remain observation-only":
    "Hub 元数据可以变更；受监测主机始终只允许观察",
  "Argon2id administrator authentication": "Argon2id 管理员认证",
  "Strict SameSite session and CSRF validation":
    "严格的 SameSite 会话与 CSRF 校验",
  "Explicit trusted proxy addresses only": "仅信任明确配置的代理地址",
  "SQLite WAL with transactional migrations": "SQLite WAL 与事务式迁移",
  "Independent revocable Agent credentials": "独立且可撤销的 Agent 凭据",
  "Backup and restore": "备份与恢复",
  "Operational guidance": "运维指南",
  "Use SQLite's online backup command or stop the Hub before copying the database, including WAL state. Test restores on a disposable Hub. Agent credentials remain bound to the restored server records.":
    "请使用 SQLite 在线备份命令，或先停止 Hub，再连同 WAL 状态一起复制数据库。应在一次性 Hub 上验证恢复；Agent 凭据仍与恢复后的服务器记录绑定。",
  "Operator audit log": "操作员审计日志",
  "Append-only Hub metadata and observation-profile changes":
    "仅追加记录 Hub 元数据与观察配置变更",
  Hub: "Hub",
  "No operator changes yet": "尚无操作员变更",
  "Enrollment, traffic, lease and server mutations appear here.":
    "注册、流量、观察租约和服务器元数据变更会显示在这里。",
  "CPU average": "CPU 平均值",
  "Peak {value}": "峰值 {value}",
  "Peak {value} · {count} cores · {samples} samples":
    "峰值 {value} · {count} 核 · {samples} 个样本",
  "Load average": "平均负载",
  "1 minute average": "1 分钟平均值",
  Memory: "内存",
  Swap: "交换空间",
  Disk: "磁盘",
  "Disk inodes": "磁盘 inode",
  "Filesystem capacity is unavailable on this Agent target":
    "此 Agent 目标上的文件系统容量不可用",
  "Filesystem inode counters are unavailable": "文件系统 inode 计数器不可用",
  Pressure: "压力",
  Connections: "连接数",
  "{percent} · {value} total": "{percent} · 总计 {value}",
  "CPU {cpu} · memory {memory} · I/O {io}":
    "CPU {cpu} · 内存 {memory} · I/O {io}",
  "TCP {tcp} · UDP {udp}": "TCP {tcp} · UDP {udp}",
  "CPU trend": "CPU 趋势",
  "Memory trend": "内存趋势",
  "Load trend": "负载趋势",
  "{count} bounded five-minute rollups": "{count} 个有界五分钟汇总点",
  "{value} total": "总计 {value}",
  "CPU PSI some avg10": "CPU PSI some avg10",
  "No resource rollup": "没有资源汇总",
  "The Agent has not submitted this collector.": "Agent 尚未提交此采集器数据。",
  "Full command lines and environment variables are never collected. Normal mode sends bounded top-N and suspicious facts.":
    "永不采集完整命令行和环境变量。普通模式只发送有界的 Top-N 与可疑事实。",
  "Search process facts": "搜索进程事实",
  "Search PID, UID, executable or cgroup": "搜索 PID、UID、可执行文件或 cgroup",
  "Suspicious only": "仅显示可疑项",
  State: "状态",
  Executable: "可执行文件",
  "CPU ticks": "CPU ticks",
  Virtual: "虚拟内存",
  "Unit / cgroup": "Unit / cgroup",
  Listeners: "监听套接字",
  Package: "软件包",
  Evidence: "证据",
  "deleted executable": "已删除的可执行文件",
  "writable path": "可写路径",
  unknown: "未知",
  "No process facts match the current filters.":
    "没有进程事实符合当前筛选条件。",
  "No process changes in this rollup": "本次汇总没有进程变化",
  "Normal mode sends only changed or suspicious bounded summaries.":
    "普通模式只发送发生变化或可疑的有界摘要。",
  "Inbound interval": "区间入站流量",
  "Outbound interval": "区间出站流量",
  "Counter confidence": "计数器置信度",
  "Raw counters are never reset": "绝不重置原始计数器",
  Interfaces: "网络接口",
  Interface: "接口",
  Accounting: "核算状态",
  "Packets RX / TX": "接收/发送数据包",
  "Errors RX / TX": "接收/发送错误",
  "Drops RX / TX": "接收/发送丢包",
  Selected: "已选定",
  "Observed only": "仅观察",
  "No interface counters observed": "未观察到接口计数器",
  "The collector may be unsupported or awaiting its first signed report.":
    "采集器可能不受支持，或仍在等待首份签名报告。",
  "Listening ports": "监听端口",
  Protocol: "协议",
  "Bind address": "绑定地址",
  Port: "端口",
  "Socket inode": "套接字 inode",
  Unknown: "未知",
  "No listening sockets observed": "未观察到监听套接字",
  "Coverage and permissions determine completeness; this does not prove the network surface is empty.":
    "完整性取决于覆盖范围和权限；这并不能证明网络暴露面为空。",
  "No finding is proof that the host is safe or compromised. Host-local telemetry may be falsified by a sufficiently privileged attacker.":
    "任何单个发现都不能证明主机安全或已失陷；具有足够权限的攻击者可能伪造主机本地遥测。",
  "{confidence} confidence · {count} occurrence(s)":
    "置信度 {confidence} · 出现 {count} 次",
  "{server} · first {first} · last {last}":
    "{server} · 首次 {first} · 最近 {last}",
  "first {first} · last {last}": "首次 {first} · 最近 {last}",
  "Manual verification and caveats": "人工验证与限制",
  "No active findings in retained telemetry": "保留的遥测中没有活动发现",
  "No finding is proof that the host is safe or compromised. Privileged attackers may falsify host-local telemetry.":
    "没有发现并不能证明主机安全或已失陷；高权限攻击者可能伪造主机本地遥测。",
  "No retained events": "没有保留的事件",
  "Availability, identity, traffic and finding transitions will appear here.":
    "可用性、身份、流量和发现状态变化会显示在这里。",
  "Coverage pending": "等待覆盖信息",
  "Waiting for the first signed Agent report.": "等待首份签名 Agent 报告。",
  resources: "资源",
  traffic: "流量",
  processes: "进程",
  listeners: "监听端口",
  psi: "压力指标",
  security_logs: "安全日志",
  available: "可用",
  "{path} is unavailable": "{path} 不可用",
  info: "信息",
  review: "需检查",
  medium: "中等",
  estimated: "估算",
  active: "活动",
  not_observed: "本次未再观察到",
  cancelled: "已取消",
  closed: "已关闭",
  owned: "有软件包归属",
  unowned: "无软件包归属",
  "server.create": "创建服务器",
  "server.delete": "删除服务器",
  "agent.enrollment_token.create": "签发 Agent 注册令牌",
  "agent.enroll": "注册 Agent",
  "traffic.rule.update": "更新流量规则",
  "traffic.seed.create": "创建流量种子",
  "traffic.adjustment.create": "创建流量调整",
  "observation_lease.create": "创建观察租约",
  "observation_lease.cancel": "取消观察租约",
  "Billing cycle rolled over": "计费周期已滚动",
  "Interface accounting policy delivered": "接口核算策略已送达",
  "Executables running from shared writable locations are easier to replace or stage.":
    "从共享可写位置运行的可执行文件更容易被替换或暂存。",
  "Verify the executable path, owner, package provenance, parent process, and expected deployment workflow locally.":
    "请在本机核验可执行文件路径、所有者、软件包来源、父进程和预期部署流程。",
  "Process paths may be hidden or falsified by a privileged attacker.":
    "高权限攻击者可能隐藏或伪造进程路径。",
  "A running deleted executable can be expected during upgrades, but also obscures the on-disk artifact.":
    "升级期间可能出现仍在运行但已删除的可执行文件，这也会掩盖磁盘上的实际制品。",
  "Compare the process start time with maintenance history and inspect the binary through approved local tooling.":
    "请将进程启动时间与维护记录对照，并使用获准的本机工具检查二进制文件。",
  "Executable links may be unreadable without additional permission.":
    "没有额外权限时，可能无法读取可执行文件链接。",
  "Sustained CPU can be ordinary workload or a cryptomining-compatible heuristic; it is not proof of compromise.":
    "持续高 CPU 可能是正常负载，也可能符合加密货币挖矿启发式特征；它不能证明主机已失陷。",
  "Compare the privacy-preserving top-process list and expected workload schedule locally.":
    "请在本机对照保护隐私的高占用进程列表与预期工作负载计划。",
  "This rule is a resource heuristic and does not identify intent.":
    "此规则只是资源启发式判断，不能识别行为意图。",
  "Reduced collection coverage limits the conclusions Parade can draw.":
    "采集覆盖下降会限制 Parade 能够得出的结论。",
  "Check the Agent service user permissions and the collector coverage panel; grant only narrowly scoped read access if needed.":
    "请检查 Agent 服务用户权限和采集器覆盖面板；如确有需要，只授予范围狭窄的只读权限。",
  "Missing telemetry is reported rather than interpreted as absence of risk.":
    "缺失的遥测会被明确报告，而不会被解释为没有风险。",
  "A newly observed listening socket changes the host's exposed attack surface.":
    "新观察到的监听套接字会改变主机暴露的攻击面。",
  "Confirm the owning service and intended bind address locally, then compare firewall/provider filtering separately.":
    "请在本机确认所属服务和预期绑定地址，再另行核对防火墙及服务商过滤策略。",
  "Parade observes host sockets; it does not prove Internet reachability.":
    "Parade 只能观察主机套接字，不能证明其可从互联网访问。",
  unauthorized: "未授权",
  "server not found": "未找到服务器",
  "deletion reason is required": "必须填写删除原因",
  "server id is tombstoned; restore must be explicit":
    "该服务器 ID 已生成墓碑；恢复必须显式进行",
  "bad public key": "公钥无效",
  "invalid report": "报告无效",
  "note is too long": "备注过长",
  "seed too large": "种子值过大",
  "no reliable checkpoint at or before effective timestamp":
    "生效时间及之前没有可靠检查点",
  "manual seed must use an exact Agent checkpoint":
    "手动种子必须使用精确的 Agent 检查点",
  "a primary seed already exists; use an audited adjustment":
    "主种子已存在；请使用经审计的调整",
  "a concise adjustment reason is required": "必须填写简明的调整原因",
  "adjustment effective time cannot be future": "调整生效时间不能在未来",
  "adjustment effective time is outside the current open cycle":
    "调整生效时间不在当前开放周期内",
  "adjustment would make usage negative": "该调整会使流量用量变为负数",
  "sum billing requires either one combined seed or both directional seeds":
    "双向合计计费需要一个合计种子，或同时提供入站与出站种子",
  "inbound-only billing requires an inbound seed": "仅入站计费需要入站种子",
  "outbound-only billing requires an outbound seed": "仅出站计费需要出站种子",
  "this billing mode requires both inbound and outbound seeds":
    "此计费模式必须同时提供入站与出站种子",
  "separate-direction billing uses inbound and outbound limits":
    "分别计费模式使用独立的入站与出站限额",
  "directional limits require separate-direction billing":
    "方向限额仅适用于分别计费模式",
  "separate-direction billing requires an inbound or outbound adjustment":
    "分别计费模式的调整必须选择入站或出站方向",
  "this billing mode accepts billed-total adjustments only":
    "此计费模式只接受计费总量调整",
  "inbound adjustment requires a directional provider seed":
    "入站调整需要方向明确的服务商种子",
  "outbound adjustment requires a directional provider seed":
    "出站调整需要方向明确的服务商种子",
  "adjustment would make inbound usage negative": "该调整会使入站用量变为负数",
  "adjustment would make outbound usage negative": "该调整会使出站用量变为负数",
  "cycle rules are immutable after seeded or closed history exists; create a new server accounting epoch":
    "存在已设种子或已关闭历史后，周期规则不可变；请创建新的服务器核算阶段",
  Full: "完整",
  Partial: "部分",
  Never: "从未",
  "{count}s ago": "{count} 秒前",
  "{count}m ago": "{count} 分钟前",
  "{count}h ago": "{count} 小时前",
  "{count}d ago": "{count} 天前",
  "Loading the latest accepted telemetry…": "正在加载最新接受的遥测…",
  "Data could not be loaded": "无法加载数据",
  "invalid IANA timezone": "IANA 时区无效",
  "anchor_time must be HH:MM or HH:MM:SS": "锚点时间必须为 HH:MM 或 HH:MM:SS",
  "too many interfaces": "接口数量过多",
  "inbound seed too large": "入站种子数值过大",
  "outbound seed too large": "出站种子数值过大",
  "limit too large": "限额数值过大",
  "inbound limit too large": "入站限额数值过大",
  "outbound limit too large": "出站限额数值过大",
  "invalid server id": "服务器 ID 无效",
  "server metadata is too long": "服务器元数据过长",
  "fleet search is too long": "服务器群搜索条件过长",
  "invalid observation profile": "观察配置无效",
  "CSRF validation failed": "CSRF 校验失败",
  "login rate limit exceeded": "登录尝试过多，请稍后重试",
  "report rate limit exceeded": "报告速率超过限制",
  "invalid credentials": "凭据无效",
  "not found": "未找到",
  "{message}. Previously displayed data may be stale; retrying is safe.":
    "{message}。之前显示的数据可能已经过期；可以安全重试。",
  Retry: "重试",
  "Session expired": "会话已过期",
  "Request failed ({status})": "请求失败（{status}）",
};

let activeLocale: Locale = "en";

export function normalizeLocale(value: string | null | undefined): Locale {
  return value?.toLowerCase().startsWith("zh") ? "zh-CN" : "en";
}

export function initialLocale(
  stored: string | null = typeof localStorage === "undefined"
    ? null
    : localStorage.getItem(LOCALE_STORAGE_KEY),
  languages: readonly string[] = typeof navigator === "undefined"
    ? []
    : navigator.languages,
): Locale {
  if (stored === "en" || stored === "zh-CN") return stored;
  return languages.some((language) => language.toLowerCase().startsWith("zh"))
    ? "zh-CN"
    : "en";
}

export function setLocale(locale: Locale): void {
  activeLocale = locale;
}

export function getLocale(): Locale {
  return activeLocale;
}

export function localeTag(): string {
  return activeLocale === "zh-CN" ? "zh-CN" : "en-US";
}

export function t(
  message: string,
  values: Record<string, string | number> = {},
): string {
  const template =
    activeLocale === "zh-CN" ? (zhCN[message] ?? message) : message;
  return template.replace(/\{([A-Za-z0-9_]+)\}/g, (match, key: string) =>
    Object.hasOwn(values, key) ? String(values[key]) : match,
  );
}
