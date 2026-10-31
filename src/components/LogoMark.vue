<script setup lang="ts">
import { onBeforeUnmount, ref } from "vue";

const props = withDefaults(defineProps<{ size?: number; interactive?: boolean }>(), { size: 36, interactive: true });
const emit = defineEmits<{ activate: [] }>();

const mystic = ref(false);
let hoverTimer: number | undefined;

function onEnter() {
  if (!props.interactive) return;
  window.clearTimeout(hoverTimer);
  hoverTimer = window.setTimeout(() => {
    mystic.value = true;
    window.setTimeout(() => (mystic.value = false), 2400);
  }, 1500);
}

function onLeave() {
  window.clearTimeout(hoverTimer);
}

onBeforeUnmount(() => window.clearTimeout(hoverTimer));
</script>

<template>
  <button
    type="button"
    class="logo-mark shrink-0 relative grid place-items-center rounded-[13px] border border-white/10"
    :class="{ mystic, interactive }"
    :style="{ width: size + 'px', height: size + 'px' }"
    :aria-label="'UDM'"
    @click="emit('activate')"
    @pointerenter="onEnter"
    @pointerleave="onLeave"
  >
    <span class="logo-sheen absolute inset-0 rounded-[13px] overflow-hidden"></span>
    <svg :width="size * 0.52" :height="size * 0.52" viewBox="0 0 24 24" fill="none" class="relative z-10 logo-glyph">
      <path d="M7 5v8.2a5 5 0 0 0 10 0V5" stroke="#170a15" stroke-width="2.6" stroke-linecap="round" />
      <circle cx="17.7" cy="17.4" r="1.7" fill="#170a15" />
    </svg>
    <span class="logo-ring absolute inset-0 rounded-[13px] pointer-events-none"></span>
  </button>
</template>

<style scoped>
.logo-mark {
  background: linear-gradient(135deg, var(--color-accent) 0%, var(--color-accent-2) 100%);
  box-shadow: 0 8px 22px -10px color-mix(in srgb, var(--color-accent) 65%, transparent), inset 0 1px 0 rgba(255, 255, 255, 0.35);
  transition: transform 200ms var(--ease-spring), box-shadow 240ms ease, border-color 200ms ease;
}
.logo-mark.interactive:hover {
  transform: translateY(-1px) scale(1.035);
  box-shadow: 0 10px 26px -8px color-mix(in srgb, var(--color-accent) 75%, transparent), inset 0 1px 0 rgba(255, 255, 255, 0.4);
}
.logo-mark:active { transform: scale(0.96); }

.logo-sheen::before {
  content: "";
  position: absolute;
  inset: -40% -60%;
  background: linear-gradient(115deg, transparent 38%, rgba(255, 255, 255, 0.5) 50%, transparent 62%);
  transform: translateX(-70%);
  animation: logo-sheen 5.6s var(--ease-in-out) infinite;
  animation-delay: 1.2s;
}
.logo-glyph { transition: transform 300ms var(--ease-spring); }
.logo-mark:hover .logo-glyph { transform: rotate(-4deg) scale(1.04); }

.logo-ring { border: 1px solid transparent; }
.logo-mark.mystic .logo-ring {
  border-color: color-mix(in srgb, #fff 60%, transparent);
  animation: logo-ring 1.2s var(--ease-out) 2;
}
.logo-mark.mystic { animation: logo-mystic 1.2s var(--ease-spring) 2; }

@keyframes logo-sheen {
  0%, 55% { transform: translateX(-70%); }
  80%, 100% { transform: translateX(70%); }
}
@keyframes logo-ring {
  0% { transform: scale(1); opacity: 0.8; }
  100% { transform: scale(1.7); opacity: 0; }
}
@keyframes logo-mystic {
  0%, 100% { transform: rotate(0deg) scale(1); }
  30% { transform: rotate(-8deg) scale(1.08); }
  60% { transform: rotate(6deg) scale(1.04); }
}
</style>
