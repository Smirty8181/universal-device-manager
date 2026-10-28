<script setup lang="ts">
import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import { ArrowUpRight, Keyboard, Loader2, Palette, RefreshCw, X } from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import EmptyState from "../components/EmptyState.vue";
import LightingStudio from "../components/LightingStudio.vue";
import { api, isTauri } from "../lib/api";
import { useAppStore } from "../stores/app";
import { useDevicesStore } from "../stores/devices";
import { useI18n } from "../lib/i18n";

const { t } = useI18n();
const router = useRouter();
const app = useAppStore();
const devices = useDevicesStore();

const keyboards = computed(() =>
  devices.devices
    .filter((device) => {
      if (device.kind !== "keyboard") return false;
      const lighting = device.capabilities.find((cap) => cap.id === "lighting");
      return (
        device.adapterId === "redsquare" ||
        (device.modules?.find((m) => m.id === "rgb")?.status === "working") ||
        lighting?.status === "working" ||
        lighting?.status === "partial"
      );
    })
    .sort((a, b) => Number(b.online) - Number(a.online))
);

const selectedId = ref<string | null>(null);
const keyboard = computed(
  () => keyboards.value.find((item) => item.id === selectedId.value) ?? keyboards.value[0] ?? null
);
const anyKeyboard = computed(() => devices.devices.some((device) => device.kind === "keyboard"));

const studio = ref<{ lightingSnapshot: () => { effect: number; brightness: number; speed: number; direction: number; color: string } } | null>(null);
const syncing = ref(false);
const syncResults = ref<{ name: string; ok: boolean; message: string }[] | null>(null);

async function syncAll() {
  const source = studio.value?.lightingSnapshot();
  if (!source || syncing.value || !isTauri()) return;
  syncing.value = true;
  syncResults.value = null;
  const results: { name: string; ok: boolean; message: string }[] = [];
  const targets = keyboards.value.filter((item) => item.online);
  for (const target of targets) {
    if (target.id === keyboard.value?.id) {
      results.push({ name: target.name, ok: true, message: t("rgb.syncSource") });
      continue;
    }
    try {
      const result = await api.keyboardSetLighting(
        target.id,
        source.effect,
        source.brightness,
        source.speed,
        source.direction,
        source.color
      );
      results.push({
        name: target.name,
        ok: result.ok,
        message: result.ok ? t("rgb.syncOk") : result.message,
      });
    } catch (e) {
      results.push({ name: target.name, ok: false, message: String(e) });
    }
  }
  for (const skipped of keyboards.value.filter((item) => !item.online)) {
    results.push({ name: skipped.name, ok: false, message: t("rgb.syncOffline") });
  }
  syncResults.value = results;
  const failed = results.filter((item) => !item.ok).length;
  app.toast(
    failed ? "warn" : "success",
    t("rgb.syncAll"),
    failed ? t("rgb.syncPartial") : t("rgb.syncDone")
  );
  if (!failed) {
    app.addActivity("device", `${t("rgb.syncAll")}: ${results.length}`, "success");
  }
  syncing.value = false;
}
</script>

<template>
  <div>
    <PageHeader :title="t('rgb.title')" :subtitle="t('rgb.subtitle')" :overline="t('nav.rgb')">
      <template #actions>
        <span v-if="keyboard" class="chip !h-8">
          <span class="status-dot" :class="keyboard.online ? 'bg-ok' : 'bg-dim'"></span>
          {{ keyboard.name }}
        </span>
        <button
          v-if="keyboards.length > 1 && keyboard"
          class="btn"
          :disabled="syncing || !keyboard.online"
          :title="t('rgb.syncAllHint')"
          @click="syncAll"
        >
          <Loader2 v-if="syncing" :size="13" class="animate-spin" />
          <RefreshCw v-else :size="13" />
          {{ syncing ? t("rgb.syncing") : t("rgb.syncAll") }}
        </button>
        <button v-if="keyboard" class="btn" @click="router.push(`/devices/${keyboard.id}`)">
          <Keyboard :size="13" />
          {{ t("rgb.openDevice") }}
          <ArrowUpRight :size="12" />
        </button>
      </template>
    </PageHeader>

    <div v-if="!keyboard" class="panel p-2">
      <EmptyState
        :icon="Palette"
        :title="anyKeyboard ? t('rgb.noVerified') : t('rgb.noDevices')"
        :body="anyKeyboard ? t('rgb.noVerifiedBody') : t('rgb.noDevicesBody')"
      >
        <button class="btn" @click="router.push('/devices')">{{ t("nav.devices") }}</button>
      </EmptyState>
    </div>

    <template v-else>
      <div v-if="keyboards.length > 1" class="flex items-center gap-1.5 mb-4 overflow-x-auto scroll-region pb-1">
        <span class="text-[10.5px] text-dim mr-1 shrink-0">{{ t("rgb.applyTo") }}</span>
        <button
          v-for="item in keyboards"
          :key="item.id"
          class="chip !h-7 shrink-0 transition-colors"
          :class="item.id === keyboard.id ? 'chip-ok' : 'hover:text-text'"
          @click="selectedId = item.id"
        >
          <span class="status-dot" :class="item.online ? 'bg-ok' : 'bg-dim'"></span>
          {{ item.name }}
        </button>
      </div>

      <div
        v-if="syncResults"
        class="panel px-4 py-3 mb-4 text-[11.5px] space-y-1.5"
      >
        <div class="flex items-center gap-2 mb-1">
          <span class="section-title !mb-0">{{ t("rgb.syncResults") }}</span>
          <button class="ml-auto icon-btn !w-6 !h-6" @click="syncResults = null"><X :size="13" /></button>
        </div>
        <div v-for="item in syncResults" :key="item.name" class="flex items-start gap-2">
          <span class="w-1.5 h-1.5 rounded-full mt-1.5 shrink-0" :class="item.ok ? 'bg-ok' : 'bg-warn'"></span>
          <span class="font-medium shrink-0">{{ item.name }}</span>
          <span class="text-dim">{{ item.message }}</span>
        </div>
      </div>

      <LightingStudio ref="studio" :key="keyboard.id" :device="keyboard" />
    </template>
  </div>
</template>