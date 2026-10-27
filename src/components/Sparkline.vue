<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{ data: number[]; height?: number; min?: number; max?: number; color?: string }>(),
  { height: 36, min: 0, max: 100, color: "var(--color-accent)" }
);

const w = 160;

const points = computed(() => {
  const d = props.data;
  if (d.length < 2) return null;
  const step = w / (d.length - 1);
  return d.map((v, i) => {
    const norm = (v - props.min) / (props.max - props.min || 1);
    return [i * step, props.height - 4 - norm * (props.height - 8)] as const;
  });
});

const line = computed(() => (points.value ? points.value.map(([x, y]) => `${x.toFixed(1)},${y.toFixed(1)}`).join(" ") : ""));
const area = computed(() => (points.value ? `0,${props.height} ${line.value} ${w},${props.height}` : ""));
</script>

<template>
  <svg v-if="points" :viewBox="`0 0 ${w} ${height}`" class="w-full" :style="{ height: height + 'px' }" preserveAspectRatio="none">
    <defs>
      <linearGradient :id="`spark-${color.replace(/[^a-z]/g, '')}`" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" :stop-color="color" stop-opacity="0.28" />
        <stop offset="100%" :stop-color="color" stop-opacity="0" />
      </linearGradient>
    </defs>
    <polygon :points="area" :fill="`url(#spark-${color.replace(/[^a-z]/g, '')})`" />
    <polyline :points="line" fill="none" :stroke="color" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
  </svg>
  <div v-else class="w-full flex items-center justify-center" :style="{ height: height + 'px' }">
    <div class="w-full border-t border-dashed border-white/10"></div>
  </div>
</template>
