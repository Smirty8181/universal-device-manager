<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from "vue";

const props = withDefaults(
  defineProps<{ value: number; duration?: number; suffix?: string; decimals?: number }>(),
  { duration: 520, suffix: "", decimals: 0 }
);

const reduced = typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
const shown = ref(props.value);
let frame = 0;

function animate(from: number, to: number) {
  cancelAnimationFrame(frame);
  if (reduced || from === to) {
    shown.value = to;
    return;
  }
  const start = performance.now();
  const tick = (now: number) => {
    const t = Math.min(1, (now - start) / props.duration);
    const eased = 1 - Math.pow(1 - t, 3);
    shown.value = from + (to - from) * eased;
    if (t < 1) frame = requestAnimationFrame(tick);
  };
  frame = requestAnimationFrame(tick);
}

watch(
  () => props.value,
  (next, prev) => animate(prev, next)
);

onBeforeUnmount(() => cancelAnimationFrame(frame));
</script>

<template>
  <span class="tabular-nums">{{ shown.toFixed(decimals) }}{{ suffix }}</span>
</template>
