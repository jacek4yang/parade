# Getting started

English | [简体中文](getting-started.zh-CN.md)

This is the shortest safe path from a signed GitHub Release to one reporting
Linux VPS. For reverse-proxy hardening, multi-host operations, backup, upgrade,
and removal, continue to [production deployment](deployment.md).

## 1. Check the prerequisites

You need:

- a signed Parade GitHub Release;
- an x86_64 or aarch64 Linux Hub with systemd;
- a DNS name and an HTTPS reverse proxy for production;
- one or more Linux Agent hosts that can reach that HTTPS origin outbound; and
- `curl`, OpenSSL, and common account/service tools on the installation hosts.

The Hub listens on `127.0.0.1:8008` by default. Agents do not listen on any
port. NAT and CGNAT Agent hosts need no port forwarding. If the Hub is behind
NAT, you must provide a stable HTTPS origin outside Parade; Parade does not
change routes, firewalls, VPNs, tunnels, or port mappings.

## 2. Install the Hub

The convenience bootstrap is:

```bash
curl -fsSL https://github.com/jacek4yang/parade/releases/latest/download/parade-install.sh | sudo bash -s -- hub
```

The command initially trusts GitHub HTTPS for the script. After the script
starts, it verifies the release manifest and every payload with a confirmed or
pinned Ed25519 public key, checks SHA-256 digests, and self-tests the Hub.
High-assurance deployments should download and review the script first and
verify the release assets with an independently obtained public-key digest; see
[production deployment](deployment.md#high-assurance-release-verification).

Choose English or 简体中文 when prompted, then enter:

1. the canonical public HTTPS origin, such as `https://parade.example.com`;
2. a unique administrator password of at least 12 characters; and
3. explicit confirmation of the displayed release-key fingerprint.

The installer refuses unsupported operating systems/architectures, an unsafe
URL, an invalid key, signature or checksum, and a partial existing Hub install.

## 3. Publish HTTPS

Keep the Hub on loopback. Configure a maintained reverse proxy using
[`nginx/parade.conf`](../nginx/parade.conf), add only the immediate proxy
addresses to `trusted_proxies`, validate the proxy configuration, and confirm
HTTPS before enabling HSTS. Parade never infers its public origin from an
untrusted `Host` or forwarded header.

Open the configured HTTPS origin and sign in. The UI follows the browser
language on first use and lets you switch between English and Simplified
Chinese.

## 4. Enroll the first Agent

In **Settings**, create a server record with a unique server ID and display
name. Copy the generated 15-minute, single-use enrollment command and run the
complete command on that exact VPS.

The Agent installer:

- detects the Linux architecture and verifies the pinned release tree;
- creates a fresh per-server Ed25519 identity;
- consumes the token only for the pre-created server record;
- installs a dedicated unprivileged `parade` service with no capabilities;
- writes only Parade's own private configuration/state paths; and
- starts an outbound-only service with no listening port.

Do not reuse one enrollment command on another host. Create a separate record
and command for servers A, B, C, and every later VPS.

## 5. Verify the first report

On the Agent host:

```bash
systemctl is-active parade-agent
sudo -u parade /usr/local/bin/parade-agent check-config /etc/parade/agent.toml
ss -ltnup
```

Confirm `parade-agent` is active and has no listening socket. In the Hub, wait
for the first normal rollup, then check:

- Fleet freshness and collector coverage;
- Resources for CPU, memory, disk/inode and pressure evidence;
- Processes and Network for bounded privacy-preserving facts;
- Security for evidence, confidence and coverage limitations; and
- Events/Audit for enrollment and report history.

Missing or unsupported collection is not proof of safety. Do not grant extra
privilege automatically; inspect the stated coverage reason first.

## 6. Start the provider traffic cycle

After the first reliable traffic checkpoint:

1. open the server's **Traffic** tab;
2. choose timezone, monthly boundary, selected-interface policy and one of the
   five closed billing modes;
3. read the provider dashboard's current-cycle usage at the same time;
4. enter that value and a source note; and
5. confirm the exact Agent checkpoint and preview equation.

Parade stores an immutable manual seed, adds only later locally observed
traffic, and rolls the next calendar cycle to zero without resetting Linux
counters. See [traffic accounting](traffic-accounting.md) before using larger-
direction or separate-direction billing.

## Next steps

- Multiple public/NAT hosts, reverse proxy, backup, upgrade, and removal:
  [production deployment](deployment.md).
- Full build-to-retirement reference: [operator lifecycle](operations.md).
- Bandwidth, memory and retention limits: [resource budgets](resource-budgets.md).
- Safe diagnosis: [troubleshooting](troubleshooting.md).
- Security assumptions: [threat model](../THREAT_MODEL.md).
