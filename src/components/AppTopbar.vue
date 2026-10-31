<script setup lang="ts">
import { computed } from "vue";
import { useRoute } from "vue-router";
import { Search, Settings2, Sparkles, Usb, Wifi } from "lucide-vue-next";
import { useI18n } from "../lib/i18n";
import { useAppStore } from "../stores/app";
import { useDevicesStore } from "../stores/devices";
import NotificationCenter from "./NotificationCenter.vue";

const route = useRoute();
const { t } = useI18n();
const app = useAppStore();
const devices = useDevicesStore();

const title = computed(() => {
  if (route.name === "device") {
    const id = String(route.params.id ?? "");
    return devices.getDevice(id)?.name ?? t("device.notFound");
  }
  if (route.name === "keyboard" || route.name === "mouse" || route.name === "audio") {
    return route.name === "keyboard" ? t("nav.keyboard") : route.name === "mouse" ? t("nav.mouse") : t("nav.audio");
  }
  const key = `nav.${String(route.name)}`;
  const label = t(key);
  return label === key ? t("nav.dashboard") : label;
});

const usbCount = computed(
  () => devices.devices.filter((d) => d.connection === "usb" || /usb/i.test(d.connectionLabel)).length
);
const wirelessCount = computed(
  () =>
    devices.devices.filter(
      (d) => d.connection === "wireless" || d.connection === "bluetooth" || /2\.4g|nearlink|bluetooth/i.test(d.connectionLabel)
    ).length
);
</script>

<template>
  <header class="h-[66px] shrink-0 flex items-center gap-4 px-6 border-b border-white/[0.055] bg-[#08060e]/55 backdrop-blur-xl relative z-30">
    <div class="min-w-0">
      <div class="text-[15px] font-semibold tracking-tight truncate">{{ title }}</div>
      <div class="text-[11px] text-dim truncate">{{ t("dashboard.subtitle") }}</div>
    </div>

    <div class="flex-1"></div>

    <div class="hidden xl:flex items-center gap-2 text-[11px] text-dim">
      <span class="chip">
        <Usb :size="12" />
        <span class="tabular-nums">{{ usbCount }}</span> {{ t("dashboard.usbConnected") }}
      </span>
      <span class="chip">
        <Wifi :size="12" />
        <span class="tabular-nums">{{ wirelessCount }}</span> {{ t("dashboard.wirelessConnected") }}
      </span>
      <span class="chip" :class="app.scanning ? 'chip-ok' : ''">
        <span class="status-dot live-dot" :class="app.scanning ? 'bg-ok' : 'bg-warn'"></span>
        {{ t("shell.scanIndicator") }}
      </span>
      <button
        v-if="app.ambientMode"
        class="chip chip-accent"
        :title="t('shell.ambientMode')"
        @click="app.toggleAmbient()"
      >
        <Sparkles :size="11" />
        <span class="w-2 h-2 rounded-full" :style="{ background: app.ambientPrimary, boxShadow: `0 0 8px ${app.ambientPrimary}` }"></span>
        Ambient
      </button>
      <span v-if="app.demo" class="chip chip-warn">{{ t("shell.demoModeTitle") }}</span>
    </div>

    <button
      class="control hidden md:flex items-center gap-2.5 w-[270px] !h-9 !px-3 text-left text-dim hover:text-muted group"
      :title="t('shell.paletteHint')"
      @click="app.openPalette()"
    >
      <Search :size="14" class="shrink-0 transition-transform group-hover:scale-110" />
      <span class="text-[12px] truncate flex-1">{{ t("shell.searchPlaceholder") }}</span>
      <span class="kbd !h-[20px] !text-[9.5px] text-dim whitespace-nowrap shrink-0">Ctrl K</span>
    </button>

    <button class="icon-btn md:hidden" :title="t('common.search')" @click="app.openPalette()">
      <Search :size="17" />
    </button>

    <NotificationCenter />

    <router-link to="/settings" class="icon-btn" :title="t('nav.settings')">
      <Settings2 :size="17" />
    </router-link>
  </header>
</template>
