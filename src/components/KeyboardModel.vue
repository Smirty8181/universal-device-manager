<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { keyColor, type EffectId } from "../lib/rgbPreview";

interface KeyDef {
  code: string;
  label: string;
  x: number;
  y: number;
  w: number;
  h?: number;
}

const props = withDefaults(
  defineProps<{
    effect?: EffectId;
    primary?: string;
    secondary?: string;
    brightness?: number;
    speed?: number;
    selectedKeys?: string[];
    interactive?: boolean;
    animate?: boolean;
    activeKey?: string | null;
    variant?: "full" | "mini";
  }>(),
  {
    effect: "static",
    primary: "#ff4fa3",
    secondary: "#a78bfa",
    brightness: 80,
    speed: 45,
    selectedKeys: () => [],
    interactive: true,
    animate: true,
    activeKey: null,
    variant: "full",
  }
);

const emit = defineEmits<{ keyClick: [code: string]; hoverKey: [code: string | null] }>();

const U = 42;
const GAP = 5;

const rows: { y: number; keys: [string, string, number][] }[] = [
  {
    y: 0,
    keys: [
      ["Escape", "Esc", 1], ["F1", "F1", 1], ["F2", "F2", 1], ["F3", "F3", 1], ["F4", "F4", 1],
      ["F5", "F5", 1], ["F6", "F6", 1], ["F7", "F7", 1], ["F8", "F8", 1], ["F9", "F9", 1],
      ["F10", "F10", 1], ["F11", "F11", 1], ["F12", "F12", 1],
    ],
  },
  {
    y: 1,
    keys: [
      ["Backquote", "`", 1], ["Digit1", "1", 1], ["Digit2", "2", 1], ["Digit3", "3", 1],
      ["Digit4", "4", 1], ["Digit5", "5", 1], ["Digit6", "6", 1], ["Digit7", "7", 1],
      ["Digit8", "8", 1], ["Digit9", "9", 1], ["Digit0", "0", 1], ["Minus", "-", 1],
      ["Equal", "=", 1], ["Backspace", "⌫", 2],
    ],
  },
  {
    y: 2,
    keys: [
      ["Tab", "Tab", 1.5], ["KeyQ", "Q", 1], ["KeyW", "W", 1], ["KeyE", "E", 1], ["KeyR", "R", 1],
      ["KeyT", "T", 1], ["KeyY", "Y", 1], ["KeyU", "U", 1], ["KeyI", "I", 1], ["KeyO", "O", 1],
      ["KeyP", "P", 1], ["BracketLeft", "[", 1], ["BracketRight", "]", 1], ["Backslash", "\\", 1.5],
    ],
  },
  {
    y: 3,
    keys: [
      ["CapsLock", "Caps", 1.75], ["KeyA", "A", 1], ["KeyS", "S", 1], ["KeyD", "D", 1],
      ["KeyF", "F", 1], ["KeyG", "G", 1], ["KeyH", "H", 1], ["KeyJ", "J", 1], ["KeyK", "K", 1],
      ["KeyL", "L", 1], ["Semicolon", ";", 1], ["Quote", "'", 1], ["Enter", "Enter", 2.25],
    ],
  },
  {
    y: 4,
    keys: [
      ["ShiftLeft", "Shift", 2.25], ["KeyZ", "Z", 1], ["KeyX", "X", 1], ["KeyC", "C", 1],
      ["KeyV", "V", 1], ["KeyB", "B", 1], ["KeyN", "N", 1], ["KeyM", "M", 1],
      ["Comma", ",", 1], ["Period", ".", 1], ["Slash", "/", 1], ["ShiftRight", "Shift", 2.75],
    ],
  },
  {
    y: 5,
    keys: [
      ["ControlLeft", "Ctrl", 1.25], ["MetaLeft", "Win", 1.25], ["AltLeft", "Alt", 1.25],
      ["Space", "", 6.25], ["AltRight", "Alt", 1.25], ["Fn", "Fn", 1.25],
      ["ContextMenu", "≡", 1.25], ["ControlRight", "Ctrl", 1.25],
    ],
  },
];

const navRows: { y: number; keys: [string, string][] }[] = [
  { y: 0, keys: [["PrintScreen", "Prt"], ["ScrollLock", "Scr"], ["Pause", "Pse"]] },
  { y: 1, keys: [["Insert", "Ins"], ["Home", "Home"], ["PageUp", "PgUp"]] },
  { y: 2, keys: [["Delete", "Del"], ["End", "End"], ["PageDown", "PgDn"]] },
];

const keys = computed<KeyDef[]>(() => {
  const out: KeyDef[] = [];
  for (const row of rows) {
    let x = 0;
    for (const [code, label, w] of row.keys) {
      out.push({ code, label, x, y: row.y, w });
      x += w;
    }
  }
  const navX = 15.35;
  for (const row of navRows) {
    row.keys.forEach(([code, label], i) => out.push({ code, label, x: navX + i, y: row.y, w: 1 }));
  }
  out.push({ code: "ArrowUp", label: "↑", x: navX + 1, y: 4, w: 1 });
  out.push({ code: "ArrowLeft", label: "←", x: navX, y: 5, w: 1 });
  out.push({ code: "ArrowDown", label: "↓", x: navX + 1, y: 5, w: 1 });
  out.push({ code: "ArrowRight", label: "→", x: navX + 2, y: 5, w: 1 });
  return out;
});

const time = ref(0);
let raf = 0;
let last = 0;

function tick(ts: number) {
  if (!last) last = ts;
  const dt = Math.min(100, ts - last);
  last = ts;
  time.value += dt / 1000;
  raf = requestAnimationFrame(tick);
}

onMounted(() => {
  if (props.animate && props.variant === "full") raf = requestAnimationFrame(tick);
});
onBeforeUnmount(() => cancelAnimationFrame(raf));

const activeSet = computed(() => new Set(props.selectedKeys));

function fillFor(userIndex: number, x: number, y: number) {
  if (props.variant === "mini") {
    const base = keyColor({
      effect: "gradient", primary: props.primary, secondary: props.secondary,
      brightness: props.brightness, speed: 0, time: 0, xNorm: x / 18.35, yNorm: y / 5,
      index: userIndex, selected: false, active: false,
    });
    return { fill: base, opacity: 0.5 };
  }
  const color = keyColor({
    effect: props.effect,
    primary: props.primary,
    secondary: props.secondary,
    brightness: props.brightness,
    speed: props.speed,
    time: time.value,
    xNorm: x / 18.35,
    yNorm: y / 5,
    index: userIndex,
    selected: activeSet.value.has(keys.value[userIndex]?.code ?? ""),
    active: props.activeKey === keys.value[userIndex]?.code,
  });
  return { fill: color, opacity: 1 };
}

function onKeyClick(code: string) {
  if (!props.interactive) return;
  emit("keyClick", code);
}
</script>

<template>
  <svg viewBox="0 0 800 285" class="w-full h-auto select-none" :class="variant === 'mini' ? 'opacity-90' : ''">
    <defs>
      <linearGradient id="kb-body" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0%" stop-color="#1b1628" />
        <stop offset="100%" stop-color="#0e0b16" />
      </linearGradient>
      <linearGradient id="kb-edge" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="rgba(255,255,255,0.16)" />
        <stop offset="100%" stop-color="rgba(255,255,255,0.02)" />
      </linearGradient>
    </defs>

    <rect x="1" y="1" width="798" height="283" rx="20" fill="url(#kb-body)" stroke="url(#kb-edge)" stroke-width="1.5" />
    <rect x="14" y="14" width="772" height="257" rx="12" fill="#08060d" stroke="rgba(255,255,255,0.04)" />

    <g v-for="(k, i) in keys" :key="k.code">
      <g
        class="cursor-pointer"
        :style="{ transition: 'transform 120ms ease' }"
        @click="onKeyClick(k.code)"
        @mouseenter="emit('hoverKey', k.code)"
        @mouseleave="emit('hoverKey', null)"
      >
        <rect
          :x="18 + k.x * U"
          :y="18 + k.y * U"
          :width="k.w * U - GAP"
          :height="(k.h ?? 1) * U - GAP"
          rx="6"
          :fill="fillFor(i, k.x + k.w / 2, k.y + 0.5).fill"
          :opacity="fillFor(i, k.x + k.w / 2, k.y + 0.5).opacity"
          :stroke="activeSet.has(k.code) ? secondary : 'rgba(255,255,255,0.08)'"
          :stroke-width="activeSet.has(k.code) ? 1.6 : 1"
          style="filter: drop-shadow(0 2px 6px rgba(0,0,0,.35))"
        />
        <text
          v-if="k.label && k.label.length <= 5"
          :x="18 + (k.x + k.w / 2) * U - GAP / 2"
          :y="18 + k.y * U + (U - GAP) / 2 + 3.5"
          text-anchor="middle"
          font-size="9"
          font-family="Segoe UI, sans-serif"
          font-weight="600"
          fill="rgba(10,8,16,0.78)"
          pointer-events="none"
        >
          {{ k.label }}
        </text>
      </g>
    </g>

    <text x="18" y="274" font-size="8" fill="rgba(255,255,255,0.18)" font-family="Segoe UI">ALUMIX TKL · 87 KEYS</text>
    <circle cx="778" cy="270" r="2.5" :fill="primary" opacity="0.9">
      <animate v-if="animate" attributeName="opacity" values="0.2;1;0.2" dur="2.4s" repeatCount="indefinite" />
    </circle>
  </svg>
</template>
