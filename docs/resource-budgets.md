# Resource and retention budgets

English | [简体中文](resource-budgets.zh-CN.md)

Parade is designed for small Linux VPSes and large fleets, but it does not
claim zero resource use or a database that can never grow. Normal telemetry,
queues and operational history have explicit bounds; low-frequency identity,
audit, tombstone, finding and accounting evidence remains durable by design.

## Default sampling and upload profile

- sample local resource counters every 10 seconds;
- upload compact rollups about every 5 minutes with random jitter;
- keep one pending Agent report with bounded retry/backoff;
- include only small OS/kernel/architecture identifiers and their content hash
  in the normal compact report;
- send bounded process/listener facts only when changed or suspicious;
- send bounded process/socket snapshots only for a typed, expiring observation
  lease;
- end live detail automatically after at most 10 minutes.

The normal upload target is below 10 MiB per Agent per 30 days. The deterministic
regression fails above 20 MiB, excluding explicit live-detail sessions and real
security event bursts. Correctness, authentication and accounting are not
weakened to reach the target.

## Current measured baseline

The successful 2026-08-08 local release baseline recorded:

| Measurement | Result |
| --- | ---: |
| Normal encoded report body | 390 B |
| Synthetic 32-process snapshot body | 3,469 B |
| Estimated normal upload / Agent / 30 days | 5,695,110 B (5.431 MiB) |
| Agent release binary | 2,947,936 B |
| Hub release binary | 6,302,600 B |
| Idle Hub peak RSS | 8,612 KiB |
| Idle Hub file descriptors / threads | 7 / 5 |
| 1,000 signed report ingest | 13.681 s (73.1 reports/s) |
| 1,000-Agent count query | 4.266 ms |
| Synthetic database size | 1,822,720 B |

The host used the `powersave` governor with `perf_event_paranoid=3`. These are
reproducible regression data, not hardware-independent capacity guarantees.
Protocol overhead assumptions include 250 bytes per request, 2,048 TLS bytes
per day, and a conservative allocation equivalent to one 32-process snapshot
per day. The Agent does not schedule a daily snapshot; it sends process evidence
when its stable evidence hash changes or a typed lease requests it, so real
process/listener changes and events add traffic according to actual activity.
The exact command and raw test ledger are in [`PLANS.md`](../PLANS.md).

## Agent hard bounds

- `MemoryHigh=96M`, `MemoryMax=128M`, `TasksMax=64` in the supplied unit;
- one pending report;
- at most 64 reported interfaces and 32 traffic anomalies;
- at most 256 same-boot interface baselines;
- at most 1 MiB from any single procfs read;
- bounded top-N process and listener snapshots;
- normal counter state checkpointed at most once per minute.

Boot changes, counter resets/new segments, pending reports, acknowledgements,
identity and policy changes are still persisted immediately. The one-minute
steady-state checkpoint reduces flash/disk write amplification without allowing
a second crash to replay an entire new counter segment as fresh traffic.

## Hub hard bounds

- `MemoryHigh=256M`, `MemoryMax=512M`, `TasksMax=128` in the supplied unit;
- report bodies no larger than 256 KiB;
- bounded rate-limit keys, 15-second HTTP handler timeouts, query pages and UI
  rows;
- SQLite WAL mode, automatic checkpoint and 16 MiB journal size limit;
- at most 10,000 expired rows deleted per table per retention transaction;
- a bounded 24-edge reporting-topology response;
- finding subject churn folded into 32 series plus one overflow series per
  server/rule/version.

Systemd limits are deployment safety defaults. Raise a Hub limit only after
measuring the real fleet and documenting the local override. Parade never
changes host-global journald quotas.

## Retention windows

| Data | Default retention |
| --- | --- |
| Detailed resource rollups | 30 days |
| Traffic rollups | 90 days |
| Process/listener changes | 7 days plus latest value |
| Events | 180 days |
| Ordinary message metadata | 1 day |
| Raw traffic checkpoints | 400 days, preserving seed-tied/latest checkpoints |
| Expired sessions/tokens/leases | periodic bounded cleanup |

Agent identity/revocation, durable tombstones, audit events, security finding
history, traffic cycle instances, immutable seeds and append-only corrections
are retained because deleting them would weaken security or accounting. An
unlimited number of real operator actions therefore produces slow legitimate
database growth. Back up and archive according to policy and monitor
`/var/lib/parade`; do not advertise “zero growth.”

## Run the regression gate

On an isolated development/CI host, not a monitored target:

```bash
PARADE_PERF_REPORT=/tmp/parade-performance.txt scripts/performance-gate.sh
```

The gate builds locked normal Release binaries, records Git/binary/machine
metadata, checks binary/dependency ceilings, executes the deterministic
bandwidth and 1,000-Agent tests, and samples an idle Hub. It does not control,
stress, kill, reconfigure or inject faults into monitored VPSes.

For troubleshooting resource growth, see [troubleshooting](troubleshooting.md).
