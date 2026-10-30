<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { Play, Square, X } from "lucide-vue-next";
import { api, emitEvent, isTauri, listen } from "../lib/api";
import { useI18n } from "../lib/i18n";
import { loadVisualizerSettings } from "../stores/visualizer";
import { chordLabel, keyLabel, resolveKeyId } from "../lib/keyboard";
import type { LiveKeyEvent, LiveStatus, VisualizerSettings } from "../lib/types";

const { t } = useI18n();
const settings = ref<VisualizerSettings>(loadVisualizerSettings());
const active = ref(false);
const held = ref<Record<string, string>>({});
const recent = ref<string[]>([]);
const sourceLabel = ref("");

const heldIds = computed(() => {
  const set = new Set<string>();
  for (const id of Object.values(held.value)) set.add(id);
  return [...set];
});
const chord = computed(() => {
  if (heldIds.value.length) return chordLabel(heldIds.value);
  return recent.value[0] ? keyLabel(recent.value[0], "en") : null;
});
const compact = computed(() => (chord.value?.length ?? 0) > 16);

const cardStyle = computed(() => ({
  background: `rgba(10, 8, 17, ${settings.value.overlayOpacity})`,
  borderColor: `color-mix(in srgb, ${settings.value.accent} 26%, rgba(255,255,255,0.1))`,
}));

const accentStyle = computed(() => ({
  "--ov-accent": settings.value.accent,
  "--ov-held": settings.value.heldColor,
}));

const offs: (() => void)[] = [];

function clear() {
  held.value = {};
  recent.value = [];
  sourceLabel.value = "";
}

function onEvent(event: LiveKeyEvent) {
  if (!active.value) return;
  const key = `${event.path}|${event.vk}`;
  const id = resolveKeyId(event.vk, event.extended, event.scan);
  sourceLabel.value = event.label;
  if (event.down) {
    if (!event.repeat) {
      if (id) {
        recent.value = [id, ...recent.value.filter((item) => item !== id)].slice(0, 5);
      }
      if (id) held.value = { ...held.value, [key]: id };
    } else if (id) {
      held.value = { ...held.value, [key]: id };
    }
  } else {
    const next = { ...held.value };
    delete next[key];
    held.value = next;
  }
}

async function refresh() {
  settings.value = loadVisualizerSettings();
  if (!isTauri()) return;
  try {
    active.value = (await api.liveInputStatus()).active;
  } catch {}
}

async function stop() {
  try {
    await api.liveInputSet(false);
  } catch {}
  active.value = false;
  clear();
  try {
    await api.overlayHide(true);
  } catch {}
  void emitEvent("visualizer:overlay", false);
}

async function start() {
  try {
    const status = await api.liveInputSet(true);
    active.value = status.active;
  } catch {}
}

async function close() {
  active.value = false;
  clear();
  try {
    await api.overlayHide(true);
  } catch {}
  void emitEvent("visualizer:overlay", false);
}

onMounted(async () => {
  document.documentElement.classList.add("overlay-mode");
  if (isTauri()) {
    offs.push(await listen<LiveKeyEvent>("keyboard:event", onEvent));
    offs.push(
      await listen<LiveStatus>("keyboard:live", (status) => {
        active.value = status.active;
        if (!status.active) clear();
      })
    );
    offs.push(
      await listen<VisualizerSettings>("visualizer:settings", (next) => {
        settings.value = next;
      })
    );
    offs.push(await listen("overlay:refresh", refresh));
  }
  await refresh();
  if (!active.value) await start();
});

onBeforeUnmount(() => {
  document.documentElement.classList.remove("overlay-mode");
  for (const off of offs.splice(0)) off();
});
</script>

<template>
  <div class="overlay-root h-screen w-screen p-[6px] select-none" :style="accentStyle">
    <section class="overlay-card h-full flex flex-col overflow-hidden rounded-[16px] border shadow-[0_18px_50px_-16px_rgba(0,0,0,0.9)] backdrop-blur-xl" :style="cardStyle">
      <header
        data-tauri-drag-region
        class="flex items-center gap-2 h-[26px] shrink-0 px-2.5 border-b border-white/[0.06]"
      >
        <span class="w-1.5 h-1.5 rounded-full shrink-0" :class="active ? 'bg-ok animate-pulse' : 'bg-white/30'"></span>
        <span class="text-[9px] font-semibold tracking-[0.16em] uppercase" :class="active ? 'text-ok' : 'text-dim'">
          {{ active ? t("overlay.live") : t("overlay.off") }}
        </span>
        <span class="text-[9px] text-dim/80 tracking-[0.1em] uppercase">UDM visualizer</span>
        <span v-if="sourceLabel" data-tauri-drag-region class="ml-1 truncate text-[9.5px] text-dim/80 max-w-[190px]">
          · {{ sourceLabel }}
        </span>
        <div class="ml-auto flex items-center gap-0.5">
          <button v-if="active" class="ov-btn" :title="t('overlay.stop')" @click="stop">
            <Square :size="10" />
          </button>
          <button v-else class="ov-btn" :title="t('overlay.start')" @click="start">
            <Play :size="10" />
          </button>
          <button class="ov-btn hover:!text-bad" :title="t('overlay.close')" @click="close">
            <X :size="11" />
          </button>
        </div>
      </header>

      <div class="flex-1 min-h-0 flex items-center gap-3 px-3">
        <div class="min-w-0 flex-1">
          <div v-if="active" class="overlay-chord" :class="{ compact }">
            {{ chord ?? t("overlay.noKeys") }}
          </div>
          <div v-else class="flex items-center gap-2">
            <span class="text-[11px] text-dim">{{ t("overlay.stopped") }}</span>
            <button class="ov-start" @click="start">
              <Play :size="10" />
              {{ t("overlay.start") }}
            </button>
          </div>
          <div class="mt-0.5 text-[9px] text-dim/80 truncate">
            {{ active ? (heldIds.length ? `${t("overlay.held")}: ${heldIds.length}` : t("overlay.hint")) : t("overlay.stoppedHint") }}
          </div>
        </div>

        <div v-if="active && recent.length" class="overlay-recent shrink-0 flex flex-col gap-0.5 items-end">
          <span
            v-for="(id, index) in recent.slice(0, 3)"
            :key="`${id}-${index}`"
            class="ov-key"
            :class="{ bright: index === 0 }"
          >{{ keyLabel(id, "en") }}</span>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.overlay-root {
  container-type: size;
  --ov-accent: #ff4fa3;
  --ov-held: #a78bfa;
}

.overlay-card {
  border-radius: 16px;
}

.ov-btn {
  display: grid;
  place-items: center;
  width: 18px;
  height: 18px;
  border-radius: 5px;
  color: color-mix(in srgb, var(--color-text) 60%, transparent);
  transition: color 120ms ease, background-color 120ms ease;
}

.ov-btn:hover {
  color: var(--color-text);
  background: rgba(255, 255, 255, 0.07);
}

.ov-start {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 20px;
  padding: 0 8px;
  border-radius: 6px;
  border: 1px solid color-mix(in srgb, var(--ov-accent) 35%, transparent);
  background: color-mix(in srgb, var(--ov-accent) 12%, transparent);
  font-size: 9.5px;
  color: var(--color-text);
}

.overlay-chord {
  font-family: var(--font-display);
  font-size: clamp(20px, 32cqh, 52px);
  font-weight: 650;
  letter-spacing: 0.01em;
  line-height: 1.02;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  color: color-mix(in srgb, var(--color-text) 96%, transparent);
  text-shadow: 0 0 24px color-mix(in srgb, var(--ov-accent) 45%, transparent);
}

.overlay-chord.compact {
  font-size: clamp(15px, 23cqh, 34px);
}

.ov-key {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 26px;
  height: 16px;
  padding: 0 5px;
  border-radius: 5px;
  border: 1px solid rgba(255, 255, 255, 0.09);
  background: rgba(255, 255, 255, 0.035);
  font-family: var(--font-mono);
  font-size: 8.5px;
  color: color-mix(in srgb, var(--color-text) 55%, transparent);
}

.ov-key.bright {
  border-color: color-mix(in srgb, var(--ov-accent) 45%, transparent);
  background: color-mix(in srgb, var(--ov-accent) 14%, transparent);
  color: var(--color-text);
}

@container (max-width: 520px) {
  .overlay-recent {
    display: none;
  }
}

@container (max-height: 110px) {
  .overlay-chord {
    font-size: clamp(18px, 30cqh, 30px);
  }
}
</style>
