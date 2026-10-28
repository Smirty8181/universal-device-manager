export type LayoutSize = "full" | "tkl" | "75" | "65" | "60";
export type LabelLang = "auto" | "ru" | "en";

export interface KeyCap {
  id: string;
  label: string;
  ru?: string;
  sub?: string;
  vks: number[];
  scan?: number[];
}

export interface PlacedKey extends KeyCap {
  x: number;
  y: number;
  w: number;
  h: number;
  cluster: "main" | "function" | "nav" | "numpad" | "arrows";
}

export interface BoardLayout {
  size: LayoutSize;
  iso: boolean;
  width: number;
  height: number;
  keys: PlacedKey[];
}

function cap(id: string, label: string, vks: number[], extra: Partial<KeyCap> = {}): KeyCap {
  return { id, label, vks, ...extra };
}

const letter = (id: string, latin: string, vk: number, ru: string) => cap(id, latin, [vk], { ru });
const digit = (id: string, label: string, sym: string, vk: number) => cap(id, label, [vk], { sub: sym });

export const KEY_CAPS: Record<string, KeyCap> = {};
function reg(...caps: KeyCap[]) {
  for (const item of caps) KEY_CAPS[item.id] = item;
}

reg(
  cap("Escape", "Esc", [0x1b]),
  cap("F1", "F1", [0x70]),
  cap("F2", "F2", [0x71]),
  cap("F3", "F3", [0x72]),
  cap("F4", "F4", [0x73]),
  cap("F5", "F5", [0x74]),
  cap("F6", "F6", [0x75]),
  cap("F7", "F7", [0x76]),
  cap("F8", "F8", [0x77]),
  cap("F9", "F9", [0x78]),
  cap("F10", "F10", [0x79]),
  cap("F11", "F11", [0x7a]),
  cap("F12", "F12", [0x7b]),
  cap("PrintScreen", "PrtSc", [0x2c]),
  cap("ScrollLock", "ScrLk", [0x91]),
  cap("Pause", "Pause", [0x13]),
  cap("Backquote", "`", [0xc0], { sub: "~", ru: "Ё" }),
  digit("Digit1", "1", "!", 0x31),
  digit("Digit2", "2", "@", 0x32),
  digit("Digit3", "3", "#", 0x33),
  digit("Digit4", "4", "$", 0x34),
  digit("Digit5", "5", "%", 0x35),
  digit("Digit6", "6", "^", 0x36),
  digit("Digit7", "7", "&", 0x37),
  digit("Digit8", "8", "*", 0x38),
  digit("Digit9", "9", "(", 0x39),
  digit("Digit0", "0", ")", 0x30),
  cap("Minus", "-", [0xbd], { sub: "_" }),
  cap("Equal", "=", [0xbb], { sub: "+" }),
  cap("Backspace", "Backspace", [0x08]),
  cap("Tab", "Tab", [0x09]),
  letter("KeyQ", "Q", 0x51, "Й"),
  letter("KeyW", "W", 0x57, "Ц"),
  letter("KeyE", "E", 0x45, "У"),
  letter("KeyR", "R", 0x52, "К"),
  letter("KeyT", "T", 0x54, "Е"),
  letter("KeyY", "Y", 0x59, "Н"),
  letter("KeyU", "U", 0x55, "Г"),
  letter("KeyI", "I", 0x49, "Ш"),
  letter("KeyO", "O", 0x4f, "Щ"),
  letter("KeyP", "P", 0x50, "З"),
  cap("BracketLeft", "[", [0xdb], { sub: "{", ru: "Х" }),
  cap("BracketRight", "]", [0xdd], { sub: "}", ru: "Ъ" }),
  cap("Backslash", "\\", [0xdc], { sub: "|", ru: "\\" }),
  cap("CapsLock", "Caps", [0x14]),
  letter("KeyA", "A", 0x41, "Ф"),
  letter("KeyS", "S", 0x53, "Ы"),
  letter("KeyD", "D", 0x44, "В"),
  letter("KeyF", "F", 0x46, "А"),
  letter("KeyG", "G", 0x47, "П"),
  letter("KeyH", "H", 0x48, "Р"),
  letter("KeyJ", "J", 0x4a, "О"),
  letter("KeyK", "K", 0x4b, "Л"),
  letter("KeyL", "L", 0x4c, "Д"),
  cap("Semicolon", ";", [0xba], { sub: ":", ru: "Ж" }),
  cap("Quote", "'", [0xde], { sub: '"', ru: "Э" }),
  cap("Enter", "Enter", [0x0d]),
  cap("IntlBackslash", "\\", [0xe2], { sub: "|" }),
  letter("KeyZ", "Z", 0x5a, "Я"),
  letter("KeyX", "X", 0x58, "Ч"),
  letter("KeyC", "C", 0x43, "С"),
  letter("KeyV", "V", 0x56, "М"),
  letter("KeyB", "B", 0x42, "И"),
  letter("KeyN", "N", 0x4e, "Т"),
  letter("KeyM", "M", 0x4d, "Ь"),
  cap("Comma", ",", [0xbc], { sub: "<", ru: "Б" }),
  cap("Period", ".", [0xbe], { sub: ">", ru: "Ю" }),
  cap("Slash", "/", [0xbf], { sub: "?", ru: "." }),
  cap("ShiftLeft", "Shift", [0xa0, 0x10], { scan: [0x2a] }),
  cap("ShiftRight", "Shift", [0xa1, 0x10], { scan: [0x36] }),
  cap("ControlLeft", "Ctrl", [0xa2, 0x11], { scan: [0x1d] }),
  cap("ControlRight", "Ctrl", [0xa3, 0x11], { scan: [0x1d] }),
  cap("AltLeft", "Alt", [0xa4, 0x12], { scan: [0x38] }),
  cap("AltRight", "AltGr", [0xa5, 0x12], { scan: [0x38] }),
  cap("MetaLeft", "Win", [0x5b]),
  cap("MetaRight", "Win", [0x5c]),
  cap("ContextMenu", "Menu", [0x5d]),
  cap("Space", "Space", [0x20]),
  cap("Insert", "Ins", [0x2d]),
  cap("Delete", "Del", [0x2e]),
  cap("Home", "Home", [0x24]),
  cap("End", "End", [0x23]),
  cap("PageUp", "PgUp", [0x21]),
  cap("PageDown", "PgDn", [0x22]),
  cap("ArrowUp", "↑", [0x26]),
  cap("ArrowLeft", "←", [0x25]),
  cap("ArrowDown", "↓", [0x28]),
  cap("ArrowRight", "→", [0x27]),
  cap("NumLock", "Num", [0x90]),
  cap("NumpadDivide", "/", [0x6f]),
  cap("NumpadMultiply", "*", [0x6a]),
  cap("NumpadSubtract", "-", [0x6d]),
  cap("NumpadAdd", "+", [0x6b]),
  cap("Numpad7", "7", [0x67]),
  cap("Numpad8", "8", [0x68]),
  cap("Numpad9", "9", [0x69]),
  cap("Numpad4", "4", [0x64]),
  cap("Numpad5", "5", [0x65]),
  cap("Numpad6", "6", [0x66]),
  cap("Numpad1", "1", [0x61]),
  cap("Numpad2", "2", [0x62]),
  cap("Numpad3", "3", [0x63]),
  cap("Numpad0", "0", [0x60]),
  cap("NumpadDecimal", ".", [0x6e]),
  cap("NumpadEnter", "Enter", [0x0d])
);

const NUMPAD_SCANS: Record<number, string> = {
  0x47: "Numpad7",
  0x48: "Numpad8",
  0x49: "Numpad9",
  0x4a: "NumpadSubtract",
  0x4b: "Numpad4",
  0x4c: "Numpad5",
  0x4d: "Numpad6",
  0x4e: "NumpadAdd",
  0x4f: "Numpad1",
  0x50: "Numpad2",
  0x51: "Numpad3",
  0x52: "Numpad0",
  0x53: "NumpadDecimal",
};

const DIRECT_VK: Record<number, string> = {
  0x08: "Backspace",
  0x09: "Tab",
  0x0d: "Enter",
  0x13: "Pause",
  0x14: "CapsLock",
  0x1b: "Escape",
  0x20: "Space",
  0x21: "PageUp",
  0x22: "PageDown",
  0x23: "End",
  0x24: "Home",
  0x25: "ArrowLeft",
  0x26: "ArrowUp",
  0x27: "ArrowRight",
  0x28: "ArrowDown",
  0x2c: "PrintScreen",
  0x2d: "Insert",
  0x2e: "Delete",
  0x5b: "MetaLeft",
  0x5c: "MetaRight",
  0x5d: "ContextMenu",
  0x5e: "NumpadDivide",
  0x5f: "NumpadMultiply",
  0x60: "Numpad0",
  0x61: "Numpad1",
  0x62: "Numpad2",
  0x63: "Numpad3",
  0x64: "Numpad4",
  0x65: "Numpad5",
  0x66: "Numpad6",
  0x67: "Numpad7",
  0x68: "Numpad8",
  0x69: "Numpad9",
  0x6a: "NumpadMultiply",
  0x6b: "NumpadAdd",
  0x6d: "NumpadSubtract",
  0x6e: "NumpadDecimal",
  0x6f: "NumpadDivide",
  0x90: "NumLock",
  0x91: "ScrollLock",
  0xa0: "ShiftLeft",
  0xa1: "ShiftRight",
  0xa2: "ControlLeft",
  0xa3: "ControlRight",
  0xa4: "AltLeft",
  0xa5: "AltRight",
  0xba: "Semicolon",
  0xbb: "Equal",
  0xbc: "Comma",
  0xbd: "Minus",
  0xbe: "Period",
  0xbf: "Slash",
  0xc0: "Backquote",
  0xdb: "BracketLeft",
  0xdc: "Backslash",
  0xdd: "BracketRight",
  0xde: "Quote",
  0xe2: "IntlBackslash",
};

for (let i = 0; i < 26; i += 1) {
  DIRECT_VK[0x41 + i] = `Key${String.fromCharCode(0x41 + i)}`;
}
for (let i = 0; i < 10; i += 1) {
  DIRECT_VK[0x30 + i] = `Digit${i}`;
}
for (let i = 0; i < 12; i += 1) {
  DIRECT_VK[0x70 + i] = `F${i + 1}`;
}

export function resolveKeyId(vk: number, extended: boolean, scan: number): string | null {
  if (vk === 0x10) return scan === 0x36 ? "ShiftRight" : "ShiftLeft";
  if (vk === 0x11) return extended ? "ControlRight" : "ControlLeft";
  if (vk === 0x12) return extended ? "AltRight" : "AltLeft";
  if (vk === 0x0d && extended) return "NumpadEnter";
  if (!extended && NUMPAD_SCANS[scan]) return NUMPAD_SCANS[scan];
  return DIRECT_VK[vk] ?? null;
}

export function isModifier(id: string | null): boolean {
  if (!id) return false;
  return /^(Shift|Control|Alt|Meta)/.test(id);
}

export function isModifierVk(vk: number): boolean {
  return vk === 0x10 || vk === 0x11 || vk === 0x12 || (vk >= 0xa0 && vk <= 0xa5) || vk === 0x5b || vk === 0x5c;
}

export function keyLabel(id: string | null, lang: LabelLang): string {
  if (!id) return "?";
  const item = KEY_CAPS[id];
  if (!item) return id;
  if (lang === "ru" && item.ru) return item.ru;
  return item.label;
}

export function friendlyName(vk: number, extended: boolean, scan: number, lang: LabelLang = "en"): string {
  const id = resolveKeyId(vk, extended, scan);
  if (!id) return `VK 0x${vk.toString(16).toUpperCase().padStart(2, "0")}`;
  const name = keyLabel(id, lang);
  if (id.startsWith("Numpad") && id !== "NumpadEnter") return `NUM ${name}`;
  return name.toUpperCase();
}

export function chordLabel(keys: string[]): string {
  const order = ["ControlLeft", "ControlRight", "AltLeft", "AltRight", "ShiftLeft", "ShiftRight", "MetaLeft", "MetaRight"];
  const mods = order.filter((id) => keys.includes(id));
  const rest = keys.filter((id) => !order.includes(id));
  const show = (id: string) => {
    if (id.startsWith("Control")) return "CTRL";
    if (id.startsWith("Alt")) return "ALT";
    if (id.startsWith("Shift")) return "SHIFT";
    if (id.startsWith("Meta")) return "WIN";
    return keyLabel(id, "en").toUpperCase();
  };
  return [...mods, ...rest].map(show).join(" + ");
}

type Add = (id: string, x: number, y: number, w?: number, h?: number, cluster?: PlacedKey["cluster"]) => void;

function placer(keys: PlacedKey[]): Add {
  return (id, x, y, w = 1, h = 1, cluster = "main") => {
    const item = KEY_CAPS[id];
    if (!item) return;
    keys.push({ ...item, x, y, w, h, cluster });
  };
}

function functionRow(add: Add, y: number, withNavTop: boolean) {
  add("Escape", 0, y, 1, 1, "function");
  const groups = [
    [1.25, ["F1", "F2", "F3", "F4"]],
    [4.75, ["F5", "F6", "F7", "F8"]],
    [8.25, ["F9", "F10", "F11", "F12"]],
  ] as const;
  for (const [start, ids] of groups) {
    ids.forEach((id, index) => add(id, start + index, y, 1, 1, "function"));
  }
  if (withNavTop) {
    add("PrintScreen", 15.25, y, 1, 1, "function");
    add("ScrollLock", 16.25, y, 1, 1, "function");
    add("Pause", 17.25, y, 1, 1, "function");
  }
}

function mainRows(add: Add, y: number, iso: boolean, withFunctionRow: boolean) {
  const firstRowY = withFunctionRow ? y + 1 : y;
  if (withFunctionRow) {
    add("Backquote", 0, firstRowY);
  } else {
    add("Escape", 0, firstRowY);
  }
  const digits = ["Digit1", "Digit2", "Digit3", "Digit4", "Digit5", "Digit6", "Digit7", "Digit8", "Digit9", "Digit0"];
  digits.forEach((id, index) => add(id, 1 + index, firstRowY));
  add("Minus", 11, firstRowY);
  add("Equal", 12, firstRowY);
  add("Backspace", 13, firstRowY, 2);

  const row2 = firstRowY + 1;
  add("Tab", 0, row2, 1.5);
  const top = ["KeyQ", "KeyW", "KeyE", "KeyR", "KeyT", "KeyY", "KeyU", "KeyI", "KeyO", "KeyP"];
  top.forEach((id, index) => add(id, 1.5 + index, row2));
  add("BracketLeft", 11.5, row2);
  add("BracketRight", 12.5, row2);
  if (iso) {
    add("Enter", 13.5, row2, 1.5);
  } else {
    add("Backslash", 13.5, row2, 1.5);
  }

  const row3 = row2 + 1;
  add("CapsLock", 0, row3, 1.75);
  const home = ["KeyA", "KeyS", "KeyD", "KeyF", "KeyG", "KeyH", "KeyJ", "KeyK", "KeyL"];
  home.forEach((id, index) => add(id, 1.75 + index, row3));
  add("Semicolon", 10.75, row3);
  add("Quote", 11.75, row3);
  if (iso) {
    add("IntlBackslash", 12.75, row3);
    add("Enter", 13.75, row3, 1.25);
  } else {
    add("Enter", 12.75, row3, 2.25);
  }

  const row4 = row3 + 1;
  if (iso) {
    add("ShiftLeft", 0, row4, 1.25);
    add("Backslash", 1.25, row4, 1);
    add("KeyZ", 2.25, row4);
    ["KeyX", "KeyC", "KeyV", "KeyB", "KeyN", "KeyM"].forEach((id, index) => add(id, 3.25 + index, row4));
    add("Comma", 9.25, row4);
    add("Period", 10.25, row4);
    add("Slash", 11.25, row4);
    add("ShiftRight", 12.25, row4, 2.75);
  } else {
    add("ShiftLeft", 0, row4, 2.25);
    add("KeyZ", 2.25, row4);
    ["KeyX", "KeyC", "KeyV", "KeyB", "KeyN", "KeyM"].forEach((id, index) => add(id, 3.25 + index, row4));
    add("Comma", 9.25, row4);
    add("Period", 10.25, row4);
    add("Slash", 11.25, row4);
    add("ShiftRight", 12.25, row4, 2.75);
  }

  const row5 = row4 + 1;
  add("ControlLeft", 0, row5, 1.25);
  add("MetaLeft", 1.25, row5, 1.25);
  add("AltLeft", 2.5, row5, 1.25);
  add("Space", 3.75, row5, 6.25);
  add("AltRight", 10, row5, 1.25);
  add("MetaRight", 11.25, row5, 1.25);
  add("ContextMenu", 12.5, row5, 1.25);
  add("ControlRight", 13.75, row5, 1.25);
}

function navCluster(add: Add) {
  add("Insert", 15.25, 1, 1, 1, "nav");
  add("Home", 16.25, 1, 1, 1, "nav");
  add("PageUp", 17.25, 1, 1, 1, "nav");
  add("Delete", 15.25, 2, 1, 1, "nav");
  add("End", 16.25, 2, 1, 1, "nav");
  add("PageDown", 17.25, 2, 1, 1, "nav");
  add("ArrowUp", 16.25, 4, 1, 1, "arrows");
  add("ArrowLeft", 15.25, 5, 1, 1, "arrows");
  add("ArrowDown", 16.25, 5, 1, 1, "arrows");
  add("ArrowRight", 17.25, 5, 1, 1, "arrows");
}

function numpad(add: Add) {
  const x = 18.5;
  add("NumLock", x, 1, 1, 1, "numpad");
  add("NumpadDivide", x + 1, 1, 1, 1, "numpad");
  add("NumpadMultiply", x + 2, 1, 1, 1, "numpad");
  add("NumpadSubtract", x + 3, 1, 1, 1, "numpad");
  add("Numpad7", x, 2, 1, 1, "numpad");
  add("Numpad8", x + 1, 2, 1, 1, "numpad");
  add("Numpad9", x + 2, 2, 1, 1, "numpad");
  add("NumpadAdd", x + 3, 2, 1, 2, "numpad");
  add("Numpad4", x, 3, 1, 1, "numpad");
  add("Numpad5", x + 1, 3, 1, 1, "numpad");
  add("Numpad6", x + 2, 3, 1, 1, "numpad");
  add("Numpad1", x, 4, 1, 1, "numpad");
  add("Numpad2", x + 1, 4, 1, 1, "numpad");
  add("Numpad3", x + 2, 4, 1, 1, "numpad");
  add("Numpad0", x, 5, 2, 1, "numpad");
  add("NumpadDecimal", x + 2, 5, 1, 1, "numpad");
  add("NumpadEnter", x + 3, 4, 1, 2, "numpad");
}

function compactRight(add: Add, top: number, height: number) {
  add("Delete", 15.25, top, 1, 1, "nav");
  add("Home", 15.25, top + 1, 1, 1, "nav");
  add("PageUp", 15.25, top + 2, 1, 1, "nav");
  add("PageDown", 15.25, top + 3, 1, 1, "nav");
  add("End", 15.25, top + 4, 1, 1, "nav");
  if (height >= 6) {
    add("PrintScreen", 15.25, 0, 1, 1, "function");
    add("ArrowUp", 17.25, 4, 1, 1, "arrows");
    add("ArrowLeft", 16.25, 5, 1, 1, "arrows");
    add("ArrowDown", 17.25, 5, 1, 1, "arrows");
    add("ArrowRight", 18.25, 5, 1, 1, "arrows");
  } else {
    add("ArrowUp", 17.25, 3, 1, 1, "arrows");
    add("ArrowLeft", 16.25, 4, 1, 1, "arrows");
    add("ArrowDown", 17.25, 4, 1, 1, "arrows");
    add("ArrowRight", 18.25, 4, 1, 1, "arrows");
  }
}

export function buildLayout(size: LayoutSize, iso: boolean): BoardLayout {
  const keys: PlacedKey[] = [];
  const add = placer(keys);
  let width = 15;
  let height = 5;
  switch (size) {
    case "full":
      height = 6;
      width = 22.5;
      functionRow(add, 0, true);
      mainRows(add, 0, iso, true);
      navCluster(add);
      numpad(add);
      break;
    case "tkl":
      height = 6;
      width = 18.25;
      functionRow(add, 0, true);
      mainRows(add, 0, iso, true);
      navCluster(add);
      break;
    case "75":
      height = 6;
      width = 19.25;
      functionRow(add, 0, false);
      mainRows(add, 0, iso, true);
      compactRight(add, 1, 6);
      break;
    case "65":
      height = 5;
      width = 19.25;
      mainRows(add, 0, iso, false);
      compactRight(add, 0, 5);
      break;
    default:
      height = 5;
      width = 15;
      mainRows(add, 0, iso, false);
      break;
  }
  return { size, iso, width, height, keys };
}

const cache = new Map<string, BoardLayout>();
export function getLayout(size: LayoutSize, iso: boolean): BoardLayout {
  const key = `${size}:${iso ? "iso" : "ansi"}`;
  let layout = cache.get(key);
  if (!layout) {
    layout = buildLayout(size, iso);
    cache.set(key, layout);
  }
  return layout;
}

const GAMING_KEYS = new Set([
  "Escape",
  "Digit1",
  "Digit2",
  "Digit3",
  "Digit4",
  "Digit5",
  "Tab",
  "KeyQ",
  "KeyW",
  "KeyE",
  "KeyR",
  "KeyA",
  "KeyS",
  "KeyD",
  "KeyF",
  "KeyG",
  "KeyZ",
  "KeyX",
  "KeyC",
  "KeyV",
  "Space",
  "ShiftLeft",
  "ControlLeft",
  "AltLeft",
]);

const TYPING_KEYS = new Set(Object.keys(KEY_CAPS).filter((id) => (KEY_CAPS[id].vks[0] ?? 0) < 0x70 && !id.startsWith("Numpad")));

export function isEmphasized(id: string, tracked: "all" | "gaming" | "typing"): boolean {
  if (tracked === "gaming") return GAMING_KEYS.has(id);
  if (tracked === "typing") return TYPING_KEYS.has(id);
  return true;
}
