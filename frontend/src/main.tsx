import { render } from "preact";
import { useEffect, useState } from "preact/hooks";
import { api, type FleetResponse, type FleetServer } from "./api";
import { bytes, percent, relative } from "./format";
import "./styles.css";

const nav = [
  "Overview",
  "Fleet",
  "Security",
  "Traffic",
  "Events",
  "Settings",
] as const;
const tabs = [
  "Overview",
  "Resources",
  "Processes",
  "Network",
  "Security",
  "Events",
  "Traffic",
  "Inventory",
];

const trafficUnits: Record<string, number> = {
  MiB: 1024 ** 2,
  GiB: 1024 ** 3,
  TiB: 1024 ** 4,
  MB: 1000 ** 2,
  GB: 1000 ** 3,
  TB: 1000 ** 4,
};

function route(): string[] {
  return (location.hash.slice(1) || "/overview").split("/").filter(Boolean);
}

function useRoute(): string[] {
  const [parts, setParts] = useState(route());
  useEffect(() => {
    const update = () => setParts(route());
    addEventListener("hashchange", update);
    return () => removeEventListener("hashchange", update);
  }, []);
  return parts;
}

function useFleet(limit = 500, offset = 0) {
  const [data, setData] = useState<FleetResponse>();
  const [error, setError] = useState("");
  useEffect(() => {
    setError("");
    api<FleetResponse>(`/api/v1/fleet?limit=${limit}&offset=${offset}`)
      .then(setData)
      .catch((reason) => setError(reason.message));
  }, [limit, offset]);
  return { data, error };
}

function Status({ value }: { value: string }) {
  return (
    <span class={`status status-${value}`}>
      <span aria-hidden="true" class="status-dot" /> {value}
    </span>
  );
}

function Shell() {
  const parts = useRoute();
  const page = parts[0] ?? "overview";
  const [theme, setTheme] = useState(
    localStorage.getItem("parade-theme") || "dark",
  );
  const [compact, setCompact] = useState(
    localStorage.getItem("parade-density") === "compact",
  );
  const [menu, setMenu] = useState(false);
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    document.documentElement.dataset.density = compact
      ? "compact"
      : "comfortable";
  }, [theme, compact]);
  const title =
    parts[0] === "servers"
      ? "Server detail"
      : page[0]?.toUpperCase() + page.slice(1);
  return (
    <div class="shell">
      <a class="skip-link" href="#content">
        Skip to content
      </a>
      <aside
        id="primary-navigation"
        class={menu ? "sidebar sidebar-open" : "sidebar"}
        aria-label="Primary navigation"
      >
        <div class="brand">
          <span class="brand-mark">P</span>
          <span>
            <strong>Parade</strong>
            <small>Fleet observability</small>
          </span>
        </div>
        <div class="readonly">
          <span aria-hidden="true">◉</span> Read-only monitoring
        </div>
        <nav>
          {nav.map((item) => (
            <a
              class={page === item.toLowerCase() ? "active" : ""}
              href={`#/${item.toLowerCase()}`}
              onClick={() => setMenu(false)}
            >
              {item}
            </a>
          ))}
        </nav>
        <div class="sidebar-foot">No remote control or remediation</div>
      </aside>
      <div class="workspace">
        <header class="topbar">
          <button
            class="menu-button"
            aria-label="Open navigation"
            aria-expanded={menu}
            aria-controls="primary-navigation"
            onClick={() => setMenu(!menu)}
          >
            ☰
          </button>
          <div>
            <span class="eyebrow">Parade / {page}</span>
            <h1>{title}</h1>
          </div>
          <div class="top-actions">
            <span class="freshness">Authenticated Hub session</span>
            <button
              onClick={() => {
                const next = compact ? "comfortable" : "compact";
                localStorage.setItem("parade-density", next);
                setCompact(!compact);
              }}
              aria-label="Toggle display density"
            >
              {compact ? "Comfortable" : "Compact"}
            </button>
            <button
              onClick={() => {
                const next = theme === "dark" ? "light" : "dark";
                localStorage.setItem("parade-theme", next);
                setTheme(next);
              }}
              aria-label="Toggle color theme"
            >
              {theme === "dark" ? "Light" : "Dark"}
            </button>
          </div>
        </header>
        <main id="content">
          {parts[0] === "servers" && parts[1] ? (
            <ServerPage
              id={decodeURIComponent(parts[1])}
              tab={parts[2] || "overview"}
            />
          ) : (
            <GlobalPage page={page} />
          )}
        </main>
      </div>
    </div>
  );
}

function GlobalPage({ page }: { page: string }) {
  if (page === "overview") return <Overview />;
  if (page === "fleet") return <Fleet />;
  if (page === "settings") return <Settings />;
  return <FleetLens page={page} />;
}

function Overview() {
  const { data, error } = useFleet();
  if (error) return <ErrorState message={error} />;
  if (!data) return <Loading />;
  const counts = data.summary;
  const attention = data.servers
    .filter(
      (server) =>
        server.status !== "online" ||
        server.coverage.some((item) => item.status !== "high"),
    )
    .slice(0, 8);
  return (
    <>
      <section class="metric-grid" aria-label="Fleet status summary">
        <Metric
          label="Online"
          value={counts.online}
          tone="ok"
          detail={`${data.total} total`}
        />
        <Metric
          label="Stale"
          value={counts.stale}
          tone="warn"
          detail="Reports delayed"
        />
        <Metric
          label="Offline"
          value={counts.offline}
          tone="bad"
          detail="Needs review"
        />
        <Metric
          label="Enrollment pending"
          value={counts.pending}
          detail="Not yet reporting"
        />
      </section>
      <section class="panel">
        <PanelHead
          title="Attention queue"
          detail="Freshness and collection gaps, ordered for review"
        />
        {attention.length ? (
          <div class="attention-list">
            {attention.map((server) => (
              <a href={`#/servers/${encodeURIComponent(server.id)}/overview`}>
                <Status value={server.status} />
                <strong>{server.name}</strong>
                <span>
                  {server.coverage.some((item) => item.status !== "high")
                    ? "Partial telemetry coverage"
                    : "Reporting gap"}
                </span>
                <time>{relative(server.last_seen)}</time>
              </a>
            ))}
          </div>
        ) : (
          <Empty
            title="Nothing urgent in the current telemetry"
            detail="This does not prove that monitored hosts are safe."
          />
        )}
      </section>
      <section class="split">
        <div class="panel">
          <PanelHead
            title="Fleet distribution"
            detail="Current reporting state"
          />
          <div class="distribution">
            {["online", "stale", "offline", "pending"].map((state) => (
              <div>
                <span>{state}</span>
                <div class="track">
                  <i
                    class={`fill fill-${state}`}
                    style={{ width: percent(counts[state] ?? 0, data.total) }}
                  />
                </div>
                <strong>{counts[state] ?? 0}</strong>
              </div>
            ))}
          </div>
        </div>
        <div class="panel">
          <PanelHead
            title="Telemetry trust"
            detail="Evidence has explicit limits"
          />
          <p class="prose">
            Parade observes selected host-local telemetry. A sufficiently
            privileged attacker can falsify it. No absence of findings is proof
            that a host is safe.
          </p>
          <a class="text-link" href="#/security">
            Review coverage gaps →
          </a>
        </div>
      </section>
    </>
  );
}

function Fleet() {
  const [offset, setOffset] = useState(0);
  const [query, setQuery] = useState("");
  const limit = 100;
  const { data, error } = useFleet(limit, offset);
  if (error) return <ErrorState message={error} />;
  if (!data) return <Loading />;
  const visible = data.servers.filter((server) =>
    `${server.name} ${server.id} ${server.group}`
      .toLowerCase()
      .includes(query.toLowerCase()),
  );
  return (
    <section class="panel fleet-panel">
      <PanelHead
        title="Fleet"
        detail={`${data.total.toLocaleString()} registered servers`}
      />
      <div class="toolbar">
        <label class="search">
          <span class="sr-only">Search fleet</span>
          <input
            value={query}
            onInput={(event) => setQuery(event.currentTarget.value)}
            placeholder="Search name, ID or group"
          />
        </label>
        <span>
          Showing {offset + 1}–{Math.min(offset + limit, data.total)}
        </span>
      </div>
      {visible.length ? (
        <div class="table-wrap">
          <table>
            <thead>
              <tr>
                <th>Status</th>
                <th>Server</th>
                <th>Group</th>
                <th>System</th>
                <th>Coverage</th>
                <th>Last report</th>
              </tr>
            </thead>
            <tbody>
              {visible.map((server) => (
                <tr>
                  <td>
                    <Status value={server.status} />
                  </td>
                  <td>
                    <a
                      class="server-link"
                      href={`#/servers/${encodeURIComponent(server.id)}/overview`}
                    >
                      <strong>{server.name}</strong>
                      <small>{server.id}</small>
                    </a>
                  </td>
                  <td>{server.group || "—"}</td>
                  <td>
                    {server.os || "Awaiting inventory"}
                    <small>{server.arch}</small>
                  </td>
                  <td>{coverage(server)}</td>
                  <td>
                    <time>{relative(server.last_seen)}</time>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : (
        <Empty
          title="No matching servers"
          detail="Change the search query or add a server in Settings."
        />
      )}
      <div class="pager">
        <button
          disabled={offset === 0}
          onClick={() => setOffset(Math.max(0, offset - limit))}
        >
          Previous
        </button>
        <button
          disabled={offset + limit >= data.total}
          onClick={() => setOffset(offset + limit)}
        >
          Next
        </button>
      </div>
    </section>
  );
}

function FleetLens({ page }: { page: string }) {
  const { data, error } = useFleet();
  const [aggregate, setAggregate] = useState<any>();
  const [aggregateError, setAggregateError] = useState("");
  useEffect(() => {
    setAggregate(undefined);
    setAggregateError("");
    api(`/api/v1/${page}`)
      .then(setAggregate)
      .catch((reason) => setAggregateError(reason.message));
  }, [page]);
  if (error) return <ErrorState message={error} />;
  if (!data) return <Loading />;
  const copy: Record<string, [string, string]> = {
    security: [
      "Security center",
      "Evidence-based findings are grouped by server and rule.",
    ],
    traffic: [
      "Fleet traffic",
      "Cycles with different boundaries remain labeled separately.",
    ],
    events: [
      "Fleet events",
      "Availability, security, traffic and audit evidence.",
    ],
  };
  const [title, detail] = copy[page] ?? [page, "Fleet observability"];
  return (
    <section class="panel">
      <PanelHead title={title} detail={detail} />
      {aggregateError ? (
        <p class="notice">{aggregateError}</p>
      ) : !aggregate ? (
        <Loading />
      ) : page === "security" ? (
        <FindingList value={aggregate.items} />
      ) : page === "events" ? (
        <EventList value={aggregate.items} />
      ) : (
        <>
          <section class="metric-grid">
            <Metric
              label="Open cycles"
              value={aggregate.open_cycles}
              detail="Calendar instances"
            />
            <Metric
              label="Manually seeded"
              value={aggregate.seeded_cycles}
              detail="Checkpoint-tied provider entries"
            />
            <Metric
              label="Uncertain"
              value={aggregate.uncertain_cycles}
              tone={aggregate.uncertain_cycles ? "warn" : "ok"}
              detail="Partial or estimated"
            />
          </section>
          <div class="boundary-groups">
            {(aggregate.boundary_groups || []).map((group: any) => (
              <div>
                <strong>{group.timezone}</strong>
                <span>
                  day {group.anchor_day} at {group.anchor_time}
                </span>
                <b>{group.servers} server(s)</b>
              </div>
            ))}
          </div>
        </>
      )}
      <h3 class="section-label">Servers</h3>
      <div class="card-grid">
        {data.servers.slice(0, 24).map((server) => (
          <a
            class="server-card"
            href={`#/servers/${encodeURIComponent(server.id)}/${page === "security" ? "security" : page}`}
          >
            <Status value={server.status} />
            <strong>{server.name}</strong>
            <span>
              {page === "traffic"
                ? "Open cycle accounting"
                : page === "security"
                  ? `${coverage(server)} coverage`
                  : relative(server.last_seen)}
            </span>
          </a>
        ))}
      </div>
      {data.total === 0 && (
        <Empty
          title="No servers yet"
          detail="Create a server record in Settings, then enroll its Agent."
        />
      )}
    </section>
  );
}

function ServerPage({ id, tab }: { id: string; tab: string }) {
  const [server, setServer] = useState<any>();
  const [error, setError] = useState("");
  useEffect(() => {
    api(`/api/v1/servers/${encodeURIComponent(id)}`)
      .then(setServer)
      .catch((reason) => setError(reason.message));
  }, [id]);
  if (error) return <ErrorState message={error} />;
  if (!server) return <Loading />;
  return (
    <>
      <section class="server-head">
        <div>
          <div class="server-title">
            <h2>{server.name}</h2>
            <Status value={server.status} />
            <span class="readonly-badge">Read-only target</span>
          </div>
          <p>
            {server.os || "Inventory pending"} ·{" "}
            {server.kernel || "kernel unknown"} ·{" "}
            {server.arch || "architecture unknown"}
          </p>
        </div>
        <div class="server-meta">
          <span>
            Last report <strong>{relative(server.last_seen)}</strong>
          </span>
          <span>
            Coverage <strong>{coverage(server)}</strong>
          </span>
        </div>
      </section>
      <nav class="tabs" aria-label="Server sections">
        {tabs.map((name) => (
          <a
            class={tab === name.toLowerCase() ? "active" : ""}
            href={`#/servers/${encodeURIComponent(id)}/${name.toLowerCase()}`}
          >
            {name}
          </a>
        ))}
      </nav>
      <ServerTab id={id} tab={tab} server={server} />
    </>
  );
}

function ServerTab({
  id,
  tab,
  server,
}: {
  id: string;
  tab: string;
  server: any;
}) {
  if (tab === "overview")
    return (
      <section class="split">
        <div class="panel">
          <PanelHead
            title="Health summary"
            detail="Latest accepted signed rollup"
          />
          <dl class="facts">
            <div>
              <dt>Availability</dt>
              <dd>
                <Status value={server.status} />
              </dd>
            </div>
            <div>
              <dt>Agent</dt>
              <dd>{server.agent_version || "Pending"}</dd>
            </div>
            <div>
              <dt>Inventory fingerprint</dt>
              <dd class="mono truncate">
                {server.inventory_hash || "Pending"}
              </dd>
            </div>
          </dl>
        </div>
        <div class="panel">
          <PanelHead
            title="Coverage"
            detail="Unavailable data is never treated as a healthy zero"
          />
          <CoverageList items={server.coverage} />
        </div>
      </section>
    );
  if (tab === "traffic") return <TrafficTab id={id} />;
  if (tab === "processes")
    return (
      <DataTab
        id={id}
        endpoint="processes"
        title="Privacy-preserving processes"
        render={ProcessTable}
        lease
      />
    );
  if (tab === "network")
    return (
      <DataTab
        id={id}
        endpoint="network"
        title="Network and listening ports"
        render={NetworkView}
        lease
      />
    );
  if (tab === "security")
    return (
      <DataTab
        id={id}
        endpoint="findings"
        title="Security evidence"
        render={FindingList}
      />
    );
  if (tab === "events")
    return (
      <DataTab id={id} endpoint="events" title="Events" render={EventList} />
    );
  if (tab === "resources")
    return (
      <DataTab
        id={id}
        endpoint="resources"
        title="Resources"
        render={ResourceView}
      />
    );
  return (
    <section class="panel">
      <PanelHead
        title="Inventory"
        detail="Static facts are sent on enrollment or content change"
      />
      <dl class="facts">
        <div>
          <dt>Operating system</dt>
          <dd>{server.os || "Unsupported"}</dd>
        </div>
        <div>
          <dt>Kernel</dt>
          <dd>{server.kernel || "Unsupported"}</dd>
        </div>
        <div>
          <dt>Architecture</dt>
          <dd>{server.arch || "Unsupported"}</dd>
        </div>
        <div>
          <dt>Observation mode</dt>
          <dd>Unprivileged, outbound-only</dd>
        </div>
      </dl>
      <CoverageList items={server.coverage} />
    </section>
  );
}

function DataTab({ id, endpoint, title, render: View, lease = false }: any) {
  const [data, setData] = useState<any>();
  const [leaseState, setLeaseState] = useState<any>();
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [clock, setClock] = useState(() => Math.floor(Date.now() / 1000));
  const loadLease = () =>
    api(`/api/v1/servers/${encodeURIComponent(id)}/leases`)
      .then(setLeaseState)
      .catch((reason) => setNotice(reason.message));
  useEffect(() => {
    api(`/api/v1/servers/${encodeURIComponent(id)}/${endpoint}`)
      .then(setData)
      .catch((reason) => setError(reason.message));
  }, [id, endpoint]);
  useEffect(() => {
    if (!lease) return;
    void loadLease();
    const clockTimer = window.setInterval(
      () => setClock(Math.floor(Date.now() / 1000)),
      1000,
    );
    const statusTimer = window.setInterval(() => void loadLease(), 10_000);
    return () => {
      clearInterval(clockTimer);
      clearInterval(statusTimer);
    };
  }, [id, lease]);
  if (error) return <ErrorState message={error} />;
  if (!data) return <Loading />;
  const requestLease = async () => {
    if (
      !confirm(
        "Enable a typed read-only detail profile for 10 minutes? This increases bandwidth and expires automatically.",
      )
    )
      return;
    try {
      const value = await api<any>(
        `/api/v1/servers/${encodeURIComponent(id)}/leases`,
        {
          method: "POST",
          body: JSON.stringify({
            profile:
              endpoint === "processes" ? "process_snapshot" : "socket_snapshot",
            duration_secs: 600,
          }),
        },
      );
      setLeaseState(value);
      setNotice(
        `Lease ${value.lease_id} active until ${new Date(value.expires_at * 1000).toLocaleTimeString()}.`,
      );
    } catch (reason) {
      setNotice((reason as Error).message);
    }
  };
  const cancelLease = async () => {
    if (!leaseState?.lease_id) return;
    try {
      const value = await api<any>(
        `/api/v1/servers/${encodeURIComponent(id)}/leases/${encodeURIComponent(leaseState.lease_id)}`,
        { method: "DELETE" },
      );
      setLeaseState({ ...leaseState, ...value });
      setNotice(
        "Lease cancelled. The Agent returns to normal mode on its next outbound acknowledgement.",
      );
    } catch (reason) {
      setNotice((reason as Error).message);
    }
  };
  const active =
    leaseState?.state === "active" && leaseState.expires_at > clock;
  const remaining = active ? Math.max(0, leaseState.expires_at - clock) : 0;
  return (
    <section class="panel">
      <PanelHead
        title={title}
        detail={`Snapshot ${data.observed_at ? relative(data.observed_at) : "not available"}`}
      />
      {lease && (
        <div class="lease">
          <div>
            <strong>
              {active
                ? `Read-only detail active · ${Math.floor(remaining / 60)}:${String(remaining % 60).padStart(2, "0")} remaining`
                : "Normal mode minimizes bandwidth"}
            </strong>
            {active ? (
              <span>
                {leaseState.response_count || 0} response(s),{" "}
                {bytes(leaseState.encoded_response_bytes || 0)} measured body
                bytes. Automatic expiry is enforced by both Hub and Agent.
              </span>
            ) : (
              <span>
                Full process/socket detail uses a closed profile, adds at most
                256 KiB per response, and expires within 10 minutes.
              </span>
            )}
          </div>
          {active ? (
            <button onClick={cancelLease}>End detail early</button>
          ) : (
            <button onClick={requestLease}>
              Request temporary live detail
            </button>
          )}
        </div>
      )}
      {notice && (
        <p role="status" class="notice">
          {notice}
        </p>
      )}
      <View value={data.data ?? data.items ?? []} />
    </section>
  );
}

function TrafficTab({ id }: { id: string }) {
  const [usage, setUsage] = useState<any>();
  const [error, setError] = useState("");
  const [amount, setAmount] = useState("");
  const [amountUnit, setAmountUnit] = useState("GiB");
  const [seedPreview, setSeedPreview] = useState(false);
  const [note, setNote] = useState("Provider dashboard current-cycle usage");
  const [adjustment, setAdjustment] = useState("");
  const [adjustmentReason, setAdjustmentReason] = useState("");
  const [timezone, setTimezone] = useState("UTC");
  const [anchorDay, setAnchorDay] = useState("1");
  const [anchorTime, setAnchorTime] = useState("00:00");
  const [limit, setLimit] = useState("");
  const [selectedInterfaces, setSelectedInterfaces] = useState("");
  const [excludedInterfaces, setExcludedInterfaces] = useState("");
  const load = () =>
    api<any>(`/api/v1/servers/${encodeURIComponent(id)}/traffic`)
      .then(setUsage)
      .catch((reason) => setError(reason.message));
  useEffect(() => {
    void load();
  }, [id]);
  useEffect(() => {
    if (!usage || usage.state === "awaiting_checkpoint") return;
    setTimezone(usage.timezone || "UTC");
    setAnchorDay(String(usage.anchor_day || 1));
    setAnchorTime((usage.anchor_time || "00:00").slice(0, 5));
    setLimit(usage.limit_bytes ? String(usage.limit_bytes / 1024 ** 3) : "");
    setSelectedInterfaces(
      (usage.selected_interfaces?.selected || []).join(", "),
    );
    setExcludedInterfaces(
      (usage.selected_interfaces?.excluded || []).join(", "),
    );
  }, [usage?.cycle_id]);
  if (error) return <ErrorState message={error} />;
  if (!usage) return <Loading />;
  if (usage.state === "awaiting_checkpoint")
    return (
      <Empty
        title="Awaiting the first traffic checkpoint"
        detail="A manual provider seed must be tied to a reliable Agent checkpoint."
      />
    );
  const seedBytes = Number(amount) * (trafficUnits[amountUnit] ?? 0);
  const submit = async (event: Event) => {
    event.preventDefault();
    const value = seedBytes;
    if (!Number.isFinite(value) || value < 0) return;
    if (!seedPreview) {
      setSeedPreview(true);
      return;
    }
    try {
      setUsage(
        await api(`/api/v1/servers/${encodeURIComponent(id)}/traffic/seed`, {
          method: "POST",
          body: JSON.stringify({
            combined_bytes: Math.round(value),
            effective_at: usage.checkpoint_at,
            note,
          }),
        }),
      );
      setSeedPreview(false);
    } catch (reason) {
      setError((reason as Error).message);
    }
  };
  const submitAdjustment = async (event: Event) => {
    event.preventDefault();
    const value = Number(adjustment) * 1024 ** 3;
    if (!Number.isFinite(value) || !adjustmentReason.trim()) return;
    try {
      setUsage(
        await api(
          `/api/v1/servers/${encodeURIComponent(id)}/traffic/adjustments`,
          {
            method: "POST",
            body: JSON.stringify({
              signed_bytes: Math.round(value),
              effective_at: usage.checkpoint_at,
              reason: adjustmentReason,
            }),
          },
        ),
      );
      setAdjustment("");
      setAdjustmentReason("");
    } catch (reason) {
      setError((reason as Error).message);
    }
  };
  const submitRule = async (event: Event) => {
    event.preventDefault();
    try {
      await api(`/api/v1/servers/${encodeURIComponent(id)}/traffic/rule`, {
        method: "PUT",
        body: JSON.stringify({
          timezone,
          anchor_day: Number(anchorDay),
          anchor_time: anchorTime,
          selected_interfaces: selectedInterfaces
            .split(",")
            .map((value) => value.trim())
            .filter(Boolean),
          excluded_interfaces: excludedInterfaces
            .split(",")
            .map((value) => value.trim())
            .filter(Boolean),
          traffic_limit_bytes: limit
            ? Math.round(Number(limit) * 1024 ** 3)
            : null,
        }),
      });
      await load();
    } catch (reason) {
      setError((reason as Error).message);
    }
  };
  return (
    <>
      <section class="metric-grid traffic-metrics">
        <Metric
          label="Cycle total"
          value={bytes(usage.total_bytes)}
          tone="accent"
          detail={
            usage.limit_bytes
              ? `${percent(usage.total_bytes, usage.limit_bytes)} of limit`
              : "No limit configured"
          }
        />
        <Metric
          label="Manual seed"
          value={bytes(usage.seed_bytes)}
          detail="Provider dashboard"
        />
        <Metric
          label="Parade observed"
          value={bytes(usage.observed_bytes)}
          detail="Since seed checkpoint"
        />
        <Metric
          label="Adjustments"
          value={bytes(Math.abs(usage.adjustment_bytes))}
          detail={
            usage.adjustment_bytes < 0
              ? "Negative correction"
              : "Audited correction"
          }
        />
        <Metric
          label="Projection"
          value={
            usage.projected_bytes == null
              ? "Insufficient history"
              : bytes(usage.projected_bytes)
          }
          tone={
            usage.limit_bytes &&
            usage.projected_bytes != null &&
            usage.projected_bytes > usage.limit_bytes
              ? "warn"
              : undefined
          }
          detail="Observed rate through cycle end"
        />
      </section>
      <section class="split">
        <div class="panel">
          <PanelHead
            title="Transparent accounting"
            detail="Seed + observed + adjustments = current total"
          />
          <div class="equation">
            <span>
              {bytes(usage.seed_bytes)}
              <small>manual seed</small>
            </span>
            <b>+</b>
            <span>
              {bytes(usage.observed_bytes)}
              <small>locally observed</small>
            </span>
            <b>+</b>
            <span>
              {bytes(usage.adjustment_bytes)}
              <small>adjustments</small>
            </span>
            <b>=</b>
            <span>
              {bytes(usage.total_bytes)}
              <small>cycle total</small>
            </span>
          </div>
          <dl class="facts">
            <div>
              <dt>Cycle</dt>
              <dd>
                {new Date(usage.cycle_start * 1000).toLocaleString()} –{" "}
                {new Date(usage.cycle_end * 1000).toLocaleString()}
              </dd>
            </div>
            <div>
              <dt>Confidence</dt>
              <dd>
                <Status value={usage.confidence} />
              </dd>
            </div>
            <div>
              <dt>Interfaces</dt>
              <dd class="mono">{JSON.stringify(usage.selected_interfaces)}</dd>
            </div>
            <div>
              <dt>Observed direction</dt>
              <dd>
                {bytes(usage.observed_rx_bytes)} inbound ·{" "}
                {bytes(usage.observed_tx_bytes)} outbound
              </dd>
            </div>
            <div>
              <dt>Observation window</dt>
              <dd>
                {new Date(usage.observation_start_at * 1000).toLocaleString()} –{" "}
                {new Date(usage.checkpoint_at * 1000).toLocaleString()}
              </dd>
            </div>
            <div>
              <dt>Provider seed source</dt>
              <dd>
                {usage.has_manual_seed
                  ? `${usage.seed_note || "Manual entry"} · ${new Date(usage.seed_effective_at * 1000).toLocaleString()}`
                  : "No manual seed entered"}
              </dd>
            </div>
          </dl>
          <p class="caveat">
            Parade measures selected Linux interface bytes. Provider billing can
            differ due to overhead, direction weighting, rounding and private
            traffic policy.
          </p>
        </div>
        {!usage.has_manual_seed ? (
          <form class="panel form" onSubmit={submit}>
            <PanelHead
              title="Enter current provider usage"
              detail="Creates one immutable primary seed at the latest checkpoint"
            />
            <label>
              Current provider-used traffic
              <span class="amount-input">
                <input
                  aria-label="Current provider-used amount"
                  type="number"
                  min="0"
                  step="0.01"
                  value={amount}
                  onInput={(event) => {
                    setAmount(event.currentTarget.value);
                    setSeedPreview(false);
                  }}
                  required
                />
                <select
                  aria-label="Traffic unit"
                  value={amountUnit}
                  onChange={(event) => {
                    setAmountUnit(event.currentTarget.value);
                    setSeedPreview(false);
                  }}
                >
                  {Object.keys(trafficUnits).map((unit) => (
                    <option value={unit}>{unit}</option>
                  ))}
                </select>
              </span>
            </label>
            <label>
              Effective checkpoint{" "}
              <input
                value={new Date(usage.checkpoint_at * 1000).toLocaleString()}
                disabled
              />
            </label>
            <label>
              Source note{" "}
              <input
                value={note}
                maxLength={500}
                onInput={(event) => setNote(event.currentTarget.value)}
              />
            </label>
            {seedPreview && (
              <div class="seed-preview" role="status">
                <strong>Confirm immutable seed</strong>
                <span>
                  Provider entry: {amount} {amountUnit} ({bytes(seedBytes)})
                </span>
                <span>
                  Agent checkpoint: {bytes(usage.agent_observed_total_bytes)}
                  {" at "}
                  {new Date(usage.checkpoint_at * 1000).toLocaleString()}
                </span>
                <span>
                  Cycle: {new Date(usage.cycle_start * 1000).toLocaleString()}
                  {" – "}
                  {new Date(usage.cycle_end * 1000).toLocaleString()}
                </span>
                <span>
                  Result after saving: {bytes(seedBytes)} + future
                  selected-interface traffic
                </span>
              </div>
            )}
            <button class="primary" type="submit">
              {seedPreview ? "Confirm and save seed" : "Preview seed"}
            </button>
            <small>
              Mistakes are corrected with an append-only audited adjustment;
              history is never silently rewritten.
            </small>
          </form>
        ) : (
          <form class="panel form" onSubmit={submitAdjustment}>
            <PanelHead
              title="Append an audited adjustment"
              detail="Corrections preserve the original seed and full history"
            />
            <label>
              Signed correction (GiB)
              <input
                type="number"
                step="0.01"
                value={adjustment}
                onInput={(event) => setAdjustment(event.currentTarget.value)}
                required
              />
            </label>
            <label>
              Reason
              <input
                value={adjustmentReason}
                minLength={3}
                maxLength={500}
                onInput={(event) =>
                  setAdjustmentReason(event.currentTarget.value)
                }
                required
              />
            </label>
            <button class="primary" type="submit">
              Append adjustment
            </button>
          </form>
        )}
      </section>
      <form class="panel form" onSubmit={submitRule}>
        <PanelHead
          title="Billing-cycle rule"
          detail="IANA timezone, calendar anchor, and optional provider limit"
        />
        <div class="form-row">
          <label>
            IANA timezone
            <input
              value={timezone}
              onInput={(event) => setTimezone(event.currentTarget.value)}
              required
            />
          </label>
          <label>
            Anchor day
            <input
              type="number"
              min="1"
              max="31"
              value={anchorDay}
              onInput={(event) => setAnchorDay(event.currentTarget.value)}
              required
            />
          </label>
          <label>
            Local anchor time
            <input
              type="time"
              value={anchorTime}
              onInput={(event) => setAnchorTime(event.currentTarget.value)}
              required
            />
          </label>
          <label>
            Traffic limit (GiB, optional)
            <input
              type="number"
              min="0"
              step="0.01"
              value={limit}
              onInput={(event) => setLimit(event.currentTarget.value)}
            />
          </label>
          <label>
            Selected interfaces (comma-separated; blank = automatic)
            <input
              value={selectedInterfaces}
              onInput={(event) =>
                setSelectedInterfaces(event.currentTarget.value)
              }
              placeholder="eth0, ens3"
            />
          </label>
          <label>
            Excluded interfaces (comma-separated)
            <input
              value={excludedInterfaces}
              onInput={(event) =>
                setExcludedInterfaces(event.currentTarget.value)
              }
              placeholder="wg0"
            />
          </label>
        </div>
        <button class="primary" type="submit">
          Save cycle rule
        </button>
        <small>
          Interface auto-selection follows the default route and excludes
          loopback, container, bridge, veth, and tunnel devices. Current
          selected identities remain visible above.
        </small>
      </form>
    </>
  );
}

function Settings() {
  const [id, setId] = useState("");
  const [name, setName] = useState("");
  const [message, setMessage] = useState("");
  const [command, setCommand] = useState("");
  const create = async (event: Event) => {
    event.preventDefault();
    try {
      const created = await api<{ id: string }>("/api/v1/servers", {
        method: "POST",
        body: JSON.stringify({ id, name, group: "" }),
      });
      const enrollment = await api<{ command: string; expires_at: number }>(
        `/api/v1/servers/${encodeURIComponent(created.id)}/enrollment`,
        { method: "POST" },
      );
      setCommand(enrollment.command);
      setMessage(
        `Created ${id}. This single-use enrollment command expires ${new Date(enrollment.expires_at * 1000).toLocaleTimeString()}.`,
      );
      setId("");
      setName("");
    } catch (reason) {
      setMessage((reason as Error).message);
    }
  };
  return (
    <section class="settings-grid">
      <form class="panel form" onSubmit={create}>
        <PanelHead
          title="Agent enrollment"
          detail="Create a server record before enrolling one independent identity"
        />
        <label>
          Server ID
          <input
            value={id}
            pattern="[A-Za-z0-9._-]+"
            maxLength={64}
            onInput={(e) => setId(e.currentTarget.value)}
            required
          />
        </label>
        <label>
          Display name
          <input
            value={name}
            maxLength={100}
            onInput={(e) => setName(e.currentTarget.value)}
            required
          />
        </label>
        <button class="primary">Create server record</button>
        {message && (
          <p role="status" class="notice">
            {message}
          </p>
        )}
        {command && (
          <pre class="enrollment-command" tabIndex={0}>
            {command}
          </pre>
        )}
      </form>
      <div class="panel">
        <PanelHead
          title="Security defaults"
          detail="Hub metadata may change; monitored hosts remain observation-only"
        />
        <ul class="checklist">
          <li>Argon2id administrator authentication</li>
          <li>Strict SameSite session and CSRF validation</li>
          <li>Explicit trusted proxy addresses only</li>
          <li>SQLite WAL with transactional migrations</li>
          <li>Independent revocable Agent credentials</li>
        </ul>
      </div>
      <div class="panel">
        <PanelHead title="Backup and restore" detail="Operational guidance" />
        <p class="prose">
          Use SQLite's online backup command or stop the Hub before copying the
          database, including WAL state. Test restores on a disposable Hub.
          Agent credentials remain bound to the restored server records.
        </p>
      </div>
      <AuditPanel />
    </section>
  );
}

function AuditPanel() {
  const [items, setItems] = useState<any[]>();
  const [error, setError] = useState("");
  useEffect(() => {
    api<{ items: any[] }>("/api/v1/audit")
      .then((value) => setItems(value.items))
      .catch((reason) => setError(reason.message));
  }, []);
  return (
    <div class="panel">
      <PanelHead
        title="Operator audit log"
        detail="Append-only Hub metadata and observation-profile changes"
      />
      {error ? (
        <p class="notice">{error}</p>
      ) : !items ? (
        <Loading />
      ) : items.length ? (
        <div class="audit-list">
          {items.slice(0, 20).map((item) => (
            <div>
              <time>{relative(item.occurred_at)}</time>
              <strong>{item.action}</strong>
              <span>{item.server_id || "Hub"}</span>
            </div>
          ))}
        </div>
      ) : (
        <Empty
          title="No operator changes yet"
          detail="Enrollment, traffic, lease and server mutations appear here."
        />
      )}
    </div>
  );
}

function ResourceView({ value }: any) {
  return value && typeof value === "object" ? (
    <div class="resource-grid">
      <Metric
        label="CPU average"
        value={`${Number(value.cpu_avg_pct || 0).toFixed(1)}%`}
        detail={`Peak ${Number(value.cpu_max_pct || 0).toFixed(1)}%`}
      />
      <Metric
        label="Memory"
        value={bytes(value.mem_used)}
        detail={`${bytes(value.mem_total)} total`}
      />
      <Metric
        label="Disk"
        value={bytes(value.disk_used)}
        detail={`${bytes(value.disk_total)} total`}
      />
      <Metric
        label="Pressure"
        value={
          value.psi_cpu_some_avg10 == null
            ? "Unsupported"
            : `${value.psi_cpu_some_avg10}%`
        }
        detail="CPU PSI some avg10"
      />
    </div>
  ) : (
    <Empty
      title="No resource rollup"
      detail="The Agent has not submitted this collector."
    />
  );
}
function ProcessTable({ value }: any) {
  const [query, setQuery] = useState("");
  const [suspiciousOnly, setSuspiciousOnly] = useState(false);
  const rows = Array.isArray(value)
    ? value.filter((process: any) => {
        const matches =
          `${process.pid} ${process.uid} ${process.executable} ${process.cgroup || ""} ${process.systemd_unit || ""}`
            .toLowerCase()
            .includes(query.toLowerCase());
        return (
          matches &&
          (!suspiciousOnly ||
            process.suspicious_writable_path ||
            process.deleted_executable ||
            process.package_ownership === "unowned")
        );
      })
    : [];
  return Array.isArray(value) && value.length ? (
    <>
      <p class="privacy">
        Full command lines and environment variables are never collected. Normal
        mode sends bounded top-N and suspicious facts.
      </p>
      <div class="mini-toolbar">
        <label>
          <span class="sr-only">Search process facts</span>
          <input
            type="search"
            placeholder="Search PID, UID, executable or cgroup"
            value={query}
            onInput={(event) => setQuery(event.currentTarget.value)}
          />
        </label>
        <label>
          <input
            type="checkbox"
            checked={suspiciousOnly}
            onChange={(event) => setSuspiciousOnly(event.currentTarget.checked)}
          />{" "}
          Suspicious only
        </label>
      </div>
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              <th>State</th>
              <th>PID</th>
              <th>PPID</th>
              <th>UID</th>
              <th>Executable</th>
              <th>CPU ticks</th>
              <th>RSS</th>
              <th>Virtual</th>
              <th>Unit / cgroup</th>
              <th>Listeners</th>
              <th>Package</th>
              <th>Evidence</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((p: any) => (
              <tr>
                <td>{p.state}</td>
                <td class="mono">{p.pid}</td>
                <td class="mono">{p.ppid ?? "—"}</td>
                <td class="mono">{p.uid}</td>
                <td class="mono">{p.executable}</td>
                <td class="mono">{p.cpu_ticks ?? "—"}</td>
                <td>{bytes(p.rss_bytes)}</td>
                <td>{bytes(p.virtual_bytes)}</td>
                <td class="mono">{p.systemd_unit || p.cgroup || "—"}</td>
                <td>{p.listening_sockets ?? "—"}</td>
                <td>{p.package_ownership || "unknown"}</td>
                <td>
                  <span class="evidence-tags">
                    {p.deleted_executable && (
                      <span class="tag bad">deleted executable</span>
                    )}
                    {p.suspicious_writable_path && (
                      <span class="tag warn">writable path</span>
                    )}
                    {!p.deleted_executable &&
                      !p.suspicious_writable_path &&
                      "—"}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {rows.length === 0 && (
        <p class="notice">No process facts match the current filters.</p>
      )}
    </>
  ) : (
    <Empty
      title="No process changes in this rollup"
      detail="Normal mode sends only changed or suspicious bounded summaries."
    />
  );
}
function NetworkView({ value }: any) {
  const interfaces = Array.isArray(value?.interfaces) ? value.interfaces : [];
  const listeners = Array.isArray(value?.listeners) ? value.listeners : [];
  return (
    <>
      <div class="resource-grid">
        <Metric
          label="Inbound interval"
          value={bytes(value?.observed_rx_delta)}
          detail="Selected interfaces"
        />
        <Metric
          label="Outbound interval"
          value={bytes(value?.observed_tx_delta)}
          detail="Selected interfaces"
        />
        <Metric
          label="Counter confidence"
          value={value?.confidence || "unsupported"}
          detail="Raw counters are never reset"
        />
      </div>
      {interfaces.length ? (
        <>
          <h3 class="section-label">Interfaces</h3>
          <div class="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Interface</th>
                  <th>Accounting</th>
                  <th>RX</th>
                  <th>TX</th>
                  <th>Packets RX / TX</th>
                  <th>Errors RX / TX</th>
                  <th>Drops RX / TX</th>
                </tr>
              </thead>
              <tbody>
                {interfaces.map((item: any) => (
                  <tr>
                    <td class="mono">{item.name}</td>
                    <td>{item.selected ? "Selected" : "Observed only"}</td>
                    <td>{bytes(item.rx_bytes)}</td>
                    <td>{bytes(item.tx_bytes)}</td>
                    <td class="mono">
                      {item.rx_packets} / {item.tx_packets}
                    </td>
                    <td class="mono">
                      {item.rx_errors} / {item.tx_errors}
                    </td>
                    <td class="mono">
                      {item.rx_drops} / {item.tx_drops}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      ) : (
        <Empty
          title="No interface counters observed"
          detail="The collector may be unsupported or awaiting its first signed report."
        />
      )}
      {Array.isArray(value?.anomaly_flags) &&
        value.anomaly_flags.length > 0 && (
          <p class="notice">{value.anomaly_flags.join(" · ")}</p>
        )}
      <h3 class="section-label">Listening ports</h3>
      <ListenerTable value={listeners} />
    </>
  );
}
function ListenerTable({ value }: any) {
  return Array.isArray(value) && value.length ? (
    <div class="table-wrap">
      <table>
        <thead>
          <tr>
            <th>Protocol</th>
            <th>Bind address</th>
            <th>Port</th>
            <th>UID</th>
            <th>Socket inode</th>
          </tr>
        </thead>
        <tbody>
          {value.map((p: any) => (
            <tr>
              <td>{p.protocol}</td>
              <td class="mono">{p.local_address}</td>
              <td class="mono">{p.port}</td>
              <td>{p.uid ?? "Unknown"}</td>
              <td class="mono">{p.inode ?? "—"}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  ) : (
    <Empty
      title="No listening sockets observed"
      detail="Coverage and permissions determine completeness; this does not prove the network surface is empty."
    />
  );
}
function FindingList({ value }: any) {
  return Array.isArray(value) && value.length ? (
    <>
      <p class="privacy">
        No finding is proof that the host is safe or compromised. Host-local
        telemetry may be falsified by a sufficiently privileged attacker.
      </p>
      <div class="finding-list">
        {value.map((f: any) => (
          <article>
            <header>
              <span class={`tag ${f.severity}`}>{f.severity}</span>
              <strong>
                {f.rule_id} <small>v{f.rule_version}</small>
              </strong>
              <span>
                {f.confidence} confidence · {f.occurrences} occurrence(s)
              </span>
            </header>
            {f.server_name && (
              <a
                class="text-link"
                href={`#/servers/${encodeURIComponent(f.server_id)}/security`}
              >
                {f.server_name} · first {relative(f.first_seen)} · last{" "}
                {relative(f.last_seen)}
              </a>
            )}
            <code>{f.evidence}</code>
            <p>{f.explanation}</p>
            <details>
              <summary>Manual verification and caveats</summary>
              <p>{f.verification}</p>
              <p>{f.coverage_caveat}</p>
            </details>
          </article>
        ))}
      </div>
    </>
  ) : (
    <Empty
      title="No active findings in retained telemetry"
      detail="No finding is proof that the host is safe or compromised. Privileged attackers may falsify host-local telemetry."
    />
  );
}
function EventList({ value }: any) {
  return Array.isArray(value) && value.length ? (
    <div class="event-list">
      {value.map((e: any) => (
        <article>
          <time>{new Date(e.occurred_at * 1000).toLocaleString()}</time>
          <span class="tag">{e.category}</span>
          <strong>{e.summary}</strong>
          {e.server_name && <span>{e.server_name}</span>}
          <code>{e.evidence}</code>
        </article>
      ))}
    </div>
  ) : (
    <Empty
      title="No retained events"
      detail="Availability, identity, traffic and finding transitions will appear here."
    />
  );
}
function CoverageList({ items }: { items: any[] }) {
  return (
    <div class="coverage-list">
      {items?.length ? (
        items.map((item) => (
          <div>
            <Status value={item.status} />
            <strong>{item.collector}</strong>
            <span>{item.detail}</span>
          </div>
        ))
      ) : (
        <Empty
          title="Coverage pending"
          detail="Waiting for the first signed Agent report."
        />
      )}
    </div>
  );
}
function coverage(server: FleetServer): string {
  return server.coverage.length &&
    server.coverage.every((item) => item.status === "high")
    ? "Full"
    : server.coverage.length
      ? "Partial"
      : "Pending";
}
function Metric({ label, value, detail, tone = "" }: any) {
  return (
    <article class={`metric ${tone}`}>
      <span>{label}</span>
      <strong>{value}</strong>
      <small>{detail}</small>
    </article>
  );
}
function PanelHead({ title, detail }: { title: string; detail: string }) {
  return (
    <header class="panel-head">
      <div>
        <h2>{title}</h2>
        <p>{detail}</p>
      </div>
    </header>
  );
}
function Loading() {
  return (
    <div class="loading" aria-live="polite">
      <span />
      <span />
      <span />
      <p>Loading the latest accepted telemetry…</p>
    </div>
  );
}
function Empty({ title, detail }: { title: string; detail: string }) {
  return (
    <div class="empty">
      <span aria-hidden="true">○</span>
      <strong>{title}</strong>
      <p>{detail}</p>
    </div>
  );
}
function ErrorState({ message }: { message: string }) {
  return (
    <div class="error-state" role="alert">
      <strong>Data could not be loaded</strong>
      <p>
        {message}. Previously displayed data may be stale; retrying is safe.
      </p>
      <button onClick={() => location.reload()}>Retry</button>
    </div>
  );
}

render(<Shell />, document.getElementById("app")!);
