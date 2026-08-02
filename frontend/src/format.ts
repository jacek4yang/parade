const IEC = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];

export function bytes(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return "—";
  const sign = value < 0 ? "−" : "";
  let amount = Math.abs(value);
  let unit = 0;
  while (amount >= 1024 && unit < IEC.length - 1) {
    amount /= 1024;
    unit += 1;
  }
  return `${sign}${unit === 0 ? amount.toFixed(0) : amount.toFixed(amount >= 100 ? 0 : 1)} ${IEC[unit]}`;
}

export function relative(timestamp: number | null): string {
  if (!timestamp) return "Never";
  const seconds = Math.max(0, Math.floor(Date.now() / 1000) - timestamp);
  if (seconds < 60) return `${seconds}s ago`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  return `${Math.floor(seconds / 86400)}d ago`;
}

export function percent(value: number, total: number): string {
  return total > 0 ? `${((value / total) * 100).toFixed(1)}%` : "—";
}
