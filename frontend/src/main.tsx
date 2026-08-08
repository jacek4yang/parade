import { render } from "preact";
import { useEffect, useState } from "preact/hooks";
import {
  api,
  type FleetResponse,
  type FleetServer,
  type TopologyResponse,
} from "./api";
import { bytes, dateTime, number, percent, relative, timeOnly } from "./format";
import {
  initialLocale,
  LOCALE_STORAGE_KEY,
  setLocale,
  t,
  type Locale,
} from "./i18n";
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

const billingModeLabels: Record<string, string> = {
  sum: "Inbound + outbound sum",
  inbound_only: "Inbound only",
  outbound_only: "Outbound only",
  max_direction: "Larger direction",
  separate_directions: "Separate inbound and outbound",
};

function billingModeLabel(value: string): string {
  return t(billingModeLabels[value] || value);
}

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

function useFleet(limit = 500, offset = 0, query = "") {
  const [data, setData] = useState<FleetResponse>();
  const [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    setError("");
    setData(undefined);
    const load = () =>
      api<FleetResponse>(
        `/api/v1/fleet?limit=${limit}&offset=${offset}&q=${encodeURIComponent(query)}`,
      )
        .then((value) => active && setData(value))
        .catch((reason) => active && setError(reason.message));
    void load();
    const timer = window.setInterval(() => void load(), 30_000);
    return () => {
      active = false;
      clearInterval(timer);
    };
  }, [limit, offset, query]);
  return { data, error };
}

function Status({ value }: { value: string }) {
  return (
    <span class={`status status-${value}`}>
      <span aria-hidden="true" class="status-dot" /> {t(value)}
    </span>
  );
}

function Shell() {
  const parts = useRoute();
  const page = parts[0] ?? "overview";
  const [locale, setLocaleState] = useState<Locale>(() => initialLocale());
  setLocale(locale);
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
    document.documentElement.lang = locale;
  }, [theme, compact, locale]);
  const pageLabel = nav.find((item) => item.toLowerCase() === page) ?? page;
  const title = t(parts[0] === "servers" ? "Server detail" : pageLabel);
  useEffect(() => {
    document.title = `${title} · Parade`;
  }, [title]);
  return (
    <div class="shell">
      <a class="skip-link" href="#content">
        {t("Skip to content")}
      </a>
      <aside
        id="primary-navigation"
        class={menu ? "sidebar sidebar-open" : "sidebar"}
        aria-label={t("Primary navigation")}
      >
        <div class="brand">
          <span class="brand-mark">P</span>
          <span>
            <strong>Parade</strong>
            <small>{t("Fleet observability")}</small>
          </span>
        </div>
        <div class="readonly">
          <span aria-hidden="true">◉</span> {t("Read-only monitoring")}
        </div>
        <nav>
          {nav.map((item) => (
            <a
              class={page === item.toLowerCase() ? "active" : ""}
              href={`#/${item.toLowerCase()}`}
              onClick={() => setMenu(false)}
            >
              {t(item)}
            </a>
          ))}
        </nav>
        <div class="sidebar-foot">{t("No remote control or remediation")}</div>
      </aside>
      <div class="workspace">
        <header class="topbar">
          <button
            class="menu-button"
            aria-label={t("Open navigation")}
            aria-expanded={menu}
            aria-controls="primary-navigation"
            onClick={() => setMenu(!menu)}
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M4 7h16M4 12h16M4 17h16" />
            </svg>
          </button>
          <div>
            <span class="eyebrow">Parade / {t(pageLabel)}</span>
            <h1>{title}</h1>
          </div>
          <div class="top-actions">
            <span class="freshness">{t("Authenticated Hub session")}</span>
            <label class="language-select">
              <span class="sr-only">{t("Language")}</span>
              <select
                aria-label={t("Language")}
                value={locale}
                onChange={(event) => {
                  const next = event.currentTarget.value as Locale;
                  localStorage.setItem(LOCALE_STORAGE_KEY, next);
                  setLocale(next);
                  setLocaleState(next);
                }}
              >
                <option value="en">English</option>
                <option value="zh-CN">简体中文</option>
              </select>
            </label>
            <button
              onClick={() => {
                const next = compact ? "comfortable" : "compact";
                localStorage.setItem("parade-density", next);
                setCompact(!compact);
              }}
              aria-label={t("Toggle display density")}
            >
              {t(compact ? "Comfortable" : "Compact")}
            </button>
            <button
              onClick={() => {
                const next = theme === "dark" ? "light" : "dark";
                localStorage.setItem("parade-theme", next);
                setTheme(next);
              }}
              aria-label={t("Toggle color theme")}
            >
              {t(theme === "dark" ? "Light" : "Dark")}
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
  const [topology, setTopology] = useState<TopologyResponse>();
  const [topologyError, setTopologyError] = useState("");
  useEffect(() => {
    api<TopologyResponse>("/api/v1/topology")
      .then(setTopology)
      .catch((reason) => setTopologyError(reason.message));
  }, []);
  if (error) return <ErrorState message={error} />;
  if (!data) return <Loading />;
  const counts = data.summary;
  const evidenceCount = counts.active_findings || 0;
  const pressureCount = counts.resource_pressure || 0;
  const attention = data.servers
    .filter(
      (server) =>
        server.status !== "online" ||
        server.coverage.some((item) => item.status !== "high") ||
        (server.active_findings || 0) > 0 ||
        server.traffic_confidence === "partial" ||
        server.traffic_confidence === "estimated" ||
        (server.resources?.cpu_avg_pct || 0) >= 90 ||
        (!!server.resources?.mem_total &&
          (server.resources.mem_used || 0) / server.resources.mem_total >=
            0.9) ||
        (!!server.resources?.disk_total &&
          (server.resources.disk_used || 0) / server.resources.disk_total >=
            0.9),
    )
    .slice(0, 8);
  return (
    <>
      <section class="metric-grid" aria-label={t("Fleet status summary")}>
        <Metric
          label="Online"
          value={counts.online}
          tone="ok"
          detail={t("{count} total", { count: number(data.total) })}
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
        <Metric
          label="Active finding evidence"
          value={evidenceCount}
          tone={evidenceCount ? "warn" : undefined}
          detail="Evidence for review, never a security score"
        />
        <Metric
          label="Resource pressure"
          value={pressureCount}
          tone={pressureCount ? "warn" : undefined}
          detail="CPU, memory or disk above review threshold"
        />
      </section>
      <section class="panel topology-panel">
        <PanelHead
          title="Observed reporting topology"
          detail="Verified outbound Agent reports to this Hub"
        />
        {topology ? (
          <ConnectionTopology value={topology} />
        ) : topologyError ? (
          <p class="caveat">{topologyError}</p>
        ) : (
          <Loading />
        )}
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
                  {(server.active_findings || 0) > 0
                    ? t("{count} active evidence finding(s)", {
                        count: server.active_findings,
                      })
                    : (server.resources?.cpu_avg_pct || 0) >= 90
                      ? t("Sustained CPU pressure")
                      : (!!server.resources?.mem_total &&
                            (server.resources.mem_used || 0) /
                              server.resources.mem_total >=
                              0.9) ||
                          (!!server.resources?.disk_total &&
                            (server.resources.disk_used || 0) /
                              server.resources.disk_total >=
                              0.9)
                        ? t("Memory or disk pressure")
                        : server.traffic_confidence === "partial" ||
                            server.traffic_confidence === "estimated"
                          ? t("Traffic accounting uncertainty")
                          : server.coverage.some(
                                (item) => item.status !== "high",
                              )
                            ? t("Partial telemetry coverage")
                            : t("Reporting gap")}
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
                <span>{t(state)}</span>
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
            {t(
              "Parade observes selected host-local telemetry. A sufficiently privileged attacker can falsify it. No absence of findings is proof that a host is safe.",
            )}
          </p>
          <a class="text-link" href="#/security">
            {t("Review coverage gaps →")}
          </a>
        </div>
      </section>
    </>
  );
}

function ConnectionTopology({ value }: { value: TopologyResponse }) {
  return (
    <>
      <div class="topology">
        <div class="topology-hub">
          <span class="brand-mark">P</span>
          <strong>{value.hub.label}</strong>
          <small>{t("Receives authenticated HTTPS reports")}</small>
        </div>
        <div
          class="topology-edges"
          role="list"
          aria-label={t("Reporting paths")}
        >
          {value.edges.map((edge) => (
            <a
              role="listitem"
              class={`topology-edge topology-${edge.status}`}
              href={`#/servers/${encodeURIComponent(edge.server_id)}/overview`}
            >
              <span class="topology-line" aria-hidden="true">
                →
              </span>
              <span>
                <strong>{edge.server_name}</strong>
                <small>{t(edge.source_category)}</small>
                {edge.source_category === "shared_observed_source" && (
                  <small>
                    {t("{count} Agents share this observed source", {
                      count: number(edge.shared_source_count),
                    })}
                  </small>
                )}
              </span>
              <Status value={edge.status} />
            </a>
          ))}
        </div>
      </div>
      {value.truncated && (
        <p class="caveat">
          {t("Showing {shown} of {total} paths; attention-first and bounded.", {
            shown: number(value.displayed),
            total: number(value.total),
          })}
        </p>
      )}
      <p class="caveat">
        {t(
          "This is not a mesh or reachability scan. Shared or private sources may indicate NAT, a proxy, VPN, or routing policy; even an internet-scope source does not prove inbound reachability.",
        )}
      </p>
    </>
  );
}

function Fleet() {
  const [offset, setOffset] = useState(0);
  const [search, setSearch] = useState("");
  const [query, setQuery] = useState("");
  useEffect(() => {
    const timer = window.setTimeout(() => setQuery(search), 200);
    return () => clearTimeout(timer);
  }, [search]);
  const limit = 100;
  const { data, error } = useFleet(limit, offset, query);
  if (error) return <ErrorState message={error} />;
  if (!data) return <Loading />;
  const visible = data.servers;
  return (
    <section class="panel fleet-panel">
      <PanelHead
        title="Fleet"
        detail={t("{count} registered servers", { count: number(data.total) })}
      />
      <div class="toolbar">
        <label class="search">
          <span class="sr-only">{t("Search fleet")}</span>
          <input
            value={search}
            onInput={(event) => {
              setSearch(event.currentTarget.value);
              setOffset(0);
            }}
            placeholder={t("Search name, ID or group")}
          />
        </label>
        <span>
          {t("Showing {start}–{end}", {
            start: number(offset + 1),
            end: number(Math.min(offset + limit, data.total)),
          })}
        </span>
      </div>
      {visible.length ? (
        <div class="table-wrap">
          <table>
            <thead>
              <tr>
                <th>{t("Status")}</th>
                <th>{t("Server")}</th>
                <th>{t("Group")}</th>
                <th>{t("System")}</th>
                <th>{t("CPU")}</th>
                <th>{t("Memory")}</th>
                <th>{t("Disk")}</th>
                <th>{t("Coverage")}</th>
                <th>{t("Findings")}</th>
                <th>{t("Traffic")}</th>
                <th>{t("Last report")}</th>
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
                    {server.os || t("Awaiting inventory")}
                    <small>{server.arch}</small>
                  </td>
                  <td>
                    {server.resources?.cpu_avg_pct == null
                      ? "—"
                      : `${Number(server.resources.cpu_avg_pct).toFixed(1)}%`}
                    <small>{t("average")}</small>
                  </td>
                  <td>
                    {server.resources?.mem_used == null
                      ? "—"
                      : percent(
                          server.resources.mem_used,
                          server.resources.mem_total || 0,
                        )}
                    <small>{bytes(server.resources?.mem_used)}</small>
                  </td>
                  <td>
                    {server.resources?.disk_used == null
                      ? "—"
                      : percent(
                          server.resources.disk_used,
                          server.resources.disk_total || 0,
                        )}
                    <small>{bytes(server.resources?.disk_used)}</small>
                  </td>
                  <td>{coverage(server)}</td>
                  <td>
                    {server.active_findings == null
                      ? "—"
                      : number(server.active_findings)}
                    <small>{t("active evidence")}</small>
                  </td>
                  <td>
                    {server.traffic_confidence ? (
                      <Status value={server.traffic_confidence} />
                    ) : (
                      "—"
                    )}
                  </td>
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
          {t("Previous")}
        </button>
        <button
          disabled={offset + limit >= data.total}
          onClick={() => setOffset(offset + limit)}
        >
          {t("Next")}
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
                  {t("day {day} at {time}", {
                    day: group.anchor_day,
                    time: group.anchor_time,
                  })}
                </span>
                <b>
                  {t("{count} server(s)", { count: number(group.servers) })}
                </b>
              </div>
            ))}
          </div>
        </>
      )}
      <h3 class="section-label">{t("Servers")}</h3>
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
                ? t("Open cycle accounting")
                : page === "security"
                  ? t("{coverage} coverage", { coverage: coverage(server) })
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
            <span class="readonly-badge">{t("Read-only target")}</span>
          </div>
          <p>
            {server.os || t("Inventory pending")} ·{" "}
            {server.kernel || t("kernel unknown")} ·{" "}
            {server.arch || t("architecture unknown")}
          </p>
        </div>
        <div class="server-meta">
          <span>
            {t("Last report")} <strong>{relative(server.last_seen)}</strong>
          </span>
          <span>
            {t("Coverage")} <strong>{coverage(server)}</strong>
          </span>
        </div>
      </section>
      <nav class="tabs" aria-label={t("Server sections")}>
        {tabs.map((name) => (
          <a
            class={tab === name.toLowerCase() ? "active" : ""}
            href={`#/servers/${encodeURIComponent(id)}/${name.toLowerCase()}`}
          >
            {t(name)}
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
  if (tab === "overview") return <ServerOverview id={id} server={server} />;
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
          <dt>{t("Operating system")}</dt>
          <dd>{server.os || t("Unsupported")}</dd>
        </div>
        <div>
          <dt>{t("Kernel")}</dt>
          <dd>{server.kernel || t("Unsupported")}</dd>
        </div>
        <div>
          <dt>{t("Architecture")}</dt>
          <dd>{server.arch || t("Unsupported")}</dd>
        </div>
        <div>
          <dt>{t("Observation mode")}</dt>
          <dd>{t("Unprivileged, outbound-only")}</dd>
        </div>
      </dl>
      <CoverageList items={server.coverage} />
    </section>
  );
}

function ServerOverview({ id, server }: { id: string; server: any }) {
  const [evidence, setEvidence] = useState<any>();
  const [error, setError] = useState("");
  useEffect(() => {
    Promise.all([
      api<any>(`/api/v1/servers/${encodeURIComponent(id)}/resources`),
      api<any>(`/api/v1/servers/${encodeURIComponent(id)}/findings`),
      api<any>(`/api/v1/servers/${encodeURIComponent(id)}/traffic`),
    ])
      .then(([resources, findings, traffic]) =>
        setEvidence({ resources: resources.data, findings, traffic }),
      )
      .catch((reason) => setError(reason.message));
  }, [id]);
  const resources = evidence?.resources;
  const traffic = evidence?.traffic;
  return (
    <>
      {evidence && (
        <section class="metric-grid">
          <Metric
            label="CPU average"
            value={`${Number(resources?.cpu_avg_pct || 0).toFixed(1)}%`}
            detail={t("Peak {value}", {
              value: `${Number(resources?.cpu_max_pct || 0).toFixed(1)}%`,
            })}
          />
          <Metric
            label="Memory"
            value={bytes(resources?.mem_used)}
            detail={percent(
              resources?.mem_used || 0,
              resources?.mem_total || 0,
            )}
          />
          <Metric
            label="Current provider traffic"
            value={
              traffic?.state === "awaiting_checkpoint"
                ? t("Awaiting checkpoint")
                : traffic?.billing_mode === "separate_directions"
                  ? `${bytes(traffic.rx_total_bytes)} / ${bytes(traffic.tx_total_bytes)}`
                  : bytes(traffic?.billed_total_bytes)
            }
            detail={
              traffic?.state === "awaiting_checkpoint"
                ? t("No accounting claim yet")
                : `${billingModeLabel(traffic.billing_mode)} · ${t(traffic.confidence)}`
            }
          />
          <Metric
            label="Active finding evidence"
            value={number(evidence.findings?.items?.length || 0)}
            tone={evidence.findings?.items?.length ? "warn" : undefined}
            detail="Evidence, not a security score"
          />
        </section>
      )}
      {error && <p class="notice">{error}</p>}
      <section class="split">
        <div class="panel">
          <PanelHead
            title="Health summary"
            detail="Latest accepted signed rollup"
          />
          <dl class="facts">
            <div>
              <dt>{t("Availability")}</dt>
              <dd>
                <Status value={server.status} />
              </dd>
            </div>
            <div>
              <dt>{t("Agent")}</dt>
              <dd>{server.agent_version || t("Pending")}</dd>
            </div>
            <div>
              <dt>{t("Inventory fingerprint")}</dt>
              <dd class="mono truncate">
                {server.inventory_hash || t("Pending")}
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
    </>
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
        t(
          "Enable a typed read-only detail profile for 10 minutes? This increases bandwidth and expires automatically.",
        ),
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
        t("Lease {id} active until {time}.", {
          id: value.lease_id,
          time: timeOnly(value.expires_at),
        }),
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
        t(
          "Lease cancelled. The Agent returns to normal mode on its next outbound acknowledgement.",
        ),
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
        detail={t("Snapshot {value}", {
          value: data.observed_at
            ? relative(data.observed_at)
            : t("not available"),
        })}
      />
      {lease && (
        <div class="lease">
          <div>
            <strong>
              {active
                ? t("Read-only detail active · {time} remaining", {
                    time: `${Math.floor(remaining / 60)}:${String(remaining % 60).padStart(2, "0")}`,
                  })
                : t("Normal mode minimizes bandwidth")}
            </strong>
            {active ? (
              <span>
                {t(
                  "{count} response(s), {bytes} measured body bytes. Automatic expiry is enforced by both Hub and Agent.",
                  {
                    count: number(leaseState.response_count || 0),
                    bytes: bytes(leaseState.encoded_response_bytes || 0),
                  },
                )}
              </span>
            ) : (
              <span>
                {t(
                  "Bounded process/socket snapshots use a closed profile, add at most 256 KiB per response, and expire within 10 minutes.",
                )}
              </span>
            )}
          </div>
          {active ? (
            <button onClick={cancelLease}>{t("End detail early")}</button>
          ) : (
            <button onClick={requestLease}>
              {t("Request temporary live detail")}
            </button>
          )}
        </div>
      )}
      {notice && (
        <p role="status" class="notice">
          {notice}
        </p>
      )}
      <View
        value={
          endpoint === "resources" ? data : (data.data ?? data.items ?? [])
        }
      />
    </section>
  );
}

function TrafficTab({ id }: { id: string }) {
  const [usage, setUsage] = useState<any>();
  const [error, setError] = useState("");
  const [amount, setAmount] = useState("");
  const [rxAmount, setRxAmount] = useState("");
  const [txAmount, setTxAmount] = useState("");
  const [amountUnit, setAmountUnit] = useState("GiB");
  const [seedPreview, setSeedPreview] = useState(false);
  const [note, setNote] = useState(() =>
    t("Provider dashboard current-cycle usage"),
  );
  const [adjustment, setAdjustment] = useState("");
  const [adjustmentDirection, setAdjustmentDirection] = useState("billed");
  const [adjustmentReason, setAdjustmentReason] = useState("");
  const [timezone, setTimezone] = useState("UTC");
  const [anchorDay, setAnchorDay] = useState("1");
  const [anchorTime, setAnchorTime] = useState("00:00");
  const [limit, setLimit] = useState("");
  const [rxLimit, setRxLimit] = useState("");
  const [txLimit, setTxLimit] = useState("");
  const [billingMode, setBillingMode] = useState("sum");
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
    setLimit(
      usage.limit_bytes != null ? String(usage.limit_bytes / 1024 ** 3) : "",
    );
    setRxLimit(
      usage.rx_limit_bytes != null
        ? String(usage.rx_limit_bytes / 1024 ** 3)
        : "",
    );
    setTxLimit(
      usage.tx_limit_bytes != null
        ? String(usage.tx_limit_bytes / 1024 ** 3)
        : "",
    );
    setBillingMode(usage.billing_mode || "sum");
    setAdjustmentDirection(
      usage.billing_mode === "separate_directions" ? "inbound" : "billed",
    );
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
  const unitBytes = trafficUnits[amountUnit] ?? 0;
  const combinedSeedBytes = Number(amount) * unitBytes;
  const rxSeedBytes = Number(rxAmount) * unitBytes;
  const txSeedBytes = Number(txAmount) * unitBytes;
  const seedMode = usage.billing_mode;
  const seedBytes =
    seedMode === "max_direction"
      ? Math.max(rxSeedBytes, txSeedBytes)
      : seedMode === "separate_directions"
        ? rxSeedBytes + txSeedBytes
        : seedMode === "inbound_only"
          ? rxSeedBytes
          : seedMode === "outbound_only"
            ? txSeedBytes
            : combinedSeedBytes;
  const submit = async (event: Event) => {
    event.preventDefault();
    const values =
      seedMode === "sum"
        ? [combinedSeedBytes]
        : seedMode === "inbound_only"
          ? [rxSeedBytes]
          : seedMode === "outbound_only"
            ? [txSeedBytes]
            : [rxSeedBytes, txSeedBytes];
    if (values.some((value) => !Number.isFinite(value) || value < 0)) return;
    if (!seedPreview) {
      setSeedPreview(true);
      return;
    }
    try {
      await api(`/api/v1/servers/${encodeURIComponent(id)}/traffic/seed`, {
        method: "POST",
        body: JSON.stringify({
          combined_bytes:
            seedMode === "sum" ? Math.round(combinedSeedBytes) : undefined,
          rx_bytes:
            seedMode === "inbound_only" ||
            seedMode === "max_direction" ||
            seedMode === "separate_directions"
              ? Math.round(rxSeedBytes)
              : undefined,
          tx_bytes:
            seedMode === "outbound_only" ||
            seedMode === "max_direction" ||
            seedMode === "separate_directions"
              ? Math.round(txSeedBytes)
              : undefined,
          effective_at: usage.checkpoint_at,
          note,
        }),
      });
      await load();
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
      await api(
        `/api/v1/servers/${encodeURIComponent(id)}/traffic/adjustments`,
        {
          method: "POST",
          body: JSON.stringify({
            signed_bytes: Math.round(value),
            direction: adjustmentDirection,
            effective_at: usage.checkpoint_at,
            reason: adjustmentReason,
          }),
        },
      );
      await load();
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
          billing_mode: billingMode,
          rx_limit_bytes: rxLimit
            ? Math.round(Number(rxLimit) * 1024 ** 3)
            : null,
          tx_limit_bytes: txLimit
            ? Math.round(Number(txLimit) * 1024 ** 3)
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
        {usage.billing_mode === "separate_directions" ? (
          <>
            <Metric
              label="Inbound provider usage"
              value={bytes(usage.rx_total_bytes)}
              tone="accent"
              detail={
                usage.rx_limit_bytes != null
                  ? t("{percent} of inbound limit", {
                      percent: percent(
                        usage.rx_total_bytes,
                        usage.rx_limit_bytes,
                      ),
                    })
                  : "No inbound limit configured"
              }
            />
            <Metric
              label="Outbound provider usage"
              value={bytes(usage.tx_total_bytes)}
              tone="accent"
              detail={
                usage.tx_limit_bytes != null
                  ? t("{percent} of outbound limit", {
                      percent: percent(
                        usage.tx_total_bytes,
                        usage.tx_limit_bytes,
                      ),
                    })
                  : "No outbound limit configured"
              }
            />
          </>
        ) : (
          <Metric
            label="Provider-billed cycle total"
            value={bytes(usage.billed_total_bytes ?? usage.total_bytes)}
            tone="accent"
            detail={
              usage.limit_bytes != null
                ? t("{percent} of limit", {
                    percent: percent(usage.total_bytes, usage.limit_bytes),
                  })
                : "No limit configured"
            }
          />
        )}
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
          label="Billing mode"
          value={billingModeLabel(usage.billing_mode)}
          detail="Closed provider accounting profile"
        />
        {usage.billing_mode === "separate_directions" ? (
          <>
            <Metric
              label="Inbound projection"
              value={
                usage.projected_rx_bytes == null
                  ? t("Insufficient history")
                  : bytes(usage.projected_rx_bytes)
              }
              tone={
                usage.rx_limit_bytes != null &&
                usage.projected_rx_bytes != null &&
                usage.projected_rx_bytes > usage.rx_limit_bytes
                  ? "warn"
                  : undefined
              }
              detail="Observed inbound rate through cycle end"
            />
            <Metric
              label="Outbound projection"
              value={
                usage.projected_tx_bytes == null
                  ? t("Insufficient history")
                  : bytes(usage.projected_tx_bytes)
              }
              tone={
                usage.tx_limit_bytes != null &&
                usage.projected_tx_bytes != null &&
                usage.projected_tx_bytes > usage.tx_limit_bytes
                  ? "warn"
                  : undefined
              }
              detail="Observed outbound rate through cycle end"
            />
          </>
        ) : (
          <Metric
            label="Projection"
            value={
              usage.projected_bytes == null
                ? t("Insufficient history")
                : bytes(usage.projected_bytes)
            }
            tone={
              usage.limit_bytes != null &&
              usage.projected_bytes != null &&
              usage.projected_bytes > usage.limit_bytes
                ? "warn"
                : undefined
            }
            detail="Observed rate through cycle end"
          />
        )}
      </section>
      <section class="split">
        <div class="panel">
          <PanelHead
            title="Transparent accounting"
            detail={
              usage.billing_mode === "separate_directions"
                ? "Inbound and outbound remain independently auditable"
                : "Seed + observed + adjustments = current total"
            }
          />
          <div class="equation">
            <span>
              {bytes(usage.seed_bytes)}
              <small>{t("manual seed")}</small>
            </span>
            <b>+</b>
            <span>
              {bytes(usage.observed_bytes)}
              <small>{t("locally observed")}</small>
            </span>
            <b>+</b>
            <span>
              {bytes(usage.adjustment_bytes)}
              <small>{t("adjustments")}</small>
            </span>
            <b>=</b>
            <span>
              {bytes(usage.total_bytes)}
              <small>
                {t(
                  usage.billing_mode === "separate_directions"
                    ? "directional sum (not a provider-billed total)"
                    : "cycle total",
                )}
              </small>
            </span>
          </div>
          <dl class="facts">
            <div>
              <dt>{t("Cycle")}</dt>
              <dd>
                {dateTime(usage.cycle_start, usage.timezone)} –{" "}
                {dateTime(usage.cycle_end, usage.timezone)} ({usage.timezone})
              </dd>
            </div>
            <div>
              <dt>{t("Confidence")}</dt>
              <dd>
                <Status value={usage.confidence} />
              </dd>
            </div>
            <div>
              <dt>{t("Interfaces")}</dt>
              <dd>
                {usage.actual_selected_interfaces?.length
                  ? usage.actual_selected_interfaces.join(", ")
                  : t("No accounting interface selected")}
                <br />
                <small class="mono">
                  {t("Policy")}: {JSON.stringify(usage.selected_interfaces)}
                </small>
              </dd>
            </div>
            <div>
              <dt>{t("Billing mode")}</dt>
              <dd>{billingModeLabel(usage.billing_mode)}</dd>
            </div>
            <div>
              <dt>{t("Observed direction")}</dt>
              <dd>
                {bytes(usage.observed_rx_bytes)} {t("inbound")} ·{" "}
                {bytes(usage.observed_tx_bytes)} {t("outbound")}
              </dd>
            </div>
            {usage.directional_seed_known && (
              <div>
                <dt>{t("Provider directional totals")}</dt>
                <dd>
                  {usage.rx_total_bytes == null
                    ? t("Not billed")
                    : `${bytes(usage.rx_total_bytes)} ${t("inbound")}`}
                  {" · "}
                  {usage.tx_total_bytes == null
                    ? t("Not billed")
                    : `${bytes(usage.tx_total_bytes)} ${t("outbound")}`}
                </dd>
              </div>
            )}
            <div>
              <dt>{t("Observation window")}</dt>
              <dd>
                {dateTime(usage.observation_start_at, usage.timezone)} –{" "}
                {dateTime(usage.checkpoint_at, usage.timezone)} (
                {usage.timezone})
              </dd>
            </div>
            <div>
              <dt>{t("Provider seed source")}</dt>
              <dd>
                {usage.has_manual_seed
                  ? `${usage.seed_note || t("Manual entry")} · ${dateTime(usage.seed_effective_at, usage.timezone)} (${usage.timezone})`
                  : t("No manual seed entered")}
              </dd>
            </div>
          </dl>
          {usage.uncertainty_reason && (
            <p class="notice" role="status">
              <strong>{t("Uncertainty")}</strong>: {t(usage.uncertainty_reason)}
            </p>
          )}
          <p class="caveat">
            {t(
              "Parade measures selected Linux interface bytes. Provider billing can differ due to overhead, direction weighting, rounding and private traffic policy.",
            )}
          </p>
        </div>
        {!usage.has_manual_seed ? (
          <form class="panel form" onSubmit={submit}>
            <PanelHead
              title="Enter current provider usage"
              detail="Creates one immutable primary seed at the latest checkpoint"
            />
            {billingMode !== seedMode && (
              <p class="notice" role="status">
                {t(
                  "Save the billing-cycle rule before entering a seed for the newly selected mode.",
                )}
              </p>
            )}
            {seedMode === "sum" ? (
              <TrafficAmountInput
                label="Current provider-used combined traffic"
                value={amount}
                onInput={(value: string) => {
                  setAmount(value);
                  setSeedPreview(false);
                }}
              />
            ) : (
              <>
                {seedMode !== "outbound_only" && (
                  <TrafficAmountInput
                    label="Current provider-used inbound traffic"
                    value={rxAmount}
                    onInput={(value: string) => {
                      setRxAmount(value);
                      setSeedPreview(false);
                    }}
                  />
                )}
                {seedMode !== "inbound_only" && (
                  <TrafficAmountInput
                    label="Current provider-used outbound traffic"
                    value={txAmount}
                    onInput={(value: string) => {
                      setTxAmount(value);
                      setSeedPreview(false);
                    }}
                  />
                )}
              </>
            )}
            <label>
              {t("Traffic unit")}
              <select
                aria-label={t("Traffic unit")}
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
            </label>
            <label>
              {t("Effective checkpoint")}{" "}
              <input
                value={`${dateTime(usage.checkpoint_at, usage.timezone)} (${usage.timezone})`}
                disabled
              />
            </label>
            <label>
              {t("Source note")}{" "}
              <input
                value={note}
                maxLength={500}
                onInput={(event) => setNote(event.currentTarget.value)}
              />
            </label>
            {seedPreview && (
              <div class="seed-preview" role="status">
                <strong>{t("Confirm immutable seed")}</strong>
                <span>
                  {t("Provider entry")}: {bytes(seedBytes)} ·{" "}
                  {billingModeLabel(seedMode)}
                </span>
                {seedMode !== "sum" && (
                  <span>
                    {t("Inbound")}: {bytes(rxSeedBytes)} · {t("Outbound")}:{" "}
                    {bytes(txSeedBytes)}
                  </span>
                )}
                <span>
                  {t("Agent checkpoint")}:{" "}
                  {bytes(usage.agent_observed_total_bytes)}
                  {" · "}
                  {dateTime(usage.checkpoint_at, usage.timezone)} (
                  {usage.timezone})
                </span>
                <span>
                  {t("Cycle")}: {dateTime(usage.cycle_start, usage.timezone)} –{" "}
                  {dateTime(usage.cycle_end, usage.timezone)} ({usage.timezone})
                </span>
                <span>
                  {t(
                    "Result after saving: {bytes} + future selected-interface traffic",
                    { bytes: bytes(seedBytes) },
                  )}
                </span>
              </div>
            )}
            <button class="primary" type="submit">
              {t(seedPreview ? "Confirm and save seed" : "Preview seed")}
            </button>
            <small>
              {t(
                "Mistakes are corrected with an append-only audited adjustment; history is never silently rewritten.",
              )}
            </small>
          </form>
        ) : (
          <form class="panel form" onSubmit={submitAdjustment}>
            <PanelHead
              title="Append an audited adjustment"
              detail="Corrections preserve the original seed and full history"
            />
            <label>
              {t("Signed correction (GiB)")}
              <input
                type="number"
                step="0.01"
                value={adjustment}
                onInput={(event) => setAdjustment(event.currentTarget.value)}
                required
              />
            </label>
            {usage.billing_mode === "separate_directions" && (
              <label>
                {t("Adjustment direction")}
                <select
                  value={adjustmentDirection}
                  onChange={(event) =>
                    setAdjustmentDirection(event.currentTarget.value)
                  }
                >
                  <option value="inbound">{t("Inbound")}</option>
                  <option value="outbound">{t("Outbound")}</option>
                </select>
              </label>
            )}
            <label>
              {t("Reason")}
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
              {t("Append adjustment")}
            </button>
          </form>
        )}
      </section>
      <TrafficHistory items={usage.history || []} timeZone={usage.timezone} />
      <form class="panel form" onSubmit={submitRule}>
        <PanelHead
          title="Billing-cycle rule"
          detail="IANA timezone, calendar anchor, and optional provider limit"
        />
        <div class="form-row">
          <label>
            {t("Provider billing mode")}
            <select
              value={billingMode}
              onChange={(event) => {
                const mode = event.currentTarget.value;
                setBillingMode(mode);
                setSeedPreview(false);
                if (mode === "separate_directions") setLimit("");
                else {
                  setRxLimit("");
                  setTxLimit("");
                }
              }}
            >
              <option value="sum">{billingModeLabel("sum")}</option>
              <option value="inbound_only">
                {billingModeLabel("inbound_only")}
              </option>
              <option value="outbound_only">
                {billingModeLabel("outbound_only")}
              </option>
              <option value="max_direction">
                {billingModeLabel("max_direction")}
              </option>
              <option value="separate_directions">
                {billingModeLabel("separate_directions")}
              </option>
            </select>
          </label>
          <label>
            {t("IANA timezone")}
            <input
              value={timezone}
              onInput={(event) => setTimezone(event.currentTarget.value)}
              required
            />
          </label>
          <label>
            {t("Anchor day")}
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
            {t("Local anchor time")}
            <input
              type="time"
              value={anchorTime}
              onInput={(event) => setAnchorTime(event.currentTarget.value)}
              required
            />
          </label>
          {billingMode === "separate_directions" ? (
            <>
              <label>
                {t("Inbound limit (GiB, optional)")}
                <input
                  type="number"
                  min="0"
                  step="0.01"
                  value={rxLimit}
                  onInput={(event) => setRxLimit(event.currentTarget.value)}
                />
              </label>
              <label>
                {t("Outbound limit (GiB, optional)")}
                <input
                  type="number"
                  min="0"
                  step="0.01"
                  value={txLimit}
                  onInput={(event) => setTxLimit(event.currentTarget.value)}
                />
              </label>
            </>
          ) : (
            <label>
              {t("Traffic limit (GiB, optional)")}
              <input
                type="number"
                min="0"
                step="0.01"
                value={limit}
                onInput={(event) => setLimit(event.currentTarget.value)}
              />
            </label>
          )}
          <label>
            {t("Selected interfaces (comma-separated; blank = automatic)")}
            <input
              value={selectedInterfaces}
              onInput={(event) =>
                setSelectedInterfaces(event.currentTarget.value)
              }
              placeholder="eth0, ens3"
            />
          </label>
          <label>
            {t("Excluded interfaces (comma-separated)")}
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
          {t("Save cycle rule")}
        </button>
        <small>
          {t(
            "Interface auto-selection follows the default route and excludes loopback, container, bridge, veth, and tunnel devices. Current selected identities remain visible above.",
          )}
        </small>
      </form>
    </>
  );
}

function TrafficHistory({
  items,
  timeZone,
}: {
  items: any[];
  timeZone: string;
}) {
  return (
    <section class="panel traffic-history">
      <PanelHead
        title="Billing-cycle history"
        detail="Latest 24 cycles with immutable seeds and append-only corrections"
      />
      {items.length ? (
        <div class="table-wrap">
          <table>
            <thead>
              <tr>
                <th>{t("Cycle")}</th>
                <th>{t("State")}</th>
                <th>{t("Billing mode")}</th>
                <th>{t("Manual seed")}</th>
                <th>{t("Observed direction")}</th>
                <th>{t("Adjustments")}</th>
                <th>{t("Provider total")}</th>
                <th>{t("Confidence")}</th>
              </tr>
            </thead>
            <tbody>
              {items.map((cycle) => (
                <tr key={cycle.cycle_id}>
                  <td>
                    <strong>{dateTime(cycle.cycle_start, timeZone)}</strong>
                    <small>
                      {dateTime(cycle.cycle_end, timeZone)} ({timeZone})
                    </small>
                  </td>
                  <td>
                    <Status value={cycle.state} />
                  </td>
                  <td>{billingModeLabel(cycle.billing_mode)}</td>
                  <td>
                    {bytes(cycle.seed_bytes)}
                    <small>
                      {cycle.has_manual_seed
                        ? `${cycle.seed_note || t("Manual entry")} · ${cycle.seed_operator || "—"}${cycle.seed_checkpoint_at ? ` · ${dateTime(cycle.seed_checkpoint_at, timeZone)}` : ""}`
                        : t("Automatic zero rollover")}
                    </small>
                  </td>
                  <td>
                    {bytes(cycle.observed_rx_bytes)} {t("inbound")}
                    <small>
                      {bytes(cycle.observed_tx_bytes)} {t("outbound")}
                    </small>
                  </td>
                  <td>
                    {bytes(cycle.adjustment_bytes)}
                    {cycle.adjustments?.length ? (
                      <details>
                        <summary>
                          {t("{count} audited corrections", {
                            count: cycle.adjustments.length,
                          })}
                        </summary>
                        <ul class="evidence-list">
                          {cycle.adjustments.map((adjustment: any) => (
                            <li>
                              <strong>
                                {bytes(adjustment.signed_bytes)} ·{" "}
                                {t(adjustment.direction)}
                              </strong>
                              <span>{adjustment.reason}</span>
                              <time>
                                {dateTime(adjustment.effective_at, timeZone)} ·{" "}
                                {adjustment.operator}
                              </time>
                            </li>
                          ))}
                        </ul>
                        {cycle.adjustments_truncated && (
                          <small>
                            {t("Only the first 50 corrections are shown")}
                          </small>
                        )}
                      </details>
                    ) : null}
                  </td>
                  <td>
                    {cycle.billing_mode === "separate_directions" ? (
                      <>
                        {bytes(cycle.rx_total_bytes)} {t("inbound")}
                        <small>
                          {bytes(cycle.tx_total_bytes)} {t("outbound")}
                        </small>
                      </>
                    ) : (
                      bytes(cycle.billed_total_bytes)
                    )}
                  </td>
                  <td>
                    <Status value={cycle.confidence} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : (
        <Empty
          title="No earlier billing cycles"
          detail="The first completed rollover will appear here; the current cycle remains visible above."
        />
      )}
    </section>
  );
}

function TrafficAmountInput({
  label,
  value,
  onInput,
}: {
  label: string;
  value: string;
  onInput: (value: string) => void;
}) {
  return (
    <label>
      {t(label)}
      <input
        aria-label={t(label)}
        type="number"
        min="0"
        step="0.01"
        value={value}
        onInput={(event) => onInput(event.currentTarget.value)}
        required
      />
    </label>
  );
}

function Settings() {
  const [id, setId] = useState("");
  const [name, setName] = useState("");
  const [message, setMessage] = useState("");
  const [command, setCommand] = useState("");
  const [enrollId, setEnrollId] = useState("");
  const [deleteId, setDeleteId] = useState("");
  const [deleteReason, setDeleteReason] = useState("");
  const mint = async (serverId: string) => {
    const enrollment = await api<{ command: string; expires_at: number }>(
      `/api/v1/servers/${encodeURIComponent(serverId)}/enrollment`,
      { method: "POST" },
    );
    setCommand(enrollment.command);
    setMessage(
      t("Enrollment for {id} expires {time}.", {
        id: serverId,
        time: timeOnly(enrollment.expires_at),
      }),
    );
  };
  const create = async (event: Event) => {
    event.preventDefault();
    try {
      const created = await api<{ id: string }>("/api/v1/servers", {
        method: "POST",
        body: JSON.stringify({ id, name, group: "" }),
      });
      setEnrollId(created.id);
      setMessage(t("Created {id}.", { id: created.id }));
      setId("");
      setName("");
      try {
        await mint(created.id);
      } catch (reason) {
        setMessage(
          t(
            "Server {id} was created, but enrollment could not be issued: {error}",
            {
              id: created.id,
              error: (reason as Error).message,
            },
          ),
        );
      }
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
          {t("Server ID")}
          <input
            value={id}
            pattern="[A-Za-z0-9._-]+"
            maxLength={64}
            onInput={(e) => setId(e.currentTarget.value)}
            required
          />
        </label>
        <label>
          {t("Display name")}
          <input
            value={name}
            maxLength={100}
            onInput={(e) => setName(e.currentTarget.value)}
            required
          />
        </label>
        <button class="primary">{t("Create server record")}</button>
        {message && (
          <p role="status" class="notice">
            {message}
          </p>
        )}
      </form>
      <form
        class="panel form"
        onSubmit={async (event) => {
          event.preventDefault();
          try {
            await mint(enrollId);
          } catch (reason) {
            setMessage((reason as Error).message);
          }
        }}
      >
        <PanelHead
          title="Issue or rotate enrollment"
          detail="One server-bound command; previous identity is revoked only after successful enrollment"
        />
        <label>
          {t("Existing server ID")}
          <input
            value={enrollId}
            pattern="[A-Za-z0-9._-]+"
            maxLength={64}
            onInput={(event) => setEnrollId(event.currentTarget.value)}
            required
          />
        </label>
        <button class="primary">{t("Issue single-use command")}</button>
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
          <li>{t("Argon2id administrator authentication")}</li>
          <li>{t("Strict SameSite session and CSRF validation")}</li>
          <li>{t("Explicit trusted proxy addresses only")}</li>
          <li>{t("SQLite WAL with transactional migrations")}</li>
          <li>{t("Independent revocable Agent credentials")}</li>
        </ul>
      </div>
      <form
        class="panel form danger-zone"
        onSubmit={async (event) => {
          event.preventDefault();
          if (
            !confirm(
              t(
                "Permanently tombstone this server ID and revoke its Agent identity? The monitored VPS is not modified.",
              ),
            )
          )
            return;
          try {
            await api(`/api/v1/servers/${encodeURIComponent(deleteId)}`, {
              method: "DELETE",
              body: JSON.stringify({ reason: deleteReason }),
            });
            setMessage(t("Server {id} was tombstoned.", { id: deleteId }));
            setDeleteId("");
            setDeleteReason("");
          } catch (reason) {
            setMessage((reason as Error).message);
          }
        }}
      >
        <PanelHead
          title="Retire a server record"
          detail="Hub-only revocation and durable tombstone; no command is sent to the VPS"
        />
        <label>
          {t("Server ID to retire")}
          <input
            value={deleteId}
            pattern="[A-Za-z0-9._-]+"
            maxLength={64}
            onInput={(event) => setDeleteId(event.currentTarget.value)}
            required
          />
        </label>
        <label>
          {t("Retirement reason")}
          <input
            value={deleteReason}
            minLength={3}
            maxLength={500}
            onInput={(event) => setDeleteReason(event.currentTarget.value)}
            required
          />
        </label>
        <button>{t("Create tombstone and revoke identity")}</button>
      </form>
      <div class="panel">
        <PanelHead title="Backup and restore" detail="Operational guidance" />
        <p class="prose">
          {t(
            "Use SQLite's online backup command or stop the Hub before copying the database, including WAL state. Test restores on a disposable Hub. Agent credentials remain bound to the restored server records.",
          )}
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
              <strong>{t(item.action)}</strong>
              <span>{item.server_id || t("Hub")}</span>
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
  const data = value?.data ?? value;
  const history = Array.isArray(value?.history)
    ? value.history
        .map((item: any) => ({
          at: Number(item.data?.interval_end || item.interval_end || 0),
          data: item.data,
        }))
        .filter((item: any) => item.data && item.at)
    : [];
  return data && typeof data === "object" ? (
    <>
      <div class="resource-grid">
        <Metric
          label="CPU average"
          value={`${Number(data.cpu_avg_pct || 0).toFixed(1)}%`}
          detail={t("Peak {value} · {count} cores · {samples} samples", {
            value: `${Number(data.cpu_max_pct || 0).toFixed(1)}%`,
            count: number(data.cpu_cores || 0),
            samples: number(data.samples || 0),
          })}
        />
        <Metric
          label="Load average"
          value={Number(data.load1_avg || 0).toFixed(2)}
          detail="1 minute average"
        />
        <Metric
          label="Memory"
          value={bytes(data.mem_used)}
          detail={t("{percent} · {value} total", {
            percent: percent(data.mem_used || 0, data.mem_total || 0),
            value: bytes(data.mem_total),
          })}
        />
        <Metric
          label="Swap"
          value={bytes(data.swap_used)}
          detail={t("{percent} · {value} total", {
            percent: percent(data.swap_used || 0, data.swap_total || 0),
            value: bytes(data.swap_total),
          })}
        />
        <Metric
          label="Disk"
          value={data.disk_total ? bytes(data.disk_used) : t("Unsupported")}
          detail={
            data.disk_total
              ? t("{percent} · {value} total", {
                  percent: percent(data.disk_used || 0, data.disk_total),
                  value: bytes(data.disk_total),
                })
              : t("Filesystem capacity is unavailable on this Agent target")
          }
        />
        <Metric
          label="Disk inodes"
          value={
            data.disk_inodes_total
              ? number(data.disk_inodes_used || 0)
              : t("Unsupported")
          }
          detail={
            data.disk_inodes_total
              ? t("{percent} · {value} total", {
                  percent: percent(
                    data.disk_inodes_used || 0,
                    data.disk_inodes_total,
                  ),
                  value: number(data.disk_inodes_total),
                })
              : t("Filesystem inode counters are unavailable")
          }
        />
        <Metric
          label="Pressure"
          value={
            data.psi_cpu_some_avg10 == null
              ? t("Unsupported")
              : `${Number(data.psi_cpu_some_avg10).toFixed(2)}%`
          }
          detail={t("CPU {cpu} · memory {memory} · I/O {io}", {
            cpu:
              data.psi_cpu_some_avg10 == null
                ? "—"
                : `${Number(data.psi_cpu_some_avg10).toFixed(2)}%`,
            memory:
              data.psi_mem_some_avg10 == null
                ? "—"
                : `${Number(data.psi_mem_some_avg10).toFixed(2)}%`,
            io:
              data.psi_io_some_avg10 == null
                ? "—"
                : `${Number(data.psi_io_some_avg10).toFixed(2)}%`,
          })}
        />
        <Metric
          label="Connections"
          value={number(
            (data.tcp_connections || 0) + (data.udp_connections || 0),
          )}
          detail={t("TCP {tcp} · UDP {udp}", {
            tcp: number(data.tcp_connections || 0),
            udp: number(data.udp_connections || 0),
          })}
        />
      </div>
      {history.length > 1 && (
        <div class="trend-grid">
          <MiniTrend
            label="CPU trend"
            values={history.map((item: any) => ({
              at: item.at,
              value: Number(item.data.cpu_avg_pct || 0),
            }))}
            max={100}
          />
          <MiniTrend
            label="Memory trend"
            values={history.map((item: any) => ({
              at: item.at,
              value: item.data.mem_total
                ? (item.data.mem_used / item.data.mem_total) * 100
                : 0,
            }))}
            max={100}
          />
          <MiniTrend
            label="Load trend"
            values={history.map((item: any) => ({
              at: item.at,
              value: Number(item.data.load1_avg || 0),
            }))}
          />
        </div>
      )}
    </>
  ) : (
    <Empty
      title="No resource rollup"
      detail="The Agent has not submitted this collector."
    />
  );
}

function MiniTrend({
  label,
  values,
  max,
}: {
  label: string;
  values: Array<{ at: number; value: number }>;
  max?: number;
}) {
  const ceiling = Math.max(max || 0, ...values.map((item) => item.value), 1);
  const gaps = values
    .slice(1)
    .map((item, index) => item.at - values[index]!.at)
    .filter((gap) => gap > 0)
    .sort((a, b) => a - b);
  const typicalGap = gaps.length
    ? (gaps[Math.floor(gaps.length / 2)] ?? 300)
    : 300;
  const gapThreshold = Math.max(900, typicalGap * 2.5);
  const first = values[0]?.at || 0;
  const duration = Math.max((values.at(-1)?.at || first) - first, 1);
  const segments: (typeof values)[] = [];
  values.forEach((item, index) => {
    if (!index || item.at - values[index - 1]!.at > gapThreshold) {
      segments.push([]);
    }
    segments.at(-1)?.push(item);
  });
  const points = (segment: typeof values) =>
    segment
      .map((item) => {
        const x = ((item.at - first) / duration) * 100;
        const y =
          30 - (Math.max(0, Math.min(item.value, ceiling)) / ceiling) * 28;
        return `${x.toFixed(2)},${y.toFixed(2)}`;
      })
      .join(" ");
  return (
    <div class="trend-card">
      <strong>{t(label)}</strong>
      <svg viewBox="0 0 100 32" role="img" aria-label={t(label)}>
        {segments.map((segment) => (
          <polyline points={points(segment)} />
        ))}
      </svg>
      <small>
        {t("{count} bounded five-minute rollups", { count: values.length })}
      </small>
    </div>
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
        {t(
          "Full command lines and environment variables are never collected. Normal mode sends bounded top-N and suspicious facts.",
        )}
      </p>
      <div class="mini-toolbar">
        <label>
          <span class="sr-only">{t("Search process facts")}</span>
          <input
            type="search"
            placeholder={t("Search PID, UID, executable or cgroup")}
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
          {t("Suspicious only")}
        </label>
      </div>
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              <th>{t("State")}</th>
              <th>PID</th>
              <th>PPID</th>
              <th>UID</th>
              <th>{t("Executable")}</th>
              <th>{t("CPU ticks")}</th>
              <th>RSS</th>
              <th>{t("Virtual")}</th>
              <th>{t("Unit / cgroup")}</th>
              <th>{t("Listeners")}</th>
              <th>{t("Package")}</th>
              <th>{t("Evidence")}</th>
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
                <td>{t(p.package_ownership || "unknown")}</td>
                <td>
                  <span class="evidence-tags">
                    {p.deleted_executable && (
                      <span class="tag bad">{t("deleted executable")}</span>
                    )}
                    {p.suspicious_writable_path && (
                      <span class="tag warn">{t("writable path")}</span>
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
        <p class="notice">{t("No process facts match the current filters.")}</p>
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
          value={t(value?.confidence || "unsupported")}
          detail="Raw counters are never reset"
        />
      </div>
      {interfaces.length ? (
        <>
          <h3 class="section-label">{t("Interfaces")}</h3>
          <div class="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>{t("Interface")}</th>
                  <th>{t("Accounting")}</th>
                  <th>RX</th>
                  <th>TX</th>
                  <th>{t("Packets RX / TX")}</th>
                  <th>{t("Errors RX / TX")}</th>
                  <th>{t("Drops RX / TX")}</th>
                </tr>
              </thead>
              <tbody>
                {interfaces.map((item: any) => (
                  <tr>
                    <td class="mono">{item.name}</td>
                    <td>{t(item.selected ? "Selected" : "Observed only")}</td>
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
      <h3 class="section-label">{t("Listening ports")}</h3>
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
            <th>{t("Protocol")}</th>
            <th>{t("Bind address")}</th>
            <th>{t("Port")}</th>
            <th>UID</th>
            <th>{t("Socket inode")}</th>
          </tr>
        </thead>
        <tbody>
          {value.map((p: any) => (
            <tr>
              <td>{p.protocol}</td>
              <td class="mono">{p.local_address}</td>
              <td class="mono">{p.port}</td>
              <td>{p.uid ?? t("Unknown")}</td>
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
        {t(
          "No finding is proof that the host is safe or compromised. Host-local telemetry may be falsified by a sufficiently privileged attacker.",
        )}
      </p>
      <div class="finding-list">
        {value.map((f: any) => (
          <article>
            <header>
              <span class={`tag ${f.severity}`}>{t(f.severity)}</span>
              <strong>
                {f.rule_id} <small>v{f.rule_version}</small>
              </strong>
              <span>
                {t("{confidence} confidence · {count} occurrence(s)", {
                  confidence: t(f.confidence),
                  count: number(f.occurrences),
                })}
              </span>
              <Status value={f.state} />
            </header>
            {f.server_name ? (
              <a
                class="text-link"
                href={`#/servers/${encodeURIComponent(f.server_id)}/security`}
              >
                {t("{server} · first {first} · last {last}", {
                  server: f.server_name,
                  first: relative(f.first_seen),
                  last: relative(f.last_seen),
                })}
              </a>
            ) : (
              <span>
                {t("first {first} · last {last}", {
                  first: relative(f.first_seen),
                  last: relative(f.last_seen),
                })}
              </span>
            )}
            <code>{f.evidence}</code>
            <p>{t(f.explanation)}</p>
            <details>
              <summary>{t("Manual verification and caveats")}</summary>
              <p>{t(f.verification)}</p>
              <p>{t(f.coverage_caveat)}</p>
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
          <time>{dateTime(e.occurred_at)}</time>
          <span class={`tag ${e.severity}`}>{t(e.severity)}</span>
          <span class="tag">{t(e.category)}</span>
          <strong>{t(e.summary)}</strong>
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
            <strong>{t(item.collector)}</strong>
            <span>{coverageDetail(item.detail)}</span>
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
    ? t("Full")
    : server.coverage.length
      ? t("Partial")
      : t("Pending");
}
function coverageDetail(detail: string): string {
  const unavailable = detail.match(/^(.+) is unavailable$/);
  return unavailable?.[1]
    ? t("{path} is unavailable", { path: unavailable[1] })
    : t(detail);
}
function Metric({ label, value, detail, tone = "" }: any) {
  return (
    <article class={`metric ${tone}`}>
      <span>{t(label)}</span>
      <strong>{value}</strong>
      <small>{typeof detail === "string" ? t(detail) : detail}</small>
    </article>
  );
}
function PanelHead({ title, detail }: { title: string; detail: string }) {
  return (
    <header class="panel-head">
      <div>
        <h2>{t(title)}</h2>
        <p>{t(detail)}</p>
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
      <p>{t("Loading the latest accepted telemetry…")}</p>
    </div>
  );
}
function Empty({ title, detail }: { title: string; detail: string }) {
  return (
    <div class="empty">
      <span aria-hidden="true">○</span>
      <strong>{t(title)}</strong>
      <p>{t(detail)}</p>
    </div>
  );
}
function ErrorState({ message }: { message: string }) {
  return (
    <div class="error-state" role="alert">
      <strong>{t("Data could not be loaded")}</strong>
      <p>
        {t(
          "{message}. Previously displayed data may be stale; retrying is safe.",
          {
            message,
          },
        )}
      </p>
      <button onClick={() => location.reload()}>{t("Retry")}</button>
    </div>
  );
}

render(<Shell />, document.getElementById("app")!);
