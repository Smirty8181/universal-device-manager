<script setup lang="ts">
import type { Capability } from "../lib/types";
import { capabilityLabel } from "../lib/capabilities";
import { useI18n } from "../lib/i18n";
import StatusChip from "./StatusChip.vue";

defineProps<{ capabilities: Capability[] }>();
const { t, locale } = useI18n();
</script>

<template>
  <div class="card p-5">
    <div class="flex items-center justify-between mb-1">
      <div class="section-title">{{ t("device.capabilityMatrix") }}</div>
      <span class="text-[10.5px] text-dim">{{ t("device.capabilityMatrixBody") }}</span>
    </div>
    <div v-for="cap in capabilities" :key="cap.id" class="grid grid-cols-[240px_150px_1fr] gap-4 items-start py-3 border-b border-white/[0.045] last:border-0">
      <div class="text-[12.5px] font-medium pt-0.5">{{ capabilityLabel(cap.id, locale) }}</div>
      <StatusChip :status="cap.status" />
      <div class="text-[11.5px] text-dim leading-relaxed">{{ locale === "ru" ? cap.noteRu : cap.noteEn }}</div>
    </div>
  </div>
</template>
