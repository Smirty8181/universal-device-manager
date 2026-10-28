<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { AlertTriangle, Check, Loader2, Lock, Plus, RefreshCw, RotateCcw, Save, Trash2 } from "lucide-vue-next";
import type { DeviceView, KeyboardStatus, Profile } from "../lib/types";
import { api, isTauri } from "../lib/api";
import { clamp, formatHex, hexToHsl, hslToHex, localizedError } from "../lib/format";
import {
  ALUMIX_EFFECTS,
  directionFromHardware,
  directionMode,
  effectIdFromKey,
  effectLabelKey,
  findLightingEffect,
  loadPresets,
  speedApplies,
  storePresets,
  usesColor,
  type LightingPreset,
} from "../lib/lighting";
import { useI18n } from "../lib/i18n";
import { useAppStore } from "../stores/app";
import { useProfilesStore } from "../stores/profiles";
import { useRgbStore } from "../stores/rgb";
import KeyboardModel from "./KeyboardModel.vue";
import BatteryRgbPanel from "./BatteryRgbPanel.vue";
import ColorPicker from "./ColorPicker.vue";
import Segmented from "./Segmented.vue";
import Slider from "./Slider.vue";
import Modal from "./Modal.vue";

const props = defineProps<{ device: DeviceView }>();
const { t, locale } = useI18n();
const app = useAppStore();
const profilesStore = useProfilesStore();
const rgb = useRgbStore();

const status = ref<KeyboardStatus | null>(null);
const loading = ref(false);
const applying = ref(false);
const error = ref<string | null>(null);

const verified = computed(
  () =>
    props.device.adapterId === "redsquare" ||
    props.device.modules?.find((m) => m.id === "rgb")?.status === "working" ||
    props.device.capabilities.some(
      (cap) => cap.id === "lighting" && (cap.status === "working" || cap.status === "partial")
    )
);

const effect = ref(8);
const brightness = ref(100);
const speed = ref(50);
const direction = ref(0);
const color = ref("#ff00ff");
const hoveredKey = ref<string | null>(null);

const secondary = computed(() => {
  const [h, s, l] = hexToHsl(color.value);
  return hslToHex((h + 46) % 360, clamp(s + 6, 0, 100), clamp(l + 6, 46, 78));
});

const effectInfo = computed(() => findLightingEffect(effect.value));
const previewEffect = computed(() => effectInfo.value.preview);
const colorEnabled = computed(() => usesColor(effect.value));
const speedEnabled = computed(() => speedApplies(effect.value));
const dirMode = computed(() => directionMode(effect.value));

const directionOptions = computed(() =>
  dirMode.value === "spiral"
    ? [
        { value: "0", label: t("keyboard.dirClockwise") },
        { value: "1", label: t("keyboard.dirCounter") },
      ]
    : [
        { value: "0", label: t("dir.right") },
        { value: "1", label: t("dir.up") },
        { value: "2", label: t("dir.left") },
        { value: "3", label: t("dir.down") },
      ]
);
const directionChoice = computed({
  get: () => String(direction.value),
  set: (value: string) => (direction.value = Number(value)),
});

const dirty = computed(() => {
  const current = status.value;
  if (!current) return false;
  if (effect.value !== current.effectId) return true;
  if (Math.round(brightness.value) !== current.brightness) return true;
  if (speedEnabled.value && Math.round(speed.value) !== current.speed) return true;
  if (colorEnabled.value && color.value.toLowerCase() !== formatHex(current.color).toLowerCase()) return true;
  if (dirMode.value === "linear" && direction.value !== directionFromHardware(current.direction, current.effectId)) return true;
  return false;
});

const transport = computed(() => (status.value?.pathKind === "dongle" ? "2.4G" : "USB"));

const lightProfiles = computed(() => profilesStore.profiles.filter((profile) => profile.rgb?.enabled));
const presets = ref<LightingPreset[]>(loadPresets());

const saveMode = ref<"preset" | "profile" | null>(null);
const saveName = ref("");

function applyStatus(next: KeyboardStatus) {
  status.value = next;
  effect.value = next.effectId;
  brightness.value = Math.round(next.brightness / 25) * 25;
  speed.value = Math.round(next.speed / 25) * 25;
  direction.value = directionFromHardware(next.direction, next.effectId);
  color.value = formatHex(next.color);
  rgb.setColors(color.value, secondary.value);
}

async function load() {
  if (!isTauri()) {
    error.value = t("rgb.demoOnly");
    return;
  }
  if (!verified.value) {
    status.value = null;
    error.value = t("device.protocolLockedBody");
    return;
  }
  loading.value = true;
  status.value = null;
  error.value = null;
  try {
    applyStatus(await api.keyboardStatus(props.device.id));
  } catch (e) {
    status.value = null;
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function apply() {
  if (!status.value || applying.value || !isTauri() || !verified.value) return;
  applying.value = true;
  try {
    const result = await api.keyboardSetLighting(
      props.device.id,
      effect.value,
      brightness.value,
      speed.value,
      dirMode.value === "spiral" ? Math.min(direction.value, 1) : direction.value,
      color.value
    );
    if (result.status) applyStatus(result.status);
    app.toast(result.ok ? "success" : "warn", t("keyboard.hwTitle"), result.ok ? t("rgb.applyConfirmed") : result.message);
    if (result.ok) app.addActivity("device", `${props.device.name}: ${t("rgb.applyConfirmed")}`, "success", props.device.id);
  } catch (e) {
    app.toast("error", t("toast.error"), String(e));
  } finally {
    applying.value = false;
  }
}

function selectEffect(id: number) {
  effect.value = id;
  if (directionMode(id) === "spiral") direction.value = Math.min(direction.value, 1);
}

function revert() {
  if (status.value) applyStatus(status.value);
}

defineExpose({
  lightingSnapshot: () => ({
    effect: effect.value,
    brightness: brightness.value,
    speed: speed.value,
    direction: dirMode.value === "spiral" ? Math.min(direction.value, 1) : direction.value,
    color: color.value,
  }),
});

function swatchFor(id: number) {
  const item = findLightingEffect(id);
  if (!item.color) return "linear-gradient(90deg,#ff6b6b,#fbbf24,#45d483,#6ee7f9,#a78bfa)";
  if (id === 19) return `linear-gradient(90deg, ${color.value}, #2a2142, ${secondary.value})`;
  return `linear-gradient(90deg, ${color.value}, ${secondary.value})`;
}

function openSave(mode: "preset" | "profile") {
  saveMode.value = mode;
  saveName.value =
    mode === "preset"
      ? `${t(effectLabelKey(effect.value))} · ${color.value.toUpperCase()}`
      : `${t("rgb.title")} ${t(effectLabelKey(effect.value))}`;
}

async function confirmSave() {
  const name = saveName.value.trim();
  if (!name || !saveMode.value) return;
  if (saveMode.value === "preset") {
    const preset: LightingPreset = {
      id: `preset-${Date.now()}`,
      name,
      effect: effect.value,
      brightness: brightness.value,
      speed: speed.value,
      direction: direction.value,
      color: color.value,
      createdAt: Date.now(),
    };
    presets.value = [preset, ...presets.value].slice(0, 24);
    storePresets(presets.value);
    app.toast("success", t("rgb.savePreset"), name);
  } else {
    const profile = profilesStore.emptyProfile(name);
    profile.rgb = {
      enabled: true,
      effect: findLightingEffect(effect.value).key,
      color: color.value,
      secondary: secondary.value,
      brightness: brightness.value,
      speed: speed.value,
      direction: String(direction.value),
    };
    profile.apps = [];
    await profilesStore.save(profile);
    app.addActivity("profile", `${t("toast.profileSaved")}: ${profile.name}`, "success");
  }
  saveMode.value = null;
}

function loadPreset(preset: LightingPreset) {
  effect.value = findLightingEffect(preset.effect).id;
  brightness.value = Math.round(preset.brightness / 25) * 25;
  speed.value = Math.round(preset.speed / 25) * 25;
  direction.value = clamp(preset.direction, 0, directionMode(effect.value) === "spiral" ? 1 : 3);
  color.value = formatHex(preset.color);
  app.toast("info", t("rgb.presets"), t("rgb.presetLoaded"));
}

function removePreset(id: string) {
  presets.value = presets.value.filter((preset) => preset.id !== id);
  storePresets(presets.value);
}

function loadProfile(profile: Profile) {
  const id = effectIdFromKey(profile.rgb.effect);
  if (id === null) return;
  effect.value = id;
  brightness.value = clamp(Math.round(profile.rgb.brightness / 25) * 25, 0, 100);
  speed.value = clamp(Math.round(profile.rgb.speed / 25) * 25, 0, 100);
  const dir = Number(profile.rgb.direction);
  direction.value = Number.isFinite(dir) ? clamp(dir, 0, directionMode(id) === "spiral" ? 1 : 3) : 0;
  color.value = formatHex(profile.rgb.color);
  app.toast("info", t("rgb.profiles"), profile.name);
}

onMounted(load);
watch(() => props.device.id, load);
</script>

<template>
  <div v-if="!verified" class="panel p-10 grid place-items-center text-center">
    <Lock :size="28" class="text-dim mb-3" />
    <div class="text-[14px] font-medium">{{ t("device.protocolLocked") }}</div>
    <div class="text-[11.5px] text-dim mt-1 max-w-[460px]">{{ t("device.protocolLockedBody") }}</div>
  </div>
  <div v-else class="grid grid-cols-12 gap-5">
    <div class="col-span-7 space-y-5">
      <div class="panel relative overflow-hidden">
        <div
          class="absolute inset-0 pointer-events-none transition-opacity duration-500"
          :style="{ background: `radial-gradient(58% 52% at 50% 66%, ${color}26, transparent 72%)`, opacity: brightness / 100 }"
        ></div>
        <div class="relative p-5">
          <div class="flex items-start justify-between gap-3 mb-3">
            <div>
              <div class="flex items-center gap-2">
                <span class="live-dot" :class="status ? 'text-ok' : 'text-warn'" :style="status ? '' : 'opacity:.5'"></span>
                <span class="text-[10px] mono uppercase tracking-[0.16em] text-muted">{{ t("rgb.live") }}</span>
              </div>
              <div class="text-[10.5px] text-dim mt-1">{{ status ? t("rgb.liveNote") : t("rgb.readFailed") }}</div>
            </div>
            <div class="flex items-center gap-1.5 flex-wrap justify-end">
              <span v-if="status" class="chip !h-6 mono">{{ t("rgb.profileChip") }} {{ status.profileIndex }}</span>
              <span v-if="status" class="chip !h-6 mono">{{ transport }}</span>
              <span v-if="status?.firmware" class="chip !h-6 mono">FW {{ status.firmware }}</span>
              <button class="icon-btn" :title="t('rgb.retry')" :disabled="loading" @click="load">
                <RefreshCw :size="13" :class="loading ? 'animate-spin' : ''" />
              </button>
            </div>
          </div>

          <div v-if="loading" class="h-[286px] grid place-items-center">
            <div class="flex items-center gap-2 text-[11.5px] text-dim">
              <Loader2 :size="15" class="animate-spin" />
              {{ t("rgb.reading") }}
            </div>
          </div>

          <div v-else-if="error && !status" class="h-[286px] grid place-items-center px-8 text-center">
            <div>
              <AlertTriangle :size="22" class="mx-auto text-warn mb-2" />
              <div class="text-[12.5px] font-medium">{{ t("rgb.readFailed") }}</div>
              <div class="text-[10.5px] text-dim mt-1 leading-relaxed">{{ localizedError(error, locale) }}</div>
              <button class="btn mt-3 !h-8 !text-[11.5px]" @click="load">
                <RefreshCw :size="12" />
                {{ t("rgb.retry") }}
              </button>
            </div>
          </div>

          <div v-else class="py-1">
            <KeyboardModel
              :effect="previewEffect"
              :primary="color"
              :secondary="secondary"
              :brightness="brightness"
              :speed="speed"
              :interactive="false"
              :active-key="hoveredKey"
              @hover-key="hoveredKey = $event"
            />
            <div class="text-center text-[10px] text-dim mt-1">{{ t("rgb.previewHint") }}</div>
          </div>

          <div class="grid grid-cols-4 gap-2 mt-3">
            <div class="card-flat px-3 py-2">
              <div class="overline">{{ t("rgb.effect") }}</div>
              <div class="text-[11.5px] mt-0.5 truncate">{{ status ? t(effectLabelKey(status.effectId)) : "—" }}</div>
            </div>
            <div class="card-flat px-3 py-2">
              <div class="overline">{{ t("rgb.brightness") }}</div>
              <div class="text-[11.5px] mt-0.5 mono">{{ status ? `${status.brightness}%` : "—" }}</div>
            </div>
            <div class="card-flat px-3 py-2">
              <div class="overline">{{ t("rgb.speed") }}</div>
              <div class="text-[11.5px] mt-0.5 mono">{{ status ? `${status.speed}%` : "—" }}</div>
            </div>
            <div class="card-flat px-3 py-2">
              <div class="overline">{{ t("rgb.color") }}</div>
              <div class="flex items-center gap-1.5 mt-0.5">
                <span v-if="status" class="w-2.5 h-2.5 rounded-full" :style="{ background: status.color }"></span>
                <span class="text-[11.5px] mono uppercase">{{ status ? status.color : "—" }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="panel p-5">
        <div class="flex items-center justify-between mb-3">
          <div class="section-title">{{ t("rgb.effectLibrary") }}</div>
          <span class="text-[10px] text-dim mono">{{ ALUMIX_EFFECTS.length }}</span>
        </div>
        <div class="grid grid-cols-3 gap-2">
          <button
            v-for="item in ALUMIX_EFFECTS"
            :key="item.id"
            class="relative rounded-xl border px-2.5 pt-2 pb-2.5 text-left transition-all duration-150 hover:-translate-y-px"
            :class="effect === item.id ? 'border-accent/50 bg-accent/[0.07] shadow-[0_0_20px_rgba(255,79,163,0.10)]' : 'border-white/[0.06] hover:border-white/15 hover:bg-white/[0.025]'"
            @click="selectEffect(item.id)"
          >
            <span class="block h-6 rounded-md mb-1.5" :style="{ background: swatchFor(item.id) }"></span>
            <span class="block text-[11px] leading-tight truncate" :class="effect === item.id ? 'text-text' : 'text-muted'">
              {{ t(effectLabelKey(item.id)) }}
            </span>
            <Check v-if="effect === item.id" :size="11" class="absolute top-2 right-2 text-accent" />
          </button>
        </div>
        <div class="text-[10px] text-dim leading-relaxed mt-3">{{ t("rgb.effectLibraryNote") }}</div>
      </div>

      <div class="panel p-5 flex items-start gap-3">
        <span class="w-8 h-8 rounded-lg grid place-items-center bg-white/[0.04] border border-white/[0.06] shrink-0">
          <span class="w-2.5 h-2.5 rounded-[3px] border border-accent/60" style="box-shadow: inset 0 0 0 2px rgba(255,79,163,.18)"></span>
        </span>
        <div>
          <div class="text-[11.5px] font-medium">{{ t("rgb.zonesTitle") }}</div>
          <div class="text-[10.5px] text-dim leading-relaxed mt-1">{{ t("rgb.zonesBody") }}</div>
        </div>
      </div>
    </div>

    <div class="col-span-5 space-y-5">
      <div class="panel p-5">
        <div class="flex items-center justify-between mb-4">
          <div class="section-title">{{ t("rgb.settingsTitle") }}</div>
          <span class="chip !h-6" :class="!status ? '' : dirty ? 'chip-warn' : 'chip-ok'">
            <span class="status-dot" :class="!status ? 'bg-dim' : dirty ? 'bg-warn' : 'bg-ok'"></span>
            {{ !status ? t("common.noData") : dirty ? t("rgb.unsaved") : t("rgb.inSync") }}
          </span>
        </div>

        <div class="flex items-center justify-between mb-1.5">
          <span class="text-[11.5px] text-muted">{{ t("rgb.color") }}</span>
          <span class="text-[10px] mono uppercase text-dim">{{ color }}</span>
        </div>
        <ColorPicker v-model="color" :disabled="!colorEnabled" />
        <div class="text-[10px] text-dim leading-relaxed mt-2">
          {{ effect === 19 ? t("rgb.customLimits") : colorEnabled ? t("rgb.colorNote") : t("rgb.colorUnused") }}
        </div>

        <div class="divider my-4"></div>

        <div class="text-[11.5px] text-muted mb-1">{{ t("rgb.brightness") }}</div>
        <Slider v-model="brightness" :min="0" :max="100" :step="25" unit="%" />

        <div class="text-[11.5px] mt-4 mb-1" :class="speedEnabled ? 'text-muted' : 'text-dim'">{{ t("rgb.speed") }}</div>
        <Slider v-model="speed" :min="0" :max="100" :step="25" unit="%" :disabled="!speedEnabled" />
        <div v-if="effect === 8" class="text-[10px] text-dim mt-1.5">{{ t("rgb.speedUnused") }}</div>

        <div v-if="dirMode" class="mt-4">
          <div class="text-[11.5px] text-muted mb-2">{{ t("rgb.direction") }}</div>
          <Segmented v-model="directionChoice" :options="directionOptions" />
        </div>

        <div v-if="effect === 19" class="text-[10px] text-dim leading-relaxed mt-3">{{ t("keyboard.hwCustomNote") }}</div>
        <div class="text-[10px] text-dim leading-relaxed mt-3">{{ t("rgb.levelsNote") }}</div>

        <div class="flex items-center gap-2 mt-4">
          <button class="btn btn-primary flex-1" :disabled="!status || applying" @click="apply">
            <Loader2 v-if="applying" :size="13" class="animate-spin" />
            <Save v-else :size="13" />
            {{ applying ? t("rgb.applying") : t("rgb.apply") }}
          </button>
          <button class="btn !px-3" :disabled="!dirty || applying" :title="t('rgb.revert')" @click="revert">
            <RotateCcw :size="13" />
          </button>
        </div>
        <div class="text-[10px] text-dim leading-relaxed mt-2">{{ t("rgb.hardwareNote") }}</div>
        <div v-if="status?.pathKind === 'dongle'" class="text-[10px] text-warn leading-relaxed mt-1.5">{{ t("keyboard.hwDongleNote") }}</div>
      </div>

      <BatteryRgbPanel :device="device" />

      <div class="panel p-5">
        <div class="flex items-center justify-between mb-3">
          <div class="section-title">{{ t("rgb.presets") }}</div>
          <button class="btn btn-ghost !h-7 !px-2 !text-[10.5px]" @click="openSave('preset')">
            <Plus :size="12" />
            {{ t("rgb.savePreset") }}
          </button>
        </div>
        <div v-if="!presets.length" class="py-2">
          <div class="text-[11.5px] text-muted">{{ t("rgb.noPresets") }}</div>
          <div class="text-[10.5px] text-dim leading-relaxed mt-1">{{ t("rgb.noPresetsBody") }}</div>
        </div>
        <div v-else class="space-y-1.5">
          <div
            v-for="preset in presets"
            :key="preset.id"
            class="group flex items-center gap-2.5 px-2.5 py-2 rounded-xl border border-white/[0.05] hover:border-white/12 hover:bg-white/[0.02] transition-colors cursor-pointer"
            @click="loadPreset(preset)"
          >
            <span class="w-7 h-7 rounded-lg shrink-0 border border-white/10" :style="{ background: `linear-gradient(135deg, ${preset.color}, #2a2142)` }"></span>
            <div class="min-w-0 flex-1">
              <div class="text-[11.5px] truncate">{{ preset.name }}</div>
              <div class="text-[10px] text-dim">{{ t(effectLabelKey(preset.effect)) }} · {{ preset.brightness }}%</div>
            </div>
            <span class="text-[10px] mono text-dim uppercase">{{ preset.color }}</span>
            <button class="btn btn-ghost !h-6 !w-6 !p-0 opacity-0 group-hover:opacity-100" :title="t('rgb.deletePreset')" @click.stop="removePreset(preset.id)">
              <Trash2 :size="11" />
            </button>
          </div>
        </div>
      </div>

      <div class="panel p-5">
        <div class="flex items-center justify-between mb-3">
          <div class="section-title">{{ t("rgb.profiles") }}</div>
          <button class="btn btn-ghost !h-7 !px-2 !text-[10.5px]" @click="openSave('profile')">
            <Save :size="12" />
            {{ t("rgb.saveProfile") }}
          </button>
        </div>
        <div v-if="!lightProfiles.length" class="text-[10.5px] text-dim leading-relaxed">{{ t("profiles.emptyBody") }}</div>
        <div v-else class="space-y-1.5">
          <div
            v-for="profile in lightProfiles"
            :key="profile.id"
            class="flex items-center gap-2.5 px-2.5 py-2 rounded-xl border border-white/[0.05] hover:border-white/12 transition-colors cursor-pointer"
            @click="loadProfile(profile)"
          >
            <span class="w-7 h-7 rounded-lg shrink-0 border border-white/10" :style="{ background: `linear-gradient(135deg, ${profile.rgb.color}, ${profile.rgb.secondary})` }"></span>
            <div class="min-w-0 flex-1">
              <div class="text-[11.5px] truncate">{{ profile.name }}</div>
              <div class="text-[10px] text-dim">{{ t("rgb.effects." + profile.rgb.effect) }} · {{ profile.rgb.brightness }}%</div>
            </div>
            <span class="text-[10px] text-dim">{{ t("rgb.loadProfile") }}</span>
          </div>
        </div>
      </div>
    </div>

    <Modal :open="saveMode !== null" :title="saveMode === 'preset' ? t('rgb.savePreset') : t('rgb.saveProfile')" width="400px" @close="saveMode = null">
      <label class="block text-[11px] text-muted mb-1.5">{{ saveMode === "preset" ? t("rgb.presetName") : t("rgb.profileName") }}</label>
      <input v-model="saveName" class="input" :placeholder="t('rgb.presetName')" @keydown.enter="confirmSave" />
      <div class="flex items-center gap-2 mt-4">
        <span class="w-7 h-7 rounded-lg border border-white/10" :style="{ background: `linear-gradient(135deg, ${color}, ${secondary})` }"></span>
        <span class="text-[11px] text-dim">{{ t(effectLabelKey(effect)) }} · {{ brightness }}% / {{ speed }}% · {{ color }}</span>
      </div>
      <div class="text-[10.5px] text-dim mt-3 leading-relaxed">{{ t("rgb.hardwareNote") }}</div>
      <template #footer>
        <button class="btn" @click="saveMode = null">{{ t("common.cancel") }}</button>
        <button class="btn btn-primary" :disabled="!saveName.trim()" @click="confirmSave">{{ t("common.save") }}</button>
      </template>
    </Modal>
  </div>
</template>
