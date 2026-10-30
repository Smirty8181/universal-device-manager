<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import {
  BarChart3,
  Flame,
  Gamepad2,
  Gauge,
  History,
  Keyboard,
  Monitor,
  MonitorOff,
  Play,
  RefreshCw,
  RotateCcw,
  ShieldCheck,
  Square,
  Type,
} from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import Tabs from "../components/Tabs.vue";
import KeyboardCanvas from "../components/KeyboardCanvas.vue";
import { useI18n } from "../lib/i18n";
import { useVisualizerStore, THEMES } from "../stores/visualizer";
import { chordLabel, keyLabel, resolveKeyId, type LabelLang, type LayoutSize } from "../lib/keyboard";
import type { LiveKeyEvent, VisualizerSettings } from "../lib/types";

type Tab = "test" | "history" | "heatmap" | "counter" | "typing" | "gaming";

const { t, locale } = useI18n();
const viz = useVisualizerStore();

const tab = ref<Tab>("test");
const selected = ref<number | null>(null);
const maxSimul = ref(0);
const nowTick = ref(Date.now());

const tabs = computed(() => [
  { id: "test" as const, label: t("vis.tabTest") },
  { id: "history" as const, label: t("vis.tabHistory"), count: viz.events.length },
  { id: "heatmap" as const, label: t("vis.tabHeatmap") },
  { id: "counter" as const, label: t("vis.tabCounter") },
  { id: "typing" as const, label: t("vis.tabTyping") },
  { id: "gaming" as const, label: t("vis.tabGaming") },
]);

const labelLang = computed<LabelLang>(() => {
  if (viz.settings.keyLabels !== "auto") return viz.settings.keyLabels;
  return locale.value === "ru" ? "ru" : "en";
});

const layoutSize = computed<LayoutSize>(() => {
  const preset = viz.settings.layout;
  if (preset !== "auto") return preset;
  const names = viz.sources.map((source) => source.name.toLowerCase()).join(" ");
  if (/\b60\b/.test(names)) return "60";
  if (/\b65\b/.test(names)) return "65";
  if (/\b75\b/.test(names)) return "75";
  if (/tkl|tenkeyless/.test(names)) return "tkl";
  return "full";
});

const selectedSource = computed(() => (selected.value === null ? null : viz.sources[selected.value] ?? null));
const pressedKeys = computed(() => {
  const source = selectedSource.value;
  if (!source) return viz.pressedIds;
  const set = new Set<string>();
  for (const path of source.paths) {
    for (const id of viz.pressedFor(path)) set.add(id);
  }
  return [...set];
});
const chord = computed(() => (pressedKeys.value.length ? chordLabel(pressedKeys.value) : null));
const sharedReceiver = computed(() => viz.sources.some((source) => source.receiverShared));
const recent = computed(() => viz.events.slice(0, 6));
const heatmap = computed(() => (tab.value === "heatmap" ? viz.counts : null));

const lastSeen = computed(() => {
  const at = viz.lastEvent?.at;
  if (!at) return t("vis.never");
  const seconds = Math.max(0, Math.round((nowTick.value - at) / 1000));
  if (seconds < 2) return t("vis.justNow");
  return `${seconds} ${t("vis.secondsAgo")}`;
});

watch(
  () => viz.sources.length,
  () => {
    if (selected.value !== null && selected.value >= viz.sources.length) selected.value = null;
  }
);

watch(
  () => pressedKeys.value.length,
  (count) => {
    if (count > maxSimul.value) maxSimul.value = count;
  }
);

watch(
  () => viz.active,
  (on) => {
    if (on) maxSimul.value = 0;
  }
);

const typingText = ref("");
const typingIndex = ref(0);
const typingWrong = ref(0);
const typingActive = ref(false);
const typingStartedAt = ref<number | null>(null);
const typingDoneAt = ref<number | null>(null);
const typingMarks = ref<("ok" | "bad" | null)[]>([]);

const PHRASES = [
  "the quick brown fox jumps over the lazy dog",
  "pack my box with five dozen liquor jugs",
  "how vexingly quick daft zebras jump",
  "bright vixens jump dozy fowl quack",
];

function charToKey(ch: string): string | null {
  if (ch === " ") return "Space";
  if (ch >= "a" && ch <= "z") return `Key${ch.toUpperCase()}`;
  if (ch >= "0" && ch <= "9") return `Digit${ch}`;
  if (ch === ".") return "Period";
  if (ch === ",") return "Comma";
  if (ch === "'") return "Quote";
  return null;
}

const typingElapsed = computed(() => {
  if (!typingStartedAt.value) return 0;
  const end = typingDoneAt.value ?? nowTick.value;
  return Math.max(0, end - typingStartedAt.value);
});
const typingWpm = computed(() => {
  if (!typingStartedAt.value || typingElapsed.value < 500) return 0;
  return Math.round(typingIndex.value / 5 / (typingElapsed.value / 60000));
});
const typingAccuracy = computed(() => {
  const typed = typingIndex.value + typingWrong.value;
  if (!typed) return 100;
  return Math.round((typingIndex.value / typed) * 100);
});

function resetTyping(next = true) {
  typingActive.value = false;
  typingIndex.value = 0;
  typingWrong.value = 0;
  typingStartedAt.value = null;
  typingDoneAt.value = null;
  typingMarks.value = [];
  typingText.value = next ? PHRASES[Math.floor(Math.random() * PHRASES.length)] : "";
  typingMarks.value = Array(typingText.value.length).fill(null);
}

async function startTyping() {
  if (!viz.active) await viz.start();
  resetTyping(true);
  typingActive.value = true;
}

function handleTyping(event: LiveKeyEvent | null) {
  if (!event || !typingActive.value || typingDoneAt.value) return;
  if (!event.down || event.repeat) return;
  if (typingIndex.value >= typingText.value.length) return;
  const actual = resolveKeyId(event.vk, event.extended, event.scan);
  if (!actual) return;
  const expected = charToKey(typingText.value[typingIndex.value]);
  if (!expected) return;
  if (!typingStartedAt.value) typingStartedAt.value = Date.now();
  if (actual === expected) {
    typingMarks.value[typingIndex.value] = "ok";
    typingIndex.value += 1;
    if (typingIndex.value >= typingText.value.length) {
      typingDoneAt.value = Date.now();
      typingActive.value = false;
    }
  } else {
    typingMarks.value[typingIndex.value] = "bad";
    typingWrong.value += 1;
  }
}

function resetAll() {
  viz.resetSession();
  maxSimul.value = 0;
  resetTyping(false);
  selected.value = null;
}

async function stopAll() {
  await viz.stop();
  if (viz.overlayVisible) await viz.hideOverlay(false);
}

function setColor(key: "accent" | "pressedColor" | "heldColor" | "modifierColor", value: string) {
  const patch: Partial<VisualizerSettings> = { theme: "custom" };
  patch[key] = value;
  viz.patch(patch);
}

function rangeStyle(value: number, min: number, max: number) {
  return { "--fill": `${Math.round(((value - min) / (max - min)) * 100)}%` };
}

function fmtTime(at: number) {
  return new Date(at).toLocaleTimeString(locale.value === "ru" ? "ru-RU" : "en-GB", { hour12: false });
}

function fmtDuration(ms: number | null) {
  if (ms === null) return "…";
  if (ms < 1000) return `${Math.round(ms)} ms`;
  return `${(ms / 1000).toFixed(1)} s`;
}

const topMax = computed(() => Math.max(1, viz.topKeys[0]?.count ?? 1));

let ticker = 0;
watch(
  () => viz.lastEvent,
  (event) => {
    handleTyping(event);
  }
);

onMounted(async () => {
  ticker = window.setInterval(() => {
    nowTick.value = Date.now();
  }, 1000);
  await viz.init();
  await viz.refreshOverlay();
  if (!viz.active) await viz.start();
});

onBeforeUnmount(() => {
  window.clearInterval(ticker);
  if (!viz.overlayVisible) void viz.stop();
});
</script>

<template>
  <div>
    <PageHeader :title="t('nav.visualizer')" :subtitle="t('vis.subtitle')" :overline="t('vis.overline')">
      <template #actions>
        <button class="btn" @click="viz.overlayVisible ? viz.hideOverlay(false) : viz.showOverlay()">
          <Monitor v-if="!viz.overlayVisible" :size="15" />
          <MonitorOff v-else :size="15" />
          {{ viz.overlayVisible ? t("vis.overlayHide") : t("vis.overlay") }}
        </button>
        <button v-if="viz.active" class="btn btn-danger" @click="stopAll">
          <Square :size="13" />
          {{ t("vis.stop") }}
        </button>
        <button v-else class="btn btn-primary" @click="viz.start()">
          <Play :size="14" />
          {{ t("vis.start") }}
        </button>
      </template>
    </PageHeader>

    <div v-if="viz.error" class="card px-4 py-3 mb-4 border-bad/25 bg-bad/[0.05] text-[12px] text-bad">
      {{ viz.error }}
    </div>

    <div class="grid grid-cols-1 2xl:grid-cols-[minmax(0,1fr)_330px] gap-4 items-start">
      <div class="min-w-0 space-y-4">
        <div class="card px-4 py-3">
          <div class="flex items-center gap-2 flex-wrap">
            <span class="chip" :class="viz.active ? 'chip-ok' : 'chip-bad'">
              <span class="status-dot" :class="viz.active ? 'bg-ok animate-pulse' : 'bg-bad'"></span>
              {{ viz.active ? t("vis.live") : t("vis.idle") }}
            </span>
            <span class="text-[11px] text-dim">{{ t("vis.events") }} <b class="text-text mono">{{ viz.eventCount }}</b></span>
            <span class="text-[11px] text-dim">{{ t("vis.rate") }} <b class="text-text mono">{{ viz.rate }}</b></span>
            <span class="text-[11px] text-dim">{{ t("vis.last") }} <b class="text-text mono">{{ lastSeen }}</b></span>
            <div class="ml-auto flex items-center gap-1">
              <button class="icon-btn !w-7 !h-7" :title="t('vis.refreshSources')" @click="viz.refreshSources(); viz.refreshStatus()">
                <RefreshCw :size="13" />
              </button>
              <button class="icon-btn !w-7 !h-7" :title="t('vis.reset')" @click="resetAll">
                <RotateCcw :size="13" />
              </button>
            </div>
          </div>

          <div class="mt-3 flex items-center gap-1.5 flex-wrap">
            <button class="src-chip" :class="{ active: selected === null }" @click="selected = null">
              <Keyboard :size="11" />
              {{ t("vis.allSources") }}
            </button>
            <button
              v-for="(source, index) in viz.sources"
              :key="`${source.name}-${index}`"
              class="src-chip"
              :class="{ active: selected === index }"
              @click="selected = index"
            >
              <span class="status-dot" :class="source.online ? 'bg-ok' : 'bg-dim'"></span>
              <span class="truncate max-w-[190px]">{{ source.name }}</span>
              <span class="text-dim/80">{{ source.connectionLabel }}</span>
            </button>
            <span v-if="!viz.sources.length" class="text-[11px] text-dim">{{ t("vis.noSources") }}</span>
          </div>

          <p v-if="sharedReceiver" class="mt-2 text-[10.5px] text-warn/80 leading-relaxed">
            {{ t("vis.sharedNote") }}
          </p>
        </div>

        <div class="card p-4">
          <Tabs v-model="tab" :tabs="tabs" />

          <KeyboardCanvas
            :layout="layoutSize"
            :iso="viz.settings.iso"
            :pressed="pressedKeys"
            :label-lang="labelLang"
            :accent="viz.settings.accent"
            :pressed-color="viz.settings.pressedColor"
            :held-color="viz.settings.heldColor"
            :modifier-color="viz.settings.modifierColor"
            :glow="viz.settings.glow"
            :animation-ms="viz.settings.animationMs"
            :heatmap="heatmap"
            :tracked="viz.settings.tracked"
          />

          <div v-if="viz.settings.showHistory && recent.length" class="mt-4 flex items-center gap-1.5 flex-wrap">
            <span class="text-[10px] uppercase tracking-[0.12em] text-dim">{{ t("vis.recent") }}</span>
            <span
              v-for="event in recent"
              :key="`${event.at}-${event.vk}-${event.path}`"
              class="kbd-chip mono"
            >
              {{ viz.keyName(event.keyId) }}
            </span>
          </div>

          <div v-if="tab === 'test'" class="mt-5 grid grid-cols-2 sm:grid-cols-4 gap-2">
            <div class="stat-box">
              <div class="stat-label">{{ t("vis.chord") }}</div>
              <div class="stat-value truncate">{{ chord ?? "—" }}</div>
            </div>
            <div class="stat-box">
              <div class="stat-label">{{ t("vis.held") }}</div>
              <div class="stat-value">{{ pressedKeys.length }}</div>
            </div>
            <div class="stat-box">
              <div class="stat-label">{{ t("vis.maxSimul") }}</div>
              <div class="stat-value">{{ maxSimul }}</div>
            </div>
            <div class="stat-box">
              <div class="stat-label">{{ t("vis.totalPresses") }}</div>
              <div class="stat-value">{{ viz.totalPresses }}</div>
            </div>
            <p class="col-span-2 sm:col-span-4 text-[10.5px] text-dim leading-relaxed">{{ t("vis.nkroHint") }}</p>
          </div>

          <div v-else-if="tab === 'history'" class="mt-5">
            <div class="flex items-center gap-2 mb-2">
              <span class="text-[12px] font-medium">{{ t("vis.history") }}</span>
              <button class="ml-auto text-[11px] text-dim hover:text-text transition-colors" @click="viz.clearHistory()">
                {{ t("vis.clearHistory") }}
              </button>
            </div>
            <div v-if="!viz.events.length" class="text-[11.5px] text-dim py-6 text-center">{{ t("vis.emptyHistory") }}</div>
            <div v-else class="hist-table">
              <div class="hist-head">
                <span>{{ t("vis.time") }}</span>
                <span>{{ t("vis.key") }}</span>
                <span>{{ t("vis.device") }}</span>
                <span>{{ t("vis.duration") }}</span>
                <span>{{ t("vis.repeats") }}</span>
              </div>
              <div v-for="event in viz.events.slice(0, 80)" :key="`${event.at}-${event.vk}`" class="hist-row">
                <span class="mono text-dim">{{ fmtTime(event.at) }}</span>
                <span class="font-medium">{{ viz.keyName(event.keyId) }}</span>
                <span class="truncate text-dim">{{ event.label || "—" }}</span>
                <span class="mono text-dim">{{ fmtDuration(event.duration) }}</span>
                <span class="mono text-dim">{{ event.repeats }}</span>
              </div>
            </div>
            <p class="mt-2 text-[10px] text-dim/80">{{ t("vis.historyRam") }}</p>
          </div>

          <div v-else-if="tab === 'heatmap'" class="mt-5">
            <p class="text-[11px] text-dim leading-relaxed">{{ t("vis.heatmapHint") }}</p>
            <p v-if="!viz.totalPresses" class="text-[11px] text-dim py-3">{{ t("vis.heatmapEmpty") }}</p>
          </div>

          <div v-else-if="tab === 'counter'" class="mt-5">
            <div class="flex items-center gap-2 mb-3">
              <span class="chip chip-accent">{{ t("vis.totalPresses") }}: {{ viz.totalPresses }}</span>
              <button class="ml-auto text-[11px] text-dim hover:text-text transition-colors" @click="viz.resetStats()">
                {{ t("vis.clearCounts") }}
              </button>
            </div>
            <div v-if="!viz.topKeys.length" class="text-[11.5px] text-dim py-6 text-center">{{ t("vis.emptyCounts") }}</div>
            <div v-else class="space-y-2">
              <div v-for="item in viz.topKeys" :key="item.id" class="flex items-center gap-3">
                <span class="w-24 truncate text-[11.5px] font-medium">{{ keyLabel(item.id, labelLang) }}</span>
                <div class="flex-1 h-2 rounded-full bg-white/[0.05] overflow-hidden">
                  <div
                    class="h-full rounded-full"
                    :style="{
                      width: `${Math.max(3, (item.count / topMax) * 100)}%`,
                      background: `linear-gradient(90deg, ${viz.settings.accent}, ${viz.settings.heldColor})`,
                    }"
                  ></div>
                </div>
                <span class="w-10 text-right mono text-[11px] text-dim">{{ item.count }}</span>
              </div>
            </div>
          </div>

          <div v-else-if="tab === 'typing'" class="mt-5">
            <div class="flex items-center gap-2 mb-3 flex-wrap">
              <button class="btn btn-primary !h-9" @click="startTyping">
                <Play :size="13" />
                {{ typingActive ? t("vis.typeRestart") : t("vis.typeStart") }}
              </button>
              <span class="chip">{{ t("vis.wpm") }}: <b class="mono">{{ typingWpm }}</b></span>
              <span class="chip">{{ t("vis.accuracy") }}: <b class="mono">{{ typingAccuracy }}%</b></span>
              <span v-if="typingDoneAt" class="chip chip-ok">{{ t("vis.typeDone") }}</span>
            </div>
            <div class="typing-box">
              <span
                v-for="(ch, index) in typingText"
                :key="index"
                class="t-char"
                :class="[
                  typingMarks[index] === 'ok' ? 't-ok' : '',
                  typingMarks[index] === 'bad' ? 't-bad' : '',
                  index === typingIndex && !typingDoneAt ? 't-cur' : '',
                ]"
              >{{ ch === " " ? "·" : ch }}</span>
              <span v-if="!typingText" class="text-[12px] text-dim">{{ t("vis.typeIdle") }}</span>
            </div>
            <p class="mt-2 text-[10.5px] text-dim leading-relaxed">{{ t("vis.typingHint") }}</p>
          </div>

          <div v-else-if="tab === 'gaming'" class="mt-5">
            <div class="grid grid-cols-1 sm:grid-cols-3 gap-2">
              <div class="stat-box">
                <div class="stat-label">{{ t("vis.gamingCurrent") }}</div>
                <div class="stat-value truncate">{{ chord ?? "—" }}</div>
              </div>
              <div class="stat-box">
                <div class="stat-label">{{ t("vis.gamingMax") }}</div>
                <div class="stat-value">{{ maxSimul }}</div>
              </div>
              <div class="stat-box">
                <div class="stat-label">{{ t("vis.gamingHeld") }}</div>
                <div class="stat-value">{{ pressedKeys.length }}</div>
              </div>
            </div>
            <p class="mt-3 text-[10.5px] text-dim leading-relaxed">{{ t("vis.gamingHint") }}</p>
          </div>
        </div>
      </div>

      <aside class="space-y-4">
        <div class="card px-4 py-3.5">
          <div class="flex items-center gap-2 mb-1.5">
            <ShieldCheck :size="15" class="text-ok" />
            <span class="text-[12.5px] font-semibold">{{ t("vis.privacyTitle") }}</span>
          </div>
          <p class="text-[10.5px] text-dim leading-relaxed">{{ t("vis.privacyBody") }}</p>
          <div class="mt-2.5 flex items-center gap-2 text-[10.5px]">
            <span class="chip" :class="viz.active ? 'chip-bad' : 'chip-ok'">
              <span class="status-dot" :class="viz.active ? 'bg-bad animate-pulse' : 'bg-ok'"></span>
              {{ viz.active ? t("vis.captureOn") : t("vis.captureOff") }}
            </span>
            <button v-if="viz.active" class="ml-auto btn btn-danger !h-7 !px-2.5 !text-[11px]" @click="stopAll">
              {{ t("vis.stopNow") }}
            </button>
          </div>
        </div>

        <div class="card px-4 py-3.5">
          <div class="section-title">{{ t("vis.settings") }}</div>

          <div class="setting-block">
            <div class="setting-label">{{ t("vis.theme") }}</div>
            <div class="grid grid-cols-3 gap-1.5">
              <button
                v-for="preset in THEMES"
                :key="preset.id"
                class="theme-btn"
                :class="{ active: viz.settings.theme === preset.id }"
                @click="viz.patch({ theme: preset.id })"
              >
                <span
                  class="w-4 h-4 rounded-full border border-white/20 shrink-0"
                  :style="{ background: `linear-gradient(135deg, ${preset.accent}, ${preset.held})` }"
                ></span>
                <span class="truncate">{{ preset.label }}</span>
              </button>
            </div>
          </div>

          <div class="setting-block">
            <div class="setting-label">{{ t("vis.colors") }}</div>
            <div class="grid grid-cols-2 gap-2">
              <label class="color-row">
                <input class="color-swatch" type="color" :value="viz.settings.accent" @input="setColor('accent', ($event.target as HTMLInputElement).value)" />
                <span>{{ t("vis.colorAccent") }}</span>
              </label>
              <label class="color-row">
                <input class="color-swatch" type="color" :value="viz.settings.pressedColor" @input="setColor('pressedColor', ($event.target as HTMLInputElement).value)" />
                <span>{{ t("vis.colorPressed") }}</span>
              </label>
              <label class="color-row">
                <input class="color-swatch" type="color" :value="viz.settings.heldColor" @input="setColor('heldColor', ($event.target as HTMLInputElement).value)" />
                <span>{{ t("vis.colorHeld") }}</span>
              </label>
              <label class="color-row">
                <input class="color-swatch" type="color" :value="viz.settings.modifierColor" @input="setColor('modifierColor', ($event.target as HTMLInputElement).value)" />
                <span>{{ t("vis.colorModifier") }}</span>
              </label>
            </div>
          </div>

          <div class="setting-block">
            <div class="setting-label">{{ t("vis.layout") }}</div>
            <div class="seg">
              <button
                v-for="size in (['auto', 'full', 'tkl', '75', '65', '60'] as const)"
                :key="size"
                class="seg-btn"
                :class="{ active: viz.settings.layout === size }"
                @click="viz.patch({ layout: size })"
              >
                {{ size === "auto" ? t("vis.layoutAuto") : size === "full" ? "Full" : size.toUpperCase() }}
              </button>
            </div>
            <label class="flex items-center gap-2 mt-2 text-[11px] text-dim cursor-pointer select-none">
              <input
                type="checkbox"
                :checked="viz.settings.iso"
                class="w-3.5 h-3.5 accent-[var(--color-accent)]"
                @change="viz.patch({ iso: ($event.target as HTMLInputElement).checked })"
              />
              {{ t("vis.iso") }}
            </label>
          </div>

          <div class="setting-block">
            <div class="setting-label">{{ t("vis.labels") }}</div>
            <div class="seg">
              <button
                v-for="lang in (['auto', 'en', 'ru'] as const)"
                :key="lang"
                class="seg-btn"
                :class="{ active: viz.settings.keyLabels === lang }"
                @click="viz.patch({ keyLabels: lang })"
              >
                {{ lang === "auto" ? t("vis.layoutAuto") : lang.toUpperCase() }}
              </button>
            </div>
          </div>

          <div class="setting-block">
            <div class="setting-label">{{ t("vis.tracked") }}</div>
            <div class="seg">
              <button
                v-for="mode in (['all', 'gaming', 'typing'] as const)"
                :key="mode"
                class="seg-btn"
                :class="{ active: viz.settings.tracked === mode }"
                @click="viz.patch({ tracked: mode })"
              >
                {{ t(`vis.tracked_${mode}`) }}
              </button>
            </div>
          </div>

          <div class="setting-block">
            <div class="flex items-center justify-between">
              <span class="setting-label !mb-0">{{ t("vis.glow") }}</span>
              <span class="mono text-[10.5px] text-dim">{{ viz.settings.glow }}%</span>
            </div>
            <input
              class="range w-full"
              type="range"
              min="0"
              max="100"
              step="1"
              :value="viz.settings.glow"
              :style="rangeStyle(viz.settings.glow, 0, 100)"
              @input="viz.patch({ glow: Number(($event.target as HTMLInputElement).value) })"
            />
          </div>

          <div class="setting-block">
            <div class="flex items-center justify-between">
              <span class="setting-label !mb-0">{{ t("vis.animation") }}</span>
              <span class="mono text-[10.5px] text-dim">{{ viz.settings.animationMs }} ms</span>
            </div>
            <input
              class="range w-full"
              type="range"
              min="40"
              max="400"
              step="10"
              :value="viz.settings.animationMs"
              :style="rangeStyle(viz.settings.animationMs, 40, 400)"
              @input="viz.patch({ animationMs: Number(($event.target as HTMLInputElement).value) })"
            />
          </div>

          <div class="setting-block">
            <div class="flex items-center justify-between">
              <span class="setting-label !mb-0">{{ t("vis.historyLimit") }}</span>
              <span class="mono text-[10.5px] text-dim">{{ viz.settings.historyLimit }}</span>
            </div>
            <input
              class="range w-full"
              type="range"
              min="20"
              max="500"
              step="20"
              :value="viz.settings.historyLimit"
              :style="rangeStyle(viz.settings.historyLimit, 20, 500)"
              @input="viz.patch({ historyLimit: Number(($event.target as HTMLInputElement).value) })"
            />
            <label class="flex items-center gap-2 mt-1 text-[11px] text-dim cursor-pointer select-none">
              <input
                type="checkbox"
                :checked="viz.settings.showHistory"
                class="w-3.5 h-3.5 accent-[var(--color-accent)]"
                @change="viz.patch({ showHistory: ($event.target as HTMLInputElement).checked })"
              />
              {{ t("vis.showRecent") }}
            </label>
          </div>
        </div>

        <div class="card px-4 py-3.5">
          <div class="section-title">{{ t("vis.overlaySettings") }}</div>

          <div class="setting-block">
            <div class="setting-label">{{ t("vis.overlaySize") }}</div>
            <div class="seg">
              <button
                v-for="size in (['s', 'm', 'l'] as const)"
                :key="size"
                class="seg-btn"
                :class="{ active: viz.settings.overlaySize === size }"
                @click="viz.patchOverlay({ overlaySize: size })"
              >
                {{ size.toUpperCase() }}
              </button>
            </div>
          </div>

          <div class="setting-block">
            <div class="setting-label">{{ t("vis.overlayPos") }}</div>
            <div class="seg">
              <button
                class="seg-btn"
                :class="{ active: viz.settings.overlayPosition === 'top' }"
                @click="viz.patchOverlay({ overlayPosition: 'top' })"
              >
                {{ t("vis.positionTop") }}
              </button>
              <button
                class="seg-btn"
                :class="{ active: viz.settings.overlayPosition === 'bottom' }"
                @click="viz.patchOverlay({ overlayPosition: 'bottom' })"
              >
                {{ t("vis.positionBottom") }}
              </button>
            </div>
          </div>

          <div class="setting-block">
            <div class="flex items-center justify-between">
              <span class="setting-label !mb-0">{{ t("vis.overlayOpacity") }}</span>
              <span class="mono text-[10.5px] text-dim">{{ Math.round(viz.settings.overlayOpacity * 100) }}%</span>
            </div>
            <input
              class="range w-full"
              type="range"
              min="0.4"
              max="1"
              step="0.02"
              :value="viz.settings.overlayOpacity"
              :style="rangeStyle(viz.settings.overlayOpacity, 0.4, 1)"
              @input="viz.patchOverlay({ overlayOpacity: Number(($event.target as HTMLInputElement).value) })"
            />
          </div>

          <button class="btn w-full mt-1" @click="viz.overlayVisible ? viz.hideOverlay(false) : viz.showOverlay()">
            <Monitor v-if="!viz.overlayVisible" :size="14" />
            <MonitorOff v-else :size="14" />
            {{ viz.overlayVisible ? t("vis.overlayHide") : t("vis.overlay") }}
          </button>
        </div>

        <div class="card px-4 py-3.5">
          <div class="section-title">{{ t("vis.hintsTitle") }}</div>
          <ul class="space-y-1.5 text-[10.5px] text-dim leading-relaxed">
            <li class="flex gap-1.5"><Gauge :size="12" class="shrink-0 mt-0.5 text-accent-3" />{{ t("vis.hintRate") }}</li>
            <li class="flex gap-1.5"><Flame :size="12" class="shrink-0 mt-0.5 text-accent" />{{ t("vis.hintHeat") }}</li>
            <li class="flex gap-1.5"><BarChart3 :size="12" class="shrink-0 mt-0.5 text-accent-2" />{{ t("vis.hintCount") }}</li>
            <li class="flex gap-1.5"><Type :size="12" class="shrink-0 mt-0.5 text-ok" />{{ t("vis.hintTyping") }}</li>
            <li class="flex gap-1.5"><Gamepad2 :size="12" class="shrink-0 mt-0.5 text-warn" />{{ t("vis.hintGaming") }}</li>
            <li class="flex gap-1.5"><History :size="12" class="shrink-0 mt-0.5 text-accent-3" />{{ t("vis.hintHistory") }}</li>
          </ul>
        </div>
      </aside>
    </div>
  </div>
</template>

<style scoped>
.src-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 26px;
  padding: 0 9px;
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.06);
  background: rgba(255, 255, 255, 0.025);
  font-size: 10.5px;
  color: color-mix(in srgb, var(--color-text) 70%, transparent);
  transition: border-color 140ms ease, background-color 140ms ease, color 140ms ease;
}

.src-chip:hover {
  color: var(--color-text);
  background: rgba(255, 255, 255, 0.045);
}

.src-chip.active {
  border-color: color-mix(in srgb, var(--color-accent) 40%, transparent);
  background: color-mix(in srgb, var(--color-accent) 10%, transparent);
  color: var(--color-text);
}

.kbd-chip {
  display: inline-flex;
  align-items: center;
  height: 22px;
  padding: 0 7px;
  border-radius: 6px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.035);
  font-size: 10.5px;
  color: var(--color-muted);
}

.stat-box {
  border-radius: 12px;
  border: 1px solid rgba(255, 255, 255, 0.05);
  background: rgba(255, 255, 255, 0.025);
  padding: 8px 10px;
  min-width: 0;
}

.stat-label {
  font-size: 9.5px;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--color-dim);
}

.stat-value {
  margin-top: 2px;
  font-family: var(--font-display);
  font-size: 17px;
  font-weight: 600;
  line-height: 1.15;
  font-variant-numeric: tabular-nums;
}

.hist-table {
  border-radius: 12px;
  border: 1px solid rgba(255, 255, 255, 0.05);
  overflow: hidden;
}

.hist-head,
.hist-row {
  display: grid;
  grid-template-columns: 82px 1fr 1.4fr 90px 64px;
  gap: 10px;
  align-items: center;
  padding: 6px 10px;
  font-size: 11px;
}

.hist-head {
  background: rgba(255, 255, 255, 0.03);
  color: var(--color-dim);
  font-size: 9.5px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.hist-row + .hist-row {
  border-top: 1px solid rgba(255, 255, 255, 0.04);
}

.hist-row:hover {
  background: rgba(255, 255, 255, 0.025);
}

.typing-box {
  display: flex;
  flex-wrap: wrap;
  gap: 2px 1px;
  min-height: 68px;
  padding: 12px;
  border-radius: 12px;
  border: 1px solid rgba(255, 255, 255, 0.055);
  background: rgba(8, 6, 13, 0.55);
  font-family: var(--font-mono);
  font-size: 16px;
  line-height: 1.7;
}

.t-char {
  color: color-mix(in srgb, var(--color-text) 55%, transparent);
  border-radius: 3px;
  padding: 0 1px;
}

.t-char.t-ok {
  color: var(--color-ok);
}

.t-char.t-bad {
  color: var(--color-bad);
  background: rgba(255, 107, 129, 0.14);
  text-decoration: underline;
}

.t-char.t-cur {
  background: color-mix(in srgb, var(--color-accent) 22%, transparent);
  color: var(--color-text);
  box-shadow: inset 0 -2px 0 var(--color-accent);
}

.section-title {
  font-size: 10.5px;
  letter-spacing: 0.13em;
  text-transform: uppercase;
  color: var(--color-dim);
  margin-bottom: 10px;
}

.setting-block {
  margin-top: 12px;
}

.setting-label {
  display: block;
  margin-bottom: 6px;
  font-size: 11px;
  color: var(--color-dim);
}

.theme-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  height: 27px;
  padding: 0 7px;
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.06);
  background: rgba(255, 255, 255, 0.025);
  font-size: 10px;
  color: color-mix(in srgb, var(--color-text) 72%, transparent);
  transition: border-color 140ms ease, background-color 140ms ease;
}

.theme-btn:hover {
  background: rgba(255, 255, 255, 0.05);
}

.theme-btn.active {
  border-color: color-mix(in srgb, var(--color-accent) 45%, transparent);
  background: color-mix(in srgb, var(--color-accent) 10%, transparent);
}

.color-row {
  display: flex;
  align-items: center;
  gap: 7px;
  font-size: 10.5px;
  color: var(--color-dim);
  cursor: pointer;
}

.color-row :deep(.color-swatch) {
  width: 28px;
  height: 28px;
  border-radius: 8px;
}

.seg {
  display: flex;
  gap: 1px;
  padding: 2px;
  border-radius: 9px;
  border: 1px solid rgba(255, 255, 255, 0.06);
  background: rgba(255, 255, 255, 0.02);
}

.seg-btn {
  flex: 1;
  height: 25px;
  border-radius: 7px;
  font-size: 10.5px;
  color: var(--color-dim);
  transition: background-color 140ms ease, color 140ms ease;
}

.seg-btn:hover {
  color: var(--color-text);
}

.seg-btn.active {
  background: rgba(255, 255, 255, 0.07);
  color: var(--color-text);
}
</style>
