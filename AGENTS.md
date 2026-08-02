# AGENTS.md

## Mission

Parade is a self-hosted, low-bandwidth, read-only Linux VPS fleet observability and security-posture console.

The monitored-host boundary is strict:

> Parade may observe, aggregate, compare, explain, and alert. Parade must never remotely control or modify a monitored VPS.

The project should remain simple to deploy:

- one Hub binary;
- one Agent binary per supported architecture;
- an embedded web UI;
- an embedded transactional database;
- no mandatory external database, message broker, CDN, analytics service, or provider integration.

## Product priorities

Use this priority order whenever requirements compete:

1. Preserve the read-only security boundary.
2. Correct authentication, identity, replay protection, and persistence.
3. Correct traffic accounting.
4. Low bandwidth and low CPU/memory overhead.
5. Reliable operation across restarts, reboots, and network failures.
6. Clear security evidence and honest uncertainty.
7. A polished, fast, accessible interface.
8. Feature breadth.

Prefer a complete, tested vertical slice over many partial features.

## Non-negotiable monitored-host boundary

### Allowed host observations

The Agent may read only narrowly defined operating-system telemetry, including:

- `/proc` and `/sys` resource counters;
- CPU, memory, swap, load, PSI, disk, inode, network, socket, process, cgroup, boot, kernel, and operating-system metadata;
- fixed allowlisted authentication and system-log sources when the running user has permission;
- package ownership metadata through read-only queries when available;
- its own configuration, identity, local accumulator, bounded spool, and logs.

### Forbidden host actions

Never add any path that can:

- execute arbitrary commands, shell fragments, scripts, templates, SQL, plugins, WASM, uploaded code, or user-defined programs;
- terminate, signal, pause, resume, reprioritize, or otherwise control a process;
- start, stop, restart, enable, disable, or reload a service;
- reboot, shut down, suspend, or reinstall a VPS;
- install, remove, or upgrade software;
- change users, groups, passwords, SSH keys, sudo policy, cron, systemd units, kernel settings, routing, DNS, firewall rules, files, permissions, or ownership;
- upload, replace, edit, delete, or download arbitrary host files;
- invoke provider write APIs;
- automatically remediate a finding.

Do not create a generic “action”, “command”, “script”, “task”, “job”, “executor”, or “plugin” API that could later bypass these rules.

### Typed read-only observation leases

The Hub may ask the Agent to temporarily collect more detail only through a closed, versioned enum of read-only observation profiles. Examples:

- `Normal`
- `ResourceDetail`
- `ProcessSnapshot`
- `SocketSnapshot`
- `SecurityLogSummary`
- `LiveDetail { expires_at }`

Rules:

- no user-supplied command text or path;
- strict payload and duration limits;
- short TTL;
- automatic expiry;
- auditable request and response;
- unknown variants rejected;
- no operation may mutate host state outside Parade's own state directory.

## Agent privilege and data minimization

- The Agent must expose no listening port.
- Agent-to-Hub communication is outbound-only over authenticated TLS.
- Run the Agent as a dedicated unprivileged system user by default.
- Never require root for steady-state operation.
- Optional enhanced security-log visibility may use narrowly documented read-only groups such as `systemd-journal` or distro equivalents.
- Degrade gracefully when a collector is unavailable.
- Report coverage limitations instead of silently claiming complete visibility.
- Never collect environment variables.
- Never collect SSH private keys, application configuration files, browser data, arbitrary file contents, or secrets.
- Omit full process command lines by default.
- When command-line information is explicitly enabled for local/private deployments, apply conservative redaction before transmission and cap length.
- Never log credentials, authorization headers, enrollment tokens, provider secrets, session cookies, or raw sensitive payloads.

## Identity and enrollment

- Every Agent must have an independent identity and credential.
- Never use one fleet-wide bearer token for all Agents.
- A compromised Agent must not be able to impersonate another server.
- Enrollment tokens must be short-lived, single-use, and bound to one server record.
- Reports must be versioned and authenticated.
- Include server identity, timestamp, monotonic sequence, and message identifier.
- Reject replayed, stale, cross-server, malformed, oversized, and unsupported-version reports.
- Support credential rotation and revocation.
- Deleting a server must create a durable tombstone. A deleted Agent must not silently recreate itself.
- Unknown-server auto-registration must be disabled by default.

## Traffic-accounting rules

`TRAFFIC_ACCOUNTING_SPEC.md` is normative.

Key principles:

- provider API integration is not required for the current milestone;
- the user may enter the provider's already-used traffic for the current billing cycle;
- Parade records that value as an immutable manual seed at a precise timestamp;
- Parade then adds locally observed network deltas;
- cycle rollover is scheduled and automatic;
- manual corrections are append-only adjustments with audit history;
- Linux counters cannot reconstruct traffic from before observation began;
- uncertainty must be visible for intervals that cannot be split exactly;
- never silently reset or modify operating-system counters.

## Low-bandwidth defaults

Design and test a low-bandwidth normal profile.

Suggested default behavior:

- sample resource counters locally every 10 seconds;
- upload compact rollups every 5 minutes with random jitter;
- upload static inventory every 24 hours or when its content hash changes;
- upload only top-N process summaries and changed/suspicious process facts in normal mode;
- batch security events and send promptly only when events exist;
- send full process/socket snapshots only during a typed observation lease;
- automatically end live-detail mode after at most 10 minutes;
- use connection reuse, bounded retries, exponential backoff, and jitter;
- keep all queues and local spools bounded.

Create a deterministic bandwidth regression test. The intended default synthetic budget is below 10 MiB upload per Agent per 30-day month. Treat 20 MiB as a hard regression ceiling, excluding explicit live-detail sessions and genuine event bursts.

Do not reduce correctness or security merely to hit a byte target. Measure encoded payloads and protocol overhead honestly.

## Architecture

- Preserve Rust for the Hub, Agent, and shared protocol.
- Prefer an embedded SQLite database in WAL mode over one mutable JSON state file.
- Use explicit versioned migrations.
- Keep final deployment self-contained.
- A frontend build step is acceptable, but production assets must be embedded into the Hub binary.
- Prefer Preact + TypeScript + Vite and a small plotting library such as uPlot.
- Do not require a heavyweight UI framework.
- No external runtime fonts, scripts, styles, trackers, analytics, or CDNs.
- Use typed, versioned protocol messages.
- Apply strict content types, timeouts, size limits, retention limits, and backpressure.
- Avoid a global lock across serialization, database I/O, network I/O, or alert delivery.
- Use bounded worker concurrency and queues.
- Make collectors testable against fixture roots instead of hard-coding every system path.

## Security findings

Security analysis must be evidence-based.

Each finding should contain:

- stable rule ID and rule version;
- severity;
- confidence;
- first seen and last seen;
- server identity;
- concise evidence;
- why it matters;
- safe manual verification guidance;
- acknowledgment/suppression state;
- whether it is new, recurring, or baseline behavior.

Never claim that a server is “clean”, “safe”, or “not compromised”.

Document that telemetry from a fully root- or kernel-compromised host is not trustworthy.

## UI principles

`UI_SPEC.md` is normative.

- Optimize for scanning, filtering, comparison, and evidence.
- Support dark and light themes.
- Support desktop and approximately 390-pixel mobile layouts.
- Use accessible contrast, keyboard navigation, visible focus, and reduced motion.
- Large fleet tables must remain responsive.
- Use progressive disclosure.
- Display stale, partial, unsupported, disconnected, loading, empty, and error states deliberately.
- Do not use a misleading single security score.
- Do not display remote-control buttons.
- Clearly label the product and each server as read-only observation.

## Configuration

Operator configuration must be minimal.

- Hub initialization should ask only for genuinely necessary values.
- Agent enrollment should be one copy-paste command after a server record is created.
- Do not require manual per-Agent TOML editing.
- Keep secrets out of example configuration and Git.
- Bind the Hub to localhost by default.
- Provide secure reverse-proxy/TLS examples.
- Use safe first-run administration; an empty password must never expose mutation APIs.
- Trust forwarded headers only from explicitly configured proxy addresses.

## Engineering quality

- Do not hide failed checks.
- Do not weaken tests to make them pass.
- Do not ignore errors with `|| true` unless the operation is explicitly best-effort and the result is reported.
- Avoid unsafe Rust unless strictly necessary and justified.
- Keep dependencies minimal and maintained.
- Add comments for security invariants and non-obvious accounting logic, not for obvious syntax.
- Maintain backwards migration where reasonable; otherwise document the exact migration boundary.

## Required verification

Run all applicable checks before completion:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Also run:

- frontend formatting;
- frontend lint;
- TypeScript typecheck;
- frontend unit tests;
- production frontend build;
- integration tests;
- Playwright end-to-end and responsive tests;
- shell syntax checks;
- ShellCheck when installed;
- dependency/security checks such as `cargo audit` and `cargo deny` when installed;
- deterministic bandwidth regression tests;
- synthetic fleet load tests;
- a complete final diff review;
- a secret scan of committed and untracked changes.

Record exact commands and results. Never claim a command passed unless it was executed successfully.

## Git and GitHub delivery

The development server already has GitHub CLI authentication. Use it, but verify it first.

- Inspect `git status`, current branch, remotes, and default branch before editing.
- Never work directly on `main` or `master`.
- Create or use `codex/read-only-vps-observability`.
- Do not rewrite unrelated history.
- Make coherent, reviewable commits after verified milestones.
- Inspect the diff before each commit.
- Push only to the existing intended `origin`.
- Use `gh auth status` before relying on GitHub.
- Open a draft pull request when implementation and required checks are ready.
- Do not merge the pull request.
- The PR must include architecture changes, threat model, test results, traffic-accounting tests, measured bandwidth, load-test results, screenshots, migration notes, known limitations, and deferred work.
- If GitHub publication fails, preserve completed local commits and report the exact failing command and error.

## Autonomous work method

- Read `CODEX_GOAL.md`, `TRAFFIC_ACCOUNTING_SPEC.md`, and `UI_SPEC.md` before making design decisions.
- Begin with a repository-wide audit.
- Create and continuously update `PLANS.md`.
- Continue from audit to implementation; do not stop after producing a plan.
- Use subagents for independent read-heavy tasks such as architecture review, threat modeling, test review, UI review, and load-test analysis.
- Avoid concurrent edits to overlapping files.
- Ask the user only when a missing decision makes safe progress impossible.
- For secondary ambiguities, choose the safest reasonable default, document the assumption, and continue.
- Provider API work is out of scope for this milestone. Do not let it delay the core product.
