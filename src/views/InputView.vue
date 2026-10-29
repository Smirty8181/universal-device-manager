<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { Gamepad2, Headphones, Keyboard, Mouse } from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import Tabs from "../components/Tabs.vue";
import KeymapPanel from "../components/panels/KeymapPanel.vue";
import MouseTuningPanel from "../components/panels/MouseTuningPanel.vue";
import AppMixerPanel from "../components/panels/AppMixerPanel.vue";
import GameModeToggle from "../components/GameModeToggle.vue";
import { useI18n } from "../lib/i18n";
import { useInputStore } from "../stores/input";
import { useDevicesStore } from "../stores/devices";

type Tab = "keyboard" | "mouse" | "audio";

const { t } = useI18n();
const input = useInputStore();
const devices = useDevicesStore();
const tab = ref<Tab>("keyboard");

const tabs = computed(() => [
  { id: "keyboard" as const, label: t("input.keyboard"), count: input.keyRules.length },
  { id: "mouse" as const, label: t("input.mouse"), count: input.buttonRules.length },
  { id: "audio" as const, label: t("nav.audio") },
]);

const detected = computed(() => {
  const keyboards = devices.devices.filter((d) => d.kind === "keyboard").length;
  const mice = devices.devices.filter((d) => d.kind === "mouse").length;
  const audio = devices.devices.filter((d) => d.kind === "headset").length;
  return { keyboards, mice, audio };
});

onMounted(() => {
  input.refresh();
  input.loadCatalog();
});
</script>

<template>
  <div>
    <PageHeader
      :title="t('nav.input')"
      :subtitle="t('input.osLevelHint')"
      :overline="t('studio.overline')"
    >
      <template #actions>
        <span class="chip" :class="input.status.engineActive ? 'chip-ok' : 'chip-bad'">
          <span class="status-dot" :class="input.status.engineActive ? 'bg-ok' : 'bg-bad'"></span>
          {{ input.status.engineActive ? t("input.engineOn") : t("input.engineOff") }}
        </span>
      </template>
    </PageHeader>

    <div class="flex items-center gap-2 mb-4 flex-wrap text-[11px] text-dim">
      <span class="chip"><Keyboard :size="11" /> {{ detected.keyboards }} {{ t("input.keyboard") }}</span>
      <span class="chip"><Mouse :size="11" /> {{ detected.mice }} {{ t("input.mouse") }}</span>
      <span class="chip"><Headphones :size="11" /> {{ detected.audio }} {{ t("nav.audio") }}</span>
      <span class="ml-auto">{{ t("input.ruleNote1") }}</span>
    </div>

    <div class="card px-4 py-3 mb-4 flex items-center gap-3">
      <span class="w-8 h-8 rounded-lg grid place-items-center bg-accent/[0.08] border border-accent/15 shrink-0">
        <Gamepad2 :size="15" class="text-accent" />
      </span>
      <div class="min-w-0">
        <div class="text-[12px] font-medium">{{ t("keyboard.gameMode") }}</div>
        <div class="text-[10.5px] text-dim leading-relaxed">{{ t("keyboard.gameModeNote") }}</div>
      </div>
      <div class="ml-auto pl-3 shrink-0">
        <GameModeToggle />
      </div>
    </div>

    <Tabs v-model="tab" :tabs="tabs" />

    <KeymapPanel v-if="tab === 'keyboard'" kind="keyboard" />
    <template v-else-if="tab === 'mouse'">
      <MouseTuningPanel />
      <div class="mt-4"><KeymapPanel kind="mouse" /></div>
    </template>
    <AppMixerPanel v-else />
  </div>
</template>
