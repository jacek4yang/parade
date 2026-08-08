# Parade complete operator lifecycle

English | [简体中文](zh-CN/OPERATIONS.md)

This runbook covers Parade from release acquisition through retirement. Replace
all example domains, paths and versions. Parade is always an observation system;
none of these workflows adds remote control of a monitored VPS.

## 1. Accept the boundary before deployment

Parade cannot execute arbitrary commands, scripts or uploaded code; control
processes/services/packages/firewalls/users/files; reboot, shut down or
reinstall a VPS; invoke provider write APIs; or automatically remediate a
finding. The Agent reads narrow OS telemetry and its own private state.

Agents have no listener and connect outbound over authenticated TLS. Temporary
detail is a closed, versioned, size-limited read-only profile with a short TTL
and audit record. Agent signatures authenticate identity, not kernel truth; a
root/kernel-compromised host may falsify every observed fact.

## 2. Plan public and NAT topology

```text
browser -> HTTPS proxy -> loopback Parade Hub -> SQLite/WAL
                             ^
                             | outbound HTTPS only
                 public, NAT and CGNAT Agents
```

Public-IP and NAT Agents behave the same and open no Parade port. A NAT Hub
requires an operator-managed HTTPS endpoint that every Agent can reach. Parade
does not discover peer paths, configure a tunnel, change routing/firewalls, or
distribute Hub storage. Shared/changed egress IP is evidence only; server-bound
credentials provide identity.

## 3. Check platform requirements

The one-line Hub installer supports x86_64 and aarch64 Linux with systemd. The
release Agent tree includes static musl targets for x86_64/aarch64/armv7 and GNU
fallbacks including riscv64gc where built by the release workflow. Unsupported
collectors degrade explicitly.

Production also requires an operator-managed DNS name, HTTPS reverse proxy,
certificate lifecycle, correct system time, and outbound HTTPS from every Agent.
Runtime has no required external database, broker, CDN, analytics or provider
integration.

## 4. Acquire and verify a signed Release

Use the [high-assurance procedure](deployment.md#high-assurance-release-verification)
to pin the release public key, verify `SHA256SUMS.release.sig`, validate hashes,
and review the installer. The convenience path is:

```bash
curl -fsSL https://github.com/jacek4yang/parade/releases/latest/download/parade-install.sh | sudo bash -s -- hub
```

This command initially trusts GitHub HTTPS; only later payloads are authenticated
by the release key. A Release workflow needs repository secret
`PARADE_RELEASE_SIGNING_KEY_B64` and a version-matched `v*` tag on the default
branch. It must fail rather than publish unsigned/missing architectures.

## 5. Build and sign from source when required

Build the embedded frontend before the Hub:

```bash
git clone https://github.com/jacek4yang/parade.git
cd parade/frontend
npm ci
npm run build
cd ..
cargo build --release --workspace --all-features --locked
```

Create an Ed25519 release key once outside the repository and keep it offline:

```bash
umask 077
openssl genpkey -algorithm Ed25519 -out /secure/offline/parade-release.key
parade_agent_stage=$(mktemp -d)
PARADE_RELEASE_SIGNING_KEY=/secure/offline/parade-release.key \
  scripts/build-agents.sh --dist "$parade_agent_stage"
sha256sum "$parade_agent_stage/release-public.pem"
```

Never put the private key, administrator password, enrollment token or
production config in Git, release staging, logs or issues. After creating the
`parade-hub` account, review the user-writable staging tree, copy only the public
key, detached signature, checksums and target binaries into a root-owned,
Hub-group-readable `/var/lib/parade-dist`, then remove the staging tree according
to local policy. A complete multi-architecture tree requires `cross`; without
it, do not advertise targets the script skipped.

After the service account exists, stage the reviewed tree:

```bash
sudo install -d -o root -g parade-hub -m 0750 /var/lib/parade-dist
sudo cp -a "$parade_agent_stage"/. /var/lib/parade-dist/
sudo chown -R root:parade-hub /var/lib/parade-dist
sudo chmod -R u=rwX,g=rX,o= /var/lib/parade-dist
```

## 6. Initialize the Hub

The installer creates the dedicated user, private database path, configuration,
and hardened systemd service. Manual builds can generate a password hash without
writing plaintext to TOML:

```bash
read -rsp 'Hub administrator password: ' parade_admin_password
printf '\n'
printf '%s\n' "$parade_admin_password" | target/release/parade-hub hash-password
unset parade_admin_password
```

Keep `listen = "127.0.0.1:8008"`; set the canonical path-free HTTPS
`public_url`, private SQLite path, root-owned release directory, pinned public-
key digest, and only immediate reverse proxies. Validate before service start:

```bash
/usr/local/bin/parade-hub check-config /etc/parade/hub.toml
```

## 7. Terminate HTTPS safely

Review [`nginx/parade.conf`](../nginx/parade.conf). Keep the Hub on loopback,
validate with `nginx -t`, test login/cookies/redirects over HTTPS, and enable
HSTS only afterward. The Hub never derives its public URL from untrusted Host or
forwarded headers.

## 8. First login and multi-host enrollment

Open `public_url`, sign in, and select English or Simplified Chinese. For each
host A, B and C:

1. create a unique server ID/name in Settings;
2. copy its 15-minute, single-use enrollment command;
3. run it only on the corresponding VPS;
4. verify service/config/listeners locally; and
5. wait for the first signed rollup and inspect coverage.

Never reuse one token or clone Agent state. Registration generates a fresh
identity bound to one server record. Unknown-server auto-registration is off.
Hub and Agent may coexist on one VPS with separate service groups and private
component configs.

## 9. Validate the deployed Agent

```bash
systemctl is-active parade-agent
sudo -u parade /usr/local/bin/parade-agent check-config /etc/parade/agent.toml
ss -ltnup
```

Confirm no `parade-agent` listener exists. An absence in `ss` is a local
observation, not proof of Internet reachability. Do not add an inbound firewall
rule for Parade.

## 10. Configure provider traffic accounting

Wait for a reliable checkpoint, then set IANA timezone, monthly boundary,
interface policy, billing mode and optional limits. Read the provider's current-
cycle value at the same time, enter it with a source note, and confirm the exact
checkpoint/equation.

The modes are inbound+outbound, inbound-only, outbound-only, larger-direction,
and separate-direction. The final two require both directional seeds. Cycles
roll to zero automatically without resetting Linux counters. Corrections are
append-only and uncertainty remains visible. Follow the full
[traffic-accounting guide](traffic-accounting.md).

## 11. Operate the read-only console

- Fleet: freshness, status, coverage, grouping and bounded report topology.
- Resources: CPU/load, memory/swap, PSI, disk/inode and trend gaps.
- Processes: bounded top-N/suspicious facts, no environment variables or full
  command lines by default.
- Network: selected interface counters/errors and decoded listening endpoints.
- Security: stable rule/version, severity, confidence, first/last evidence,
  recurrence and safe manual verification.
- Events/Audit: availability, identity, accounting and operator changes.
- Typed detail: a closed read-only profile set; process/socket snapshots are
  forced and bounded, and live detail ends after at most ten minutes.

Never interpret no finding as proof that a host is clean or uncompromised.

## 12. Monitor resource and retention limits

Supplied systemd units cap Agent memory at 128 MiB and Hub memory at 512 MiB,
plus tasks, descriptors and restart/log rates. Queues, procfs reads, interfaces,
anomalies, report size, retention deletion batches and WAL are bounded. Normal
measured upload is 5.431 MiB/Agent/30 days on the recorded baseline.

Detailed telemetry expires by policy, while identity, tombstone, audit, finding
and accounting evidence is intentionally durable. Monitor and back up the state
directory; see [resource budgets](resource-budgets.md).

## 13. Rotate Agent credentials and upgrade Agents

Mint a new enrollment token for the same server and rerun the complete command
locally. The installer stages verified replacement artifacts before redemption
and keeps the existing binary until enrollment succeeds. A lost response after
redemption may leave the old identity revoked; mint another token instead of
forcing the old service to run.

Protocol-incompatible releases require a coordinated Hub/Agent upgrade or
re-enrollment. Parade does not guess an old report shape.

## 14. Upgrade or roll back the Hub

Before upgrade:

1. make a consistent database backup;
2. record Hub version and maximum `schema_migrations` version;
3. back up Hub config and the signed Agent tree;
4. verify the new Release and replace the local Hub binary;
5. validate login, a signed report, traffic cycles and Audit.

Migrations are transactional before listening. Schema downgrade is not
supported; a true rollback restores the complete pre-upgrade backup. See
[`MIGRATION.md`](../MIGRATION.md).

## 15. Back up and restore

Use SQLite online `.backup`, or stop the Hub and copy the database with any WAL
and SHM files as one set. Encrypt backups, preserve permissions and test
restoration on an isolated Hub.

Old restored replay state may lag current Agents. Review the gap and rotate
affected identities if integrity is uncertain; do not suppress sequence/replay
checks.

## 16. Revoke, delete and retire

- Leaked unused token: let it expire/discard it and mint a new one.
- Suspected Agent private-key compromise: rotate or tombstone the server.
- Deleted server: retain its durable tombstone; reuse a new server ID for a new
  asset.
- Suspected Hub compromise: treat database, displayed telemetry and release
  staging as untrusted; rebuild and rotate Agents individually.

Local uninstall preserves state unless `--purge` is explicitly selected:

```bash
curl -fsSL https://github.com/jacek4yang/parade/releases/latest/download/parade-uninstall.sh | sudo bash -s -- agent
curl -fsSL https://github.com/jacek4yang/parade/releases/latest/download/parade-uninstall.sh | sudo bash -s -- hub
```

These convenience commands trust GitHub HTTPS for a root-level script. The
[production deployment removal procedure](deployment.md#local-removal) shows
how to pin a tag, verify the signed checksum, review the script and run it
locally. Back up audit/accounting evidence first. Parade cannot remotely trigger
removal.

## 17. Troubleshoot without weakening controls

Use service status, read-only journals, `check-config`, DNS/time tests,
interface selection/coverage evidence and audit records. Never bypass a key,
signature, TLS, replay, sequence or size failure. The symptom-by-symptom runbook
is [troubleshooting.md](troubleshooting.md).

## 18. Acceptance and repository verification

Host acceptance includes version output, intended Hub listener, no Agent
listener, working HTTPS login, first signed report, visible traffic formula and
Audit entry.

Repository checks:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

cd frontend
npm run format:check
npm run lint
npm run typecheck
npm test
npm run build
npm run test:e2e
cd ..

bash -n hub/assets/install.sh scripts/*.sh
shellcheck hub/assets/install.sh scripts/*.sh
bash scripts/test-installer-integrity.sh
npm --prefix frontend audit --audit-level=high
cargo audit
```

Run `scripts/performance-gate.sh` only on an isolated development/CI host. The
deterministic bandwidth test is
`default_monthly_bandwidth_is_below_target`; the fleet test is
`synthetic_fleet_load_1000_agents_is_bounded`. Claim a pass only for a command
that actually completed successfully. Exact current results and blockers are in
[`PLANS.md`](../PLANS.md).

## Known limitations

- The report topology is not a peer mesh, connectivity scanner, relay or
  distributed store.
- Provider APIs and write actions are intentionally absent.
- Shared NAT/provider billing may not be attributable to one VPS.
- Root/kernel compromise can hide or falsify telemetry.
- Some journal, package, socket ownership and architecture-specific collectors
  remain explicit partial/unsupported coverage.
- Durable security/accounting history grows slowly with real operator actions.
- Real systemd installs on every supported distribution/architecture require
  disposable-host validation.
