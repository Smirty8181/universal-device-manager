<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { BatteryCharging, Loader2, Lock, Plus, Save, Trash2 } from "lucide-vue-next";
import type { BatteryRgbConfig, BatteryRgbRange, BatteryRgbView, DeviceView } from "../lib/types";
import { api, isTauri } from "../lib/api";
import { BATTERY_RGB_TEMPLATES, cloneRanges, colorForPercent, rangesCoverAll } from "../lib/batteryRgb";
import { useI18n } from "../lib/i18n";
import { useAppStore } from "../stores/app";
import { useDevicesStore } from "../stores/devices";
import Toggle from "./Toggle.vue";

const props = defineProps<{ device: DeviceView }>();
const { t, locale } = useI18n();
const app = useAppStore();
const devices = useDevicesStore();

const view = ref<BatteryRgbView | null>(null);
const config = ref<BatteryRgbConfig | null>(null);
const loading = ref(false);
const saving = ref(false);
const toggling = ref(false);
const probing = ref(false);
const error = ref<string | null>(null);

const available = computed(() => view.value?.available === true);
const active = computed(() => view.value?.config.enabled === true);
const percent = computed(() => view.value?.percent ?? null);
const reason = computed(() =>
  locale.value === "ru" ? view.value?.reasonRu : view.value?.reasonEn
);
const statusLabel = computed(() => {
  if (!view.value) return t("common.noData");
  if (!available.value) return t("batteryRgb.locked");
  return active.value ? t("batteryRgb.active") : t("batteryRgb.inactive");
});

const ranges = computed(() => config.value?.ranges ?? []);
const coverage = computed(() => rangesCoverAll(ranges.value));
const currentColor = computed(() =>
  percent.value === null ? null : colorForPercent(ranges.value, percent.value)
);
const dirty = computed(() => {
  if (!view.value || !config.value) return false;
  return JSON.stringify(config.value.ranges) !== JSON.stringify(view.value.config.ranges);
});
const canEdit = computed(() => Boolean(config.value) && !saving.value && !toggling.value);
const toggleDisabled = computed(
  () => toggling.value || !view.value || (!active.value && !available.value)
);

function applyView(next: BatteryRgbView) {
  view.value = next;
  config.value = {
    ...next.config,
    ranges: cloneRanges(next.config.ranges),
  };
}

async function load() {
  if (!isTauri()) return;
  loading.value = true;
  error.value = null;
  try {
    applyView(await api.batteryRgbGet(props.device.id));
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function probe() {
  probing.value = true;
  try {
    await devices.probeBattery(props.device.id);
    await load();
  } catch (e) {
    app.toast("error", t("toast.error"), String(e));
  } finally {
    probing.value = false;
  }
}

function useTemplate(id: string) {
  const template = BATTERY_RGB_TEMPLATES.find((item) => item.id === id);
  if (!template || !config.value) return;
  config.value.ranges = cloneRanges(template.ranges);
  config.value.chargingColor = template.chargingColor;
}

function addRange() {
  if (!config.value) return;
  const list = [...config.value.ranges].sort((a, b) => a.min - b.min);
  const index = list.findIndex((item) => item.max - item.min >= 10);
  if (index === -1) return;
  const item = list[index];
  const split = Math.floor((item.min + item.max) / 2);
  const left = { ...item, max: split };
  const right = { ...item, min: split + 1 };
  list.splice(index, 1, left, right);
  config.value.ranges = list;
}

function removeRange(index: number) {
  if (!config.value || config.value.ranges.length <= 1) return;
  const list = [...config.value.ranges].sort((a, b) => a.min - b.min);
  if (index === 0) {
    list[1].min = 0;
  } else if (index === list.length - 1) {
    list[index - 1].max = 100;
  } else {
    list[index - 1].max = list[index + 1].min - 1;
  }
  list.splice(index, 1);
  config.value.ranges = list;
}

function setBound(range: BatteryRgbRange, key: "min" | "max", value: string) {
  const parsed = Math.max(0, Math.min(100, Math.round(Number(value) || 0)));
  range[key] = parsed;
}

async function save() {
  if (!config.value || saving.value) return false;
  if (!coverage.value) {
    app.toast("warn", t("batteryRgb.coverageError"));
    return false;
  }
  saving.value = true;
  try {
    applyView(await api.batteryRgbSave(props.device.id, config.value));
    app.toast("success", t("batteryRgb.title"), t("batteryRgb.saved"));
    return true;
  } catch (e) {
    app.toast("error", t("toast.error"), String(e));
    return false;
  } finally {
    saving.value = false;
  }
}

async function toggle(enabled: boolean) {
  if (toggling.value) return;
  toggling.value = true;
  try {
    if (enabled) {
      const saved = await save();
      if (!saved) return;
      const result = await api.batteryRgbEnable(props.device.id, true);
      applyView(result.view);
      app.toast(result.ok ? "success" : "warn", t("batteryRgb.title"), result.message);
      if (result.ok) {
        app.addActivity("device", `${props.device.name}: ${t("batteryRgb.active")}`, "info", props.device.id);
      }
    } else {
      const result = await api.batteryRgbEnable(props.device.id, false);
      applyView(result.view);
      app.toast(result.ok ? "success" : "warn", t("batteryRgb.title"), result.message);
    }
  } catch (e) {
    app.toast("error", t("toast.error"), String(e));
  } finally {
    toggling.value = false;
  }
}

onMounted(load);
watch(() => props.device.id, load);
</script>

<template>
  <div class="panel p-5">
    <div class="flex items-start justify-between gap-3">
      <div>
        <div class="section-title flex items-center gap-2">
          <BatteryCharging :size="15" class="text-accent" />
          {{ t("batteryRgb.title") }}
        </div>
        <div class="text-[10.5px] text-dim leading-relaxed mt-1 max-w-[360px]">{{ t("batteryRgb.subtitle") }}</div>
      </div>
      <div class="flex items-center gap-2.5 shrink-0">
        <span class="chip !h-6" :class="active && available ? 'chip-ok' : ''">
          <span class="status-dot" :class="active && available ? 'bg-ok' : 'bg-dim'"></span>
          {{ statusLabel }}
        </span>
        <Toggle :model-value="active" :disabled="toggleDisabled" @update:model-value="toggle" />
      </div>
    </div>

    <div v-if="loading" class="py-6 grid place-items-center text-dim">
      <Loader2 :size="16" class="animate-spin" />
    </div>

    <template v-else-if="view && config">
      <div class="flex items-center gap-3 mt-3">
        <span
          class="w-8 h-8 rounded-full border border-white/10 shrink-0"
          :style="{ background: currentColor ?? 'transparent', boxShadow: currentColor ? `0 0 18px ${currentColor}55` : 'none' }"
        ></span>
        <div class="min-w-0">
          <div class="text-[11.5px] text-muted">
            {{ t("batteryRgb.current") }}:
            <span class="mono">{{ percent !== null ? `${percent}%` : t("battery.unknown") }}</span>
            <span v-if="view.batteryStatus" class="text-dim"> · {{ t("battery.status." + view.batteryStatus) }}</span>
          </div>
          <div class="text-[10px] text-dim">
            {{ active ? t("batteryRgb.restoreHint") : t("batteryRgb.enableHint") }}
          </div>
        </div>
      </div>

      <div v-if="!available" class="card-flat px-3 py-2.5 mt-3 flex items-start gap-2.5">
        <Lock :size="13" class="text-warn shrink-0 mt-0.5" />
        <div class="min-w-0">
          <div class="text-[11px] text-muted">{{ reason || t("batteryRgb.lockedBody") }}</div>
          <button class="btn btn-ghost !h-7 !px-2 !text-[10.5px] mt-1.5" :disabled="probing" @click="probe">
            <Loader2 v-if="probing" :size="11" class="animate-spin" />
            <BatteryCharging v-else :size="11" />
            {{ t("batteryRgb.checkBattery") }}
          </button>
        </div>
      </div>
      <div v-if="error" class="text-[10.5px] text-warn mt-2 break-words">{{ error }}</div>

      <div class="divider my-4"></div>

      <div class="text-[11.5px] text-muted mb-2">{{ t("batteryRgb.templates") }}</div>
      <div class="flex flex-wrap gap-1.5">
        <button
          v-for="template in BATTERY_RGB_TEMPLATES"
          :key="template.id"
          class="h-7 px-2.5 rounded-lg text-[11px] border transition-colors"
          :class="JSON.stringify(ranges) === JSON.stringify(template.ranges) ? 'border-accent/50 bg-accent/10 text-text' : 'border-white/8 text-dim hover:text-muted'"
          :disabled="!canEdit"
          @click="useTemplate(template.id)"
        >
          {{ template.name }}
        </button>
      </div>

      <div class="flex items-center justify-between mt-4 mb-1.5">
        <div class="text-[11.5px] text-muted">{{ t("batteryRgb.ranges") }}</div>
        <button class="btn btn-ghost !h-7 !px-2 !text-[10.5px]" :disabled="!canEdit" @click="addRange">
          <Plus :size="12" />
          {{ t("batteryRgb.addRange") }}
        </button>
      </div>

      <div class="space-y-1.5">
        <div
          v-for="(range, index) in ranges"
          :key="index"
          class="grid grid-cols-[44px_10px_44px_1fr_78px_26px] items-center gap-1.5 px-2 py-1.5 rounded-lg border transition-colors"
          :class="percent !== null && percent >= range.min && percent <= range.max ? 'border-accent/40 bg-accent/[0.06]' : 'border-white/[0.05]'"
        >
          <input :value="range.min" type="number" min="0" max="100" class="input !h-7 !px-1.5 mono !text-[11px]" :disabled="!canEdit" @input="setBound(range, 'min', ($event.target as HTMLInputElement).value)" />
          <span class="text-[10px] text-dim text-center">–</span>
          <input :value="range.max" type="number" min="0" max="100" class="input !h-7 !px-1.5 mono !text-[11px]" :disabled="!canEdit" @input="setBound(range, 'max', ($event.target as HTMLInputElement).value)" />
          <div class="flex items-center gap-1.5 min-w-0">
            <input
              type="color"
              class="color-swatch !w-7 !h-7 shrink-0"
              :value="range.color"
              :disabled="!canEdit"
              @input="range.color = ($event.target as HTMLInputElement).value"
            />
            <span class="text-[10px] mono uppercase text-dim truncate">{{ range.color }}</span>
          </div>
          <select v-model.number="range.brightness" class="input !h-7 !px-1 !text-[10.5px]" :disabled="!canEdit">
            <option :value="25">25%</option>
            <option :value="50">50%</option>
            <option :value="75">75%</option>
            <option :value="100">100%</option>
          </select>
          <button class="icon-btn !w-6 !h-6" :title="t('batteryRgb.removeRange')" :disabled="!canEdit || ranges.length <= 1" @click="removeRange(index)">
            <Trash2 :size="11" />
          </button>
        </div>
      </div>
      <div v-if="!coverage" class="text-[10.5px] text-warn mt-2">{{ t("batteryRgb.coverageError") }}</div>

      <div class="flex items-center gap-3 mt-4">
        <span class="text-[11.5px] text-muted shrink-0">{{ t("batteryRgb.chargingColor") }}</span>
        <input
          type="color"
          class="color-swatch !w-7 !h-7"
          :value="config.chargingColor ?? '#22d3ee'"
          :disabled="!canEdit"
          @input="config.chargingColor = ($event.target as HTMLInputElement).value"
        />
        <span class="text-[10px] text-dim">{{ t("batteryRgb.chargingHint") }}</span>
      </div>

      <div class="text-[10px] text-dim leading-relaxed mt-3">{{ t("batteryRgb.blinkUnsupported") }}</div>
      <div class="text-[10px] text-dim leading-relaxed mt-1">{{ t("batteryRgb.unsupportedLayout") }}</div>
      <div class="text-[10px] text-dim leading-relaxed mt-1">{{ t("batteryRgb.levelsNote") }}</div>

      <button class="btn w-full mt-3 !h-8 !text-[11.5px]" :disabled="!canEdit || !coverage" @click="save">
        <Loader2 v-if="saving" :size="12" class="animate-spin" />
        <Save v-else :size="12" />
        {{ t("batteryRgb.save") }}
      </button>
      <div v-if="dirty" class="text-[10px] text-dim mt-1.5">{{ t("batteryRgb.dirty") }}</div>
    </template>

    <div v-else-if="error" class="text-[10.5px] text-warn mt-3 break-words">{{ error }}</div>
  </div>
</template>
