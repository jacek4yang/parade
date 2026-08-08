import { t } from "./i18n";

export type FleetServer = {
  id: string;
  name: string;
  group: string;
  status: "online" | "stale" | "offline" | "pending" | "revoked";
  last_seen: number | null;
  os: string | null;
  kernel: string | null;
  arch: string | null;
  agent_version: string | null;
  coverage: Array<{ collector: string; status: string; detail: string }>;
  resources: {
    cpu_avg_pct?: number;
    mem_used?: number;
    mem_total?: number;
    disk_used?: number;
    disk_total?: number;
    psi_cpu_some_avg10?: number | null;
    psi_mem_some_avg10?: number | null;
    psi_io_some_avg10?: number | null;
  } | null;
  active_findings: number;
  traffic_confidence: string | null;
};

export type FleetResponse = {
  total: number;
  limit: number;
  offset: number;
  servers: FleetServer[];
  generated_at: number;
  summary: Record<string, number>;
};

export type TopologyResponse = {
  api_version: 1;
  relationship: "verified_agent_outbound_reports";
  hub: { label: string };
  total: number;
  displayed: number;
  truncated: boolean;
  generated_at: number;
  active_probing: false;
  peer_connections: false;
  edges: Array<{
    server_id: string;
    server_name: string;
    status: FleetServer["status"];
    last_seen: number | null;
    source_category:
      | "shared_observed_source"
      | "private_observed_source"
      | "special_observed_source"
      | "internet_scope_source"
      | "loopback_or_proxy_boundary"
      | "unknown";
    shared_source_count: number;
  }>;
};

function csrf(): string {
  return (
    document.cookie
      .split(";")
      .map((part) => part.trim())
      .find((part) => part.startsWith("parade_csrf="))
      ?.slice("parade_csrf=".length) ?? ""
  );
}

export async function api<T>(path: string, init: RequestInit = {}): Promise<T> {
  const method = (init.method ?? "GET").toUpperCase();
  const headers = new Headers(init.headers);
  if (init.body) headers.set("Content-Type", "application/json");
  if (!["GET", "HEAD", "OPTIONS"].includes(method))
    headers.set("X-Parade-CSRF", csrf());
  const response = await fetch(path, { ...init, headers });
  if (response.status === 401) {
    window.location.assign("/");
    throw new Error(t("Session expired"));
  }
  if (!response.ok) {
    const body = (await response.json().catch(() => ({}))) as {
      error?: string;
    };
    throw new Error(
      body.error
        ? t(body.error)
        : t("Request failed ({status})", { status: response.status }),
    );
  }
  if (response.status === 204) return undefined as T;
  return response.json() as Promise<T>;
}
