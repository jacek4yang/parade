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
};

export type FleetResponse = {
  total: number;
  limit: number;
  offset: number;
  servers: FleetServer[];
  generated_at: number;
  summary: Record<string, number>;
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
    throw new Error("Session expired");
  }
  if (!response.ok) {
    const body = (await response.json().catch(() => ({}))) as {
      error?: string;
    };
    throw new Error(body.error ?? `Request failed (${response.status})`);
  }
  if (response.status === 204) return undefined as T;
  return response.json() as Promise<T>;
}
