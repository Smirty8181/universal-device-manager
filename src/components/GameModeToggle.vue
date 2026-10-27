<script setup lang="ts">
import { ref } from "vue";
import Toggle from "./Toggle.vue";
import { isTauri } from "../lib/api";
import { useI18n } from "../lib/i18n";
import { useAppStore } from "../stores/app";

const { t } = useI18n();
const app = useAppStore();
const busy = ref(false);

async function setGameMode(next: boolean) {
  if (busy.value) return;
  if (!isTauri()) {
    app.toast("info", t("keyboard.gameMode"), t("keyboard.gameModeDesktopOnly"));
    return;
  }
  busy.value = true;
  try {
    const saved = await app.patchSettings({ gameMode: next });
    if (saved) app.toast("success", t("keyboard.gameMode"), next ? t("keyboard.gameModeOn") : t("keyboard.gameModeOff"));
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <Toggle :model-value="app.settings.gameMode" :disabled="busy || !isTauri()" @update:model-value="setGameMode" />
</template>
