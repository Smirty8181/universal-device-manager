<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import {
  BatteryCharging,
  BatteryWarning,
  Bluetooth,
  ChevronRight,
  Cpu,
  Gamepad2,
  Headphones,
  Keyboard,
  Layers,
  LayoutDashboard,
  Monitor,
  Mouse,
  MousePointerClick,
  Palette,
  Power,
  RefreshCw,
  Settings,
  Usb,
  Wifi,
  X,
} from "lucide-vue-next";
import LogoMark from "../components/LogoMark.vue";
import { api, listen } from "../lib/api";
import { useI18n } from "../lib/i18n";
import type { DeviceView } from "../lib/types";
import { useAppStore } from "../stores/app";
import { useDevicesStore } from "../stores/devices";

const app = useAppStore();
const devices = useDevicesStore();
const { t } = useI18n();

const scanning = ref(false);
const scanError = ref<string | null>(null);
let offRefresh: (() => void) | null = null;
let offSettings: (() => void) | null = null;

const sorted = computed(() =>
  [...devices.devices].sort((a, b) => Number(b.online) - Number(a.online) || a.name.localeCompare(b.name))
);
const online = computed(() => devices.devices.filter((d) => d.online).length);
const wireless = computed(
  () => devices.devices.filter((d) => d.online && (d.connection === "wireless" || d.connection === "bluetooth")).length
);
const withBattery = computed(() => devices.devices.filter((d) => trustedPercent(d) !== null).length);
const lowDevices = computed(() =>
  devices.devices.filter((d) => {
    const p = trustedPercent(d);
    return d.online && p !== null && p <= app.settings.lowBatteryThreshold;
  })
);

// Only percentages backed by a real, charging or estimated source are shown.
function trustedPercent(device: DeviceView): number | null {
  const battery = device.battery;
  if (!battery || battery.percent === null) return null;
  if (battery.status === "real" || battery.status === "charging" || battery.status === "estimated") {
    return battery.percent;
  }
  return null;
}

function isLow(device: DeviceView) {
  const p = trustedPercent(device);
  return device.online && p !== null && p <= app.settings.lowBatteryThreshold;
}

function iconFor(kind: DeviceView["kind"]) {
  switch (kind) {
    case "keyboard":
      return Keyboard;
    case "mouse":
      return Mouse;
    case "headset":
    case "microphone":
      return Headphones;
    case "gamepad":
      return Gamepad2;
    default:
      return Usb;
  }
}

function accentFor(kind: DeviceView["kind"]) {
  switch (kind) {
    case "keyboard":
      return "#ff4fa3";
    case "mouse":
      return "#a78bfa";
    case "headset":
    case "microphone":
      return "#6ee7f9";
    case "gamepad":
      return "#5eead4";
    default:
      return "#a78bfa";
  }
}

function connectionIcon(device: DeviceView) {
  if (device.connection === "bluetooth") return Bluetooth;
  if (device.connection === "wireless") return Wifi;
  return Usb;
}

function hasVerifiedRgb(device: DeviceView) {
  return device.online && device.capabilities.some((c) => c.id === "rgb" && c.status === "working");
}

async function open(route: string) {
  await api.trayOpen(route);
}

async function close() {
  await api.trayHide();
}

async function quit() {
  await api.quitApp();
}

async function scan() {
  if (scanning.value) return;
  scanning.value = true;
  scanError.value = null;
  try {
    await devices.refresh(true);
    scanError.value = devices.error;
  } finally {
    scanning.value = false;
  }
}

onMounted(async () => {
  document.documentElement.classList.add("tray-mode");
  offRefresh = await listen("tray:refresh", () => {
    void devices.refreshPassive();
  });
  offSettings = await listen("settings:changed", async () => {
    await app.load(false);
  });
});

onBeforeUnmount(() => {
  document.documentElement.classList.remove("tray-mode");
  offRefresh?.();
  offSettings?.();
});
</script>

<template>
  <div class="h-full p-2 select-none">
    <section
      class="tray-card relative h-full flex flex-col overflow-hidden rounded-[18px] border border-white/[0.09] bg-[#0b0812]/95 backdrop-blur-xl shadow-[0_28px_70px_-18px_rgba(0,0,0,0.92)]"
    >
      <header
        data-tauri-drag-region
        class="flex items-center gap-2.5 px-3.5 h-[54px] shrink-0 border-b border-white/[0.055]"
      >
        <LogoMark :size="30" :interactive="false" />
        <div class="leading-tight min-w-0" data-tauri-drag-region>
          <div class="font-display font-semibold tracking-tight text-[13.5px]" data-tauri-drag-region>Universal Device Manager</div>
          <div class="flex items-center gap-1.5 text-[10px] text-dim" data-tauri-drag-region>
            <span
              class="w-1.5 h-1.5 rounded-full"
              :class="app.scanning ? 'bg-ok animate-pulse' : 'bg-white/25'"
            ></span>
            {{ app.scanning ? t("shell.scanActive") : t("tray.subtitle") }}
          </div>
        </div>
        <div class="ml-auto flex items-center gap-1">
          <button class="icon-btn !w-7 !h-7" :title="t('nav.settings')" @click="open('/settings')">
            <Settings :size="15" />
          </button>
          <button class="icon-btn !w-7 !h-7" :title="t('common.close')" @click="close">
            <X :size="15" />
          </button>
        </div>
      </header>

      <div class="grid grid-cols-3 gap-1.5 px-3 py-2.5 shrink-0">
        <div class="rounded-xl border border-white/[0.05] bg-white/[0.03] px-2.5 py-1.5">
          <div class="font-display text-[16px] leading-tight tabular-nums">{{ online }}</div>
          <div class="text-[9.5px] text-dim uppercase tracking-[0.08em]">{{ t("tray.online") }}</div>
        </div>
        <div class="rounded-xl border border-white/[0.05] bg-white/[0.03] px-2.5 py-1.5">
          <div class="font-display text-[16px] leading-tight tabular-nums">{{ wireless }}</div>
          <div class="text-[9.5px] text-dim uppercase tracking-[0.08em]">{{ t("tray.wireless") }}</div>
        </div>
        <div class="rounded-xl border border-white/[0.05] bg-white/[0.03] px-2.5 py-1.5">
          <div class="font-display text-[16px] leading-tight tabular-nums">{{ withBattery }}</div>
          <div class="text-[9.5px] text-dim uppercase tracking-[0.08em]">{{ t("tray.withBattery") }}</div>
        </div>
      </div>

      <div
        v-if="lowDevices.length"
        class="mx-3 mb-2 shrink-0 flex items-center gap-2 rounded-xl border border-bad/25 bg-bad/[0.08] px-2.5 py-1.5 text-[11px] text-bad"
      >
        <BatteryWarning :size="13" />
        <span class="font-semibold">{{ t("tray.lowBattery") }}</span>
        <span class="text-bad/80 truncate">{{ lowDevices.map((d) => d.name).join(", ") }}</span>
      </div>

      <div class="flex items-center gap-2 px-3.5 pb-1.5 shrink-0">
        <span class="text-[10px] uppercase tracking-[0.14em] text-dim">{{ t("tray.devices") }}</span>
        <span class="mono text-[10px] text-dim/70">{{ sorted.length }}</span>
        <button
          class="ml-auto flex items-center gap-1 text-[10.5px] text-dim hover:text-text transition-colors"
          :disabled="scanning"
          @click="scan"
        >
          <RefreshCw :size="12" :class="scanning ? 'animate-spin' : ''" />
          {{ scanning ? t("tray.scanning") : t("tray.scan") }}
        </button>
      </div>

      <div class="flex-1 min-h-0 overflow-y-auto scroll-region px-2 pb-2">
        <p v-if="scanError" class="mx-1.5 mb-1.5 rounded-lg border border-bad/20 bg-bad/[0.06] px-2 py-1.5 text-[10.5px] text-bad">
          {{ scanError }}
        </p>
        <div
          v-for="device in sorted"
          :key="device.id"
          role="button"
          tabindex="0"
          class="group w-full flex items-center gap-2.5 rounded-xl px-2 py-2 text-left transition-colors hover:bg-white/[0.045] focus-visible:bg-white/[0.045] cursor-pointer"
          @click="open(`/devices/${device.id}`)"
          @keydown.enter="open(`/devices/${device.id}`)"
        >
          <span
            class="grid place-items-center w-8 h-8 rounded-[10px] border shrink-0"
            :style="
              device.online
                ? { color: accentFor(device.kind), borderColor: accentFor(device.kind) + '33', background: accentFor(device.kind) + '12' }
                : undefined
            "
            :class="device.online ? '' : 'border-white/[0.06] bg-white/[0.03] text-dim'"
          >
            <component :is="iconFor(device.kind)" :size="15" />
          </span>
          <span class="min-w-0 flex-1">
            <span class="flex items-center gap-1.5">
              <span class="truncate text-[12px] font-medium" :class="device.online ? 'text-text' : 'text-dim'">{{ device.name }}</span>
              <span v-if="!device.online" class="shrink-0 text-[9px] text-dim/80">{{ t("tray.offline") }}</span>
            </span>
            <span class="flex items-center gap-1 text-[10px] text-dim truncate">
              <component :is="connectionIcon(device)" :size="10" class="shrink-0" />
              <span class="truncate">{{ device.connectionLabel }}</span>
              <template v-if="device.battery?.charging">
                <span class="text-warn/80">· {{ t("tray.charging") }}</span>
              </template>
            </span>
          </span>

          <span
            v-if="trustedPercent(device) !== null"
            class="shrink-0 flex items-center gap-1 mono text-[11.5px] tabular-nums"
            :class="isLow(device) ? 'text-bad' : 'text-text'"
          >
            <BatteryCharging v-if="device.battery?.charging" :size="12" class="text-warn" />
            <BatteryWarning v-else-if="isLow(device)" :size="12" />
            {{ trustedPercent(device) }}%
          </span>
          <span
            v-else
            class="shrink-0 text-[10px] text-dim/70"
            :title="t('tray.batteryUnavailable')"
          >—</span>

          <span class="shrink-0 flex items-center gap-0.5 opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 transition-opacity">
            <button
              v-if="hasVerifiedRgb(device)"
              class="icon-btn !w-6 !h-6"
              :title="t('tray.lighting')"
              @click.stop="open(`/devices/${device.id}?tab=lighting`)"
            >
              <Palette :size="12.5" />
            </button>
            <button
              v-if="device.online && device.battery"
              class="icon-btn !w-6 !h-6"
              :title="t('tray.power')"
              @click.stop="open(`/devices/${device.id}?tab=power`)"
            >
              <BatteryCharging :size="12.5" />
            </button>
            <ChevronRight :size="13" class="text-dim/60" />
          </span>
        </div>
        <p v-if="!sorted.length" class="px-3 py-8 text-center text-[11.5px] text-dim">{{ t("tray.noDevices") }}</p>
      </div>

      <footer class="shrink-0 border-t border-white/[0.055] px-2 pt-2 pb-1.5">
        <div class="grid grid-cols-3 gap-1">
          <button class="tray-action" @click="open('/')">
            <BatteryCharging :size="14" />
            <span>{{ t("tray.batteryMonitor") }}</span>
          </button>
          <button class="tray-action" @click="open('/intelligence')">
            <Cpu :size="14" />
            <span>{{ t("tray.intelligence") }}</span>
          </button>
          <button class="tray-action" @click="open('/visualizer')">
            <Monitor :size="14" />
            <span>{{ t("tray.visualizer") }}</span>
          </button>
          <button class="tray-action" @click="open('/autoclicker')">
            <MousePointerClick :size="14" />
            <span>{{ t("tray.autoclicker") }}</span>
          </button>
          <button class="tray-action" @click="open('/rgb')">
            <Palette :size="14" />
            <span>RGB Studio</span>
          </button>
          <button class="tray-action" @click="open('/')">
            <LayoutDashboard :size="14" />
            <span>{{ t("tray.openApp") }}</span>
          </button>
          <button class="tray-action" @click="open('/profiles')">
            <Layers :size="14" />
            <span>{{ t("nav.profiles") }}</span>
          </button>
          <button class="tray-action" @click="open('/devices')">
            <Usb :size="14" />
            <span>{{ t("tray.devices") }}</span>
          </button>
          <button class="tray-action" :disabled="scanning" @click="scan">
            <RefreshCw :size="14" :class="scanning ? 'animate-spin' : ''" />
            <span>{{ t("tray.scan") }}</span>
          </button>
          <button class="tray-action !text-bad/85 hover:!text-bad" @click="quit">
            <Power :size="14" />
            <span>{{ t("shell.quit") }}</span>
          </button>
        </div>
        <p class="text-center text-[9px] text-dim/60 pt-1.5">{{ t("tray.hint") }}</p>
      </footer>
    </section>
  </div>
</template>

<style scoped>
.tray-action {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 3px;
  padding: 6px 2px 5px;
  border-radius: 10px;
  font-size: 9.5px;
  line-height: 1.1;
  color: color-mix(in srgb, var(--color-text) 62%, transparent);
  border: 1px solid transparent;
  transition: background-color 140ms ease, color 140ms ease, border-color 140ms ease;
}

.tray-action:hover,
.tray-action:focus-visible {
  background: rgba(255, 255, 255, 0.04);
  border-color: rgba(255, 255, 255, 0.05);
  color: var(--color-text);
}

.tray-action:disabled {
  opacity: 0.55;
  cursor: default;
}
</style>