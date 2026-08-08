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

async function createFixtureServers(
  page: import("@playwright/test").Page,
  prefix: string,
  count: number,
) {
  await page.evaluate(
    async ({ fixturePrefix, fixtureCount }) => {
      const csrf = document.cookie
        .split(";")
        .map((part) => part.trim())
        .find((part) => part.startsWith("parade_csrf="))
        ?.slice("parade_csrf=".length);
      for (let index = 0; index < fixtureCount; index += 1) {
        const suffix = String(index).padStart(2, "0");
        const current = `${fixturePrefix}-${suffix}`;
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
    },
    { fixturePrefix: prefix, fixtureCount: count },
  );
}

test("authenticated Fleet is keyboard accessible and responsive", async ({
  page,
}, testInfo) => {
  await signIn(page);
  const serverId = `e2e-${testInfo.project.name}`;
  await createFixtureServers(page, serverId, 16);
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
  const desktop = testInfo.project.name === "desktop";
  await signIn(page);
  if (desktop) await page.setViewportSize({ width: 1440, height: 1100 });
  const now = Math.floor(Date.now() / 1000);
  let trafficMode = "sum";
  await page.route("**/api/v1/servers/visual**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (
      path === "/api/v1/servers/visual/traffic/rule" &&
      route.request().method() === "PUT"
    ) {
      trafficMode = JSON.parse(route.request().postData() || "{}").billing_mode;
      await route.fulfill({ status: 204 });
      return;
    }
    if (
      path === "/api/v1/servers/visual/traffic/seed" &&
      route.request().method() === "POST"
    ) {
      await route.fulfill({ json: { accepted: true } });
      return;
    }
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
      "/api/v1/servers/visual/resources": {
        observed_at: now - 20,
        data: {
          interval_start: now - 320,
          interval_end: now - 20,
          samples: 30,
          cpu_avg_pct: 18.4,
          cpu_max_pct: 44.2,
          cpu_cores: 4,
          load1_avg: 0.74,
          mem_total: 8589934592,
          mem_used: 3221225472,
          swap_total: 2147483648,
          swap_used: 134217728,
          disk_total: 107374182400,
          disk_used: 42949672960,
          disk_inodes_total: 6553600,
          disk_inodes_used: 740000,
          psi_cpu_some_avg10: 0.12,
          psi_mem_some_avg10: 0.03,
          psi_io_some_avg10: 0.08,
          tcp_connections: 42,
          udp_connections: 7,
        },
        history: [],
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
            state: "active",
          },
        ],
      },
      "/api/v1/servers/visual/traffic": {
        cycle_id: 7,
        cycle_start: now - 12 * 86400,
        cycle_end: now + 18 * 86400,
        seed_bytes: 0,
        seed_rx_bytes: null,
        seed_tx_bytes: null,
        seed_combined_bytes: 0,
        directional_seed_known: false,
        has_manual_seed: false,
        seed_effective_at: null,
        seed_checkpoint_at: null,
        seed_note: null,
        observed_rx_bytes: 8589934592,
        observed_tx_bytes: 3221225472,
        observed_bytes: 11811160064,
        adjustment_bytes: -1073741824,
        billed_adjustment_bytes: -1073741824,
        rx_adjustment_bytes: 0,
        tx_adjustment_bytes: 0,
        total_bytes: 10737418240,
        billed_total_bytes: 10737418240,
        limit_bytes: 536870912000,
        rx_total_bytes: null,
        tx_total_bytes: null,
        rx_limit_bytes: null,
        tx_limit_bytes: null,
        billing_mode: "sum",
        confidence: "partial",
        checkpoint_at: now - 18,
        agent_observed_total_bytes: 8796093022208,
        observation_start_at: now - 86400,
        projected_bytes: 335007449088,
        projected_rx_bytes: null,
        projected_tx_bytes: null,
        selected_interfaces: {
          mode: "auto",
          selected: ["ens3"],
          excluded: ["docker0", "wg0"],
        },
        actual_selected_interfaces: ["ens3"],
        traffic_anomalies: [
          "counter reset was isolated to an earlier interval",
        ],
        uncertainty_reason: "counter reset was isolated to an earlier interval",
        timezone: "UTC",
        anchor_day: 1,
        anchor_time: "00:00",
        history: [
          {
            cycle_id: 6,
            cycle_start: now - 42 * 86400,
            cycle_end: now - 12 * 86400,
            state: "closed",
            confidence: "high",
            billing_mode: "sum",
            has_manual_seed: true,
            seed_bytes: 107374182400,
            seed_note: "Provider invoice checkpoint",
            seed_effective_at: now - 40 * 86400,
            seed_checkpoint_at: now - 40 * 86400,
            seed_operator: "admin",
            observed_rx_bytes: 21474836480,
            observed_tx_bytes: 10737418240,
            adjustment_bytes: -1073741824,
            total_bytes: 138512695296,
            billed_total_bytes: 138512695296,
            rx_total_bytes: null,
            tx_total_bytes: null,
            adjustments: [
              {
                direction: "billed",
                signed_bytes: -1073741824,
                effective_at: now - 20 * 86400,
                reason: "Provider dashboard correction",
                created_at: now - 20 * 86400,
                operator: "admin",
              },
            ],
            adjustments_truncated: false,
          },
        ],
      },
    };
    let value = data[path];
    if (path === "/api/v1/servers/visual/traffic" && value) {
      value = {
        ...(value as Record<string, unknown>),
        billing_mode: trafficMode,
        directional_seed_known: trafficMode !== "sum",
        seed_rx_bytes: trafficMode === "sum" ? null : 0,
        seed_tx_bytes: trafficMode === "sum" ? null : 0,
        history: (
          (value as Record<string, unknown>).history as Array<
            Record<string, unknown>
          >
        ).map((cycle) => ({ ...cycle, billing_mode: trafficMode })),
      };
    }
    if (value === undefined) return route.fallback();
    await route.fulfill({ json: value });
  });

  const directory = resolve(process.cwd(), "../docs/screenshots");
  await mkdir(directory, { recursive: true });
  await page.goto("/#/servers/visual/overview");
  await expect(page.getByText("Edge gateway · synthetic")).toBeVisible();
  await page.screenshot({
    path: resolve(directory, `server-overview-${testInfo.project.name}.png`),
    fullPage: false,
  });

  await page.goto("/#/servers/visual/processes");
  await expect(page.getByText("/tmp/cache-worker")).toBeVisible();
  await page.evaluate(() => window.scrollTo(0, 0));
  if (desktop)
    await page.screenshot({
      path: resolve(directory, "process-evidence-desktop.png"),
      fullPage: false,
    });

  await page.goto("/#/servers/visual/network");
  await expect(page.getByText("ens3")).toBeVisible();
  await page.evaluate(() => window.scrollTo(0, 0));
  if (desktop)
    await page.screenshot({
      path: resolve(directory, "network-evidence-desktop.png"),
      fullPage: false,
    });

  await page.goto("/#/servers/visual/security");
  await expect(page.getByText("process.writable_executable")).toBeVisible();
  await page.evaluate(() => window.scrollTo(0, 0));
  if (desktop)
    await page.screenshot({
      path: resolve(directory, "security-evidence-desktop.png"),
      fullPage: false,
    });

  await page.goto("/#/servers/visual/traffic");
  await page.getByLabel("Current provider-used combined traffic").fill("123.4");
  await page.getByRole("button", { name: "Preview seed" }).click();
  await expect(page.getByText("Confirm immutable seed")).toBeVisible();
  await page.evaluate(() => window.scrollTo(0, 0));
  if (desktop)
    await page.screenshot({
      path: resolve(directory, "traffic-seed-preview-desktop.png"),
      fullPage: false,
    });
  const seedRequest = page.waitForRequest(
    (request) =>
      request.method() === "POST" &&
      new URL(request.url()).pathname === "/api/v1/servers/visual/traffic/seed",
  );
  await page.getByRole("button", { name: "Confirm and save seed" }).click();
  expect((await seedRequest).postDataJSON().combined_bytes).toBe(
    Math.round(123.4 * 1024 ** 3),
  );
  await expect(
    page.getByRole("button", { name: "Preview seed" }),
  ).toBeVisible();
  await page.getByLabel("Provider billing mode").selectOption("max_direction");
  await expect(
    page.getByText(
      "Save the billing-cycle rule before entering a seed for the newly selected mode.",
    ),
  ).toBeVisible();
  await page.getByRole("button", { name: "Save cycle rule" }).click();
  await expect(
    page.getByLabel("Current provider-used inbound traffic"),
  ).toBeVisible();
  await page.getByLabel("Current provider-used inbound traffic").fill("100");
  await page.getByLabel("Current provider-used outbound traffic").fill("90");
  await page.getByRole("button", { name: "Preview seed" }).click();
  await expect(page.locator(".seed-preview")).toContainText("Larger direction");
  await page.screenshot({
    path: resolve(
      directory,
      `traffic-billing-modes-${testInfo.project.name}.png`,
    ),
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

test("Simplified Chinese persists from login through the application", async ({
  page,
}, testInfo) => {
  await page.goto("/");
  await page.getByLabel("Language").selectOption("zh-CN");
  await expect(page.locator("html")).toHaveAttribute("lang", "zh-CN");
  await expect(page.getByLabel("管理员密码")).toBeVisible();
  await expect(page.getByRole("button", { name: "登录" })).toBeVisible();
  await page.reload();
  await expect(page.getByLabel("管理员密码")).toBeVisible();
  await page.getByLabel("管理员密码").fill("correct horse battery staple");
  await page.getByRole("button", { name: "登录" }).click();

  await expect(page.locator(".readonly")).toContainText("只读监测");
  await expect(page.getByLabel("语言")).toHaveValue("zh-CN");
  await createFixtureServers(page, `zh-${testInfo.project.name}`, 8);
  const menuButton = page.getByRole("button", { name: "打开导航" });
  if (await menuButton.isVisible()) {
    await menuButton.click();
  }
  await page.getByRole("link", { name: "服务器群", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "服务器群", level: 1 }),
  ).toBeVisible();
  const search = page.getByPlaceholder("搜索名称、ID 或分组");
  await expect(search).toBeVisible();
  await search.fill(`zh-${testInfo.project.name}`);
  await expect(
    page.getByText(`Synthetic zh-${testInfo.project.name}-00`),
  ).toBeVisible();
  await expect(page.locator("tbody tr")).toHaveCount(8);
  const overflows = await page.evaluate(
    () => document.documentElement.scrollWidth > window.innerWidth,
  );
  expect(overflows).toBe(false);

  const directory = resolve(process.cwd(), "../docs/screenshots");
  await mkdir(directory, { recursive: true });
  await page.screenshot({
    path: resolve(directory, `fleet-zh-CN-${testInfo.project.name}.png`),
    fullPage: true,
  });

  await page.getByLabel("语言").selectOption("en");
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await expect(page.getByLabel("Language")).toHaveValue("en");
});
