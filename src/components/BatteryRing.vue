<script setup lang="ts">
import { computed } from "vue";
import { BatteryCharging, Zap } from "lucide-vue-next";
import type { BatteryInfo } from "../lib/types";
import { useI18n } from "../lib/i18n";
import { timeAgo } from "../lib/format";

const props = withDefaults(
  defineProps<{
    battery?: BatteryInfo | null;
    size?: number;
    stroke?: number;
    showLabel?: boolean;
    showSource?: boolean;
  }>(),
  { battery: null, size: 64, stroke: 5, showLabel: true, showSource: false }
);

const { t, locale } = useI18n();

const percent = computed(() => props.battery?.percent ?? null);
const radius = computed(() => (props.size - props.stroke) / 2);
const circumference = computed(() => 2 * Math.PI * radius.value);
const offset = computed(() =>
  percent.value === null ? circumference.value : circumference.value * (1 - Math.min(100, Math.max(0, percent.value)) / 100)
);

const status = computed(() => props.battery?.status ?? (percent.value !== null ? "real" : "unknown"));
const stale = computed(() => status.value === "disconnected" || status.value === "unknown");
const color = computed(() => {
  const p = percent.value;
  if (p === null || stale.value) return "#5f5874";
  if (p <= 10) return "#ff6b81";
  if (p <= 25) return "#f5c451";
  if (p <= 60) return "#6ee7f9";
  return "#45d483";
});

const note = computed(() => (locale.value === "ru" ? props.battery?.noteRu : props.battery?.noteEn) ?? "");
const full = computed(() => percent.value !== null && percent.value >= 99);
const charging = computed(() => props.battery?.charging === true || status.value === "charging");
const statusLabel = computed(() => t(`battery.status.${status.value}`));
const label = computed(() => {
  if (percent.value === null) return t("battery.unknown");
  if (stale.value) return `${percent.value}% · ${t("battery.lastKnown")}`;
  if (status.value === "estimated") return `${percent.value}% · ${t("battery.status.estimated")}`;
  if (full.value) return t("battery.full");
  if (charging.value) return t("battery.chargerConnected");
  return `${percent.value}%`;
});
</script>

<template>
  <div class="flex items-center gap-3" :title="note || t('battery.ringLabel')">
    <div class="relative shrink-0" :style="{ width: size + 'px', height: size + 'px' }">
      <svg :width="size" :height="size" class="-rotate-90">
        <circle
          :cx="size / 2"
          :cy="size / 2"
          :r="radius"
          fill="none"
          stroke="rgba(255,255,255,0.07)"
          :stroke-width="stroke"
        />
        <circle
          v-if="percent !== null"
          class="ring-progress"
          :cx="size / 2"
          :cy="size / 2"
          :r="radius"
          fill="none"
          :stroke="color"
          :stroke-width="stroke"
          stroke-linecap="round"
          :stroke-dasharray="circumference"
          :stroke-dashoffset="offset"
          :style="{ filter: `drop-shadow(0 0 5px ${color}66)` }"
        />
      </svg>
      <div class="absolute inset-0 grid place-items-center">
        <template v-if="percent !== null">
          <span class="font-display font-semibold tabular-nums leading-none" :style="{ fontSize: Math.max(12, size * 0.26) + 'px', color }">
            {{ percent }}<span class="text-[0.55em] opacity-70">%</span>
          </span>
        </template>
        <Zap v-else :size="Math.max(12, size * 0.28)" class="text-dim" />
      </div>
      <span
        v-if="charging"
        class="absolute -bottom-0.5 -right-0.5 w-[18px] h-[18px] rounded-full grid place-items-center bg-[#171123] border border-white/10"
        :title="t('battery.chargerConnected')"
      >
        <BatteryCharging :size="11" class="text-warn" />
      </span>
    </div>

    <div v-if="showLabel" class="min-w-0">
      <div class="flex items-baseline gap-1.5">
        <span class="text-[12px] font-semibold" :style="percent !== null && !stale ? { color } : undefined" :class="stale ? 'text-dim' : ''">
          {{ label }}
        </span>
      </div>
      <div v-if="showSource" class="text-[10px] text-dim mt-0.5">
        {{ t("battery.knownSources") }}: {{ t("battery.source." + (battery?.source ?? "unavailable")) }}
        · {{ statusLabel }}
        <template v-if="battery"> · {{ timeAgo(battery.updatedAt, locale) }}</template>
      </div>
      <div v-else-if="note" class="text-[10px] text-dim mt-0.5 truncate max-w-[220px]" :title="note">{{ note }}</div>
    </div>
  </div>
</template>
