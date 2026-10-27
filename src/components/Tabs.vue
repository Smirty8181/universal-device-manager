<script setup lang="ts" generic="T extends string">
defineProps<{ modelValue: T; tabs: { id: T; label: string; count?: number }[] }>();
const emit = defineEmits<{ "update:modelValue": [value: T] }>();
</script>

<template>
  <div class="flex items-center gap-1 border-b border-white/[0.06] mb-5">
    <button
      v-for="tab in tabs"
      :key="tab.id"
      class="relative h-9 px-3.5 text-[12.5px] font-medium transition-colors"
      :class="modelValue === tab.id ? 'text-text' : 'text-dim hover:text-muted'"
      @click="emit('update:modelValue', tab.id)"
    >
      {{ tab.label }}
      <span v-if="tab.count !== undefined" class="ml-1.5 text-[10.5px] text-dim mono">{{ tab.count }}</span>
      <span v-if="modelValue === tab.id" class="absolute left-2 right-2 -bottom-px h-[2px] rounded-full" style="background: linear-gradient(90deg, var(--color-accent), var(--color-accent-2))"></span>
    </button>
  </div>
</template>
