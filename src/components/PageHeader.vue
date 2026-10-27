<script setup lang="ts">
defineProps<{ title: string; subtitle?: string; overline?: string }>();
</script>

<template>
  <div class="flex items-end justify-between gap-6 mb-6">
    <div>
      <div v-if="overline" class="overline mb-2">{{ overline }}</div>
      <h1 class="font-display text-[26px] leading-none font-semibold tracking-tight">{{ title }}</h1>
      <p v-if="subtitle" class="text-[12.5px] text-dim mt-2">{{ subtitle }}</p>
    </div>
    <div class="flex items-center gap-2 shrink-0">
      <slot name="actions" />
    </div>
  </div>
</template>
