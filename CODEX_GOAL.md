# CODEX_GOAL.md

## Assignment

Transform the current Parade repository into a secure, polished, low-bandwidth, strictly read-only Linux VPS fleet observability and security-posture platform.

Work autonomously from repository audit through implementation, verification, Git commits, push, and draft GitHub pull request.

The Linux development server already has GitHub CLI authentication. Verify it with `gh auth status` and use the existing intended `origin`.

Do not stop after writing a plan.

## Required source documents

Read these files completely before major design or implementation work:

- `AGENTS.md`
- `TRAFFIC_ACCOUNTING_SPEC.md`
- `UI_SPEC.md`

Treat them as normative.

Create and maintain:

- `PLANS.md`
- `AUDIT.md`
- `ARCHITECTURE.md`
- `THREAT_MODEL.md`
- `MIGRATION.md`
- `SECURITY.md`

Update `README.md`.

## Current repository context

The existing repository is expected to contain approximately:

- a Rust workspace;
- `agent`, `hub`, and `common` crates;
- Axum-based Hub code;
- a Rust Agent;
- TOML example configuration;
- systemd units;
- shell install/build scripts;
- embedded HTML assets;
- JSON-based state;
- nginx examples.

Verify the actual repository rather than relying blindly on this summary.

## Known issues to verify

Audit the code and confirm or refute each point with file/line references:

1. Empty or disabled dashboard authentication may expose management mutations.
2. Agent installation may create a root-only config while systemd uses `DynamicUser`, preventing Agent startup.
3. HTTPS enrollment may conflict with Agent binaries built without TLS support.
4. A fleet-wide Agent token may allow one compromised server to impersonate another.
5. Deleted servers may reappear through unknown-server auto-registration.
6. Online/offline state, replay state, and traffic state may not survive Hub restarts correctly.
7. JSON-only state may be fragile and difficult to migrate transactionally.
8. A global mutex may be held over serialization, disk I/O, networking, or alert work.
9. Fixed-frequency reports may synchronize the fleet and waste traffic.
10. Full-fleet WebSocket snapshots may scale poorly.
11. Alert queues or background work may be unbounded.
12. Installer artifact integrity verification may be insufficient.
13. Installer self-tests may ignore failures.
14. Automated tests, CI, load tests, bandwidth tests, and security checks may be insufficient.

Record verified findings in `AUDIT.md` with:

- severity;
- affected code;
- exploit/failure scenario;
- chosen remediation;
- test that prevents regression.

## Product scope

### In scope

- secure Agent enrollment;
- independent Agent identity;
- authenticated, replay-resistant reports;
- resource observability;
- privacy-preserving process observability;
- network/socket observability;
- security-log summaries where permitted;
- transparent security findings;
- low-bandwidth protocol;
- durable traffic accounting using manual current-cycle seeds;
- automatic billing-cycle rollover;
- minimal configuration;
- scalable Fleet UI;
- server detail views;
- alerts;
- audit logs;
- migrations;
- tests;
- CI;
- GitHub draft PR.

### Explicitly out of scope

- provider API integration;
- DMIT API integration;
- BandwagonHost/KiwiVM API integration;
- arbitrary remote execution;
- remote shell;
- remote file management;
- package/service/process/firewall/user control;
- VPS reboot/shutdown/reinstall;
- automatic remediation;
- multi-tenant SaaS billing;
- Kubernetes support unless already trivial and non-disruptive.

Do not spend the milestone on provider APIs. The manual seed and local accumulation model is the required solution.

## Product boundary

Enforce the read-only boundary structurally, not merely through documentation.

The Agent may write only:

- its own identity;
- configuration;
- monotonic counter checkpoints;
- bounded event spool;
- logs;
- temporary files in its own private runtime/state directories.

No Hub message may contain executable text.

Use closed enums for read-only observation leases.

Add tests proving that no API or message type can request arbitrary code execution or monitored-host mutation.

## Target deployment

### Hub

- one self-contained Rust binary;
- embedded web assets;
- embedded SQLite database in WAL mode;
- localhost bind by default;
- reverse-proxy/TLS deployment examples;
- minimal initialization;
- no mandatory external services.

### Agent

- one Rust binary per target architecture;
- outbound-only TLS;
- no listening socket;
- dedicated unprivileged user;
- hardened systemd unit;
- minimal configuration;
- one-command enrollment;
- bounded local state and spool;
- artifact checksum/signature verification.

### Supported environments

Prioritize common Linux VPS distributions and kernels:

- Debian;
- Ubuntu;
- AlmaLinux/Rocky Linux;
- other systemd-based distributions where feasible.

Collectors must detect unsupported features gracefully.

## Phase 0: repository and Git safety

Before editing:

1. Run and inspect:

```bash
pwd
git status --short --branch
git remote -v
git branch --show-current
gh auth status
```

2. Identify the default branch without assuming its name.
3. Refuse to edit directly on the default branch.
4. Create or switch to:

```text
codex/read-only-vps-observability
```

5. Preserve pre-existing user changes. Do not overwrite or discard them.
6. Record the initial state in `PLANS.md`.

If the repository has no `origin`, do not create an arbitrary public repository. Report the blocker after completing safe local work.

## Phase 1: verified audit and architecture

Inspect all source, scripts, service files, configuration, and frontend assets.

Create:

### `AUDIT.md`

Verified defects and risks.

### `THREAT_MODEL.md`

Cover:

- stolen enrollment token;
- stolen Agent credential;
- one compromised VPS;
- replay;
- cross-server impersonation;
- malicious Hub user;
- reverse-proxy spoofing;
- database theft;
- installer/artifact compromise;
- telemetry poisoning;
- secret leakage through process metadata;
- root/kernel compromise;
- denial of service from many Agents;
- alert flooding.

### `ARCHITECTURE.md`

Document:

- component boundaries;
- trust boundaries;
- identity;
- enrollment;
- protocol;
- persistence;
- collectors;
- traffic accounting;
- security finding engine;
- UI data flow;
- retention;
- failure recovery;
- upgrade/migration;
- read-only enforcement.

Do not stop here. Continue implementation.

## Phase 2: secure identity and enrollment

Replace any global Agent credential with per-Agent identity.

Required flow:

1. Operator creates a server record in the Hub.
2. Hub creates a short-lived, single-use enrollment token.
3. Installer downloads a supported Agent artifact.
4. Installer verifies checksum and signature against a pinned public key or another robustly justified trust model.
5. Agent creates its independent credential or signing key locally.
6. Enrollment binds the Agent identity to one server ID.
7. The Hub returns only the configuration needed by that Agent.
8. Enrollment token is consumed atomically.
9. Agent credential can be rotated and revoked.
10. Deleted/revoked servers cannot re-enroll silently.

Reports must include:

- protocol version;
- server ID;
- Agent identity;
- timestamp;
- monotonic sequence;
- unique message ID;
- authenticated body.

Reject:

- replay;
- stale timestamp beyond policy;
- sequence rollback;
- wrong server binding;
- unknown Agent;
- revoked Agent;
- malformed encoding;
- unsupported protocol;
- oversized payload;
- wrong content type.

Use constant-time comparisons and suitable cryptographic libraries.

## Phase 3: systemd and installer correctness

Create a secure operating model.

Agent requirements:

- dedicated system user or a correctly designed `DynamicUser` configuration;
- config and state readable/writable exactly as required;
- no root at steady state;
- no inbound socket;
- strict `ProtectSystem`, `ProtectHome`, `PrivateTmp`, `NoNewPrivileges`, capability, device, namespace, and writable-path settings where compatible;
- narrowly documented optional journal/log group access;
- graceful collector permission failures.

Installer requirements:

- `set -euo pipefail`;
- explicit supported architecture detection;
- HTTPS only for artifact download;
- checksum and signature verification;
- atomic install;
- correct ownership/mode;
- actual Agent version/self-test command;
- systemd start verification;
- timeout and useful error output;
- no success message when the service failed;
- idempotent rerun;
- no embedded production secret.

Hub service must follow equivalent hardening appropriate to its database and listener.

## Phase 4: persistence

Replace fragile JSON-only mutable state with SQLite WAL unless repository evidence proves a better embedded transactional solution.

Persist:

- server registry;
- tombstones/revocations;
- Agent identity and replay state;
- enrollment tokens;
- sessions;
- audit events;
- inventory;
- resource rollups;
- process summaries/snapshots;
- network/socket summaries;
- security events/findings;
- traffic interface state;
- monotonic traffic checkpoints;
- billing-cycle rules and instances;
- traffic seeds and adjustments;
- alerts;
- schema version.

Requirements:

- explicit migrations;
- transactions;
- foreign keys;
- indexes for Fleet, findings, events, and time ranges;
- bounded retention;
- downsampling;
- documented backup/restore;
- migration from current state where possible;
- atomic handling of reports and traffic checkpoints;
- no global lock across I/O.

Add migration tests.

## Phase 5: low-bandwidth protocol

Create a compact, versioned, typed protocol.

Possible message groups:

- enrollment;
- heartbeat and rollup;
- inventory hash/update;
- process summary;
- process snapshot;
- socket summary/snapshot;
- security event batch;
- traffic checkpoint;
- observation lease;
- acknowledgment/error.

Normal profile target:

- local 10-second sampling;
- 5-minute rollup upload with jitter;
- inventory daily or on hash change;
- top-N/changed process facts only;
- security-event batch only when events exist;
- no continuous full connection/process streaming;
- bounded retry and exponential backoff;
- connection reuse;
- bounded payload and spool.

Create a deterministic bandwidth model/test.

Report:

- encoded body bytes;
- estimated request/TLS overhead under documented assumptions;
- monthly default total;
- event/live-detail exclusions.

Target:

- less than 10 MiB/month upload per idle/default Agent in the synthetic benchmark;
- fail the regression test above 20 MiB/month.

Use measured tradeoffs, not guessed claims.

## Phase 6: traffic accounting

Implement `TRAFFIC_ACCOUNTING_SPEC.md` completely.

The required operator workflow is:

1. Configure the billing cycle and selected interfaces.
2. Enter current provider-used traffic.
3. Store an immutable seed tied to the latest reliable Agent checkpoint.
4. Add subsequent Agent-observed deltas.
5. Roll to the next cycle automatically.
6. Allow audited corrections.
7. Show source, components, timestamps, and confidence.

Provider APIs are not required.

Key acceptance cases:

- no loss across Hub restart;
- no negative delta after reboot/reset;
- no double counting;
- no silent history rewrite;
- scheduled rollover;
- correct day-31 behavior;
- explicit uncertainty for an unsplittable boundary interval;
- interface selection avoids obvious virtual-interface double counting.

## Phase 7: resource and process collectors

Add collectors with fixture-based tests.

### Resource data

Where supported:

- CPU total/per-core;
- load average;
- CPU/memory/I/O PSI;
- memory/cache/swap;
- OOM events;
- filesystem capacity;
- inode usage;
- disk I/O;
- network bytes/packets/errors/drops;
- TCP state counts;
- conntrack pressure;
- uptime;
- boot ID;
- clock skew;
- OS/kernel/architecture;
- virtualization/container hints;
- Agent version;
- collector coverage.

### Process data

Privacy-preserving fields:

- PID;
- PPID;
- UID/user;
- state;
- start time;
- elapsed time;
- CPU;
- RSS;
- virtual memory;
- executable basename/path when readable;
- cgroup/container/systemd unit;
- listening/open socket counts;
- deleted executable marker;
- suspicious writable-location marker;
- package ownership state when available.

Do not collect environment variables.

Do not periodically transmit every process.

Use:

- top-N summaries;
- changed process facts;
- suspicious process facts;
- typed temporary full snapshots.

Apply payload limits and stable sorting.

## Phase 8: security event and finding engine

Implement transparent rules and baselines.

Initial high-value findings:

- repeated SSH authentication failures;
- successful root login;
- new login source;
- unusual sudo/authentication event;
- new listening port;
- executable from `/tmp`, `/var/tmp`, `/dev/shm`, or another suspicious writable path;
- deleted executable still running;
- new privileged process;
- newly observed executable;
- sustained unusual CPU compatible with cryptomining, labeled explicitly as heuristic;
- abnormal outbound connection volume;
- OOM;
- kernel oops/panic signal;
- filesystem read-only/error signal;
- repeated service crash signal where logs permit;
- changed cron/systemd inventory where safely visible;
- telemetry coverage reduction;
- clock skew;
- reporting gap.

Each finding must expose evidence and confidence.

Do not implement automatic remediation.

Deduplicate and group recurring events.

Use bounded queues and alert rate limits.

## Phase 9: authentication and web security

- No unauthenticated mutation mode.
- Empty/missing administration secret must trigger secure setup or refuse unsafe startup.
- Hash passwords with Argon2id or a justified equivalent.
- Use Secure/HttpOnly/SameSite cookies.
- Add CSRF protection for mutations.
- Add session expiry and revocation.
- Trust forwarded headers only from configured proxy CIDRs/addresses.
- Rate-limit login, enrollment, Agent reports, and expensive detail endpoints with bounded state.
- Enforce content types, size limits, timeouts, CSP, frame protection, and safe cache behavior.
- Provide HSTS guidance at the TLS proxy.
- Add user-visible audit logs.

The read-only boundary applies to monitored VPSes. Hub-side metadata, acknowledgments, tags, seeds, and settings may be modified by authenticated operators and must be audited.

## Phase 10: UI redesign

Implement `UI_SPEC.md`.

Required pages:

- Overview;
- Fleet;
- Security;
- Traffic;
- Events;
- Settings;
- server Overview;
- Resources;
- Processes;
- Network;
- Security;
- Events;
- Traffic;
- Inventory.

Requirements:

- attractive and restrained;
- dark/light themes;
- desktop/mobile;
- accessible;
- no external assets;
- responsive large Fleet table;
- explicit freshness and coverage;
- evidence-based security UX;
- transparent traffic seed/observed/adjustment breakdown;
- no monitored-host action controls;
- typed temporary live-detail request with visible expiry and bandwidth notice.

Add Playwright tests and representative screenshots.

## Phase 11: alerts

Preserve or improve current alert destinations only after core correctness.

Requirements:

- bounded queues;
- bounded worker concurrency;
- deduplication;
- cooldown;
- retry with backoff;
- no lock held over network delivery;
- no secret logging;
- rate limits during fleet-wide incidents;
- testable message formatting.

Large Telegram/Fleet outputs must be paginated or summarized.

## Phase 12: scale and reliability

Add synthetic testing.

At minimum:

- 1,000 synthetic Agents;
- staggered and synchronized reconnect scenarios;
- Hub restart;
- database checkpoint/retention;
- large Fleet UI data;
- finding burst;
- alert burst;
- report replay/flood;
- network outage/recovery.

Measure:

- report latency;
- throughput;
- memory;
- database growth;
- queue depth;
- error rate;
- UI responsiveness;
- bandwidth.

Attempt 5,000 Agents if the development server has sufficient resources, but do not destabilize the host.

Use jitter to avoid fleet thundering herds.

## Phase 13: documentation

Update/add:

- `README.md`
- `AUDIT.md`
- `ARCHITECTURE.md`
- `THREAT_MODEL.md`
- `SECURITY.md`
- `MIGRATION.md`
- `PLANS.md`

README must include:

- precise product definition;
- strict read-only boundary;
- architecture diagram;
- quick Hub setup;
- one-command Agent enrollment;
- manual current-cycle traffic seed workflow;
- cycle rollover behavior;
- unsupported pre-observation history;
- security-telemetry limitations under root/kernel compromise;
- backup/restore;
- upgrade;
- uninstall;
- troubleshooting;
- resource/bandwidth expectations;
- supported distributions/architectures;
- screenshot(s).

Do not call Parade a generic remote-control plane.

## Required tests

At minimum prove:

1. One Agent cannot report for another server.
2. Replayed/stale/cross-server/oversized/malformed reports are rejected.
3. Enrollment token is short-lived, single-use, and bound.
4. Revoked/deleted Agent cannot resurrect.
5. Empty admin setup cannot expose mutation APIs.
6. Untrusted proxy headers are ignored.
7. Hub restart preserves identities, replay state, traffic, findings, and tombstones.
8. Traffic seed plus observed delta produces the correct total.
9. Reboot/counter reset cannot create negative or absurd traffic.
10. Billing cycle rolls automatically.
11. Day-31 and timezone behavior are correct.
12. Interface selection avoids obvious double counting.
13. Unsplittable intervals are marked estimated.
14. Agent service can read its own configuration and state.
15. Agent can run unprivileged.
16. TLS-enabled artifacts are built.
17. Artifact verification failure aborts installation.
18. Queues, payloads, spools, and retention are bounded.
19. Default bandwidth remains under target.
20. 1,000 synthetic Agents operate without unbounded latency/memory growth.
21. Process environment/secrets are not exposed.
22. No API/protocol/UI path can execute code or mutate monitored hosts.
23. UI works at desktop and approximately 390-pixel mobile width.
24. Database migrations are reversible where promised or fail safely.
25. Existing supported behavior has regression coverage.

## Required verification commands

Run all applicable checks and record exact output summaries:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Also run the frontend formatter, lint, typecheck, unit tests, production build, Playwright tests, shell syntax checks, ShellCheck if installed, dependency audits if installed, bandwidth test, load test, and secret scan.

If a tool is unavailable:

- do not claim it passed;
- install it only when safe and appropriate;
- otherwise record the missing tool and exact remaining command.

## Commit strategy

Use coherent commits such as:

1. audit and architecture;
2. secure identity/enrollment;
3. transactional persistence/migrations;
4. traffic accounting;
5. low-bandwidth protocol/collectors;
6. security findings;
7. UI redesign;
8. tests/CI/docs.

The exact split may vary, but avoid one opaque mega-commit when practical.

Before each commit:

```bash
git status --short
git diff --check
git diff
```

Do not commit:

- secrets;
- private keys;
- generated production credentials;
- real server IPs/domains;
- database state;
- build outputs unless repository policy requires them;
- screenshots containing sensitive data.

## GitHub delivery

After full review and required checks:

1. Push `codex/read-only-vps-observability` to the existing `origin`.
2. Open a draft PR using `gh pr create --draft`.
3. Do not merge.

PR body must contain:

- product boundary;
- verified original issues;
- architecture;
- threat-model summary;
- traffic-accounting semantics;
- migration;
- test commands/results;
- bandwidth result;
- load-test result;
- screenshots;
- known limitations;
- deferred work;
- exact manual testing steps on disposable VPSes.

## Completion definition

The task is complete only when:

- P0 security defects are fixed;
- the product remains structurally read-only toward monitored hosts;
- independent Agent identity and replay defense work;
- persistence is transactional;
- manual traffic seed plus automatic accumulation and cycle rollover work;
- core resource/process/network/security views work;
- the redesigned UI is usable;
- required tests pass or exact blockers are documented;
- commits exist on the feature branch;
- the branch is pushed;
- a draft PR exists when GitHub access permits;
- the final response reports branch, commits, PR URL, tests, bandwidth, load results, screenshots, limitations, and blockers.

Do not claim completion from documentation or mockups alone.
