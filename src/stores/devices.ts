import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { api, listen } from "../lib/api";
import { t } from "../lib/i18n";
import type { BatteryInfo, BatterySnapshot, DeviceView } from "../lib/types";
import { useAppStore } from "./app";

export const useDevicesStore = defineStore("devices", () => {
  const devices = ref<DeviceView[]>([]);
  const loading = ref(true);
  const error = ref<string | null>(null);
  const batteryHistory = ref<BatterySnapshot[]>([]);
  const lastScanAt = ref<number | null>(null);
  const recentlyConnected = ref<Set<string>>(new Set());
  const unlisteners: (() => void)[] = [];
  let passiveMode = false;

  const app = useAppStore();

  const onlineCount = computed(() => devices.value.filter((d) => d.online).length);
  const byId = computed(() => {
    const map = new Map<string, DeviceView>();
    for (const d of devices.value) map.set(d.id, d);
    return map;
  });

  function getDevice(id: string) {
    return byId.value.get(id);
  }

  function markConnected(id: string) {
    recentlyConnected.value = new Set(recentlyConnected.value).add(id);
    window.setTimeout(() => {
      const updated = new Set(recentlyConnected.value);
      updated.delete(id);
      recentlyConnected.value = updated;
    }, 3800);
  }

  function diffAndLog(prev: DeviceView[], next: DeviceView[]) {
    const prevById = new Map(prev.map((d) => [d.id, d]));
    const nextIds = new Set(next.map((d) => d.id));
    for (const d of next) {
      const before = prevById.get(d.id);
      if (!before) {
        app.addActivity("device", `${t("dashboard.events.deviceFound")}: ${d.name}`, "success", d.id);
        if (prev.length > 0) markConnected(d.id);
        continue;
      }
      if (!before.online && d.online) {
        app.addActivity("device", `${t("dashboard.events.deviceFound")}: ${d.name}`, "success", d.id);
        markConnected(d.id);
        if (app.settings.notificationsEnabled) {
          app.toast("success", t("toast.deviceConnected"), d.name);
        }
      } else if (before.online && !d.online) {
        app.addActivity("device", `${t("dashboard.events.deviceLost")}: ${d.name}`, "warn", d.id);
        if (app.settings.notificationsEnabled) {
          app.toast("warn", t("toast.deviceDisconnected"), d.name);
        }
      }
    }
    for (const d of prev) {
      if (!nextIds.has(d.id)) {
        app.addActivity("device", `${t("dashboard.events.deviceLost")}: ${d.name}`, "warn", d.id);
        if (app.settings.notificationsEnabled) {
          app.toast("warn", t("toast.deviceDisconnected"), d.name);
        }
      }
    }
  }

  async function refresh(silent = false) {
    if (!silent) loading.value = true;
    try {
      const next = await api.rescanDevices();
      const prev = devices.value;
      diffAndLog(prev, next);
      devices.value = next;
      lastScanAt.value = Date.now();
      error.value = null;
      app.scanning = app.settings.autoScan;
    } catch (e) {
      error.value = String(e);
      app.scanning = false;
      if (!silent) app.toast("error", t("toast.error"), String(e));
    } finally {
      loading.value = false;
    }
  }

  async function loadBatteryHistory() {
    try {
      batteryHistory.value = await api.batteryHistory();
    } catch {
      batteryHistory.value = [];
    }
  }

  // Reads the backend cache without triggering a PnP rescan; used by the tray window.
  async function refreshPassive() {
    try {
      const next = await api.listDevices();
      devices.value = next;
      lastScanAt.value = Date.now();
      error.value = null;
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function probeBattery(deviceId: string) {
    try {
      const battery = await api.probeBattery(deviceId);
      if (battery) {
        const d = getDevice(deviceId);
        if (d) d.battery = battery;
        if (battery.percent !== null) {
          checkLowBattery(d, battery);
        }
      }
      return battery;
    } catch (e) {
      app.toast("error", t("toast.error"), String(e));
      return null;
    }
  }

  function checkLowBattery(device: DeviceView | undefined, battery: BatteryInfo) {
    if (!device || battery.percent === null) return;
    if (passiveMode) return;
    if (battery.percent <= app.settings.lowBatteryThreshold) {
      app.addActivity("battery", `${t("toast.batteryLow")}: ${device.name} — ${battery.percent}%`, "warn", device.id);
      if (app.settings.notificationsEnabled) {
        app.toast("warn", t("toast.batteryLow"), `${device.name}: ${battery.percent}%`);
      }
    }
  }

  function historyFor(deviceId: string) {
    return batteryHistory.value.filter((h) => h.deviceId === deviceId).sort((a, b) => a.at - b.at);
  }

  async function init(passive = false) {
    passiveMode = passive;
    if (passive) {
      await refreshPassive();
    } else {
      await refresh();
      await loadBatteryHistory();
    }
    unlisteners.push(
      await listen<DeviceView[]>("devices:update", (payload) => {
        const prev = devices.value;
        if (!passiveMode) diffAndLog(prev, payload);
        devices.value = payload;
        lastScanAt.value = Date.now();
      })
    );
    unlisteners.push(
      await listen<{ deviceId: string; battery: BatteryInfo }>("battery:update", (payload) => {
        const d = getDevice(payload.deviceId);
        if (d) {
          d.battery = payload.battery;
          checkLowBattery(d, payload.battery);
        }
      })
    );
    if (!passive) {
      unlisteners.push(
        await listen<string>("toast", (message) => {
          if (app.settings.notificationsEnabled) app.toast("info", message);
        })
      );
    }
  }

  function dispose() {
    for (const off of unlisteners.splice(0)) off();
  }

  return {
    devices,
    loading,
    error,
    onlineCount,
    lastScanAt,
    batteryHistory,
    recentlyConnected,
    refresh,
    refreshPassive,
    init,
    dispose,
    getDevice,
    probeBattery,
    loadBatteryHistory,
    historyFor,
  };
});
