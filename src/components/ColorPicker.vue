<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { clamp, formatHex, hexToRgb, rgbToHex } from "../lib/format";

const props = withDefaults(defineProps<{ modelValue: string; disabled?: boolean; presets?: boolean }>(), {
  disabled: false,
  presets: true,
});
const emit = defineEmits<{ "update:modelValue": [value: string] }>();

const palette = ["#ff4fa3", "#a78bfa", "#6ee7f9", "#45d483", "#f5c451", "#ff6b81", "#ffffff", "#2a2142"];

function rgbToHsv(hex: string): [number, number, number] {
  let [r, g, b] = hexToRgb(hex);
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const d = max - min;
  let h = 0;
  if (d !== 0) {
    if (max === r) h = ((g - b) / d + (g < b ? 6 : 0)) / 6;
    else if (max === g) h = ((b - r) / d + 2) / 6;
    else h = ((r - g) / d + 4) / 6;
  }
  return [h * 360, max === 0 ? 0 : d / max, max];
}

function hsvToHex(h: number, s: number, v: number): string {
  const c = v * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = v - c;
  const seg = Math.floor(((h % 360) + 360) % 360 / 60);
  const table: [number, number, number][] = [
    [c, x, 0], [x, c, 0], [0, c, x], [0, x, c], [x, 0, c], [c, 0, x],
  ];
  const [r, g, b] = table[seg] ?? [0, 0, 0];
  return rgbToHex((r + m) * 255, (g + m) * 255, (b + m) * 255);
}

const hsv = ref<[number, number, number]>(rgbToHsv(formatHex(props.modelValue)));
watch(
  () => props.modelValue,
  (value) => {
    const next = rgbToHsv(formatHex(value));
    if (hsvToHex(hsv.value[0], hsv.value[1], hsv.value[2]).toLowerCase() !== formatHex(value).toLowerCase()) {
      hsv.value = next;
    }
  }
);

const hue = computed(() => hsv.value[0]);
const hex = computed(() => hsvToHex(hsv.value[0], hsv.value[1], hsv.value[2]));
const rgb = computed(() => hexToRgb(hex.value));

const text = ref(hex.value);
watch(hex, (value) => (text.value = value));

function commitHex(value: string) {
  if (!/^#?[0-9a-fA-F]{6}$/.test(value)) return;
  const clean = formatHex(value);
  emit("update:modelValue", clean);
  hsv.value = rgbToHsv(clean);
}

function onText(ev: Event) {
  text.value = (ev.target as HTMLInputElement).value;
  commitHex(text.value);
}

function setChannel(index: number, value: string) {
  const next = [...rgb.value] as [number, number, number];
  next[index] = clamp(Number(value) || 0, 0, 255);
  commitHex(rgbToHex(...next));
}

const sv = ref<HTMLElement | null>(null);
const dragging = ref(false);

function pickSv(ev: PointerEvent) {
  const el = sv.value;
  if (!el) return;
  const rect = el.getBoundingClientRect();
  const s = clamp((ev.clientX - rect.left) / rect.width, 0, 1);
  const v = clamp(1 - (ev.clientY - rect.top) / rect.height, 0, 1);
  hsv.value = [hsv.value[0], s, v];
  emit("update:modelValue", hex.value);
}

function onSvDown(ev: PointerEvent) {
  dragging.value = true;
  (ev.currentTarget as HTMLElement).setPointerCapture(ev.pointerId);
  pickSv(ev);
}

function onSvMove(ev: PointerEvent) {
  if (dragging.value) pickSv(ev);
}

function onHue(ev: Event) {
  hsv.value = [Number((ev.target as HTMLInputElement).value), hsv.value[1], hsv.value[2]];
  emit("update:modelValue", hex.value);
}
</script>

<template>
  <div :class="disabled ? 'opacity-40 pointer-events-none select-none' : ''">
    <div
      ref="sv"
      class="relative h-[132px] rounded-xl border border-white/10 overflow-hidden cursor-crosshair touch-none"
      :style="{
        background: `linear-gradient(to top, #000, transparent), linear-gradient(to right, #fff, hsl(${hue} 100% 50%))`,
      }"
      @pointerdown="onSvDown"
      @pointermove="onSvMove"
      @pointerup="dragging = false"
      @pointercancel="dragging = false"
    >
      <span
        class="absolute w-4 h-4 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-white shadow-[0_0_0_1px_rgba(0,0,0,.45)]"
        :style="{ left: hsv[1] * 100 + '%', top: (1 - hsv[2]) * 100 + '%', background: hex }"
      ></span>
    </div>

    <input
      type="range"
      class="hue-slider mt-3 w-full"
      min="0"
      max="360"
      step="1"
      :value="Math.round(hue)"
      @input="onHue"
    />

    <div class="flex items-center gap-2 mt-3">
      <input class="input !h-9 mono !text-[12px] uppercase flex-1" :value="text" maxlength="7" @input="onText" />
      <input
        v-for="(channel, index) in rgb"
        :key="index"
        class="input no-spin !h-9 !w-12 mono !text-[11.5px] text-center"
        type="number"
        min="0"
        max="255"
        :value="channel"
        @change="setChannel(index, ($event.target as HTMLInputElement).value)"
      />
    </div>

    <div v-if="presets" class="flex flex-wrap gap-1.5 mt-3">
      <button
        v-for="swatch in palette"
        :key="swatch"
        class="w-6 h-6 rounded-full border border-white/15 transition-transform duration-150 hover:scale-110"
        :style="{ background: swatch }"
        :title="swatch"
        @click="commitHex(swatch)"
      ></button>
    </div>
  </div>
</template>

<style scoped>
.hue-slider {
  -webkit-appearance: none;
  appearance: none;
  height: 12px;
  border-radius: 999px;
  background: linear-gradient(90deg, #ff4b4b, #ffd84b, #5eea6d, #4bd8e8, #5b7bff, #c45bff, #ff4b4b);
  outline: none;
  cursor: pointer;
}
.hue-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 16px;
  height: 16px;
  border-radius: 999px;
  background: #fff;
  border: 2px solid rgba(0, 0, 0, 0.35);
  box-shadow: 0 1px 6px rgba(0, 0, 0, 0.55);
}
</style>
