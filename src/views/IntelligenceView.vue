<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import {
  Activity,
  Battery,
  BatteryCharging,
  BatteryWarning,
  ChevronDown,
  ChevronRight,
  Cpu,
  Database,
  FlaskConical,
  Gamepad2,
  Headphones,
  Keyboard,
  Mouse,
  RefreshCw,
  Usb,
  Wifi,
} from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import { api, isTauri } from "../lib/api";
import { useI18n } from "../lib/i18n";
import { useDevicesStore } from "../stores/devices";
import type {
  BatteryState,
  CaptureResult,
  Confidence,
  DeviceView,
  FeatureProbe,
  TelemetryProtocol,
  TelemetryReport,
} from "../lib/types";

const { t, locale } = useI18n();
const devices = useDevicesStore();

const selected = ref<string | null>(null);
const report = ref<TelemetryReport | null>(null);
const scanning = ref(false);
const loading = ref(false);
const error = ref<string | null>(null);
const expanded = ref<Set<string>>(new Set());
const openValues = ref<Set<string>>(new Set());
const probes = reactive<Record<string, FeatureProbe[]>>({});
const captures = reactive<Record<string, CaptureResult>>({});
const busy = reactive<Record<string, "feature" | "capture" | null>>({});
const actionErrors = reactive<Record<string, string>>({});

const sorted = computed(() =>
  [...devices.devices].sort((a, b) => Number(b.online) - Number(a.online) || a.name.localeCompare(b.name))
);
const device = computed<DeviceView | null>(() => devices.devices.find((item) => item.id === selected.value) ?? null);

function text(ru: string, en: string) {
  return locale.value === "ru" ? ru : en;
}

function toggleValue(key: string) {
  const next = new Set(openValues.value);
  if (next.has(key)) next.delete(key);
  else next.add(key);
  openValues.value = next;
}

function stateClass(state: BatteryState | string) {
  if (state === "real" || state === "full") return "chip-ok";
  if (state === "charging") return "chip-info";
  if (state === "estimated") return "chip-warn";
  if (state === "unsupported") return "chip-bad";
  return "";
}

function confidenceClass(confidence: Confidence) {
  if (confidence === "verified") return "chip-ok";
  if (confidence === "supported") return "chip-info";
  if (confidence === "experimental" || confidence === "estimated") return "chip-warn";
  if (confidence === "unsupported") return "chip-bad";
  return "";
}

function deviceIcon(kind: DeviceView["kind"]) {
  if (kind === "keyboard") return Keyboard;
  if (kind === "mouse") return Mouse;
  if (kind === "headset" || kind === "microphone") return Headphones;
  if (kind === "gamepad") return Gamepad2;
  return Usb;
}

function batteryPercent(item: DeviceView) {
  const battery = item.battery;
  if (!battery || battery.percent === null) return null;
  if (battery.status === "real" || battery.status === "charging" || battery.status === "estimated") return battery.percent;
  return null;
}

function valueLabel(labelRu: string, labelEn: string) {
  return text(labelRu, labelEn);
}

function fmtScan(at: number | null | undefined) {
  if (!at) return "—";
  return new Date(at).toLocaleString(locale.value === "ru" ? "ru-RU" : "en-GB", { hour12: false });
}

function usageHex(page: number, usage: number) {
  return `0x${page.toString(16).toUpperCase().padStart(4, "0")} : 0x${usage.toString(16).toUpperCase().padStart(2, "0")}`;
}

function toggle(path: string) {
  const next = new Set(expanded.value);
  if (next.has(path)) next.delete(path);
  else next.add(path);
  expanded.value = next;
}

async function loadCached(id: string) {
  report.value = null;
  error.value = null;
  if (!isTauri()) return;
  loading.value = true;
  try {
    report.value = await api.telemetryCached(id);
  } catch {
    report.value = null;
  } finally {
    loading.value = false;
  }
}

async function select(id: string) {
  selected.value = id;
  await loadCached(id);
}

async function scan() {
  const current = device.value;
  if (!current || scanning.value) return;
  if (!current.online) {
    error.value = t("intel.offline");
    return;
  }
  scanning.value = true;
  error.value = null;
  try {
    report.value = await api.telemetryScan(current.id);
    for (const key of Object.keys(probes)) delete probes[key];
    for (const key of Object.keys(captures)) delete captures[key];
    for (const key of Object.keys(actionErrors)) delete actionErrors[key];
    expanded.value = new Set();
    await devices.refreshPassive();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    scanning.value = false;
  }
}

async function readFeature(protocol: TelemetryProtocol) {
  const current = device.value;
  if (!current) return;
  busy[protocol.path] = "feature";
  delete actionErrors[protocol.path];
  try {
    probes[protocol.path] = await api.protocolFeatureSweep(current.id, protocol.path);
  } catch (e) {
    actionErrors[protocol.path] = e instanceof Error ? e.message : String(e);
  } finally {
    busy[protocol.path] = null;
  }
}

async function capture(protocol: TelemetryProtocol) {
  const current = device.value;
  if (!current) return;
  busy[protocol.path] = "capture";
  delete actionErrors[protocol.path];
  try {
    captures[protocol.path] = await api.protocolCapture(current.id, protocol.path, 1500);
  } catch (e) {
    actionErrors[protocol.path] = e instanceof Error ? e.message : String(e);
  } finally {
    busy[protocol.path] = null;
  }
}

onMounted(async () => {
  if (!devices.devices.length) await devices.refreshPassive();
  const preferred: DeviceView["kind"][] = ["keyboard", "mouse", "headset", "gamepad", "microphone", "hid", "audio", "other"];
  const first =
    preferred
      .map((kind) => sorted.value.find((item) => item.online && item.kind === kind))
      .find((item): item is DeviceView => Boolean(item)) ??
    sorted.value.find((item) => item.online) ??
    sorted.value[0];
  if (first) await select(first.id);
});
</script>

<template>
  <div>
    <PageHeader :title="t('nav.intelligence')" :subtitle="t('intel.subtitle')" :overline="t('intel.overline')">
      <template #actions>
        <button class="btn" :disabled="!device?.online || scanning" @click="scan">
          <RefreshCw :size="14" :class="scanning ? 'animate-spin' : ''" />
          {{ scanning ? t("intel.scanning") : t("intel.scan") }}
        </button>
      </template>
    </PageHeader>

    <div v-if="!isTauri()" class="card px-4 py-3 mb-4 text-[12px] text-warn/90">
      {{ t("common.onlyInApp") }}
    </div>

    <div v-if="error" class="card px-4 py-3 mb-4 border-bad/25 bg-bad/[0.05] text-[12px] text-bad">
      {{ error }}
    </div>

    <div class="grid grid-cols-1 xl:grid-cols-[300px_minmax(0,1fr)] gap-4 items-start">
      <aside class="card p-2.5">
        <div class="flex items-center gap-2 px-2 py-1.5 mb-1">
          <Cpu :size="13" class="text-accent" />
          <span class="text-[10.5px] uppercase tracking-[0.12em] text-dim">{{ t("intel.devices") }}</span>
          <span class="ml-auto mono text-[10px] text-dim/70">{{ sorted.length }}</span>
        </div>
        <div class="space-y-0.5 max-h-[70vh] overflow-y-auto scroll-region">
          <button
            v-for="item in sorted"
            :key="item.id"
            class="w-full flex items-center gap-2.5 rounded-xl px-2 py-2 text-left transition-colors"
            :class="selected === item.id ? 'bg-white/[0.06]' : 'hover:bg-white/[0.035]'"
            @click="select(item.id)"
          >
            <span
              class="grid place-items-center w-7 h-7 rounded-lg border shrink-0"
              :class="item.online ? 'border-accent/25 bg-accent/[0.08] text-accent' : 'border-white/[0.06] bg-white/[0.02] text-dim'"
            >
              <component :is="deviceIcon(item.kind)" :size="13" />
            </span>
            <span class="min-w-0 flex-1">
              <span class="block truncate text-[11.5px] font-medium" :class="item.online ? '' : 'text-dim'" :title="item.name">{{ item.name }}</span>
              <span class="block truncate text-[10px] text-dim" :title="`${item.vendor} · ${item.connectionLabel}`">{{ item.vendor }} · {{ item.connectionLabel }}</span>
            </span>
            <span v-if="batteryPercent(item) !== null" class="mono text-[10.5px] text-dim">{{ batteryPercent(item) }}%</span>
            <component :is="item.connection === 'wireless' ? Wifi : Usb" :size="11" class="text-dim/60 shrink-0" />
          </button>
          <p v-if="!sorted.length" class="px-2 py-6 text-center text-[11px] text-dim">{{ t("intel.noDevices") }}</p>
        </div>
      </aside>

      <div class="min-w-0 space-y-4">
        <div v-if="!device" class="card px-5 py-10 text-center text-[12px] text-dim">
          {{ t("intel.select") }}
        </div>

        <template v-else>
          <div class="card px-5 py-4">
            <div class="flex items-start gap-3 flex-wrap">
              <span class="grid place-items-center w-10 h-10 rounded-xl border border-accent/25 bg-accent/[0.08] text-accent shrink-0">
                <component :is="deviceIcon(device.kind)" :size="18" />
              </span>
              <div class="min-w-0">
                <div class="font-display text-[18px] font-semibold leading-tight truncate">{{ device.name }}</div>
                <div class="text-[11.5px] text-dim mt-0.5">
                  {{ device.vendor }} · {{ device.model }} · {{ device.connectionLabel }}
                </div>
              </div>
              <div class="ml-auto flex items-center gap-1.5 flex-wrap justify-end">
                <span class="chip" :class="device.online ? 'chip-ok' : 'chip-bad'">
                  <span class="status-dot" :class="device.online ? 'bg-ok' : 'bg-bad'"></span>
                  {{ device.online ? t("intel.online") : t("intel.offlineShort") }}
                </span>
                <span v-if="report" class="chip" :class="report.source === 'live' ? 'chip-accent' : 'chip-info'">
                  <Database v-if="report.source === 'cache'" :size="11" />
                  <Activity v-else :size="11" />
                  {{ report.source === "live" ? t("intel.live") : t("intel.cached") }}
                </span>
                <span v-if="report" class="text-[10.5px] text-dim">{{ t("intel.scannedAt") }} {{ fmtScan(report.scannedAt) }}</span>
              </div>
            </div>

            <div class="mt-3 flex items-center gap-1.5 flex-wrap">
              <span v-if="!report" class="chip chip-warn">{{ t("intel.noReport") }}</span>
              <span v-if="report" class="chip" :class="stateClass(report.batteryState)">
                <Battery :size="11" />
                {{ t("intel.battery") }}: {{ t(`intel.state_${report.batteryState}`) }}
                <template v-if="report.battery.percent !== null"> · {{ report.battery.percent }}%</template>
              </span>
              <span v-if="report?.battery.charging" class="chip chip-info">
                <BatteryCharging :size="11" />
                {{ t("intel.chargingNow") }}
              </span>
              <span v-if="report && report.protocols.length" class="chip">
                <Activity :size="11" />
                HID: {{ report.protocols.length }}
              </span>
              <span v-if="report?.receiver" class="chip" :class="report.receiver.protocolKnown ? 'chip-info' : 'chip-warn'">
                <Wifi :size="11" />
                {{ report.receiver.protocolKnown ? t("intel.protocolKnown") : t("intel.protocolUnknown") }}
              </span>
            </div>

            <p v-if="!report" class="mt-3 text-[11.5px] text-dim leading-relaxed">
              {{ loading ? t("intel.loadingCache") : t("intel.noReportHint") }}
            </p>
          </div>

          <div v-if="report" class="grid grid-cols-1 2xl:grid-cols-[minmax(0,1fr)_320px] gap-4 items-start">
            <div class="card px-5 py-4">
              <div class="section-title">{{ t("intel.sectionValues") }}</div>
              <p class="text-[10.5px] text-dim mb-1">{{ t("intel.valuesHint") }}</p>
              <div class="values">
                <div v-for="item in report.values" :key="item.key" class="value-row">
                  <div class="value-label">{{ valueLabel(item.labelRu, item.labelEn) }}</div>
                  <div class="value-main">
                    <span
                      class="value-text"
                      :class="{ empty: !item.value, open: openValues.has(item.key) }"
                      :title="item.value ?? ''"
                      @click="toggleValue(item.key)"
                    >
                      {{ item.value ?? t("intel.noValue") }}
                    </span>
                    <span class="chip !h-5 !text-[9.5px]" :class="confidenceClass(item.confidence)">
                      {{ t(`intel.conf_${item.confidence}`) }}
                    </span>
                  </div>
                  <div class="value-meta mono">{{ item.source }} · {{ item.method }}</div>
                </div>
              </div>
            </div>

            <div class="space-y-4">
              <div class="card px-5 py-4">
                <div class="section-title flex items-center gap-2">
                  <BatteryWarning :size="12" />
                  {{ t("intel.batteryTitle") }}
                </div>
                <div class="space-y-2 text-[11.5px]">
                  <div class="flex items-center justify-between gap-2">
                    <span class="text-dim">{{ t("intel.batteryState") }}</span>
                    <span class="chip" :class="stateClass(report.batteryState)">{{ t(`intel.state_${report.batteryState}`) }}</span>
                  </div>
                  <div class="flex items-center justify-between gap-2">
                    <span class="text-dim">{{ t("intel.percent") }}</span>
                    <span class="mono">{{ report.battery.percent !== null ? `${report.battery.percent}%` : "—" }}</span>
                  </div>
                  <div class="flex items-center justify-between gap-2">
                    <span class="text-dim">{{ t("intel.charging") }}</span>
                    <span class="mono">
                      {{ report.battery.charging === true ? t("intel.yes") : report.battery.charging === false ? t("intel.no") : t("intel.unknown") }}
                    </span>
                  </div>
                  <div class="flex items-center justify-between gap-2">
                    <span class="text-dim">{{ t("intel.source") }}</span>
                    <span class="mono text-right">{{ report.battery.source || "—" }}</span>
                  </div>
                  <p class="text-[10.5px] text-dim leading-relaxed pt-1">{{ t("intel.batteryHint") }}</p>
                </div>
              </div>

              <div v-if="report.receiver" class="card px-5 py-4">
                <div class="section-title flex items-center gap-2">
                  <Wifi :size="12" />
                  {{ t("intel.receiverTitle") }}
                </div>
                <div class="space-y-2 text-[11.5px]">
                  <div class="flex items-center justify-between gap-2">
                    <span class="text-dim">VID / PID</span>
                    <span class="mono">
                      {{ report.receiver.vid !== null ? report.receiver.vid.toString(16).toUpperCase().padStart(4, "0") : "—" }}
                      /
                      {{ report.receiver.pid !== null ? report.receiver.pid.toString(16).toUpperCase().padStart(4, "0") : "—" }}
                    </span>
                  </div>
                  <div class="flex items-center justify-between gap-2">
                    <span class="text-dim">{{ t("intel.transport") }}</span>
                    <span class="mono text-right">{{ report.receiver.transport }}</span>
                  </div>
                  <div class="flex items-center justify-between gap-2">
                    <span class="text-dim">{{ t("intel.adapter") }}</span>
                    <span class="chip" :class="report.receiver.protocolKnown ? 'chip-info' : 'chip-warn'">
                      {{ report.receiver.adapter ?? t("intel.unknownProtocol") }}
                    </span>
                  </div>
                  <div class="flex items-center justify-between gap-2">
                    <span class="text-dim">{{ t("intel.batteryChannel") }}</span>
                    <span class="mono">{{ report.receiver.batteryChannel }}</span>
                  </div>
                  <div class="flex items-center justify-between gap-2">
                    <span class="text-dim">{{ t("intel.interfaces") }}</span>
                    <span class="mono">{{ report.receiver.interfaces }} · vendor {{ report.receiver.vendorInterfaces }}</span>
                  </div>
                  <p class="text-[10.5px] text-dim leading-relaxed pt-1">
                    {{ text(report.receiver.noteRu, report.receiver.noteEn) }}
                  </p>
                </div>
              </div>
            </div>
          </div>

          <div v-if="report" class="card px-5 py-4">
            <div class="section-title flex items-center gap-2">
              <FlaskConical :size="12" />
              {{ t("intel.protocolsTitle") }}
            </div>
            <p class="text-[10.5px] text-dim leading-relaxed mb-3">{{ t("intel.protocolsHint") }}</p>

            <div v-if="!report.protocols.length" class="text-[11.5px] text-dim py-4 text-center">{{ t("intel.noProtocols") }}</div>
            <div v-else class="space-y-2">
              <div v-for="protocol in report.protocols" :key="protocol.path" class="proto">
                <button class="proto-head" @click="toggle(protocol.path)">
                  <component :is="expanded.has(protocol.path) ? ChevronDown : ChevronRight" :size="13" class="text-dim shrink-0" />
                  <span class="mono text-[11px]">{{ usageHex(protocol.usagePage, protocol.usage) }}</span>
                  <span v-if="protocol.interfaceNumber !== null" class="text-[10px] text-dim">iface {{ protocol.interfaceNumber }}</span>
                  <span v-if="protocol.vendorSpecific" class="chip chip-warn !h-5 !text-[9px]">vendor</span>
                  <span v-if="protocol.batteryField" class="chip chip-ok !h-5 !text-[9px]">battery</span>
                  <span class="ml-auto flex items-center gap-1.5 text-[10px] text-dim mono">
                    <span>in {{ protocol.inputReports.length }}</span>
                    <span>out {{ protocol.outputReports.length }}</span>
                    <span>feat {{ protocol.featureReports.length }}</span>
                  </span>
                </button>

                <div v-if="expanded.has(protocol.path)" class="proto-body">
                  <div class="grid grid-cols-2 sm:grid-cols-4 gap-2 text-[10.5px]">
                    <div><span class="text-dim">{{ t("intel.fields") }}:</span> <span class="mono">{{ protocol.fieldCount }}</span></div>
                    <div><span class="text-dim">{{ t("intel.descriptor") }}:</span> <span class="mono">{{ protocol.descriptorOk ? "ok" : "—" }}</span></div>
                    <div><span class="text-dim">{{ t("intel.featureReports") }}:</span> <span class="mono">{{ protocol.featureReports.join(", ") || "—" }}</span></div>
                    <div><span class="text-dim">{{ t("intel.batteryField") }}:</span> <span class="mono">{{ protocol.batteryField ?? "—" }}</span></div>
                  </div>
                  <p class="mt-2 text-[10.5px] text-dim leading-relaxed">{{ text(protocol.noteRu, protocol.noteEn) }}</p>
                  <div class="mt-2 flex items-center gap-1.5 flex-wrap">
                    <button
                      class="btn !h-7 !px-2.5 !text-[10.5px]"
                      :disabled="Boolean(busy[protocol.path])"
                      @click="readFeature(protocol)"
                    >
                      {{ busy[protocol.path] === "feature" ? t("intel.reading") : t("intel.readFeature") }}
                    </button>
                    <button
                      class="btn !h-7 !px-2.5 !text-[10.5px]"
                      :disabled="Boolean(busy[protocol.path])"
                      @click="capture(protocol)"
                    >
                      {{ busy[protocol.path] === "capture" ? t("intel.capturing") : t("intel.capture") }}
                    </button>
                    <span class="text-[9.5px] text-dim/80">{{ t("intel.rawOnly") }}</span>
                  </div>

                  <p v-if="actionErrors[protocol.path]" class="mt-2 text-[10.5px] text-bad">{{ actionErrors[protocol.path] }}</p>

                  <div v-if="probes[protocol.path]" class="mt-2.5 space-y-1">
                    <div class="text-[9.5px] uppercase tracking-[0.1em] text-dim">{{ t("intel.featureResult") }}</div>
                    <div v-for="probe in probes[protocol.path]" :key="`${probe.reportId}-${probe.at}`" class="raw-line">
                      <span class="text-dim">report {{ probe.reportId }} · {{ probe.length }} B</span>
                      <span class="mono break-all">{{ probe.error ?? probe.data }}</span>
                    </div>
                    <p v-if="!probes[protocol.path].length" class="text-[10.5px] text-dim">{{ t("intel.noFeatureReports") }}</p>
                  </div>

                  <div v-if="captures[protocol.path]" class="mt-2.5">
                    <div class="text-[9.5px] uppercase tracking-[0.1em] text-dim mb-1">
                      {{ t("intel.captureResult") }} · {{ captures[protocol.path].samples.length }} {{ t("intel.samples") }}
                    </div>
                    <p v-if="!captures[protocol.path].samples.length" class="text-[10.5px] text-dim">{{ t("intel.noSamples") }}</p>
                    <div v-else class="space-y-1 max-h-[190px] overflow-y-auto scroll-region">
                      <div v-for="(sample, index) in captures[protocol.path].samples.slice(0, 24)" :key="`${sample.at}-${index}`" class="raw-line">
                        <span class="text-dim shrink-0">+{{ sample.at - (captures[protocol.path].samples.at(0)?.at ?? sample.at) }} ms</span>
                        <span class="text-dim shrink-0">r{{ sample.reportId }}</span>
                        <span class="mono break-all">{{ sample.hex }}</span>
                        <span v-if="sample.repeats > 1" class="text-dim shrink-0">×{{ sample.repeats }}</span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div v-if="report?.notes.length" class="card px-5 py-4">
            <div class="section-title">{{ t("intel.notesTitle") }}</div>
            <div class="space-y-1.5">
              <div
                v-for="(note, index) in report.notes"
                :key="index"
                class="flex gap-2 text-[11px] leading-relaxed"
                :class="note.level === 'warn' ? 'text-warn/90' : note.level === 'error' ? 'text-bad' : 'text-muted'"
              >
                <span class="status-dot shrink-0 mt-1.5" :class="note.level === 'warn' ? 'bg-warn' : note.level === 'error' ? 'bg-bad' : 'bg-info'"></span>
                <span>{{ text(note.ru, note.en) }}</span>
              </div>
            </div>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.section-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 10.5px;
  letter-spacing: 0.13em;
  text-transform: uppercase;
  color: var(--color-dim);
  margin-bottom: 10px;
}

.values {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 2px 18px;
}

.value-row {
  padding: 7px 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.04);
  min-width: 0;
}

.value-label {
  font-size: 10px;
  color: var(--color-dim);
  letter-spacing: 0.04em;
}

.value-main {
  display: flex;
  align-items: center;
  gap: 7px;
  margin-top: 2px;
  min-width: 0;
}

.value-text {
  font-size: 12px;
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  cursor: pointer;
}

.value-text.open {
  white-space: normal;
  word-break: break-all;
}

.value-text.empty {
  color: var(--color-dim);
  font-weight: 400;
}

.value-meta {
  margin-top: 2px;
  font-size: 9px;
  color: color-mix(in srgb, var(--color-dim) 75%, transparent);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.proto {
  border-radius: 12px;
  border: 1px solid rgba(255, 255, 255, 0.055);
  background: rgba(255, 255, 255, 0.02);
  overflow: hidden;
}

.proto-head {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 7px 10px;
  text-align: left;
  transition: background-color 120ms ease;
}

.proto-head:hover {
  background: rgba(255, 255, 255, 0.03);
}

.proto-body {
  padding: 2px 10px 10px 31px;
  border-top: 1px solid rgba(255, 255, 255, 0.04);
}

.raw-line {
  display: flex;
  gap: 8px;
  align-items: baseline;
  font-size: 10px;
  line-height: 1.5;
  padding: 2px 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.03);
}
</style>
