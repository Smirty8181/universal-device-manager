<script setup lang="ts">
import { computed } from "vue";
import { CheckCircle2, CircleDashed, TriangleAlert, XCircle } from "lucide-vue-next";
import type { CapabilityStatus } from "../lib/types";
import { useI18n } from "../lib/i18n";

const props = defineProps<{ status: CapabilityStatus; compact?: boolean }>();
const { t } = useI18n();

const config = computed(() => {
  switch (props.status) {
    case "working":
      return { icon: CheckCircle2, chip: "chip-ok", label: t("status.working") };
    case "partial":
      return { icon: TriangleAlert, chip: "chip-warn", label: t("status.partial") };
    case "unsupported":
      return { icon: XCircle, chip: "chip-bad", label: t("status.unsupported") };
    default:
      return { icon: CircleDashed, chip: "chip-info", label: t("status.research") };
  }
});
</script>

<template>
  <span class="chip" :class="config.chip">
    <component :is="config.icon" :size="11" />
    <span v-if="!compact">{{ config.label }}</span>
  </span>
</template>
