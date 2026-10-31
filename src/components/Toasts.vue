<script setup lang="ts">
import { Check, Info, TriangleAlert, XCircle, X } from "lucide-vue-next";
import { useAppStore } from "../stores/app";

const app = useAppStore();

const kindConfig = {
  info: { icon: Info, cls: "text-info border-info/30 bg-info/5" },
  success: { icon: Check, cls: "text-ok border-ok/30 bg-ok/5" },
  warn: { icon: TriangleAlert, cls: "text-warn border-warn/30 bg-warn/5" },
  error: { icon: XCircle, cls: "text-bad border-bad/30 bg-bad/5" },
} as const;
</script>

<template>
  <div class="fixed bottom-5 right-5 z-[100] flex flex-col gap-2 w-[340px]">
    <transition-group name="toast">
      <div v-for="toast in app.toasts" :key="toast.id" class="glass rounded-2xl px-4 py-3 border shadow-2xl flex gap-3 items-start" :class="kindConfig[toast.kind].cls">
        <component :is="kindConfig[toast.kind].icon" :size="16" class="mt-0.5 shrink-0" />
        <div class="min-w-0 flex-1">
          <div class="text-[12.5px] font-semibold leading-snug">{{ toast.title }}</div>
          <div v-if="toast.body" class="text-[11.5px] text-muted mt-0.5 break-words">{{ toast.body }}</div>
        </div>
        <button class="text-dim hover:text-text transition-colors" @click="app.dismissToast(toast.id)">
          <X :size="14" />
        </button>
      </div>
    </transition-group>
  </div>
</template>

<style scoped>
.toast-enter-active, .toast-leave-active { transition: all 220ms ease; }
.toast-enter-from { opacity: 0; transform: translateX(20px); }
.toast-leave-to { opacity: 0; transform: translateY(8px); }
</style>
