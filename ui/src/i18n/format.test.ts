import { describe, expect, it } from "vitest";

import {
  countdownSeconds,
  cpsFromInterval,
  formatCount,
  formatCps,
  formatDuration,
  formatElapsed,
  formatOneDecimal,
} from "./format";

describe("formatCps", () => {
  it("uses a comma and one decimal in pt-BR", () => {
    expect(formatCps(10, "pt-BR")).toBe("10,0");
    expect(formatCps(cpsFromInterval(100, true), "pt-BR")).toBe("20,0");
  });

  it("rounds to one decimal", () => {
    expect(formatCps(cpsFromInterval(35, false), "pt-BR")).toBe("28,6");
  });
});

describe("number formats", () => {
  it("groups thousands in pt-BR", () => {
    expect(formatCount(1284, "pt-BR")).toBe("1.284");
    expect(formatCount(0, "pt-BR")).toBe("0");
  });

  it("formats one decimal", () => {
    expect(formatOneDecimal(2.43, "pt-BR")).toBe("2,4");
    expect(formatOneDecimal(2.43, "en-US")).toBe("2.4");
  });
});

describe("time formats", () => {
  it("shows the footer clock in tenths", () => {
    expect(formatElapsed(0)).toBe("00:00.0");
    expect(formatElapsed(65_430)).toBe("01:05.4");
    expect(formatElapsed(-5)).toBe("00:00.0");
  });

  it("shows durations as hh:mm:ss", () => {
    expect(formatDuration(300)).toBe("00:05:00");
    expect(formatDuration(3_725)).toBe("01:02:05");
    expect(formatDuration(-1)).toBe("00:00:00");
  });

  it("rounds the countdown up and never shows zero", () => {
    expect(countdownSeconds(2_400)).toBe(3);
    expect(countdownSeconds(3_000)).toBe(3);
    expect(countdownSeconds(50)).toBe(1);
    expect(countdownSeconds(0)).toBe(1);
  });
});
