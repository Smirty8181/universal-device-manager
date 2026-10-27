<script setup lang="ts">
const props = withDefaults(defineProps<{ modelValue: boolean; disabled?: boolean }>(), { disabled: false });
const emit = defineEmits<{ "update:modelValue": [value: boolean] }>();

function toggle() {
  if (props.disabled) return;
  emit("update:modelValue", !props.modelValue);
}
</script>

<template>
  <button
    type="button"
    class="w-[42px] h-[24px] rounded-full relative transition-colors duration-150 shrink-0"
    :class="modelValue ? 'bg-gradient-to-r from-accent to-accent-2' : 'bg-white/[0.09]'"
    :style="modelValue ? 'box-shadow: 0 0 18px rgba(255,79,163,.25)' : ''"
    :disabled="disabled"
    :aria-pressed="modelValue"
    @click="toggle"
  >
    <span
      class="absolute top-[3px] w-[18px] h-[18px] rounded-full bg-white transition-all duration-150"
      :class="modelValue ? 'left-[21px]' : 'left-[3px]'"
      :style="modelValue ? 'background:#160a12' : 'opacity:.85'"
    ></span>
  </button>
</template>
