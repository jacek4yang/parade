# Parade documentation

English | [简体中文](index.zh-CN.md)

Parade is a self-hosted, low-bandwidth, strictly read-only Linux VPS fleet
observability console. Start with the short guide, then use the production
guide before exposing a Hub or enrolling real servers.

## Start here

| Guide | Purpose |
| --- | --- |
| [Getting started](getting-started.md) | Install one Hub, enroll the first Agent, and verify the first report. |
| [Production deployment](deployment.md) | HTTPS, NAT/public hosts, multi-host enrollment, upgrades, backup, and removal. |
| [Complete operator lifecycle](operations.md) | Build-to-retirement reference, including release signing and incident response. |
| [Troubleshooting](troubleshooting.md) | Safe diagnosis without adding privileges or weakening verification. |

## Accounting and capacity

| Guide | Purpose |
| --- | --- |
| [Manual provider traffic accounting](traffic-accounting.md) | Five billing modes, checkpoint-bound seeds, rollover, corrections, and uncertainty. |
| [Resource and retention budgets](resource-budgets.md) | Bandwidth, memory, disk, spool, retention, and measured regression baselines. |

## Design and security evidence

| Document | Purpose |
| --- | --- |
| [Architecture](../ARCHITECTURE.md) | Components, protocol flow, persistence, concurrency, and recovery. |
| [Threat model](../THREAT_MODEL.md) | Trust boundaries, abuse cases, controls, and claims Parade does not make. |
| [Security policy](../SECURITY.md) | Secure deployment and credential-response procedures. |
| [Traffic-accounting specification](../TRAFFIC_ACCOUNTING_SPEC.md) | Normative accounting model and required edge cases. |
| [Migration policy](../MIGRATION.md) | Schema versions, backup, restoration, and rollback boundaries. |
| [Repository audit](../AUDIT.md) | Audited implementation surface and defect history. |
| [Verification ledger](../PLANS.md) | Exact checks, measurements, limitations, and delivery state. |

## Language map

| Topic | English | 简体中文 |
| --- | --- | --- |
| Project overview | [English](../README.md) | [简体中文](../README.zh-CN.md) |
| Documentation index | [English](index.md) | [简体中文](index.zh-CN.md) |
| Getting started | [English](getting-started.md) | [简体中文](getting-started.zh-CN.md) |
| Production deployment | [English](deployment.md) | [简体中文](deployment.zh-CN.md) |
| Complete lifecycle | [English](operations.md) | [简体中文](zh-CN/OPERATIONS.md) |
| Traffic accounting | [English](traffic-accounting.md) | [简体中文](traffic-accounting.zh-CN.md) |
| Resource budgets | [English](resource-budgets.md) | [简体中文](resource-budgets.zh-CN.md) |
| Troubleshooting | [English](troubleshooting.md) | [简体中文](troubleshooting.zh-CN.md) |
| Security policy | [English](../SECURITY.md) | [简体中文](../SECURITY.zh-CN.md) |

## Authority and safety

The shipped binaries, closed protocol enums, database migrations, and
configuration validators are authoritative when prose and behavior differ.
Do not bypass a failed signature, checksum, enrollment, TLS, or configuration
check. Parade never needs an Agent listening port and never provides a remote
command, shell, process/service control, firewall modification, file mutation,
provider write, or automatic-remediation path.
