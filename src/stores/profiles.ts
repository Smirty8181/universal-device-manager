import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { api, listen } from "../lib/api";
import { t } from "../lib/i18n";
import type { Profile } from "../lib/types";
import { useAppStore } from "./app";

const ACTIVE_KEY = "udm.activeProfile";

export const useProfilesStore = defineStore("profiles", () => {
  const profiles = ref<Profile[]>([]);
  const activeId = ref<string | null>(localStorage.getItem(ACTIVE_KEY));
  const loading = ref(false);
  const app = useAppStore();

  const active = computed(() => profiles.value.find((p) => p.id === activeId.value) ?? null);

  async function load() {
    loading.value = true;
    try {
      profiles.value = (await api.profilesList()).map((p) => ({
        ...p,
        audio: {
          ...p.audio,
          volume: p.audio?.volume ?? 70,
          muted: p.audio?.muted ?? false,
          micGain: p.audio?.micGain ?? 70,
        },
      })).sort((a, b) => b.updatedAt - a.updatedAt);
      if (activeId.value && !profiles.value.some((p) => p.id === activeId.value)) {
        activeId.value = null;
      }
    } catch (e) {
      app.toast("error", t("toast.error"), String(e));
    } finally {
      loading.value = false;
    }
  }

  function emptyProfile(name: string): Profile {
    return {
      id: `profile-${Date.now()}`,
      name,
      createdAt: Date.now(),
      updatedAt: Date.now(),
      apps: [],
      audio: { volume: 70, muted: false, micGain: 70 },
      rgb: {
        enabled: true,
        effect: "static",
        color: "#ff4fa3",
        secondary: "#a78bfa",
        brightness: 80,
        speed: 45,
        direction: "left",
      },
      mouse: { dpi: 1600, polling: 1000, lod: 1, debounce: 4 },
      keyboard: { layout: "tkl", gameMode: false, batteryLight: null },
    };
  }

  async function save(profile: Profile) {
    profile.updatedAt = Date.now();
    await api.profileSave(profile);
    const idx = profiles.value.findIndex((p) => p.id === profile.id);
    if (idx >= 0) profiles.value[idx] = profile;
    else profiles.value.unshift(profile);
    app.toast("success", t("toast.profileSaved"));
  }

  async function remove(id: string) {
    await api.profileDelete(id);
    profiles.value = profiles.value.filter((p) => p.id !== id);
    if (activeId.value === id) {
      activeId.value = null;
      localStorage.removeItem(ACTIVE_KEY);
    }
    app.toast("info", t("toast.deleted"));
  }

  async function activate(id: string) {
    const p = profiles.value.find((x) => x.id === id);
    if (!p) return;
    try {
      await api.profileActivate(id);
    } catch (e) {
      app.toast("error", t("toast.error"), String(e));
      return;
    }
    activeId.value = id;
    localStorage.setItem(ACTIVE_KEY, id);
    app.addActivity("profile", `${t("dashboard.events.profile")}: ${p.name}`, "success");
    app.toast("success", t("toast.profileApplied"), p.name);
  }

  async function init() {
    await load();
    await listen<{ id: string }>("profile:activated", (payload) => {
      activeId.value = payload.id;
      localStorage.setItem(ACTIVE_KEY, payload.id);
      const p = profiles.value.find((x) => x.id === payload.id);
      if (p) app.addActivity("profile", `${t("dashboard.events.profile")}: ${p.name}`, "success");
    });
  }

  return { profiles, activeId, active, loading, load, emptyProfile, save, remove, activate, init };
});
