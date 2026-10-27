<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    modelValue: number;
    min?: number;
    max?: number;
    step?: number;
    disabled?: boolean;
    unit?: string;
    format?: (v: number) => string;
  }>(),
  { min: 0, max: 100, step: 1, disabled: false, unit: "" }
);

const emit = defineEmits<{ "update:modelValue": [value: number] }>();

const display = computed(() => (props.format ? props.format(props.modelValue) : `${Math.round(props.modelValue)}${props.unit}`));

function onInput(ev: Event) {
  emit("update:modelValue", Number((ev.target as HTMLInputElement).value));
}

const fill = computed(() => {
  const pct = ((props.modelValue - props.min) / (props.max - props.min)) * 100;
  return Math.max(0, Math.min(100, pct));
});
</script>

<template>
  <div class="flex items-center gap-3">
    <input
      type="range"
      class="range flex-1"
      :min="min"
      :max="max"
      :step="step"
      :value="modelValue"
      :disabled="disabled"
      :style="{ '--fill': fill + '%' }"
      @input="onInput"
    />
    <span class="mono text-[11.5px] text-muted w-[54px] text-right tabular-nums">{{ display }}</span>
  </div>
</template>
