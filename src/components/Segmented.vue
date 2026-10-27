<script setup lang="ts" generic="T extends string">
defineProps<{
  modelValue: T;
  options: { value: T; label: string }[];
  disabled?: boolean;
}>();
const emit = defineEmits<{ "update:modelValue": [value: T] }>();
</script>

<template>
  <div class="inline-flex p-1 rounded-xl bg-white/[0.035] border border-white/[0.06] gap-0.5">
    <button
      v-for="opt in options"
      :key="opt.value"
      class="h-7 px-3 rounded-[9px] text-[12px] font-medium transition-all duration-150"
      :class="modelValue === opt.value ? 'bg-white/[0.09] text-text shadow-sm' : 'text-dim hover:text-muted'"
      :disabled="disabled"
      @click="emit('update:modelValue', opt.value)"
    >
      {{ opt.label }}
    </button>
  </div>
</template>
