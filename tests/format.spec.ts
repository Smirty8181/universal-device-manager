import { describe, expect, it } from "vitest";
import {
  clamp,
  compatibilitySummary,
  deviceKindLabel,
  formatDuration,
  formatHex,
  hexBytes,
  hexToRgb,
  localizedError,
  percent,
  rgbToHex,
  rgbToHsl,
  shortenHomePath,
  toHexBytes,
} from "../src/lib/format";
import type { Capability, CapabilityStatus } from "../src/lib/types";

function cap(id: string, status: CapabilityStatus): Capability {
  return { id, status, noteRu: "", noteEn: "" };
}

describe("color helpers", () => {
  it("clamps values to the given range", () => {
    expect(clamp(5, 0, 10)).toBe(5);
    expect(clamp(-2, 0, 10)).toBe(0);
    expect(clamp(42, 0, 10)).toBe(10);
  });

  it("parses short and long hex colors", () => {
    expect(hexToRgb("#f0a")).toEqual([255, 0, 170]);
    expect(hexToRgb("22c55e")).toEqual([34, 197, 94]);
  });

  it("falls back to the accent color on invalid input", () => {
    expect(hexToRgb("#zzz")).toEqual([255, 79, 163]);
    expect(hexToRgb("")).toEqual([255, 79, 163]);
  });

  it("serializes rgb back to hex without overflow", () => {
    expect(rgbToHex(255, 0, 170)).toBe("#ff00aa");
    expect(rgbToHex(300, -4, 15.6)).toBe("#ff0010");
  });

  it("normalizes arbitrary input through rgb", () => {
    expect(formatHex("#F0A")).toBe("#ff00aa");
    expect(formatHex("bad")).toBe("#bbaadd");
    expect(formatHex("#zzz")).toBe("#ff4fa3");
  });

  it("converts red to hsl and back", () => {
    expect(rgbToHsl(255, 0, 0)).toEqual([0, 100, 50]);
    expect(rgbToHsl(128, 128, 128)).toEqual([0, 0, 50]);
  });
});

describe("byte helpers", () => {
  it("reads hex bytes and ignores separators", () => {
    expect(hexBytes("04 ff 1a")).toEqual([0x04, 0xff, 0x1a]);
    expect(hexBytes("#04ff1a")).toEqual([0x04, 0xff, 0x1a]);
  });

  it("drops a trailing nibble", () => {
    expect(hexBytes("abc")).toEqual([0xab]);
  });

  it("formats bytes back into a spaced string", () => {
    expect(toHexBytes([4, 255, 26])).toBe("04 ff 1a");
  });
});

describe("display helpers", () => {
  it("splits bilingual error strings", () => {
    expect(localizedError("Ошибка / Error", "ru")).toBe("Ошибка");
    expect(localizedError("Ошибка / Error", "en")).toBe("Error");
    expect(localizedError("plain", "en")).toBe("plain");
  });

  it("rounds percentages", () => {
    expect(percent(89.6)).toBe("90%");
  });

  it("formats durations for short and long runs", () => {
    expect(formatDuration(500)).toBe("0.50 s");
    expect(formatDuration(61_000)).toBe("1:01");
    expect(formatDuration(-5)).toBe("0.00 s");
  });

  it("localizes device kinds and keeps unknown ids", () => {
    expect(deviceKindLabel("keyboard", "ru")).toBe("Клавиатура");
    expect(deviceKindLabel("mouse", "en")).toBe("Mouse");
    expect(deviceKindLabel("dock", "en")).toBe("dock");
  });

  it("summarizes capability statuses", () => {
    const summary = compatibilitySummary([
      cap("a", "working"),
      cap("b", "working"),
      cap("c", "partial"),
      cap("d", "research"),
    ]);
    expect(summary.counts).toEqual({ working: 2, partial: 1, research: 1, unsupported: 0 });
    expect(summary.confirmed).toBe(3);
    expect(summary.total).toBe(4);
  });

  it("handles empty capability lists", () => {
    expect(compatibilitySummary()).toEqual({
      counts: { working: 0, partial: 0, research: 0, unsupported: 0 },
      confirmed: 0,
      total: 0,
    });
  });
});

describe("shortenHomePath", () => {
  it("hides the account name behind an environment variable", () => {
    expect(shortenHomePath("C:\\Users\\anya\\AppData\\Roaming\\com.udm.desktop\\device-db.sqlite")).toBe(
      "%USERPROFILE%\\AppData\\Roaming\\com.udm.desktop\\device-db.sqlite",
    );
  });

  it("leaves other paths alone", () => {
    expect(shortenHomePath("D:\\Tools\\udm.log")).toBe("D:\\Tools\\udm.log");
    expect(shortenHomePath("\\\\?\\usb#vid_342d&pid_e411\\mi_00")).toBe("\\\\?\\usb#vid_342d&pid_e411\\mi_00");
    expect(shortenHomePath()).toBe("");
  });
});
