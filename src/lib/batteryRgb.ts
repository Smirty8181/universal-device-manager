import type { BatteryRgbRange } from "./types";

export interface BatteryRgbTemplate {
  id: string;
  name: string;
  ranges: BatteryRgbRange[];
  chargingColor: string;
}

const range = (min: number, max: number, color: string, blink = false): BatteryRgbRange => ({
  min,
  max,
  color,
  brightness: 100,
  blink,
});

export const BATTERY_RGB_TEMPLATES: BatteryRgbTemplate[] = [
  {
    id: "classic",
    name: "Classic",
    chargingColor: "#22d3ee",
    ranges: [
      range(91, 100, "#22c55e"),
      range(71, 90, "#a3e635"),
      range(51, 70, "#facc15"),
      range(31, 50, "#fb923c"),
      range(11, 30, "#ef4444"),
      range(0, 10, "#ef4444", true),
    ],
  },
  {
    id: "traffic",
    name: "Traffic Light",
    chargingColor: "#22d3ee",
    ranges: [range(61, 100, "#22c55e"), range(21, 60, "#facc15"), range(0, 20, "#ef4444")],
  },
  {
    id: "minimal",
    name: "Minimal",
    chargingColor: "#22d3ee",
    ranges: [range(51, 100, "#e5e7eb"), range(21, 50, "#a78bfa"), range(0, 20, "#ff4fa3")],
  },
  {
    id: "neon",
    name: "Neon",
    chargingColor: "#22d3ee",
    ranges: [
      range(76, 100, "#22d3ee"),
      range(51, 75, "#a78bfa"),
      range(26, 50, "#ff4fa3"),
      range(0, 25, "#f43f5e"),
    ],
  },
  {
    id: "gaming",
    name: "Gaming",
    chargingColor: "#22d3ee",
    ranges: [
      range(81, 100, "#22c55e"),
      range(61, 80, "#a3e635"),
      range(41, 60, "#facc15"),
      range(21, 40, "#fb923c"),
      range(0, 20, "#ef4444"),
    ],
  },
];

export function colorForPercent(ranges: BatteryRgbRange[], percent: number): string | null {
  const match = ranges.find((item) => percent >= item.min && percent <= item.max);
  return match?.color ?? ranges[ranges.length - 1]?.color ?? null;
}

export function rangesCoverAll(ranges: BatteryRgbRange[]): boolean {
  if (!ranges.length) return false;
  const sorted = [...ranges].sort((a, b) => a.min - b.min);
  let expected = 0;
  for (const item of sorted) {
    if (item.min !== expected || item.max > 100 || item.min > item.max) return false;
    expected = item.max + 1;
  }
  return expected === 101;
}

export function cloneRanges(ranges: BatteryRgbRange[]): BatteryRgbRange[] {
  return ranges.map((item) => ({ ...item }));
}
