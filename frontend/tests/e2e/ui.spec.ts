import { expect, test } from "@playwright/test";
import { mkdir } from "node:fs/promises";
import { resolve } from "node:path";

async function signIn(page: import("@playwright/test").Page) {
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /Parade/i })).toBeVisible();
  const password = page.getByLabel("Administrator password");
  await password.focus();
  await password.fill("correct horse battery staple");
  await page.keyboard.press("Tab");
  await page.keyboard.press("Enter");
  await expect(page.locator(".readonly")).toContainText("Read-only monitoring");
}

test("authenticated Fleet is keyboard accessible and responsive", async ({
  page,
}, testInfo) => {
  await signIn(page);
  const serverId = `e2e-${testInfo.project.name}`;
  await page.evaluate(async (id) => {
    const csrf = document.cookie
      .split(";")
      .map((part) => part.trim())
      .find((part) => part.startsWith("parade_csrf="))
      ?.slice("parade_csrf=".length);
    for (let index = 0; index < 16; index += 1) {
      const suffix = String(index).padStart(2, "0");
      const current = `${id}-${suffix}`;
      const response = await fetch("/api/v1/servers", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-Parade-CSRF": csrf || "",
        },
        body: JSON.stringify({
          id: current,
          name: `Synthetic ${current}`,
          group: "e2e",
        }),
      });
      if (!response.ok && response.status !== 409)
        throw new Error(`fixture server creation failed: ${response.status}`);
    }
  }, serverId);
  await page.goto("/#/fleet");
  await expect(
    page.getByRole("heading", { name: "Fleet", level: 1 }),
  ).toBeVisible();
  await expect(page.getByText(`Synthetic ${serverId}-00`)).toBeVisible();
  await expect(page.locator(".readonly")).toContainText("Read-only monitoring");

  if (testInfo.project.name === "mobile") {
    const menu = page.getByRole("button", { name: "Open navigation" });
    await expect(menu).toHaveAttribute("aria-expanded", "false");
    await menu.click();
    await expect(menu).toHaveAttribute("aria-expanded", "true");
    await expect(page.locator("#primary-navigation")).toBeVisible();
    await page.getByRole("link", { name: "Fleet", exact: true }).click();
    await expect(menu).toHaveAttribute("aria-expanded", "false");
  }

  const directory = resolve(process.cwd(), "../docs/screenshots");
  await mkdir(directory, { recursive: true });
  await page.screenshot({
    path: resolve(directory, `fleet-${testInfo.project.name}.png`),
    fullPage: true,
  });
});

test("representative server evidence and traffic views render", async ({
  page,
}, testInfo) => {
  test.skip(testInfo.project.name !== "desktop", "desktop visual evidence set");
  await signIn(page);
  await page.setViewportSize({ width: 1440, height: 1100 });
  const now = Math.floor(Date.now() / 1000);
  await page.route("**/api/v1/servers/visual**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    const data: Record<string, unknown> = {
      "/api/v1/servers/visual": {
        id: "visual",
        name: "Edge gateway · synthetic",
        group: "production",
        status: "online",
        last_seen: now - 18,
        os: "Debian GNU/Linux 13",
        kernel: "6.12.0-amd64",
        arch: "x86_64",
        agent_version: "0.1.0",
        inventory_hash: "b".repeat(64),
        coverage: [
          { collector: "resources", status: "high", detail: "available" },
          {
            collector: "security_logs",
            status: "partial",
            detail: "journal group not granted",
          },
        ],
      },
      "/api/v1/servers/visual/leases": {
        state: "inactive",
        response_count: 0,
        encoded_response_bytes: 0,
      },
      "/api/v1/servers/visual/processes": {
        observed_at: now - 20,
        data: [
          {
            state: "S",
            pid: 4217,
            ppid: 1,
            uid: 1001,
            executable: "/tmp/cache-worker",
            cpu_ticks: 88123,
            rss_bytes: 188743680,
            virtual_bytes: 536870912,
            cgroup: "/system.slice/cache-worker.service",
            systemd_unit: "cache-worker.service",
            listening_sockets: 1,
            package_ownership: "unowned",
            suspicious_writable_path: true,
            deleted_executable: false,
          },
          {
            state: "S",
            pid: 817,
            ppid: 1,
            uid: 0,
            executable: "/usr/sbin/sshd",
            cpu_ticks: 2217,
            rss_bytes: 12582912,
            virtual_bytes: 104857600,
            cgroup: "/system.slice/ssh.service",
            systemd_unit: "ssh.service",
            listening_sockets: 2,
            package_ownership: "owned",
            suspicious_writable_path: false,
            deleted_executable: false,
          },
        ],
      },
      "/api/v1/servers/visual/network": {
        observed_at: now - 20,
        data: {
          observed_rx_delta: 73400320,
          observed_tx_delta: 18874368,
          confidence: "high",
          anomaly_flags: [],
          interfaces: [
            {
              name: "ens3",
              selected: true,
              rx_bytes: 8796093022208,
              tx_bytes: 2199023255552,
              rx_packets: 9812312,
              tx_packets: 5512211,
              rx_errors: 0,
              tx_errors: 0,
              rx_drops: 12,
              tx_drops: 0,
            },
          ],
          listeners: [
            {
              protocol: "tcp",
              local_address: "0.0.0.0",
              port: 22,
              uid: 0,
              inode: 88271,
            },
          ],
        },
      },
      "/api/v1/servers/visual/findings": {
        observed_at: now - 20,
        data: [
          {
            severity: "review",
            confidence: "high",
            rule_id: "process.writable_executable",
            rule_version: 1,
            occurrences: 3,
            evidence: "pid=4217 executable=/tmp/cache-worker uid=1001",
            explanation:
              "An executable is running from a generally writable temporary path.",
            verification:
              "Verify the process locally with ps and package ownership tools.",
            coverage_caveat:
              "Process metadata may be hidden or falsified by a privileged attacker.",
          },
        ],
      },
      "/api/v1/servers/visual/traffic": {
        cycle_id: 7,
        cycle_start: now - 12 * 86400,
        cycle_end: now + 18 * 86400,
        seed_bytes: 0,
        has_manual_seed: false,
        seed_effective_at: null,
        seed_checkpoint_at: null,
        seed_note: null,
        observed_rx_bytes: 8589934592,
        observed_tx_bytes: 3221225472,
        observed_bytes: 11811160064,
        adjustment_bytes: -1073741824,
        total_bytes: 10737418240,
        limit_bytes: 536870912000,
        confidence: "partial",
        checkpoint_at: now - 18,
        agent_observed_total_bytes: 8796093022208,
        observation_start_at: now - 86400,
        projected_bytes: 335007449088,
        selected_interfaces: {
          mode: "auto",
          selected: ["ens3"],
          excluded: ["docker0", "wg0"],
        },
        timezone: "UTC",
        anchor_day: 1,
        anchor_time: "00:00",
      },
    };
    const value = data[path];
    if (value === undefined) return route.fallback();
    await route.fulfill({ json: value });
  });

  const directory = resolve(process.cwd(), "../docs/screenshots");
  await mkdir(directory, { recursive: true });
  await page.goto("/#/servers/visual/overview");
  await expect(page.getByText("Edge gateway · synthetic")).toBeVisible();
  await page.screenshot({
    path: resolve(directory, "server-overview-desktop.png"),
    fullPage: false,
  });

  await page.goto("/#/servers/visual/processes");
  await expect(page.getByText("/tmp/cache-worker")).toBeVisible();
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.screenshot({
    path: resolve(directory, "process-evidence-desktop.png"),
    fullPage: false,
  });

  await page.goto("/#/servers/visual/network");
  await expect(page.getByText("ens3")).toBeVisible();
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.screenshot({
    path: resolve(directory, "network-evidence-desktop.png"),
    fullPage: false,
  });

  await page.goto("/#/servers/visual/security");
  await expect(page.getByText("process.writable_executable")).toBeVisible();
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.screenshot({
    path: resolve(directory, "security-evidence-desktop.png"),
    fullPage: false,
  });

  await page.goto("/#/servers/visual/traffic");
  await page.getByLabel("Current provider-used amount").fill("123.4");
  await page.getByRole("button", { name: "Preview seed" }).click();
  await expect(page.getByText("Confirm immutable seed")).toBeVisible();
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.screenshot({
    path: resolve(directory, "traffic-seed-preview-desktop.png"),
    fullPage: false,
  });
});

test("login rejects an incorrect password without exposing the application", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByLabel("Administrator password").fill("incorrect password");
  await page.getByRole("button", { name: "Sign in" }).click();
  await expect(page.getByRole("status")).toContainText("Access denied");
  await expect(
    page.getByText("Read-only monitoring", { exact: true }),
  ).toHaveCount(0);
});
