<script setup lang="ts">
import { computed, onMounted, ref, watch, watchEffect } from "vue";
import { useRoute } from "vue-router";
import { Activity, BatteryCharging, Bluetooth, CircuitBoard, Copy, Cpu, Database, FileDown, FileText, FlaskConical, RefreshCw, Send, ShieldCheck, Terminal } from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import InfoRow from "../components/InfoRow.vue";
import Modal from "../components/Modal.vue";
import StatusChip from "../components/StatusChip.vue";
import { api, copyText, isTauri, openPath } from "../lib/api";
import { useAppStore } from "../stores/app";
import { useDevicesStore } from "../stores/devices";
import { useI18n } from "../lib/i18n";
import { formatDateTime, shortenHomePath, timeAgo } from "../lib/format";
import type { GattServiceInfo } from "../lib/types";

const { t, locale } = useI18n();
const route = useRoute();
const app = useAppStore();
const devices = useDevicesStore();

const selectedDeviceId = ref(typeof route.query.device === "string" ? route.query.device : "");
const selectedPath = ref("");
const reportId = ref(0);
const length = ref(32);
const writeHex = ref("");
const busy = ref(false);
const confirmSend = ref(false);
const result = ref<{ title: string; text: string; error: string | null } | null>(null);
const probeText = ref<string | null>(null);
const logs = ref("");
const loadingLogs = ref(false);
const savingReport = ref(false);
const savedReport = ref<string | null>(null);
const gattServices = ref<GattServiceInfo[]>([]);
const gattError = ref<string | null>(null);
const gattLoading = ref(false);

const device = computed(() => (selectedDeviceId.value ? devices.getDevice(selectedDeviceId.value) : undefined));
const interfaces = computed(() => {
  const list = device.value?.interfaces ?? [];
  return [...list].sort((a, b) => Number(b.usagePage >= 0xff00) - Number(a.usagePage >= 0xff00));
});
const vendorCount = computed(() => interfaces.value.filter((i) => i.usagePage >= 0xff00).length);
const selectedInterface = computed(() => interfaces.value.find((i) => i.path === selectedPath.value) ?? null);

const deviceOptions = computed(() => devices.devices.filter((d) => d.interfaces.length > 0));

watchEffect(() => {
  const list = deviceOptions.value;
  if (!list.length) return;
  if (!selectedDeviceId.value || !list.some((d) => d.id === selectedDeviceId.value)) {
    selectedDeviceId.value = list[0].id;
  }
});

watchEffect(() => {
  const first = interfaces.value[0]?.path ?? "";
  if (!interfaces.value.some((i) => i.path === selectedPath.value)) selectedPath.value = first;
});

function usageHex(page: number) {
  return `0x${page.toString(16).toUpperCase().padStart(2, "0")}`;
}

async function readFeature() {
  if (!device.value || !selectedInterface.value) return;
  busy.value = true;
  try {
    const r = await api.hidRead(device.value.id, selectedInterface.value.path, reportId.value, Math.max(1, Math.min(256, length.value)));
    result.value = { title: `${t("diagnostics.readFeature")} · report ${reportId.value}`, text: r.data, error: r.error };
    if (r.error) app.toast("error", t("toast.error"), r.error);
  } catch (e) {
    result.value = { title: t("diagnostics.readFeature"), text: "", error: String(e) };
    app.toast("error", t("toast.error"), String(e));
  } finally {
    busy.value = false;
  }
}

async function getDescriptor() {
  if (!device.value) return;
  busy.value = true;
  try {
    const text = await api.hidDescriptor(device.value.id);
    result.value = { title: t("diagnostics.descriptor"), text, error: null };
  } catch (e) {
    result.value = { title: t("diagnostics.descriptor"), text: "", error: String(e) };
    app.toast("error", t("toast.error"), String(e));
  } finally {
    busy.value = false;
  }
}

function askSend() {
  if (!writeHex.value.trim()) return;
  if (!/^[0-9a-fA-F\s]+$/.test(writeHex.value.trim())) {
    app.toast("warn", t("lab.invalidHex"));
    return;
  }
  confirmSend.value = true;
}

async function sendFeature() {
  if (!device.value || !selectedInterface.value) return;
  confirmSend.value = false;
  busy.value = true;
  try {
    const r = await api.hidWrite(device.value.id, selectedInterface.value.path, writeHex.value.trim());
    result.value = { title: t("diagnostics.sendFeature"), text: r.data, error: r.error };
    if (r.error) app.toast("error", t("toast.error"), r.error);
    else app.toast("success", t("lab.featureSent"));
  } catch (e) {
    result.value = { title: t("diagnostics.sendFeature"), text: "", error: String(e) };
    app.toast("error", t("toast.error"), String(e));
  } finally {
    busy.value = false;
  }
}

async function probeBattery() {
  if (!device.value) return;
  const battery = await devices.probeBattery(device.value.id);
  if (battery && battery.percent !== null) {
    probeText.value = `${battery.percent}% · ${t("battery.source." + battery.source)}`;
  } else {
    probeText.value = battery && (locale.value === "ru" ? battery.noteRu : battery.noteEn)
      ? (locale.value === "ru" ? battery.noteRu : battery.noteEn)!
      : t("lab.batteryNoAnswer");
  }
}

async function copyResult() {
  if (!result.value) return;
  const body = result.value.error ? `error: ${result.value.error}\n${result.value.text}` : result.value.text;
  try {
    await copyText(body);
    app.toast("success", t("toast.copy"));
  } catch (e) {
    app.toast("error", t("toast.error"), String(e));
  }
}

async function saveReport() {
  if (!device.value) return;
  savingReport.value = true;
  try {
    savedReport.value = await api.diagnosticsExport(device.value.id);
    app.toast("success", t("diagnostics.reportSaved"), shortenHomePath(savedReport.value));
  } catch (e) {
    app.toast("error", t("toast.error"), String(e));
  } finally {
    savingReport.value = false;
  }
}

async function inspectGatt() {
  if (!device.value) return;
  gattLoading.value = true;
  gattError.value = null;
  try {
    gattServices.value = await api.bleInspect(device.value.id);
  } catch (e) {
    gattServices.value = [];
    gattError.value = String(e);
  } finally {
    gattLoading.value = false;
  }
}

watch(selectedDeviceId, () => {
  gattServices.value = [];
  gattError.value = null;
  savedReport.value = null;
  probeText.value = null;
});

async function loadLogs() {
  loadingLogs.value = true;
  try {
    logs.value = await api.logTail(120);
  } catch {
    logs.value = "";
  } finally {
    loadingLogs.value = false;
  }
}

function openLog() {
  if (!app.systemInfo?.logFile) {
    app.toast("info", t("common.notAvailable"));
    return;
  }
  void openPath(app.systemInfo.logFile);
}

onMounted(async () => {
  await app.loadSystemInfo();
  await loadLogs();
});
</script>

<template>
  <div>
    <PageHeader :title="t('diagnostics.title')" :subtitle="t('diagnostics.subtitle')" :overline="t('nav.diagnostics')">
      <template #actions>
        <span class="chip !h-8" :class="isTauri() ? 'chip-ok' : 'chip-warn'">
          <ShieldCheck :size="12" />
          {{ isTauri() ? t("diagnostics.tauriOk") : t("diagnostics.tauriNo") }}
        </span>
        <button class="btn" :disabled="savingReport" @click="saveReport">
          <FileDown :size="13" />
          {{ t("diagnostics.saveReport") }}
        </button>
        <button class="btn" @click="app.loadSystemInfo">
          <RefreshCw :size="13" />
          {{ t("diagnostics.refreshInfo") }}
        </button>
      </template>
    </PageHeader>

    <div v-if="savedReport" class="text-[10.5px] text-dim mono mb-3 break-all">
      {{ t("diagnostics.reportSaved") }}: {{ savedReport }}
    </div>

    <div class="grid grid-cols-12 gap-4">
      <div class="col-span-7 space-y-4">
        <div class="panel p-5">
          <div class="flex items-center justify-between mb-1">
            <div class="section-title flex items-center gap-2">
              <FlaskConical :size="15" class="text-accent" />
              {{ t("diagnostics.hidLab") }}
            </div>
            <span class="chip chip-info !h-6"><CircuitBoard :size="11" /> {{ vendorCount }} {{ t("diagnostics.vendorInterfaces").toLowerCase() }}</span>
          </div>
          <div class="text-[10.5px] text-dim mb-4 leading-relaxed">{{ t("diagnostics.hidLabNote") }}</div>

          <div v-if="!deviceOptions.length" class="text-[11.5px] text-dim py-3">{{ t("diagnostics.noDevices") }}</div>

          <template v-else>
            <div class="grid grid-cols-2 gap-3">
              <div>
                <label class="block text-[10.5px] text-muted mb-1">{{ t("diagnostics.device") }}</label>
                <select v-model="selectedDeviceId" class="input !h-9 !text-[11.5px]">
                  <option v-for="d in deviceOptions" :key="d.id" :value="d.id">{{ d.name }}</option>
                </select>
              </div>
              <div>
                <label class="block text-[10.5px] text-muted mb-1">{{ t("diagnostics.selectInterface") }}</label>
                <select v-model="selectedPath" class="input !h-9 !text-[11.5px]">
                  <option v-for="i in interfaces" :key="i.path" :value="i.path">
                    {{ usageHex(i.usagePage) }}:{{ i.usage.toString(16).padStart(2, "0") }} · MI {{ i.interfaceNumber ?? "—" }}{{ i.usagePage >= 0xff00 ? " · vendor" : "" }}
                  </option>
                </select>
              </div>
            </div>

            <div v-if="selectedInterface" class="mt-2 flex items-center gap-2">
              <span class="chip !h-5 mono !text-[9px]">{{ usageHex(selectedInterface.usagePage) }}:{{ selectedInterface.usage.toString(16).padStart(2, "0") }}</span>
              <span v-if="selectedInterface.usagePage >= 0xff00" class="chip chip-accent !h-5 !text-[9px]">{{ t("lab.warning") }}</span>
              <span class="mono text-[9.5px] text-dim truncate" :title="selectedInterface.path">{{ selectedInterface.path }}</span>
            </div>

            <div class="grid grid-cols-[110px_110px_1fr] gap-2 mt-3 items-end">
              <div>
                <label class="block text-[10.5px] text-muted mb-1">{{ t("diagnostics.reportId") }}</label>
                <input v-model.number="reportId" type="number" min="0" max="255" class="input !h-8 mono !text-[11.5px]" />
              </div>
              <div>
                <label class="block text-[10.5px] text-muted mb-1">{{ t("diagnostics.length") }}</label>
                <input v-model.number="length" type="number" min="1" max="256" class="input !h-8 mono !text-[11.5px]" />
              </div>
              <div class="flex gap-1.5">
                <button class="btn flex-1 !h-8 !text-[11px]" :disabled="busy || !selectedInterface" @click="readFeature">
                  {{ t("diagnostics.readFeature") }}
                </button>
                <button class="btn !h-8 !text-[11px]" :disabled="busy || !device" @click="getDescriptor">{{ t("diagnostics.getDescriptor") }}</button>
              </div>
            </div>

            <div class="mt-3">
              <label class="block text-[10.5px] text-muted mb-1">{{ t("diagnostics.hexPayload") }}</label>
              <div class="flex gap-2">
                <input v-model="writeHex" class="input flex-1 mono !text-[11.5px]" placeholder="06 01 00 00" spellcheck="false" @keydown.enter="askSend" />
                <button class="btn btn-primary !h-9" :disabled="busy || !selectedInterface" @click="askSend">
                  <Send :size="12" />
                  {{ t("diagnostics.sendFeature") }}
                </button>
              </div>
              <div class="text-[10px] text-warn/80 mt-1.5">{{ t("diagnostics.dangerNote") }}</div>
            </div>

            <div class="mt-4">
              <div class="flex items-center justify-between mb-1">
                <span class="text-[10.5px] text-muted">{{ t("diagnostics.response") }}</span>
                <button class="text-[10.5px] text-dim hover:text-text flex items-center gap-1" :disabled="!result" @click="copyResult">
                  <Copy :size="10" />
                  {{ t("diagnostics.copyResult") }}
                </button>
              </div>
              <div v-if="result" class="card-flat p-3">
                <div class="text-[9.5px] text-dim mb-1">{{ result.title }}</div>
                <pre v-if="result.error" class="mono text-[10.5px] text-bad whitespace-pre-wrap break-all">{{ result.error }}</pre>
                <pre v-else class="mono text-[10.5px] text-muted whitespace-pre-wrap break-all max-h-[180px] overflow-auto scroll-region">{{ result.text || "—" }}</pre>
              </div>
              <div v-else class="text-[10.5px] text-dim py-2">{{ t("common.noData") }}</div>
            </div>
          </template>
        </div>

        <div class="panel p-5">
          <div class="flex items-center justify-between mb-2">
            <div class="section-title flex items-center gap-2">
              <Terminal :size="14" class="text-dim" />
              {{ t("diagnostics.logTail") }}
            </div>
            <div class="flex items-center gap-1.5">
              <button class="btn btn-ghost !h-7 !px-2" :disabled="loadingLogs" @click="loadLogs">
                <RefreshCw :size="12" :class="loadingLogs ? 'animate-spin' : ''" />
              </button>
              <button class="btn btn-ghost !h-7 !px-2" @click="openLog">
                <FileText :size="12" />
                <span class="text-[10.5px]">{{ t("diagnostics.openLog") }}</span>
              </button>
            </div>
          </div>
          <pre class="mono text-[10px] text-dim whitespace-pre-wrap break-all max-h-[220px] overflow-auto scroll-region leading-relaxed">{{ logs || t("common.noData") }}</pre>
        </div>
      </div>

      <div class="col-span-5 space-y-4">
        <div class="panel p-5">
          <div class="flex items-center gap-2 mb-2">
            <Cpu :size="14" class="text-accent-3" />
            <div class="section-title">{{ t("diagnostics.system") }}</div>
          </div>
          <InfoRow :label="t('diagnostics.appVersion')" :value="app.systemInfo?.appVersion ?? '—'" mono />
          <InfoRow :label="t('diagnostics.os')" :value="app.systemInfo?.osVersion ?? '—'" />
          <InfoRow :label="t('diagnostics.admin')" :value="app.systemInfo ? (app.systemInfo.admin ? t('diagnostics.adminYes') : t('diagnostics.adminNo')) : '—'" />
          <InfoRow :label="t('diagnostics.deviceCount')" :value="app.systemInfo?.deviceCount ?? devices.devices.length" mono />
          <InfoRow :label="t('diagnostics.adapters')" :value="app.systemInfo?.adapterCount ?? '—'" mono />
          <InfoRow :label="t('diagnostics.started')" :value="app.systemInfo ? formatDateTime(app.systemInfo.startedAt, locale) : '—'" />
          <InfoRow :label="t('diagnostics.dataDir')" :value="app.systemInfo?.dataDir ?? '—'" mono wrap />
          <InfoRow :label="t('diagnostics.logFile')" :value="app.systemInfo?.logFile ?? '—'" mono wrap />
          <div class="flex items-center gap-2 mt-3">
            <span class="status-dot" :class="app.systemInfo?.tauri ? 'bg-ok' : 'bg-warn'"></span>
            <span class="text-[10.5px] text-dim">{{ app.systemInfo?.tauri ? t("diagnostics.tauriOk") : t("diagnostics.tauriNo") }}</span>
          </div>
        </div>

        <div class="panel p-5">
          <div class="flex items-center justify-between mb-2">
            <div class="section-title flex items-center gap-2">
              <BatteryCharging :size="14" class="text-accent" />
              {{ t("diagnostics.probeBattery") }}
            </div>
            <StatusChip status="research" compact />
          </div>
          <select v-model="selectedDeviceId" class="input !h-9 !text-[11.5px]">
            <option v-for="d in deviceOptions" :key="d.id" :value="d.id">{{ d.name }}</option>
          </select>
          <button class="btn w-full mt-2" :disabled="!device" @click="probeBattery">{{ t("diagnostics.probeBattery") }}</button>
          <div v-if="probeText" class="card-flat px-3 py-2 mt-2 text-[10.5px] text-muted">{{ probeText }}</div>
        </div>

        <div class="panel p-5">
          <div class="flex items-center justify-between mb-2">
            <div class="section-title flex items-center gap-2">
              <Bluetooth :size="14" class="text-accent" />
              {{ t("diagnostics.gatt") }}
            </div>
            <span v-if="device?.bluetoothAddress" class="chip !h-6 mono">{{ device.bluetoothAddress }}</span>
          </div>
          <button class="btn w-full" :disabled="!device?.bluetoothAddress || gattLoading" @click="inspectGatt">
            <Bluetooth :size="13" :class="gattLoading ? 'animate-pulse' : ''" />
            {{ t("diagnostics.gattRead") }}
          </button>
          <div v-if="gattError" class="text-[10.5px] text-warn mt-2 break-words">{{ gattError }}</div>
          <div v-if="gattServices.length" class="mt-3 space-y-1.5 max-h-[260px] overflow-y-auto scroll-region">
            <div v-for="service in gattServices" :key="service.uuid" class="card-flat px-3 py-2">
              <div class="mono text-[11px]">{{ service.uuid }}</div>
              <div class="text-[10px] text-dim mono break-words">{{ service.characteristics.join(", ") || "—" }}</div>
            </div>
          </div>
          <div v-else-if="!gattError && !gattLoading" class="text-[10.5px] text-dim mt-2">{{ t("diagnostics.gattEmpty") }}</div>
        </div>

        <div class="panel p-5">
          <div class="flex items-center gap-2 mb-2">
            <Activity :size="14" class="text-dim" />
            <div class="section-title">{{ t("diagnostics.events") }}</div>
          </div>
          <div v-if="!app.activity.length" class="text-[11px] text-dim py-2">{{ t("diagnostics.noEvents") }}</div>
          <div v-else class="space-y-1 max-h-[280px] overflow-y-auto scroll-region">
            <div v-for="ev in app.activity.slice(0, 14)" :key="ev.id" class="flex items-start gap-2 py-1.5 border-b border-white/[0.035] last:border-0">
              <span class="status-dot mt-1.5 shrink-0" :class="ev.severity === 'warn' ? 'bg-warn' : ev.severity === 'error' ? 'bg-bad' : ev.severity === 'success' ? 'bg-ok' : 'bg-info'"></span>
              <div class="min-w-0 flex-1">
                <div class="text-[10.5px] text-muted truncate" :title="ev.text">{{ ev.text }}</div>
                <div class="text-[9px] text-dim mono">{{ timeAgo(ev.at, locale) }}</div>
              </div>
            </div>
          </div>
          <div class="flex items-center gap-2 mt-3 text-[10px] text-dim">
            <Database :size="11" />
            udm.activity
          </div>
        </div>
      </div>
    </div>

    <Modal :open="confirmSend" :title="t('diagnostics.labConfirm')" width="420px" @close="confirmSend = false">
      <div class="text-[11.5px] text-dim leading-relaxed">{{ t("diagnostics.labConfirmBody") }}</div>
      <pre class="mono text-[11px] mt-3 card-flat p-3 whitespace-pre-wrap break-all">{{ writeHex }}</pre>
      <template #footer>
        <button class="btn" @click="confirmSend = false">{{ t("common.cancel") }}</button>
        <button class="btn btn-primary" @click="sendFeature">{{ t("diagnostics.sendAnyway") }}</button>
      </template>
    </Modal>
  </div>
</template>
