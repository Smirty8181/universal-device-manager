import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { api, isTauri } from "../lib/api";
import { t } from "../lib/i18n";
import type { InputMap, InputStatus, MouseSystem, PollingStat } from "../lib/types";
import { useAppStore } from "./app";

const fallbackStatus: InputStatus = {
  engineActive: false,
  remapping: false,
  seenKeys: 0,
  swallowedKeys: 0,
  seenButtons: 0,
  swallowedButtons: 0,
  recent: [],
  map: { enabled: false, keys: [], buttons: [] },
  system: { pointerSpeed: 10, acceleration: true, wheelLines: 3, swapButtons: false },
};

export const useInputStore = defineStore("input", () => {
  const status = ref<InputStatus>({ ...fallbackStatus });
  const catalog = ref<{ vk: number; name: string }[]>([]);
  const stats = ref<PollingStat[]>([]);
  const busy = ref(false);
  const loaded = ref(false);
  const app = useAppStore();

  const available = computed(() => isTauri());
  const keyRules = computed(() => status.value.map.keys);
  const buttonRules = computed(() => status.value.map.buttons);

  function fail(e: unknown) {
    app.toast("error", t("toast.error"), e instanceof Error ? e.message : String(e));
  }

  async function refresh() {
    if (!available.value) return;
    try {
      status.value = await api.inputStatus();
      loaded.value = true;
    } catch (e) {
      fail(e);
    }
  }

  async function loadCatalog() {
    if (catalog.value.length || !available.value) return;
    try {
      const list = await api.keyCatalog();
      catalog.value = list
        .map(([vk, name]) => ({ vk, name }))
        .sort((a, b) => a.name.localeCompare(b.name) || a.vk - b.vk);
    } catch (e) {
      fail(e);
    }
  }

  async function commit(next: InputMap): Promise<boolean> {
    if (!available.value) return false;
    busy.value = true;
    try {
      status.value = await api.inputMapSet(next);
      app.toast("success", t("toast.inputSaved"));
      return true;
    } catch (e) {
      fail(e);
      return false;
    } finally {
      busy.value = false;
    }
  }

  async function setEnabled(enabled: boolean) {
    if (!available.value) return;
    busy.value = true;
    try {
      status.value = await api.inputEnabledSet(enabled);
    } catch (e) {
      fail(e);
    } finally {
      busy.value = false;
    }
  }

  async function applySystem(next: MouseSystem): Promise<boolean> {
    if (!available.value) return false;
    busy.value = true;
    try {
      const readBack = await api.mouseSystemSet(next);
      status.value = { ...status.value, system: readBack };
      // Windows can refuse a live pointer-speed change, so the read-back decides the message
      if (readBack.pointerSpeed !== next.pointerSpeed) {
        app.toast("warn", t("toast.mouseSpeedBlocked"));
        return false;
      }
      app.toast("success", t("toast.mouseSystemApplied"));
      return true;
    } catch (e) {
      fail(e);
      return false;
    } finally {
      busy.value = false;
    }
  }

  async function pollStats(windowMs = 1000) {
    if (!available.value) return;
    try {
      stats.value = await api.pollingStats(windowMs);
    } catch (e) {
      fail(e);
    }
  }

  async function resetStats() {
    if (!available.value) return;
    try {
      await api.pollingReset();
      stats.value = [];
    } catch (e) {
      fail(e);
    }
  }

  return {
    status,
    catalog,
    stats,
    busy,
    loaded,
    available,
    keyRules,
    buttonRules,
    refresh,
    loadCatalog,
    commit,
    setEnabled,
    applySystem,
    pollStats,
    resetStats,
  };
});
