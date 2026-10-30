<script setup lang="ts">
import type { HidInterface } from "../lib/types";
import { useI18n } from "../lib/i18n";

defineProps<{ interfaces: HidInterface[] }>();
const { t } = useI18n();

function usagePageHex(page: number) {
  return `0x${page.toString(16).toUpperCase().padStart(2, "0")}`;
}
function isVendor(page: number) {
  return page >= 0xff00;
}
</script>

<template>
  <div>
    <div v-if="!interfaces.length" class="text-[12px] text-dim py-2">{{ t("device.noInterfaces") }}</div>
    <div v-for="(i, idx) in interfaces" :key="i.path + idx" class="flex items-center gap-3 py-2.5 border-b border-white/[0.035] last:border-0">
      <span class="chip !h-[22px] mono !text-[10px]">{{ usagePageHex(i.usagePage) }}:{{ i.usage.toString(16).padStart(2, "0") }}</span>
      <span v-if="isVendor(i.usagePage)" class="chip chip-accent !h-[22px] !text-[10px]">{{ t("lab.warning") }}</span>
      <span class="text-[11.5px] text-dim">MI {{ i.interfaceNumber ?? "—" }}</span>
      <span class="ml-auto mono text-[10px] text-dim truncate max-w-[46%]" :title="i.path">{{ i.path }}</span>
    </div>
  </div>
</template>
