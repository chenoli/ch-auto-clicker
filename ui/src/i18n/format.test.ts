import { describe, expect, it } from "vitest";

import { cpsFromInterval, formatCps } from "./format";

describe("formatCps", () => {
  it("uses a comma and one decimal in pt-BR", () => {
    expect(formatCps(10, "pt-BR")).toBe("10,0");
    expect(formatCps(cpsFromInterval(100, true), "pt-BR")).toBe("20,0");
  });

  it("rounds to one decimal", () => {
    expect(formatCps(cpsFromInterval(35, false), "pt-BR")).toBe("28,6");
  });
});
