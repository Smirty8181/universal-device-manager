<script setup lang="ts">
import { onBeforeUnmount, onMounted } from "vue";
import { X } from "lucide-vue-next";

const props = withDefaults(defineProps<{ open: boolean; title?: string; width?: string }>(), { width: "460px" });
const emit = defineEmits<{ close: [] }>();

function onKey(ev: KeyboardEvent) {
  if (ev.key === "Escape" && props.open) emit("close");
}
onMounted(() => window.addEventListener("keydown", onKey));
onBeforeUnmount(() => window.removeEventListener("keydown", onKey));
</script>

<template>
  <teleport to="body">
    <transition name="fade">
      <div v-if="open" class="fixed inset-0 z-[90] bg-black/60 backdrop-blur-sm grid place-items-center p-6" @mousedown.self="emit('close')">
        <div class="card !bg-[#0d0b15]/95 w-full max-h-[86vh] flex flex-col shadow-2xl" :style="{ maxWidth: width }">
          <div class="flex items-center justify-between px-5 py-4 border-b border-white/5">
            <div class="section-title">{{ title }}</div>
            <button class="btn btn-ghost !h-8 !w-8 !p-0" @click="emit('close')"><X :size="15" /></button>
          </div>
          <div class="p-5 overflow-y-auto scroll-region">
            <slot />
          </div>
          <div v-if="$slots.footer" class="px-5 py-4 border-t border-white/5 flex justify-end gap-2">
            <slot name="footer" />
          </div>
        </div>
      </div>
    </transition>
  </teleport>
</template>
