import { afterEach, describe, expect, it } from "vitest";
import { initialLocale, normalizeLocale, setLocale, t } from "../src/i18n";
import { bytes, dateTime, percent, relative } from "../src/format";

afterEach(() => setLocale("en"));

describe("locale selection", () => {
  it("honors an explicit choice before browser language", () => {
    expect(initialLocale("en", ["zh-CN"])).toBe("en");
    expect(initialLocale("zh-CN", ["en-US"])).toBe("zh-CN");
    expect(initialLocale(null, ["zh-Hans-CN", "en-US"])).toBe("zh-CN");
    expect(normalizeLocale("zh-SG")).toBe("zh-CN");
  });

  it("translates stable UI values and safely preserves evidence", () => {
    setLocale("zh-CN");
    expect(t("Fleet")).toBe("服务器群");
    expect(t("{count} registered servers", { count: 12 })).toBe(
      "已登记 12 台服务器",
    );
    expect(t("PROC_WRITABLE_EXEC")).toBe("PROC_WRITABLE_EXEC");
    expect(t("/tmp/cache-worker")).toBe("/tmp/cache-worker");
  });

  it("uses the selected locale for time and numeric presentation", () => {
    setLocale("zh-CN");
    expect(relative(Math.floor(Date.now() / 1000) - 65)).toBe("1 分钟前");
    expect(bytes(1024 ** 3)).toBe("1.0 GiB");
    expect(percent(1, 2)).toBe("50.0%");
    expect(dateTime(0, "Asia/Shanghai")).toBe("1970年01月01日 08:00:00");
  });
});
