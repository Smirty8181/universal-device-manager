<script setup lang="ts">
withDefaults(defineProps<{ label: string; value?: string | number | null; mono?: boolean; wrap?: boolean }>(), { mono: false, wrap: false });
</script>

<template>
  <div class="flex items-baseline justify-between gap-4 py-2 border-b border-white/[0.035] last:border-0">
    <span class="text-[11.5px] text-dim shrink-0">{{ label }}</span>
    <span
      class="text-[12px] text-right"
      :class="[mono ? 'mono tabular-nums' : '', wrap ? 'break-all' : 'truncate max-w-[68%]']"
    >
      <slot>{{ value ?? "—" }}</slot>
    </span>
  </div>
</template>
