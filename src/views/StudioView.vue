<script setup lang="ts">
import { computed, ref, watch, type Component } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ArrowUpRight, Headphones, Keyboard, Lock, Mouse, Palette, Usb } from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import EmptyState from "../components/EmptyState.vue";
import BatteryPill from "../components/BatteryPill.vue";
import KeyboardModel from "../components/KeyboardModel.vue";
import MouseModel from "../components/MouseModel.vue";
import HeadsetModel from "../components/HeadsetModel.vue";
import KeyboardPanel from "../components/panels/KeyboardPanel.vue";
import MousePanel from "../components/panels/MousePanel.vue";
import HeadsetPanel from "../components/panels/HeadsetPanel.vue";
import KeymapPanel from "../components/panels/KeymapPanel.vue";
import MouseTuningPanel from "../components/panels/MouseTuningPanel.vue";
import AppMixerPanel from "../components/panels/AppMixerPanel.vue";
import { useDevicesStore } from "../stores/devices";
import { useProfilesStore } from "../stores/profiles";
import { useI18n } from "../lib/i18n";
import { compatibilitySummary } from "../lib/format";
import { deviceImage } from "../lib/deviceImages";

type StudioKind = "keyboard" | "mouse" | "audio";

const route = useRoute();
const router = useRouter();
const devices = useDevicesStore();
const profiles = useProfilesStore();
const { t } = useI18n();

const studios: Record<StudioKind, {
  titleKey: string;
  subtitleKey: string;
  kinds: string[];
  icon: Component;
  panel: Component;
  accent: string;
  rgb: boolean;
}> = {
  keyboard: {
    titleKey: "nav.keyboard",
    subtitleKey: "studio.keyboardSubtitle",
    kinds: ["keyboard"],
    icon: Keyboard,
    panel: KeyboardPanel,
    accent: "#ff4fa3",
    rgb: true,
  },
  mouse: {
    titleKey: "nav.mouse",
    subtitleKey: "studio.mouseSubtitle",
    kinds: ["mouse"],
    icon: Mouse,
    panel: MousePanel,
    accent: "#a78bfa",
    rgb: true,
  },
  audio: {
    titleKey: "nav.audio",
    subtitleKey: "studio.audioSubtitle",
    kinds: ["headset"],
    icon: Headphones,
    panel: HeadsetPanel,
    accent: "#6ee7f9",
    rgb: false,
  },
};

const kind = computed<StudioKind>(() => {
  const name = String(route.name ?? "keyboard");
  return name === "mouse" || name === "audio" ? name : "keyboard";
});
const studio = computed(() => studios[kind.value]);

const candidates = computed(() =>
  devices.devices
    .filter((d) => studio.value.kinds.includes(d.kind))
    .sort((a, b) => Number(b.online) - Number(a.online))
);

const selectedId = ref<string | null>(null);
const device = computed(
  () => candidates.value.find((d) => d.id === selectedId.value) ?? candidates.value[0] ?? null
);

const compat = computed(() => compatibilitySummary(device.value?.capabilities ?? []));
const photo = computed(() => (device.value ? deviceImage(device.value) : null));
const hex = (v: number | null) => (v === null ? "—" : `0x${v.toString(16).toUpperCase().padStart(4, "0")}`);

const canControl = computed(() => {
  const current = device.value;
  if (!current || !current.online) return false;
  const moduleStatus = (id: string) => current.modules?.find((m) => m.id === id)?.status;
  if (kind.value === "keyboard") {
    return current.adapterId === "redsquare" || moduleStatus("keyboard") === "working";
  }
  if (kind.value === "mouse") {
    return current.adapterId === "ajazz" || moduleStatus("mouse") === "working";
  }
  return true;
});

function resolveSelection() {
  const query = typeof route.query.device === "string" ? route.query.device : null;
  if (query && candidates.value.some((d) => d.id === query)) {
    selectedId.value = query;
    return;
  }
  if (selectedId.value && candidates.value.some((d) => d.id === selectedId.value)) return;
  let remembered: string | null = null;
  try {
    remembered = localStorage.getItem(`udm.studio.${kind.value}`);
  } catch {}
  selectedId.value =
    candidates.value.find((d) => d.id === remembered)?.id ?? candidates.value[0]?.id ?? null;
}

watch([candidates, () => route.query.device, kind], resolveSelection, { immediate: true });

function selectDevice(id: string) {
  selectedId.value = id;
  try {
    localStorage.setItem(`udm.studio.${kind.value}`, id);
  } catch {}
  router.replace({ query: { ...route.query, device: id } });
}

function openCard() {
  if (device.value) router.push(`/devices/${device.value.id}`);
}
</script>

<template>
  <div>
    <PageHeader :title="t(studio.titleKey)" :subtitle="t(studio.subtitleKey)" :overline="t('studio.overline')">
      <template #actions>
        <button v-if="studio.rgb" class="btn" @click="router.push('/rgb')">
          <Palette :size="13" />
          {{ t("nav.rgb") }}
        </button>
        <button v-if="device" class="btn btn-primary" @click="openCard">
          <component :is="studio.icon" :size="13" />
          {{ t("studio.openCard") }}
          <ArrowUpRight :size="13" />
        </button>
      </template>
    </PageHeader>

    <div v-if="!candidates.length" class="panel p-2">
      <EmptyState :icon="studio.icon" :title="t('studio.noDevice')" :body="t('studio.noDeviceBody')">
        <button class="btn" @click="router.push('/devices')">
          <Usb :size="13" />
          {{ t("nav.devices") }}
        </button>
      </EmptyState>
    </div>

    <template v-else>
      <div class="flex items-center gap-2 mb-3 flex-wrap">
        <span class="text-[10.5px] uppercase tracking-[0.12em] text-dim mr-1">{{ t("studio.device") }}</span>
        <button
          v-for="d in candidates"
          :key="d.id"
          class="chip transition-colors"
          :class="device?.id === d.id ? 'border-accent/40 text-text bg-accent/[0.07]' : 'hover:border-white/15'"
          @click="selectDevice(d.id)"
        >
          <span class="status-dot" :class="d.online ? 'bg-ok' : 'bg-dim'"></span>
          {{ d.name }}
          <span class="text-dim">· {{ d.connectionLabel }}</span>
        </button>
        <span class="ml-auto text-[10.5px] text-dim">{{ t("studio.detectedOnly") }}</span>
      </div>

      <div v-if="device" class="panel p-5 mb-4 relative overflow-hidden">
        <div
          class="absolute inset-0 pointer-events-none opacity-70"
          :style="{ background: `radial-gradient(420px 160px at 18% 0%, ${studio.accent}12, transparent 72%)` }"
        ></div>
        <div class="grid grid-cols-[260px_1fr_auto] gap-5 items-center relative">
          <div class="h-[150px] flex items-center justify-center">
            <img
              v-if="photo"
              :src="photo"
              :alt="device.name"
              class="max-h-full max-w-full object-contain select-none pointer-events-none drop-shadow-[0_16px_28px_rgba(0,0,0,0.5)]"
              :class="device.kind === 'keyboard' ? 'w-[94%] rounded-xl' : 'h-full w-auto rounded-2xl'"
              draggable="false"
            />
            <KeyboardModel
              v-else-if="kind === 'keyboard'"
              variant="mini"
              :animate="false"
              :interactive="false"
              :primary="studio.accent"
              secondary="#a78bfa"
              :brightness="70"
            />
            <MouseModel
              v-else-if="kind === 'mouse'"
              compact
              :interactive="false"
              :accent="studio.accent"
              class="!h-[150px] !w-auto"
            />
            <HeadsetModel v-else compact :accent="studio.accent" class="max-h-[150px] w-auto" />
          </div>

          <div class="min-w-0">
            <div class="flex items-center gap-2 flex-wrap">
              <span class="chip" :class="device.online ? 'chip-ok' : ''">
                <span class="status-dot" :class="device.online ? 'bg-ok' : 'bg-dim'"></span>
                {{ device.online ? t("device.online") : t("device.offline") }}
              </span>
              <span class="chip">{{ device.connectionLabel }}</span>
              <span class="chip mono">{{ hex(device.vid) }}:{{ hex(device.pid) }}</span>
            </div>
            <div class="text-[20px] font-display font-semibold tracking-tight mt-3 truncate">{{ device.name }}</div>
            <div class="text-[11.5px] text-dim mt-0.5">{{ device.vendor }} · {{ device.model }}</div>

            <div class="mt-4 max-w-[420px]">
              <div class="flex items-center justify-between text-[10.5px] mb-1.5">
                <span class="text-dim">{{ t("database.compat") }}</span>
                <span class="mono text-muted">{{ compat.confirmed }} / {{ compat.total }}</span>
              </div>
              <div class="flex gap-1 h-[4px] rounded-full overflow-hidden bg-white/5">
                <span
                  v-for="(count, status) in compat.counts"
                  :key="status"
                  class="h-full"
                  :class="{
                    'bg-ok': status === 'working',
                    'bg-warn': status === 'partial',
                    'bg-info/70': status === 'research',
                    'bg-bad/70': status === 'unsupported',
                  }"
                  :style="{ flexGrow: count, flexBasis: 0 }"
                ></span>
              </div>
              <div class="flex gap-3 mt-2 text-[10px] text-dim flex-wrap">
                <span>{{ compat.counts.working }} {{ t("status.working").toLowerCase() }}</span>
                <span>{{ compat.counts.partial }} {{ t("status.partial").toLowerCase() }}</span>
                <span>{{ compat.counts.research }} {{ t("common.research").toLowerCase() }}</span>
                <span v-if="compat.counts.unsupported">{{ compat.counts.unsupported }} {{ t("status.unsupported").toLowerCase() }}</span>
              </div>
            </div>
          </div>

          <div class="flex flex-col items-end gap-2.5">
            <BatteryPill :battery="device.battery" />
            <span class="chip">
              {{ t("studio.profile") }}:
              <span class="text-muted">{{ profiles.active?.name ?? t("dashboard.noProfile") }}</span>
            </span>
            <span class="text-[10px] text-dim mono">{{ device.adapterLabel }}</span>
          </div>
        </div>
      </div>

      <component :is="studio.panel" v-if="device && canControl" :device="device" />
      <div v-else-if="device" class="card p-10 grid place-items-center text-center">
        <Lock :size="28" class="text-dim mb-3" />
        <div class="text-[14px] font-medium">{{ device.online ? t("device.protocolLocked") : t("device.offline") }}</div>
        <div class="text-[11.5px] text-dim mt-1 max-w-[460px]">{{ t("device.protocolLockedBody") }}</div>
        <button class="btn mt-4" @click="openCard">{{ t("studio.openCard") }}</button>
      </div>
    </template>

    <div class="mt-6">
      <div v-if="kind !== 'audio'" class="flex items-center gap-2 mb-3">
        <span class="section-title">{{ t("nav.input") }}</span>
        <span class="chip !text-[10px]">{{ t("input.osLevel") }}</span>
      </div>
      <KeymapPanel v-if="kind === 'keyboard'" kind="keyboard" />
      <template v-else-if="kind === 'mouse'">
        <MouseTuningPanel />
        <div class="mt-4"><KeymapPanel kind="mouse" /></div>
      </template>
      <div v-else class="mt-4"><AppMixerPanel /></div>
    </div>
  </div>
</template>
