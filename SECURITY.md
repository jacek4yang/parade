# Security policy and operations

English | [简体中文](SECURITY.zh-CN.md)

## Product boundary

Parade observes monitored Linux hosts. It does not remotely execute commands,
control processes/services/packages/firewalls/users/files, reboot or reinstall
machines, call provider write APIs, or remediate findings. Please report any
code path capable of those operations as a critical vulnerability.

## Secure deployment

1. Run the Hub as the dedicated `parade-hub` user using the supplied unit.
2. Keep it bound to `127.0.0.1:8008`; expose only a maintained TLS reverse
   proxy. Enable HSTS after HTTPS is proven for the complete domain.
3. Generate a unique administrator hash with `parade-hub hash-password`; never
   store the plaintext in TOML, shell history, Git, or the service environment.
4. Configure only immediate proxy addresses in `trusted_proxies`. Never use a
   broad address merely for convenience.
5. Restrict `/etc/parade/hub.toml`, `/var/lib/parade`, backups, and release
   staging. The database contains sensitive telemetry and active session hashes.
6. Build TLS-enabled Agent artifacts, stage their offline-signed `SHA256SUMS`,
   detached signature, and pinned public key, then copy the complete enrollment
   command from the authenticated UI. Tokens expire after 15 minutes and are
   single-use. The existing binary is not replaced before successful
   enrollment. A failure before token redemption restarts a previously active
   service; once redemption begins, the old credential may already be revoked
   and is deliberately left stopped after an ambiguous failure.
7. The installed Agent runs as `parade` with no capabilities and no listener.
   Optional log visibility should be granted only through a narrow read-only
   group after assessing the extra exposure.

The Hub refuses an empty/non-Argon2id password setup, a remote plaintext public
URL, unsafe URL characters, and invalid freshness thresholds.

## Credential response

- Suspected enrollment-token disclosure before use: discard it and mint a new
  token. A used/expired token cannot be reused.
- Suspected Agent key compromise: delete/tombstone the server record or enroll a
  newly created replacement identity. Deletion durably revokes active identity
  and outstanding enrollment tokens.
- Suspected administrator session theft: restart after revoking sessions in the
  SQLite database or rotate the dashboard password hash and invalidate session
  rows. Preserve audit evidence first.
- Suspected Hub compromise: treat database contents, release manifest, Agent
  enrollment, and all displayed telemetry as untrusted; rebuild from a known
  image and rotate Agents individually.

Never paste tokens, cookies, private keys, raw sensitive reports, or production
configuration into an issue. Reports should include version, threat scenario,
minimal reproduction, and proposed disclosure window.

## Limits

Ed25519 authentication establishes which enrolled Agent sent bytes; it does not
establish that its kernel told the truth. Root/kernel compromise can falsify or
hide all observed facts. Parade must never be the sole incident-response source.
Provider usage may differ from interface counters, and Linux cannot reconstruct
traffic from before observation began.

## Backup and audit

Use SQLite's online backup facility, or stop the Hub before copying the database
and its WAL/SHM state. Encrypt backups, test restoration, and preserve file
ownership. Operator mutations are stored in `audit_events`; traffic seeds and
adjustments are append-only. Routine retention does not erase those records.
