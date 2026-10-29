<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { ArrowUpRight, BadgeCheck, Check, CircleAlert, Download, ExternalLink, Fingerprint, FolderOpen, HardDrive, History, Link2, RefreshCw, ShieldCheck, ShieldQuestion, TriangleAlert } from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import EmptyState from "../components/EmptyState.vue";
import Modal from "../components/Modal.vue";
import { api, openExternal, openPath } from "../lib/api";
import { useAppStore } from "../stores/app";
import { useI18n } from "../lib/i18n";
import { formatDateTime, shortenHomePath } from "../lib/format";
import type { AutoDownloadReport, CatalogPackage, CatalogUpdate, DriverCandidate, DriverEntry, DriverResolution, FirmwareInfo, StoreDriver } from "../lib/types";

const { t, locale } = useI18n();
const app = useAppStore();

const entries = ref<DriverEntry[]>([]);
const history = ref<{ at: number; instanceId: string; name: string; result: string }[]>([]);
const loading = ref(false);
const scanning = ref(false);
const verifyingId = ref<string | null>(null);
const filter = ref<"all" | "problems">("all");
const filters = ["all", "problems"] as const;

const resolution = ref<DriverResolution | null>(null);
const firmware = ref<FirmwareInfo | null>(null);
const resolvingId = ref<string | null>(null);
const detailOpen = ref(false);
const selected = ref<DriverCandidate | null>(null);
const confirmed = ref(false);
const installing = ref(false);
const installNote = ref<string | null>(null);

const storeDrivers = ref<StoreDriver[]>([]);
const storeLoading = ref(false);

const catalogUpdates = ref<CatalogUpdate[]>([]);
const catalogLoading = ref(false);
const catalogError = ref<string | null>(null);
const downloadingId = ref<string | null>(null);
const downloaded = ref<CatalogPackage | null>(null);
const autoReport = ref<AutoDownloadReport | null>(null);
const autoLoading = ref(false);

const filtered = computed(() => (filter.value === "problems" ? entries.value.filter((e) => e.problemCode !== 0) : entries.value));
const problemCount = computed(() => entries.value.filter((e) => e.problemCode !== 0).length);
const signedCount = computed(() => entries.value.filter((e) => e.signature === "windows" || e.signature === "publisher").length);
const unsignedCount = computed(() => entries.value.filter((e) => e.signature === "unsigned").length);
const installable = computed(() => {
  if (!selected.value) return false;
  return selected.value.signature === "windows" || selected.value.signature === "publisher";
});
const isAdmin = computed(() => app.systemInfo?.admin === true);
const firmwareSignature = computed(() => signatureInfo((firmware.value?.signature ?? "unknown") as DriverEntry["signature"]));

function signatureInfo(sig: DriverEntry["signature"]) {
  switch (sig) {
    case "windows":
      return { label: t("drivers.signedWindows"), chip: "chip-ok" };
    case "publisher":
      return { label: t("drivers.signedPublisher"), chip: "chip-ok" };
    case "unsigned":
      return { label: t("drivers.unsigned"), chip: "chip-bad" };
    default:
      return { label: t("drivers.unknownSignature"), chip: "" };
  }
}

async function loadHistory() {
  try {
    history.value = await api.driversHistory();
  } catch {
    history.value = [];
  }
}

async function scan(quiet = false) {
  scanning.value = true;
  try {
    entries.value = await api.driversScan();
    await loadHistory();
    if (!quiet) app.toast("success", t("toast.scanDone"), `${entries.value.length} ${t("diagnostics.deviceCount").toLowerCase()}`);
  } catch (e) {
    app.toast("error", t("toast.error"), String(e));
  } finally {
    scanning.value = false;
    loading.value = false;
  }
}

async function verify(entry: DriverEntry) {
  verifyingId.value = entry.instanceId;
  try {
    const updated = await api.driverVerify(entry.instanceId);
    const idx = entries.value.findIndex((e) => e.instanceId === entry.instanceId);
    if (idx >= 0) entries.value[idx] = updated;
    app.toast("success", t("drivers.signatureChecked"), signatureInfo(updated.signature).label);
    await loadHistory();
  } catch (e) {
    app.toast("error", t("toast.error"), String(e));
  } finally {
    verifyingId.value = null;
  }
}

async function resolve(entry: DriverEntry) {
  resolvingId.value = entry.instanceId;
  installNote.value = null;
  confirmed.value = false;
  selected.value = null;
  catalogUpdates.value = [];
  catalogError.value = null;
  downloaded.value = null;
  try {
    const [result, fw] = await Promise.all([api.driverResolve(entry.instanceId), api.firmwareInfo(entry.instanceId)]);
    resolution.value = result;
    firmware.value = fw;
    selected.value =
      result.candidates.find((c) => !c.isInstalled && (c.signature === "windows" || c.signature === "publisher")) ??
      result.candidates[0] ??
      null;
    detailOpen.value = true;
  } catch (e) {
    app.toast("error", t("toast.error"), String(e));
  } finally {
    resolvingId.value = null;
  }
}

async function searchCatalog() {
  const hardwareId = resolution.value?.hardwareIds[0];
  if (!hardwareId) return;
  catalogLoading.value = true;
  catalogError.value = null;
  catalogUpdates.value = [];
  try {
    catalogUpdates.value = await api.driverCatalog(hardwareId);
    if (!catalogUpdates.value.length) catalogError.value = t("drivers.catalogEmpty");
  } catch (e) {
    catalogError.value = String(e);
  } finally {
    catalogLoading.value = false;
  }
}

async function downloadUpdate(update: CatalogUpdate) {
  downloadingId.value = update.updateId;
  installNote.value = null;
  try {
    const pkg = await api.driverCatalogDownload(update.updateId, update.title);
    downloaded.value = pkg;
    const candidate: DriverCandidate = {
      infPath: pkg.infs[0],
      publishedName: null,
      description: pkg.title,
      provider: "Microsoft Update Catalog",
      version: update.version,
      date: update.date,
      className: "",
      signature: pkg.signature === "windows" || pkg.signature === "publisher" ? pkg.signature : "unknown",
      exactHwidMatch: true,
      newerThanInstalled: true,
      isInstalled: false,
    };
    selected.value = candidate;
    confirmed.value = false;
    installNote.value = t("drivers.catalogInstallNote");
    app.toast("success", t("toast.catalogDownloaded"), pkg.title);
  } catch (e) {
    catalogError.value = String(e);
    app.toast("error", t("toast.error"), String(e));
  } finally {
    downloadingId.value = null;
  }
}

async function runAutoDownload() {
  autoLoading.value = true;
  try {
    autoReport.value = await api.driverAutoDownload();
    if (autoReport.value.downloaded.length) {
      app.toast("success", t("drivers.autoDownload"), t("drivers.autoFound", { count: autoReport.value.downloaded.length }));
    }
  } catch (e) {
    autoReport.value = { enabled: true, checked: 0, downloaded: [], skipped: [String(e)] };
  } finally {
    autoLoading.value = false;
  }
}

async function install() {
  const candidate = selected.value;
  if (!candidate || !resolution.value || !confirmed.value || !installable.value) return;
  installing.value = true;
  installNote.value = null;
  try {
    if (isAdmin.value) {
      await api.driverInstall(candidate, resolution.value.instanceId);
      installNote.value = t("drivers.installDone");
      app.toast("success", t("drivers.installDone"), candidate.publishedName ?? candidate.infPath);
      entries.value = await api.driversScan();
      await loadHistory();
    } else {
      await api.driverInstallElevated(candidate, resolution.value.instanceId);
      installNote.value = t("drivers.elevationNote");
      app.toast("success", t("drivers.elevationRequested"), candidate.publishedName ?? candidate.infPath);
    }
  } catch (e) {
    installNote.value = `${t("drivers.installFailed")}: ${String(e)}`;
    app.toast("error", t("drivers.installFailed"), String(e));
  } finally {
    installing.value = false;
  }
}

async function loadStore() {
  storeLoading.value = true;
  try {
    storeDrivers.value = await api.driverStore();
  } catch (e) {
    app.toast("error", t("toast.error"), String(e));
  } finally {
    storeLoading.value = false;
  }
}

async function openCatalog(entry: DriverEntry) {
  const query = entry.hardwareIds[0] ?? entry.name;
  await openExternal(`https://www.catalog.update.microsoft.com/Search.aspx?q=${encodeURIComponent(query)}`);
}

function closeDetail() {
  detailOpen.value = false;
}

onMounted(() => {
  loading.value = true;
  void scan(true).then(() => {
    if (app.settings.autoDownloadDrivers) void runAutoDownload();
  });
});
</script>

<template>
  <div>
    <PageHeader :title="t('drivers.title')" :subtitle="t('drivers.subtitle')" :overline="t('nav.drivers')">
      <template #actions>
        <span class="chip !h-8" :class="app.settings.autoDownloadDrivers ? 'chip-ok' : 'chip-warn'">
          <Download v-if="app.settings.autoDownloadDrivers" :size="12" />
          <ShieldQuestion v-else :size="12" />
          {{ app.settings.autoDownloadDrivers ? t("drivers.autoDownload") : t("drivers.manualInstall") }}
        </span>
        <button class="btn btn-primary" :disabled="scanning" @click="scan()">
          <RefreshCw :size="13" :class="scanning ? 'animate-spin' : ''" />
          {{ scanning ? t("drivers.scanning") : t("drivers.scan") }}
        </button>
      </template>
    </PageHeader>

    <div class="grid grid-cols-4 gap-3 mb-4">
      <div class="card-flat px-4 py-3">
        <div class="text-[10.5px] text-dim">{{ t("drivers.allDevices") }}</div>
        <div class="font-display text-[22px] leading-tight">{{ entries.length }}</div>
      </div>
      <div class="card-flat px-4 py-3">
        <div class="text-[10.5px] text-dim">{{ t("drivers.problems") }}</div>
        <div class="font-display text-[22px] leading-tight" :class="problemCount ? 'text-warn' : ''">{{ problemCount }}</div>
      </div>
      <div class="card-flat px-4 py-3">
        <div class="text-[10.5px] text-dim">{{ t("drivers.signature") }}</div>
        <div class="font-display text-[22px] leading-tight text-ok">{{ signedCount }}</div>
      </div>
      <div class="card-flat px-4 py-3">
        <div class="text-[10.5px] text-dim">{{ t("drivers.unsigned") }}</div>
        <div class="font-display text-[22px] leading-tight" :class="unsignedCount ? 'text-bad' : ''">{{ unsignedCount }}</div>
      </div>
    </div>

    <div class="grid grid-cols-12 gap-4">
      <div class="col-span-8">
        <div class="flex items-center gap-1.5 mb-3">
          <button
            v-for="f in filters"
            :key="f"
            class="h-8 px-3 rounded-lg text-[12px] border transition-colors"
            :class="filter === f ? 'border-white/15 bg-white/[0.07] text-text' : 'border-transparent text-dim hover:text-muted'"
            @click="filter = f"
          >
            {{ f === "all" ? t("drivers.allDevices") : t("drivers.problems") }}
          </button>
        </div>

        <div v-if="loading" class="panel p-8 flex items-center justify-center gap-3 text-[12px] text-dim">
          <RefreshCw :size="14" class="animate-spin" />
          {{ t("drivers.scanning") }}
        </div>

        <EmptyState v-else-if="!filtered.length" :icon="BadgeCheck" :title="t('drivers.noProblems')" :body="t('drivers.noProblemsBody')" />

        <div v-else class="space-y-2">
          <div v-for="entry in filtered" :key="entry.instanceId" class="panel p-4">
            <div class="flex items-start gap-3">
              <span class="w-9 h-9 rounded-lg grid place-items-center shrink-0 border" :class="entry.problemCode !== 0 ? 'border-warn/30 bg-warn/10' : 'border-white/[0.06] bg-white/[0.03]'">
                <TriangleAlert v-if="entry.problemCode !== 0" :size="15" class="text-warn" />
                <BadgeCheck v-else :size="15" class="text-ok" />
              </span>
              <div class="min-w-0 flex-1">
                <div class="flex items-center gap-2">
                  <span class="text-[12.5px] font-medium truncate" :title="entry.name">{{ entry.name }}</span>
                  <span class="chip !h-5 !text-[9.5px]">{{ entry.class }}</span>
                  <span v-if="entry.problemCode !== 0" class="chip chip-warn !h-5 !text-[9.5px]">
                    {{ t("drivers.problem") }} {{ entry.problemCode }}<template v-if="entry.problemLabel"> · {{ entry.problemLabel }}</template>
                  </span>
                </div>
                <div class="grid grid-cols-2 gap-3 mt-2 text-[10.5px] xl:grid-cols-3">
                  <div>
                    <div class="text-dim">{{ t("drivers.driverVersion") }}</div>
                    <div class="mono mt-0.5 truncate" :title="entry.driverVersion ?? ''">{{ entry.driverVersion ?? "—" }}</div>
                  </div>
                  <div>
                    <div class="text-dim">{{ t("drivers.driverProvider") }}</div>
                    <div class="mt-0.5 truncate" :title="entry.driverProvider ?? ''">{{ entry.driverProvider ?? "—" }}</div>
                  </div>
                  <div>
                    <div class="text-dim">{{ t("drivers.driverDate") }}</div>
                    <div class="mono mt-0.5 truncate" :title="entry.driverDate ?? ''">{{ entry.driverDate ?? "—" }}</div>
                  </div>
                </div>
                <div class="flex flex-wrap gap-1 mt-2">
                  <span v-for="id in entry.hardwareIds.slice(0, 2)" :key="id" class="chip !h-5 mono !text-[9px] max-w-[55%] truncate" :title="id">{{ id }}</span>
                </div>
              </div>
              <div class="flex flex-col items-end gap-1.5 shrink-0">
                <span class="chip !h-6 !text-[10px]" :class="signatureInfo(entry.signature).chip">
                  <ShieldCheck v-if="entry.signature === 'windows' || entry.signature === 'publisher'" :size="10" />
                  <CircleAlert v-else :size="10" />
                  {{ signatureInfo(entry.signature).label }}
                </span>
                <div class="flex items-center gap-1">
                  <button class="btn btn-primary !h-7 !px-2 !text-[10.5px]" :disabled="resolvingId === entry.instanceId" @click="resolve(entry)">
                    <Fingerprint :size="11" />
                    {{ resolvingId === entry.instanceId ? t("drivers.resolving") : t("drivers.resolve") }}
                  </button>
                  <button class="btn btn-ghost !h-7 !px-2 !text-[10.5px]" :disabled="verifyingId === entry.instanceId" @click="verify(entry)">
                    {{ t("drivers.checkSignature") }}
                  </button>
                  <button class="btn btn-ghost !h-7 !w-7 !p-0" :title="t('drivers.catalog')" @click="openCatalog(entry)">
                    <ExternalLink :size="12" />
                  </button>
                </div>
              </div>
            </div>
            <div class="flex items-center justify-between mt-2 pt-2 border-t border-white/[0.04]">
              <span class="mono text-[9.5px] text-dim truncate max-w-[70%]" :title="entry.instanceId">{{ entry.instanceId }}</span>
              <span class="mono text-[9.5px] text-dim truncate max-w-[28%]" :title="entry.driverInf ?? ''">{{ entry.driverInf ?? "—" }}</span>
            </div>
          </div>
        </div>
      </div>

      <div class="col-span-4 space-y-4">
        <div v-if="app.settings.autoDownloadDrivers" class="panel p-5">
          <div class="flex items-center justify-between mb-2">
            <div class="flex items-center gap-2">
              <Download :size="14" class="text-dim" />
              <div class="section-title">{{ t("drivers.autoDownload") }}</div>
            </div>
            <RefreshCw v-if="autoLoading" :size="13" class="animate-spin text-dim" />
          </div>
          <div v-if="autoLoading && !autoReport" class="text-[10.5px] text-dim">{{ t("drivers.catalogSearching") }}</div>
          <template v-else-if="autoReport">
            <div class="text-[10.5px] text-dim">{{ t("drivers.autoCheck", { count: autoReport.checked }) }}</div>
            <div v-if="autoReport.downloaded.length" class="mt-2 space-y-2">
              <div v-for="item in autoReport.downloaded" :key="item.dir" class="flex items-center gap-2">
                <span class="chip chip-ok !h-5 !text-[9px] shrink-0">{{ Math.round(item.sizeBytes / 1048576) }} MB</span>
                <div class="min-w-0 flex-1">
                  <div class="text-[11px] truncate">{{ item.title }}</div>
                  <div class="text-[9.5px] text-dim truncate">{{ item.device }}</div>
                </div>
                <button class="icon-btn shrink-0" :title="t('drivers.openFolder')" @click="openPath(item.dir)">
                  <FolderOpen :size="12" />
                </button>
              </div>
              <div class="text-[9.5px] text-dim leading-relaxed">{{ t("drivers.autoInstallHint") }}</div>
            </div>
            <div v-else class="text-[10.5px] text-dim mt-1">{{ t("drivers.autoNothing") }}</div>
            <div v-if="autoReport.skipped.length" class="mt-2 text-[9.5px] text-dim leading-relaxed">
              {{ autoReport.skipped.slice(0, 3).join(" · ") }}
            </div>
          </template>
        </div>

        <div class="panel p-5">
          <div class="flex items-center justify-between mb-2">
            <div class="section-title">{{ t("drivers.history") }}</div>
            <History :size="14" class="text-dim" />
          </div>
          <div v-if="!history.length" class="text-[11px] text-dim py-2">{{ t("drivers.noHistory") }}</div>
          <div v-else class="space-y-1.5">
            <div v-for="(h, i) in history.slice(0, 12)" :key="i" class="flex items-start gap-2 py-1.5 border-b border-white/[0.035] last:border-0">
              <span class="status-dot bg-ok mt-1.5 shrink-0"></span>
              <div class="min-w-0 flex-1">
                <div class="text-[11px] truncate">{{ h.name }}</div>
                <div class="text-[9.5px] text-dim">{{ h.result }} · {{ formatDateTime(h.at, locale) }}</div>
              </div>
            </div>
          </div>
        </div>

        <div class="panel p-5">
          <div class="flex items-center justify-between mb-2">
            <div class="flex items-center gap-2">
              <HardDrive :size="14" class="text-dim" />
              <div class="section-title">{{ t("drivers.store") }}</div>
            </div>
            <span v-if="storeDrivers.length" class="chip !h-5 !text-[9.5px]">{{ storeDrivers.length }}</span>
          </div>
          <button class="btn !h-7 text-[11px] w-full" :disabled="storeLoading" @click="loadStore">
            <RefreshCw :size="11" :class="storeLoading ? 'animate-spin' : ''" />
            {{ storeLoading ? t("drivers.storeLoading") : t("drivers.storeLoad") }}
          </button>
          <div v-if="storeDrivers.length" class="mt-3 space-y-1.5 max-h-[240px] overflow-y-auto scroll-region pr-1">
            <div v-for="pkg in storeDrivers.slice(0, 40)" :key="pkg.publishedName" class="py-1.5 border-b border-white/[0.035] last:border-0">
              <div class="flex items-center justify-between gap-2">
                <span class="mono text-[10px] truncate">{{ pkg.publishedName }}</span>
                <span class="mono text-[9.5px] text-dim shrink-0">{{ pkg.version }}</span>
              </div>
              <div class="text-[9.5px] text-dim truncate mt-0.5">{{ pkg.provider || pkg.className }} · {{ pkg.date ?? "—" }}</div>
            </div>
          </div>
          <div v-else-if="!storeLoading" class="text-[10.5px] text-dim mt-2">{{ t("drivers.storeEmpty") }}</div>
        </div>

        <div class="panel p-5">
          <div class="flex items-center gap-2 mb-2">
            <ShieldCheck :size="14" class="text-ok" />
            <div class="text-[11.5px] font-medium">{{ t("drivers.manualInstall") }}</div>
          </div>
          <div class="text-[10.5px] text-dim leading-relaxed">{{ t("drivers.safetyNote") }}</div>
          <div class="divider my-3"></div>
          <div class="text-[10.5px] text-dim leading-relaxed">{{ t("drivers.adminNote") }}</div>
        </div>

        <div class="panel p-5">
          <div class="flex items-center justify-between">
            <div class="text-[11.5px]">{{ t("drivers.catalog") }}</div>
            <ExternalLink :size="13" class="text-dim" />
          </div>
          <div class="text-[10.5px] text-dim leading-relaxed mt-1">Microsoft Update Catalog · Windows Update</div>
        </div>
      </div>
    </div>

    <Modal :open="detailOpen && !!resolution" :title="resolution?.deviceName ?? ''" width="760px" @close="closeDetail">
      <template v-if="resolution">
        <div class="grid grid-cols-3 gap-3 text-[10.5px] mb-4">
          <div class="card-flat px-3 py-2">
            <div class="text-dim">{{ t("drivers.currentVersion") }}</div>
            <div class="mono mt-0.5 truncate">{{ resolution.installedVersion ?? "—" }}</div>
          </div>
          <div class="card-flat px-3 py-2">
            <div class="text-dim">{{ t("drivers.currentInf") }}</div>
            <div class="mono mt-0.5 truncate">{{ resolution.installedInf ?? "—" }}</div>
          </div>
          <div class="card-flat px-3 py-2">
            <div class="text-dim">{{ t("drivers.signature") }}</div>
            <div class="mt-0.5 truncate">{{ firmwareSignature.label }}</div>
          </div>
        </div>

        <div class="flex flex-wrap gap-1 mb-4">
          <span v-for="id in resolution.hardwareIds" :key="id" class="chip !h-5 mono !text-[9px]">{{ id }}</span>
        </div>

        <div class="section-title mb-2">{{ t("drivers.resolutionTitle") }}</div>
        <div v-if="!resolution.candidates.length" class="card-flat px-4 py-3 text-[11px] text-dim mb-4">
          <div class="text-text text-[11.5px] mb-1">{{ t("drivers.noCandidates") }}</div>
          {{ t("drivers.noCandidatesBody") }}
        </div>
        <div v-else class="space-y-1.5 mb-4 max-h-[260px] overflow-y-auto scroll-region pr-1">
          <button
            v-for="c in resolution.candidates"
            :key="c.infPath"
            class="w-full text-left card-flat px-3 py-2.5 transition-colors"
            :class="selected?.infPath === c.infPath ? 'border-accent/40 bg-accent/[0.07]' : 'hover:bg-white/[0.04]'"
            @click="selected = c; confirmed = false; installNote = null"
          >
            <div class="flex items-center gap-2">
              <span class="text-[11.5px] truncate flex-1">{{ c.description || c.provider }}</span>
              <span class="chip !h-5 !text-[9px]" :class="signatureInfo(c.signature).chip">{{ signatureInfo(c.signature).label }}</span>
              <span v-if="c.isInstalled" class="chip chip-ok !h-5 !text-[9px]">{{ t("drivers.installed") }}</span>
              <span v-if="c.newerThanInstalled" class="chip chip-info !h-5 !text-[9px]">{{ t("drivers.newer") }}</span>
              <span v-if="c.exactHwidMatch" class="chip !h-5 !text-[9px]">{{ t("drivers.exactMatch") }}</span>
            </div>
            <div class="flex items-center gap-3 mt-1.5 text-[9.5px] text-dim">
              <span class="mono">{{ c.version }}</span>
              <span>{{ c.date ?? "—" }}</span>
              <span class="truncate">{{ c.provider }}</span>
            </div>
            <div class="mono text-[9px] text-dim truncate mt-1" :title="c.infPath">{{ t("drivers.infFile") }}: {{ shortenHomePath(c.infPath) }}</div>
          </button>
        </div>

        <div class="panel p-4 mb-4">
          <div class="flex items-center justify-between gap-2 mb-2">
            <div class="section-title">{{ t("drivers.catalogResults") }}</div>
            <button class="btn btn-ghost !h-7 !text-[10.5px]" :disabled="catalogLoading" @click="searchCatalog">
              <RefreshCw :size="11" :class="catalogLoading ? 'animate-spin' : ''" />
              {{ catalogLoading ? t("drivers.catalogSearching") : t("drivers.catalogSearch") }}
            </button>
          </div>
          <div class="text-[10.5px] text-dim leading-relaxed">{{ t("drivers.catalogHint") }}</div>
          <div v-if="catalogError" class="text-[10.5px] text-bad mt-2">{{ catalogError }}</div>
          <div v-if="catalogUpdates.length" class="mt-2 space-y-1.5 max-h-[210px] overflow-y-auto scroll-region pr-1">
            <div v-for="u in catalogUpdates" :key="u.updateId" class="card-flat px-3 py-2">
              <div class="flex items-center gap-2">
                <span class="text-[11px] truncate flex-1">{{ u.title }}</span>
                <span class="chip !h-5 !text-[9px] shrink-0">{{ u.version }}</span>
                <span class="mono text-[9px] text-dim shrink-0">{{ u.sizeText }}</span>
                <button class="btn btn-primary !h-6 !px-2 !text-[9.5px] shrink-0" :disabled="downloadingId !== null" @click="downloadUpdate(u)">
                  <Download :size="10" />
                  {{ downloadingId === u.updateId ? t("drivers.catalogDownloading") : t("drivers.catalogDownload") }}
                </button>
              </div>
              <div class="text-[9px] text-dim mt-1">{{ u.date }} · {{ u.classification }}</div>
            </div>
          </div>
          <div v-if="downloaded" class="mt-3 rounded-lg border border-ok/25 bg-ok/[0.06] px-3 py-2">
            <div class="flex items-center gap-2">
              <ShieldCheck :size="12" class="text-ok shrink-0" />
              <span class="text-[10.5px]">{{ t("drivers.catalogDownloaded") }}</span>
              <span class="chip chip-ok !h-5 !text-[9px] shrink-0">{{ signatureInfo(downloaded.signature as DriverEntry["signature"]).label }}</span>
              <button class="icon-btn ml-auto shrink-0" :title="t('drivers.openFolder')" @click="openPath(downloaded.dir)">
                <FolderOpen :size="12" />
              </button>
            </div>
            <div class="mono text-[9px] text-dim truncate mt-1" :title="downloaded.infs.join('\n')">{{ downloaded.infs[0] }}</div>
          </div>
        </div>

        <div v-if="resolution.sources.length" class="mb-4">
          <div class="section-title mb-2">{{ t("drivers.sources") }}</div>
          <div class="flex flex-wrap gap-1.5">
            <button
              v-for="s in resolution.sources"
              :key="s.url"
              class="chip !h-6 hover:bg-white/[0.07] transition-colors max-w-full"
              @click="openExternal(s.url)"
            >
              <Link2 :size="10" />
              <span class="truncate max-w-[280px]">{{ s.label }}</span>
              <ArrowUpRight :size="10" />
            </button>
          </div>
        </div>

        <div v-if="firmware" class="panel p-4">
          <div class="flex items-center gap-2 mb-2">
            <div class="section-title">{{ t("drivers.firmware") }}</div>
            <span class="chip !h-5 !text-[9px]">{{ firmware.revision ?? "—" }}</span>
          </div>
          <div class="grid grid-cols-3 gap-3 text-[10.5px]">
            <div>
              <div class="text-dim">{{ t("drivers.revision") }}</div>
              <div class="mono mt-0.5">{{ firmware.revision ?? "—" }}</div>
            </div>
            <div>
              <div class="text-dim">{{ t("drivers.protocol") }}</div>
              <div class="mt-0.5 truncate">{{ firmware.protocolVersion ?? "—" }}</div>
            </div>
            <div>
              <div class="text-dim">{{ t("drivers.knownRevisions") }}</div>
              <div class="mono mt-0.5 truncate">{{ firmware.knownRevisions.join(" · ") || "—" }}</div>
            </div>
          </div>
          <div v-if="!firmware.flashable" class="mt-3 flex items-start gap-2 rounded-lg border border-warn/25 bg-warn/[0.07] px-3 py-2">
            <TriangleAlert :size="13" class="text-warn shrink-0 mt-0.5" />
            <div class="text-[10.5px] leading-relaxed">
              <div class="text-text">{{ t("drivers.noFlash") }}</div>
              <div class="text-dim mt-0.5">{{ t("drivers.noFlashBody") }}</div>
            </div>
          </div>
        </div>

        <label class="flex items-start gap-2.5 mt-4 cursor-pointer select-none group">
          <input v-model="confirmed" type="checkbox" class="peer sr-only" />
          <span
            class="w-[17px] h-[17px] rounded-[5px] border grid place-items-center shrink-0 mt-0.5 transition-colors peer-focus-visible:ring-2 peer-focus-visible:ring-accent/40 peer-focus-visible:ring-offset-2 peer-focus-visible:ring-offset-surface"
            :class="confirmed ? 'border-accent bg-accent/25 text-accent' : 'border-white/20 bg-white/[0.02] text-transparent group-hover:border-white/35'"
          >
            <Check :size="11" />
          </span>
          <span class="text-[11px] leading-snug" :class="confirmed ? 'text-text' : 'text-muted'">
            {{ t("drivers.confirmInstall") }}
            <span class="block text-[10px] text-dim mt-0.5">
              {{ isAdmin ? t("drivers.confirmHint") : t("drivers.confirmHintUac") }}
            </span>
          </span>
        </label>
        <div v-if="installNote" class="text-[10.5px] text-dim mt-2">{{ installNote }}</div>
      </template>

      <template #footer>
        <button class="btn btn-ghost" @click="closeDetail">{{ t("common.close") }}</button>
        <button class="btn btn-primary" :disabled="!installable || !confirmed || installing" @click="install">
          <RefreshCw :size="12" :class="installing ? 'animate-spin' : ''" />
          {{ installing ? t("drivers.installing") : isAdmin ? t("drivers.install") : t("drivers.installElevated") }}
        </button>
      </template>
    </Modal>
  </div>
</template>
