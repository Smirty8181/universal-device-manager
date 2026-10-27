<script setup lang="ts">
import { computed } from "vue";
import { BatteryFull, BatteryLow, BatteryMedium, BatteryWarning, Zap } from "lucide-vue-next";
import type { BatteryInfo } from "../lib/types";
import { useI18n } from "../lib/i18n";

const props = withDefaults(defineProps<{ battery?: BatteryInfo | null; showText?: boolean }>(), { battery: null, showText: true });
const { t, locale } = useI18n();

const percent = computed(() => props.battery?.percent ?? null);
const status = computed(() => props.battery?.status ?? (percent.value !== null ? "real" : "unknown"));
const stale = computed(() => status.value === "disconnected" || status.value === "unknown");

const config = computed(() => {
  const p = percent.value;
  if (p === null) return { icon: BatteryWarning, color: "text-dim", bar: "bg-white/15", label: t("battery.unknown") };
  if (stale.value) return { icon: BatteryWarning, color: "text-dim", bar: "bg-white/15", label: `${p}%` };
  if (p <= 10) return { icon: BatteryWarning, color: "text-bad", bar: "bg-bad", label: `${p}%` };
  if (p <= 25) return { icon: BatteryLow, color: "text-warn", bar: "bg-warn", label: `${p}%` };
  if (p <= 60) return { icon: BatteryMedium, color: "text-info", bar: "bg-info", label: `${p}%` };
  return { icon: BatteryFull, color: "text-ok", bar: "bg-ok", label: `${p}%` };
});

const note = computed(() => (locale.value === "ru" ? props.battery?.noteRu : props.battery?.noteEn) ?? "");
const statusLabel = computed(() => t(`battery.status.${status.value}`));
</script>

<template>
  <div class="flex items-center gap-2" :title="[statusLabel, stale ? t('battery.lastKnown') : '', note].filter(Boolean).join(' · ')">
    <component :is="config.icon" :size="15" :class="config.color" />
    <template v-if="showText">
      <span class="mono text-[12px] tabular-nums" :class="percent === null || stale ? 'text-dim' : 'text-text'">{{ config.label }}</span>
      <Zap v-if="battery?.charging" :size="12" class="text-warn" />
      <span v-if="stale && percent !== null" class="text-[9.5px] text-dim">{{ t("battery.lastKnown") }}</span>
    </template>
  </div>
</template>
