<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute } from "vue-router";
import AppSidebar from "./components/AppSidebar.vue";
import AppTopbar from "./components/AppTopbar.vue";
import BootScreen from "./components/BootScreen.vue";
import CommandPalette from "./components/CommandPalette.vue";
import ConnectNotice from "./components/ConnectNotice.vue";
import Toasts from "./components/Toasts.vue";
import { useAppStore } from "./stores/app";
import { useDevicesStore } from "./stores/devices";
import { useProfilesStore } from "./stores/profiles";
import { useMacrosStore } from "./stores/macros";
import { useRgbStore } from "./stores/rgb";
import { useI18n } from "./lib/i18n";
import { listen } from "./lib/api";
import router from "./router";
import { Info } from "lucide-vue-next";

const app = useAppStore();
const devices = useDevicesStore();
const profiles = useProfilesStore();
const macros = useMacrosStore();
const rgb = useRgbStore();
const route = useRoute();
const { t } = useI18n();
const isTray = computed(() => route.path === "/tray");
const isOverlay = computed(() => route.path === "/overlay");
const trayMode = window.location.hash.startsWith("#/tray");
const overlayMode = window.location.hash.startsWith("#/overlay");

const demoHidden = ref(sessionStorage.getItem("udm.demoHidden") === "1");
const mainScroll = ref<HTMLElement | null>(null);

const booting = ref(!trayMode && !overlayMode);
const bootLeaving = ref(false);
const bootProgress = ref(10);
const bootStepKey = ref("boot.stepSettings");
const bootStep = computed(() => t(bootStepKey.value));
const wait = (ms: number) => new Promise<void>((resolve) => window.setTimeout(resolve, ms));

async function runBoot(startedAt: number) {
  bootProgress.value = 14;
  await app.load(true);

  bootProgress.value = 36;
  bootStepKey.value = "boot.stepDevices";
  await devices.init();

  bootProgress.value = 62;
  bootStepKey.value = "boot.stepProfiles";
  await Promise.allSettled([profiles.init(), macros.init()]);

  bootProgress.value = 82;
  bootStepKey.value = "boot.stepSystem";
  await app.loadSystemInfo();

  bootProgress.value = 100;
  bootStepKey.value = "boot.stepReady";
  const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const minimum = reduce ? 450 : 1500;
  await wait(Math.max(0, minimum - (performance.now() - startedAt)));
  if (!reduce) await wait(200);
  bootLeaving.value = true;
  await wait(reduce ? 20 : 420);
  booting.value = false;
}

function hideDemo() {
  demoHidden.value = true;
  sessionStorage.setItem("udm.demoHidden", "1");
}

watch(
  () => route.fullPath,
  () => mainScroll.value?.scrollTo({ top: 0 })
);

watch(
  () => [rgb.primary, rgb.secondary, app.ambientMode] as const,
  ([primary, secondary]) => {
    app.setAmbient(primary, secondary);
  }
);

let offNavigate: (() => void) | null = null;

onMounted(async () => {
  const startedAt = performance.now();
  if (overlayMode) return;
  if (!trayMode) {
    offNavigate = await listen<string>("udm:navigate", (path) => {
      if (path) void router.push(path);
    });
  }
  if (trayMode) {
    await app.load(false);
    await devices.init(true);
    app.scanning = app.settings.autoScan;
    return;
  }
  await runBoot(startedAt);
});

onBeforeUnmount(() => {
  devices.dispose();
  offNavigate?.();
});
</script>

<template>
  <router-view v-if="isTray || isOverlay" />
  <div v-else class="h-full flex relative z-10">
    <AppSidebar />
    <div class="flex-1 flex flex-col min-w-0">
      <AppTopbar />

      <div v-if="app.demo && !demoHidden" class="px-7 py-2 flex items-center gap-2.5 text-[11.5px] text-warn/90 bg-warn/[0.05] border-b border-warn/10">
        <Info :size="13" />
        <span class="font-semibold">{{ t("shell.demoModeTitle") }}</span>
        <span class="text-muted">{{ t("shell.demoModeBody") }}</span>
        <button class="ml-auto text-dim hover:text-text" @click="hideDemo">×</button>
      </div>

      <main ref="mainScroll" class="flex-1 overflow-y-auto scroll-region px-7 pb-12">
        <div class="mx-auto max-w-[1280px] py-6">
          <router-view v-slot="{ Component }">
            <transition name="page" mode="out-in">
              <component :is="Component" />
            </transition>
          </router-view>
        </div>
      </main>
    </div>
    <Toasts />
    <ConnectNotice />
    <CommandPalette />
    <BootScreen v-if="booting" :progress="bootProgress" :step="bootStep" :leaving="bootLeaving" />
  </div>
</template>
