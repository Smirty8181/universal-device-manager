<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { ExternalLink, FolderOpen, Headphones, Mic, MicOff, RefreshCw, RotateCcw, Save, Volume2, VolumeX } from "lucide-vue-next";
import type { AudioEndpoint, AudioState, DeviceView } from "../../lib/types";
import { api, openExternal, openPath } from "../../lib/api";
import { useDeviceState } from "../../lib/deviceStore";
import { useI18n } from "../../lib/i18n";
import { useAppStore } from "../../stores/app";
import { useProfilesStore } from "../../stores/profiles";
import EqGraph from "../EqGraph.vue";
import HeadsetModel from "../HeadsetModel.vue";
import Slider from "../Slider.vue";
import StatusChip from "../StatusChip.vue";

const props = defineProps<{ device: DeviceView }>();
const { t } = useI18n();
const app = useAppStore();
const profiles = useProfilesStore();

const FREQS = [32, 64, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];

const presets: Record<string, number[]> = {
  flat: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  gaming: [2, 2, 1, 0, 0, 0, 1, 2, 3, 2],
  music: [3, 2, 1, 0, -1, -1, 0, 1, 2, 3],
  movies: [4, 3, 2, 1, 0, 0, 1, 2, 3, 2],
  bass: [8, 7, 5, 3, 1, 0, 0, 0, 0, 0],
  vocal: [-2, -1, 0, 1, 3, 4, 4, 2, 0, -1],
  fps: [1, 2, 2, 1, 0, 0, 1, 3, 5, 6],
  footsteps: [0, 1, 2, 2, 1, 2, 4, 6, 7, 5],
};

const presetOrder = ["flat", "gaming", "music", "movies", "bass", "vocal", "fps", "footsteps"];

const state = useDeviceState(props.device.id + ".headset", {
  bands: [...presets.flat],
  preset: "flat",
});

const renderEndpoints = ref<AudioEndpoint[]>([]);
const captureEndpoints = ref<AudioEndpoint[]>([]);
const renderId = ref("");
const captureId = ref("");
const renderState = ref<AudioState | null>(null);
const captureState = ref<AudioState | null>(null);
const loading = ref(true);
const audioError = ref<string | null>(null);
const eapo = ref<{ installed: boolean; presetDir: string } | null>(null);
const savingPreset = ref(false);

const volumeTimers: Record<string, number | undefined> = {};

const graphBands = computed(() => state.value.bands);
const attached = computed(() => renderEndpoints.value.some((e) => e.id === renderId.value));

function pickEndpoint(list: AudioEndpoint[]) {
  return list.find((e) => /graphite/i.test(e.name)) ?? list.find((e) => e.isDefault) ?? list[0] ?? null;
}

function endpointLabel(e: AudioEndpoint) {
  const stateKey = `audio.state.${e.state.toLowerCase()}`;
  const stateLabel = t(stateKey);
  return `${e.name}${e.isDefault ? ` · ${t("audio.default")}` : ""}${e.state !== "Active" ? ` · ${stateLabel === stateKey ? e.state : stateLabel}` : ""}`;
}

async function loadRender() {
  if (!renderId.value) {
    renderState.value = null;
    return;
  }
  try {
    renderState.value = await api.audioState(renderId.value);
  } catch (e) {
    audioError.value = String(e);
  }
}

async function loadCapture() {
  if (!captureId.value) {
    captureState.value = null;
    return;
  }
  try {
    captureState.value = await api.audioState(captureId.value);
  } catch (e) {
    audioError.value = String(e);
  }
}

async function refreshAudio() {
  loading.value = true;
  audioError.value = null;
  try {
    const endpoints = await api.audioList();
    renderEndpoints.value = endpoints.filter((e) => e.direction === "render");
    captureEndpoints.value = endpoints.filter((e) => e.direction === "capture");
    if (!renderEndpoints.value.some((e) => e.id === renderId.value)) {
      renderId.value = pickEndpoint(renderEndpoints.value)?.id ?? "";
    }
    if (!captureEndpoints.value.some((e) => e.id === captureId.value)) {
      captureId.value = pickEndpoint(captureEndpoints.value)?.id ?? "";
    }
    await Promise.all([loadRender(), loadCapture()]);
  } catch (e) {
    audioError.value = String(e);
  } finally {
    loading.value = false;
  }
}

function scheduleVolume(key: string, endpointId: string, level: number) {
  window.clearTimeout(volumeTimers[key]);
  volumeTimers[key] = window.setTimeout(() => {
    api.audioSetVolume(endpointId, level).catch((e) => app.toast("error", t("toast.error"), String(e)));
  }, 140);
}

const volumePct = computed({
  get: () => Math.round((renderState.value?.volume ?? 0) * 100),
  set: (next: number) => {
    const st = renderState.value;
    if (!st) return;
    renderState.value = { ...st, volume: next / 100 };
    scheduleVolume("render", renderId.value, next / 100);
  },
});

const micPct = computed({
  get: () => Math.round((captureState.value?.volume ?? 0) * 100),
  set: (next: number) => {
    const st = captureState.value;
    if (!st) return;
    captureState.value = { ...st, volume: next / 100 };
    scheduleVolume("capture", captureId.value, next / 100);
  },
});

const channelCount = computed(() => renderState.value?.channelCount ?? 0);
const leftPct = computed({
  get: () => Math.round((renderState.value?.channels[0] ?? renderState.value?.volume ?? 0) * 100),
  set: (next: number) => setChannel(0, next),
});
const rightPct = computed({
  get: () => Math.round((renderState.value?.channels[1] ?? renderState.value?.volume ?? 0) * 100),
  set: (next: number) => setChannel(1, next),
});

function setChannel(index: number, percent: number) {
  const st = renderState.value;
  if (!st || index >= st.channelCount) return;
  const channels = [...st.channels];
  channels[index] = percent / 100;
  renderState.value = { ...st, channels };
  const endpoint = renderId.value;
  window.clearTimeout(volumeTimers[`ch${index}`]);
  volumeTimers[`ch${index}`] = window.setTimeout(() => {
    api.audioSetChannel(endpoint, index, percent / 100).catch((e) => app.toast("error", t("toast.error"), String(e)));
  }, 140);
}

async function toggleRenderMute() {
  const st = renderState.value;
  if (!st) return;
  const muted = !st.muted;
  renderState.value = { ...st, muted };
  try {
    await api.audioSetMute(renderId.value, muted);
  } catch (e) {
    app.toast("error", t("toast.error"), String(e));
  }
}

async function toggleMicMute() {
  const st = captureState.value;
  if (!st) return;
  const muted = !st.muted;
  captureState.value = { ...st, muted };
  try {
    await api.audioSetMute(captureId.value, muted);
  } catch (e) {
    app.toast("error", t("toast.error"), String(e));
  }
}

function applyPreset(key: string) {
  state.value.bands = [...presets[key]];
  state.value.preset = key;
}

function onBands(next: number[]) {
  state.value.bands = next;
  state.value.preset = "custom";
}

async function saveEqPreset() {
  if (!eapo.value?.installed) return;
  savingPreset.value = true;
  try {
    const maxGain = Math.max(0, ...state.value.bands);
    const lines = [
      `Preamp: ${(-maxGain).toFixed(1)} dB`,
      `GraphicEQ: ${FREQS.map((f, i) => `${f} ${state.value.bands[i].toFixed(1)}`).join("; ")}`,
    ];
    const name = `UDM ${props.device.model} ${state.value.preset}`;
    const path = await api.eapoSavePreset(name, lines);
    app.toast("success", t("headset.eqPresetSaved"), path || undefined);
  } catch (e) {
    app.toast("error", t("toast.error"), String(e));
  } finally {
    savingPreset.value = false;
  }
}

async function openPresetFolder() {
  if (eapo.value?.presetDir) await openPath(eapo.value.presetDir);
}

async function openSound() {
  try {
    await api.openSoundSettings();
  } catch (e) {
    app.toast("error", t("toast.error"), String(e));
  }
}

async function saveToProfile() {
  const profile = profiles.active;
  if (!profile) {
    app.toast("info", t("profiles.noProfile"));
    return;
  }
  profile.audio = {
    ...profile.audio,
    volume: renderState.value ? Math.round(renderState.value.volume * 100) : profile.audio.volume,
    muted: renderState.value?.muted ?? profile.audio.muted,
    channels: renderState.value?.channels,
    micGain: captureState.value ? Math.round(captureState.value.volume * 100) : profile.audio.micGain,
  };
  await profiles.save(profile);
}

watch(renderId, () => void loadRender());
watch(captureId, () => void loadCapture());

onMounted(async () => {
  await Promise.all([refreshAudio(), api.eapoStatus().then((s) => (eapo.value = s)).catch(() => (eapo.value = null))]);
});

onBeforeUnmount(() => {
  Object.values(volumeTimers).forEach((timer) => window.clearTimeout(timer));
});
</script>

<template>
  <div class="grid grid-cols-12 gap-4">
    <div class="col-span-5 space-y-4">
      <div class="card p-5">
        <div class="flex items-center justify-between">
          <div class="section-title flex items-center gap-2">
            <Headphones :size="15" class="text-accent" />
            {{ t("headset.title") }}
          </div>
          <StatusChip status="working" />
        </div>
        <HeadsetModel class="h-[250px] mx-auto my-1" />
        <div class="grid grid-cols-2 gap-2 text-[11px]">
          <div class="card-flat px-3 py-2">
            <div class="text-dim text-[10px]">{{ t("common.connection") }}</div>
            <div class="mt-0.5">{{ device.connectionLabel }}</div>
          </div>
          <div class="card-flat px-3 py-2">
            <div class="text-dim text-[10px]">{{ t("device.adapter") }}</div>
            <div class="mt-0.5">{{ device.adapterLabel }}</div>
          </div>
        </div>
      </div>

      <div class="card p-5">
        <div class="flex items-center justify-between mb-3">
          <div class="section-title">{{ t("headset.deviceSelect") }}</div>
          <button class="btn btn-ghost !h-7 !px-2" :title="t('audio.refresh')" @click="refreshAudio">
            <RefreshCw :size="13" :class="loading ? 'animate-spin' : ''" />
          </button>
        </div>

        <div v-if="audioError" class="card-flat px-3 py-2.5 text-[11px] text-bad mb-3">{{ audioError }}</div>

        <label class="block text-[11px] text-muted mb-1">{{ t("headset.output") }}</label>
        <select v-model="renderId" class="input !h-9 !text-[11.5px]">
          <option v-for="e in renderEndpoints" :key="e.id" :value="e.id">{{ endpointLabel(e) }}</option>
        </select>

        <label class="block text-[11px] text-muted mt-3 mb-1">{{ t("headset.input") }}</label>
        <select v-model="captureId" class="input !h-9 !text-[11.5px]">
          <option v-for="e in captureEndpoints" :key="e.id" :value="e.id">{{ endpointLabel(e) }}</option>
        </select>

        <div v-if="!loading && (!renderEndpoints.length || !captureEndpoints.length)" class="text-[11px] text-dim mt-2">
          {{ t("headset.audioUnavailable") }}
        </div>

        <button class="btn w-full mt-3" @click="openSound">
          <ExternalLink :size="13" />
          {{ t("headset.openSoundSettings") }}
        </button>
        <div class="text-[10.5px] text-dim mt-2 leading-relaxed">{{ t("headset.defaultDeviceNote") }}</div>
      </div>

      <div class="card p-5">
        <div class="flex items-center justify-between mb-3">
          <div class="section-title">{{ t("headset.volume") }}</div>
          <button class="btn btn-ghost !h-8 !px-2.5" :disabled="!attached" @click="toggleRenderMute">
            <VolumeX v-if="renderState?.muted" :size="14" class="text-bad" />
            <Volume2 v-else :size="14" />
            <span class="text-[10.5px]">{{ renderState?.muted ? t("audio.unmute") : t("audio.mute") }}</span>
          </button>
        </div>
        <Slider v-model="volumePct" :min="0" :max="100" :disabled="!renderState" unit="%" />
        <div class="text-[10.5px] text-dim mt-2">{{ t("headset.volumeNote") }}</div>

        <div class="divider my-4"></div>

        <div class="flex items-center justify-between mb-2">
          <div class="text-[11.5px] text-muted">{{ t("headset.balance") }}</div>
          <span v-if="channelCount > 2" class="text-[10px] text-dim">{{ t("headset.multichannelNote") }}</span>
        </div>
        <div class="space-y-2">
          <div class="flex items-center gap-3">
            <span class="text-[10.5px] text-dim w-12">{{ t("headset.balanceLeft") }}</span>
            <Slider v-model="leftPct" :min="0" :max="100" :disabled="!renderState || channelCount < 2" unit="%" />
          </div>
          <div class="flex items-center gap-3">
            <span class="text-[10.5px] text-dim w-12">{{ t("headset.balanceRight") }}</span>
            <Slider v-model="rightPct" :min="0" :max="100" :disabled="!renderState || channelCount < 2" unit="%" />
          </div>
        </div>

        <button class="btn w-full mt-4" @click="saveToProfile">
          <Save :size="13" />
          {{ t("headset.profileSave") }}
        </button>
      </div>
    </div>

    <div class="col-span-7 space-y-4">
      <div class="card p-5">
        <div class="flex items-center justify-between mb-1">
          <div class="section-title">{{ t("headset.eq") }}</div>
          <StatusChip status="partial" />
        </div>
        <div class="text-[10.5px] text-dim mb-2">{{ t("headset.eqNote") }}</div>

        <EqGraph :bands="graphBands" @update:bands="onBands" />

        <div class="flex flex-wrap gap-1.5 mt-2">
          <button
            v-for="key in presetOrder"
            :key="key"
            class="chip !h-7 !px-3 transition-colors"
            :class="state.preset === key ? 'chip-ok !border-accent/40 !text-text' : '!text-dim hover:!text-muted'"
            @click="applyPreset(key)"
          >
            {{ t("eq." + key) }}
          </button>
          <button v-if="state.preset === 'custom'" class="chip chip-info !h-7 !px-3">{{ t("eq.custom") }}</button>
          <button class="chip !h-7 !px-2.5 !text-dim hover:!text-muted" :title="t('headset.resetEq')" @click="applyPreset('flat')">
            <RotateCcw :size="12" />
          </button>
        </div>

        <div class="flex items-center justify-between gap-3 mt-4 card-flat px-3 py-2.5">
          <div class="min-w-0">
            <div class="flex items-center gap-2">
              <span class="text-[11.5px]">{{ eapo?.installed ? t("headset.eqEapoInstalled") : t("headset.eqRequireEapo") }}</span>
              <span class="chip !h-5 !text-[9.5px]" :class="eapo?.installed ? 'chip-ok' : 'chip-warn'">
                {{ eapo?.installed ? "Equalizer APO" : t("common.research") }}
              </span>
            </div>
            <div class="text-[10px] text-dim mt-1 leading-relaxed">
              {{ eapo?.installed ? t("headset.eqFileNote") : t("headset.eqEapoMissing") }}
            </div>
          </div>
          <div class="flex items-center gap-1.5 shrink-0">
            <button v-if="eapo?.installed" class="btn !h-8 !px-2.5" :title="t('headset.eqOpenFolder')" @click="openPresetFolder">
              <FolderOpen :size="13" />
            </button>
            <button class="btn btn-primary !h-8 !px-3 !text-[11px]" :disabled="!eapo?.installed || savingPreset" @click="saveEqPreset">
              {{ t("headset.eqSavePreset") }}
            </button>
          </div>
        </div>
        <button v-if="eapo && !eapo.installed" class="btn btn-ghost w-full mt-2 !text-[11px]" @click="openExternal('https://sourceforge.net/projects/equalizerapo/')">
          <ExternalLink :size="12" />
          {{ t("headset.eqGetEapo") }}
        </button>
      </div>

      <div class="card p-5">
        <div class="flex items-center justify-between mb-3">
          <div class="section-title flex items-center gap-2">
            <Mic :size="14" class="text-accent-3" />
            {{ t("headset.mic") }}
          </div>
          <button class="btn btn-ghost !h-8 !px-2.5" :disabled="!captureState" @click="toggleMicMute">
            <MicOff v-if="captureState?.muted" :size="14" class="text-bad" />
            <Mic v-else :size="14" />
            <span class="text-[10.5px]">{{ captureState?.muted ? t("audio.unmute") : t("audio.mute") }}</span>
          </button>
        </div>
        <div class="text-[11px] text-muted mb-1">{{ t("headset.micLevel") }}</div>
        <Slider v-model="micPct" :min="0" :max="100" :disabled="!captureState" unit="%" />
        <div class="text-[10.5px] text-dim mt-2">{{ t("headset.micLevelNote") }}</div>
      </div>

      <div class="card p-5">
        <div class="flex items-start justify-between gap-4">
          <div>
            <div class="text-[11.5px]">{{ t("headset.spatial") }}</div>
            <div class="text-[10.5px] text-dim mt-1 leading-relaxed">{{ t("headset.spatialNote") }}</div>
          </div>
          <StatusChip status="partial" />
        </div>
        <button class="btn w-full mt-3" @click="openSound">
          <ExternalLink :size="13" />
          {{ t("headset.openSoundSettings") }}
        </button>
      </div>

      <div class="card p-5">
        <div class="flex items-center justify-between mb-2">
          <div class="section-title">{{ t("device.batteryUnknownTitle") }}</div>
          <StatusChip status="research" />
        </div>
        <div class="text-[11px] text-dim leading-relaxed">{{ t("headset.batteryNote") }}</div>
      </div>
    </div>
  </div>
</template>
