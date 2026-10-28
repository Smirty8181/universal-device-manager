<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { AlertTriangle, Command, Loader2, Lock, RefreshCw } from "lucide-vue-next";
import type { DeviceView, KeyboardStatus } from "../../lib/types";
import { useI18n } from "../../lib/i18n";
import { useAppStore } from "../../stores/app";
import { api, isTauri } from "../../lib/api";
import { localizedError } from "../../lib/format";
import {
  ALUMIX_EFFECTS,
  directionFromHardware,
  directionMode,
  effectLabelKey,
  findLightingEffect,
  usesColor,
} from "../../lib/lighting";
import KeyboardModel from "../KeyboardModel.vue";
import ColorField from "../ColorField.vue";
import GameModeToggle from "../GameModeToggle.vue";
import Segmented from "../Segmented.vue";
import StatusChip from "../StatusChip.vue";

const props = defineProps<{ device: DeviceView }>();
const { t, locale } = useI18n();
const router = useRouter();
const app = useAppStore();

const hw = ref<KeyboardStatus | null>(null);
const hwError = ref<string | null>(null);
const loading = ref(false);
const applying = ref(false);

const verified = computed(() => props.device.adapterId === "redsquare");

const effect = ref(8);
const brightness = ref(100);
const speed = ref(50);
const direction = ref(0);
const color = ref("#ff4fa3");

const effects = computed(() => ALUMIX_EFFECTS.map((item) => ({ id: item.id, label: t(effectLabelKey(item.id)) })));
const showsColor = computed(() => usesColor(effect.value));
const showsDirection = computed(() => directionMode(effect.value) !== null);

const directionOptions = computed(() => {
  if (directionMode(effect.value) === "spiral") {
    return [
      { value: "0", label: t("keyboard.dirClockwise") },
      { value: "1", label: t("keyboard.dirCounter") },
    ];
  }
  return [
    { value: "0", label: t("dir.right") },
    { value: "1", label: t("dir.up") },
    { value: "2", label: t("dir.left") },
    { value: "3", label: t("dir.down") },
  ];
});
const directionChoice = computed({
  get: () => String(direction.value),
  set: (value: string) => (direction.value = Number(value)),
});

const levels = [0, 25, 50, 75, 100];
const levelLabel = (value: number) => (value === 0 ? t("keyboard.levelOff") : `${value}%`);

const previewEffect = computed(() => findLightingEffect(effect.value).preview);

const batteryText = computed(() => {
  if (!hw.value) return null;
  if (hw.value.battery !== null) return `${hw.value.battery}%`;
  return null;
});

function applyStatus(status: KeyboardStatus) {
  hw.value = status;
  effect.value = status.effectId;
  brightness.value = status.brightness;
  speed.value = status.speed;
  direction.value = directionFromHardware(status.direction, status.effectId);
  color.value = status.color;
}

watch(effect, (id) => {
  if (directionMode(id) === "spiral") direction.value = Math.min(direction.value, 1);
});

async function load() {
  if (!isTauri()) return;
  if (!verified.value) {
    hw.value = null;
    hwError.value = t("device.protocolLockedBody");
    return;
  }
  loading.value = true;
  hw.value = null;
  hwError.value = null;
  try {
    applyStatus(await api.keyboardStatus(props.device.id));
  } catch (error) {
    hw.value = null;
    hwError.value = String(error);
  } finally {
    loading.value = false;
  }
}

async function applyLighting() {
  if (!isTauri() || applying.value || !verified.value) return;
  applying.value = true;
  try {
    const result = await api.keyboardSetLighting(
      props.device.id,
      effect.value,
      brightness.value,
      speed.value,
      direction.value,
      color.value
    );
    if (result.status) applyStatus(result.status);
    app.toast(result.ok ? "success" : "warn", t("keyboard.hwTitle"), result.ok ? t("keyboard.hwConfirmed") : result.message);
  } catch (error) {
    app.toast("error", t("toast.error"), String(error));
  } finally {
    applying.value = false;
  }
}

onMounted(load);
watch(() => props.device.id, load);
</script>

<template>
  <div v-if="!verified" class="card p-10 grid place-items-center text-center">
    <Lock :size="28" class="text-dim mb-3" />
    <div class="text-[14px] font-medium">{{ t("device.protocolLocked") }}</div>
    <div class="text-[11.5px] text-dim mt-1 max-w-[460px]">{{ t("device.protocolLockedBody") }}</div>
    <button class="btn mt-4" @click="router.push(`/devices/${device.id}`)">{{ t("studio.openCard") }}</button>
  </div>
  <div v-else class="grid grid-cols-12 gap-4">
    <div class="col-span-7 space-y-4">
      <div class="card p-5">
        <div class="flex items-center justify-between mb-3">
          <div class="section-title">{{ t("keyboard.lightingTitle") }}</div>
          <div class="flex items-center gap-2">
            <StatusChip :status="hw ? 'working' : hwError ? 'partial' : 'research'" />
            <span v-if="hw" class="chip chip-ok !text-[10px]">{{ t("keyboard.hwProfile") }} {{ hw.profileIndex }}</span>
          </div>
        </div>
        <div class="relative">
          <div class="transition-all duration-300" :class="!hw ? 'opacity-35 grayscale' : ''">
            <KeyboardModel
              :effect="previewEffect"
              :primary="color"
              secondary="#a78bfa"
              :brightness="brightness"
              :speed="speed"
              :selected-keys="[]"
            />
          </div>
          <div v-if="!hw" class="absolute inset-0 grid place-items-center">
            <span class="chip chip-warn !h-7 !text-[10.5px]">
              <AlertTriangle :size="12" />
              {{ t("keyboard.hwPreviewUnavailableShort") }}
            </span>
          </div>
        </div>
        <div class="text-[11px] text-dim leading-relaxed mt-2 border-t border-white/[0.05] pt-3 flex items-start gap-2">
          <Lock :size="12" class="mt-0.5 shrink-0" :class="hw ? 'text-warn' : 'text-dim'" />
          {{ hw ? t("keyboard.hwPreviewNote") : t("keyboard.hwPreviewUnavailable") }}
        </div>
      </div>

      <div class="card p-5">
        <div class="flex items-center justify-between mb-3">
          <div class="section-title">{{ t("keyboard.hwState") }}</div>
          <button v-if="!loading" class="btn btn-ghost !h-7 !px-2 !text-[10.5px]" @click="load">
            <RefreshCw :size="12" />
            {{ t("keyboard.hwRetry") }}
          </button>
        </div>
        <div v-if="loading" class="flex items-center gap-2 py-3 text-[11.5px] text-dim">
          <Loader2 :size="14" class="animate-spin" />
          {{ t("keyboard.hwReading") }}
        </div>
        <div v-else-if="hwError" class="py-2">
          <div class="flex items-start gap-2 text-[11.5px] text-warn leading-relaxed">
            <AlertTriangle :size="13" class="mt-0.5 shrink-0" />
            <span>{{ t("keyboard.hwReadFailed") }}: {{ localizedError(hwError, locale) }}</span>
          </div>
        </div>
        <div v-else-if="hw" class="grid grid-cols-3 gap-2">
          <div class="card-flat p-3">
            <div class="overline">{{ t("keyboard.hwPolling") }}</div>
            <div class="text-[13px] mt-1">{{ hw.pollingHz ? `${hw.pollingHz} Hz` : "—" }}</div>
          </div>
          <div class="card-flat p-3">
            <div class="overline">{{ t("keyboard.hwLatency") }}</div>
            <div class="text-[13px] mt-1">{{ hw.latencyMs !== null ? `${hw.latencyMs} ms` : "—" }}</div>
          </div>
          <div class="card-flat p-3">
            <div class="overline">{{ t("keyboard.hwFirmware") }}</div>
            <div class="text-[13px] mt-1 mono">{{ hw.firmware ?? "—" }}</div>
          </div>
          <div class="card-flat p-3">
            <div class="overline">{{ t("keyboard.hwSleep") }}</div>
            <div class="text-[13px] mt-1">{{ hw.sleepMinutes !== null ? hw.sleepMinutes : "—" }}</div>
          </div>
          <div class="card-flat p-3">
            <div class="overline">{{ t("keyboard.hwDebounce") }}</div>
            <div class="text-[13px] mt-1">{{ hw.debounceMs !== null ? `${hw.debounceMs} ms` : "—" }}</div>
          </div>
          <div class="card-flat p-3">
            <div class="overline">{{ t("keyboard.batteryTitle") }}</div>
            <div class="text-[13px] mt-1">{{ batteryText ?? "—" }}</div>
          </div>
        </div>
        <div v-if="hw && hw.battery === null" class="text-[10.5px] text-dim leading-relaxed mt-3">
          {{ t("keyboard.hwBatteryOfficial") }}
          <span v-if="hw.batteryRaw !== null"> {{ t("keyboard.hwBatteryRaw").replace("{raw}", String(hw.batteryRaw)) }}</span>
        </div>
      </div>
    </div>

    <div class="col-span-5 space-y-4">
      <div class="card p-5">
        <div class="flex items-center justify-between mb-4">
          <div class="section-title">{{ t("keyboard.hwTitle") }}</div>
          <span v-if="hw" class="text-[10px] text-dim mono">{{ hw.pathKind === "usb" ? "USB" : "2.4G" }}</span>
        </div>

        <fieldset
          class="min-w-0 border-0 p-0 m-0 transition-opacity"
          :class="!hw ? 'opacity-50 pointer-events-none select-none' : ''"
          :disabled="!hw"
        >
          <label class="text-[11.5px] text-muted">{{ t("keyboard.hwEffect") }}</label>
          <select v-model.number="effect" class="input mt-1.5 mb-4">
            <option v-for="item in effects" :key="item.id" :value="item.id">{{ item.label }}</option>
          </select>

          <div class="text-[11.5px] text-muted mb-1.5">{{ t("keyboard.hwBrightness") }}</div>
          <div class="flex gap-1.5">
            <button
              v-for="level in levels"
              :key="'b' + level"
              class="flex-1 h-8 rounded-lg border text-[11px] transition-colors"
              :class="brightness === level ? 'border-accent/50 bg-accent/[0.10] text-text' : 'border-white/[0.07] text-dim hover:text-muted'"
              @click="brightness = level"
            >
              {{ levelLabel(level) }}
            </button>
          </div>

          <div class="text-[11.5px] text-muted mb-1.5 mt-3">{{ t("keyboard.hwSpeed") }}</div>
          <div class="flex gap-1.5">
            <button
              v-for="level in levels"
              :key="'s' + level"
              class="flex-1 h-8 rounded-lg border text-[11px] transition-colors"
              :class="speed === level ? 'border-accent/50 bg-accent/[0.10] text-text' : 'border-white/[0.07] text-dim hover:text-muted'"
              @click="speed = level"
            >
              {{ levelLabel(level) }}
            </button>
          </div>

          <div v-if="showsDirection" class="mt-4">
            <div class="text-[11.5px] text-muted mb-2">{{ t("keyboard.hwDirection") }}</div>
            <Segmented v-model="directionChoice" :options="directionOptions" />
          </div>

          <div v-if="showsColor" class="mt-4">
            <ColorField v-model="color" :label="t('keyboard.hwColor')" />
          </div>

          <div v-if="effect === 19" class="text-[10.5px] text-dim leading-relaxed mt-3">{{ t("keyboard.hwCustomNote") }}</div>
        </fieldset>

        <button class="btn btn-primary w-full mt-5" :disabled="!hw || applying" @click="applyLighting">
          <Loader2 v-if="applying" :size="13" class="animate-spin" />
          <Lock v-else-if="!hw" :size="13" />
          <RefreshCw v-else :size="13" />
          {{ applying ? t("keyboard.hwApplying") : t("keyboard.hwApply") }}
        </button>
        <div class="text-[10.5px] text-dim mt-2 leading-relaxed">{{ t("keyboard.hwNote") }}</div>
        <div v-if="hw?.pathKind === 'dongle'" class="text-[10.5px] text-warn mt-1.5 leading-relaxed">{{ t("keyboard.hwDongleNote") }}</div>
      </div>

      <div class="card p-5">
        <div class="section-title mb-2">{{ t("keyboard.profile") }}</div>
        <div class="flex items-center justify-between py-2">
          <div class="text-[12px]">{{ t("keyboard.gameMode") }}</div>
          <GameModeToggle />
        </div>
        <div class="text-[10.5px] text-dim leading-relaxed">{{ t("keyboard.gameModeNote") }}</div>
        <div class="divider my-3"></div>
        <div class="flex items-start gap-2 text-[10.5px] text-dim leading-relaxed">
          <Command :size="12" class="mt-0.5 shrink-0 text-accent-2" />
          {{ t("keyboard.macrosNote") }}
        </div>
        <button class="btn w-full mt-3 !h-8 !text-[11.5px]" @click="router.push('/macros')">
          <Command :size="13" />
          {{ t("nav.macros") }}
        </button>
      </div>
    </div>
  </div>
</template>
