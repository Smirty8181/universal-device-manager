<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute } from "vue-router";
import {
  Activity,
  ChevronsLeft,
  ChevronsRight,
  Command,
  Cpu,
  Database,
  HardDrive,
  Headphones,
  Keyboard,
  Layers,
  LayoutDashboard,
  Monitor,
  Mouse,
  MousePointerClick,
  Palette,
  Settings,
  SlidersHorizontal,
  Sparkles,
  Usb,
  type LucideIcon,
} from "lucide-vue-next";
import LogoMark from "./LogoMark.vue";
import { useI18n } from "../lib/i18n";
import { useAppStore } from "../stores/app";
import { useDevicesStore } from "../stores/devices";

const route = useRoute();
const { t } = useI18n();
const app = useAppStore();
const devices = useDevicesStore();

const items = computed<{ name: string; path: string; icon: LucideIcon; label: string; divider?: boolean }[]>(() => [
  { name: "dashboard", path: "/", icon: LayoutDashboard, label: t("nav.dashboard") },
  { name: "devices", path: "/devices", icon: Usb, label: t("nav.devices") },
  { name: "intelligence", path: "/intelligence", icon: Cpu, label: t("nav.intelligence") },
  { name: "rgb", path: "/rgb", icon: Palette, label: t("nav.rgb"), divider: true },
  { name: "keyboard", path: "/keyboard", icon: Keyboard, label: t("nav.keyboard") },
  { name: "visualizer", path: "/visualizer", icon: Monitor, label: t("nav.visualizer") },
  { name: "autoclicker", path: "/autoclicker", icon: MousePointerClick, label: t("nav.autoclicker") },
  { name: "mouse", path: "/mouse", icon: Mouse, label: t("nav.mouse") },
  { name: "audio", path: "/audio", icon: Headphones, label: t("nav.audio"), divider: true },
  { name: "input", path: "/input", icon: SlidersHorizontal, label: t("nav.input") },
  { name: "profiles", path: "/profiles", icon: Layers, label: t("nav.profiles"), divider: true },
  { name: "macros", path: "/macros", icon: Command, label: t("nav.macros") },
  { name: "database", path: "/database", icon: Database, label: t("nav.database"), divider: true },
  { name: "drivers", path: "/drivers", icon: HardDrive, label: t("nav.drivers") },
  { name: "diagnostics", path: "/diagnostics", icon: Activity, label: t("nav.diagnostics") },
  { name: "settings", path: "/settings", icon: Settings, label: t("nav.settings") },
]);

function isActive(path: string) {
  if (path === "/") return route.path === "/";
  if (path === "/devices") return route.path.startsWith("/devices");
  return route.path === path;
}

const online = computed(() => devices.onlineCount);
const total = computed(() => devices.devices.length);

const navRef = ref<HTMLElement | null>(null);
const indicator = ref({ top: 0, height: 24, visible: false });

function placeIndicator() {
  const active = navRef.value?.querySelector<HTMLElement>(".nav-item.active");
  if (!active) {
    indicator.value = { ...indicator.value, visible: false };
    return;
  }
  const parentTop = navRef.value!.getBoundingClientRect().top;
  const box = active.getBoundingClientRect();
  indicator.value = { top: box.top - parentTop, height: box.height, visible: true };
}

watch(
  () => [route.path, app.sidebarCollapsed, items.value.length],
  async () => {
    await nextTick();
    placeIndicator();
  },
  { immediate: true }
);

let observer: ResizeObserver | null = null;
onMounted(() => {
  placeIndicator();
  if (navRef.value) {
    observer = new ResizeObserver(() => placeIndicator());
    observer.observe(navRef.value);
  }
  window.addEventListener("resize", placeIndicator);
});
onBeforeUnmount(() => {
  observer?.disconnect();
  window.removeEventListener("resize", placeIndicator);
});
</script>

<template>
  <aside
    class="shrink-0 h-full flex flex-col border-r border-white/[0.055] bg-[#08060e]/75 relative z-20 transition-[width] duration-[240ms]"
    :class="app.sidebarCollapsed ? 'w-[78px]' : 'w-[244px]'"
    style="transition-timing-function: var(--ease-out)"
  >
    <div class="flex items-center h-[70px] shrink-0" :class="app.sidebarCollapsed ? 'justify-center px-3' : 'px-5 gap-3'">
      <LogoMark :size="36" @activate="app.registerLogoClick()" />
      <transition name="fade">
        <div v-if="!app.sidebarCollapsed" class="leading-tight min-w-0">
          <div class="font-display font-semibold tracking-tight text-[15px] truncate">Universal</div>
          <div class="text-[10.5px] text-dim tracking-[0.16em] uppercase">Device Manager</div>
        </div>
      </transition>
      <button
        v-if="!app.sidebarCollapsed"
        class="icon-btn !w-7 !h-7 ml-auto"
        :title="t('shell.collapse')"
        @click="app.toggleSidebar()"
      >
        <ChevronsLeft :size="15" />
      </button>
    </div>

    <nav ref="navRef" class="relative px-3 mt-1 overflow-y-auto scroll-region flex-1" :class="app.sidebarCollapsed ? 'space-y-1' : 'space-y-0.5'">
      <span
        class="nav-indicator absolute left-3 right-3 rounded-xl pointer-events-none"
        :class="indicator.visible ? 'opacity-100' : 'opacity-0'"
        :style="{
          transform: `translateY(${indicator.top}px)`,
          height: indicator.height + 'px',
        }"
      ></span>

      <div v-for="item in items" :key="item.name">
        <div v-if="item.divider" class="mx-2 my-2 h-px bg-white/[0.05]"></div>
        <router-link
          :to="item.path"
          class="nav-item relative z-10"
          :class="{ active: isActive(item.path), 'justify-center !px-0': app.sidebarCollapsed, 'tip': app.sidebarCollapsed }"
          :data-tip="app.sidebarCollapsed ? item.label : undefined"
          :aria-current="isActive(item.path) ? 'page' : undefined"
        >
          <component :is="item.icon" :size="17" :stroke-width="1.9" class="shrink-0" />
          <transition name="fade">
            <span v-if="!app.sidebarCollapsed" class="truncate">{{ item.label }}</span>
          </transition>
        </router-link>
      </div>
    </nav>

    <div class="mt-auto px-3 pb-4">
      <button
        v-if="app.sidebarCollapsed"
        class="nav-item w-full justify-center !px-0 mb-2 tip"
        :data-tip="t('shell.expand')"
        @click="app.toggleSidebar()"
      >
        <ChevronsRight :size="17" />
      </button>

      <div
        class="rounded-2xl border border-white/[0.06] bg-white/[0.025] overflow-hidden transition-all duration-200"
        :class="app.sidebarCollapsed ? 'p-2' : 'p-3.5'"
      >
        <template v-if="!app.sidebarCollapsed">
          <div class="flex items-center justify-between">
            <span class="text-[10px] text-dim uppercase tracking-[0.14em]">{{ t("dashboard.detection") }}</span>
            <span class="flex items-center gap-1.5 text-[10.5px]" :class="app.scanning ? 'text-ok' : 'text-warn'">
              <span class="status-dot live-dot" :class="app.scanning ? 'bg-ok' : 'bg-warn'"></span>
              {{ app.scanning ? t("shell.scanActive") : t("shell.offline") }}
            </span>
          </div>
          <div class="mt-2 flex items-end justify-between">
            <div class="text-[22px] font-display font-semibold leading-none tabular-nums">
              {{ online }}<span class="text-dim text-[12px] font-normal"> / {{ total }}</span>
            </div>
            <div class="text-[10.5px] text-dim pb-0.5">{{ t("shell.devicesOnline") }}</div>
          </div>
          <div class="mt-3 flex gap-1 h-[3px] rounded-full overflow-hidden">
            <div
              v-for="d in devices.devices"
              :key="d.id"
              class="flex-1 rounded-full transition-colors duration-300"
              :class="d.online ? 'bg-gradient-to-r from-accent to-accent-2' : 'bg-white/10'"
            ></div>
          </div>
          <button
            class="mt-3 w-full h-[30px] rounded-lg border border-white/[0.07] flex items-center justify-center gap-1.5 text-[10.5px] font-medium transition-colors"
            :class="app.ambientMode ? 'text-accent border-accent/30 bg-accent/[0.06]' : 'text-dim hover:text-text hover:border-white/15'"
            @click="app.toggleAmbient()"
          >
            <Sparkles :size="12" />
            {{ t("shell.ambientMode") }}
            <span class="ml-1 w-1.5 h-1.5 rounded-full" :class="app.ambientMode ? 'bg-accent' : 'bg-white/20'"></span>
          </button>
        </template>

        <template v-else>
          <div class="flex flex-col items-center gap-1.5 tip" :data-tip="`${online} / ${total} ${t('shell.devicesOnline')}`">
            <span class="status-dot live-dot" :class="app.scanning ? 'bg-ok' : 'bg-warn'"></span>
            <span class="text-[12px] font-display font-semibold tabular-nums">{{ online }}</span>
          </div>
          <button
            class="mt-2 w-full h-7 rounded-lg grid place-items-center tip"
            :class="app.ambientMode ? 'text-accent' : 'text-dim'"
            :data-tip="t('shell.ambientMode')"
            @click="app.toggleAmbient()"
          >
            <Sparkles :size="13" />
          </button>
        </template>
      </div>

      <div class="flex items-center justify-between px-1 pt-3 text-[10px] text-dim" :class="app.sidebarCollapsed ? 'flex-col gap-1' : ''">
        <span class="mono">v0.1.0</span>
        <span v-if="!app.sidebarCollapsed">{{ t("shell.localAccount") }}</span>
        <span v-else class="w-1 h-1 rounded-full bg-ok/70"></span>
      </div>
    </div>
  </aside>
</template>

<style scoped>
.nav-indicator {
  top: 0;
  background: linear-gradient(90deg, color-mix(in srgb, var(--ambient) 16%, transparent), rgba(255, 255, 255, 0.025));
  border: 1px solid color-mix(in srgb, var(--ambient) 20%, transparent);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.04);
  transition: transform 280ms var(--ease-out), height 200ms var(--ease-out), opacity 200ms ease;
}
.nav-indicator::before {
  content: "";
  position: absolute;
  left: 0;
  top: 9px;
  bottom: 9px;
  width: 3px;
  border-radius: 4px;
  background: linear-gradient(180deg, var(--ambient), var(--ambient-2));
  box-shadow: 0 0 12px color-mix(in srgb, var(--ambient) 60%, transparent);
}
</style>
