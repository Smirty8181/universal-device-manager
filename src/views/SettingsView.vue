<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
import {
  Database, Download, ExternalLink, FileText, FolderOpen, Info, Moon, RotateCcw, ShieldCheck, Upload,
} from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import SettingRow from "../components/SettingRow.vue";
import InfoRow from "../components/InfoRow.vue";
import Segmented from "../components/Segmented.vue";
import Slider from "../components/Slider.vue";
import Toggle from "../components/Toggle.vue";
import Modal from "../components/Modal.vue";
import { useAppStore } from "../stores/app";
import { useProfilesStore } from "../stores/profiles";
import { useMacrosStore } from "../stores/macros";
import { api, copyText, isTauri, openPath } from "../lib/api";
import { useI18n } from "../lib/i18n";
import type { AppSettings, Macro, Profile } from "../lib/types";

const { t } = useI18n();
const router = useRouter();
const app = useAppStore();
const profiles = useProfilesStore();
const macros = useMacrosStore();

const accents: { value: AppSettings["accent"]; label: string; colors: [string, string] }[] = [
  { value: "pink", label: "Magenta", colors: ["#ff4fa3", "#a78bfa"] },
  { value: "violet", label: "Violet", colors: ["#a78bfa", "#6ee7f9"] },
  { value: "cyan", label: "Cyan", colors: ["#6ee7f9", "#a78bfa"] },
];

const languages: { value: "ru" | "en"; label: string }[] = [
  { value: "ru", label: "Русский" },
  { value: "en", label: "English" },
];

const intervalOptions = [2000, 3000, 5000, 10000];
const intervalLabel = (ms: number) => t(`settings.interval${ms / 1000}`);

const lowBattery = ref(app.settings.lowBatteryThreshold);
let batteryTimer: number | undefined;
watch(
  () => app.settings.lowBatteryThreshold,
  (v) => (lowBattery.value = v)
);
function commitLowBattery(value: number) {
  lowBattery.value = value;
  window.clearTimeout(batteryTimer);
  batteryTimer = window.setTimeout(() => void app.patchSettings({ lowBatteryThreshold: value }), 250);
}

const resetOpen = ref(false);
const backupOpen = ref(false);
const backupMode = ref<"export" | "import">("export");
const backupText = ref("");

onMounted(() => {
  if (!app.systemInfo) void app.loadSystemInfo();
});

async function setAutostart(value: boolean) {
  const previous = app.settings.autostart;
  await app.patchSettings({ autostart: value });
  try {
    const ok = await api.setAutostart(value);
    if (!ok) throw new Error(t("settings.autostartError"));
  } catch (e) {
    await app.patchSettings({ autostart: previous });
    app.toast("error", t("settings.autostartError"), String(e));
  }
}

function openData(path?: string | null) {
  if (!path) {
    app.toast("info", t("common.notAvailable"));
    return;
  }
  void openPath(path);
}

async function openExport() {
  backupText.value = JSON.stringify(
    {
      app: "Universal Device Manager",
      format: "udm-backup",
      version: 1,
      exportedAt: new Date().toISOString(),
      settings: app.settings,
      profiles: profiles.profiles,
      macros: macros.macros,
    },
    null,
    2
  );
  backupMode.value = "export";
  backupOpen.value = true;
}

function openImport() {
  backupText.value = "";
  backupMode.value = "import";
  backupOpen.value = true;
}

async function copyBackup() {
  try {
    await copyText(backupText.value);
    app.toast("success", t("common.copied"));
  } catch (e) {
    app.toast("error", t("toast.error"), String(e));
  }
}

async function applyImport() {
  try {
    const data = JSON.parse(backupText.value) as {
      settings?: Partial<AppSettings>;
      profiles?: Profile[];
      macros?: Macro[];
    };
    if (!data || typeof data !== "object") throw new Error("shape");
    if (data.settings) await app.patchSettings(data.settings);
    let profileCount = 0;
    let macroCount = 0;
    if (Array.isArray(data.profiles)) {
      for (const p of data.profiles) {
        await api.profileSave(p);
        profileCount++;
      }
      await profiles.load();
    }
    if (Array.isArray(data.macros)) {
      for (const m of data.macros) {
        await api.macroSave(m);
        macroCount++;
      }
      await macros.load();
    }
    backupOpen.value = false;
    app.toast("success", t("toast.imported"), t("settings.importDone", { profiles: profileCount, macros: macroCount }));
  } catch {
    app.toast("error", t("toast.error"), t("settings.importInvalid"));
  }
}

async function resetAll() {
  await app.resetSettings();
  resetOpen.value = false;
}

const version = computed(() => app.systemInfo?.appVersion ?? (isTauri() ? "—" : "dev"));
const admin = computed(() => (app.systemInfo ? (app.systemInfo.admin ? t("diagnostics.adminYes") : t("diagnostics.adminNo")) : "—"));
</script>

<template>
  <div>
    <PageHeader :title="t('settings.title')" :subtitle="t('settings.subtitle')" :overline="t('nav.settings')">
      <template #actions>
        <button class="btn" @click="router.push('/diagnostics')">
          <ExternalLink :size="13" />
          {{ t("settings.diagnosticsLink") }}
        </button>
      </template>
    </PageHeader>

    <div class="grid grid-cols-12 gap-4">
      <div class="col-span-7 space-y-4">
        <div class="panel p-5">
          <div class="section-title mb-1">{{ t("settings.appearance") }}</div>

          <SettingRow :label="t('settings.accent')">
            <div class="flex items-center gap-2">
              <button
                v-for="a in accents"
                :key="a.value"
                class="w-8 h-8 rounded-full border-2 transition-transform hover:scale-105"
                :class="app.settings.accent === a.value ? 'border-white/70' : 'border-transparent'"
                :style="{ background: `linear-gradient(135deg, ${a.colors[0]}, ${a.colors[1]})` }"
                :title="a.label"
                @click="app.patchSettings({ accent: a.value })"
              ></button>
            </div>
          </SettingRow>

          <SettingRow :label="t('settings.language')">
            <Segmented :model-value="app.settings.language" :options="languages" @update:model-value="(v) => app.patchSettings({ language: v })" />
          </SettingRow>

          <SettingRow :label="t('settings.theme')" :description="t('settings.themeNote')">
            <span class="chip chip-ok !h-7"><Moon :size="11" /> {{ t("settings.themeDark") }}</span>
          </SettingRow>
        </div>

        <div class="panel p-5">
          <div class="section-title mb-1">{{ t("settings.behavior") }}</div>

          <SettingRow :label="t('settings.autostart')" :description="t('drivers.adminNote')">
            <Toggle :model-value="app.settings.autostart" @update:model-value="setAutostart" />
          </SettingRow>
          <SettingRow :label="t('settings.startMinimized')">
            <Toggle :model-value="app.settings.startMinimized" @update:model-value="(v) => app.patchSettings({ startMinimized: v })" />
          </SettingRow>
          <SettingRow :label="t('settings.minimizeToTray')">
            <Toggle :model-value="app.settings.minimizeToTray" @update:model-value="(v) => app.patchSettings({ minimizeToTray: v })" />
          </SettingRow>
          <SettingRow :label="t('settings.autoScan')">
            <Toggle :model-value="app.settings.autoScan" @update:model-value="(v) => app.patchSettings({ autoScan: v })" />
          </SettingRow>
          <SettingRow :label="t('settings.scanInterval')">
            <select
              class="input !h-8 !w-[132px] !text-[11.5px]"
              :value="app.settings.scanIntervalMs"
              @change="app.patchSettings({ scanIntervalMs: Number(($event.target as HTMLSelectElement).value) })"
            >
              <option v-for="ms in intervalOptions" :key="ms" :value="ms">{{ intervalLabel(ms) }}</option>
            </select>
          </SettingRow>
          <SettingRow :label="t('settings.autoSwitch')" :description="t('settings.autoSwitchNote')">
            <Toggle :model-value="app.settings.autoSwitchProfiles" @update:model-value="(v) => app.patchSettings({ autoSwitchProfiles: v })" />
          </SettingRow>
          <SettingRow :label="t('settings.notifications')">
            <Toggle :model-value="app.settings.notificationsEnabled" @update:model-value="(v) => app.patchSettings({ notificationsEnabled: v })" />
          </SettingRow>
          <SettingRow :label="t('settings.trayBattery')" :description="t('settings.trayBatteryNote')">
            <Toggle :model-value="app.settings.trayBattery" @update:model-value="(v) => app.patchSettings({ trayBattery: v })" />
          </SettingRow>
          <SettingRow :label="t('settings.lowBattery')">
            <div class="w-[190px]">
              <Slider :model-value="lowBattery" :min="5" :max="50" :step="5" unit="%" @update:model-value="commitLowBattery" />
            </div>
          </SettingRow>
          <SettingRow :label="t('settings.storeHistory')" :description="t('settings.batteryHistoryNote')">
            <Toggle :model-value="app.settings.storeHistory" @update:model-value="(v) => app.patchSettings({ storeHistory: v })" />
          </SettingRow>
          <SettingRow :label="t('settings.autoDownloadDrivers')" :description="t('settings.autoDownloadDriversNote')">
            <Toggle :model-value="app.settings.autoDownloadDrivers" @update:model-value="(v) => app.patchSettings({ autoDownloadDrivers: v })" />
          </SettingRow>
        </div>
      </div>

      <div class="col-span-5 space-y-4">
        <div class="panel p-5">
          <div class="section-title mb-2">{{ t("settings.privacy") }}</div>
          <InfoRow :label="t('settings.dataFolder')" :value="app.systemInfo?.dataDir ?? '—'" mono wrap />
          <InfoRow :label="t('settings.logs')" :value="app.systemInfo?.logFile ?? t('common.notAvailable')" mono wrap />
          <div class="flex gap-2 mt-3">
            <button class="btn flex-1 !text-[11px]" @click="openData(app.systemInfo?.dataDir)">
              <FolderOpen :size="12" />
              {{ t("settings.openData") }}
            </button>
            <button class="btn flex-1 !text-[11px]" @click="openData(app.systemInfo?.logFile)">
              <FileText :size="12" />
              {{ t("settings.openLogs") }}
            </button>
          </div>
        </div>

        <div class="panel p-5">
          <div class="section-title mb-2">{{ t("settings.fullBackup") }}</div>
          <div class="text-[11px] text-dim leading-relaxed">{{ t("settings.fullBackupNote") }}</div>
          <div class="flex gap-2 mt-3">
            <button class="btn flex-1" @click="openExport">
              <Download :size="12" />
              {{ t("settings.exportData") }}
            </button>
            <button class="btn flex-1" @click="openImport">
              <Upload :size="12" />
              {{ t("settings.importData") }}
            </button>
          </div>
        </div>

        <div class="panel p-5">
          <div class="section-title mb-2">{{ t("settings.about") }}</div>
          <InfoRow :label="t('settings.version')" :value="version" mono />
          <InfoRow :label="t('settings.platform')" value="Windows 10 / 11 · WebView2 + Rust" />
          <InfoRow :label="t('diagnostics.admin')" :value="admin" />
          <InfoRow :label="t('diagnostics.deviceCount')" :value="app.systemInfo?.deviceCount ?? '—'" mono />
          <InfoRow :label="t('diagnostics.adapters')" :value="app.systemInfo?.adapterCount ?? '—'" mono />
          <div class="flex items-center gap-2 mt-3">
            <span class="chip !h-6" :class="isTauri() ? 'chip-ok' : 'chip-warn'">
              <ShieldCheck :size="10" />
              {{ isTauri() ? t("diagnostics.tauriOk") : t("diagnostics.tauriNo") }}
            </span>
          </div>
          <div class="text-[10.5px] text-dim leading-relaxed mt-3">{{ t("settings.aboutNote") }}</div>
        </div>

        <div class="panel p-5">
          <div class="flex items-start justify-between gap-3">
            <div>
              <div class="flex items-center gap-2">
                <Info :size="13" class="text-dim" />
                <span class="text-[11.5px]">{{ t("settings.resetSettings") }}</span>
              </div>
              <div class="text-[10.5px] text-dim mt-1 leading-relaxed">{{ t("settings.resetConfirm") }}</div>
            </div>
            <button class="btn btn-danger !h-8 !px-2.5 !text-[11px] shrink-0" @click="resetOpen = true">
              <RotateCcw :size="12" />
              {{ t("common.reset") }}
            </button>
          </div>
        </div>

        <div class="panel p-5 flex items-center gap-3">
          <Database :size="16" class="text-dim" />
          <div class="text-[10.5px] text-dim leading-relaxed">{{ t("settings.savedToast") }} · JSON · {{ t("diagnostics.dataDir") }}</div>
        </div>
      </div>
    </div>

    <Modal :open="backupOpen" :title="backupMode === 'export' ? t('settings.exportData') : t('settings.importData')" width="560px" @close="backupOpen = false">
      <div class="text-[11px] text-dim mb-2">{{ t("settings.fullBackupNote") }}</div>
      <textarea
        v-model="backupText"
        class="input !h-[320px] font-mono !text-[10.5px]"
        spellcheck="false"
        :readonly="backupMode === 'export'"
        :placeholder="t('settings.fullBackupNote')"
      ></textarea>
      <template #footer>
        <button class="btn" @click="backupOpen = false">{{ t("common.close") }}</button>
        <button v-if="backupMode === 'export'" class="btn btn-primary" @click="copyBackup">{{ t("settings.copyBackup") }}</button>
        <button v-else class="btn btn-primary" :disabled="!backupText.trim()" @click="applyImport">{{ t("common.import") }}</button>
      </template>
    </Modal>

    <Modal :open="resetOpen" :title="t('settings.resetConfirm')" width="380px" @close="resetOpen = false">
      <div class="text-[11.5px] text-dim">{{ t("settings.aboutNote") }}</div>
      <template #footer>
        <button class="btn" @click="resetOpen = false">{{ t("common.cancel") }}</button>
        <button class="btn btn-danger" @click="resetAll">{{ t("common.reset") }}</button>
      </template>
    </Modal>
  </div>
</template>
