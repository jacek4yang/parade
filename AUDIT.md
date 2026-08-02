# Repository audit

This audit records the repository as delivered on 2026-08-02 and the current
remediation. Historical line references identify the inspected pre-refactor
files; those implementations were replaced. Regression tests name the current
test that enforces the invariant.

| Severity | Verified defect and affected delivered code | Failure scenario | Remediation and regression coverage |
| --- | --- | --- | --- |
| Critical | Empty dashboard password authorized management mutations (`hub/src/web.rs` delivered lines 58–60, 92–96, 275–404); example default was empty/weak. | An unauthenticated network client could mutate fleet state. | Startup now requires an Argon2id hash; sessions are server-side, expiring, SameSite, HttpOnly and CSRF-bound. `config::tests::empty_or_plaintext_admin_setup_fails_closed`; frontend login path. |
| Critical | One fleet-wide bearer token (`agent/src/main.rs` delivered lines 20–23, 64, 112–115; `hub/src/web.rs` 216–241, 469–478). | One compromised VPS could report as any server. | Per-Agent Ed25519 key, pre-created server binding, atomic one-use enrollment, revocation, signed identities. `db::tests::report_binding_replay_state_findings_and_identity_survive_restart`. |
| Critical | Reports had no version, timestamp, sequence, message ID, signature, stale check, or replay persistence. | Replay and cross-server telemetry overwrite. | Versioned postcard envelope, complete signature, freshness/size/content checks, durable sequence/message uniqueness. Common signature test, web envelope test, DB restart replay test. |
| High | Unknown-server auto-registration defaulted true (`hub/src/config.rs` delivered 38–43, 265–272; `state.rs` 235–247); tombstones were bypassable. | Deleted Agent silently recreated itself. | Unknown registration is absent; enrollment requires an existing record; deletion revokes credentials and outstanding tokens and creates a durable tombstone. Enrollment/tombstone test. |
| High | Traffic was a mutable month total/reset with no checkpoint seed, calendar instances, adjustments, or uncertainty. | Restart/reset lost or double-counted bytes; manual provider usage could not be reconciled honestly. | Persistent Agent monotonic counters; SQLite checkpoints/cycles; immutable checkpoint-tied seed; append-only adjustments; timer rollover; estimated boundary interpolation. Agent state and traffic module tests. |
| High | Mutable JSON state and no explicit migrations (`hub/src/state.rs` delivered 470–556). | Partial writes and incompatible upgrades corrupted registry/replay/traffic. | SQLite WAL, foreign keys, immediate report transactions, schema migration table, indexes and retention. Migration idempotence/restart tests. |
| High | Global state mutex crossed serialization/disk/network/alert work. | One slow disk or notification stalled all reports and UI clients. | Short SQLite connections/transactions; no global application state lock. Only a bounded rate-limit map is locked briefly. 1,000-Agent load test. |
| High | Installer wrote root-only config while service used `DynamicUser` (`hub/assets/install.sh` delivered 107–127); self-test failures were ignored and a failed service still printed success. | Installed Agent could not read config and operator saw false success. | Static `parade` user, exact ownership/modes, real `--version` and unprivileged `check-config`, hard failure if systemd is inactive. Bash syntax and installer negative-integrity test procedure. |
| High | Agent release builds could omit TLS while enrollment advertised HTTPS. | One-command install succeeded but reports could not reach the Hub. | TLS is an Agent default and build script always uses `--all-features`; non-loopback HTTP is rejected. `tests::remote_plain_http_is_rejected`. |
| High | Artifact integrity relied on same-origin download/no pinned value. | Proxy/repository compromise could substitute a binary. | Build emits an offline Ed25519-signed `SHA256SUMS`; the enrollment command pins the public key and manifest; installer verifies signature and binary before consuming the token, over HTTPS. |
| Medium | Fixed three-second reports had no jitter, rollup, bounded spool, or compact encoding. | ~333 MiB body/month before protocol overhead and synchronized fleet bursts. | Ten-second local sampling, five-minute compact rollup, jitter, changed-only detail, one pending report, backoff. Deterministic bandwidth regression test. |
| Medium | UI received full-fleet snapshots every two seconds; a measured 1,000-server snapshot was 614,299 bytes (~1.03 GiB/browser-hour). | Large fleets caused bandwidth and DOM churn. | Paginated REST (100 rows UI, 500 server maximum API), Preact stable rendering, no polling full-fleet WebSocket. Playwright desktop/mobile and synthetic load coverage. |
| Medium | Alert channel/background email tasks were unbounded. | Fleet incident exhausted memory/tasks and spammed operators. | Legacy outbound delivery was removed from the milestone; findings deduplicate and events have retention. No external alert queue exists. |
| Medium | Forwarded headers were trusted through a boolean default. | Direct client spoofed source IP/scheme. | Only explicit proxy peer IPs activate `X-Forwarded-For`; public URL is strict config. `web::tests::forwarded_client_ip_is_used_only_for_an_explicitly_trusted_peer`. |
| Medium | Tests, CI, bandwidth/load, frontend, migration, security, and fixture coverage were absent; baseline `cargo test` ran zero tests. | Regressions shipped without detection. | Rust/front-end/unit/fixture/traffic/replay/migration/bandwidth/1,000-Agent/Playwright suites and CI workflow are present. Exact executed results are maintained in `PLANS.md`. |

Additional verified delivered concerns were misleading “control plane” language,
public bind defaults, secrets in example-shaped configuration, no explicit
partial/unsupported UI states, no mobile/light/reduced-motion behavior, and no
process-environment privacy regression. All were removed or covered by the
current architecture, UI, examples, and tests.

## Current residual findings

- Authentication/system log collectors and several desired evidence rules
  (SSH failures, root login, sudo, OOM/kernel/filesystem/service-crash signals)
  are not yet implemented. The UI reports coverage limitations and makes no
  clean/safe claim.
- Artifact trust uses a root-owned Hub-read-only staging tree, configured
  public-key digest, detached offline Ed25519 manifest signature, command-pinned
  manifest/key digests, and per-binary checksums. Compromise of the offline
  release key or root/administrator session remains a release compromise.
- External notification destinations are deliberately deferred; current
  alerts are in-console findings/events only.
- Several desirable high-cardinality views and collectors remain intentionally
  narrow: authentication/journal rules, long-term downsampled charts, remote
  endpoint aggregation, and full Fleet saved-view/sort controls are deferred.
  The implemented vertical slice reports unsupported/partial coverage rather
  than inventing healthy values.
- rusqlite calls in HTTP handlers are synchronous. Transactions are short and
  rate-limited, and the 1,000-Agent benchmark is bounded, but a future scale
  pass should move database work to bounded blocking workers and measure
  staggered/synchronized reconnects through the full HTTP stack.
- Legacy mutable JSON state has no automatic importer because its semantics
  cannot safely reconstruct identities, replay cursors, or seed checkpoints.
  `MIGRATION.md` documents the fail-safe boundary.
