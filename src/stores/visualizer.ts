import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { api, emitEvent, isTauri, listen } from "../lib/api";
import { KEY_CAPS, resolveKeyId } from "../lib/keyboard";
import type {
  KeyboardSource,
  LiveKeyEvent,
  LiveStatus,
  VisualizerEvent,
  VisualizerSettings,
} from "../lib/types";

export interface ThemePreset {
  id: VisualizerSettings["theme"];
  label: string;
  accent: string;
  pressed: string;
  held: string;
  modifier: string;
  glow: number;
}

export const THEMES: ThemePreset[] = [
  { id: "neon", label: "Neon Pink", accent: "#ff4fa3", pressed: "#ff4fa3", held: "#a78bfa", modifier: "#6ee7f9", glow: 70 },
  { id: "purple", label: "Purple", accent: "#a78bfa", pressed: "#a78bfa", held: "#c4b5fd", modifier: "#f0abfc", glow: 62 },
  { id: "cyber", label: "Cyber", accent: "#22d3ee", pressed: "#22d3ee", held: "#38bdf8", modifier: "#facc15", glow: 78 },
  { id: "minimal", label: "Minimal", accent: "#e5e7eb", pressed: "#e5e7eb", held: "#9ca3af", modifier: "#6b7280", glow: 0 },
  { id: "matrix", label: "Matrix", accent: "#45d483", pressed: "#45d483", held: "#86efac", modifier: "#22c55e", glow: 66 },
  { id: "custom", label: "Custom", accent: "#ff4fa3", pressed: "#ff4fa3", held: "#a78bfa", modifier: "#6ee7f9", glow: 70 },
];

const STORAGE_KEY = "udm.visualizer.settings";

export function defaultVisualizerSettings(): VisualizerSettings {
  return {
    theme: "neon",
    layout: "auto",
    iso: false,
    accent: "#ff4fa3",
    pressedColor: "#ff4fa3",
    heldColor: "#a78bfa",
    modifierColor: "#6ee7f9",
    glow: 70,
    animationMs: 90,
    keyLabels: "auto",
    showHistory: true,
    historyLimit: 200,
    showHeatmap: false,
    tracked: "all",
    overlayOpacity: 0.92,
    overlaySize: "m",
    overlayPosition: "bottom",
  };
}

export function loadVisualizerSettings(): VisualizerSettings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return { ...defaultVisualizerSettings(), ...(JSON.parse(raw) as Partial<VisualizerSettings>) };
  } catch {}
  return defaultVisualizerSettings();
}

export function saveVisualizerSettings(settings: VisualizerSettings) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch {}
}

const WEB_PATH = "web://browser-keyboard";

let webKeyDown: ((event: KeyboardEvent) => void) | null = null;
let webKeyUp: ((event: KeyboardEvent) => void) | null = null;

export const useVisualizerStore = defineStore("visualizer", () => {
  const settings = ref<VisualizerSettings>(loadVisualizerSettings());
  const active = ref(false);
  const initialized = ref(false);
  const overlayVisible = ref(false);
  const sources = ref<KeyboardSource[]>([]);
  const events = ref<VisualizerEvent[]>([]);
  const counts = ref<Record<string, number>>({});
  const pressedByPath = ref<Record<string, string[]>>({});
  const eventCount = ref(0);
  const lastEvent = ref<LiveKeyEvent | null>(null);
  const error = ref<string | null>(null);
  const stamps = ref<number[]>([]);

  const offs: (() => void)[] = [];
  let webListening = false;
  let sourceRefresh = 0;

  const totalPresses = computed(() => Object.values(counts.value).reduce((sum, value) => sum + value, 0));
  const pressedIds = computed(() => {
    const set = new Set<string>();
    for (const list of Object.values(pressedByPath.value)) {
      for (const id of list) set.add(id);
    }
    return [...set];
  });

  const topKeys = computed(() =>
    Object.entries(counts.value)
      .map(([id, count]) => ({ id, count }))
      .sort((a, b) => b.count - a.count)
      .slice(0, 12)
  );

  const rate = computed(() => {
    if (!stamps.value.length) return 0;
    const cutoff = Date.now() - 3000;
    const recent = stamps.value.filter((at) => at >= cutoff);
    return Math.round((recent.length / 3) * 10) / 10;
  });

  function keyName(id: string | null): string {
    if (!id) return "?";
    const item = KEY_CAPS[id];
    return item ? item.label : id;
  }

  function trimStamps(at: number) {
    stamps.value.push(at);
    const cutoff = at - 5000;
    while (stamps.value.length && stamps.value[0] < cutoff) stamps.value.shift();
  }

  function pushEvent(event: LiveKeyEvent) {
    const keyId = resolveKeyId(event.vk, event.extended, event.scan);
    const path = normalize(event.path);
    lastEvent.value = event;
    eventCount.value += 1;
    trimStamps(event.at);

    if (event.down) {
      const list = new Set(pressedByPath.value[path] ?? []);
      if (keyId) list.add(keyId);
      pressedByPath.value = { ...pressedByPath.value, [path]: [...list] };
    } else {
      const list = new Set(pressedByPath.value[path] ?? []);
      if (keyId) list.delete(keyId);
      pressedByPath.value = { ...pressedByPath.value, [path]: [...list] };
    }

    if (event.down && event.repeat) {
      const held = events.value.find(
        (item) => item.path === event.path && item.vk === event.vk && item.duration === null
      );
      if (held) held.repeats += 1;
      return;
    }
    if (!event.down) {
      const held = events.value.find(
        (item) => item.path === event.path && item.vk === event.vk && item.duration === null
      );
      if (held) held.duration = Math.max(0, event.at - held.at);
      return;
    }
    const record: VisualizerEvent = { ...event, keyId, duration: null, repeats: 0 };
    events.value.unshift(record);
    const limit = Math.max(20, Math.min(500, settings.value.historyLimit));
    if (events.value.length > limit) events.value.length = limit;
    const index = keyId ?? `vk:${event.vk}`;
    counts.value = { ...counts.value, [index]: (counts.value[index] ?? 0) + 1 };
  }

  function resetSession() {
    events.value = [];
    counts.value = {};
    pressedByPath.value = {};
    eventCount.value = 0;
    stamps.value = [];
    lastEvent.value = null;
  }

  function applyTheme(id: VisualizerSettings["theme"]) {
    settings.value.theme = id;
    const preset = THEMES.find((theme) => theme.id === id);
    if (!preset || id === "custom") return;
    settings.value.accent = preset.accent;
    settings.value.pressedColor = preset.pressed;
    settings.value.heldColor = preset.held;
    settings.value.modifierColor = preset.modifier;
    settings.value.glow = preset.glow;
  }

  function persist() {
    saveVisualizerSettings(settings.value);
    void emitEvent("visualizer:settings", settings.value);
  }

  function patch(patchValue: Partial<VisualizerSettings>) {
    settings.value = { ...settings.value, ...patchValue };
    if (patchValue.theme && patchValue.theme !== "custom") applyTheme(patchValue.theme);
    persist();
  }

  async function refreshStatus() {
    if (!isTauri()) return;
    try {
      const status = await api.liveInputStatus();
      active.value = status.active;
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    }
  }

  async function refreshSources() {
    if (!isTauri()) {
      if (active.value) {
        sources.value = [
          {
            deviceId: null,
            name: "Browser keyboard",
            connectionLabel: "DOM events (demo)",
            online: true,
            managed: false,
            receiverShared: false,
            interfaces: 1,
            paths: [WEB_PATH],
          },
        ];
      }
      return;
    }
    try {
      sources.value = await api.keyboardSources();
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    }
  }

  function normalize(path: string) {
    const lower = path.trim().replace(/^\\\\\?\\/, "").toLowerCase();
    const base = lower.split("#{")[0];
    return base.replace(/&col[0-9a-f]{2}(?=#)/, "");
  }

  function ensureSource(path: string) {
    const key = normalize(path);
    const known = sources.value.some((source) => source.paths.some((item) => normalize(item) === key));
    if (known || sourceRefresh) return;
    sourceRefresh = window.setTimeout(() => {
      sourceRefresh = 0;
      void refreshSources();
    }, 1200);
  }

  function startWebCapture() {
    if (webListening) return;
    webListening = true;
    webKeyDown = (event) => {
      const id = codeToId(event.code);
      if (!id) return;
      const item = KEY_CAPS[id];
      const vk = item?.vks[0] ?? 0;
      pushEvent({
        at: Date.now(),
        vk,
        scan: item?.scan?.[0] ?? 0,
        extended: isExtendedId(id),
        down: true,
        repeat: event.repeat,
        path: WEB_PATH,
        label: "Browser keyboard",
        deviceId: null,
      });
    };
    webKeyUp = (event) => {
      const id = codeToId(event.code);
      if (!id) return;
      const item = KEY_CAPS[id];
      pushEvent({
        at: Date.now(),
        vk: item?.vks[0] ?? 0,
        scan: item?.scan?.[0] ?? 0,
        extended: isExtendedId(id),
        down: false,
        repeat: false,
        path: WEB_PATH,
        label: "Browser keyboard",
        deviceId: null,
      });
    };
    window.addEventListener("keydown", webKeyDown);
    window.addEventListener("keyup", webKeyUp);
  }

  function stopWebCapture() {
    if (!webListening) return;
    webListening = false;
    if (webKeyDown) window.removeEventListener("keydown", webKeyDown);
    if (webKeyUp) window.removeEventListener("keyup", webKeyUp);
    webKeyDown = null;
    webKeyUp = null;
  }

  async function refreshOverlay() {
    if (!isTauri()) return;
    try {
      overlayVisible.value = await api.overlayStatus();
    } catch {}
  }

  async function init() {
    if (initialized.value) return;
    initialized.value = true;
    if (isTauri()) {
      offs.push(
        await listen<LiveKeyEvent>("keyboard:event", (event) => {
          if (!active.value) return;
          pushEvent(event);
          ensureSource(event.path);
        })
      );
      offs.push(
        await listen<LiveStatus>("keyboard:live", (status) => {
          active.value = status.active;
          if (status.active) {
            resetSession();
            void refreshSources();
          }
        })
      );
      offs.push(
        await listen<boolean>("visualizer:overlay", (visible) => {
          overlayVisible.value = visible;
        })
      );
    }
    await refreshStatus();
    await refreshSources();
    await refreshOverlay();
  }

  async function start() {
    error.value = null;
    if (!isTauri()) {
      active.value = true;
      resetSession();
      startWebCapture();
      await refreshSources();
      return;
    }
    try {
      resetSession();
      const status = await api.liveInputSet(true);
      active.value = status.active;
      await refreshSources();
    } catch (e) {
      active.value = false;
      error.value = e instanceof Error ? e.message : String(e);
    }
  }

  async function stop() {
    stopWebCapture();
    if (isTauri()) {
      try {
        await api.liveInputSet(false);
      } catch (e) {
        error.value = e instanceof Error ? e.message : String(e);
      }
    }
    active.value = false;
    pressedByPath.value = {};
  }

  async function toggle() {
    if (active.value) await stop();
    else await start();
  }

  async function showOverlay() {
    if (!active.value) await start();
    if (!isTauri()) {
      overlayVisible.value = true;
      return;
    }
    try {
      await api.overlayShow();
      await applyOverlayGeometry();
      overlayVisible.value = await api.overlayStatus();
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    }
  }

  async function hideOverlay(stopCapture = false) {
    if (isTauri()) {
      try {
        await api.overlayHide(stopCapture);
      } catch (e) {
        error.value = e instanceof Error ? e.message : String(e);
      }
    }
    if (stopCapture) active.value = false;
    overlayVisible.value = false;
  }

  async function applyOverlayGeometry() {
    if (!isTauri()) return;
    const { overlaySize } = settings.value;
    const size = overlaySize === "s" ? [460, 96] : overlaySize === "l" ? [780, 190] : [620, 132];
    try {
      await api.overlayResize(size[0], size[1], settings.value.overlayPosition);
    } catch {}
  }

  async function patchOverlay(patchValue: Partial<VisualizerSettings>) {
    patch(patchValue);
    if (overlayVisible.value) await applyOverlayGeometry();
  }

  function resetStats() {
    counts.value = {};
    events.value = [];
    stamps.value = [];
    eventCount.value = 0;
  }

  function clearHistory() {
    events.value = [];
  }

  function pressedFor(path: string | null): string[] {
    if (!path) return pressedIds.value;
    return pressedByPath.value[normalize(path)] ?? [];
  }

  function dispose() {
    for (const off of offs.splice(0)) off();
    if (sourceRefresh) window.clearTimeout(sourceRefresh);
    sourceRefresh = 0;
    stopWebCapture();
    resetSession();
    initialized.value = false;
  }

  return {
    settings,
    active,
    initialized,
    overlayVisible,
    sources,
    events,
    counts,
    pressedByPath,
    pressedIds,
    eventCount,
    lastEvent,
    error,
    totalPresses,
    topKeys,
    rate,
    keyName,
    init,
    start,
    stop,
    toggle,
    resetSession,
    resetStats,
    clearHistory,
    patch,
    patchOverlay,
    applyTheme,
    persist,
    refreshStatus,
    refreshSources,
    refreshOverlay,
    showOverlay,
    hideOverlay,
    applyOverlayGeometry,
    pressedFor,
    dispose,
  };
});

function codeToId(code: string): string | null {
  if (!code) return null;
  if (KEY_CAPS[code]) return code;
  if (code.startsWith("Numpad") && KEY_CAPS[code]) return code;
  return null;
}

function isExtendedId(id: string): boolean {
  return (
    id.startsWith("ControlRight") ||
    id.startsWith("AltRight") ||
    id.startsWith("Arrow") ||
    id === "NumpadEnter" ||
    id === "NumpadDivide" ||
    id === "Insert" ||
    id === "Delete" ||
    id === "Home" ||
    id === "End" ||
    id === "PageUp" ||
    id === "PageDown" ||
    id === "MetaLeft" ||
    id === "MetaRight" ||
    id === "ContextMenu" ||
    id === "PrintScreen"
  );
}
