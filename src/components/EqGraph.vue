<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from "vue";

const props = withDefaults(defineProps<{ bands: number[]; editable?: boolean; min?: number; max?: number }>(), {
  editable: true,
  min: -12,
  max: 12,
});

const emit = defineEmits<{ "update:bands": [value: number[]] }>();

const FREQS = [32, 64, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];
const W = 720;
const H = 260;
const PAD = { l: 26, r: 14, t: 14, b: 26 };

const svgRef = ref<SVGSVGElement | null>(null);
const dragIndex = ref<number | null>(null);

const xFor = (i: number) => PAD.l + (i * (W - PAD.l - PAD.r)) / (FREQS.length - 1);
const yFor = (gain: number) => {
  const norm = (gain - props.min) / (props.max - props.min);
  return H - PAD.b - norm * (H - PAD.t - PAD.b);
};

const points = computed(() => props.bands.map((g, i) => [xFor(i), yFor(g)] as const));

function smoothPath(pts: readonly (readonly [number, number])[], close = false) {
  if (pts.length < 2) return "";
  let d = `M ${pts[0][0]} ${pts[0][1]}`;
  for (let i = 0; i < pts.length - 1; i++) {
    const p0 = pts[i - 1] ?? pts[i];
    const p1 = pts[i];
    const p2 = pts[i + 1];
    const p3 = pts[i + 2] ?? p2;
    const c1x = p1[0] + (p2[0] - p0[0]) / 6;
    const c1y = p1[1] + (p2[1] - p0[1]) / 6;
    const c2x = p2[0] - (p3[0] - p1[0]) / 6;
    const c2y = p2[1] - (p3[1] - p1[1]) / 6;
    d += ` C ${c1x} ${c1y}, ${c2x} ${c2y}, ${p2[0]} ${p2[1]}`;
  }
  if (close) d += ` L ${pts[pts.length - 1][0]} ${H - PAD.b} L ${pts[0][0]} ${H - PAD.b} Z`;
  return d;
}

const linePath = computed(() => smoothPath(points.value));
const areaPath = computed(() => smoothPath(points.value, true));

function gainFromEvent(ev: PointerEvent): number {
  const svg = svgRef.value;
  if (!svg) return 0;
  const rect = svg.getBoundingClientRect();
  const y = ((ev.clientY - rect.top) / rect.height) * H;
  const norm = 1 - (y - PAD.t) / (H - PAD.t - PAD.b);
  const gain = props.min + Math.max(0, Math.min(1, norm)) * (props.max - props.min);
  return Math.round(gain * 2) / 2;
}

function onPointerDown(i: number, ev: PointerEvent) {
  if (!props.editable) return;
  dragIndex.value = i;
  (ev.target as Element).setPointerCapture?.(ev.pointerId);
  applyGain(i, ev);
}

function onPointerMove(ev: PointerEvent) {
  if (dragIndex.value === null) return;
  applyGain(dragIndex.value, ev);
}

function applyGain(i: number, ev: PointerEvent) {
  const next = [...props.bands];
  next[i] = gainFromEvent(ev);
  emit("update:bands", next);
}

function onPointerUp() {
  dragIndex.value = null;
}

onBeforeUnmount(() => (dragIndex.value = null));
</script>

<template>
  <svg
    ref="svgRef"
    :viewBox="`0 0 ${W} ${H}`"
    class="w-full select-none touch-none"
    :class="editable ? 'cursor-crosshair' : ''"
    @pointermove="onPointerMove"
    @pointerup="onPointerUp"
    @pointerleave="onPointerUp"
  >
    <defs>
      <linearGradient id="eq-fill" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="var(--color-accent)" stop-opacity="0.22" />
        <stop offset="100%" stop-color="var(--color-accent)" stop-opacity="0" />
      </linearGradient>
      <linearGradient id="eq-line" x1="0" y1="0" x2="1" y2="0">
        <stop offset="0%" stop-color="var(--color-accent)" />
        <stop offset="100%" stop-color="var(--color-accent-2)" />
      </linearGradient>
    </defs>

    <g stroke="rgba(255,255,255,0.05)" stroke-width="1">
      <line v-for="g in [12, 6, 0, -6, -12]" :key="g" :x1="PAD.l" :x2="W - PAD.r" :y1="yFor(g)" :y2="yFor(g)" />
      <line v-for="(f, i) in FREQS" :key="f" :x1="xFor(i)" :x2="xFor(i)" :y1="PAD.t" :y2="H - PAD.b" />
    </g>

    <text v-for="g in [12, 0, -12]" :key="'l' + g" :x="PAD.l - 7" :y="yFor(g) + 3" text-anchor="end" font-size="8" fill="rgba(255,255,255,0.28)" font-family="Cascadia Mono">{{ g > 0 ? "+" + g : g }}</text>

    <path :d="areaPath" fill="url(#eq-fill)" />
    <path :d="linePath" fill="none" stroke="url(#eq-line)" stroke-width="2" stroke-linecap="round" />

    <g v-for="(p, i) in points" :key="i">
      <circle
        :cx="p[0]"
        :cy="p[1]"
        :r="dragIndex === i ? 6 : 4.5"
        fill="#0c0912"
        stroke="var(--color-accent)"
        stroke-width="1.8"
        :style="dragIndex === i ? 'filter: drop-shadow(0 0 8px rgba(255,79,163,.8))' : ''"
        @pointerdown.prevent="onPointerDown(i, $event)"
      />
      <text :x="p[0]" :y="H - 10" text-anchor="middle" font-size="8" fill="rgba(255,255,255,0.3)" font-family="Cascadia Mono">
        {{ FREQS[i] >= 1000 ? FREQS[i] / 1000 + "k" : FREQS[i] }}
      </text>
      <text :x="p[0]" :y="p[1] - 10" text-anchor="middle" font-size="7.5" fill="rgba(255,255,255,0.5)" font-family="Cascadia Mono">
        {{ bands[i] > 0 ? "+" + bands[i] : bands[i] }}
      </text>
    </g>
  </svg>
</template>
