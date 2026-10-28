<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { ArrowUpRight, CircleDashed, Grid2x2, Grid3x3, Headphones, Keyboard, List, Mouse, Search, Sparkles, Usb } from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import DeviceCard from "../components/DeviceCard.vue";
import BatteryRing from "../components/BatteryRing.vue";
import EmptyState from "../components/EmptyState.vue";
import { useDevicesStore } from "../stores/devices";
import { useI18n } from "../lib/i18n";
import { compatibilitySummary, deviceKindLabel } from "../lib/format";

type ViewMode = "grid" | "compact" | "list" | "showcase";

const VIEW_KEY = "udm.devices.view";
const devices = useDevicesStore();
const router = useRouter();
const { t, locale } = useI18n();

function loadView(): ViewMode {
  try {
    const raw = localStorage.getItem(VIEW_KEY);
    if (raw === "grid" || raw === "compact" || raw === "list" || raw === "showcase") return raw;
  } catch {}
  return "grid";
}

const kindFilter = ref("all");
const query = ref("");
const viewMode = ref<ViewMode>(loadView());
watch(viewMode, (value) => {
  try {
    localStorage.setItem(VIEW_KEY, value);
  } catch {}
});

const modes: { id: ViewMode; icon: typeof Grid3x3; label: string }[] = [
  { id: "grid", icon: Grid3x3, label: "devices.viewGrid" },
  { id: "compact", icon: Grid2x2, label: "devices.viewCompact" },
  { id: "list", icon: List, label: "devices.viewList" },
  { id: "showcase", icon: Sparkles, label: "devices.viewShowcase" },
];

const kinds = computed(() => {
  const set = new Set(devices.devices.map((d) => d.kind));
  return ["all", ...Array.from(set)];
});

const filtered = computed(() => {
  const q = query.value.trim().toLowerCase();
  return devices.devices.filter((d) => {
    if (kindFilter.value !== "all" && d.kind !== kindFilter.value) return false;
    if (!q) return true;
    return `${d.name} ${d.vendor} ${d.model} ${d.connectionLabel}`.toLowerCase().includes(q);
  });
});

function kindIcon(kind: string) {
  if (kind === "keyboard") return Keyboard;
  if (kind === "mouse") return Mouse;
  if (kind === "headset" || kind === "microphone") return Headphones;
  return Usb;
}

function accentFor(kind: string) {
  if (kind === "keyboard") return "#ff4fa3";
  if (kind === "mouse") return "#a78bfa";
  return "#6ee7f9";
}

function open(id: string) {
  router.push(`/devices/${id}`);
}
</script>

<template>
  <div>
    <PageHeader :title="t('nav.devices')" :subtitle="t('devices.subtitle')" :overline="t('nav.devices')" />

    <div class="flex items-center gap-3 mb-5 flex-wrap">
      <div class="flex items-center gap-1.5 flex-wrap">
        <button
          v-for="k in kinds"
          :key="k"
          class="h-8 px-3 rounded-lg text-[12px] font-medium border transition-colors"
          :class="kindFilter === k ? 'border-white/15 bg-white/[0.07] text-text' : 'border-transparent text-dim hover:text-muted hover:bg-white/[0.03]'"
          @click="kindFilter = k"
        >
          {{ k === "all" ? t("devices.filterAll") : deviceKindLabel(k, locale) }}
        </button>
      </div>
      <div class="flex-1"></div>
      <div class="relative w-[240px]">
        <Search :size="14" class="absolute left-3 top-1/2 -translate-y-1/2 text-dim" />
        <input v-model="query" class="input h-9 pl-9" :placeholder="t('common.search')" />
      </div>
      <div class="inline-flex p-1 rounded-xl bg-white/[0.035] border border-white/[0.06] gap-0.5">
        <button
          v-for="mode in modes"
          :key="mode.id"
          class="w-7 h-7 rounded-[8px] grid place-items-center transition-all duration-150"
          :class="viewMode === mode.id ? 'bg-white/[0.09] text-text' : 'text-dim hover:text-muted'"
          :data-tip="t(mode.label)"
          :title="t(mode.label)"
          @click="viewMode = mode.id"
        >
          <component :is="mode.icon" :size="13" />
        </button>
      </div>
    </div>

    <Transition name="mode" mode="out-in">
      <div v-if="filtered.length" :key="viewMode">
        <div v-if="viewMode === 'grid'" class="grid grid-cols-3 gap-4">
          <DeviceCard v-for="d in filtered" :key="d.id" :device="d" />
        </div>

        <div v-else-if="viewMode === 'compact'" class="grid grid-cols-4 gap-3">
          <DeviceCard v-for="d in filtered" :key="d.id" :device="d" variant="compact" />
        </div>

        <div v-else-if="viewMode === 'showcase'" class="grid grid-cols-2 gap-5">
          <DeviceCard v-for="d in filtered" :key="d.id" :device="d" variant="showcase" />
        </div>

        <div v-else class="space-y-2">
          <button
            v-for="d in filtered"
            :key="d.id"
            class="panel w-full px-4 py-3 flex items-center gap-4 text-left hover:border-white/15 hover:bg-white/[0.02] transition-colors group"
            @click="open(d.id)"
          >
            <span
              class="w-10 h-10 rounded-xl border border-white/10 grid place-items-center shrink-0"
              :style="{ background: `linear-gradient(135deg, ${accentFor(d.kind)}24, transparent)` }"
            >
              <component :is="kindIcon(d.kind)" :size="17" :style="{ color: accentFor(d.kind) }" />
            </span>
            <div class="min-w-0 w-[24%]">
              <div class="text-[12.5px] font-medium truncate" :title="d.name">{{ d.name }}</div>
              <div class="text-[10.5px] text-dim truncate" :title="`${d.vendor} · ${d.model}`">{{ d.vendor }} · {{ d.model }}</div>
            </div>
            <span class="chip !h-6 shrink-0">{{ deviceKindLabel(d.kind, locale) }}</span>
            <span class="chip !h-6 shrink-0 hidden xl:inline-flex">{{ d.connectionLabel }}</span>
            <span class="chip !h-6 shrink-0">
              <span class="status-dot" :class="d.online ? 'bg-ok' : 'bg-dim'"></span>
              {{ d.online ? t("device.online") : t("device.offline") }}
            </span>
            <div class="flex-1"></div>
            <span class="text-[10.5px] text-dim hidden xl:block">
              {{ t("compat.short", { n: compatibilitySummary(d.capabilities).confirmed, m: compatibilitySummary(d.capabilities).total }) }}
            </span>
            <BatteryRing :battery="d.battery" :size="32" :stroke="3" :show-label="false" />
            <ArrowUpRight :size="14" class="text-dim group-hover:text-text transition-colors shrink-0" />
          </button>
        </div>
      </div>

      <div v-else :key="'empty-' + viewMode" class="panel p-2">
        <EmptyState :icon="CircleDashed" :title="t('devices.emptyTitle')" :body="t('devices.emptyBody')" />
      </div>
    </Transition>

    <div class="text-[11px] text-dim mt-4">{{ filtered.length }} / {{ devices.devices.length }}</div>

    <div class="card p-4 mt-4 flex items-start gap-3">
      <span class="w-8 h-8 rounded-lg grid place-items-center bg-accent/[0.08] border border-accent/15 shrink-0">
        <Sparkles :size="14" class="text-accent" />
      </span>
      <div>
        <div class="text-[11.5px] font-medium">{{ t("devices.brandNoteTitle") }}</div>
        <div class="text-[10.5px] text-dim leading-relaxed mt-1 max-w-[920px]">{{ t("devices.brandNoteBody") }}</div>
      </div>
    </div>
  </div>
</template>
