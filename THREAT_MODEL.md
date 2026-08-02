# Threat model

## Assets and assumptions

Assets are Agent private signing keys, enrollment capabilities, administrator
sessions, SQLite state/audit history, telemetry integrity, and release
artifacts. TLS and the reverse proxy are assumed correctly operated. The Hub
host and administrator are trusted to administer Parade metadata. A monitored
VPS is explicitly not trusted as an attestation source.

The primary invariant is stronger than feature scope: no Hub input can cause
arbitrary execution or host mutation on a monitored VPS.

## Threats and controls

| Threat | Impact | Controls and remaining risk |
| --- | --- | --- |
| Stolen enrollment token | Attacker races enrollment | 15-minute, random, hashed, single-use token bound to one existing non-tombstoned server; atomic redemption binds one fresh public key. A token stolen before use can still win the race, so the operator must revoke/rotate that Agent. |
| Stolen Agent key | Forged observations for one VPS | Independent Ed25519 identity, server binding, revocation, monotonic sequence, unique message IDs, freshness window. It cannot impersonate another server or control the host, but it can poison its bound server's evidence until revoked. |
| One compromised VPS | False telemetry or key theft | Compromise is isolated to one identity. No fleet bearer and no Hub-to-Agent command channel exist. Root/kernel compromise can falsify all local evidence; the UI discloses this. |
| Replay or reordering | Duplicate/downgraded state | Signed timestamp/sequence/message ID; SQLite uniqueness and replay cursor are updated in the same immediate transaction as telemetry. State survives restart. |
| Cross-server impersonation | One Agent overwrites another | Hub looks up only an active `(server_id, agent_id)` binding and verifies its key; signature includes both identities. |
| Malicious Hub user | Abuse of monitored targets | Closed observation enum, ten-minute maximum TTL, no user path/text in leases, Agent-side validation, request/response audit, byte measurement, and early cancellation. A Hub operator can change Parade metadata or request bounded observation detail, but cannot express remote control. |
| Reverse-proxy spoofing | Bypass login/report rate attribution | Forwarded IP is ignored unless the immediate peer is explicitly trusted. The canonical public URL is validated configuration and never inferred from `Host`. Proxy TLS/HSTS remains an operator responsibility. |
| Database theft | Fleet metadata and public keys exposed; session hashes attacked | Database contains hashed enrollment/session/CSRF values and public keys, not plaintext dashboard passwords or Agent private keys. Telemetry and audit can still be sensitive; encrypt backups and restrict filesystem access. An active session token stolen elsewhere remains usable until expiry/revocation. |
| Installer/artifact compromise | Malicious Agent on VPS | Root-owned Hub-read-only artifacts, configured Ed25519 public-key fingerprint, detached offline signature, command-pinned key/manifest digests, per-binary checksum, HTTPS, and actual self-test before token redemption. Compromise of the offline key or Hub administrator/root can still replace the trust root/command. |
| Telemetry poisoning | Misleading findings/accounting | Signed identity proves source, not truth. Type/length limits, monotonic traffic checks, evidence/confidence labels, and append-only accounting corrections limit damage. Root/kernel-compromised hosts remain untrustworthy. |
| Process secret leakage | Credentials exposed to Hub | Agent never reads process environments or full command lines; fields are bounded to executable identity, UID, state, resource facts, cgroup/unit hints, and security markers. Executable paths may still reveal application names. |
| Denial of service by Agents | CPU, memory, database or disk exhaustion | 256 KiB body limit, strict content type, peer-IP pre-authentication and verified-Agent rate limits, bounded 10,000-key limiter, SQLite busy timeout, compact five-minute default, bounded Agent spool, indexed queries, and retention. A sufficiently large authenticated flood can still saturate a single Hub. Some rusqlite work remains synchronous inside async handlers, so edge concurrency limits are recommended. |
| Alert/finding flood | Operator overload and database growth | Findings upsert by stable rule/evidence and count recurrences; events and detail rollups have retention. External notification delivery is not in this milestone, so there is no network alert queue to exhaust. |
| Clock manipulation | Stale rejection or wrong cycle attribution | Hub applies a ±10-minute signed-report window; cycles are calculated on Hub time. Clock skew coverage/finding breadth is currently limited and is documented. |
| Symlink/path traversal in artifacts | Read arbitrary Hub files | Artifact paths have a closed character set, reject dot/empty segments, are canonicalized, and must remain beneath the canonical dist root. |

## Abuse cases that are structurally absent

There is no remote shell, command/script/task/job/plugin API, arbitrary path
read/download, process/service/package/firewall/user/file control, reboot,
shutdown, reinstall, provider write integration, or remediation worker. Adding
one would violate the security model and requires rejection, not a feature flag.

## Security claims Parade does not make

Parade does not prove a server is clean, safe, uncompromised, or policy
compliant. Missing telemetry is not a healthy zero. Host-local observations are
not trustworthy after sufficiently privileged compromise. Provider byte totals
may differ from Linux counters because of billing direction, overhead, rounding,
excluded traffic, and pre-observation history.
