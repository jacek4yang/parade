# 快速上手

[English](getting-started.md) | 简体中文

这是从已签名 GitHub Release 到第一台 Linux VPS 正常上报的最短安全路径。反向
代理加固、多主机运维、备份、升级和卸载请继续阅读[生产部署](deployment.zh-CN.md)。

## 1. 检查前置条件

你需要：

- 一个已发布的 Parade 签名 Release；
- 一台带 systemd 的 x86_64 或 aarch64 Linux Hub；
- 生产环境使用的域名与 HTTPS 反向代理；
- 一台或多台能够出站访问该 HTTPS 地址的 Linux Agent 主机；
- 安装主机上的 `curl`、OpenSSL 和常见账号/服务工具。

Hub 默认只监听 `127.0.0.1:8008`。Agent 不监听任何端口。NAT/CGNAT Agent
无需端口映射；如果 Hub 位于 NAT 后，必须在 Parade 之外提供稳定可达的 HTTPS
地址。Parade 不修改路由、防火墙、VPN、隧道或端口映射。

## 2. 安装 Hub

便捷引导命令：

```bash
curl -fsSL https://github.com/jacek4yang/parade/releases/latest/download/parade-install.sh | sudo bash -s -- hub
```

这条命令最初依赖 GitHub HTTPS 取得脚本。脚本开始执行后，才会使用已确认或固定
的 Ed25519 发布公钥验证清单和全部制品，并检查 SHA-256 与 Hub 自检。高保障
环境应先下载并审阅脚本，再使用独立渠道取得的公钥摘要验证发布物；完整流程见
[生产部署](deployment.zh-CN.md#高保障-release-验证)。

按提示选择 English 或简体中文，然后输入：

1. 规范的公网 HTTPS 地址，例如 `https://parade.example.com`；
2. 至少 12 字符、从未复用的管理员密码；
3. 对屏幕所示发布公钥指纹的明确确认。

安装器会拒绝不支持的系统/架构、不安全 URL、无效公钥/签名/校验和以及残缺的
既有 Hub 安装，不会猜测修复。

## 3. 配置 HTTPS

保持 Hub 监听回环地址。以 [`nginx/parade.conf`](../nginx/parade.conf) 为基础
配置受维护的反向代理，只把直接代理地址写入 `trusted_proxies`，先验证代理
配置和完整 HTTPS，再启用 HSTS。Parade 不会根据不可信 `Host` 或转发头猜测
公网地址。

打开配置好的 HTTPS 地址并登录。界面首次跟随浏览器语言，也可以随时在 English
与简体中文之间切换。

## 4. 注册第一台 Agent

在**设置**中创建具有唯一 server ID 和名称的主机记录。复制生成的 15 分钟、
单次注册命令，并在这条记录对应的 VPS 上完整运行。

Agent 安装器会：

- 检测 Linux 架构并验证固定的发布树；
- 为这一台主机创建全新的 Ed25519 身份；
- 只把 token 用于预先创建的 server 记录；
- 安装无 capabilities 的专用非特权 `parade` 服务；
- 只写 Parade 自己的私有配置与状态目录；
- 启动无监听端口、仅出站连接的服务。

不要把一台主机的注册命令复制给另一台。主机 A、B、C 以及后续每台 VPS 都要
分别创建记录和命令。

## 5. 验证首份上报

在 Agent 主机运行：

```bash
systemctl is-active parade-agent
sudo -u parade /usr/local/bin/parade-agent check-config /etc/parade/agent.toml
ss -ltnup
```

确认 `parade-agent` 为 active，且没有监听 socket。在 Hub 等待首个正常汇总后，
检查：

- Fleet 的时效和采集覆盖；
- Resources 的 CPU、内存、磁盘/inode 和压力证据；
- Processes 与 Network 的有界隐私保护信息；
- Security 的证据、置信度和覆盖限制；
- Events/Audit 的注册与上报历史。

缺失或不支持的采集不等于主机安全。不要自动增加权限，先阅读明确的 coverage
原因。

## 6. 启动厂商流量周期

出现首个可靠流量检查点后：

1. 打开该主机的 **Traffic** 页面；
2. 设置时区、月度边界、接口策略和五种闭合计费方式之一；
3. 同时查看厂商面板的本周期当前用量；
4. 输入数值和来源说明；
5. 确认精确 Agent 检查点和保存后的公式预览。

Parade 保存不可变人工种子，只增加之后本机观察到的流量；下一个日历周期自动从
零开始，但不重置 Linux 计数器。使用“较大方向”或“分别计费”前请阅读
[厂商流量计费](traffic-accounting.zh-CN.md)。

## 下一步

- 多台公网/NAT 主机、反向代理、备份、升级和卸载：
  [生产部署](deployment.zh-CN.md)。
- 从构建到退役：[完整生命周期运维](zh-CN/OPERATIONS.md)。
- 带宽、内存和保留限制：[资源预算](resource-budgets.zh-CN.md)。
- 安全诊断：[故障排查](troubleshooting.zh-CN.md)。
- 安全假设：[简体中文安全策略](../SECURITY.zh-CN.md)。
