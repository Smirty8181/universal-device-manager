<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import {
  Circle, Clock, Command, Copy, Download, Keyboard, MousePointer2, Play, Square, Trash2, Upload,
} from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import EmptyState from "../components/EmptyState.vue";
import Modal from "../components/Modal.vue";
import Toggle from "../components/Toggle.vue";
import { useMacrosStore } from "../stores/macros";
import { useAppStore } from "../stores/app";
import { useI18n } from "../lib/i18n";
import { formatDuration, timeAgo } from "../lib/format";
import { copyText } from "../lib/api";
import type { Macro } from "../lib/types";

const { t, locale } = useI18n();
const macros = useMacrosStore();
const app = useAppStore();

const elapsed = ref(0);
let timer: number | undefined;

const editOpen = ref(false);
const draft = ref<Macro | null>(null);
const importOpen = ref(false);
const importText = ref("");
const deleteTarget = ref<Macro | null>(null);

const liveEvents = computed(() => macros.recordingEvents.slice(-14));
const recordedCount = computed(() => macros.recordingEvents.length);

function startTimer() {
  timer = window.setInterval(() => {
    if (macros.recordingStartedAt) elapsed.value = Date.now() - macros.recordingStartedAt;
  }, 120);
}
function stopTimer() {
  window.clearInterval(timer);
  timer = undefined;
}

function macroDuration(m: Macro) {
  const last = m.events.length ? m.events[m.events.length - 1].t : 0;
  return formatDuration(last);
}

async function startRecording() {
  await macros.startRecording();
  if (macros.recording) startTimer();
}

async function stopRecording() {
  stopTimer();
  const macro = await macros.stopRecording();
  if (macro) {
    macro.name = macro.name || `${t("macros.new")} ${macros.macros.length + 1}`;
    openEditor(macro);
  }
}

async function cancelRecording() {
  stopTimer();
  await macros.cancelRecording();
}

function openEditor(macro: Macro) {
  draft.value = JSON.parse(JSON.stringify(macro)) as Macro;
  editOpen.value = true;
}

async function saveDraft() {
  if (!draft.value || !draft.value.name.trim()) return;
  await macros.save(draft.value);
  editOpen.value = false;
  draft.value = null;
}

async function togglePlay(m: Macro) {
  if (macros.playingId === m.id) {
    await macros.stop();
  } else {
    await macros.play(m.id);
  }
}

async function removeMacro() {
  if (!deleteTarget.value) return;
  await macros.remove(deleteTarget.value.id);
  deleteTarget.value = null;
}

async function exportMacro(m: Macro) {
  try {
    await copyText(JSON.stringify(m, null, 2));
    app.toast("success", t("macros.exportDone"));
  } catch (e) {
    app.toast("error", t("toast.error"), String(e));
  }
}

function openImport() {
  importText.value = "";
  importOpen.value = true;
}

async function importMacro() {
  try {
    const data = JSON.parse(importText.value) as Partial<Macro>;
    if (!Array.isArray(data.events)) throw new Error("events");
    const macro: Macro = {
      id: `macro-${Date.now()}`,
      name: (data.name || "").trim() || t("macros.new"),
      createdAt: Date.now(),
      events: data.events,
      repeat: Math.min(999, Math.max(1, Number(data.repeat) || 1)),
      loop: Boolean(data.loop),
      hotkey: typeof data.hotkey === "string" && data.hotkey.trim() ? data.hotkey.trim() : null,
    };
    importOpen.value = false;
    openEditor(macro);
  } catch {
    app.toast("error", t("toast.error"), t("macros.importInvalid"));
  }
}

onMounted(() => {
  if (macros.recording && macros.recordingStartedAt) startTimer();
});
onBeforeUnmount(stopTimer);
</script>

<template>
  <div>
    <PageHeader :title="t('macros.title')" :subtitle="t('macros.subtitle')" :overline="t('nav.macros')">
      <template #actions>
        <button class="btn" @click="openImport">
          <Upload :size="13" />
          {{ t("common.import") }}
        </button>
        <button v-if="!macros.recording" class="btn btn-primary" @click="startRecording">
          <Circle :size="12" class="fill-current" />
          {{ t("macros.record") }}
        </button>
        <button v-else class="btn btn-danger" @click="stopRecording">
          <Square :size="12" class="fill-current" />
          {{ t("macros.stopRecording") }}
        </button>
      </template>
    </PageHeader>

    <div v-if="macros.recording" class="panel p-5 mb-4 border-bad/30" style="box-shadow: 0 0 40px rgba(255,80,80,.06) inset">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-3">
          <span class="w-3 h-3 rounded-full bg-bad animate-pulse" style="box-shadow: 0 0 14px rgba(255,90,90,.8)"></span>
          <div>
            <div class="text-[13px] font-semibold">{{ t("macros.recording") }}</div>
            <div class="text-[10.5px] text-dim">{{ t("macros.recordHint") }}</div>
          </div>
        </div>
        <div class="text-right">
          <div class="mono text-[22px] tabular-nums leading-none">{{ formatDuration(elapsed) }}</div>
          <div class="text-[10px] text-dim mt-1">{{ recordedCount }} {{ t("macros.events").toLowerCase() }}</div>
        </div>
      </div>

      <div class="grid grid-cols-2 gap-1.5 mt-4 max-h-[190px] overflow-y-auto scroll-region">
        <div v-for="(ev, i) in liveEvents" :key="i" class="flex items-center gap-2 px-2.5 py-1.5 rounded-lg bg-white/[0.03] border border-white/[0.04]">
          <component :is="ev.kind === 'key' ? Keyboard : MousePointer2" :size="11" class="text-dim shrink-0" />
          <span class="mono text-[10px] text-dim">{{ formatDuration(ev.t) }}</span>
          <span class="text-[11px] truncate">{{ ev.label }}</span>
          <span class="ml-auto chip !h-4 !px-1.5 !text-[8.5px]" :class="ev.down ? 'chip-ok' : ''">{{ ev.down ? "↓" : "↑" }}</span>
        </div>
        <div v-if="!liveEvents.length" class="col-span-2 text-[11px] text-dim py-3 text-center">{{ t("macros.eventsEmpty") }}</div>
      </div>

      <div class="flex items-center justify-between mt-4">
        <div class="text-[10.5px] text-dim">{{ t("macros.recordingNote") }}</div>
        <div class="flex gap-2">
          <button class="btn" @click="cancelRecording">{{ t("macros.cancelRecording") }}</button>
          <button class="btn btn-primary" @click="stopRecording">{{ t("macros.saveRecorded") }}</button>
        </div>
      </div>
    </div>

    <div class="grid grid-cols-12 gap-4">
      <div class="col-span-7">
        <div class="panel p-5">
          <div class="flex items-center justify-between mb-3">
            <div class="section-title">{{ t("macros.title") }}</div>
            <span class="text-[10.5px] text-dim">{{ macros.macros.length }}</span>
          </div>

          <EmptyState v-if="!macros.macros.length" :icon="Command" :title="t('macros.emptyTitle')" :body="t('macros.emptyBody')" />

          <div v-else class="space-y-1.5">
            <div v-for="m in macros.macros" :key="m.id" class="px-3 py-3 rounded-xl border border-white/[0.05] hover:border-white/10 transition-colors">
              <div class="flex items-center gap-3">
                <span class="w-9 h-9 rounded-lg grid place-items-center bg-white/[0.04] border border-white/[0.06] shrink-0">
                  <Play :size="14" :class="macros.playingId === m.id ? 'text-ok animate-pulse' : 'text-dim'" />
                </span>
                <div class="min-w-0 flex-1">
                  <div class="flex items-center gap-2">
                    <span class="text-[12.5px] font-medium truncate">{{ m.name }}</span>
                    <span v-if="m.hotkey" class="chip !h-5 mono !text-[9.5px]">{{ m.hotkey }}</span>
                    <span v-if="m.loop" class="chip chip-info !h-5 !text-[9.5px]">{{ t("macros.loop") }}</span>
                    <span v-if="m.repeat > 1" class="chip !h-5 !text-[9.5px] mono">×{{ m.repeat }}</span>
                  </div>
                  <div class="text-[10px] text-dim mt-0.5">
                    {{ m.events.length }} {{ t("macros.events").toLowerCase() }} · {{ macroDuration(m) }} · {{ timeAgo(m.createdAt, locale) }}
                  </div>
                </div>
                <div class="flex items-center gap-1 shrink-0">
                  <button class="btn btn-ghost !h-7 !px-2" :title="macros.playingId === m.id ? t('macros.stop') : t('macros.play')" @click="togglePlay(m)">
                    <Square v-if="macros.playingId === m.id" :size="12" />
                    <Play v-else :size="12" />
                  </button>
                  <button class="btn btn-ghost !h-7 !px-2" :title="t('common.edit')" @click="openEditor(m)">
                    <Command :size="12" />
                  </button>
                  <button class="btn btn-ghost !h-7 !px-2" :title="t('common.export')" @click="exportMacro(m)">
                    <Download :size="12" />
                  </button>
                  <button class="btn btn-ghost !h-7 !px-2 text-dim hover:text-bad" :title="t('common.delete')" @click="deleteTarget = m">
                    <Trash2 :size="12" />
                  </button>
                </div>
              </div>
              <div v-if="m.events.length" class="flex flex-wrap gap-1 mt-2 pl-12">
                <span v-for="(ev, i) in m.events.slice(0, 8)" :key="i" class="chip !h-4 !px-1.5 !text-[8.5px] mono">{{ ev.label }}</span>
                <span v-if="m.events.length > 8" class="text-[9px] text-dim self-center">+{{ m.events.length - 8 }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="col-span-5 space-y-4">
        <div class="panel p-5">
          <div class="section-title mb-2">{{ t("macros.hotkey") }}</div>
          <div class="text-[11px] text-dim leading-relaxed">{{ t("macros.hotkeyHint") }}</div>
        </div>

        <div class="panel p-5">
          <div class="section-title mb-2">{{ t("common.import") }} / {{ t("common.export") }}</div>
          <div class="text-[11px] text-dim leading-relaxed">{{ t("macros.importHint") }}</div>
          <div class="flex gap-2 mt-3">
            <button class="btn flex-1" @click="openImport">
              <Upload :size="12" />
              {{ t("common.import") }}
            </button>
            <button class="btn flex-1" :disabled="!macros.macros.length" @click="exportMacro(macros.macros[0])">
              <Copy :size="12" />
              {{ t("common.export") }}
            </button>
          </div>
        </div>

        <div class="panel p-5">
          <div class="flex items-center gap-2 mb-1">
            <Clock :size="13" class="text-dim" />
            <div class="text-[11.5px]">{{ t("macros.duration") }}</div>
          </div>
          <div class="text-[10.5px] text-dim leading-relaxed">{{ t("macros.recordingNote") }}</div>
        </div>
      </div>
    </div>

    <Modal :open="editOpen" :title="t('macros.saveRecorded')" width="440px" @close="editOpen = false">
      <template v-if="draft">
        <label class="block text-[11px] text-muted mb-1.5">{{ t("common.name") }}</label>
        <input v-model="draft.name" class="input" :placeholder="t('macros.namePlaceholder')" />

        <div class="grid grid-cols-2 gap-3 mt-3">
          <div>
            <label class="block text-[11px] text-muted mb-1.5">{{ t("macros.repeat") }}</label>
            <input v-model.number="draft.repeat" type="number" min="1" max="999" class="input mono" />
          </div>
          <div>
            <label class="block text-[11px] text-muted mb-1.5">{{ t("macros.hotkey") }}</label>
            <input v-model="draft.hotkey" class="input mono" placeholder="Ctrl+Alt+M" />
          </div>
        </div>

        <div class="flex items-center justify-between mt-4">
          <div>
            <div class="text-[11.5px]">{{ t("macros.loop") }}</div>
            <div class="text-[10px] text-dim">{{ t("macros.hotkeyHint") }}</div>
          </div>
          <Toggle v-model="draft.loop" />
        </div>

        <div class="card-flat px-3 py-2.5 mt-4 flex items-center justify-between">
          <span class="text-[11px] text-dim">{{ t("macros.events") }}</span>
          <span class="mono text-[11px]">{{ draft.events.length }} · {{ macroDuration(draft) }}</span>
        </div>
      </template>
      <template #footer>
        <button class="btn" @click="editOpen = false">{{ t("common.cancel") }}</button>
        <button class="btn btn-primary" :disabled="!draft?.name.trim()" @click="saveDraft">{{ t("common.save") }}</button>
      </template>
    </Modal>

    <Modal :open="importOpen" :title="t('common.import')" width="480px" @close="importOpen = false">
      <label class="block text-[11px] text-muted mb-1.5">{{ t("macros.importHint") }}</label>
      <textarea v-model="importText" class="input !h-[220px] font-mono !text-[11px]" spellcheck="false" placeholder='{ "name": "...", "events": [] }'></textarea>
      <template #footer>
        <button class="btn" @click="importOpen = false">{{ t("common.cancel") }}</button>
        <button class="btn btn-primary" :disabled="!importText.trim()" @click="importMacro">{{ t("common.import") }}</button>
      </template>
    </Modal>

    <Modal :open="Boolean(deleteTarget)" :title="t('macros.deleteConfirm')" width="380px" @close="deleteTarget = null">
      <div class="text-[12px] text-muted">{{ deleteTarget?.name }}</div>
      <template #footer>
        <button class="btn" @click="deleteTarget = null">{{ t("common.cancel") }}</button>
        <button class="btn btn-danger" @click="removeMacro">{{ t("common.delete") }}</button>
      </template>
    </Modal>
  </div>
</template>
