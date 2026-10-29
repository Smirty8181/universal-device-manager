<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { AudioLines, RefreshCw, SlidersHorizontal, Volume2, VolumeX } from "lucide-vue-next";
import type { AppSession, AudioEndpoint } from "../../lib/types";
import { api, isTauri } from "../../lib/api";
import { useI18n } from "../../lib/i18n";
import { useAppStore } from "../../stores/app";
import Slider from "../Slider.vue";

const { t } = useI18n();
const app = useAppStore();

const endpoints = ref<AudioEndpoint[]>([]);
const endpointId = ref("");
const sessions = ref<AppSession[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);

const renderEndpoints = computed(() => endpoints.value.filter((e) => e.direction === "render"));
const current = computed(() => endpoints.value.find((e) => e.id === endpointId.value) ?? null);

async function loadEndpoints() {
  if (!isTauri()) return;
  try {
    endpoints.value = await api.audioList();
    const preferred = renderEndpoints.value.find((e) => e.isDefault) ?? renderEndpoints.value[0];
    if (preferred && !endpointId.value) endpointId.value = preferred.id;
  } catch (e) {
    error.value = String(e);
  }
}

async function loadSessions() {
  if (!endpointId.value || !isTauri()) return;
  loading.value = true;
  error.value = null;
  try {
    sessions.value = await api.audioSessions(endpointId.value);
  } catch (e) {
    error.value = String(e);
    sessions.value = [];
  } finally {
    loading.value = false;
  }
}

async function setVolume(session: AppSession, percent: number) {
  const level = Math.min(100, Math.max(0, percent)) / 100;
  session.volume = level;
  try {
    await api.audioSessionVolume(endpointId.value, session.instanceId, level);
  } catch (e) {
    app.toast("error", t("toast.error"), String(e));
    loadSessions();
  }
}

async function toggleMute(session: AppSession) {
  const next = !session.muted;
  session.muted = next;
  try {
    await api.audioSessionMute(endpointId.value, session.instanceId, next);
  } catch (e) {
    app.toast("error", t("toast.error"), String(e));
    loadSessions();
  }
}

onMounted(async () => {
  await loadEndpoints();
  await loadSessions();
});
</script>

<template>
  <div class="card p-5">
    <div class="flex items-center justify-between gap-3 flex-wrap mb-1">
      <div class="section-title">{{ t("input.appMixer") }}</div>
      <div class="flex items-center gap-2">
        <select
          v-model="endpointId"
          class="card-flat px-2.5 py-1.5 bg-transparent text-[11.5px] outline-none max-w-[280px]"
          @change="loadSessions"
        >
          <option value="" disabled>{{ t("input.pickEndpoint") }}</option>
          <option v-for="endpoint in renderEndpoints" :key="endpoint.id" :value="endpoint.id">
            {{ endpoint.name }}{{ endpoint.isDefault ? " · " + t("device.defaultDevice") : "" }}
          </option>
        </select>
        <button class="icon-btn" :title="t('common.refresh')" :disabled="loading" @click="loadSessions">
          <RefreshCw :size="13" :class="loading ? 'animate-spin' : ''" />
        </button>
      </div>
    </div>
    <div class="text-[11px] text-dim mb-4 leading-relaxed">{{ t("input.appMixerHint") }}</div>

    <div v-if="error" class="card-flat px-3 py-2.5 text-[11.5px] text-bad mb-3">{{ error }}</div>

    <div v-if="!isTauri()" class="text-[11.5px] text-dim py-6 text-center">{{ t("common.onlyInApp") }}</div>

    <template v-else>
      <div v-if="sessions.length" class="space-y-2">
        <div v-for="session in sessions" :key="session.instanceId" class="card-flat px-3 py-2.5">
          <div class="flex items-center gap-2 mb-2">
            <AudioLines :size="13" class="text-dim shrink-0" />
            <span class="text-[12px] truncate">{{ session.name || t("input.unnamed") }}</span>
            <span class="mono text-[10.5px] text-dim ml-auto shrink-0">PID {{ session.processId }}</span>
          </div>
          <div class="flex items-center gap-2">
            <button
              class="icon-btn shrink-0"
              :class="session.muted ? 'text-bad' : ''"
              :title="t('input.mute')"
              @click="toggleMute(session)"
            >
              <VolumeX v-if="session.muted" :size="13" />
              <Volume2 v-else :size="13" />
            </button>
            <Slider
              :model-value="Math.round(session.volume * 100)"
              :min="0"
              :max="100"
              unit="%"
              :disabled="session.muted"
              @update:model-value="(v: number) => setVolume(session, v)"
            />
          </div>
        </div>
      </div>
      <div v-else-if="!loading" class="text-[11.5px] text-dim py-6 text-center leading-relaxed">
        <SlidersHorizontal :size="18" class="mx-auto mb-2 opacity-60" />
        {{ t("input.noSessions") }}
        <div class="mt-1">{{ current?.name ?? "" }}</div>
      </div>
      <div v-else class="py-6 text-center text-[11.5px] text-dim">{{ t("common.loading") }}</div>
    </template>
  </div>
</template>
