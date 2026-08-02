# Parade

Parade is a self-hosted, low-bandwidth, strictly read-only Linux VPS fleet
observability and security-posture console. It collects resource, network,
privacy-preserving process, listener, coverage, event, and traffic evidence. It
may explain and alert; it cannot remotely control or modify a monitored VPS.

There is one Rust Hub binary, one outbound-only Rust Agent per monitored host,
an embedded Preact UI, and embedded SQLite/WAL persistence. Production has no
mandatory database server, broker, CDN, analytics, provider integration, or
frontend runtime dependency.

```text
browser --HTTPS--> reverse proxy --> Parade Hub --> SQLite WAL
                                        ^
                                        | signed compact reports / TLS
                                        |
                         unprivileged outbound-only Agents
                              read /proc and /sys only
```

## Safety boundary

Monitored VPSes are observation targets. Parade has no arbitrary command,
shell, script, task, plugin, file management, process/service/package/firewall/
user control, reboot, shutdown, reinstall, provider write, or remediation API.
The Hub can request additional detail only through a closed, versioned enum of
read-only profiles with strict limits and at most a ten-minute expiry.

Every Agent has an independent Ed25519 identity. Enrollment tokens are
short-lived, single-use, hashed, and bound to one pre-created server. Signed
reports contain version, server/Agent IDs, time, monotonic sequence, unique
message ID, and body; replay, staleness, cross-server identity, wrong content
type, malformed encoding, unsupported versions, and oversized bodies fail
closed.

Host-local telemetry is not attestation. A root- or kernel-compromised machine
can hide activity or lie to Parade. The UI never claims a server is clean, safe,
or uncompromised.

## What is included

- Fleet overview and paginated/scannable fleet table for large installations.
- CPU/load, memory/swap, PSI, disk/inodes, uptime, socket, OS/kernel/architecture,
  interface traffic, freshness, and collector coverage.
- Bounded top-N and suspicious process facts without environment variables or
  full command lines; changed listening ports and temporary typed snapshots.
- Evidence-based, versioned findings with confidence, recurrence, explanation,
  verification guidance, and limitations.
- Events, operator audit history, themes, density, keyboard focus, reduced
  motion, and desktop/~390 px responsive layouts.
- Transparent manual-seed traffic accounting with automatic calendar rollover.
- Compact five-minute rollups, jitter, bounded retry/spool, and a deterministic
  below-10-MiB/month default synthetic bandwidth test.

## Interface

The Playwright suite generates non-sensitive synthetic views directly into the
repository. They exercise the same embedded production bundle served by the
Hub.

![Fleet at desktop width](docs/screenshots/fleet-desktop.png)

![Transparent manual traffic seed preview](docs/screenshots/traffic-seed-preview-desktop.png)

Additional views: [390 px Fleet](docs/screenshots/fleet-mobile.png),
[process evidence](docs/screenshots/process-evidence-desktop.png),
[network evidence](docs/screenshots/network-evidence-desktop.png),
[security evidence](docs/screenshots/security-evidence-desktop.png), and
[server overview](docs/screenshots/server-overview-desktop.png).

## Build

Prerequisites are a current stable Rust toolchain and Node.js for rebuilding the
embedded UI.

```bash
cd frontend
npm ci
npm run build
cd ..
cargo build --release --workspace --all-features
```

The UI build writes `hub/assets/app.js`, `app.css`, `login.js`, and the embedded
HTML. The Hub binary then includes those assets at compile time.

Generate the administrator password hash without putting a real password in
Git:

```bash
printf '%s\n' 'a unique password of at least 12 characters' | target/release/parade-hub hash-password
```

## Hub setup

Create the dedicated account and private paths, install the binary and example,
then replace `password_hash`, `public_url`, and proxy addresses locally:

```bash
sudo useradd --system --home-dir /var/lib/parade --shell /usr/sbin/nologin parade-hub
sudo install -d -o root -g parade-hub -m 0750 /etc/parade
sudo install -d -o parade-hub -g parade-hub -m 0700 /var/lib/parade
sudo install -o root -g root -m 0755 target/release/parade-hub /usr/local/bin/
sudo install -o root -g parade-hub -m 0640 config/hub.toml /etc/parade/hub.toml
sudo install -o root -g root -m 0644 systemd/parade-hub.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now parade-hub
```

The safe default listener is `127.0.0.1:8008`. Terminate HTTPS at a maintained
reverse proxy; adapt `nginx/parade.conf`, set HSTS only after HTTPS works, and
list only the immediate proxy IPs in `trusted_proxies`. Parade never infers its
public URL from forwarded or Host headers.

## Build and stage Agent releases

Create and protect an offline Ed25519 release key outside this repository (once):

```bash
umask 077
openssl genpkey -algorithm Ed25519 -out /secure/offline/parade-release.key
```

The release script builds TLS-enabled artifacts, writes each target under
`dist/<triple>/parade-agent`, produces `SHA256SUMS`, signs it, and exports the
public key. The private key path is supplied only to the offline build process:

```bash
PARADE_RELEASE_SIGNING_KEY=/secure/offline/parade-release.key \
  scripts/build-agents.sh --musl-only --dist /var/lib/parade-dist
sudo chown -R root:parade-hub /var/lib/parade-dist
sudo chmod -R u=rwX,g=rX,o= /var/lib/parade-dist
sha256sum /var/lib/parade-dist/release-public.pem
```

`cross` is recommended for x86_64, aarch64, armv7, and riscv64 targets; without
it the script attempts locally installed Rust targets. Static musl is preferred,
with GNU fallback. The Agent requires Linux/systemd for the supported one-command
install. Debian, Ubuntu, AlmaLinux, and Rocky Linux are the primary targets;
unsupported collectors degrade with an explicit coverage reason.
Put the printed public-key digest in `release_public_key_sha256`. The Hub service
has read-only access to the root-owned artifact tree; the installer verifies the
configured key pin, detached offline signature, manifest digest, and binary
checksum before execution.

## One-command enrollment

In Settings, create a server record. Parade mints a 15-minute one-use token and
an enrollment command containing a pinned SHA-256 digest of the release
manifest. Copy the complete command to that VPS. The installer:

1. requires HTTPS outside loopback development;
2. verifies the pinned manifest and selected Agent binary;
3. runs the actual Agent `--version` self-test;
4. creates the static unprivileged `parade` user and private state directory;
5. generates a fresh local signing identity and atomically consumes the token;
6. validates config/state readability as that user; and
7. starts a hardened outbound-only systemd service, failing if it is inactive.

On upgrade, the existing Agent binary is not replaced until enrollment has
succeeded. If enrollment fails, a previously active service is restarted.

No per-Agent TOML editing and no fleet-wide credential are required.

## Manual current-cycle traffic workflow

Provider APIs are deliberately outside this milestone. Parade implements the
provider-dashboard workflow directly:

1. Open a server's Traffic tab and configure its IANA timezone, day 1–31, local
   boundary time, optional provider limit, and interface policy.
2. Wait for the first reliable Agent checkpoint.
3. Read the provider dashboard's already-used current-cycle traffic.
4. Enter that value and source note. Confirm the exact checkpoint/effective
   time; Parade stores one immutable primary seed.
5. Parade displays and updates:

   ```text
   manual provider seed
   + Linux bytes observed after the seed checkpoint
   + append-only audited corrections
   = current cycle total
   ```

6. At the configured monthly boundary the Hub automatically opens a zero-seed
   cycle without resetting Linux counters. Closed cycles and audit history stay
   intact. If a report interval straddles the boundary, the split is visibly
   `estimated` and is refined from adjacent checkpoints after reconnection.

Linux cannot reconstruct bytes from before Parade began observing. Provider
billing may differ because of direction weighting, protocol overhead, rounding,
private traffic, or provider policy. Parade shows the selected interfaces,
source, timestamps, components, and uncertainty rather than hiding those gaps.

## Resource expectations

The normal synthetic protocol body and request/TLS assumptions are measured by
`default_monthly_bandwidth_is_below_target`. The regression fails above 20 MiB
and targets below 10 MiB upload per Agent per 30 days, excluding explicit
live-detail sessions and genuine event bursts. The final measured value for the
current tree is recorded in `PLANS.md`.

Detailed resources retain 30 days, traffic rollups 90 days, process/socket
changes seven days plus the latest value, and events 180 days. Identity, replay,
audit, findings, tombstones, manual seeds, adjustments, and cycle totals are
durable. Raw traffic checkpoints are bounded to 400 days while preserving every
seed checkpoint and the latest checkpoint per server.

## Backup, restore, and upgrade

Use SQLite's online `.backup` command/API, or stop the Hub and copy the database
with any WAL/SHM files. Encrypt backups and test restoration. Upgrade only after
a consistent backup; migrations run transactionally before the listener opens.
See `MIGRATION.md` for rollback and the safe legacy-JSON transition. Restoring
an old identity/replay database requires security review because Agents may have
advanced beyond its cursors.

## Uninstall

On an Agent host, this local operator action removes only Parade's own service
and private files:

```bash
sudo systemctl disable --now parade-agent
sudo rm /etc/systemd/system/parade-agent.service /usr/local/bin/parade-agent
sudo rm /etc/parade/agent.toml
sudo userdel parade
```

Review and remove `/var/lib/parade-agent` only after deciding whether its local
identity/traffic state is needed for investigation. Parade never initiates
uninstall remotely.

For the Hub, stop/disable its service, preserve the SQLite backup and audit
records, then remove the binary/config/service and dedicated user at the local
operator's discretion.

## Troubleshooting

- Hub refuses startup: validate every TOML field and generate a real Argon2id
  hash; remote `public_url` must be HTTPS and path-free.
- No enrollment command: build/stage `SHA256SUMS` and target artifacts in
  `dist_dir` with readable ownership.
- Installer verification fails: do not bypass it. Rebuild/stage the release and
  mint a new command so the pinned digest matches.
- Agent service fails: inspect `journalctl -u parade-agent -e`; verify the
  `parade` user can read `/etc/parade/agent.toml` and read/write its private
  state. `sudo -u parade parade-agent check-config /etc/parade/agent.toml` must
  succeed.
- Traffic says awaiting checkpoint: allow one signed Agent rollup before adding
  a manual provider seed.
- Partial/unsupported security data: grant no extra privilege automatically.
  Review collector coverage and, only if acceptable, add the documented narrow
  read-only log group locally.

## Development verification

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
npx playwright test
```

Architecture, audit, threat model, operational security, migration boundary,
and exact executed check results are in `ARCHITECTURE.md`, `AUDIT.md`,
`THREAT_MODEL.md`, `SECURITY.md`, `MIGRATION.md`, and `PLANS.md`.
