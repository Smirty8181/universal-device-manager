import { describe, expect, it } from "vitest";
import {
  buildLayout,
  chordLabel,
  friendlyName,
  isModifier,
  isModifierVk,
  keyLabel,
  resolveKeyId,
} from "../src/lib/keyboard";

describe("keyboard mapping", () => {
  it("maps letters, digits and function keys by virtual key", () => {
    expect(resolveKeyId(0x41, false, 0x1e)).toBe("KeyA");
    expect(resolveKeyId(0x31, false, 0x02)).toBe("Digit1");
    expect(resolveKeyId(0x75, false, 0x40)).toBe("F6");
  });

  it("separates left and right modifiers", () => {
    expect(resolveKeyId(0x10, false, 0x2a)).toBe("ShiftLeft");
    expect(resolveKeyId(0x10, false, 0x36)).toBe("ShiftRight");
    expect(resolveKeyId(0x11, true, 0x1d)).toBe("ControlRight");
    expect(resolveKeyId(0x11, false, 0x1d)).toBe("ControlLeft");
  });

  it("routes numpad scans and extended enter", () => {
    expect(resolveKeyId(0x0d, true, 0x1c)).toBe("NumpadEnter");
    expect(resolveKeyId(0x0d, false, 0x1c)).toBe("Enter");
  });

  it("keeps unknown virtual keys unmapped", () => {
    expect(resolveKeyId(0xff, false, 0)).toBeNull();
  });

  it("labels caps in both languages", () => {
    expect(keyLabel("KeyA", "en")).toBe("A");
    expect(keyLabel("KeyA", "ru")).toBe("Ф");
    expect(keyLabel(null, "en")).toBe("?");
    expect(keyLabel("KeyA", "auto")).toBe("A");
  });

  it("builds friendly names with fallback", () => {
    expect(friendlyName(0x41, false, 0x1e, "en")).toBe("A");
    expect(friendlyName(0xff, false, 0, "en")).toBe("VK 0xFF");
  });

  it("orders chords with modifiers first", () => {
    expect(chordLabel(["KeyA", "ShiftLeft"])).toBe("SHIFT + A");
    expect(chordLabel(["ControlLeft", "KeyC"])).toBe("CTRL + C");
  });

  it("recognizes modifiers", () => {
    expect(isModifier("ShiftLeft")).toBe(true);
    expect(isModifier("KeyA")).toBe(false);
    expect(isModifier(null)).toBe(false);
    expect(isModifierVk(0x5b)).toBe(true);
    expect(isModifierVk(0x41)).toBe(false);
  });
});

describe("keyboard layouts", () => {
  it("builds a 60% board without function row", () => {
    const layout = buildLayout("60", false);
    expect(layout.size).toBe("60");
    expect(layout.keys.length).toBeGreaterThan(50);
    expect(layout.keys.some((key) => key.id === "F1")).toBe(false);
    expect(layout.keys.some((key) => key.id === "KeyA")).toBe(true);
  });

  it("builds a full board with numpad", () => {
    const layout = buildLayout("full", false);
    expect(layout.keys.some((key) => key.id === "F12")).toBe(true);
    expect(layout.keys.some((key) => key.id.startsWith("Numpad"))).toBe(true);
  });

  it("keeps every key inside the declared bounds", () => {
    for (const size of ["full", "tkl", "75", "65", "60"] as const) {
      const layout = buildLayout(size, false);
      for (const key of layout.keys) {
        expect(key.x).toBeGreaterThanOrEqual(0);
        expect(key.y).toBeGreaterThanOrEqual(0);
        expect(key.x + key.w).toBeLessThanOrEqual(layout.width + 0.01);
        expect(key.y + key.h).toBeLessThanOrEqual(layout.height + 0.01);
      }
    }
  });
});
