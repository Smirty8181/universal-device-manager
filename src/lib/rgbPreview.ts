import { hexToRgb, rgbToHex, hslToHex } from "./format";

export type EffectId = "static" | "breathing" | "rainbow" | "wave" | "reactive" | "ripple" | "gradient" | "spectrum" | "fire" | "rain" | "custom";

export interface PreviewOptions {
  effect: EffectId;
  primary: string;
  secondary: string;
  brightness: number;
  speed: number;
  time: number;
  xNorm: number;
  yNorm: number;
  index: number;
  selected: boolean;
  active: boolean;
}

function lerp(a: number, b: number, t: number) {
  return a + (b - a) * t;
}

function lerpColor(a: string, b: string, t: number) {
  const [r1, g1, b1] = hexToRgb(a);
  const [r2, g2, b2] = hexToRgb(b);
  return rgbToHex(lerp(r1, r2, t), lerp(g1, g2, t), lerp(b1, b2, t));
}

function dim(hex: string, factor: number) {
  const [r, g, b] = hexToRgb(hex);
  return rgbToHex(r * factor, g * factor, b * factor);
}

function fireColor(index: number, time: number) {
  const n = Math.sin(index * 12.9898 + time * 2.4) * 43758.5453;
  const flicker = n - Math.floor(n);
  const hue = 18 + flicker * 38;
  const light = 34 + flicker * 30;
  return hslToHex(hue, 92, light);
}

export function keyColor(o: PreviewOptions): string {
  const bright = 0.28 + (o.brightness / 100) * 0.72;
  const speed = 0.25 + (o.speed / 100) * 1.85;
  const t = o.time * speed;

  if (o.selected && o.effect !== "rainbow" && o.effect !== "spectrum") {
    return dim(o.secondary, bright);
  }

  switch (o.effect) {
    case "static":
      return dim(o.primary, bright);
    case "breathing": {
      const pulse = 0.45 + 0.55 * (0.5 + 0.5 * Math.sin(t * 1.6));
      return dim(o.primary, bright * pulse);
    }
    case "rainbow": {
      const hue = (o.index * 4 + t * 40) % 360;
      return dim(hslToHex(hue, 82, 58), bright);
    }
    case "wave": {
      const hue = (o.xNorm * 260 + t * 60) % 360;
      return dim(hslToHex(hue, 80, 56), bright);
    }
    case "reactive": {
      if (o.active) return dim(o.primary, bright);
      return dim(o.secondary, bright * 0.35);
    }
    case "ripple": {
      const dist = Math.hypot(o.xNorm - 0.5, o.yNorm - 0.5);
      const ring = 0.5 + 0.5 * Math.sin(dist * 9 - t * 3);
      return dim(o.primary, bright * (0.25 + ring * 0.75));
    }
    case "gradient": {
      const base = lerpColor(o.primary, o.secondary, o.xNorm);
      return dim(base, bright);
    }
    case "spectrum": {
      const hue = o.xNorm * 300;
      return dim(hslToHex(hue, 84, 56), bright);
    }
    case "fire":
      return dim(fireColor(o.index, o.time), bright);
    case "rain": {
      const seed = Math.sin(o.index * 91.7 + Math.floor(t * 6) * 17.3) * 43758.5453;
      const drop = seed - Math.floor(seed);
      const sparkle = Math.pow(drop, 7);
      return dim(o.primary, bright * (0.1 + 0.9 * sparkle));
    }
    case "custom": {
      const band = Math.abs(Math.floor(o.xNorm * 9) - Math.floor(o.yNorm * 4)) % 3;
      const base = band === 0 ? o.primary : band === 1 ? o.secondary : lerpColor(o.primary, o.secondary, 0.5);
      const pulse = 0.74 + 0.26 * Math.sin(o.index * 0.7 + t * 0.8);
      return dim(base, bright * pulse);
    }
    default: {
      if (o.selected) return dim(o.primary, bright);
      return dim(o.secondary, bright * 0.4);
    }
  }
}
