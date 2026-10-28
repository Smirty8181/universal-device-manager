<script setup lang="ts">
import { computed, ref } from "vue";
import { t } from "../lib/i18n";

const props = withDefaults(
  defineProps<{
    selected?: string | null;
    assigned?: Record<string, string>;
    compact?: boolean;
    interactive?: boolean;
    accent?: string;
  }>(),
  { selected: null, assigned: () => ({}), compact: false, interactive: true, accent: "#ff4fa3" }
);

const emit = defineEmits<{ pick: [id: string] }>();
const hover = ref<string | null>(null);

function click(id: string) {
  if (!props.interactive) return;
  emit("pick", id);
}

const active = computed(() => props.selected ?? hover.value);

function stroke(id: string) {
  return active.value === id ? props.accent : "rgba(255,255,255,0.08)";
}
function glow(id: string) {
  return active.value === id ? "drop-shadow(0 0 12px rgba(255,79,163,.35))" : "none";
}
</script>

<template>
  <svg viewBox="0 0 320 520" class="w-full h-full max-h-[430px] select-none" :class="compact ? 'pointer-events-none' : ''">
    <defs>
      <linearGradient id="ms-body" x1="0" y1="0" x2="0.8" y2="1">
        <stop offset="0%" stop-color="#1e1830" />
        <stop offset="55%" stop-color="#120e1f" />
        <stop offset="100%" stop-color="#0a0812" />
      </linearGradient>
      <linearGradient id="ms-sheen" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0%" stop-color="rgba(255,255,255,0.14)" />
        <stop offset="45%" stop-color="rgba(255,255,255,0.02)" />
        <stop offset="100%" stop-color="rgba(255,255,255,0.05)" />
      </linearGradient>
      <radialGradient id="ms-glow" cx="0.5" cy="0.5" r="0.5">
        <stop offset="0%" :stop-color="accent" stop-opacity="0.22" />
        <stop offset="100%" :stop-color="accent" stop-opacity="0" />
      </radialGradient>
      <clipPath id="ms-clip">
        <path d="M160,24 C226,24 262,84 262,170 C262,308 232,472 160,494 C88,472 58,308 58,170 C58,84 94,24 160,24 Z" />
      </clipPath>
    </defs>

    <ellipse cx="160" cy="470" rx="120" ry="26" fill="url(#ms-glow)" />

    <g :style="{ filter: glow('body') }">
      <path
        d="M160,24 C226,24 262,84 262,170 C262,308 232,472 160,494 C88,472 58,308 58,170 C58,84 94,24 160,24 Z"
        fill="url(#ms-body)"
        stroke="rgba(255,255,255,0.09)"
        stroke-width="1.5"
      />
      <path
        d="M160,24 C226,24 262,84 262,170 C262,308 232,472 160,494 C88,472 58,308 58,170 C58,84 94,24 160,24 Z"
        fill="url(#ms-sheen)"
        opacity="0.5"
      />
    </g>

    <!-- left button -->
    <g class="cursor-pointer" @click="click('left')" @mouseenter="hover = 'left'" @mouseleave="hover = null">
      <path
        d="M158,26 C100,28 68,88 66,168 L66,196 C96,210 130,216 158,216 Z"
        fill="#171225"
        :stroke="stroke('left')"
        stroke-width="1.2"
        style="transition: fill 120ms ease, stroke 120ms ease"
        :style="{ filter: glow('left') }"
      />
      <text x="106" y="128" font-size="8.5" fill="rgba(255,255,255,0.22)" font-family="Segoe UI" letter-spacing="1">LMB</text>
      <title>{{ t("mouse.left") }}</title>
    </g>

    <!-- right button -->
    <g class="cursor-pointer" @click="click('right')" @mouseenter="hover = 'right'" @mouseleave="hover = null">
      <path
        d="M162,26 C220,28 252,88 254,168 L254,196 C224,210 190,216 162,216 Z"
        fill="#171225"
        :stroke="stroke('right')"
        stroke-width="1.2"
        style="transition: fill 120ms ease, stroke 120ms ease"
        :style="{ filter: glow('right') }"
      />
      <text x="196" y="128" font-size="8.5" fill="rgba(255,255,255,0.22)" font-family="Segoe UI" letter-spacing="1">RMB</text>
      <title>{{ t("mouse.right") }}</title>
    </g>

    <!-- seam -->
    <path d="M160,26 L160,150" stroke="rgba(0,0,0,0.55)" stroke-width="2" />
    <path d="M66,196 C100,210 130,216 160,216 C190,216 224,210 254,196" fill="none" stroke="rgba(0,0,0,0.5)" stroke-width="1.4" />

    <!-- wheel -->
    <g class="cursor-pointer" @click="click('wheel')" @mouseenter="hover = 'wheel'" @mouseleave="hover = null">
      <rect x="147" y="54" width="26" height="64" rx="13" fill="#0a0813" :stroke="stroke('wheel')" stroke-width="1.4" :style="{ filter: glow('wheel') }" />
      <rect x="153" y="64" width="14" height="12" rx="3" fill="rgba(255,255,255,0.22)" />
      <rect x="153" y="80" width="14" height="12" rx="3" fill="rgba(255,255,255,0.12)" />
      <title>{{ t("mouse.wheel") }}</title>
    </g>

    <!-- DPI button -->
    <g class="cursor-pointer" @click="click('dpi')" @mouseenter="hover = 'dpi'" @mouseleave="hover = null">
      <rect x="149" y="128" width="22" height="12" rx="6" fill="#241a38" :stroke="stroke('dpi')" stroke-width="1.2" :style="{ filter: glow('dpi') }" />
      <circle cx="160" cy="134" r="2" :fill="accent" />
      <title>{{ t("mouse.dpi") }}</title>
    </g>

    <!-- side buttons -->
    <g class="cursor-pointer" @click="click('sideForward')" @mouseenter="hover = 'sideForward'" @mouseleave="hover = null">
      <path d="M66,232 L48,226 C44,225 42,230 44,235 L64,248 Z" fill="#1c1630" :stroke="stroke('sideForward')" stroke-width="1.2" :style="{ filter: glow('sideForward') }" />
      <title>{{ t("mouse.sideForward") }}</title>
    </g>
    <g class="cursor-pointer" @click="click('sideBack')" @mouseenter="hover = 'sideBack'" @mouseleave="hover = null">
      <path d="M64,262 L46,258 C42,257 41,262 43,267 L63,278 Z" fill="#1c1630" :stroke="stroke('sideBack')" stroke-width="1.2" :style="{ filter: glow('sideBack') }" />
      <title>{{ t("mouse.sideBack") }}</title>
    </g>

    <!-- accents -->
    <g clip-path="url(#ms-clip)">
      <path d="M74,330 C120,352 200,352 246,330" stroke="rgba(255,255,255,0.05)" stroke-width="1" fill="none" />
      <path d="M80,356 C122,376 198,376 240,356" stroke="rgba(255,255,255,0.035)" stroke-width="1" fill="none" />
    </g>
    <circle cx="160" cy="300" r="4" :fill="accent" opacity="0.9">
      <animate attributeName="opacity" values="0.35;1;0.35" dur="3.2s" repeatCount="indefinite" />
    </circle>
    <circle cx="160" cy="300" r="12" :fill="accent" opacity="0.08" />

    <text x="160" y="416" text-anchor="middle" font-size="26" font-weight="700" fill="rgba(255,255,255,0.06)" font-family="Segoe UI" letter-spacing="4">AJAZZ</text>
    <text x="160" y="432" text-anchor="middle" font-size="8" fill="rgba(255,255,255,0.16)" font-family="Segoe UI" letter-spacing="3">AJ159 NL</text>
  </svg>
</template>
