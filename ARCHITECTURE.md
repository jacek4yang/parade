# Parade architecture

Parade is a self-hosted, read-only Linux VPS observability system. Production
deployment consists of one Hub binary, one outbound-only Agent binary per VPS,
an embedded Preact UI, and an embedded SQLite database. No external database,
queue, CDN, analytics service, provider API, or runtime JavaScript dependency is
required.

```text
operator browser                 monitored VPS
  authenticated HTTPS              unprivileged parade user
          |                                  |
          v                                  | reads allowlisted /proc,/sys facts
 reverse proxy/TLS                            | writes only Parade private state
          |                                  v
          v                         Parade Agent + bounded spool
 Parade Hub  <---- signed postcard/TLS ----- |
   |      ^          every 5 min + jitter
   |      |
   v      +---- closed typed lease in acknowledgement
 SQLite WAL
```

## Trust boundaries

- The browser can mutate Hub-side metadata only after Argon2id login, a
  server-side expiring session, and CSRF validation. It cannot express an
  operation against the monitored operating system.
- The reverse proxy terminates TLS. The Hub accepts forwarded client identity
  only when the immediate peer IP is in `trusted_proxies`; its canonical public
  URL is never inferred from request headers.
- Every Agent creates its own Ed25519 key. A short-lived, single-use token binds
  that public key and a fresh Agent ID to one pre-created server record.
- Reports are signed over protocol version, server/Agent identity, timestamp,
  monotonic sequence, unique message ID, and body. The Hub rejects wrong
  content types, malformed/oversized/unsupported/stale/replayed reports before
  one transaction persists the report and its checkpoint.
- Host telemetry is evidence, not an attestation. A root- or kernel-compromised
  VPS can lie to Parade or hide activity.

## Structural read-only enforcement

`parade-common::ObservationProfile` is the complete Hub-to-Agent request
vocabulary: `Normal`, `ResourceDetail`, `ProcessSnapshot`, `SocketSnapshot`,
`SecurityLogSummary`, and expiring `LiveDetail`. Unknown variants fail serde
decoding. Requests carry no command, script, path, SQL, plugin, task, service,
process, file, package, firewall, user, provider, or remediation operation.
All leases are audited, bounded, and expire after at most ten minutes.
Lease responses must name the exact lease and match its stored profile. The Hub
links every accepted response to that lease, accounts encoded bytes, exposes a
countdown/measurement in the UI, and supports audited early cancellation. A
cancelled Agent returns to normal mode on its next outbound acknowledgement;
Agent-side expiry remains authoritative during network loss.

The Agent has no listener. Its steady-state process is an unprivileged static
user with no capabilities. It reads narrowly defined Linux telemetry and writes
only its configuration, private signing identity, monotonic traffic accumulator,
one pending signed report, active lease state, and logs in Parade-owned paths.

## Data and protocol flow

The Agent samples locally every ten seconds. Every five minutes plus stable and
per-cycle jitter it sends a compact `postcard` rollup. Static inventory is
represented by a content hash; bounded top-N/suspicious process facts and
listener snapshots are sent only when their content changes. A single pending
signed report survives restart and retries with bounded exponential backoff.

Collectors accept a fixture root. Current Linux sources include CPU/load,
memory/swap, PSI, disk/inodes, socket counts, uptime/boot/OS/kernel, interface
counters, bounded privacy-preserving process facts, listeners, and collection
coverage. Process environment variables and full command lines are never read
or transmitted.

## Persistence and concurrency

SQLite is opened in WAL mode with foreign keys, a busy timeout, explicit schema
migrations, indexed fleet/time queries, and short-lived connections. Report
identity/replay state, telemetry rollups, findings, events, traffic checkpoints,
seeds, adjustments, cycles, sessions, leases, audit records, revocations, and
tombstones are transactional. There is no application-wide state mutex across
serialization, database, network, or alert delivery. The only mutex bounds the
small in-memory rate-limiter map to 10,000 keys.

Detailed resources are retained for 30 days, traffic rollups for 90 days,
process/socket changes for seven days while preserving the latest per server,
and events for 180 days. Durable identity, replay cursors, audit, findings,
tombstones, seeds, adjustments, and cycle history are not erased by routine
retention. Raw checkpoints are bounded to 400 days while every seed-tied
checkpoint and the latest checkpoint per server are retained.

## Traffic accounting

The Agent converts selected raw interface counters into persistent monotonic RX
and TX totals across process restarts, counter resets, and boots. The default
interface policy follows default routes and excludes loopback, bridge, veth,
container, and tunnel interfaces. Ambiguous coverage is reported.

Interface policy delivery is versioned per Agent credential. When the Hub rule
advances it accepts exactly one report made with the previously delivered
version, advances the delivery cursor in the same ingest transaction, and
returns the new version in the acknowledgement. An exact duplicate may retrieve
a lost acknowledgement; a new report that continues using the old version is
rejected. This avoids both update deadlock and indefinite stale-policy use.

For each calendar cycle Parade stores a checkpoint-tied immutable manual seed:

```text
cycle total = max(0, provider manual seed
                     + locally observed bytes after its checkpoint
                     + append-only audited adjustments)
```

Rules use an IANA timezone, day 1–31, and local time; short months clamp to the
last day. A Hub timer rolls due cycles even without a report. If no counter was
observed exactly at the boundary, the split is labeled `estimated`, then refined
from the adjacent checkpoints when reporting resumes. History is preserved and
Linux counters are never reset.

## Finding engine and UI

The current engine emits stable versioned evidence for suspicious writable-path
executables, deleted executables, sustained high CPU heuristics, newly listening
ports, and reduced collector coverage. Findings contain severity, confidence,
first/last seen, recurrence count, evidence, explanation, manual verification,
and a coverage caveat. It never labels a server clean or safe.

The embedded Preact/TypeScript UI uses paginated fleet queries and progressive
server tabs. It includes overview, fleet, security, traffic, events, settings,
resources, privacy-preserving processes, listeners, findings, inventory,
operator audit, and traffic seed/rule/adjustment workflows. Temporary detail
leases have a countdown, response-byte measurement, and early cancellation.
Themes, density, keyboard focus, reduced motion, explicit states, and 390 px
layouts are built in.

## Recovery and upgrades

Agents retain their sequence, key, pending report, and traffic accumulator over
Hub outages. The Hub atomically rejects duplicates after restart. SQLite
migrations run before listening and fail startup rather than partially applying.
Back up with SQLite's online backup API/CLI or stop the Hub and copy the database
plus WAL state. See `MIGRATION.md` for the legacy boundary and rollback policy.
