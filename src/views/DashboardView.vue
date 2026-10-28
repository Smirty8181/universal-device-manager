<script setup lang="ts">
import { computed } from "vue";
import { useRouter } from "vue-router";
import {
  Activity, Battery, BatteryMedium, ChevronRight, CircleDashed, Command, Headphones, Keyboard, Layers, Mouse,
  Palette, Power, RefreshCw, ScanLine, Settings2, Sparkles, Usb, Wifi, ArrowRight,
  type LucideIcon,
} from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import DeviceCard from "../components/DeviceCard.vue";
import BatteryRing from "../components/BatteryRing.vue";
import AnimatedNumber from "../components/AnimatedNumber.vue";
import Sparkline from "../components/Sparkline.vue";
import EmptyState from "../components/EmptyState.vue";
import { useAppStore } from "../stores/app";
import { useDevicesStore } from "../stores/devices";
import { useProfilesStore } from "../stores/profiles";
import { useI18n } from "../lib/i18n";
import { timeAgo, deviceKindLabel } from "../lib/format";

const router = useRouter();
const app = useAppStore();
const devices = useDevicesStore();
const profiles = useProfilesStore();
const { t, locale } = useI18n();

const online = computed(() => devices.devices.filter((d) => d.online).length);
const total = computed(() => devices.devices.length);
const wireless = computed(() => devices.devices.filter((d) => d.connection !== "usb").length);
const withBattery = computed(() =>
  devices.devices.filter((d) => d.battery?.percent !== null && d.battery?.percent !== undefined)
);
const adapterCount = computed(() => new Set(devices.devices.map((d) => d.adapterId)).size);
const profileEffectLabel = (effect: string) => t("rgb.effects." + effect);
const allGood = computed(() => devices.devices.length > 0 && online.value === total.value);
const lowest = computed(() =>
  [...withBattery.value].sort((a, b) => (a.battery!.percent ?? 100) - (b.battery!.percent ?? 100))[0]
);

const stats = computed<{
  key: string;
  icon: LucideIcon;
  tint: string;
  label: string;
  value?: number;
  suffix?: string;
  valueText?: string;
  detail: string;
}[]>(() => [
  {
    key: "online",
    icon: Usb,
    tint: "#45d483",
    label: t("dashboard.connectedDevices"),
    value: online.value,
    suffix: `/ ${total.value}`,
    detail: `${adapterCount.value} ${t("diagnostics.adapters").toLowerCase()} · ${total.value ? Math.round((online.value / total.value) * 100) : 0}%`,
  },
  {
    key: "wireless",
    icon: Wifi,
    tint: "#a78bfa",
    label: t("dashboard.wirelessConnected"),
    value: wireless.value,
    detail: devices.devices
      .filter((d) => d.connection !== "usb")
      .map((d) => d.name)
      .slice(0, 2)
      .join(", ") || "—",
  },
  {
    key: "battery",
    icon: BatteryMedium,
    tint: "#6ee7f9",
    label: t("dashboard.withBatteries"),
    value: withBattery.value.length,
    detail: lowest.value
      ? `${lowest.value.name}: ${lowest.value.battery!.percent}%`
      : t("dashboard.unknownBatteries"),
  },
  {
    key: "profile",
    icon: Layers,
    tint: "#ff4fa3",
    label: t("dashboard.activeProfile"),
    valueText: profiles.active?.name ?? t("dashboard.noProfile"),
    detail: profiles.active
      ? `${profiles.active.apps.length} ${t("profiles.apps").toLowerCase()} · ${profileEffectLabel(profiles.active.rgb.effect)}`
      : t("profiles.create"),
  },
]);

const eventIcons: Record<string, typeof Activity> = {
  device: Usb, scan: ScanLine, profile: Layers, settings: Settings2, macro: Command, battery: Battery, app: Power,
};

const severityColor: Record<string, string> = {
  info: "text-info", success: "text-ok", warn: "text-warn", error: "text-bad",
};

async function rescan() {
  await devices.refresh();
  app.toast("success", t("toast.scanDone"), `${devices.devices.length} ${t("diagnostics.deviceCount").toLowerCase()}`);
}

async function probe(deviceId: string) {
  const battery = await devices.probeBattery(deviceId);
  if (!battery || battery.percent === null) {
    app.toast("info", t("lab.batteryNoAnswer"), t("device.batteryUnknownBody"));
  }
}

const sparkColor = (kind: string) => (kind === "keyboard" ? "#ff4fa3" : kind === "mouse" ? "#a78bfa" : "#6ee7f9");
</script>

<template>
  <div>
    <PageHeader :title="t('dashboard.title')" :subtitle="t('dashboard.subtitle')" :overline="t('nav.dashboard')">
      <template #actions>
        <button class="btn" :disabled="devices.loading" @click="rescan">
          <RefreshCw :size="14" :class="devices.loading ? 'animate-spin' : ''" />
          {{ t("dashboard.rescan") }}
        </button>
        <button class="btn btn-primary" @click="router.push('/devices')">
          {{ t("dashboard.viewAll") }}
          <ArrowRight :size="14" />
        </button>
      </template>
    </PageHeader>

    <div class="grid grid-cols-4 gap-3 mb-6">
      <button
        v-for="(s, i) in stats"
        :key="s.key"
        class="stat-widget text-left reveal"
        :style="{ '--tint': s.tint, '--i': i }"
        @click="s.key === 'profile' ? router.push('/profiles') : router.push('/devices')"
      >
        <div class="flex items-start justify-between">
          <div class="stat-orb"><component :is="s.icon" :size="16" /></div>
          <Sparkles v-if="s.key === 'profile' && profiles.active" :size="13" class="text-dim/60" />
          <span v-else-if="s.key === 'online'" class="flex items-center gap-1 text-[10px]" :class="allGood ? 'text-ok' : 'text-warn'">
            <span class="status-dot" :class="allGood ? 'bg-ok' : 'bg-warn'"></span>
            {{ allGood ? t("dashboard.allGood") : t("dashboard.attention") }}
          </span>
        </div>
        <div class="mt-3 flex items-baseline gap-1.5 min-w-0">
          <span v-if="s.valueText" class="font-display text-[17px] font-semibold tracking-tight truncate">{{ s.valueText }}</span>
          <template v-else>
            <span class="font-display text-[26px] leading-none font-semibold tracking-tight">
              <AnimatedNumber :value="s.value ?? 0" />
            </span>
            <span class="text-[11px] text-dim truncate">{{ s.suffix }}</span>
          </template>
        </div>
        <div class="text-[10.5px] text-dim truncate mt-1">{{ s.label }}</div>
        <div class="stat-detail">
          <div>
            <div class="pt-2 mt-1 border-t border-white/[0.05] text-[10px] text-muted leading-relaxed truncate">{{ s.detail }}</div>
          </div>
        </div>
        <span
          v-if="s.key === 'profile' && profiles.active"
          class="profile-strip"
          aria-hidden="true"
          :style="{ backgroundImage: `linear-gradient(90deg, ${profiles.active.rgb.color}, ${profiles.active.rgb.secondary}, ${profiles.active.rgb.color})` }"
        />
      </button>
    </div>

    <div class="flex items-center justify-between mb-3">
      <div class="section-title flex items-center gap-2">
        {{ t("dashboard.devicesTitle") }}
        <span class="chip !h-[19px] !text-[9.5px] mono">{{ devices.devices.length }}</span>
      </div>
      <div class="flex items-center gap-2 text-[11px] text-dim">
        <span class="status-dot" :class="allGood ? 'bg-ok' : 'bg-warn'"></span>
        {{ allGood ? t("dashboard.allGood") : t("dashboard.attention") }}
      </div>
    </div>

    <div class="grid grid-cols-3 gap-4 mb-6">
      <DeviceCard v-for="(d, i) in devices.devices" :key="d.id" :device="d" class="reveal" :style="{ '--i': i + 4 }" />
      <div v-if="!devices.devices.length" class="col-span-3 panel p-2">
        <EmptyState :icon="CircleDashed" :title="t('device.notFound')" :body="t('device.notFoundBody')" />
      </div>
    </div>

    <div class="grid grid-cols-12 gap-4">
      <div class="col-span-8 space-y-4">
        <div class="panel p-5 reveal" :style="{ '--i': 8 }">
          <div class="flex items-center justify-between mb-1">
            <div class="section-title flex items-center gap-2">
              {{ t("dashboard.batteries") }}
              <span class="chip !h-[19px] !text-[9.5px] mono">{{ withBattery.length }}</span>
            </div>
            <span class="text-[10.5px] text-dim">{{ t("settings.storeHistory") }}</span>
          </div>
          <div
            v-for="d in devices.devices"
            :key="d.id"
            class="flex items-center gap-4 py-3 border-b border-white/[0.045] last:border-0 last:pb-0"
          >
            <BatteryRing :battery="d.battery" :size="46" :stroke="4.5" :show-label="false" />
            <div class="w-[140px] shrink-0 min-w-0">
              <div class="text-[12.5px] font-medium truncate" :title="d.name">{{ d.name }}</div>
              <div class="text-[10.5px] text-dim truncate">{{ deviceKindLabel(d.kind, locale) }}</div>
            </div>
            <div class="flex-1 min-w-0">
              <Sparkline :data="devices.historyFor(d.id).map((h) => h.percent)" :color="sparkColor(d.kind)" :height="28" />
            </div>
            <div class="w-[150px] shrink-0 text-right">
              <div v-if="d.battery?.percent !== null && d.battery?.percent !== undefined" class="text-[12px] tabular-nums font-semibold">
                {{ d.battery.percent }}%
                <span v-if="d.battery.charging" class="text-warn text-[10px] ml-1">{{ t("battery.chargerConnected") }}</span>
              </div>
              <div v-else class="text-[10.5px] text-dim truncate" :title="d.battery?.noteRu">{{ t("battery.unknown") }}</div>
              <div class="text-[9.5px] text-dim mt-0.5">
                {{ d.battery ? t("battery.source." + d.battery.source) : "—" }}
              </div>
            </div>
            <button class="icon-btn !w-7 !h-7 shrink-0" :title="t('diagnostics.probeBattery')" @click="probe(d.id)">
              <RefreshCw :size="12" />
            </button>
          </div>
          <div class="text-[10px] text-dim mt-3 flex items-center gap-1.5">
            <CircleDashed :size="11" />
            {{ t("dashboard.unknownBatteries") }}
          </div>
        </div>

        <div class="panel p-5 reveal" :style="{ '--i': 9 }">
          <div class="flex items-center justify-between mb-2">
            <div class="section-title">{{ t("dashboard.recentActivity") }}</div>
            <span class="text-[10.5px] text-dim">{{ app.activity.length }}</span>
          </div>
          <div v-if="!app.activity.length" class="text-[12px] text-dim py-4">{{ t("dashboard.noEvents") }}</div>
          <div
            v-for="ev in app.activity.slice(0, 6)"
            :key="ev.id"
            class="flex items-center gap-3 py-2.5 border-b border-white/[0.035] last:border-0"
          >
            <div class="w-7 h-7 rounded-lg bg-white/[0.04] border border-white/[0.05] grid place-items-center shrink-0">
              <component :is="eventIcons[ev.kind] ?? Activity" :size="13" :class="severityColor[ev.severity]" />
            </div>
            <div class="text-[12px] flex-1 min-w-0 truncate" :title="ev.text">{{ ev.text }}</div>
            <div class="text-[10.5px] text-dim shrink-0">{{ timeAgo(ev.at, locale) }}</div>
          </div>
        </div>
      </div>

      <div class="col-span-4 space-y-4">
        <div class="panel p-5 relative overflow-hidden reveal" :style="{ '--i': 8 }">
          <div
            class="absolute inset-0 pointer-events-none opacity-70"
            style="background: radial-gradient(300px 140px at 85% -10%, color-mix(in srgb, var(--ambient) 16%, transparent), transparent 70%)"
          ></div>
          <div class="flex items-center justify-between relative">
            <div class="section-title">{{ t("dashboard.activeProfile") }}</div>
            <Layers :size="15" class="text-dim" />
          </div>
          <template v-if="profiles.active">
            <div class="flex items-center gap-2.5 mt-3">
              <span
                class="w-9 h-9 rounded-xl border border-white/10 shrink-0"
                :style="{ background: `linear-gradient(135deg, ${profiles.active.rgb.color}, ${profiles.active.rgb.secondary})` }"
              ></span>
              <div class="min-w-0">
                <div class="text-[18px] font-display font-semibold tracking-tight truncate" :title="profiles.active.name">{{ profiles.active.name }}</div>
                <div class="text-[10.5px] text-dim">{{ profiles.active.apps.length }} {{ t("profiles.apps").toLowerCase() }} · {{ profileEffectLabel(profiles.active.rgb.effect) }}</div>
              </div>
            </div>
            <button class="btn btn-ghost w-full mt-3 !h-8 !text-[11px] justify-between" @click="router.push('/profiles')">
              {{ t("nav.profiles") }}
              <ChevronRight :size="13" />
            </button>
          </template>
          <template v-else>
            <div class="text-[12px] text-dim mt-3">{{ t("profiles.noProfile") }}</div>
            <button class="btn !h-8 !text-[11.5px] mt-3" @click="router.push('/profiles')">{{ t("profiles.create") }}</button>
          </template>
        </div>

        <div class="panel p-5 reveal" :style="{ '--i': 9 }">
          <div class="flex items-center justify-between mb-2">
            <div class="section-title">{{ t("dashboard.detection") }}</div>
            <span class="flex items-center gap-1.5 text-[10.5px]" :class="app.scanning ? 'text-ok' : 'text-warn'">
              <span class="status-dot live-dot" :class="app.scanning ? 'bg-ok' : 'bg-warn'"></span>
              {{ app.scanning ? t("shell.scanActive") : t("shell.offline") }}
            </span>
          </div>
          <div class="text-[11.5px] text-dim flex justify-between py-1.5">
            <span>{{ t("dashboard.scanInterval") }}</span>
            <span class="mono text-muted">{{ (app.settings.scanIntervalMs / 1000).toFixed(0) }} s</span>
          </div>
          <div class="text-[11.5px] text-dim flex justify-between py-1.5">
            <span>{{ t("diagnostics.adapters") }}</span>
            <span class="mono text-muted">{{ adapterCount }}</span>
          </div>
          <div class="text-[11.5px] text-dim flex justify-between py-1.5">
            <span>{{ t("device.updatedAt") }}</span>
            <span class="mono text-muted">{{ devices.lastScanAt ? timeAgo(devices.lastScanAt, locale) : "—" }}</span>
          </div>
          <button class="btn w-full mt-3 !h-8 !text-[11.5px]" @click="router.push('/settings')">
            <Settings2 :size="13" />
            {{ t("settings.title") }}
          </button>
        </div>

        <div class="panel p-5 reveal" :style="{ '--i': 10 }">
          <div class="section-title mb-3">{{ t("dashboard.quickActions") }}</div>
          <div class="grid grid-cols-3 gap-2">
            <button class="card-flat p-3 text-left panel-hover" @click="router.push('/keyboard')">
              <Keyboard :size="16" class="text-accent mb-2" />
              <div class="text-[11px] font-medium">Keyboard</div>
            </button>
            <button class="card-flat p-3 text-left panel-hover" @click="router.push('/mouse')">
              <Mouse :size="16" class="text-accent-2 mb-2" />
              <div class="text-[11px] font-medium">Mouse</div>
            </button>
            <button class="card-flat p-3 text-left panel-hover" @click="router.push('/audio')">
              <Headphones :size="16" class="text-info mb-2" />
              <div class="text-[11px] font-medium">Audio</div>
            </button>
            <button class="card-flat p-3 text-left panel-hover" @click="router.push('/rgb')">
              <Palette :size="16" class="text-accent mb-2" />
              <div class="text-[11px] font-medium">{{ t("nav.rgb") }}</div>
            </button>
            <button class="card-flat p-3 text-left panel-hover" @click="router.push('/macros')">
              <Command :size="16" class="text-accent-2 mb-2" />
              <div class="text-[11px] font-medium">{{ t("nav.macros") }}</div>
            </button>
            <button class="card-flat p-3 text-left panel-hover" @click="router.push('/diagnostics')">
              <Activity :size="16" class="text-ok mb-2" />
              <div class="text-[11px] font-medium leading-tight">{{ t("nav.diagnostics") }}</div>
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
