<script setup lang="ts">
import { computed } from "vue";
import { useRouter } from "vue-router";
import { ArrowUpRight, Bluetooth, Headphones, Keyboard, Mouse, Gamepad2, Settings2, Usb, Wifi } from "lucide-vue-next";
import type { DeviceView } from "../lib/types";
import { compatibilitySummary, deviceKindLabel } from "../lib/format";
import { capabilityLabel } from "../lib/capabilities";
import { useI18n } from "../lib/i18n";
import { useDevicesStore } from "../stores/devices";
import BatteryRing from "./BatteryRing.vue";
import KeyboardModel from "./KeyboardModel.vue";
import MouseModel from "./MouseModel.vue";
import HeadsetModel from "./HeadsetModel.vue";
import { deviceImage } from "../lib/deviceImages";

const props = withDefaults(defineProps<{ device: DeviceView; variant?: "grid" | "compact" | "showcase" }>(), {
  variant: "grid",
});
const router = useRouter();
const { t, locale } = useI18n();
const devices = useDevicesStore();

const kindIcon = computed(() => {
  switch (props.device.kind) {
    case "keyboard": return Keyboard;
    case "mouse": return Mouse;
    case "headset":
    case "microphone": return Headphones;
    case "gamepad": return Gamepad2;
    default: return Usb;
  }
});

const connectionIcon = computed(() => {
  if (props.device.connection === "wireless") return Wifi;
  if (props.device.connection === "bluetooth") return Bluetooth;
  return Usb;
});

const compat = computed(() => compatibilitySummary(props.device.capabilities));
const working = computed(() => props.device.capabilities.filter((c) => c.status === "working"));
const researchCount = computed(() => props.device.capabilities.filter((c) => c.status === "research").length);

const accent = computed(() => {
  if (props.device.kind === "keyboard") return "#ff4fa3";
  if (props.device.kind === "mouse") return "#a78bfa";
  return "#6ee7f9";
});

const isNew = computed(() => devices.recentlyConnected.has(props.device.id));
const photo = computed(() => deviceImage(props.device));
const batteryHint = computed(() => {
  if (props.device.battery?.percent != null) return null;
  if (props.device.kind === "keyboard") return t("battery.hintCable");
  if (props.device.kind === "headset") return t("battery.hintReceiver");
  return null;
});

const studioPath = computed(() => {
  if (props.device.kind === "keyboard") return `/keyboard?device=${props.device.id}`;
  if (props.device.kind === "mouse") return `/mouse?device=${props.device.id}`;
  if (props.device.kind === "headset") return `/audio?device=${props.device.id}`;
  return `/devices/${props.device.id}`;
});

function open() {
  router.push(`/devices/${props.device.id}`);
}
</script>

<template>
  <div
    class="device-card group cursor-pointer"
    :class="[
      variant === 'compact' ? 'p-4' : variant === 'showcase' ? 'p-6' : 'p-5',
      { 'is-new': isNew },
    ]"
    :style="{ '--tint': accent }"
    role="link"
    tabindex="0"
    @click="open"
    @keydown.enter="open"
  >
    <div class="device-scan"></div>

    <div class="flex items-center justify-between relative z-10">
      <span class="chip">
        <component :is="kindIcon" :size="11" />
        {{ deviceKindLabel(device.kind, locale) }}
      </span>
      <div class="flex items-center gap-2">
        <span class="flex items-center gap-1.5 text-[10.5px]" :class="device.online ? 'text-ok' : 'text-dim'">
          <span class="status-dot" :class="device.online ? 'bg-ok live-dot' : 'bg-dim'"></span>
          {{ device.online ? t("device.online") : t("device.offline") }}
        </span>
        <router-link
          :to="studioPath"
          class="device-actions icon-btn !w-7 !h-7"
          :title="t('nav.settings')"
          @click.stop
        >
          <Settings2 :size="13" />
        </router-link>
      </div>
    </div>

    <div
      class="relative flex items-center justify-center"
      :class="variant === 'compact' ? 'h-[104px] mt-1' : variant === 'showcase' ? 'h-[216px] mt-3' : 'h-[156px] mt-2'"
    >
      <div
        class="absolute left-1/2 bottom-3 -translate-x-1/2 w-[76%] h-[46px] rounded-[50%] blur-2xl opacity-50 transition-opacity duration-300 group-hover:opacity-80"
        :style="{ background: `radial-gradient(closest-side, ${accent}55, transparent)` }"
      ></div>
      <div class="device-visual w-full h-full flex items-center justify-center">
        <img
          v-if="photo"
          :src="photo"
          :alt="device.name"
          class="max-h-full max-w-full object-contain select-none pointer-events-none drop-shadow-[0_16px_28px_rgba(0,0,0,0.5)]"
          :class="device.kind === 'keyboard' ? 'w-[92%] rounded-xl' : 'h-full w-auto rounded-2xl'"
          draggable="false"
          loading="lazy"
        />
        <KeyboardModel
          v-else-if="device.kind === 'keyboard'"
          variant="mini"
          :animate="false"
          :interactive="false"
          :primary="accent"
          secondary="#a78bfa"
          :brightness="78"
        />
        <div v-else-if="device.kind === 'mouse'" class="h-[150px] flex justify-center">
          <MouseModel compact :accent="accent" class="!h-[150px] !w-auto" />
        </div>
        <HeadsetModel v-else-if="device.kind === 'headset'" compact :accent="accent" class="max-h-[150px] w-auto mx-auto" />
        <div v-else class="w-[74px] h-[74px] rounded-[22px] border border-white/10 bg-white/[0.03] grid place-items-center">
          <component :is="kindIcon" :size="30" class="text-dim" />
        </div>
      </div>
    </div>

    <div class="min-w-0 mt-1">
      <div class="font-semibold tracking-tight truncate" :class="variant === 'compact' ? 'text-[13.5px]' : variant === 'showcase' ? 'text-[18px]' : 'text-[15.5px]'">{{ device.name }}</div>
      <div class="text-[11px] text-dim mt-0.5 truncate">{{ device.vendor }} · {{ device.model }}</div>
    </div>

    <div class="flex items-center gap-1.5 mt-3 flex-wrap">
      <span class="chip">
        <component :is="connectionIcon" :size="11" />
        {{ device.connectionLabel }}
      </span>
      <span class="chip" :title="`${working.length} · ${researchCount}`">
        {{ t("compat.short", { n: compat.confirmed, m: compat.total }) }}
      </span>
      <span v-if="working.length" class="chip chip-ok !border-ok/20">
        {{ working.length }} {{ t("status.working").toLowerCase() }}
      </span>
    </div>

    <div v-if="variant !== 'compact'" class="flex flex-wrap gap-x-3 gap-y-1 mt-3 min-h-[18px]">
      <span v-for="cap in working.slice(0, variant === 'showcase' ? 5 : 3)" :key="cap.id" class="text-[10px] text-dim flex items-center gap-1">
        <span class="w-1 h-1 rounded-full bg-ok/70"></span>
        {{ capabilityLabel(cap.id, locale) }}
      </span>
      <span v-if="working.length > (variant === 'showcase' ? 5 : 3)" class="text-[10px] text-dim">+{{ working.length - (variant === 'showcase' ? 5 : 3) }}</span>
    </div>

    <div class="divider" :class="variant === 'compact' ? 'my-2.5' : 'my-3.5'"></div>

    <div class="flex items-center justify-between mt-auto gap-2">
      <div class="flex items-center gap-2 min-w-0">
        <BatteryRing :battery="device.battery" :size="variant === 'showcase' ? 48 : 42" :stroke="4" :show-label="false" />
        <span v-if="batteryHint" class="text-[9.5px] text-dim leading-snug max-w-[124px]">{{ batteryHint }}</span>
      </div>
      <span class="flex items-center gap-1 text-[11px] font-medium text-dim group-hover:text-text transition-colors shrink-0">
        {{ t("common.details") }}
        <ArrowUpRight :size="13" class="transition-transform group-hover:translate-x-0.5 group-hover:-translate-y-0.5" />
      </span>
    </div>
  </div>
</template>
