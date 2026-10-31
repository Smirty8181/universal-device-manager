import { describe, expect, it } from "vitest";
import {
  BATTERY_RGB_TEMPLATES,
  cloneRanges,
  colorForPercent,
  rangesCoverAll,
} from "../src/lib/batteryRgb";

describe("battery rgb ranges", () => {
  it("picks the color of the matching band", () => {
    const ranges = [
      { min: 0, max: 20, color: "#ef4444", brightness: 100, blink: false },
      { min: 21, max: 100, color: "#22c55e", brightness: 100, blink: false },
    ];
    expect(colorForPercent(ranges, 15)).toBe("#ef4444");
    expect(colorForPercent(ranges, 21)).toBe("#22c55e");
    expect(colorForPercent(ranges, 100)).toBe("#22c55e");
  });

  it("returns null when there is nothing to show", () => {
    expect(colorForPercent([], 50)).toBeNull();
  });

  it("detects full coverage without gaps", () => {
    const ranges = [
      { min: 0, max: 50, color: "#000", brightness: 100, blink: false },
      { min: 51, max: 100, color: "#fff", brightness: 100, blink: false },
    ];
    expect(rangesCoverAll(ranges)).toBe(true);
    expect(rangesCoverAll(ranges.slice(0, 1))).toBe(false);
    expect(rangesCoverAll([])).toBe(false);
  });

  it("rejects overlapping or inverted bands", () => {
    expect(
      rangesCoverAll([
        { min: 0, max: 60, color: "#000", brightness: 100, blink: false },
        { min: 50, max: 100, color: "#fff", brightness: 100, blink: false },
      ]),
    ).toBe(false);
    expect(
      rangesCoverAll([{ min: 80, max: 20, color: "#000", brightness: 100, blink: false }]),
    ).toBe(false);
  });

  it("ships only complete templates", () => {
    expect(BATTERY_RGB_TEMPLATES.length).toBeGreaterThan(0);
    for (const template of BATTERY_RGB_TEMPLATES) {
      expect(rangesCoverAll(template.ranges)).toBe(true);
    }
  });

  it("clones ranges without sharing objects", () => {
    const source = [{ min: 0, max: 100, color: "#fff", brightness: 50, blink: true }];
    const copy = cloneRanges(source);
    expect(copy).toEqual(source);
    expect(copy[0]).not.toBe(source[0]);
  });
});
