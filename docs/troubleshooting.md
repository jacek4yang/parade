# Troubleshooting

English | [简体中文](troubleshooting.zh-CN.md)

Diagnose Parade without weakening its trust boundary. Never fix an installation
by disabling TLS, signature/checksum validation, replay checks, size limits, or
the dedicated service account. Never open an Agent port or add remote command
execution.

## Hub does not start

Read the local status and journal, then validate the exact configuration:

```bash
systemctl status parade-hub --no-pager
journalctl -u parade-hub -e --no-pager
/usr/local/bin/parade-hub check-config /etc/parade/hub.toml
```

Check that:

- `password_hash` is a real Argon2id hash and no empty password is accepted;
- a remote `public_url` uses HTTPS and has no path, query or credentials;
- `trusted_proxies` contains only direct proxy addresses;
- the database directory is writable by `parade-hub`;
- the release directory is root-owned but readable by the Hub group; and
- the configured release-public-key digest matches the staged key.

Migrations run before the listener opens. If a migration fails, preserve the
database/WAL/SHM and restore the complete known-good backup; do not edit schema
version rows manually.

## HTTPS login, redirect, or client address is wrong

Verify the Hub still listens only on the intended address and test the reverse
proxy configuration locally:

```bash
ss -ltnp
nginx -t
```

Confirm the configured `public_url` exactly matches the browser origin and the
immediate proxy address is explicitly trusted. Do not trust a broad network to
make forwarded headers work. Enable HSTS only after the complete HTTPS path is
valid.

## No Agent enrollment command

The Hub only mints a command when the staged Agent release tree is complete and
its manifest signature is valid. Check the configured `dist_dir` for the
manifest, detached signature, public key and required architecture file, plus
Hub read permissions. Rebuild/sign/stage a complete release; do not bypass the
verification in code or edit a command by hand.

## Installer rejects a Release

Stop on a key-pin, Ed25519 signature, manifest digest, checksum, archive or
self-test failure. Ensure every downloaded file came from the same tag, compare
the public-key digest through an independent channel, and mint a new enrollment
command if its pinned manifest changed.

If re-enrollment failed after token redemption began, the old credential may
already be revoked even when the response was lost. The installer deliberately
does not restart that identity. Mint a fresh single-use token and retry the
reviewed installer.

## Agent is installed but not reporting

On that Agent host, use read-only checks:

```bash
systemctl status parade-agent --no-pager
journalctl -u parade-agent -e --no-pager
sudo -u parade /usr/local/bin/parade-agent check-config /etc/parade/agent.toml
timedatectl status
getent hosts parade.example.com
ss -ltnup
```

Check outbound DNS/HTTPS reachability, accurate time, configuration permissions,
Hub URL, identity/state ownership and whether the server was revoked or
tombstoned. NAT/CGNAT hosts need no inbound rule. Confirm there is no
`parade-agent` listener rather than opening one.

The Agent keeps one pending signed report. After a long outage, an authenticated
stale-report marker lets it retire only that envelope, preserve monotonic
traffic state, and resend current bounded evidence. Repeated identity/sequence
rejection instead suggests copied/rolled-back state or an old Hub restore and
requires identity/replay review before rotation.

## Data is stale, partial, unsupported, or missing

Use the displayed collector coverage reason. Common causes include kernel/
architecture differences, absent procfs fields, permission to an allowlisted
log source, a counter reset, or a report gap. Parade deliberately renders
unavailable values instead of inventing zero.

Do not automatically run the Agent as root. If authentication-log visibility is
required, a local administrator may assess a documented narrow read-only group
such as `systemd-journal`; that expands exposure and is optional.

## Traffic is waiting for a checkpoint

Allow one reliable signed rollup before creating a provider seed. A seed must
bind to a precise checkpoint. Verify that at least one intended interface is
actually selected and review any anomaly/partial reason.

## Parade and the provider disagree

Compare the configured billing mode, timezone/boundary, seed checkpoint, actual
selected interfaces, directional components and corrections. Providers may
count overhead, rounding, shared NAT, private traffic or different directions.
Use a reasoned append-only correction; never edit the original seed or Linux
counters.

An `estimated` boundary means the Agent was offline or counters could not be
split exactly. It is evidence of uncertainty, not a transient UI error.

## Disk or memory use is higher than expected

Check the local service accounting and file sizes without deleting evidence:

```bash
systemctl show parade-hub -p MemoryCurrent -p TasksCurrent
systemctl show parade-agent -p MemoryCurrent -p TasksCurrent
du -h /var/lib/parade
ls -lh /var/lib/parade/parade.sqlite3*
```

Normal rollups, queues and WAL are bounded/retained. Identity, tombstone, audit,
finding and traffic-cycle history is intentionally durable. Back up and review
those records before any local archival policy. Parade does not change the
host-wide journald quota.

Run the performance gate only on an isolated development/CI host, never a
monitored VPS. See [resource budgets](resource-budgets.md).

## UI shows an old bundle or an error state

Static JS/CSS use strong ETags and revalidation; HTML and API responses are
`no-store`. Perform a normal browser reload, inspect the HTTP status/content
type, and confirm the Hub was rebuilt after the production frontend build.
Do not add a CDN, external script, tracker, or permissive content type.

## Information to preserve for a report

Include Parade version, architecture, a redacted configuration shape, exact
error, timestamps/timezone, collector coverage and the smallest reproduction.
Never include enrollment tokens, authorization headers, cookies, private keys,
passwords, full production config, raw sensitive reports or arbitrary host
files. Follow [SECURITY.md](../SECURITY.md) for suspected vulnerabilities.
