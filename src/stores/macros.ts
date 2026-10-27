import { defineStore } from "pinia";
import { ref } from "vue";
import { api, listen } from "../lib/api";
import { t } from "../lib/i18n";
import type { Macro, MacroEvent } from "../lib/types";
import { useAppStore } from "./app";

export const useMacrosStore = defineStore("macros", () => {
  const macros = ref<Macro[]>([]);
  const recording = ref(false);
  const recordingEvents = ref<MacroEvent[]>([]);
  const recordingStartedAt = ref<number | null>(null);
  const playingId = ref<string | null>(null);
  const busy = ref(false);
  const app = useAppStore();

  async function load() {
    try {
      macros.value = (await api.macrosList()).sort((a, b) => b.createdAt - a.createdAt);
    } catch (e) {
      app.toast("error", t("toast.error"), String(e));
    }
  }

  async function save(macro: Macro) {
    await api.macroSave(macro);
    const idx = macros.value.findIndex((m) => m.id === macro.id);
    if (idx >= 0) macros.value[idx] = macro;
    else macros.value.unshift(macro);
    if (macro.hotkey) {
      const ok = await api.hotkeySet(macro.id, macro.hotkey);
      if (!ok) {
        macro.hotkey = null;
        await api.macroSave(macro);
        app.toast("warn", t("toast.error"), t("toast.hotkeyTaken"));
      }
    }
    app.toast("success", t("toast.macroSaved"));
  }

  async function remove(id: string) {
    await api.macroDelete(id);
    macros.value = macros.value.filter((m) => m.id !== id);
    app.toast("info", t("toast.deleted"));
  }

  async function startRecording() {
    try {
      recordingEvents.value = [];
      await api.macroRecordStart();
      recording.value = true;
      recordingStartedAt.value = Date.now();
      app.toast("info", t("toast.recordingStarted"));
    } catch (e) {
      app.toast("error", t("toast.error"), String(e));
    }
  }

  async function stopRecording(): Promise<Macro | null> {
    try {
      const macro = await api.macroRecordStop();
      recording.value = false;
      recordingStartedAt.value = null;
      return macro;
    } catch (e) {
      recording.value = false;
      app.toast("error", t("toast.error"), String(e));
      return null;
    }
  }

  async function cancelRecording() {
    try {
      await api.macroRecordCancel();
    } catch {}
    recording.value = false;
    recordingEvents.value = [];
    recordingStartedAt.value = null;
  }

  async function play(id: string) {
    try {
      await api.macroPlay(id);
      playingId.value = id;
      app.toast("info", t("toast.macroPlayed"));
    } catch (e) {
      app.toast("error", t("toast.error"), String(e));
    }
  }

  async function stop() {
    try {
      await api.macroStop();
    } catch {}
    playingId.value = null;
  }

  function newMacroShell(): Macro {
    return {
      id: `macro-${Date.now()}`,
      name: "",
      createdAt: Date.now(),
      events: [],
      repeat: 1,
      loop: false,
      hotkey: null,
    };
  }

  async function init() {
    await load();
    await listen<MacroEvent>("macro:event", (ev) => {
      if (recording.value) recordingEvents.value.push(ev);
    });
    await listen<{ playing: string | null }>("macro:state", (s) => {
      playingId.value = s.playing;
    });
  }

  return {
    macros,
    recording,
    recordingEvents,
    recordingStartedAt,
    playingId,
    busy,
    load,
    save,
    remove,
    startRecording,
    stopRecording,
    cancelRecording,
    play,
    stop,
    newMacroShell,
    init,
  };
});
