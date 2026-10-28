<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { formatHex, hexToRgb, rgbToHex } from "../lib/format";

const props = defineProps<{ modelValue: string; label?: string; disabled?: boolean }>();
const emit = defineEmits<{ "update:modelValue": [value: string] }>();

const hex = ref(props.modelValue);
watch(
  () => props.modelValue,
  (v) => (hex.value = v)
);

const rgb = computed(() => hexToRgb(hex.value));

function commit() {
  emit("update:modelValue", formatHex(hex.value));
}

function onText(ev: Event) {
  hex.value = (ev.target as HTMLInputElement).value;
  if (/^#?[0-9a-fA-F]{6}$/.test(hex.value)) commit();
}

function setChannel(idx: number, value: string) {
  const n = Math.max(0, Math.min(255, Number(value) || 0));
  const next = [...rgb.value] as [number, number, number];
  next[idx] = n;
  hex.value = rgbToHex(...next);
  commit();
}
</script>

<template>
  <div>
    <div v-if="label" class="text-[11.5px] text-muted mb-1.5">{{ label }}</div>
    <div class="flex items-center gap-2.5">
      <input type="color" class="color-swatch" :value="formatHex(modelValue)" :disabled="disabled" @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)" />
      <input class="input !h-9 mono !text-[12px] uppercase" :value="hex" :disabled="disabled" maxlength="7" @input="onText" />
      <div class="flex gap-1.5">
        <input
          v-for="(ch, i) in rgb"
          :key="i"
          class="input no-spin !h-9 !w-12 mono !text-[11.5px] text-center"
          type="number"
          min="0"
          max="255"
          :value="ch"
          :disabled="disabled"
          @change="setChannel(i, ($event.target as HTMLInputElement).value)"
        />
      </div>
    </div>
  </div>
</template>
