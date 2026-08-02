import { describe, expect, it } from "vitest";
import { bytes, percent } from "../src/format";

describe("traffic presentation", () => {
  it("uses honest IEC units", () => {
    expect(bytes(1024 ** 3)).toBe("1.0 GiB");
    expect(bytes(123.4 * 1024 ** 3)).toBe("123 GiB");
    expect(bytes(-1024)).toBe("−1.0 KiB");
  });
  it("keeps seed arithmetic understandable", () => {
    const seed = 100 * 1024 ** 3;
    const observed = 5 * 1024 ** 3;
    expect(bytes(seed + observed)).toBe("105 GiB");
    expect(percent(seed + observed, 200 * 1024 ** 3)).toBe("52.5%");
  });
});
