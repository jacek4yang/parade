# Production deployment

English | [简体中文](deployment.zh-CN.md)

This guide turns the quick start into a production deployment. Parade remains
a read-only observer throughout: local installers manage only Parade's own
files and services; neither the Hub nor an Agent can operate another host.

## Supported topology

```text
operators -> HTTPS reverse proxy -> 127.0.0.1:8008 Parade Hub -> SQLite/WAL
                                      ^
                                      | outbound HTTPS only
                       public-IP / NAT / CGNAT Linux Agents
```

- A public-IP Agent opens no Parade port.
- A NAT/CGNAT Agent requires only DNS, correct time, and outbound HTTPS access.
- A Hub behind NAT needs an operator-managed reachable HTTPS origin.
- Several Agents may share or change an egress address. Server-bound identity,
  not source IP, identifies them.
- The UI's topology is bounded evidence of verified Agent-to-Hub reports. It is
  not a scanner, peer mesh, relay, route optimizer, or distributed database.

## High-assurance Release verification

The one-line installer initially trusts GitHub HTTPS. For a stronger bootstrap,
select one explicit published tag and download every asset from that immutable
tag without running it:

```bash
tag=v1.0.0
release_base="https://github.com/jacek4yang/parade/releases/download/${tag}"
curl -fLO "${release_base}/parade-install.sh"
curl -fLO "${release_base}/SHA256SUMS.release"
curl -fLO "${release_base}/SHA256SUMS.release.sig"
curl -fLO "${release_base}/release-public.pem"
```

Obtain the expected SHA-256 digest of `release-public.pem` through an
independent trusted channel, compare it, verify the detached Ed25519 signature,
then verify and review the installer:

```bash
sha256sum release-public.pem
openssl pkeyutl -verify -pubin -inkey release-public.pem -rawin \
  -in SHA256SUMS.release -sigfile SHA256SUMS.release.sig
sha256sum --check --ignore-missing SHA256SUMS.release
less parade-install.sh
```

Replace `v1.0.0` with the exact Release you intend to install. Do not combine
files from different releases and do not continue after any failure. Run the
reviewed local script with both the same tag and trusted key pin, so its later
binary downloads cannot follow a changed `latest` pointer:

```bash
tag=v1.0.0
sudo env \
  PARADE_VERSION="${tag}" \
  PARADE_RELEASE_KEY_SHA256='<trusted 64-hex digest>' \
  bash parade-install.sh hub
```

The first public signed Release cannot exist until the repository owner
provisions `PARADE_RELEASE_SIGNING_KEY_B64` and the independently recorded
`PARADE_RELEASE_PUBLIC_KEY_SHA256` in the protected `signed-release`
environment, then publishes a version-matched `v*` tag reachable from the
default branch. The workflow fails closed when signing material, its pinned
public-key digest or a required architecture is absent.

## Hub installation and HTTPS

The interactive installer asks for language, canonical public URL and one
administrator password. It installs the Hub as a dedicated account, checks the
configuration, and starts the supplied hardened systemd unit. It does not
install or configure a reverse proxy.

Use [`nginx/parade.conf`](../nginx/parade.conf) as a reviewed example:

1. keep the Hub listener on `127.0.0.1:8008`;
2. obtain and renew a valid certificate outside Parade;
3. configure only immediate proxy IPs in `trusted_proxies`;
4. run `nginx -t` before a local reload;
5. verify authentication, cookies and redirects over HTTPS; and
6. enable HSTS only after the whole domain is proven HTTPS-only.

If the proxy is remote, explicitly isolate the Hub-to-proxy path. Do not expose
the Hub's mutation APIs on plaintext transport.

## Enroll multiple servers

For each host A, B, C, and later host:

1. sign in to the Hub and create one server record;
2. copy its unique 15-minute, single-use command;
3. run it only on the matching VPS;
4. verify `parade-agent` is active and has no listening socket;
5. wait for the first signed report and inspect collector coverage; and
6. record the server's billing cycle and seed only after a reliable checkpoint.

Never share a token or copy `/var/lib/parade-agent` between machines. Each Agent
has an independent private key and sequence state. A compromised Agent cannot
authenticate as another server.

Hub and Agent can share one VPS. `/etc/parade` remains a root-owned traversal
directory, while each component's configuration is readable only by its own
service group. Do not replace these permissions with a shared fleet secret.

## Automated Hub installation

Automation must explicitly provide every trust/input value. For an attended
automation run, read the password without terminal echo or a shell-history
literal, export it only for the installer, and remove it afterward:

```bash
read -rsp 'Hub administrator password: ' PARADE_ADMIN_PASSWORD
printf '\n'
export PARADE_ADMIN_PASSWORD
export PARADE_LANG=en
export PARADE_VERSION=v1.0.0
export PARADE_PUBLIC_URL=https://parade.example.com
export PARADE_RELEASE_KEY_SHA256='<trusted 64-hex digest>'
sudo --preserve-env=PARADE_LANG,PARADE_VERSION,PARADE_PUBLIC_URL,PARADE_ADMIN_PASSWORD,PARADE_RELEASE_KEY_SHA256 \
  bash parade-install.sh hub
unset PARADE_ADMIN_PASSWORD
```

For a fully unattended runner, have its secret manager inject
`PARADE_ADMIN_PASSWORD` into the process environment instead of constructing a
command containing the value. Environment secrets are still briefly available
to root and the executing account through process inspection, so use a dedicated
runner and clear the variable after completion. Do not place real values in
Git, shell history, examples, images, or issue reports. Agent enrollment remains
per-server and single-use; there is no fleet-wide bearer token.

## Operational validation

Run on the applicable local host:

```bash
parade-hub --version
parade-agent --version
systemctl is-active parade-hub
systemctl is-active parade-agent
sudo -u parade /usr/local/bin/parade-agent check-config /etc/parade/agent.toml
ss -ltnup
```

Confirm the Hub listens only where intended, the Agent has no listener, HTTPS
login works, a signed report appears, Traffic shows its checkpoint/formula, and
Audit contains the operator changes. An unsupported collector should be shown
as unavailable/partial, never silently treated as zero or safe.

## Backup and restore

Use SQLite's online backup API/`.backup`, or stop the Hub before copying the
database together with any WAL/SHM files. Encrypt the backup, preserve
ownership, include Hub configuration and the signed Agent release tree, and
test restoration on an isolated Hub.

Restoring an old identity/replay database can move Hub cursors behind Agents.
Review that interval and rotate individual Agent identities when integrity is
uncertain. Schema migrations are transactional, but there is no automatic
downgrade; restore the complete pre-upgrade backup to roll back.

## Upgrade and credential rotation

Before a Hub upgrade, record the current versions and migration level, make a
consistent backup, verify all new Release assets, replace only Parade's own Hub
binary, and validate login/report/accounting/audit after startup.

To rotate or upgrade an Agent, mint a fresh enrollment command for the same
server and rerun it locally. The installer stages the new binary, unit and
identity before token redemption. If the response is lost after redemption,
the old identity may already be revoked; the installer leaves it stopped rather
than falsely rolling back. Mint a new token and retry.

## Local removal

The Release uninstaller operates only on the machine where an administrator
runs it and preserves state by default:

```bash
curl -fsSL https://github.com/jacek4yang/parade/releases/latest/download/parade-uninstall.sh | sudo bash -s -- agent
curl -fsSL https://github.com/jacek4yang/parade/releases/latest/download/parade-uninstall.sh | sudo bash -s -- hub
```

These convenience commands execute a root-level script trusted through GitHub
HTTPS; unlike the installer, the uninstaller downloads no later payload to
authenticate. For high assurance, pin one tag and verify/review the script
before running it locally:

```bash
tag=v1.0.0
release_base="https://github.com/jacek4yang/parade/releases/download/${tag}"
curl -fLO "${release_base}/parade-uninstall.sh"
curl -fLO "${release_base}/SHA256SUMS.release"
curl -fLO "${release_base}/SHA256SUMS.release.sig"
curl -fLO "${release_base}/release-public.pem"
sha256sum release-public.pem
openssl pkeyutl -verify -pubin -inkey release-public.pem -rawin \
  -in SHA256SUMS.release -sigfile SHA256SUMS.release.sig
sha256sum --check --ignore-missing SHA256SUMS.release
less parade-uninstall.sh
sudo bash parade-uninstall.sh agent
```

Compare the public-key digest through the same independent channel as install;
use `hub` instead of `agent` only on the intended Hub host.

Use `--purge` only after deciding whether identity, accounting and audit
evidence is still required and after backing it up. Unattended removal also
requires `PARADE_CONFIRM_UNINSTALL=uninstall`. Parade has no remote uninstall
path.

## Continue reading

- [Complete operator lifecycle](operations.md)
- [Manual traffic accounting](traffic-accounting.md)
- [Resource and retention budgets](resource-budgets.md)
- [Troubleshooting](troubleshooting.md)
- [Security policy](../SECURITY.md)
- [Migration and rollback](../MIGRATION.md)
