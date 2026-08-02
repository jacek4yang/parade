# Parade implementation plan and execution ledger

Last updated: 2026-08-02 (Asia/Shanghai)

## Delivery state

The specifications were read completely before design work: `AGENTS.md`,
`CODEX_GOAL.md`, `TRAFFIC_ACCOUNTING_SPEC.md`, and `UI_SPEC.md`. The repository
was audited from its delivered Rust, frontend, shell, systemd, configuration,
and nginx sources through the current implementation.

Local implementation and verification are complete for the secured vertical
slice. The operator subsequently created the exact intended empty repository,
`jacek4yang/parade`, and explicitly asked publication to begin. GitHub CLI
authentication is valid for that account with `repo` scope and SSH Git access.

The empty remote was bootstrapped without developing on the default branch:
commit `1d48c80` contains no project files and is the sole `main` baseline.
All project work is on `codex/read-only-vps-observability`. The reviewed local
milestones are `23ba6a7` (specifications and architecture), `ea1d612` (signed
telemetry and traffic accounting), `4921d65` (embedded UI), and `73b55fb`
(deployment and CI). Final ledger/PR material is being prepared before the
feature branch is pushed and the draft PR is opened; nothing will be merged.

## Product invariants

1. Monitored VPSes are read-only observation targets. There is no arbitrary
   command, host mutation, process/service/package/firewall/user/file control,
   provider write action, or automatic remediation path.
2. Hub-to-Agent requests are a closed versioned enum of bounded, expiring
   observation profiles. Each request and matching response is audited; response
   bytes are measured; unknown/mismatched profiles fail closed.
3. Every Agent has a server-bound Ed25519 identity. Enrollment is short-lived,
   single-use, and transactional; signed reports are ordered, fresh, unique,
   size-bounded, replay-safe, revocable, and durable across restart.
4. The Hub uses SQLite/WAL with foreign keys, immediate security/accounting
   transactions, explicit migrations, bounded retention, and no global state
   lock around database/network/serialization work.
5. Manual provider usage is an immutable checkpoint-tied seed. Current usage is
   seed plus subsequent monotonic selected-interface deltas plus append-only
   adjustments. Calendar rollover is automatic, history is retained, and
   unsplittable intervals remain explicitly estimated.
6. Normal telemetry is compact, jittered, privacy-preserving, and bounded.
   Process environments and full command lines are never collected.

## Execution plan

| Phase | Scope | Result |
| --- | --- | --- |
| 0 | Verify workspace, Git, origin/default branch, and GitHub auth | Completed; exact empty origin is `jacek4yang/parade`, default branch is the empty `main` baseline, CLI auth is valid, and work is isolated on the required feature branch |
| 1 | Repository-wide defect audit | Completed in `AUDIT.md`; all 14 known issues verified and remediated or explicitly bounded |
| 2 | Architecture, threat model, security, migration, and operational docs | Completed in `ARCHITECTURE.md`, `THREAT_MODEL.md`, `SECURITY.md`, `MIGRATION.md`, and `README.md` |
| 3 | Independent identity, enrollment, replay protection, web auth, migrations | Completed; migrations 1–3 and regression tests pass |
| 4 | Manual-seed traffic accounting and Agent monotonic accumulation | Completed vertical slice; rollover, restart, reset, DST/day-31, outage estimation, policy delivery, and audit tested |
| 5 | Resource/process/network collection and evidence findings | Completed vertical slice with fixture-root tests, privacy limits, interface/listener evidence, stable findings, and coverage caveats |
| 6 | Embedded Preact UI | Completed required routes, aggregate fleet lenses, dark/light/density/mobile states, temporary lease UX, and traffic workflows |
| 7 | Installer, artifact trust, services, proxy, CI, bandwidth/load/browser tests | Completed locally; cross-architecture release publication needs an operator-held offline signing key |
| 8 | Diff review, commits, push, draft PR | File/security review and coherent local milestone commits completed; feature-branch push and draft PR creation are in progress |

Independent read-heavy reviews were performed by architecture, security, and
UI/test subagents. Their high-severity findings (interface-policy enforcement,
timestamp/rotation/rollover races, revocation races, loopback parsing, proxy
spoofing, limiter abuse, artifact signing/write access, fleet counts, stale
status, traffic form corruption, mobile navigation, and shallow Playwright
coverage) were addressed. Residual findings are listed below and in
`AUDIT.md`.

## Verification ledger

Only commands that actually ran are recorded as passed.

| Command | Result | Exact summary |
| --- | --- | --- |
| `cargo fmt --all -- --check` | Passed | Workspace formatting clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Passed | No warnings |
| `cargo test --workspace --all-features` | Passed | 22 tests: Agent 5, protocol 3, Hub 14; 0 failed |
| `cargo build -p parade-agent --release --all-features` | Passed | Optimized current-host Agent built with TLS features enabled |
| temporary-key `scripts/build-agents.sh --gnu-only` plus `openssl pkeyutl -verify` | Passed | Current-host artifact staged; SHA256SUMS signed and signature verified independently |
| `npm run format:check` | Passed | All frontend files match Prettier |
| `npm run lint` | Passed | ESLint clean |
| `npm run typecheck` | Passed | TypeScript `--noEmit` clean |
| `npm test` | Passed | 2 Vitest tests passed |
| `npm run build` | Passed | Embedded JS 47.01 kB raw / 15.73 kB gzip; CSS 14.51 kB / 3.79 kB gzip; combined 61.52 kB raw / 19.52 kB gzip |
| `npm run test:e2e` | Passed | Chromium: authenticated desktop/mobile Fleet, representative server/process/network/security/traffic views, bad-login denial; 5 passed, 1 deliberate non-desktop visual skip |
| `bash -n hub/assets/install.sh scripts/*.sh` | Passed | All supplied shell files parse |
| ShellCheck 0.10.0 (user-space Debian package) | Passed | Installer/build/test shell files clean |
| `bash scripts/test-installer-integrity.sh` | Passed | Bad manifest rejected before execution; existing binary is not replaced before enrollment succeeds |
| `npm audit --audit-level=high` | Passed | `found 0 vulnerabilities` |
| temporary `cargo-audit 0.22.2` install | Passed | Isolated tool build completed |
| `cargo audit` | Blocked | RustSec fetch failed with `git operation failed: An IO error occurred when talking to the server`; direct shallow advisory-db clone made no progress and was interrupted after an additional bounded wait |
| deterministic bandwidth test | Passed | 390-byte encoded body; 8,640 reports/30d; 250 B/request plus 2,048 B/day TLS assumption; 5,591,040 B = 5.332 MiB/month |
| synthetic fleet test | Passed | 1,000 Agents; setup 212 ms; ingest 9,636 ms; 103.8 reports/s; fleet count 0.888 ms; DB 1,761,280 bytes |
| Playwright screenshot generation | Passed | Seven synthetic, non-sensitive screenshots under `docs/screenshots/` |
| source boundary scan | Passed | No monitored-host execution/control implementation found; only the local installer manages its own Agent service |
| workspace secret-pattern scan | Passed | No token/private-key/cloud-key/bearer/password candidates found outside explicit invalid examples/tests |
| initial `git status --short --branch`, `git remote -v`, `git branch --show-current` | Blocked | Initial delivered workspace had no `.git`; no history or origin was invented before the operator created the repository |
| initial `gh auth status` | Failed | Active account token was reported invalid at kickoff |
| final `gh auth status` | Passed | Active `jacek4yang`, SSH Git protocol, `repo` scope present |
| `gh repo view jacek4yang/parade ...` | Passed | Exact intended public repository exists, was empty, and grants `ADMIN`; SSH origin verified |
| final Git state verification | Passed | `origin` is `git@github.com:jacek4yang/parade.git`; empty `main` baseline; current branch `codex/read-only-vps-observability` |
| earlier `GH_PROMPT_DISABLED=1 gh pr create --draft ... --body-file PR_BODY.md` | Blocked | Before repository creation it failed with `fatal: not a git repository`; draft creation will be retried after the final feature push |

Unavailable tools were not claimed as passed: `cargo-deny`, `gitleaks`,
`sqlite3`, and `nginx`. `cargo-audit` installed in an isolated prefix, but its
advisory database fetch was network-blocked, so no RustSec result is claimed.
Remaining commands are `cargo audit`, `cargo deny check`, an equivalent
Gitleaks workspace scan, and `nginx -t` after installing the example on a
disposable proxy host. CI installs and runs ShellCheck and `cargo audit` on a
standard Ubuntu runner.

## Measured outcomes

- Normal-profile upload estimate: **5.332 MiB per Agent per 30 days**, under the
  10 MiB target and 20 MiB hard ceiling. It excludes genuine event bursts and
  explicit detail leases; those lease response bodies are measured separately.
- Synthetic Hub load: **1,000 signed Agent reports in 9.636 s** (**103.8/s**),
  count query **0.888 ms**, database **1.68 MiB**. This is deterministic local
  SQLite ingestion, not a claim about full HTTP/TLS capacity.
- Frontend JS+CSS: **19.52 kB gzip combined**, far below the 500 KiB ceiling.

## Safe defaults chosen

- UTC, day 1, 00:00 is the initial billing rule. Day 29–31 clamps to the last
  day and DST gaps/ambiguity are handled deterministically.
- Interface auto-selection follows Linux default routes and excludes loopback,
  bridges, veth/container, and tunnel-like devices. Overrides are versioned and
  delivered to the Agent in enrollment/report acknowledgements.
- Hub binds to `127.0.0.1`; remote public origins require HTTPS and Secure
  cookies. Forwarded identity is accepted only from exact configured proxy IPs.
- Detailed rollups retain 30 days, traffic rollups 90 days, process/socket
  changes seven days plus latest, events 180 days, and raw checkpoints 400 days
  while preserving seed-tied/latest checkpoints. Security/audit/cycle history
  is durable.

## Known limitations and deferred work

- Authentication/journal summaries and rules for SSH/root/sudo, OOM/kernel/
  filesystem/service-crash signals are not yet implemented. Coverage is
  explicit; Parade never claims a server is clean or safe.
- The resource UI is a latest-rollup vertical slice rather than synchronized
  long-term charts. Hourly two-year downsampling, traffic cycle-history tables,
  Fleet saved views/sort/column controls, event query filters, and finding
  acknowledgement/suppression mutation UX remain deferred.
- Process/socket association, remote endpoint aggregation, package ownership
  queries, per-core/disk-I/O/conntrack/clock-skew collectors, and actual journal
  access need additional distro fixtures and disposable-host validation.
- HTTP handlers currently call synchronous rusqlite operations. Transactions
  are short and rate-limited, but a future full-stack load pass should use
  bounded blocking workers and measure staggered/synchronized reconnects,
  finding bursts, memory, and end-to-end latency.
- External alert destinations are deliberately absent; findings and events are
  in-console only. Provider integrations are intentionally out of scope.
- The installer and systemd hardening are syntax/integrity tested, but an
  actual unprivileged install on each supported distro/architecture was not
  possible without root/disposable VPS access. Cross-target signed release
  staging needs the operator's offline Ed25519 key.
- Root/kernel-compromised monitored hosts can falsify or hide all local
  telemetry. Signatures authenticate the Agent identity, not host truth.

## Publication state

- Origin: `git@github.com:jacek4yang/parade.git`
- Base: `main` at the intentionally empty initialization commit `1d48c80`
- Head: `codex/read-only-vps-observability`
- Draft PR body: `PR_BODY.md`
- Merge state: prohibited; this delivery remains draft pending disposable-host
  validation.

The remaining publication operations are a final diff/secret review, feature
branch push, draft PR creation, and read-back verification of its base, head,
draft state, and URL.
