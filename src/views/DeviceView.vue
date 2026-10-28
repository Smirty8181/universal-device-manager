<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ArrowLeft, BatteryCharging, Bluetooth, CircuitBoard, ClipboardCopy, FileDown, FlaskConical, Keyboard, Lock, Mouse, Headphones, Usb, Gamepad2 } from "lucide-vue-next";
import { useDevicesStore } from "../stores/devices";
import { useAppStore } from "../stores/app";
import { useI18n } from "../lib/i18n";
import { deviceKindLabel, timeAgo, compatibilitySummary } from "../lib/format";
import { capabilityLabel } from "../lib/capabilities";
import { api, copyText, isTauri } from "../lib/api";
import type { GattServiceInfo } from "../lib/types";
import BatteryRing from "../components/BatteryRing.vue";
import CapabilityMatrix from "../components/CapabilityMatrix.vue";
import HeadsetModel from "../components/HeadsetModel.vue";
import LightingStudio from "../components/LightingStudio.vue";
import HidInterfacesList from "../components/HidInterfacesList.vue";
import InfoRow from "../components/InfoRow.vue";
import KeyboardModel from "../components/KeyboardModel.vue";
import MouseModel from "../components/MouseModel.vue";
import PageHeader from "../components/PageHeader.vue";
import StatusChip from "../components/StatusChip.vue";
import Tabs from "../components/Tabs.vue";
import KeyboardPanel from "../components/panels/KeyboardPanel.vue";
import MousePanel from "../components/panels/MousePanel.vue";
import HeadsetPanel from "../components/panels/HeadsetPanel.vue";
import { deviceImage } from "../lib/deviceImages";

type TabId = "overview" | "lighting" | "control" | "power" | "info" | "capabilities" | "diagnostics";

const route = useRoute();
const router = useRouter();
const devices = useDevicesStore();
const app = useAppStore();
const { t, locale } = useI18n();

const tab = ref<TabId>("overview");
const TAB_IDS: TabId[] = ["overview", "lighting", "control", "power", "info", "capabilities", "diagnostics"];
const queryTab = route.query.tab;
if (typeof queryTab === "string" && (TAB_IDS as string[]).includes(queryTab)) {
  tab.value = queryTab as TabId;
}
watch(
  () => route.query.tab,
  (next) => {
    if (typeof next === "string" && (TAB_IDS as string[]).includes(next)) tab.value = next as TabId;
  }
);
const probing = ref(false);
const probeResult = ref<string | null>(null);
const savingReport = ref(false);
const savedReport = ref<string | null>(null);
const gattServices = ref<GattServiceInfo[]>([]);
const gattError = ref<string | null>(null);
const gattLoading = ref(false);

const device = computed(() => devices.getDevice(String(route.params.id)));
const photo = computed(() => (device.value ? deviceImage(device.value) : null));

const moduleStatus = (id: string) => device.value?.modules?.find((m) => m.id === id)?.status;
const keyboardControl = computed(
  () =>
    device.value?.kind === "keyboard" &&
    device.value.online &&
    (device.value.adapterId === "redsquare" || moduleStatus("keyboard") === "working")
);
const mouseControl = computed(
  () =>
    device.value?.kind === "mouse" &&
    device.value.online &&
    (device.value.adapterId === "ajazz" || moduleStatus("mouse") === "working")
);
const audioControl = computed(
  () => device.value?.online && ["headset", "audio", "microphone"].includes(device.value.kind)
);

const lightingCap = computed(() => device.value?.capabilities.find((cap) => cap.id === "lighting"));
const hasLighting = computed(
  () => device.value?.kind === "keyboard" && (lightingCap.value?.status === "working" || lightingCap.value?.status === "partial")
);

const tabs = computed<{ id: TabId; label: string; count?: number }[]>(() => {
  const list: { id: TabId; label: string; count?: number }[] = [
    { id: "overview", label: t("device.overview") },
  ];
  if (hasLighting.value) list.push({ id: "lighting", label: t("device.lighting") });
  list.push(
    { id: "control", label: t("device.control") },
    { id: "power", label: t("device.power") },
    { id: "info", label: t("device.info") },
    { id: "capabilities", label: t("device.capabilities"), count: device.value?.capabilities.length },
    { id: "diagnostics", label: t("device.diagnostics") }
  );
  return list;
});

const kindIcon = computed(() => {
  switch (device.value?.kind) {
    case "keyboard": return Keyboard;
    case "mouse": return Mouse;
    case "headset": return Headphones;
    case "gamepad": return Gamepad2;
    default: return Usb;
  }
});

const accent = computed(() => {
  if (device.value?.kind === "keyboard") return "#ff4fa3";
  if (device.value?.kind === "mouse") return "#a78bfa";
  return "#6ee7f9";
});

const compat = computed(() => compatibilitySummary(device.value?.capabilities ?? []));
const batteryPercent = computed(() => device.value?.battery?.percent ?? null);
const batteryCharging = computed(() => device.value?.battery?.charging === true);
const batteryNote = computed(() =>
  locale.value === "ru" ? device.value?.battery?.noteRu : device.value?.battery?.noteEn
);
const batteryStatus = computed(
  () => device.value?.battery?.status ?? (batteryPercent.value !== null ? "real" : "unknown")
);
const batteryStale = computed(() => batteryStatus.value === "disconnected" || batteryStatus.value === "unknown");
const batteryProven = computed(
  () => batteryPercent.value !== null && (batteryStatus.value === "real" || batteryStatus.value === "charging")
);
const batteryFallbackNote = computed(() =>
  batteryProven.value && device.value?.battery?.source === "vendor"
    ? t("device.batteryVendorBody")
    : t("device.batteryUnknownBody")
);
const batteryCaps = computed(
  () => device.value?.capabilities.filter((cap) => cap.id === "battery_level" || cap.id === "battery_light_modes") ?? []
);

const studioPath = computed(() => {
  switch (device.value?.kind) {
    case "keyboard": return "/keyboard";
    case "mouse": return "/mouse";
    case "headset":
    case "audio": return "/audio";
    default: return null;
  }
});

const studioLabel = computed(() => {
  switch (device.value?.kind) {
    case "keyboard": return t("nav.keyboard");
    case "mouse": return t("nav.mouse");
    case "headset":
    case "audio": return t("nav.audio");
    default: return t("studio.overline");
  }
});

const hex = (v: number | null) => (v === null ? "—" : `0x${v.toString(16).toUpperCase().padStart(4, "0")}`);

async function probeBattery() {
  const d = device.value;
  if (!d) return;
  probing.value = true;
  try {
    const battery = await devices.probeBattery(d.id);
    if (battery && battery.percent !== null) {
      probeResult.value = `${t("lab.batteryFound")}: ${battery.percent}% · ${t("battery.source." + battery.source)}`;
      app.toast("success", t("lab.batteryFound"), `${battery.percent}%`);
    } else {
      probeResult.value = battery && (locale.value === "ru" ? battery.noteRu : battery.noteEn)
        ? (locale.value === "ru" ? battery.noteRu : battery.noteEn)!
        : t("lab.batteryNoAnswer");
      app.toast("info", t("lab.batteryNoAnswer"));
    }
  } finally {
    probing.value = false;
  }
}

async function saveReport() {
  const d = device.value;
  if (!d || !isTauri()) return;
  savingReport.value = true;
  try {
    const path = await api.diagnosticsExport(d.id);
    savedReport.value = path;
    app.toast("success", t("device.reportSaved"), path);
  } catch (e) {
    app.toast("error", t("toast.error"), String(e));
  } finally {
    savingReport.value = false;
  }
}

async function inspectGatt() {
  const d = device.value;
  if (!d || !d.bluetoothAddress) return;
  gattLoading.value = true;
  gattError.value = null;
  try {
    gattServices.value = await api.bleInspect(d.id);
  } catch (e) {
    gattServices.value = [];
    gattError.value = String(e);
  } finally {
    gattLoading.value = false;
  }
}

watch(
  () => route.params.id,
  () => {
    gattServices.value = [];
    gattError.value = null;
    savedReport.value = null;
    probeResult.value = null;
  }
);

async function copyReport() {
  const d = device.value;
  if (!d) return;
  const lines = [
    `UDM device report`,
    `name: ${d.name}`,
    `vendor: ${d.vendor}`,
    `model: ${d.model}`,
    `kind: ${d.kind}`,
    `vid: ${hex(d.vid)}`,
    `pid: ${hex(d.pid)}`,
    `serial: ${d.serial ?? "n/a"}`,
    `connection: ${d.connection} (${d.connectionLabel})`,
    `adapter: ${d.adapterId} / ${d.adapterLabel}`,
    `registryId: ${d.registryId ?? "n/a"}`,
    `containerId: ${d.containerId ?? "n/a"}`,
    `bluetoothAddress: ${d.bluetoothAddress ?? "n/a"}`,
    `physicalPath: ${d.physicalPath ?? "n/a"}`,
    `driver: ${d.driver?.provider ?? "n/a"} ${d.driver?.version ?? ""} (${d.driver?.service ?? "n/a"})`,
    `battery: ${d.battery?.percent ?? "n/a"}% status=${d.battery?.status ?? "unknown"} source=${d.battery?.source ?? "n/a"}`,
    `interfaces:`,
    ...d.interfaces.map((i) => `  ${i.path} usage=${hex(i.usagePage)}:${i.usage} mi=${i.interfaceNumber ?? "-"}`),
    `modules:`,
    ...(d.modules ?? []).map((m) => `  ${m.id}: ${m.status}${m.provider ? ` (${m.provider})` : ""}`),
    `capabilities:`,
    ...d.capabilities.map((c) => `  ${c.id}: ${c.status}`),
  ];
  try {
    await copyText(lines.join("\n"));
    app.toast("success", t("device.reportCopied"));
  } catch (e) {
    app.toast("error", t("toast.error"), String(e));
  }
}

function openLab() {
  if (device.value) router.push({ path: "/diagnostics", query: { device: device.value.id } });
}
</script>

<template>
  <div v-if="device">
    <div class="flex items-center gap-3 mb-4">
      <button class="btn btn-ghost !h-8 !px-2.5" @click="router.push('/devices')">
        <ArrowLeft :size="14" />
        {{ t("device.backToDevices") }}
      </button>
      <div class="text-[11px] text-dim">
        {{ t("device.updatedAt") }} {{ timeAgo(device.updatedAt, locale) }} · {{ t("common.id") }} <span class="mono">{{ device.id }}</span>
      </div>
    </div>

    <PageHeader :title="device.name" :subtitle="`${device.vendor} · ${device.model}`" :overline="deviceKindLabel(device.kind, locale)">
      <template #actions>
        <button v-if="studioPath" class="btn" @click="router.push({ path: studioPath, query: { device: device.id } })">
          <component :is="kindIcon" :size="13" />
          {{ studioLabel }}
        </button>
        <button class="btn" @click="copyReport">
          <ClipboardCopy :size="13" />
          {{ t("device.copyReport") }}
        </button>
        <button v-if="isTauri()" class="btn" :disabled="savingReport" @click="saveReport">
          <FileDown :size="13" />
          {{ t("device.saveReport") }}
        </button>
        <button class="btn btn-primary" :disabled="probing" @click="probeBattery">
          <BatteryCharging :size="13" :class="probing ? 'animate-pulse' : ''" />
          {{ t("device.probeBattery") }}
        </button>
      </template>
    </PageHeader>

    <div class="card p-5 mb-5 grid grid-cols-[300px_1fr] gap-5 items-center">
      <div class="h-[170px] flex items-center justify-center">
        <img
          v-if="photo"
          :src="photo"
          :alt="device.name"
          class="max-h-full max-w-full object-contain select-none pointer-events-none drop-shadow-[0_18px_32px_rgba(0,0,0,0.5)]"
          :class="device.kind === 'keyboard' ? 'w-[94%] rounded-xl' : 'h-full w-auto rounded-2xl'"
          draggable="false"
        />
        <KeyboardModel v-else-if="device.kind === 'keyboard'" variant="mini" :animate="false" :interactive="false" :primary="accent" secondary="#a78bfa" :brightness="70" />
        <MouseModel v-else-if="device.kind === 'mouse'" compact :accent="accent" class="!h-[170px] !w-auto" />
        <HeadsetModel v-else-if="device.kind === 'headset'" compact :accent="accent" class="max-h-[170px] w-auto" />
        <div v-else class="w-20 h-20 rounded-2xl border border-white/10 bg-white/[0.03] grid place-items-center">
          <component :is="kindIcon" :size="30" class="text-dim" />
        </div>
      </div>
      <div>
        <div class="flex items-center gap-2 flex-wrap">
          <span class="chip" :class="device.online ? 'chip-ok' : ''">
            <span class="status-dot" :class="device.online ? 'bg-ok' : 'bg-dim'"></span>
            {{ device.online ? t("device.online") : t("device.offline") }}
          </span>
          <span class="chip">{{ device.connectionLabel }}</span>
          <span class="chip">{{ hex(device.vid) }}:{{ hex(device.pid) }}</span>
          <span class="chip">{{ t("compat.short", { n: compat.confirmed, m: compat.total }) }}</span>
          <span v-if="device.serial" class="chip mono">{{ device.serial }}</span>
        </div>

        <div class="grid grid-cols-4 gap-2 mt-4">
          <div class="card-flat px-3 py-2.5">
            <div class="text-[10px] text-dim">{{ t("status.working") }}</div>
            <div class="font-display text-[20px] leading-tight mt-0.5">{{ compat.counts.working }}</div>
          </div>
          <div class="card-flat px-3 py-2.5">
            <div class="text-[10px] text-dim">{{ t("status.partial") }}</div>
            <div class="font-display text-[20px] leading-tight mt-0.5">{{ compat.counts.partial }}</div>
          </div>
          <div class="card-flat px-3 py-2.5">
            <div class="text-[10px] text-dim">{{ t("common.research") }}</div>
            <div class="font-display text-[20px] leading-tight mt-0.5">{{ compat.counts.research }}</div>
          </div>
          <div class="card-flat px-3 py-2.5">
            <div class="text-[10px] text-dim">{{ t("device.interfaceCount") }}</div>
            <div class="font-display text-[20px] leading-tight mt-0.5">{{ device.interfaces.length }}</div>
          </div>
        </div>

        <div class="flex items-center justify-between mt-4 gap-3">
          <BatteryRing :battery="device.battery" :size="54" :stroke="5" />
          <span class="text-[11px] text-dim text-right">{{ device.adapterLabel }}</span>
        </div>
        <div v-if="probeResult" class="text-[10.5px] text-dim mt-2">{{ t("device.probeResult") }}: {{ probeResult }}</div>
      </div>
    </div>

    <Tabs v-model="tab" :tabs="tabs" />

    <template v-if="tab === 'overview'">
      <div class="grid grid-cols-12 gap-4">
        <div class="col-span-7 space-y-4">
          <div class="card p-5">
            <div class="section-title mb-2">{{ t("device.connectionInfo") }}</div>
            <InfoRow :label="t('common.connection')" :value="device.connectionLabel" />
            <InfoRow :label="'VID'" :value="hex(device.vid)" mono />
            <InfoRow :label="'PID'" :value="hex(device.pid)" mono />
            <InfoRow :label="t('common.serial')" :value="device.serial" mono />
            <InfoRow :label="t('device.physicalPath')" :value="device.physicalPath" mono wrap />
            <InfoRow :label="t('device.adapter')" :value="device.adapterLabel" />
          </div>

          <div class="card p-5">
            <div class="flex items-center justify-between mb-2">
              <div class="section-title">{{ t("device.driverInfo") }}</div>
              <StatusChip :status="device.driver ? 'working' : 'research'" compact />
            </div>
            <InfoRow :label="t('common.version')" :value="device.driver?.version" mono />
            <InfoRow :label="t('common.vendor')" :value="device.driver?.provider" />
            <InfoRow :label="t('device.driverDate')" :value="device.driver?.date" />
            <InfoRow :label="t('device.service')" :value="device.driver?.service" mono />
          </div>

          <div class="card p-5">
            <div class="section-title mb-2">{{ t("device.hidInterfaces") }}</div>
            <HidInterfacesList :interfaces="device.interfaces" />
          </div>
        </div>

        <div class="col-span-5 space-y-4">
          <div class="card p-5">
            <div class="flex items-center justify-between mb-2">
              <div class="section-title">{{ t("device.capabilityMatrix") }}</div>
              <button class="text-[11px] text-dim hover:text-text transition-colors" @click="tab = 'capabilities'">{{ t("common.details") }}</button>
            </div>
            <div v-for="cap in device.capabilities.slice(0, 6)" :key="cap.id" class="flex items-center justify-between gap-3 py-2 border-b border-white/[0.035] last:border-0">
              <span class="text-[11.5px] text-muted truncate" :title="capabilityLabel(cap.id, locale)">{{ capabilityLabel(cap.id, locale) }}</span>
              <StatusChip :status="cap.status" compact />
            </div>
          </div>

          <div class="card p-5">
            <div class="flex items-center justify-between mb-3">
              <div class="section-title">{{ t("device.batteryUnknownTitle") }}</div>
              <span
                class="chip !h-6"
                :class="batteryStatus === 'real' || batteryStatus === 'charging' ? 'chip-ok' : batteryStale ? '' : 'chip-warn'"
              >
                {{ t("battery.status." + batteryStatus) }}
              </span>
            </div>
            <BatteryRing :battery="device.battery" :size="76" show-source />
            <div class="text-[10.5px] text-dim leading-relaxed mt-2">{{ batteryNote || batteryFallbackNote }}</div>
          </div>

          <div class="card p-5">
            <div class="text-[11.5px] text-muted">{{ t("device.researchTitle") }}</div>
            <div class="text-[10.5px] text-dim leading-relaxed mt-1">{{ t("device.researchBody") }}</div>
            <button class="btn w-full mt-3" @click="tab = 'control'">{{ t("device.openSettingsTab") }}</button>
          </div>
        </div>
      </div>
    </template>

    <template v-else-if="tab === 'lighting'">
      <LightingStudio v-if="device.kind === 'keyboard'" :key="device.id" :device="device" />
    </template>

    <template v-else-if="tab === 'control'">
      <KeyboardPanel v-if="device.kind === 'keyboard' && keyboardControl" :device="device" />
      <MousePanel v-else-if="device.kind === 'mouse' && mouseControl" :device="device" />
      <HeadsetPanel v-else-if="device.kind === 'headset' && audioControl" :device="device" />
      <div
        v-else-if="device.kind === 'keyboard' || device.kind === 'mouse'"
        class="card p-10 grid place-items-center text-center"
      >
        <Lock :size="28" class="text-dim mb-3" />
        <div class="text-[14px] font-medium">{{ device.online ? t("device.protocolLocked") : t("device.offline") }}</div>
        <div class="text-[11.5px] text-dim mt-1 max-w-[460px]">{{ t("device.protocolLockedBody") }}</div>
        <div class="flex gap-2 mt-4">
          <button class="btn" @click="tab = 'diagnostics'">
            <CircuitBoard :size="13" />
            {{ t("device.openDiagnostics") }}
          </button>
          <button class="btn" @click="tab = 'capabilities'">{{ t("device.capabilities") }}</button>
        </div>
      </div>
      <div v-else class="card p-10 grid place-items-center text-center">
        <component :is="kindIcon" :size="28" class="text-dim mb-3" />
        <div class="text-[14px] font-medium">{{ t("device.noPanel") }}</div>
        <div class="text-[11.5px] text-dim mt-1 max-w-[420px]">{{ t("device.noPanelBody") }}</div>
      </div>
    </template>

    <template v-else-if="tab === 'power'">
      <div class="grid grid-cols-12 gap-4">
        <div class="col-span-7 space-y-4">
          <div class="panel p-6 flex items-center gap-6">
            <BatteryRing :battery="device.battery" :size="128" :stroke="9" :show-label="false" />
            <div class="min-w-0">
              <div class="overline">{{ batteryPercent !== null ? t("battery.ringLabel") : t("device.batteryUnknownTitle") }}</div>
              <div class="font-display text-[26px] leading-tight mt-1 tabular-nums" :class="batteryStale ? 'text-dim' : ''">
                {{ batteryPercent !== null ? `${batteryPercent}%` : t("battery.unknown") }}
              </div>
              <div v-if="batteryStale && batteryPercent !== null" class="text-[10px] text-dim mt-0.5">
                {{ t("battery.lastKnownAt", { p: batteryPercent, time: timeAgo(device.battery?.updatedAt ?? Date.now(), locale) }) }}
              </div>
              <div class="flex items-center gap-2 mt-1.5 flex-wrap">
                <span class="chip !h-6" :class="batteryStatus === 'real' ? 'chip-ok' : batteryStatus === 'charging' ? 'chip-warn' : ''">
                  {{ t("battery.status." + batteryStatus) }}
                </span>
                <span class="chip !h-6" :class="batteryCharging ? 'chip-warn' : ''">
                  <BatteryCharging :size="11" />
                  {{
                    batteryPercent === null
                      ? t("battery.unknown")
                      : batteryCharging
                        ? t("battery.chargerConnected")
                        : device.battery?.charging === false
                          ? t("battery.discharging")
                          : t("battery.noChargingData")
                  }}
                </span>
                <span v-if="device.battery" class="chip !h-6">
                  {{ t("battery.source." + device.battery.source) }}
                </span>
              </div>
              <div class="text-[10.5px] text-dim leading-relaxed mt-3 max-w-[360px]">{{ batteryNote || batteryFallbackNote }}</div>
              <button class="btn mt-4" :disabled="probing" @click="probeBattery">
                <BatteryCharging :size="13" :class="probing ? 'animate-pulse' : ''" />
                {{ t("device.probeBattery") }}
              </button>
              <div v-if="probeResult" class="card-flat px-3 py-2 mt-2 text-[11px] text-muted">{{ probeResult }}</div>
            </div>
          </div>

          <div class="panel p-5">
            <div class="section-title mb-2">{{ t("battery.knownSources") }}</div>
            <InfoRow :label="t('common.source')" :value="device.battery ? t('battery.source.' + device.battery.source) : t('battery.source.unavailable')" />
            <InfoRow :label="t('common.status')" :value="t('battery.status.' + batteryStatus)" />
            <InfoRow :label="t('battery.updated')" :value="device.battery ? timeAgo(device.battery.updatedAt, locale) : t('common.notAvailable')" />
            <button class="btn w-full mt-3" :disabled="probing" @click="probeBattery">
              <BatteryCharging :size="13" />
              {{ t("device.probeBattery") }}
            </button>
            <div class="text-[10.5px] text-dim leading-relaxed mt-2">{{ t("device.probeNote") }}</div>
          </div>
        </div>

        <div class="col-span-5 space-y-4">
          <div class="panel p-5">
            <div class="section-title mb-3">{{ t("device.capabilities") }}</div>
            <div v-if="!batteryCaps.length" class="text-[11px] text-dim">{{ t("device.batteryUnknownBody") }}</div>
            <div v-for="cap in batteryCaps" :key="cap.id" class="flex items-center justify-between gap-3 py-2 border-b border-white/[0.035] last:border-0">
              <span class="text-[11.5px] text-muted">{{ capabilityLabel(cap.id, locale) }}</span>
              <StatusChip :status="cap.status" compact />
            </div>
          </div>
          <div v-if="!batteryProven" class="panel p-5">
            <div class="flex items-start gap-2.5">
              <FlaskConical :size="15" class="text-accent shrink-0 mt-0.5" />
              <div class="text-[10.5px] text-dim leading-relaxed">{{ t("device.batteryUnknownBody") }}</div>
            </div>
          </div>
        </div>
      </div>
    </template>

    <template v-else-if="tab === 'info'">
      <div class="grid grid-cols-12 gap-4">
        <div class="col-span-7 space-y-4">
          <div class="card p-5">
            <div class="section-title mb-2">{{ t("device.connectionInfo") }}</div>
            <InfoRow :label="t('common.connection')" :value="device.connectionLabel" />
            <InfoRow :label="'VID'" :value="hex(device.vid)" mono />
            <InfoRow :label="'PID'" :value="hex(device.pid)" mono />
            <InfoRow :label="t('common.serial')" :value="device.serial" mono />
            <InfoRow :label="t('device.physicalPath')" :value="device.physicalPath" mono wrap />
            <InfoRow :label="t('device.adapter')" :value="device.adapterLabel" />
            <InfoRow :label="'Registry ID'" :value="device.registryId ?? t('common.notAvailable')" mono />
            <InfoRow :label="'Container ID'" :value="device.containerId ?? t('common.notAvailable')" mono />
            <InfoRow :label="'Bluetooth'" :value="device.bluetoothAddress ?? t('common.notAvailable')" mono />
          </div>
          <div class="card p-5">
            <div class="section-title mb-2">{{ t("device.hardware") }}</div>
            <InfoRow :label="t('common.model')" :value="device.model" />
            <InfoRow :label="t('common.vendor')" :value="device.vendor" />
            <InfoRow :label="t('common.firmware')" :value="device.firmware ?? t('common.notAvailable')" />
            <InfoRow :label="t('device.adapter')" :value="`${device.adapterId} · ${device.adapterLabel}`" />
            <InfoRow :label="t('device.updatedAt')" :value="timeAgo(device.updatedAt, locale)" />
          </div>
        </div>
        <div class="col-span-5 space-y-4">
          <div class="card p-5">
            <div class="flex items-center justify-between mb-2">
              <div class="section-title">{{ t("device.driverInfo") }}</div>
              <StatusChip :status="device.driver ? 'working' : 'research'" compact />
            </div>
            <InfoRow :label="t('common.version')" :value="device.driver?.version" mono />
            <InfoRow :label="t('common.vendor')" :value="device.driver?.provider" />
            <InfoRow :label="t('device.driverDate')" :value="device.driver?.date" />
            <InfoRow :label="t('device.service')" :value="device.driver?.service" mono />
          </div>
          <div class="card p-5">
            <div class="flex items-center justify-between mb-2">
              <div class="section-title">{{ t("device.hidInterfaces") }}</div>
              <span class="chip !h-6 mono">{{ device.interfaces.length }}</span>
            </div>
            <HidInterfacesList :interfaces="device.interfaces" />
          </div>
          <div class="card p-5">
            <div class="flex items-center justify-between mb-1">
              <div class="section-title">{{ t("device.modules") }}</div>
              <span class="chip !h-6 mono">{{ device.modules?.length ?? 0 }}</span>
            </div>
            <div class="text-[10.5px] text-dim leading-relaxed mb-2">{{ t("device.modulesBody") }}</div>
            <div v-for="module in device.modules ?? []" :key="module.id" class="py-1.5 border-b border-white/[0.035] last:border-0">
              <div class="flex items-center justify-between gap-2">
                <span class="text-[11.5px] text-muted">{{ module.label }}</span>
                <StatusChip :status="module.status" compact />
              </div>
              <div class="text-[10px] text-dim mt-0.5">
                {{ locale === "ru" ? module.noteRu : module.noteEn }}
                <span v-if="module.provider" class="mono"> · {{ module.provider }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </template>

    <template v-else-if="tab === 'capabilities'">
      <CapabilityMatrix :capabilities="device.capabilities" />
      <div class="card p-4 mt-4 flex items-start gap-3">
        <FlaskConical :size="15" class="text-accent shrink-0 mt-0.5" />
        <div class="text-[11px] text-dim leading-relaxed">{{ t("device.researchBody") }}</div>
      </div>
    </template>

    <template v-else>
      <div class="grid grid-cols-12 gap-4">
        <div class="col-span-7 space-y-4">
          <div class="card p-5">
            <div class="flex items-center justify-between mb-2">
              <div class="section-title">{{ t("device.hidInterfaces") }}</div>
              <span class="chip !h-6 mono">{{ device.interfaces.length }}</span>
            </div>
            <HidInterfacesList :interfaces="device.interfaces" />
          </div>

          <div class="card p-5">
            <div class="flex items-center justify-between mb-2">
              <div class="section-title">{{ t("device.experimentalTitle") }}</div>
              <StatusChip status="research" />
            </div>
            <div class="text-[11px] text-dim leading-relaxed">{{ t("device.experimentalBody") }}</div>
            <div class="flex gap-2 mt-3">
              <button class="btn flex-1" @click="openLab">
                <CircuitBoard :size="13" />
                {{ t("device.openLab") }}
              </button>
              <button v-if="isTauri()" class="btn flex-1" :disabled="savingReport" @click="saveReport">
                <FileDown :size="13" />
                {{ t("device.saveReport") }}
              </button>
            </div>
            <div v-if="savedReport" class="text-[10px] text-dim mono mt-2 break-all">{{ savedReport }}</div>
          </div>

          <div v-if="device.bluetoothAddress" class="card p-5">
            <div class="flex items-center justify-between mb-2">
              <div class="section-title flex items-center gap-2">
                <Bluetooth :size="14" class="text-accent" />
                {{ t("diagnostics.gatt") }}
              </div>
              <span class="chip !h-6 mono">{{ device.bluetoothAddress }}</span>
            </div>
            <button class="btn w-full" :disabled="gattLoading" @click="inspectGatt">
              <Bluetooth :size="13" :class="gattLoading ? 'animate-pulse' : ''" />
              {{ t("diagnostics.gattRead") }}
            </button>
            <div v-if="gattError" class="text-[10.5px] text-warn mt-2 break-words">{{ gattError }}</div>
            <div v-if="gattServices.length" class="mt-3 space-y-1.5">
              <div v-for="service in gattServices" :key="service.uuid" class="card-flat px-3 py-2">
                <div class="mono text-[11px]">{{ service.uuid }}</div>
                <div class="text-[10px] text-dim mono break-words">{{ service.characteristics.join(", ") || "—" }}</div>
              </div>
            </div>
            <div v-else-if="!gattError && !gattLoading" class="text-[10.5px] text-dim mt-2">{{ t("diagnostics.gattEmpty") }}</div>
          </div>
        </div>

        <div class="col-span-5 space-y-4">
          <div class="card p-5">
            <div class="flex items-center justify-between mb-2">
              <div class="section-title">{{ t("device.probeBattery") }}</div>
              <BatteryCharging :size="15" class="text-accent" />
            </div>
            <div class="text-[11px] text-dim leading-relaxed">{{ t("device.probeNote") }}</div>
            <div v-if="probeResult" class="card-flat px-3 py-2 mt-2 text-[11px] text-muted">{{ probeResult }}</div>
          </div>

          <div class="card p-5">
            <div class="text-[11.5px] text-muted">{{ t("device.note") }}</div>
            <div class="text-[10.5px] text-dim leading-relaxed mt-1">{{ t("device.researchBody") }}</div>
          </div>
        </div>
      </div>
    </template>
  </div>

  <div v-else class="card p-12 grid place-items-center text-center">
    <Usb :size="30" class="text-dim mb-3" />
    <div class="text-[15px] font-medium">{{ t("device.notFound") }}</div>
    <div class="text-[11.5px] text-dim mt-1">{{ t("device.notFoundBody") }}</div>
    <button class="btn mt-4" @click="router.push('/devices')">{{ t("device.backToDevices") }}</button>
  </div>
</template>
