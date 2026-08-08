# Parade implementation plan and execution ledger

Last updated: 2026-08-08 (Asia/Shanghai)

## 2026-08-08 v1.0.0 publication execution

Status: PR #3 was merged externally as `d05f398`; Parade did not perform the
merge. The `signed-release` environment now has the two scoped signing secrets,
an exact `v1.0.0` deployment policy and a required owner review. Immutable
Releases, an active owner-audited `v*` creation/update/deletion ruleset and
full-SHA Actions pinning are enabled. The SSH-signed annotated `v1.0.0` tag
was recreated after PR #4 at `c087eae`, then removed again when the pre-signing
cross-architecture gate failed. No current tag, Release or release asset is
published.

- Release run `31251524938` attempt 1 stopped before cross-compilation because
  the synthetic 1,000-Agent load gate took 31.061 seconds against its unchanged
  30-second ceiling. The same commit passed at 15.770 seconds in PR CI and
  11.395 seconds in merged-main CI, so one complete rerun was allowed without
  weakening the threshold.
- Attempt 2 passed the same performance gate in 14.930 seconds, then exposed a
  deterministic workflow defect: unlike normal CI, the Release job did not
  build `target/debug/parade-hub` before the Playwright web server tried to run
  it. Add the missing locked Hub build and deliver the change through a focused
  PR; do not bypass the failed check or publish manually.
- After that PR is merged and merged-main CI succeeds, recreate the still
  unpublished signed tag at the new merge commit, run the complete release
  workflow, approve the isolated signing job only after `verify-build`, and
  independently download and verify the exact eight immutable assets.
- PR #4 was merged externally as `c087eae`, and merged-main CI passed. Release
  run `31253783653` then passed the complete verification suite but stopped at
  the seven-architecture gate: five `cross` containers reused host-built Cargo
  build scripts from the shared target directory and could not satisfy their
  newer glibc requirements. Use a distinct `CARGO_TARGET_DIR` for every Agent
  and Hub cross target, as recommended by the cross-rs glibc/fingerprint
  workaround. Keep all seven required Agent targets and both Hub targets; no
  Release or signing step may proceed until every isolated build succeeds.

## 2026-08-08 v1.0.0 signed Release preparation

Status: the local candidate is verified on
`codex/read-only-vps-observability`. Draft PR #2 was merged externally as
`25d8a58`; Parade did not perform the merge. The feature branch was
fast-forwarded to that default-branch commit before preparing the release
candidate.

- The repository currently has zero Releases and zero Actions Secrets. The
  signed workflow cannot publish until the operator creates and protects the
  `signed-release` environment, then provisions the scoped Ed25519 key as
  `PARADE_RELEASE_SIGNING_KEY_B64` and its independently recorded public-key
  digest as `PARADE_RELEASE_PUBLIC_KEY_SHA256`.
- Prepare semantic version `1.0.0` (tag `v1.0.0`) in the workspace, internal
  path-dependency constraints, lockfile and pinned-tag documentation.
- Run the release-proportionate Rust/frontend/browser/installer/performance/
  dependency checks, then commit and push a new draft release-preparation PR.
- Do not create the tag until the exact release-candidate commit is reachable
  from `main`; do not bypass signing, publish an unsigned GitHub Release, merge
  a PR, or generate an operator trust root without an operator-controlled
  backup.
- The final release review found and closed two publication-chain hazards:
  every workflow Action is pinned to a full commit SHA at a current Node
  24-compatible major version, and publication now compares an exact eight-file
  asset allowlist before passing the same explicit list to `gh release create`.
  The signing job also verifies the operator-pinned public-key digest, restores
  and rechecks all seven Agent executable modes after the artifact round trip,
  and runs only in `signed-release`.

Release-candidate verification on 2026-08-08:

- `cargo fmt --all -- --check`, strict all-feature Clippy and all 40 workspace
  tests passed with every local crate reporting version `1.0.0`.
- Frontend format, lint, TypeScript, five unit tests, production build and
  `npm audit --audit-level=high` passed. Eight desktop/mobile Playwright flows
  passed after supplying the existing user-space Chromium library path. The
  first browser attempt failed before test execution because this host lacks
  `libnspr4.so`; the assertions themselves did not fail.
- Shell syntax and both installer integrity scenarios passed. ShellCheck is not
  installed on this host and must be executed by the draft PR workflow.
- The installer regression suite now also proves that Linux `armv7l` selects
  the published `armv7-unknown-linux-musleabihf` target. This closed a release
  review blocker where the prior generic triple omitted the hard-float ABI
  suffix and could never find either published armv7 binary.
- Both workflow YAML files passed Prettier parsing. The extracted signing block
  passed an end-to-end disposable fixture with a one-use Ed25519 test key:
  the inner manifest was checked before signing, both inner and outer signatures
  and checksums verified again after extraction, all seven archived Agent files
  retained executable mode, and an unexpected ninth asset was rejected by the
  exact allowlist. The disposable key was never committed or uploaded and was
  disabled after the test.
- `cargo deny check` passed with only the already-explained duplicate-version
  warnings. Online `cargo audit` could not refresh RustSec because the local
  GitHub fetch failed; `cargo audit --no-fetch --stale` then scanned 1,190
  locally cached advisories and passed. The PR workflow must repeat a fresh
  online audit.
- The final deterministic performance gate passed after rebuilding the Hub with
  the corrected embedded armv7 installer: Agent 2,947,936 B, Hub 6,302,792 B,
  conservative normal traffic 5.431 MiB/Agent/30d, 1,000 reports ingested in
  10,541 ms (94.9 reports/s), fleet count query 1.037 ms, database 1,822,720 B,
  and idle Hub peak 8,540 KiB RSS / 7 FD / 5
  threads. The host used the `powersave` governor and disallowed `perf` sampling
  with `perf_event_paranoid=3`.
- Publication remains intentionally blocked: GitHub still reports zero Actions
  Secrets and zero Releases, and this unmerged candidate is not yet reachable
  from `main`. The next safe step is a reviewed draft PR, followed by operator
  protection of the `signed-release` environment and provisioning of both
  release secrets; only then may tag `v1.0.0` trigger the signed Release
  workflow. Branch/tag rules and immutable Releases are also absent in current
  GitHub repository settings and must be treated as release-governance
  limitations rather than silently claimed protections.

## 2026-08-08 resource and bandwidth hardening

Status: implementation and full pre-documentation verification are complete in
draft PR #2. PR #1 was merged externally; Parade did not perform that merge.
The bilingual documentation follow-up is being reviewed and will update the
existing draft PR #2 only from `codex/read-only-vps-observability`, never by
writing directly to the default branch.

Final independent security, traffic, UI and binary reviews are complete and
their high-risk findings are being closed before publication:

- Normal mode now hashes stable process identity/evidence rather than volatile
  CPU/RSS, while typed process/socket/live leases force bounded snapshots. The
  deterministic budget conservatively allocates bytes equivalent to one
  realistic 32-process snapshot per day; the Agent itself sends process evidence
  on stable evidence changes or typed leases rather than a daily schedule.
- IPv4 and IPv6 default routes participate in automatic interface selection;
  missing/unselected counters are explicitly partial with surfaced anomaly
  reasons and actual selected-interface names.
- A cryptographically authenticated stale-report response lets the one-item
  spool recover after long outages without weakening Hub freshness, losing
  monotonic traffic counters, growing a queue, or requiring re-enrollment.
- Filesystem inode counters are collected on supported raw-statfs targets;
  unsupported disk/inode values display as unavailable, and resource trends
  break across timestamp gaps rather than interpolating.
- Installer identity ownership, Hub/Agent co-location permissions, staged
  manifest signature verification, post-redemption failure handling, archive
  validation, and legacy-finding migration preservation are hardened.
- Finding evidence is retired to `not_observed` on a fresh relevant bounded
  snapshot and reactivated on recurrence. Fleet-wide finding/pressure totals
  are computed over the full database rather than the first page.
- Seed/adjustment mutations reload full cycle history; closed history excludes
  the current open cycle and includes operator/checkpoint audit metadata.
  Interface policies and limits remain editable after history while timezone,
  anchor and billing mode remain immutable to prevent reinterpretation.

- Read-only reviewed `~/work/kimi-rust-reality-performance/` and adopted its
  evidence discipline: exact Git/binary/machine metadata, locked normal Release
  binaries, preserved raw measurements, fixed regression ceilings, and separate
  instrumented-forensics builds. No reference file was changed and no stress,
  cgroup, process-control or network-fault script is run on monitored hosts.
- Kept the measured release profile (`opt-level=3`, Thin LTO, one codegen unit,
  stripped, panic abort). Rejected Fat LTO, `target-cpu=native`, blind PGO/BOLT,
  alternate allocators, io_uring and unsafe shortcuts because evidence does not
  justify their compatibility or complexity cost.
- Removed unused Axum WebSocket/macros, broad Tokio/tower-http features and
  Agent gzip support. Normal Agent counter persistence moved from every
  ten-second sample to a one-minute checkpoint while boot/reset/new-segment
  transitions, pending reports, ACKs and policy changes stay immediately
  durable. Same-boot recovery, reboot uncertainty and second-crash replay have
  regression tests.
- Static JS/CSS now use strong ETags and revalidation while API/HTML remain
  `no-store`, reducing repeat UI transfer without a CDN or stale private data.
- Added `scripts/performance-gate.sh` to enforce 5 MiB Agent / 16 MiB Hub
  binary ceilings, excluded dependency features, the traffic budget, the
  1,000-Agent test, and Linux idle Hub RSS. The final successful local baseline
  recorded Agent 2,947,936 B, Hub 6,302,600 B, 5.431 MiB/Agent/30d, 73.1 signed
  reports/s for 1,000 Agents, and idle Hub 8,612 KiB RSS / 7 FD / 5 threads.
  The measurement host was `powersave`, `perf_event_paranoid=3`.
- Bounded finding churn to 32 hashed subject series plus one overflow series per
  server/rule/version. Added provider-timezone traffic cycle history,
  independent directional projections, bounded resource trends, Fleet resource
  pressure/finding/traffic columns, and evidence-rich server overview.
- Release automation now verifies the tag is version-matched and reachable from
  the default branch, gives unsigned build/tests no signing secret, signs only
  in the isolated publication step, requires every advertised Agent target, and
  creates deterministic Agent archives. Hub install refuses unsafe in-place
  overwrite; Agent rotation stages all artifacts and state before committing.

## Simplified Chinese follow-up

Status: implementation and verification complete on
`codex/read-only-vps-observability`; draft PR #2 is open and remains unmerged.

- Add an explicit English / 简体中文 selector to both authenticated and login
  surfaces, persist the choice locally, honor browser language on first use,
  and localize time/status/accessibility text without changing telemetry or
  security semantics.
- Add GitHub-visible Simplified Chinese entry documentation plus an end-to-end
  operator lifecycle covering build, Hub initialization, TLS/proxy setup,
  offline Agent signing, enrollment, manual traffic seeding, observation,
  backup/restore, upgrade, revocation/deletion, troubleshooting, and verification.
- Extend unit and Playwright coverage, rebuild embedded assets, rerun applicable
  Rust/frontend/browser/security checks, review the final diff, push coherent
  commits, and update the existing draft PR without merging it.
- Implement the subsequent operator requirements with the same strict boundary:
  five closed provider billing modes, China-style dates, bounded Agent/Hub
  memory and operational retention, signed tag-driven Releases, bilingual
  interactive one-line install/uninstall, NAT/public deployment guidance, and
  a bounded evidence-only Agent-to-Hub reporting topology. The requested
  automatic peer mesh/distributed database was deliberately not implemented:
  it conflicts with the no-Agent-listener, outbound-only, one-Hub/SQLite
  architecture and would create a new control/relay attack surface.

## Bilingual documentation structure follow-up

Status: implementation, local link/command checks, and independent read-only
security re-review are complete on `codex/read-only-vps-observability`; no
documentation blocker or important issue remains before the coherent commit,
push and draft PR #2 refresh.

- Read the documentation entry points, bilingual index, getting-started and
  deployment structure under the operator-provided, read-only
  `rust-reality` reference. No reference file or script is modified or run.
- Add a paired English / Simplified Chinese documentation index and paired
  quick-start, production deployment, traffic-accounting, resource-budget and
  troubleshooting guides. Keep the existing Chinese full-lifecycle guide and
  add a complete English lifecycle counterpart instead of duplicating a second
  Chinese source of truth.
- Make both root READMEs lead first-time operators to the shortest safe path,
  then to production hardening and design/security evidence. Preserve the
  release-signature trust distinction: `curl | bash` initially trusts GitHub
  HTTPS, while downloaded payloads are verified by the pinned Ed25519 key.
- Validate local Markdown links, review commands against the shipped installer
  and configuration, run secret/boundary and diff checks, then have an
  independent read-only documentation/security review before publication.
- The review caught and closed a `releases/latest` switching race in the
  high-assurance prose by pinning one explicit tag for every download and
  `PARADE_VERSION`. It also aligned credential-rotation failure semantics,
  transient password handling, ownership-marker uninstall, high-assurance
  uninstall verification, user-writable build staging, synthetic bandwidth
  assumptions, actual timeout/concurrency claims, locked builds and current
  40-test/eight-browser-flow results. The final re-review found no remaining
  blocker or important issue and independently passed local Markdown/anchor
  checks.

### Follow-up implementation ledger

- UI locale selection, login localization, status/evidence translation, Chinese
  date formatting, and Apple-system visual treatment are implemented without
  external fonts or runtime language chunks.
- Traffic schema migration 4 and exact tests cover sum, inbound-only,
  outbound-only, maximum-direction, and separate-direction provider semantics.
  Maximum/separate seeds require both directional provider values; ambiguous
  combined input fails closed.
- Agent procfs reads, interface vectors, anomaly vectors, baseline state,
  process candidates, listeners, and pending upload are bounded. Hub pruning is
  batched and server-correlated; WAL and supplied systemd resources are capped.
  Durable audit/identity/tombstone/finding/cycle evidence remains intentionally
  persistent and is not misrepresented as zero-growth storage.
- The topology endpoint and UI expose at most 24 verified outbound reporting
  edges, classify but never expose observed source addresses, and describe
  shared sources only as possible NAT/proxy/VPN evidence. There is no scan,
  peer link, relay, tunnel, dynamic routing, or distributed data plane.
- Added signed tag-driven GitHub Release automation, bilingual public Hub and
  per-Agent installers, a local-only uninstaller, and complete Chinese
  README/full-lifecycle operations documentation. Publishing a Release still
  requires the operator-held `PARADE_RELEASE_SIGNING_KEY_B64` repository secret
  and a matching version tag; no private signing key is generated in Git.

Current local checks are complete: Rust fmt/clippy and 40 workspace tests,
frontend format/lint/typecheck/five unit tests/production build, ShellCheck,
installer integrity, eight Playwright flows with no skip, screenshots, and the
deterministic performance gate all pass. A direct
Playwright attempt first failed before launch because local Chromium could not
find `libnspr4.so`; it passed after using the prepared user-space library path.
A parallel frontend/Hub build once embedded the prior asset bundle; the required
ordered frontend-then-Hub rebuild passed. Neither environment failure is hidden.

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
All project work is on `codex/read-only-vps-observability`. The reviewed and
pushed
milestones are `23ba6a7` (specifications and architecture), `ea1d612` (signed
telemetry and traffic accounting), `4921d65` (embedded UI), and `73b55fb`
(deployment and CI), followed by `eda03da` (verification and delivery notes).
The publication follow-up `b69dc86` fills npm 10-incompatible dependency-lock
metadata without removing registry integrity hashes.
PR [#1](https://github.com/jacek4yang/parade/pull/1) was merged externally on
2026-08-02; Parade did not perform that prohibited merge. The current follow-up
will be committed and pushed on the same feature branch, then opened as a new
draft PR without merging it.

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
| 3 | Independent identity, enrollment, replay protection, web auth, migrations | Completed; migrations 1–4 and regression tests pass |
| 4 | Manual-seed traffic accounting and Agent monotonic accumulation | Completed vertical slice; rollover, restart, reset, DST/day-31, outage estimation, policy delivery, and audit tested |
| 5 | Resource/process/network collection and evidence findings | Completed vertical slice with fixture-root tests, privacy limits, interface/listener evidence, stable findings, and coverage caveats |
| 6 | Embedded Preact UI | Completed required routes, aggregate fleet lenses, dark/light/density/mobile states, temporary lease UX, and traffic workflows |
| 7 | Installer, artifact trust, services, proxy, CI, bandwidth/load/browser tests | Completed locally; cross-architecture release publication needs an operator-held offline signing key |
| 8 | Diff review, commits, push, draft PR | Completed for the implementation milestones in draft PR #2; the bilingual documentation follow-up is undergoing final review before its coherent commit/push |

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
| `cargo test --workspace --all-features` | Passed | 40 tests: Agent 13, protocol 3, Hub 24; 0 failed |
| `cargo build --release --workspace --all-features --locked` | Passed | Optimized locked current-host Hub and TLS Agent built |
| temporary-key `scripts/build-agents.sh --gnu-only` plus `openssl pkeyutl -verify` | Passed | Current-host artifact staged; SHA256SUMS signed and signature verified independently |
| `npm run format:check` | Passed | All frontend files match Prettier |
| `npm run lint` | Passed | ESLint clean |
| `npm run typecheck` | Passed | TypeScript `--noEmit` clean |
| `npm test` | Passed | 5 Vitest tests passed |
| `npm run build` | Passed | Embedded app JS 96.27 kB / 33.03 kB gzip; CSS 17.70 kB / 4.56 kB; login JS 1.96 kB / 1.14 kB |
| `npm run test:e2e` | Passed | Chromium: desktop/mobile Fleet, server evidence, actual seed POST/reload, traffic history, bad-login denial and Simplified Chinese persistence; 8 passed |
| `bash -n hub/assets/install.sh scripts/*.sh` | Passed | All supplied shell files parse |
| ShellCheck 0.10.0 (user-space Debian package) | Passed | Installer/build/test shell files clean |
| `bash scripts/test-installer-integrity.sh` | Passed | Bad manifest rejected; Agent artifacts stage before token redemption and binary/identity commit only after successful enrollment |
| `npm audit --audit-level=high` | Passed | `found 0 vulnerabilities` |
| `npx --yes npm@10.9.3 ci` | Passed | Reproduced the GitHub runner failure, then installed all 285 packages after restoring the lock's missing version/transitive/integrity metadata |
| temporary `cargo-audit 0.22.2` install | Passed | Isolated tool build completed |
| `cargo audit` | Blocked | RustSec fetch failed with `git operation failed: An IO error occurred when talking to the server`; direct shallow advisory-db clone made no progress and was interrupted after an additional bounded wait |
| `cargo deny check` | Passed | advisories, licenses and sources clean; bans clean with two explicitly reported duplicate transitive families |
| deterministic bandwidth test | Passed | 390-byte normal body plus a conservative allocation equivalent to one 3,469-byte 32-process snapshot/day; 8,640 reports/30d; 250 B/request plus 2,048 B/day TLS assumption; 5,695,110 B = 5.431 MiB/month; no daily snapshot scheduler is claimed |
| `scripts/performance-gate.sh` | Passed | Agent 2,947,936 B; Hub 6,302,600 B; dependency gate clean; Hub idle 8,612 KiB RSS, 7 FD, 5 threads, DB 266,240 B |
| synthetic fleet test | Passed | 1,000 Agents; setup 256 ms; ingest 13,681 ms; 73.1 reports/s; fleet count 4.266 ms; DB 1,822,720 bytes |
| Playwright screenshot generation | Passed | Twelve synthetic, non-sensitive English/Chinese desktop/mobile screenshots under `docs/screenshots/` |
| source boundary scan | Passed | No monitored-host execution/control implementation found; only the local installer manages its own Agent service |
| workspace secret-pattern scan | Passed | No token/private-key/cloud-key/bearer/password candidates found outside explicit invalid examples/tests |
| initial `git status --short --branch`, `git remote -v`, `git branch --show-current` | Blocked | Initial delivered workspace had no `.git`; no history or origin was invented before the operator created the repository |
| initial `gh auth status` | Failed | Active account token was reported invalid at kickoff |
| final `gh auth status` | Passed | Active `jacek4yang`, SSH Git protocol, `repo` scope present |
| `gh repo view jacek4yang/parade ...` | Passed | Exact intended public repository exists, was empty, and grants `ADMIN`; SSH origin verified |
| final Git state verification | Passed | `origin` is `git@github.com:jacek4yang/parade.git`; empty `main` baseline; current branch `codex/read-only-vps-observability` |
| earlier `GH_PROMPT_DISABLED=1 gh pr create --draft ... --body-file PR_BODY.md` | Blocked | Before repository creation it failed with `fatal: not a git repository`; it was retried successfully after the final feature push |
| `git diff --check origin/main...HEAD`, final secret scan, refined host-control scan, and `git fsck --full` | Passed | Clean worktree/diff and no secret or remote-control candidates; Git objects valid (one harmless dangling blob from an earlier unstaged edit) |
| `git push -u origin codex/read-only-vps-observability` | Passed | Feature branch created on the exact intended origin; local upstream now tracks that feature branch |
| `gh pr create --draft --base main --head codex/read-only-vps-observability ...` | Passed | Created https://github.com/jacek4yang/parade/pull/1 |
| `gh pr view 1 ...` | Passed | PR is `OPEN`, draft, base `main`, head `codex/read-only-vps-observability`; initial CI run entered the queue |
| GitHub Actions CI run `30727221592` | Passed | 5m53s: npm clean install/audit/format/lint/typecheck/unit/build, Rust fmt/clippy/tests/Hub build, Chromium install/E2E, shell syntax/installer test/ShellCheck, and RustSec `cargo audit` all succeeded |
| GitHub Actions CI run `31247485992` | Passed | Draft PR #2 pre-documentation head: frontend audit/format/lint/typecheck/unit/build, Rust fmt/clippy/40 tests/performance gate/Hub build, 8 Playwright flows, installer syntax/integrity/ShellCheck, and RustSec `cargo audit` all completed successfully |

Unavailable local tools were not claimed as passed: Gitleaks, `sqlite3`, and
`nginx`. The equivalent workspace secret-pattern scan is clean. Local
`cargo-audit` installed in an isolated prefix, but its advisory database fetch
was network-blocked; GitHub Actions run `31247485992` independently completed
RustSec `cargo audit` successfully. `nginx -t` and real service installation
remain checks for a disposable deployment host. `cargo deny check` passed
locally with two reported duplicate transitive families.

## Measured outcomes

- Normal-profile upload estimate: **5.431 MiB per Agent per 30 days**, using a
  conservative synthetic allocation equivalent to one realistic 32-process
  snapshot per day and remaining under the 10 MiB target and 20 MiB hard
  ceiling. The Agent has no daily snapshot scheduler; process/listener evidence
  changes, genuine event bursts and explicit detail leases contribute according
  to actual activity, with lease response bodies measured separately.
- Synthetic Hub load: **1,000 signed Agent reports in 13.681 s** (**73.1/s**),
  count query **4.266 ms**, database **1,822,720 bytes**. This is deterministic local
  SQLite ingestion, not a claim about full HTTP/TLS capacity.
- Idle release Hub: **8,612 KiB peak RSS**, **7 FDs**, **5 threads**; release
  binaries are Agent **2,947,936 B**, Hub **6,302,600 B**.
- Embedded app JS+CSS: **37.59 kB gzip combined**, with a 1.14 kB gzip login
  bundle; all assets are local.

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
- Resource trends currently expose the latest 72 retained rollups rather than
  multi-year downsampling. Fleet saved views/sort/column controls, event query
  filters, and finding acknowledgement/suppression mutation UX remain deferred.
- Fleet-wide finding and pressure totals query all servers, but the interactive
  attention queue remains limited to the first 500 fleet records. Closed-cycle
  provider adjustments, per-server Fleet traffic projections and a first-class
  accounting-epoch transition API remain deferred. After seeded history,
  interface policies/limits remain editable while timezone, anchor and billing
  mode stay frozen to prevent historical reinterpretation.
- Process/socket association, remote endpoint aggregation, package ownership
  queries, persisted uptime/boot inventory, per-core/disk-I/O/conntrack/
  clock-skew collectors, armv7 statfs, and actual journal access need additional
  distro fixtures and disposable-host validation.
- Traffic tests cover IPv4/IPv6 defaults, rename continuity, 32-bit wrap,
  formulas, seeds, corrections, rollover, crash replay and monotonic storage.
  Bridge/member topology fixtures, concurrent HTTP-ingest assertions and broad
  property tests remain follow-up work.
- HTTP handlers currently call synchronous rusqlite operations. Transactions
  are short and rate-limited, but a future full-stack load pass should use
  bounded blocking workers and measure staggered/synchronized reconnects,
  finding bursts, memory, and end-to-end latency.
- External alert destinations are deliberately absent; findings and events are
  in-console only. Provider integrations are intentionally out of scope.
- The installer and systemd hardening are syntax/integrity tested, but an
  actual unprivileged install on each supported distro/architecture was not
  possible without root/disposable VPS access. Publishing the first signed
  Release needs repository secret `PARADE_RELEASE_SIGNING_KEY_B64` and a
  version-matched tag reachable from the default branch.
- Root/kernel-compromised monitored hosts can falsify or hide all local
  telemetry. Signatures authenticate the Agent identity, not host truth.

## Publication state

- Origin: `git@github.com:jacek4yang/parade.git`
- Base: `main` at externally merged PR #1 commit `1542f8b`
- Head: `codex/read-only-vps-observability`
- Follow-up commits: `bd9d570` telemetry/accounting, `bb40322` bilingual UI,
  `ac58d7a` signed release/deployment, `2ccf7d6` delivery ledger; the bilingual
  documentation commit is being reviewed before push
- Draft PR body: `PR_BODY.md`
- Previous PR: https://github.com/jacek4yang/parade/pull/1 (externally merged)
- Current draft PR: https://github.com/jacek4yang/parade/pull/2
- Merge state: prohibited; the new delivery will remain draft pending
  disposable-host validation.

GitHub Actions run
[31247485992](https://github.com/jacek4yang/parade/actions/runs/31247485992)
passed on the draft PR #2 pre-documentation head, including the performance gate,
browser suite, installers, ShellCheck and RustSec. A new run will be required
after the documentation commit; it will not be claimed early.
