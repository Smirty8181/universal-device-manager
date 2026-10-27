import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { api, defaultSettings, isTauri } from "../lib/api";
import { locale, setLocale, t } from "../lib/i18n";
import type { AppSettings, SystemInfo, TosatMessage } from "../lib/types";

export interface ActivityEvent {
  id: string;
  at: number;
  kind: "device" | "scan" | "profile" | "settings" | "macro" | "battery" | "app";
  text: string;
  deviceId?: string;
  severity: "info" | "success" | "warn" | "error";
}

const ACTIVITY_KEY = "udm.activity";
const UI_KEY = "udm.ui";

function loadUi() {
  try {
    const raw = localStorage.getItem(UI_KEY);
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}

const accentThemes: Record<string, [string, string]> = {
  pink: ["#ff4fa3", "#a78bfa"],
  violet: ["#a78bfa", "#6ee7f9"],
  cyan: ["#6ee7f9", "#a78bfa"],
  aurora: ["#5eead4", "#a78bfa"],
};

function loadActivity(): ActivityEvent[] {
  try {
    const raw = localStorage.getItem(ACTIVITY_KEY);
    return raw ? (JSON.parse(raw) as ActivityEvent[]) : [];
  } catch {
    return [];
  }
}

export const useAppStore = defineStore("app", () => {
  const ui = loadUi();
  const settings = ref<AppSettings>(defaultSettings());
  const loaded = ref(false);
  const demo = ref(!isTauri());
  const scanning = ref(true);
  const systemInfo = ref<SystemInfo | null>(null);
  const toasts = ref<TosatMessage[]>([]);
  const activity = ref<ActivityEvent[]>(loadActivity());
  const activityOpen = ref(false);
  const searchQuery = ref("");
  const nowTick = ref(Date.now());
  const sidebarCollapsed = ref<boolean>(Boolean(ui.sidebarCollapsed));
  const ambientMode = ref<boolean>(Boolean(ui.ambientMode));
  const ambientPrimary = ref<string>(ui.ambientPrimary ?? "#ff4fa3");
  const ambientSecondary = ref<string>(ui.ambientSecondary ?? "#a78bfa");
  const secretTheme = ref<boolean>(Boolean(ui.secretTheme));
  const paletteOpen = ref(false);
  let logoClicks = 0;
  let logoClickTimer: number | undefined;
  let toastSeq = 1;

  setInterval(() => {
    nowTick.value = Date.now();
  }, 15000);

  const unreadCount = computed(() => Math.min(activity.value.length, 9));

  function addActivity(kind: ActivityEvent["kind"], text: string, severity: ActivityEvent["severity"] = "info", deviceId?: string) {
    activity.value.unshift({
      id: `${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
      at: Date.now(),
      kind,
      text,
      severity,
      deviceId,
    });
    activity.value = activity.value.slice(0, 80);
    try {
      localStorage.setItem(ACTIVITY_KEY, JSON.stringify(activity.value));
    } catch {}
  }

  function toast(kind: TosatMessage["kind"], title: string, body?: string) {
    const id = toastSeq++;
    toasts.value.push({ id, kind, title, body });
    setTimeout(() => dismissToast(id), kind === "error" ? 6500 : 4200);
  }

  function dismissToast(id: number) {
    toasts.value = toasts.value.filter((x) => x.id !== id);
  }

  async function load(trackStart = true) {
    try {
      settings.value = await api.settingsGet();
    } catch (e) {
      console.error(e);
    }
    applyAppearance();
    setLocale(settings.value.language);
    loaded.value = true;
    if (trackStart) addActivity("app", t("dashboard.events.started"), "info");
  }

  function applyAppearance() {
    document.documentElement.dataset.accent = settings.value.accent;
    const [accent, secondary] = ambientMode.value
      ? [ambientPrimary.value, ambientSecondary.value]
      : secretTheme.value
        ? accentThemes.aurora
        : accentThemes[settings.value.accent] ?? accentThemes.pink;
    const root = document.documentElement.style;
    root.setProperty("--color-accent", accent);
    root.setProperty("--color-accent-2", secondary);
    root.setProperty("--ambient", accent);
    root.setProperty("--ambient-2", secondary);
    persistUi();
  }

  function persistUi() {
    try {
      localStorage.setItem(
        UI_KEY,
        JSON.stringify({
          sidebarCollapsed: sidebarCollapsed.value,
          ambientMode: ambientMode.value,
          ambientPrimary: ambientPrimary.value,
          ambientSecondary: ambientSecondary.value,
          secretTheme: secretTheme.value,
        })
      );
    } catch {}
  }

  function toggleSidebar() {
    sidebarCollapsed.value = !sidebarCollapsed.value;
    persistUi();
  }

  function setAmbient(primary: string, secondary?: string) {
    ambientPrimary.value = primary;
    if (secondary) ambientSecondary.value = secondary;
    if (ambientMode.value) applyAppearance();
    else persistUi();
  }

  function toggleAmbient() {
    ambientMode.value = !ambientMode.value;
    applyAppearance();
    toast(
      "info",
      t("shell.ambientMode"),
      ambientMode.value ? t("shell.ambientOn") : t("shell.ambientOff")
    );
  }

  function registerLogoClick() {
    logoClicks += 1;
    window.clearTimeout(logoClickTimer);
    logoClickTimer = window.setTimeout(() => {
      logoClicks = 0;
    }, 1100);
    if (logoClicks >= 7) {
      logoClicks = 0;
      secretTheme.value = !secretTheme.value;
      applyAppearance();
      toast(secretTheme.value ? "success" : "info", t("shell.secretTheme"), secretTheme.value ? t("shell.secretOn") : t("shell.secretOff"));
    }
  }

  function openPalette() {
    paletteOpen.value = true;
  }

  function closePalette() {
    paletteOpen.value = false;
  }

  async function patchSettings(patch: Partial<AppSettings>): Promise<boolean> {
    const previous = settings.value;
    settings.value = { ...settings.value, ...patch };
    applyAppearance();
    if (patch.language) setLocale(patch.language);
    try {
      await api.settingsSet(settings.value);
      return true;
    } catch (e) {
      settings.value = previous;
      applyAppearance();
      if (patch.language) setLocale(previous.language);
      toast("error", t("toast.error"), String(e));
      return false;
    }
  }

  async function resetSettings() {
    settings.value = defaultSettings();
    applyAppearance();
    setLocale(settings.value.language);
    try {
      await api.settingsSet(settings.value);
      toast("success", t("toast.settingsSaved"));
    } catch (e) {
      toast("error", t("toast.error"), String(e));
    }
  }

  async function loadSystemInfo() {
    try {
      systemInfo.value = await api.systemInfo();
    } catch (e) {
      console.error("system_info", e);
      systemInfo.value = null;
    }
  }

  return {
    settings,
    loaded,
    demo,
    locale,
    scanning,
    systemInfo,
    toasts,
    activity,
    activityOpen,
    searchQuery,
    nowTick,
    unreadCount,
    sidebarCollapsed,
    ambientMode,
    ambientPrimary,
    ambientSecondary,
    secretTheme,
    paletteOpen,
    addActivity,
    toast,
    dismissToast,
    load,
    patchSettings,
    resetSettings,
    loadSystemInfo,
    toggleSidebar,
    setAmbient,
    toggleAmbient,
    registerLogoClick,
    openPalette,
    closePalette,
    applyAppearance,
  };
});
