# Parade 文档

[English](index.md) | 简体中文

Parade 是自托管、低带宽、严格只读的 Linux VPS 多主机观察台。第一次使用请先走
最短上手流程；接入真实主机或开放 Hub 前，再完成生产部署检查。

## 从这里开始

| 指南 | 用途 |
| --- | --- |
| [快速上手](getting-started.zh-CN.md) | 安装一个 Hub、注册第一台 Agent，并确认首份上报。 |
| [生产部署](deployment.zh-CN.md) | HTTPS、NAT/公网主机、多机注册、升级、备份和卸载。 |
| [完整生命周期运维](zh-CN/OPERATIONS.md) | 从构建、签名、部署一直到轮换、响应与退役。 |
| [故障排查](troubleshooting.zh-CN.md) | 不提权、不关闭安全校验的诊断路径。 |

## 计费与容量

| 指南 | 用途 |
| --- | --- |
| [厂商流量人工计费](traffic-accounting.zh-CN.md) | 五种计费模式、检查点种子、月结、修正与不确定性。 |
| [资源与保留预算](resource-budgets.zh-CN.md) | 带宽、内存、硬盘、队列、保留窗口和实测回归基线。 |

## 设计与安全证据

| 文档 | 用途 |
| --- | --- |
| [架构（英文）](../ARCHITECTURE.md) | 组件、协议流、持久化、并发与恢复。 |
| [威胁模型（英文）](../THREAT_MODEL.md) | 信任边界、滥用场景、控制措施与不作出的安全承诺。 |
| [安全策略](../SECURITY.zh-CN.md) | 安全部署与凭据事件响应。 |
| [流量计费规范（英文）](../TRAFFIC_ACCOUNTING_SPEC.md) | 权威计费模型与必须覆盖的边界情况。 |
| [迁移策略（英文）](../MIGRATION.md) | Schema 版本、备份、恢复与回滚边界。 |
| [仓库审计（英文）](../AUDIT.md) | 已审计的实现面与缺陷历史。 |
| [验证台账（英文）](../PLANS.md) | 实际执行的检查、测量、限制与交付状态。 |

## 中英文对照

| 主题 | English | 简体中文 |
| --- | --- | --- |
| 项目概览 | [English](../README.md) | [简体中文](../README.zh-CN.md) |
| 文档索引 | [English](index.md) | [简体中文](index.zh-CN.md) |
| 快速上手 | [English](getting-started.md) | [简体中文](getting-started.zh-CN.md) |
| 生产部署 | [English](deployment.md) | [简体中文](deployment.zh-CN.md) |
| 完整生命周期 | [English](operations.md) | [简体中文](zh-CN/OPERATIONS.md) |
| 厂商流量计费 | [English](traffic-accounting.md) | [简体中文](traffic-accounting.zh-CN.md) |
| 资源预算 | [English](resource-budgets.md) | [简体中文](resource-budgets.zh-CN.md) |
| 故障排查 | [English](troubleshooting.md) | [简体中文](troubleshooting.zh-CN.md) |
| 安全策略 | [English](../SECURITY.md) | [简体中文](../SECURITY.zh-CN.md) |

## 权威来源与安全原则

若文档描述与程序行为不同，以发布二进制、闭合协议枚举、数据库迁移和配置校验器
为准。签名、校验和、注册、TLS 或配置检查失败时，不能通过关闭校验来“修好”。
Agent 永远不需要监听端口；Parade 不提供远程命令、Shell、进程/服务控制、
防火墙修改、任意文件变更、厂商写入或自动修复能力。
