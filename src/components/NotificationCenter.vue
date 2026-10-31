<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import {
  Activity,
  Battery,
  Bell,
  CheckCheck,
  ChevronRight,
  Command,
  Layers,
  ScanLine,
  Settings2,
  Usb,
  type LucideIcon,
} from "lucide-vue-next";
import { useAppStore, type ActivityEvent } from "../stores/app";
import { useI18n } from "../lib/i18n";
import { timeAgo } from "../lib/format";

const app = useAppStore();
const router = useRouter();
const { t, locale } = useI18n();

const open = ref(false);
const root = ref<HTMLElement | null>(null);
const filter = ref<"all" | "unread">("all");
const seenAt = ref(Number(localStorage.getItem("udm.activitySeen") ?? 0));

const icons: Record<ActivityEvent["kind"], LucideIcon> = {
  device: Usb,
  scan: ScanLine,
  profile: Layers,
  settings: Settings2,
  macro: Command,
  battery: Battery,
  app: Activity,
};

const tone: Record<ActivityEvent["severity"], string> = {
  info: "text-info bg-info/10 border-info/20",
  success: "text-ok bg-ok/10 border-ok/20",
  warn: "text-warn bg-warn/10 border-warn/20",
  error: "text-bad bg-bad/10 border-bad/20",
};

const unread = computed(() => app.activity.filter((e) => e.at > seenAt.value).length);
const list = computed(() =>
  filter.value === "unread" ? app.activity.filter((e) => e.at > seenAt.value) : app.activity
);

function markSeen() {
  seenAt.value = Date.now();
  localStorage.setItem("udm.activitySeen", String(seenAt.value));
}

function toggle() {
  open.value = !open.value;
  if (open.value) markSeen();
}

function targetFor(ev: ActivityEvent) {
  if (ev.deviceId) return `/devices/${ev.deviceId}`;
  switch (ev.kind) {
    case "profile":
      return "/profiles";
    case "macro":
      return "/macros";
    case "scan":
    case "battery":
      return "/diagnostics";
    case "settings":
      return "/settings";
    default:
      return null;
  }
}

function go(ev: ActivityEvent) {
  const target = targetFor(ev);
  open.value = false;
  if (target) router.push(target);
}

function onDocClick(event: MouseEvent) {
  if (open.value && root.value && !root.value.contains(event.target as Node)) open.value = false;
}

onMounted(() => document.addEventListener("click", onDocClick));
onBeforeUnmount(() => document.removeEventListener("click", onDocClick));
</script>

<template>
  <div ref="root" class="relative">
    <button class="icon-btn relative" :title="t('shell.notifications')" @click.stop="toggle">
      <Bell :size="17" />
      <span
        v-if="unread > 0"
        class="absolute -top-0.5 -right-0.5 min-w-[15px] h-[15px] px-1 rounded-full bg-accent text-[9px] font-bold text-black grid place-items-center"
      >
        {{ unread > 9 ? "9+" : unread }}
      </span>
      <span v-else-if="app.scanning" class="absolute top-1.5 right-1.5 w-1.5 h-1.5 rounded-full bg-ok/80"></span>
    </button>

    <transition name="pop">
      <div
        v-if="open"
        class="absolute right-0 top-11 w-[360px] rounded-[18px] overflow-hidden border border-white/[0.08] shadow-[0_36px_90px_-30px_rgba(0,0,0,1)] z-50"
        style="background: linear-gradient(170deg, rgba(24, 19, 36, 0.97), rgba(11, 9, 17, 0.98)); backdrop-filter: blur(22px)"
        @click.stop
      >
        <div class="px-4 h-[48px] flex items-center justify-between border-b border-white/[0.06]">
          <div class="flex items-center gap-2">
            <span class="text-[12.5px] font-semibold">{{ t("shell.notifications") }}</span>
            <span v-if="unread" class="chip chip-accent !h-[19px] !text-[9.5px]">{{ t("shell.unreadCount", { n: unread }) }}</span>
          </div>
          <button class="text-[10.5px] text-dim hover:text-text flex items-center gap-1 transition-colors" @click="markSeen">
            <CheckCheck :size="12" />
            {{ t("shell.markRead") }}
          </button>
        </div>

        <div class="px-3 pt-2 flex items-center gap-1">
          <button
            class="h-7 px-2.5 rounded-lg text-[10.5px] font-medium transition-colors"
            :class="filter === 'all' ? 'bg-white/[0.07] text-text' : 'text-dim hover:text-muted'"
            @click="filter = 'all'"
          >
            {{ t("common.all") }}
          </button>
          <button
            class="h-7 px-2.5 rounded-lg text-[10.5px] font-medium transition-colors"
            :class="filter === 'unread' ? 'bg-white/[0.07] text-text' : 'text-dim hover:text-muted'"
            @click="filter = 'unread'"
          >
            {{ t("shell.unreadCount", { n: unread }) }}
          </button>
          <span class="ml-auto text-[10px] text-dim pr-1">{{ app.activity.length }}</span>
        </div>

        <div class="max-h-[340px] overflow-y-auto scroll-region py-1.5">
          <div v-if="!list.length" class="px-5 py-10 text-center">
            <div class="w-10 h-10 rounded-2xl mx-auto grid place-items-center border border-white/[0.07] bg-white/[0.03]">
              <Bell :size="16" class="text-dim" />
            </div>
            <div class="text-[11.5px] text-dim mt-3 leading-relaxed px-4">{{ t("shell.notificationEmpty") }}</div>
          </div>
          <button
            v-for="ev in list.slice(0, 16)"
            :key="ev.id"
            class="w-full px-3 text-left"
            @click="go(ev)"
          >
            <div class="flex items-start gap-2.5 px-2 py-2.5 rounded-xl hover:bg-white/[0.035] transition-colors">
              <span class="w-[26px] h-[26px] rounded-lg border grid place-items-center shrink-0 mt-0.5" :class="tone[ev.severity]">
                <component :is="icons[ev.kind] ?? Activity" :size="12" />
              </span>
              <span class="min-w-0 flex-1">
                <span class="block text-[11.5px] leading-snug" :class="ev.at > seenAt ? 'text-text' : 'text-muted'">{{ ev.text }}</span>
                <span class="block text-[10px] text-dim mt-0.5">{{ timeAgo(ev.at, locale) }}</span>
              </span>
              <ChevronRight v-if="targetFor(ev)" :size="12" class="text-dim shrink-0 mt-1.5" />
            </div>
          </button>
        </div>
      </div>
    </transition>
  </div>
</template>
