import type { Capability } from "./types";

export function clamp(v: number, min: number, max: number) {
  return Math.min(max, Math.max(min, v));
}

export function hexToRgb(hex: string): [number, number, number] {
  const clean = hex.replace("#", "");
  const full = clean.length === 3 ? clean.split("").map((c) => c + c).join("") : clean;
  const n = parseInt(full, 16);
  if (Number.isNaN(n)) return [255, 79, 163];
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

export function rgbToHex(r: number, g: number, b: number): string {
  const to = (v: number) => clamp(Math.round(v), 0, 255).toString(16).padStart(2, "0");
  return `#${to(r)}${to(g)}${to(b)}`;
}

export function rgbToHsl(r: number, g: number, b: number): [number, number, number] {
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  let h = 0;
  let s = 0;
  const l = (max + min) / 2;
  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    switch (max) {
      case r:
        h = (g - b) / d + (g < b ? 6 : 0);
        break;
      case g:
        h = (b - r) / d + 2;
        break;
      default:
        h = (r - g) / d + 4;
    }
    h /= 6;
  }
  return [Math.round(h * 360), Math.round(s * 100), Math.round(l * 100)];
}

export function hexToHsl(hex: string): [number, number, number] {
  const [r, g, b] = hexToRgb(hex);
  return rgbToHsl(r, g, b);
}

export function hslToHex(h: number, s: number, l: number): string {
  const sn = s / 100;
  const ln = l / 100;
  const k = (n: number) => (n + h / 30) % 12;
  const a = sn * Math.min(ln, 1 - ln);
  const f = (n: number) => ln - a * Math.max(-1, Math.min(k(n) - 3, Math.min(9 - k(n), 1)));
  return rgbToHex(f(0) * 255, f(8) * 255, f(4) * 255);
}

export function formatHex(value: string): string {
  const rgb = hexToRgb(value);
  return rgbToHex(rgb[0], rgb[1], rgb[2]);
}

export function localizedError(message: string, locale: "ru" | "en"): string {
  const separator = message.indexOf(" / ");
  if (separator < 0) return message;
  return locale === "en" ? message.slice(separator + 3) : message.slice(0, separator);
}

export function percent(v: number): string {
  return `${Math.round(v)}%`;
}

export function timeAgo(ts: number, locale: "ru" | "en" = "ru"): string {
  const diff = Date.now() - ts;
  const min = Math.floor(diff / 60000);
  if (min < 1) return locale === "ru" ? "только что" : "just now";
  if (min < 60) return locale === "ru" ? `${min} мин назад` : `${min} min ago`;
  const hours = Math.floor(min / 60);
  if (hours < 24) return locale === "ru" ? `${hours} ч назад` : `${hours} h ago`;
  const days = Math.floor(hours / 24);
  return locale === "ru" ? `${days} д назад` : `${days} d ago`;
}

export function formatDuration(ms: number): string {
  const total = Math.max(0, Math.round(ms));
  const s = Math.floor(total / 1000);
  const m = Math.floor(s / 60);
  if (m > 0) return `${m}:${String(s % 60).padStart(2, "0")}`;
  return `${(total / 1000).toFixed(2)} s`;
}

export function formatDateTime(ts: number, locale: "ru" | "en" = "ru"): string {
  const d = new Date(ts);
  return d.toLocaleString(locale === "ru" ? "ru-RU" : "en-US", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function hexBytes(hex: string): number[] {
  const clean = hex.replace(/[^0-9a-fA-F]/g, "");
  const out: number[] = [];
  for (let i = 0; i + 1 < clean.length; i += 2) out.push(parseInt(clean.slice(i, i + 2), 16));
  return out;
}

export function toHexBytes(bytes: number[]): string {
  return bytes.map((b) => b.toString(16).padStart(2, "0")).join(" ");
}

export function deviceKindLabel(kind: string, locale: "ru" | "en"): string {
  const map: Record<string, [string, string]> = {
    keyboard: ["Клавиатура", "Keyboard"],
    mouse: ["Мышь", "Mouse"],
    headset: ["Гарнитура", "Headset"],
    microphone: ["Микрофон", "Microphone"],
    gamepad: ["Контроллер", "Controller"],
    hid: ["HID-устройство", "HID device"],
    audio: ["Аудиоустройство", "Audio device"],
    other: ["Устройство", "Device"],
  };
  return map[kind]?.[locale === "ru" ? 0 : 1] ?? kind;
}

export function compatibilitySummary(caps: Capability[] = []) {
  const counts = { working: 0, partial: 0, research: 0, unsupported: 0 };
  for (const c of caps) counts[c.status] = (counts[c.status] ?? 0) + 1;
  const confirmed = counts.working + counts.partial;
  return { counts, confirmed, total: caps.length };
}

// Домашний каталог свёрнут в переменную окружения: путь остаётся понятным, но не выдаёт логин.
export function shortenHomePath(value = ""): string {
  return value.replace(/^([a-zA-Z]:[\\/][Uu]sers[\\/][^\\/]+)/, "%USERPROFILE%");
}
