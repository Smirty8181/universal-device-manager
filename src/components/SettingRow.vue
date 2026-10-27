<script setup lang="ts">
withDefaults(defineProps<{ label: string; description?: string; border?: boolean }>(), { border: true });
</script>

<template>
  <div class="flex items-center justify-between gap-6 py-3.5" :class="border ? 'border-b border-white/[0.045] last:border-0' : ''">
    <div class="min-w-0">
      <div class="text-[12.5px] font-medium">{{ label }}</div>
      <div v-if="description" class="text-[11px] text-dim mt-0.5 leading-relaxed">{{ description }}</div>
    </div>
    <div class="shrink-0 flex items-center gap-2"><slot /></div>
  </div>
</template>
