<script setup lang="ts">
import type { Component } from "vue";
import { CircleDashed } from "lucide-vue-next";

withDefaults(defineProps<{ icon?: Component; title: string; body?: string }>(), { icon: () => CircleDashed });
</script>

<template>
  <div class="flex flex-col items-center justify-center text-center py-12 px-6">
    <div class="w-12 h-12 rounded-2xl border border-white/10 bg-white/[0.03] grid place-items-center text-dim mb-4">
      <component :is="icon" :size="20" />
    </div>
    <div class="text-[13.5px] font-semibold">{{ title }}</div>
    <div v-if="body" class="text-[12px] text-dim mt-1.5 max-w-[380px] leading-relaxed">{{ body }}</div>
    <div v-if="$slots.default" class="mt-4"><slot /></div>
  </div>
</template>
