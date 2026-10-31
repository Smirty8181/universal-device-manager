<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { MousePointerClick, Play, Square } from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import Segmented from "../components/Segmented.vue";
import { api, isTauri, listen } from "../lib/api";
import { useI18n } from "../lib/i18n";
import type { AutoClickerButton, AutoClickerConfig, AutoClickerMode, AutoClickerStatus } from "../lib/types";

const { t } = useI18n();
const STORAGE_KEY = "udm.autoclicker";

const defaults: AutoClickerConfig = {
  button: "left",
  mode: "single",
  intervalMs: 100,
  jitterMs: 0,
  limit: 0,
  durationMs: 0,
  delayMs: 0,
  holdMs: 60,
};

function loadConfig(): AutoClickerConfig {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...defaults };
    return { ...defaults, ...(JSON.parse(raw) as Partial<AutoClickerConfig>) };
  } catch {
    return { ...defaults };
  }
}

const config = ref<AutoClickerConfig>(loadConfig());
const status = ref<AutoClickerStatus>({ running: false, clicks: 0, elapsedMs: 0, config: null });
const error = ref("");
const busy = ref(false);
const now = ref(Date.now());
const lastStateAt = ref(0);
const preset = ref<string>("normal");

const buttonOptions = computed(() => [
  { value: "left" as AutoClickerButton, label: t("ac.left") },
  { value: "right" as AutoClickerButton, label: t("ac.right") },
  { value: "middle" as AutoClickerButton, label: t("ac.middle") },
]);

const modeOptions = computed(() => [
  { value: "single" as AutoClickerMode, label: t("ac.single") },
  { value: "double" as AutoClickerMode, label: t("ac.double") },
  { value: "hold" as AutoClickerMode, label: t("ac.hold") },
]);

const presets = computed(() => [
  { id: "slow", label: t("ac.presetSlow"), ms: 500 },
  { id: "normal", label: t("ac.presetNormal"), ms: 200 },
  { id: "fast", label: t("ac.presetFast"), ms: 100 },
  { id: "veryfast", label: t("ac.presetVeryFast"), ms: 25 },
  { id: "custom", label: t("ac.presetCustom"), ms: 0 },
]);

const perSecond = computed(() => {
  if (config.value.mode !== "single") return null;
  const base = Math.max(1, config.value.intervalMs);
  return (1000 / base).toFixed(base <= 50 ? 1 : 2);
});

const rate = computed(() => {
  const ms = Math.max(1, config.value.intervalMs);
  const perClick = config.value.mode === "double" ? 2 : 1;
  return Math.round((60000 / ms) * perClick);
});

const elapsed = computed(() => {
  if (!status.value.running) return status.value.elapsedMs;
  const since = Math.max(0, now.value - lastStateAt.value);
  return status.value.elapsedMs + since;
});

const buttonLabel = computed(() => {
  const found = buttonOptions.value.find((b) => b.value === config.value.button);
  return found?.label ?? "";
});

function formatElapsed(ms: number) {
  const total = Math.floor(ms / 1000);
  const minutes = Math.floor(total / 60);
  const seconds = total % 60;
  return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}

function persist() {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(config.value));
  } catch {}
}

function setPreset(ms: number, id: string) {
  preset.value = id;
  if (ms > 0) {
    config.value.intervalMs = ms;
    persist();
  }
}

function applyStatus(next: AutoClickerStatus) {
  status.value = next;
  lastStateAt.value = Date.now();
  now.value = Date.now();
  if (next.config) config.value = { ...next.config };
}

async function start() {
  if (busy.value) return;
  error.value = "";
  busy.value = true;
  try {
    applyStatus(await api.autoclickerStart({ ...config.value }));
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}

async function stop() {
  if (busy.value) return;
  busy.value = true;
  try {
    applyStatus(await api.autoclickerStop());
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}

const offs: (() => void)[] = [];
let ticker = 0;

onMounted(async () => {
  if (isTauri()) {
    offs.push(await listen<AutoClickerStatus>("autoclicker:state", applyStatus));
    try {
      applyStatus(await api.autoclickerStatus());
    } catch {}
  }
  ticker = window.setInterval(() => {
    now.value = Date.now();
  }, 250);
});

onBeforeUnmount(() => {
  for (const off of offs.splice(0)) off();
  window.clearInterval(ticker);
  if (status.value.running) void api.autoclickerStop();
});
</script>

<template>
  <div>
    <PageHeader :title="t('nav.autoclicker')" :subtitle="t('ac.subtitle')" :overline="t('ac.overline')">
      <template #actions>
        <span class="chip" :class="status.running ? 'chip-ok' : 'chip-bad'">
          <span class="status-dot" :class="status.running ? 'bg-ok animate-pulse' : 'bg-bad'"></span>
          {{ status.running ? t("ac.running") : t("ac.stopped") }}
        </span>
      </template>
    </PageHeader>

    <div v-if="error" class="card px-4 py-3 mb-4 border-bad/25 bg-bad/[0.05] text-[12px] text-bad">
      {{ t("ac.error") }}: {{ error }}
    </div>

    <div class="grid gap-4 lg:grid-cols-[minmax(0,1fr)_330px]">
      <section class="card p-5 relative overflow-hidden self-start">
        <div class="absolute -right-16 -top-20 h-56 w-56 rounded-full bg-accent/10 blur-3xl pointer-events-none"></div>
        <div class="flex items-start gap-4 relative">
          <button
            class="ac-power shrink-0"
            :class="{ on: status.running }"
            :disabled="busy"
            @click="status.running ? stop() : start()"
          >
            <Square v-if="status.running" :size="22" />
            <Play v-else :size="22" />
          </button>
          <div class="min-w-0">
            <div class="text-[10px] uppercase tracking-[0.16em] text-dim">{{ t("ac.button") }}</div>
            <div class="display text-[26px] leading-tight">{{ buttonLabel }}</div>
            <div class="text-[11.5px] text-muted mt-1">
              {{ t("ac.interval") }} {{ config.intervalMs }} ms
              <template v-if="config.jitterMs">± {{ config.jitterMs }}</template>
              · {{ Math.round(rate).toLocaleString("ru-RU") }} {{ t("ac.rateUnit") }}
            </div>
            <div class="text-[11px] text-dim mt-0.5">{{ t("ac.stopHint") }}</div>
          </div>
          <button
            class="ml-auto h-9 px-4 rounded-xl text-[12px] font-semibold transition-colors shrink-0"
            :class="status.running ? 'bg-bad/15 text-bad border border-bad/30 hover:bg-bad/20' : 'bg-accent text-white hover:bg-accent/90'"
            :disabled="busy"
            @click="status.running ? stop() : start()"
          >
            {{ status.running ? t("ac.stop") : t("ac.start") }}
          </button>
        </div>

        <div class="grid grid-cols-2 sm:grid-cols-4 gap-2.5 mt-5 relative">
          <div class="ac-stat">
            <span>{{ t("ac.clicks") }}</span>
            <strong class="mono">{{ status.clicks.toLocaleString("ru-RU") }}</strong>
          </div>
          <div class="ac-stat">
            <span>{{ t("ac.elapsed") }}</span>
            <strong class="mono">{{ formatElapsed(elapsed) }}</strong>
          </div>
          <div class="ac-stat">
            <span>{{ t("ac.rate") }}</span>
            <strong class="mono">{{ rate.toLocaleString("ru-RU") }}</strong>
          </div>
          <div class="ac-stat">
            <span>{{ t("ac.mode") }}</span>
            <strong>{{ modeOptions.find((m) => m.value === config.mode)?.label }}</strong>
          </div>
        </div>

        <p class="text-[11px] text-dim mt-4 leading-relaxed relative max-w-[62ch]">{{ t("ac.modeHint") }}</p>

        <div class="mt-4 relative">
          <div class="ac-label">{{ t("ac.presets") }}</div>
          <div class="flex flex-wrap gap-1.5">
            <button
              v-for="item in presets"
              :key="item.id"
              class="ac-preset"
              :class="{ active: preset === item.id }"
              :disabled="status.running"
              @click="setPreset(item.ms, item.id)"
            >
              {{ item.label }}
              <span v-if="item.ms" class="text-dim ml-1">{{ item.ms }}ms</span>
            </button>
          </div>
        </div>
      </section>

      <aside class="card p-4 space-y-4 h-fit">
        <div>
          <div class="ac-label">{{ t("ac.button") }}</div>
          <Segmented v-model="config.button" :options="buttonOptions" :disabled="status.running" />
        </div>

        <div>
          <div class="ac-label">{{ t("ac.mode") }}</div>
          <Segmented v-model="config.mode" :options="modeOptions" :disabled="status.running" />
        </div>

        <label class="ac-field">
          <span>{{ t("ac.interval") }}, ms</span>
          <input v-model.number="config.intervalMs" class="input mono" type="number" min="1" max="60000" :disabled="status.running" @change="preset = 'custom'" />
        </label>

        <label class="ac-field">
          <span>{{ t("ac.jitter") }}, ± ms</span>
          <input v-model.number="config.jitterMs" class="input mono" type="number" min="0" max="5000" :disabled="status.running" />
        </label>

        <label class="ac-field">
          <span>{{ t("ac.holdTime") }}</span>
          <input v-model.number="config.holdMs" class="input mono" type="number" min="1" max="10000" :disabled="status.running || config.mode !== 'hold'" />
        </label>

        <div class="grid grid-cols-2 gap-2">
          <label class="ac-field">
            <span>{{ t("ac.limit") }}</span>
            <input v-model.number="config.limit" class="input mono" type="number" min="0" :disabled="status.running" />
          </label>
          <label class="ac-field">
            <span>{{ t("ac.duration") }}</span>
            <input
              :value="Math.round(config.durationMs / 1000)"
              class="input mono"
              type="number"
              min="0"
              :disabled="status.running"
              @input="config.durationMs = Math.max(0, Number(($event.target as HTMLInputElement).value)) * 1000"
            />
          </label>
        </div>

        <label class="ac-field">
          <span>{{ t("ac.delay") }}</span>
          <input
            :value="Math.round(config.delayMs / 1000)"
            class="input mono"
            type="number"
            min="0"
            max="60"
            :disabled="status.running"
            @input="config.delayMs = Math.max(0, Number(($event.target as HTMLInputElement).value)) * 1000"
          />
        </label>

        <p class="text-[10.5px] text-dim">{{ t("ac.settingsKeep") }} · {{ t("ac.limitHint") }}</p>
        <div class="chip !text-[10.5px] w-fit"><MousePointerClick :size="11" /> {{ perSecond ? `${perSecond} ${t("ac.clicksUnit")}` : t("ac.rateHint") }}</div>
      </aside>
    </div>
  </div>
</template>

<style scoped>
.ac-power {
  display: grid;
  place-items: center;
  width: 62px;
  height: 62px;
  border-radius: 18px;
  border: 1px solid color-mix(in srgb, var(--color-accent) 34%, transparent);
  background: color-mix(in srgb, var(--color-accent) 12%, transparent);
  color: var(--color-text);
  transition: transform 140ms ease, background-color 140ms ease;
}
.ac-power:hover {
  transform: translateY(-1px);
  background: color-mix(in srgb, var(--color-accent) 20%, transparent);
}
.ac-power.on {
  border-color: color-mix(in srgb, var(--color-bad) 40%, transparent);
  background: color-mix(in srgb, var(--color-bad) 14%, transparent);
}
.ac-stat {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 9px 11px;
  border-radius: 12px;
  border: 1px solid rgba(255, 255, 255, 0.06);
  background: rgba(255, 255, 255, 0.025);
}
.ac-stat span {
  font-size: 9.5px;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  color: var(--color-dim);
}
.ac-stat strong {
  font-size: 16px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}
.ac-label {
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.14em;
  color: var(--color-dim);
  margin-bottom: 7px;
}
.ac-preset {
  display: inline-flex;
  align-items: center;
  height: 26px;
  padding: 0 9px;
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.07);
  background: rgba(255, 255, 255, 0.03);
  font-size: 11px;
  color: var(--color-muted);
  transition: border-color 120ms ease, color 120ms ease;
}
.ac-preset:hover {
  color: var(--color-text);
}
.ac-preset.active {
  border-color: color-mix(in srgb, var(--color-accent) 45%, transparent);
  background: color-mix(in srgb, var(--color-accent) 12%, transparent);
  color: var(--color-text);
}
.ac-preset:disabled {
  opacity: 0.55;
}
.ac-field {
  display: flex;
  flex-direction: column;
  gap: 5px;
  font-size: 11px;
  color: var(--color-muted);
}
.ac-field .input {
  height: 32px;
  font-size: 12px;
}
</style>
