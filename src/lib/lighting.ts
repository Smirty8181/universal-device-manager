import type { EffectId } from "./rgbPreview";

export interface LightingEffect {
  id: number;
  key: string;
  preview: EffectId;
  color: boolean;
  direction: "linear" | "spiral" | null;
}

// Effect ids and direction bytes are verified against Alumix TKL firmware 0x34.
export const ALUMIX_EFFECTS: LightingEffect[] = [
  { id: 0, key: "wave", preview: "wave", color: false, direction: "linear" },
  { id: 1, key: "spiral", preview: "wave", color: false, direction: "spiral" },
  { id: 2, key: "random", preview: "rainbow", color: false, direction: null },
  { id: 3, key: "star", preview: "ripple", color: false, direction: null },
  { id: 4, key: "footprint", preview: "reactive", color: true, direction: null },
  { id: 5, key: "river", preview: "gradient", color: true, direction: null },
  { id: 6, key: "cycle", preview: "spectrum", color: false, direction: null },
  { id: 7, key: "breathing", preview: "breathing", color: true, direction: null },
  { id: 8, key: "solid", preview: "static", color: true, direction: null },
  { id: 9, key: "ripples", preview: "ripple", color: true, direction: null },
  { id: 10, key: "trigger", preview: "reactive", color: true, direction: null },
  { id: 11, key: "dischange", preview: "gradient", color: true, direction: null },
  { id: 12, key: "sine", preview: "wave", color: true, direction: null },
  { id: 13, key: "rain", preview: "rain", color: true, direction: null },
  { id: 19, key: "custom", preview: "custom", color: false, direction: null },
];

const LEGACY_KEYS: Record<string, number> = {
  static: 8,
  rainbow: 2,
  spectrum: 6,
  gradient: 5,
  reactive: 10,
  ripple: 9,
  fire: 13,
};

export function findLightingEffect(id: number): LightingEffect {
  return ALUMIX_EFFECTS.find((effect) => effect.id === id) ?? ALUMIX_EFFECTS[8];
}

export function effectLabelKey(id: number): string {
  return `kbEffect.${findLightingEffect(id).key}`;
}

export function effectIdFromKey(key: string): number | null {
  return ALUMIX_EFFECTS.find((effect) => effect.key === key)?.id ?? LEGACY_KEYS[key] ?? null;
}

export function usesColor(id: number): boolean {
  return findLightingEffect(id).color;
}

export function directionMode(id: number): LightingEffect["direction"] {
  return findLightingEffect(id).direction;
}

export function speedApplies(id: number): boolean {
  return id !== 8 && id !== 19;
}

// Hardware reports the raw HID direction byte; the engine accepts UI order 0=right,1=up,2=left,3=down.
export function directionFromHardware(value: number, effect: number): number {
  if (effect === 1) return Math.min(value, 1);
  switch (value) {
    case 0: return 0;
    case 3: return 1;
    case 1: return 2;
    case 2: return 3;
    default: return 0;
  }
}

export interface LightingPreset {
  id: string;
  name: string;
  effect: number;
  brightness: number;
  speed: number;
  direction: number;
  color: string;
  createdAt: number;
}

const PRESET_KEY = "udm.lighting.presets";

export function loadPresets(): LightingPreset[] {
  try {
    const raw = localStorage.getItem(PRESET_KEY);
    const parsed = raw ? (JSON.parse(raw) as LightingPreset[]) : [];
    return Array.isArray(parsed) ? parsed.filter((preset) => effectIdFromKey(String(preset.effect)) !== null || typeof preset.effect === "number") : [];
  } catch {
    return [];
  }
}

export function storePresets(presets: LightingPreset[]) {
  try {
    localStorage.setItem(PRESET_KEY, JSON.stringify(presets));
  } catch {}
}
