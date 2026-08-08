import { getLocale, localeTag, t } from "./i18n";

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
  const digits = unit === 0 || amount >= 100 ? 0 : 1;
  const formatted = new Intl.NumberFormat(localeTag(), {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  }).format(amount);
  return `${sign}${formatted} ${IEC[unit]}`;
}

export function relative(timestamp: number | null): string {
  if (!timestamp) return t("Never");
  const seconds = Math.max(0, Math.floor(Date.now() / 1000) - timestamp);
  if (seconds < 60) return t("{count}s ago", { count: seconds });
  if (seconds < 3600)
    return t("{count}m ago", { count: Math.floor(seconds / 60) });
  if (seconds < 86400)
    return t("{count}h ago", { count: Math.floor(seconds / 3600) });
  return t("{count}d ago", { count: Math.floor(seconds / 86400) });
}

export function percent(value: number, total: number): string {
  return total > 0
    ? `${new Intl.NumberFormat(localeTag(), {
        minimumFractionDigits: 1,
        maximumFractionDigits: 1,
      }).format((value / total) * 100)}%`
    : "—";
}

export function dateTime(timestamp: number, timeZone?: string): string {
  const date = new Date(timestamp * 1000);
  if (getLocale() === "zh-CN") {
    const parts = Object.fromEntries(
      new Intl.DateTimeFormat("zh-CN", {
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
        hour12: false,
        timeZone,
      })
        .formatToParts(date)
        .map(({ type, value }) => [type, value]),
    );
    return `${parts.year}年${parts.month}月${parts.day}日 ${parts.hour}:${parts.minute}:${parts.second}`;
  }
  return date.toLocaleString(localeTag(), timeZone ? { timeZone } : undefined);
}

export function timeOnly(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleTimeString(localeTag(), {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
}

export function number(value: number): string {
  return new Intl.NumberFormat(localeTag()).format(value);
}
