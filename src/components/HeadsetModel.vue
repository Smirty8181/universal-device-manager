<script setup lang="ts">
withDefaults(defineProps<{ accent?: string; compact?: boolean }>(), { accent: "#ff4fa3", compact: false });
</script>

<template>
  <svg viewBox="0 0 360 340" class="w-full h-auto select-none" :class="compact ? 'opacity-95' : ''">
    <defs>
      <linearGradient id="hs-band" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0%" stop-color="#2a2142" />
        <stop offset="100%" stop-color="#120e1e" />
      </linearGradient>
      <linearGradient id="hs-cup" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="#221a36" />
        <stop offset="100%" stop-color="#0d0a16" />
      </linearGradient>
      <radialGradient id="hs-glow" cx="0.5" cy="0.5" r="0.5">
        <stop offset="0%" :stop-color="accent" stop-opacity="0.28" />
        <stop offset="100%" :stop-color="accent" stop-opacity="0" />
      </radialGradient>
    </defs>

    <ellipse cx="180" cy="286" rx="130" ry="34" fill="url(#hs-glow)" />

    <!-- headband -->
    <path d="M64,214 C64,86 296,86 296,214" fill="none" stroke="url(#hs-band)" stroke-width="34" stroke-linecap="round" />
    <path d="M70,208 C70,96 290,96 290,208" fill="none" stroke="rgba(255,255,255,0.06)" stroke-width="2" />
    <path d="M84,196 C90,112 270,112 276,196" fill="none" stroke="rgba(255,255,255,0.035)" stroke-width="12" stroke-linecap="round" />

    <!-- left cup -->
    <g>
      <rect x="34" y="188" width="92" height="112" rx="34" fill="url(#hs-cup)" stroke="rgba(255,255,255,0.08)" />
      <rect x="48" y="202" width="64" height="84" rx="26" fill="#07060b" stroke="rgba(255,255,255,0.04)" />
      <ellipse cx="80" cy="244" rx="18" ry="24" fill="none" :stroke="accent" stroke-opacity="0.35" stroke-width="1.4" />
      <circle cx="80" cy="244" r="4" :fill="accent" opacity="0.85">
        <animate attributeName="opacity" values="0.35;0.95;0.35" dur="3.6s" repeatCount="indefinite" />
      </circle>
    </g>

    <!-- right cup -->
    <g>
      <rect x="234" y="188" width="92" height="112" rx="34" fill="url(#hs-cup)" stroke="rgba(255,255,255,0.08)" />
      <rect x="248" y="202" width="64" height="84" rx="26" fill="#07060b" stroke="rgba(255,255,255,0.04)" />
      <ellipse cx="280" cy="244" rx="18" ry="24" fill="none" :stroke="accent" stroke-opacity="0.35" stroke-width="1.4" />
      <circle cx="280" cy="244" r="4" :fill="accent" opacity="0.85">
        <animate attributeName="opacity" values="0.95;0.35;0.95" dur="3.6s" repeatCount="indefinite" />
      </circle>
    </g>

    <!-- boom mic hint -->
    <path d="M62,286 C58,310 78,318 96,314" fill="none" stroke="rgba(255,255,255,0.12)" stroke-width="4" stroke-linecap="round" />
    <circle cx="99" cy="313" r="5" fill="#171226" stroke="rgba(255,255,255,0.2)" />

    <text x="180" y="330" text-anchor="middle" font-size="7.5" fill="rgba(255,255,255,0.16)" font-family="Segoe UI" letter-spacing="3">GRAPHITE 2 PRO WIRELESS</text>
  </svg>
</template>
