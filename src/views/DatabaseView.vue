<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { Archive, Database, History, Link2, RefreshCw, Search, ShieldCheck, Upload } from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import EmptyState from "../components/EmptyState.vue";
import Segmented from "../components/Segmented.vue";
import { useDevicesStore } from "../stores/devices";
import { useAppStore } from "../stores/app";
import { useI18n } from "../lib/i18n";
import { api, isTauri, openExternal } from "../lib/api";
import { capabilityLabel } from "../lib/capabilities";
import { compatibilitySummary, deviceKindLabel, shortenHomePath } from "../lib/format";
import type { DbHistoryEntry, DbInfo, RegistryEntry } from "../lib/types";

const { t, locale } = useI18n();
const devices = useDevicesStore();
const app = useAppStore();

const entries = ref<RegistryEntry[]>([]);
const dbInfo = ref<DbInfo | null>(null);
const dbHistory = ref<DbHistoryEntry[]>([]);
const loading = ref(false);
const backupBusy = ref(false);
const importBusy = ref(false);
const importInput = ref<HTMLInputElement | null>(null);
const importNote = ref<string | null>(null);
const query = ref("");
const category = ref<"all" | "keyboard" | "mouse" | "headset" | "audio">("all");

const categories = computed(() => [
  { value: "all" as const, label: t("database.all") },
  { value: "keyboard" as const, label: deviceKindLabel("keyboard", locale.value) },
  { value: "mouse" as const, label: deviceKindLabel("mouse", locale.value) },
  { value: "headset" as const, label: deviceKindLabel("headset", locale.value) },
  { value: "audio" as const, label: deviceKindLabel("audio", locale.value) },
]);

const filtered = computed(() => {
  const q = query.value.trim().toLowerCase();
  return entries.value.filter((e) => {
    if (category.value !== "all" && e.category !== category.value) return false;
    if (!q) return true;
    const haystack = [e.vendor, e.model, e.category, e.adapterLabel, ...e.vidPids, ...e.hardwareIds, ...e.connections]
      .join(" ")
      .toLowerCase();
    return haystack.includes(q);
  });
});

const compatOf = (entry: RegistryEntry) => compatibilitySummary(entry.capabilities);
const isConnected = (entry: RegistryEntry) =>
  devices.devices.some((d) => d.adapterId === entry.adapterId && d.online);

function formatBytes(bytes: number) {
  if (!bytes) return "—";
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

function verificationLabel(status: string) {
  const key = `status.${status}`;
  const label = t(key);
  return label === key ? status : label;
}

function verificationClass(status: string) {
  if (status === "verified" || status === "supported") return "chip-ok";
  if (status === "experimental") return "chip-info";
  if (status === "unsupported") return "chip-bad";
  return "chip";
}

function actionLabel(action: string) {
  const key = `database.action.${action}`;
  const label = t(key);
  return label === key ? action : label;
}

async function load() {
  loading.value = true;
  try {
    entries.value = await api.registryList();
  } catch (e) {
    app.toast("error", t("toast.error"), String(e));
  }
  try {
    const [info, history] = await Promise.all([api.dbInfo(), api.dbHistory(8)]);
    dbInfo.value = info;
    dbHistory.value = history;
  } catch (e) {
    app.toast("error", t("toast.error"), String(e));
  } finally {
    loading.value = false;
  }
}

async function backup() {
  backupBusy.value = true;
  try {
    const path = await api.dbBackup();
    app.toast("success", t("database.backupDone"), shortenHomePath(path));
    dbHistory.value = await api.dbHistory(8);
    dbInfo.value = await api.dbInfo();
  } catch (e) {
    app.toast("error", t("database.backupFail"), String(e));
  } finally {
    backupBusy.value = false;
  }
}

async function openSource(url: string) {
  try {
    await openExternal(url);
  } catch (e) {
    app.toast("error", t("toast.error"), String(e));
  }
}

function pickImport() {
  importInput.value?.click();
}

async function onImportFile(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (!file) return;
  importBusy.value = true;
  try {
    const summary = await api.registryImport(await file.text());
    importNote.value = `${summary.imported} / ${summary.bindings} · ${file.name}`;
    app.toast("success", t("database.importDone"), `${summary.imported} · ${summary.bindings}`);
    await load();
    await devices.refresh(true);
  } catch (e) {
    app.toast("error", t("database.importFail"), String(e));
  } finally {
    importBusy.value = false;
  }
}

onMounted(load);
</script>

<template>
  <div>
    <PageHeader :title="t('database.title')" :subtitle="t('database.subtitle')" :overline="t('nav.database')">
      <template #actions>
        <span class="chip mono">{{ t("database.schema") }} v{{ dbInfo?.schemaVersion ?? "—" }}</span>
        <span class="chip chip-ok">{{ t("database.offline") }}</span>
      </template>
    </PageHeader>

    <div class="grid grid-cols-12 gap-4">
      <div class="col-span-8 space-y-3">
        <div class="flex items-center gap-2">
          <label class="relative flex-1">
            <Search :size="14" class="absolute left-3 top-1/2 -translate-y-1/2 text-dim" />
            <input v-model="query" class="input !pl-9" :placeholder="t('database.search')" />
          </label>
          <Segmented v-model="category" :options="categories" />
        </div>

        <div class="text-[10.5px] text-dim px-1">
          {{ t("database.records") }}: {{ filtered.length }} / {{ entries.length }}
        </div>

        <EmptyState v-if="!filtered.length" :icon="Database" :title="t('database.empty')" :body="t('database.emptyBody')" />

        <div v-for="entry in filtered" :key="entry.id" class="panel p-5">
          <div class="flex items-start justify-between gap-4">
            <div class="min-w-0">
              <div class="text-[16px] font-semibold tracking-tight truncate" :title="entry.model">{{ entry.model }}</div>
              <div class="text-[11px] text-dim mt-0.5">{{ entry.vendor }} · {{ deviceKindLabel(entry.category, locale) }}</div>
            </div>
            <div class="flex items-center gap-1.5 shrink-0">
              <span v-if="isConnected(entry)" class="chip chip-ok">
                <span class="status-dot bg-ok"></span>
                {{ t("database.connected") }}
              </span>
              <span v-else class="chip">{{ t("database.notConnected") }}</span>
              <span class="chip" :class="verificationClass(entry.verification)">{{ verificationLabel(entry.verification) }}</span>
            </div>
          </div>

          <div class="flex items-center gap-1.5 mt-3 flex-wrap">
            <span v-for="id in entry.vidPids" :key="id" class="chip mono !text-[10px]">{{ id }}</span>
            <span v-for="c in entry.connections" :key="c" class="chip !text-[10px]">{{ c }}</span>
          </div>
          <div class="flex items-center gap-1.5 mt-1.5 flex-wrap">
            <span class="text-[10px] text-dim">{{ t("database.hardwareIds") }}:</span>
            <span v-for="id in entry.hardwareIds" :key="id" class="text-[10px] text-dim mono">{{ id }}</span>
            <span v-if="!entry.hardwareIds.length" class="text-[10px] text-dim">—</span>
            <span class="text-[10px] text-dim ml-2">{{ t("database.firmware") }}:</span>
            <span class="text-[10px] text-dim mono">{{ entry.firmware.length ? entry.firmware.join(", ") : t("database.notSet") }}</span>
          </div>

          <div class="text-[11.5px] text-muted leading-relaxed mt-3">
            {{ locale === "ru" ? entry.protocolRu : entry.protocolEn }}
          </div>

          <div class="divider my-3.5"></div>

          <div class="flex items-center justify-between text-[10.5px] mb-1.5">
            <span class="text-dim">{{ t("database.compat") }}</span>
            <span class="mono text-muted">{{ compatOf(entry).confirmed }} / {{ compatOf(entry).total }}</span>
          </div>
          <div class="flex gap-1 h-[4px] rounded-full overflow-hidden bg-white/5">
            <span
              v-for="(count, status) in compatOf(entry).counts"
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

          <div class="flex flex-wrap gap-1 mt-3">
            <span
              v-for="cap in entry.capabilities"
              :key="cap.id"
              class="chip !text-[10px] !h-6"
              :class="{
                'chip-ok': cap.status === 'working',
                'chip-warn': cap.status === 'partial',
                'chip-bad': cap.status === 'unsupported',
                'chip-info': cap.status === 'research',
              }"
              :title="locale === 'ru' ? cap.noteRu : cap.noteEn"
            >
              {{ capabilityLabel(cap.id, locale) }}
            </span>
          </div>

          <div class="flex items-center justify-between gap-3 mt-3.5">
            <div class="flex items-center gap-1.5 flex-wrap">
              <button
                v-for="s in entry.sources"
                :key="s.url"
                class="text-[10.5px] text-dim hover:text-muted transition-colors inline-flex items-center gap-1"
                :title="s.url"
                @click="openSource(s.url)"
              >
                <Link2 :size="10" />
                {{ s.label }}
              </button>
            </div>
            <span class="text-[10px] text-dim mono shrink-0">{{ entry.adapterLabel }}</span>
          </div>
        </div>
      </div>

      <div class="col-span-4 space-y-4">
        <div class="panel p-5 relative overflow-hidden">
          <div class="absolute inset-0 pointer-events-none opacity-70" style="background: radial-gradient(280px 120px at 90% -10%, rgba(255,79,163,.14), transparent 70%)"></div>
          <div class="flex items-center justify-between relative">
            <div class="section-title">{{ t("database.sqlite") }}</div>
            <Database :size="15" class="text-dim" />
          </div>
          <div class="text-[11.5px] text-dim mt-2 leading-relaxed">{{ t("database.sqliteBody") }}</div>
          <div class="divider my-3.5"></div>
          <div class="flex items-center justify-between text-[11.5px] py-1">
            <span class="text-dim">{{ t("database.schema") }}</span>
            <span class="mono text-muted">v{{ dbInfo?.schemaVersion ?? "—" }} · {{ dbInfo?.migrations.length ?? 0 }}</span>
          </div>
          <div class="flex items-center justify-between text-[11.5px] py-1">
            <span class="text-dim">{{ t("database.models") }}</span>
            <span class="mono text-muted">{{ dbInfo?.deviceCount ?? entries.length }}</span>
          </div>
          <div class="flex items-center justify-between text-[11.5px] py-1">
            <span class="text-dim">{{ t("database.features") }}</span>
            <span class="mono text-muted">{{ dbInfo?.capabilityCount ?? "—" }}</span>
          </div>
          <div class="flex items-center justify-between text-[11.5px] py-1">
            <span class="text-dim">{{ t("database.size") }}</span>
            <span class="mono text-muted">{{ formatBytes(dbInfo?.sizeBytes ?? 0) }}</span>
          </div>
          <div class="flex items-center justify-between text-[11.5px] py-1">
            <span class="text-dim">{{ t("database.lastUpdated") }}</span>
            <span class="mono text-muted">{{ dbInfo?.lastUpdated ?? "—" }}</span>
          </div>
          <div class="text-[10px] text-dim mono mt-2 truncate" :title="dbInfo?.path">{{ shortenHomePath(dbInfo?.path ?? "") }}</div>
          <button class="btn w-full mt-3 !h-8 !text-[11.5px]" @click="backup" :disabled="backupBusy || !dbInfo">
            <Archive :size="12" :class="backupBusy ? 'animate-pulse' : ''" />
            {{ t("database.backup") }}
          </button>
          <button class="btn w-full mt-2 !h-8 !text-[11.5px]" @click="pickImport" :disabled="importBusy || !isTauri()">
            <Upload :size="12" :class="importBusy ? 'animate-pulse' : ''" />
            {{ t("database.import") }}
          </button>
          <input ref="importInput" type="file" accept="application/json,.json" class="hidden" @change="onImportFile" />
          <div class="text-[10px] text-dim leading-relaxed mt-2">{{ t("database.importNote") }}</div>
          <div v-if="importNote" class="text-[10px] text-ok mono mt-1 break-all">{{ importNote }}</div>
        </div>

        <div class="panel p-5">
          <div class="flex items-center gap-2 mb-3">
            <History :size="15" class="text-dim" />
            <div class="section-title">{{ t("database.history") }}</div>
          </div>
          <div v-if="dbHistory.length" class="space-y-2">
            <div v-for="item in dbHistory" :key="item.id" class="text-[10.5px] leading-snug">
              <div class="flex items-center justify-between gap-2">
                <span class="text-muted">{{ actionLabel(item.action) }}</span>
                <span class="text-dim mono shrink-0">{{ item.ts.slice(5, 16) }}</span>
              </div>
              <div v-if="item.details" class="text-dim truncate" :title="item.details">{{ shortenHomePath(item.details) }}</div>
            </div>
          </div>
          <div v-else class="text-[11px] text-dim">{{ t("database.historyEmpty") }}</div>
        </div>

        <div class="panel p-5">
          <div class="flex items-center gap-2 mb-2">
            <ShieldCheck :size="15" class="text-ok" />
            <div class="section-title">{{ t("database.verification") }}</div>
          </div>
          <div class="text-[11.5px] text-dim leading-relaxed">{{ t("database.verificationBody") }}</div>
          <div class="divider my-3"></div>
          <div class="space-y-1.5 text-[11px]">
            <div class="flex items-center gap-2"><span class="status-dot bg-ok"></span>{{ t("status.working") }}</div>
            <div class="flex items-center gap-2"><span class="status-dot bg-warn"></span>{{ t("status.partial") }}</div>
            <div class="flex items-center gap-2"><span class="status-dot bg-info"></span>{{ t("status.research") }}</div>
            <div class="flex items-center gap-2"><span class="status-dot bg-bad"></span>{{ t("status.unsupported") }}</div>
          </div>
          <div class="divider my-3"></div>
          <div class="text-[10.5px] text-dim leading-relaxed">{{ t("database.localNote") }}</div>
          <button class="btn w-full mt-3 !h-8 !text-[11.5px]" @click="load" :disabled="loading">
            <RefreshCw :size="12" :class="loading ? 'animate-spin' : ''" />
            {{ t("common.refreshList") }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
