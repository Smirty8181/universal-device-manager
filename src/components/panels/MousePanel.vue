<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { BatteryCharging, BatteryMedium, Loader2, Lock, Radio, RefreshCw } from "lucide-vue-next";
import type { DeviceView, MouseStatus } from "../../lib/types";
import { useDeviceState } from "../../lib/deviceStore";
import { useI18n } from "../../lib/i18n";
import { useAppStore } from "../../stores/app";
import { useMacrosStore } from "../../stores/macros";
import { api, isTauri } from "../../lib/api";
import { localizedError } from "../../lib/format";
import MouseModel from "../MouseModel.vue";
import Slider from "../Slider.vue";
import Modal from "../Modal.vue";
import StatusChip from "../StatusChip.vue";

const props = defineProps<{ device: DeviceView }>();
const { t, locale } = useI18n();
const app = useAppStore();
const macros = useMacrosStore();

const state = useDeviceState(props.device.id + ".mouse", {
  buttons: {} as Record<string, string>,
});

const selectedButton = ref<string | null>(null);
const assignOpen = ref(false);

const buttons = computed(() => [
  { id: "left", label: t("mouse.left") },
  { id: "right", label: t("mouse.right") },
  { id: "wheel", label: t("mouse.wheel") },
  { id: "dpi", label: t("mouse.dpiButton") },
  { id: "sideForward", label: t("mouse.sideForward") },
  { id: "sideBack", label: t("mouse.sideBack") },
]);

const mediaKeys = [
  { value: "play", label: "Play / Pause" },
  { value: "next", label: "Next" },
  { value: "prev", label: "Previous" },
  { value: "mute", label: "Mute" },
  { value: "volup", label: "Volume +" },
  { value: "voldown", label: "Volume −" },
];

const assignKind = ref<"none" | "macro" | "dpi" | "media" | "disabled">("none");
const assignMacro = ref<string>("");
const assignMedia = ref<string>("play");

const hw = ref<MouseStatus | null>(null);
const hwError = ref<string | null>(null);
const loading = ref(false);
const selectedStage = ref(0);

const verified = computed(() => props.device.adapterId === "ajazz");
const stageEdits = ref<number[]>([400, 800, 1600, 3200, 6400, 12800]);
const stagePending = ref<number | null>(null);
const pollingPending = ref<number | null>(null);

const pollingOptions = [125, 250, 500, 1000];

const batteryPercent = computed(() => hw.value?.battery ?? null);
const batteryClass = computed(() => {
  const value = batteryPercent.value;
  if (value === null) return "text-dim";
  if (value <= 20) return "text-bad";
  if (value <= 40) return "text-warn";
  return "text-ok";
});
const batteryBarClass = computed(() => {
  const value = batteryPercent.value;
  if (value === null) return "bg-white/10";
  if (value <= 20) return "bg-bad";
  if (value <= 40) return "bg-warn";
  return "bg-ok";
});
const chargingLabel = computed(() => {
  if (hw.value?.charging === true) return t("mouse.charging");
  if (hw.value?.charging === false) return t("mouse.discharging");
  return t("mouse.chargeUnknown");
});

const stageValue = computed({
  get: () => stageEdits.value[selectedStage.value] ?? 800,
  set: (value: number) => {
    stageEdits.value[selectedStage.value] = value;
  },
});

const actionLabel = (id: string) => {
  const action = state.value.buttons[id];
  if (!action || action === "none") return t("mouse.actionNone");
  if (action === "disabled") return t("mouse.actionDisabled");
  if (action === "dpi") return t("mouse.actionDpi");
  if (action.startsWith("macro:")) {
    const m = macros.macros.find((x) => x.id === action.slice(6));
    return m ? `${t("mouse.actionMacro")}: ${m.name}` : t("mouse.actionMacro");
  }
  if (action.startsWith("media:")) {
    const key = mediaKeys.find((k) => k.value === action.slice(6));
    return `${t("mouse.actionMedia")}: ${key?.label ?? action}`;
  }
  return t("mouse.actionNone");
};

function openAssign(id: string) {
  selectedButton.value = id;
  const action = state.value.buttons[id] ?? "none";
  if (action.startsWith("macro:")) {
    assignKind.value = "macro";
    assignMacro.value = action.slice(6);
  } else if (action.startsWith("media:")) {
    assignKind.value = "media";
    assignMedia.value = action.slice(6);
  } else if (action === "dpi") {
    assignKind.value = "dpi";
  } else if (action === "disabled") {
    assignKind.value = "disabled";
  } else {
    assignKind.value = "none";
  }
  assignOpen.value = true;
}

function saveAssign() {
  if (!selectedButton.value) return;
  let action = "none";
  if (assignKind.value === "macro") action = `macro:${assignMacro.value}`;
  else if (assignKind.value === "media") action = `media:${assignMedia.value}`;
  else if (assignKind.value === "dpi") action = "dpi";
  else if (assignKind.value === "disabled") action = "disabled";
  state.value.buttons = { ...state.value.buttons, [selectedButton.value]: action };
  assignOpen.value = false;
  app.toast("info", t("common.saved"), t("mouse.buttonsAppOnly"));
}

function applyStatus(status: MouseStatus) {
  hw.value = status;
  if (status.dpi.length) {
    stageEdits.value = status.dpi.slice(0, 6);
    if (selectedStage.value >= stageEdits.value.length) selectedStage.value = 0;
  }
}

async function loadStatus() {
  hw.value = null;
  hwError.value = null;
  if (!isTauri()) {
    hwError.value = t("mouse.desktopOnly");
    return;
  }
  if (!verified.value) {
    hwError.value = t("device.protocolLockedBody");
    return;
  }
  loading.value = true;
  try {
    applyStatus(await api.mouseStatus(props.device.id));
  } catch (error) {
    hwError.value = String(error);
  } finally {
    loading.value = false;
  }
}

async function applyStage() {
  if (!hw.value || stagePending.value !== null || !verified.value) return;
  const stage = selectedStage.value;
  const value = Math.round(stageValue.value / 50) * 50;
  if (value < 100 || value > 30000) {
    app.toast("warn", t("mouse.dpi"), t("mouse.dpiRangeHint"));
    return;
  }
  stageEdits.value[stage] = value;
  stagePending.value = stage;
  try {
    const result = await api.mouseSetDpi(props.device.id, stage, value);
    if (result.status) applyStatus(result.status);
    const body = `${t("mouse.stageN")} ${stage + 1} = ${value} DPI — ${result.ok ? t("mouse.stageApplied") : t("mouse.stageWriteFailed")}`;
    app.toast(result.ok ? "success" : "warn", t("mouse.title"), body);
  } catch (error) {
    app.toast("error", t("toast.error"), String(error));
  } finally {
    stagePending.value = null;
  }
}

async function applyPolling(hz: number) {
  if (!hw.value || pollingPending.value !== null || !verified.value) return;
  pollingPending.value = hz;
  try {
    const result = await api.mouseSetPolling(props.device.id, hz);
    if (result.status) applyStatus(result.status);
    const body = `${hz} Hz — ${result.ok ? t("mouse.stageApplied") : t("mouse.stageWriteFailed")}`;
    app.toast(result.ok ? "success" : "warn", t("mouse.polling"), body);
  } catch (error) {
    app.toast("error", t("toast.error"), String(error));
  } finally {
    pollingPending.value = null;
  }
}

watch(() => props.device.id, loadStatus, { immediate: true });
</script>

<template>
  <div v-if="!verified" class="card p-10 grid place-items-center text-center">
    <Lock :size="28" class="text-dim mb-3" />
    <div class="text-[14px] font-medium">{{ t("device.protocolLocked") }}</div>
    <div class="text-[11.5px] text-dim mt-1 max-w-[460px]">{{ t("device.protocolLockedBody") }}</div>
  </div>
  <div v-else class="grid grid-cols-12 gap-4">
    <div class="col-span-5 space-y-4">
      <div class="card p-5">
        <div class="flex items-center justify-between mb-2">
          <div class="section-title">{{ t("mouse.buttons") }}</div>
          <StatusChip status="research" />
        </div>
        <div class="text-[11px] text-dim mb-1">{{ t("mouse.visualHint") }}</div>
        <div class="h-[340px] flex justify-center">
          <MouseModel :selected="selectedButton" :assigned="state.buttons" accent="#a78bfa" @pick="openAssign" />
        </div>
        <div class="grid grid-cols-2 gap-1.5 mt-2">
          <button v-for="b in buttons" :key="b.id" class="card-flat px-2.5 py-2 text-left hover:border-white/15 transition-colors" @click="openAssign(b.id)">
            <div class="text-[10.5px] text-dim">{{ b.label }}</div>
            <div class="text-[10.5px] mt-0.5 truncate">{{ actionLabel(b.id) }}</div>
          </button>
        </div>
        <div class="text-[10.5px] text-dim leading-relaxed mt-3">{{ t("mouse.buttonsAppOnly") }}</div>
      </div>
    </div>

    <div class="col-span-7 space-y-4">
      <div class="card p-5">
        <div class="flex items-center justify-between mb-4">
          <div class="section-title">{{ t("mouse.title") }}</div>
          <div class="flex items-center gap-1.5">
            <span v-if="hw" class="chip chip-ok !text-[10px]">{{ t("mouse.readFromDevice") }}</span>
            <span v-else class="chip chip-warn !text-[10px]">{{ t("mouse.deviceStatus") }}</span>
            <button class="btn !h-7 !px-2 !text-[11px]" :disabled="loading" :title="t('mouse.retry')" @click="loadStatus">
              <RefreshCw :size="12" :class="loading ? 'animate-spin' : ''" />
            </button>
          </div>
        </div>

        <div class="flex items-center justify-between">
          <div class="text-[11.5px] text-muted">{{ t("mouse.deviceStages") }}</div>
          <span class="text-[10.5px] text-dim">{{ t("mouse.selectStage") }}</span>
        </div>
        <div class="mt-2">
          <Slider v-if="hw" v-model="stageValue" :min="100" :max="30000" :step="50" unit=" dpi" :disabled="stagePending !== null" />
          <div v-else class="card-flat px-3 py-2 text-[11px] text-dim">{{ t("common.noData") }}</div>
        </div>

        <div class="grid grid-cols-6 gap-1.5 mt-3">
          <button
            v-for="(_, i) in stageEdits"
            :key="i"
            class="card-flat px-2 py-2 text-left transition-colors"
            :class="selectedStage === i ? 'border-accent/60 bg-accent/10' : 'hover:border-white/15'"
            :disabled="!hw"
            @click="selectedStage = i"
          >
            <div class="text-[10px] text-dim">{{ t("mouse.stageN") }} {{ i + 1 }}</div>
            <input
              v-if="hw"
              v-model.number="stageEdits[i]"
              type="number"
              min="100"
              max="30000"
              step="50"
              class="input !h-7 !border-0 !bg-transparent !px-0 mono !text-[12px]"
              :disabled="stagePending !== null"
              @click.stop
            />
            <div v-else class="h-7 flex items-center text-[12px] text-dim mono">—</div>
          </button>
        </div>

        <div class="flex items-center gap-2 mt-3">
          <button class="btn btn-primary !h-8 !text-[11.5px]" :disabled="!hw || stagePending !== null || loading" @click="applyStage">
            <Loader2 v-if="stagePending !== null" :size="12" class="animate-spin" />
            <Lock v-else-if="!hw" :size="12" />
            <Radio v-else :size="12" />
            {{ stagePending !== null ? t("mouse.applying") : t("mouse.applyStage") }}
          </button>
          <span class="text-[10.5px] text-dim">{{ t("mouse.dpiWriteNote") }}</span>
        </div>

        <div class="divider my-4"></div>

        <div class="text-[11.5px] text-muted mb-1.5">{{ t("mouse.polling") }}</div>
        <div class="flex flex-wrap gap-1.5">
          <button
            v-for="p in pollingOptions"
            :key="p"
            class="h-8 px-2.5 rounded-lg text-[11px] mono border transition-colors inline-flex items-center gap-1.5"
            :class="hw?.pollingHz === p ? 'border-accent/50 bg-accent/10 text-text' : 'border-white/8 text-dim hover:text-muted'"
            :disabled="!hw || pollingPending !== null"
            @click="applyPolling(p)"
          >
            <Loader2 v-if="pollingPending === p" :size="11" class="animate-spin" />
            {{ p }} Hz
          </button>
        </div>
        <div class="text-[10.5px] text-dim mt-1.5">{{ t("mouse.pollingUnsupported") }}</div>

        <div class="grid grid-cols-3 gap-3 mt-4">
          <div class="card-flat px-3 py-2.5">
            <div class="text-[10.5px] text-dim flex items-center gap-1"><Lock :size="10" /> {{ t("mouse.lod") }}</div>
            <div class="text-[11px] text-muted mt-1">—</div>
          </div>
          <div class="card-flat px-3 py-2.5">
            <div class="text-[10.5px] text-dim flex items-center gap-1"><Lock :size="10" /> {{ t("mouse.debounce") }}</div>
            <div class="text-[11px] text-muted mt-1">—</div>
          </div>
          <div class="card-flat px-3 py-2.5">
            <div class="text-[10.5px] text-dim flex items-center gap-1"><Lock :size="10" /> {{ t("mouse.angle") }}</div>
            <div class="text-[11px] text-muted mt-1">—</div>
          </div>
        </div>
        <div class="text-[10.5px] text-dim leading-relaxed mt-2">{{ t("mouse.unsupportedSensor") }}</div>

        <button class="btn w-full mt-4" disabled>
          <Lock :size="13" />
          {{ t("rgb.unavailable") }}
        </button>
        <div class="text-[10.5px] text-dim mt-2 leading-relaxed">{{ t("mouse.unsupportedRgb") }}</div>
      </div>

      <div class="card p-5">
        <div class="flex items-center justify-between mb-2">
          <div class="section-title">{{ t("mouse.batteryTitle") }}</div>
          <StatusChip :status="hw && batteryPercent !== null ? 'working' : 'research'" />
        </div>

        <template v-if="hw && batteryPercent !== null">
          <div class="flex items-end gap-3">
            <div class="text-[34px] leading-none font-semibold tracking-tight mono" :class="batteryClass">{{ batteryPercent }}%</div>
            <div class="pb-1 text-[11px] text-dim inline-flex items-center gap-1.5">
              <BatteryCharging v-if="hw.charging === true" :size="13" />
              <BatteryMedium v-else :size="13" />
              {{ chargingLabel }}
            </div>
          </div>
          <div class="h-[5px] rounded-full bg-white/5 overflow-hidden mt-3">
            <div class="h-full transition-all" :class="batteryBarClass" :style="{ width: batteryPercent + '%' }"></div>
          </div>
          <div class="text-[10.5px] text-dim mt-2">{{ t("mouse.batteryNote") }}</div>
        </template>

        <template v-else-if="hwError">
          <div class="text-[11.5px] text-warn leading-relaxed">{{ t("mouse.readFailed") }}</div>
          <div class="text-[10.5px] text-dim mt-1 break-all">{{ localizedError(hwError, locale) }}</div>
          <button class="btn !h-8 !text-[11.5px] mt-3" :disabled="loading" @click="loadStatus">
            <RefreshCw :size="12" :class="loading ? 'animate-spin' : ''" />
            {{ t("mouse.retry") }}
          </button>
        </template>

        <template v-else>
          <div class="text-[11.5px] text-dim leading-relaxed flex items-center gap-2">
            <Loader2 v-if="loading" :size="13" class="animate-spin" />
            {{ loading ? t("mouse.reading") : t("device.batteryUnknownBody") }}
          </div>
        </template>

        <div class="divider my-3"></div>
        <div class="flex items-center justify-between">
          <div class="text-[11.5px] text-muted">{{ t("mouse.profilesOnboard") }}</div>
          <span class="chip chip-info">{{ t("status.research") }}</span>
        </div>
        <div class="text-[10.5px] text-dim mt-1.5">{{ t("mouse.onboardNote") }}</div>
      </div>
    </div>

    <Modal :open="assignOpen" :title="t('mouse.assign')" width="420px" @close="assignOpen = false">
      <div class="text-[11px] text-dim mb-3">{{ selectedButton ? buttons.find((b) => b.id === selectedButton)?.label : "" }}</div>
      <div class="space-y-1.5">
        <label v-for="opt in ['none', 'macro', 'dpi', 'media', 'disabled'] as const" :key="opt" class="flex items-center gap-2.5 px-2 py-2 rounded-lg hover:bg-white/[0.03] cursor-pointer">
          <input v-model="assignKind" type="radio" :value="opt" class="accent-[#ff4fa3]" />
          <span class="text-[12px]">{{ opt === 'none' ? t('mouse.actionNone') : opt === 'macro' ? t('mouse.actionMacro') : opt === 'dpi' ? t('mouse.actionDpi') : opt === 'media' ? t('mouse.actionMedia') : t('mouse.actionDisabled') }}</span>
        </label>
      </div>
      <div v-if="assignKind === 'macro'" class="mt-3">
        <select v-model="assignMacro" class="input">
          <option value="" disabled>{{ t("common.noneSelected") }}</option>
          <option v-for="m in macros.macros" :key="m.id" :value="m.id">{{ m.name }}</option>
        </select>
        <div v-if="!macros.macros.length" class="text-[10.5px] text-dim mt-1.5">{{ t("macros.emptyBody") }}</div>
      </div>
      <div v-if="assignKind === 'media'" class="mt-3">
        <select v-model="assignMedia" class="input">
          <option v-for="k in mediaKeys" :key="k.value" :value="k.value">{{ k.label }}</option>
        </select>
      </div>
      <div class="flex items-start gap-2 text-[10.5px] text-dim mt-4">
        <Radio :size="12" class="mt-0.5 shrink-0" />
        {{ t("mouse.buttonsAppOnly") }}
      </div>
      <template #footer>
        <button class="btn" @click="assignOpen = false">{{ t("common.cancel") }}</button>
        <button class="btn btn-primary" @click="saveAssign">{{ t("common.save") }}</button>
      </template>
    </Modal>
  </div>
</template>
