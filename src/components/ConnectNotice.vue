<script setup lang="ts">
import { computed } from "vue";
import { useRouter } from "vue-router";
import { ArrowUpRight, Headphones, Keyboard, Mouse, Usb } from "lucide-vue-next";
import { useAppStore } from "../stores/app";
import { useDevicesStore } from "../stores/devices";
import { useI18n } from "../lib/i18n";
import { deviceKindLabel } from "../lib/format";

const app = useAppStore();
const devices = useDevicesStore();
const router = useRouter();
const { t, locale } = useI18n();

const notices = computed(() => {
  if (!app.settings.notificationsEnabled) return [];
  return Array.from(devices.recentlyConnected)
    .map((id) => devices.getDevice(id))
    .filter((d): d is NonNullable<typeof d> => Boolean(d));
});

function iconFor(kind: string) {
  if (kind === "keyboard") return Keyboard;
  if (kind === "mouse") return Mouse;
  if (kind === "headset" || kind === "microphone") return Headphones;
  return Usb;
}

function accentFor(kind: string) {
  if (kind === "keyboard") return "#ff4fa3";
  if (kind === "mouse") return "#a78bfa";
  return "#6ee7f9";
}

function open(id: string) {
  router.push(`/devices/${id}`);
}
</script>

<template>
  <div class="fixed top-[62px] right-5 z-[95] flex flex-col gap-2 w-[300px] pointer-events-none">
    <TransitionGroup name="notice">
      <button
        v-for="d in notices"
        :key="d.id"
        class="notice-card pointer-events-auto text-left relative overflow-hidden rounded-2xl border border-white/10 glass px-3.5 py-3 flex gap-3 items-center group"
        :style="{ '--accent': accentFor(d.kind) }"
        @click="open(d.id)"
      >
        <span class="notice-wave" aria-hidden="true"></span>
        <span class="notice-tile shrink-0 w-10 h-10 rounded-xl grid place-items-center relative">
          <span class="notice-ring" aria-hidden="true"></span>
          <component :is="iconFor(d.kind)" :size="17" class="relative" />
        </span>
        <span class="min-w-0 flex-1 relative">
          <span class="block text-[9.5px] uppercase tracking-[0.14em] text-accent/80 font-semibold">{{ t("notice.connected") }}</span>
          <span class="block text-[12.5px] font-semibold truncate mt-0.5">{{ d.name }}</span>
          <span class="block text-[10.5px] text-dim truncate">{{ deviceKindLabel(d.kind, locale) }} · {{ d.connectionLabel }}</span>
        </span>
        <ArrowUpRight :size="14" class="text-dim group-hover:text-text transition-colors shrink-0 relative" />
        <span class="notice-drain" aria-hidden="true"></span>
      </button>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.notice-card {
  min-height: 64px;
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.45), 0 0 0 1px color-mix(in srgb, var(--accent) 8%, transparent);
}

.notice-tile {
  color: var(--accent);
  background: linear-gradient(135deg, color-mix(in srgb, var(--accent) 20%, transparent), transparent);
  border: 1px solid color-mix(in srgb, var(--accent) 22%, transparent);
}

.notice-ring {
  position: absolute;
  inset: -1px;
  border-radius: 12px;
  border: 1px solid color-mix(in srgb, var(--accent) 45%, transparent);
  animation: notice-ping 1.9s var(--ease-out) infinite;
  pointer-events: none;
}

.notice-wave {
  position: absolute;
  width: 140px;
  height: 140px;
  left: -34px;
  top: 50%;
  margin-top: -70px;
  border-radius: 999px;
  background: radial-gradient(circle, color-mix(in srgb, var(--accent) 22%, transparent), transparent 68%);
  animation: notice-wave 1.6s var(--ease-out) both;
  pointer-events: none;
}

.notice-drain {
  position: absolute;
  left: 0;
  bottom: 0;
  height: 2px;
  width: 100%;
  background: linear-gradient(90deg, var(--accent), color-mix(in srgb, var(--accent) 30%, transparent));
  transform-origin: left;
  animation: notice-drain 3.7s linear forwards;
}

.notice-enter-active { transition: opacity 280ms var(--ease-out), transform 280ms var(--ease-spring); }
.notice-leave-active { transition: opacity 180ms ease, transform 180ms ease; position: absolute; right: 0; width: 100%; }
.notice-enter-from { opacity: 0; transform: translateX(26px) scale(0.98); }
.notice-leave-to { opacity: 0; transform: translateX(14px); }

@keyframes notice-ping {
  0% { opacity: 0.9; transform: scale(1); }
  70% { opacity: 0; transform: scale(1.25); }
  100% { opacity: 0; transform: scale(1.25); }
}

@keyframes notice-wave {
  0% { opacity: 0; transform: scale(0.4); }
  35% { opacity: 0.9; }
  100% { opacity: 0; transform: scale(1.5); }
}

@keyframes notice-drain {
  from { transform: scaleX(1); }
  to { transform: scaleX(0); }
}

@media (prefers-reduced-motion: reduce) {
  .notice-ring, .notice-wave { animation: none; opacity: 0; }
  .notice-drain { animation: none; }
}
</style>
