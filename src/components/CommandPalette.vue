<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
import {
  Activity,
  Command,
  CornerDownLeft,
  Cpu,
  Headphones,
  Keyboard,
  Layers,
  LayoutDashboard,
  Monitor,
  MonitorCog,
  Mouse,
  MousePointerClick,
  Palette,
  RefreshCw,
  Search,
  Settings2,
  SlidersHorizontal,
  Sparkles,
  Usb,
  type LucideIcon,
} from "lucide-vue-next";
import { useAppStore } from "../stores/app";
import { useDevicesStore } from "../stores/devices";
import { useProfilesStore } from "../stores/profiles";
import { useI18n } from "../lib/i18n";
import { deviceKindLabel } from "../lib/format";

interface Entry {
  id: string;
  group: string;
  label: string;
  hint?: string;
  icon: LucideIcon;
  keywords?: string;
  run: () => void;
}

const app = useAppStore();
const devices = useDevicesStore();
const profiles = useProfilesStore();
const router = useRouter();
const { t, locale } = useI18n();

const query = ref("");
const active = ref(0);
const inputRef = ref<HTMLInputElement | null>(null);
const listRef = ref<HTMLElement | null>(null);

function close() {
  app.closePalette();
  query.value = "";
  active.value = 0;
}

function go(path: string) {
  close();
  router.push(path);
}

const pages: { id: string; path: string; label: string; icon: LucideIcon; keywords: string }[] = [
  { id: "dashboard", path: "/", label: t("nav.dashboard"), icon: LayoutDashboard, keywords: "overview главная обзор" },
  { id: "devices", path: "/devices", label: t("nav.devices"), icon: Usb, keywords: "devices устройства showcase" },
  { id: "intelligence", path: "/intelligence", label: t("nav.intelligence"), icon: Cpu, keywords: "telemetry телеметрия батарея протокол hid research" },
  { id: "rgb", path: "/rgb", label: t("nav.rgb"), icon: Palette, keywords: "rgb подсветка цвет lighting" },
  { id: "keyboard", path: "/keyboard", label: t("nav.keyboard"), icon: Keyboard, keywords: "keyboard клавиатура" },
  { id: "visualizer", path: "/visualizer", label: t("nav.visualizer"), icon: Monitor, keywords: "visualizer визуализатор raw input нажатия тест клавиатуры" },
  { id: "autoclicker", path: "/autoclicker", label: t("nav.autoclicker"), icon: MousePointerClick, keywords: "autoclicker автокликер клики мышь f6" },
  { id: "mouse", path: "/mouse", label: t("nav.mouse"), icon: Mouse, keywords: "mouse мышь dpi" },
  { id: "audio", path: "/audio", label: t("nav.audio"), icon: Headphones, keywords: "audio звук гарнитура" },
  { id: "input", path: "/input", label: t("nav.input"), icon: SlidersHorizontal, keywords: "remap переназначение клавиши hooks polling" },
  { id: "profiles", path: "/profiles", label: t("nav.profiles"), icon: Layers, keywords: "profiles профили" },
  { id: "macros", path: "/macros", label: t("nav.macros"), icon: Command, keywords: "macros макросы" },
  { id: "drivers", path: "/drivers", label: t("nav.drivers"), icon: MonitorCog, keywords: "drivers драйверы" },
  { id: "settings", path: "/settings", label: t("nav.settings"), icon: Settings2, keywords: "settings настройки" },
];

const actions = computed<Entry[]>(() => [
  {
    id: "rescan",
    group: "actions",
    label: t("palette.rescan"),
    icon: RefreshCw,
    keywords: "scan rescan сканирование обновить",
    run: () => {
      close();
      void devices.refresh();
    },
  },
  {
    id: "ambient",
    group: "actions",
    label: t("palette.toggleAmbient"),
    icon: Sparkles,
    keywords: "ambient подсветка интерфейс",
    run: () => {
      close();
      app.toggleAmbient();
    },
  },
  {
    id: "diagnostics",
    group: "actions",
    label: t("palette.openDiagnostics"),
    icon: Activity,
    keywords: "diagnostics протокол инспектор hid",
    run: () => go("/diagnostics"),
  },
  {
    id: "settings",
    group: "actions",
    label: t("palette.openSettings"),
    icon: Settings2,
    keywords: "settings настройки параметры",
    run: () => go("/settings"),
  },
]);

const pagesEntries = computed<Entry[]>(() =>
  pages.map((p) => ({
    id: `page-${p.id}`,
    group: "pages",
    label: p.label,
    icon: p.icon,
    keywords: p.keywords,
    run: () => go(p.path),
  }))
);

const deviceEntries = computed<Entry[]>(() =>
  devices.devices.map((d) => ({
    id: `device-${d.id}`,
    group: "devices",
    label: d.name,
    hint: `${deviceKindLabel(d.kind, locale.value)} · ${d.connectionLabel}`,
    icon: Usb,
    keywords: `${d.vendor} ${d.model} ${d.connectionLabel}`,
    run: () => go(`/devices/${d.id}`),
  }))
);

const profileEntries = computed<Entry[]>(() =>
  profiles.profiles
    .filter((p) => p.id !== profiles.activeId)
    .map((p) => ({
      id: `profile-${p.id}`,
      group: "profiles",
      label: p.name,
      hint: t("palette.profiles"),
      icon: Layers,
      keywords: "profile профиль",
      run: () => {
        close();
        void profiles.activate(p.id);
      },
    }))
);

function score(entry: Entry, q: string) {
  const haystack = `${entry.label} ${entry.hint ?? ""} ${entry.keywords ?? ""}`.toLowerCase();
  if (!q) return 1;
  const label = entry.label.toLowerCase();
  if (label.startsWith(q)) return 4;
  if (label.includes(q)) return 3;
  if (haystack.includes(q)) return 2;
  const words = q.split(/\s+/).filter(Boolean);
  if (words.length && words.every((w) => haystack.includes(w))) return 1;
  return 0;
}

const groups = computed<Entry[]>(() => {
  const q = query.value.trim().toLowerCase();
  const all = [...deviceEntries.value, ...pagesEntries.value, ...actions.value, ...profileEntries.value];
  if (!q) {
    const recent = all.filter((e) => e.group === "devices" || e.group === "pages").slice(0, 9);
    return [...recent, ...actions.value.slice(0, 2)];
  }
  return all
    .map((e) => ({ e, s: score(e, q) }))
    .filter((x) => x.s > 0)
    .sort((a, b) => b.s - a.s)
    .slice(0, 14)
    .map((x) => x.e);
});

const grouped = computed(() => {
  const order = ["devices", "pages", "actions", "profiles"];
  const map = new Map<string, Entry[]>();
  for (const entry of groups.value) {
    const arr = map.get(entry.group) ?? [];
    arr.push(entry);
    map.set(entry.group, arr);
  }
  return order.filter((g) => map.has(g)).map((g) => ({ id: g, label: t(`palette.${g}`), items: map.get(g)! }));
});

const flat = computed(() => grouped.value.flatMap((g) => g.items));

watch(flat, () => {
  active.value = 0;
});

watch(
  () => app.paletteOpen,
  async (open) => {
    if (open) {
      await nextTick();
      inputRef.value?.focus();
      active.value = 0;
    }
  }
);

function move(delta: number) {
  const count = flat.value.length;
  if (!count) return;
  active.value = (active.value + delta + count) % count;
  void nextTick(() => {
    listRef.value?.querySelector<HTMLElement>(`[data-idx="${active.value}"]`)?.scrollIntoView({ block: "nearest" });
  });
}

function runActive() {
  const entry = flat.value[active.value];
  entry?.run();
}

function onKey(ev: KeyboardEvent) {
  if (!app.paletteOpen) return;
  if (ev.key === "ArrowDown") {
    ev.preventDefault();
    move(1);
  } else if (ev.key === "ArrowUp") {
    ev.preventDefault();
    move(-1);
  } else if (ev.key === "Enter") {
    ev.preventDefault();
    runActive();
  } else if (ev.key === "Escape") {
    ev.preventDefault();
    close();
  }
}

function globalKey(ev: KeyboardEvent) {
  if ((ev.ctrlKey || ev.metaKey) && ev.key.toLowerCase() === "k") {
    ev.preventDefault();
    if (app.paletteOpen) close();
    else app.openPalette();
  }
}

function indexOf(entry: Entry) {
  return flat.value.indexOf(entry);
}
onMounted(() => {
  window.addEventListener("keydown", globalKey);
  window.addEventListener("keydown", onKey);
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", globalKey);
  window.removeEventListener("keydown", onKey);
});
</script>

<template>
  <Teleport to="body">
    <transition name="fade">
      <div v-if="app.paletteOpen" class="fixed inset-0 z-[90] bg-black/55 backdrop-blur-[3px]" @click="close"></div>
    </transition>
    <transition name="zoom-fade">
      <div
        v-if="app.paletteOpen"
        class="fixed left-1/2 top-[12vh] -translate-x-1/2 z-[91] w-[min(640px,calc(100vw-40px))] overflow-hidden rounded-[22px] border border-white/10 shadow-[0_50px_120px_-40px_rgba(0,0,0,1)]"
        style="background: linear-gradient(165deg, rgba(24, 19, 36, 0.97), rgba(11, 9, 17, 0.98)); backdrop-filter: blur(22px)"
        @click.stop
      >
        <div class="flex items-center gap-3 px-5 h-[58px] border-b border-white/[0.06]">
          <Search :size="17" class="text-dim shrink-0" />
          <input
            ref="inputRef"
            v-model="query"
            class="flex-1 bg-transparent outline-none text-[14.5px] placeholder:text-dim"
            :placeholder="t('palette.placeholder')"
            spellcheck="false"
          />
          <span class="kbd">Esc</span>
        </div>

        <div ref="listRef" class="max-h-[52vh] overflow-y-auto scroll-region py-2">
          <template v-if="grouped.length">
            <div v-for="group in grouped" :key="group.id">
              <div class="overline px-5 pt-2 pb-1.5">{{ group.label }}</div>
              <button
                v-for="entry in group.items"
                :key="entry.id"
                :data-idx="indexOf(entry)"
                class="w-full flex items-center gap-3 px-5 h-[42px] text-left transition-colors"
                :class="indexOf(entry) === active ? 'bg-white/[0.06] text-text' : 'text-muted hover:bg-white/[0.03]'"
                @click="entry.run()"
                @pointermove="active = indexOf(entry)"
              >
                <span
                  class="w-[26px] h-[26px] rounded-lg grid place-items-center shrink-0 border transition-colors"
                  :class="indexOf(entry) === active ? 'border-accent/40 bg-accent/10 text-accent' : 'border-white/[0.07] bg-white/[0.03] text-dim'"
                >
                  <component :is="entry.icon" :size="13" />
                </span>
                <span class="text-[13px] font-medium truncate">{{ entry.label }}</span>
                <span v-if="entry.hint" class="text-[10.5px] text-dim truncate ml-auto max-w-[45%]">{{ entry.hint }}</span>
                <CornerDownLeft v-if="indexOf(entry) === active" :size="12" class="text-dim shrink-0" />
              </button>
            </div>
          </template>
          <div v-else class="px-5 py-10 text-center text-[12.5px] text-dim">{{ t("palette.noResults") }}</div>
        </div>

        <div class="px-5 h-[38px] flex items-center gap-3 border-t border-white/[0.05] text-[10.5px] text-dim">
          <span>{{ t("palette.hint") }}</span>
          <span class="ml-auto mono">Ctrl K</span>
        </div>
      </div>
    </transition>
  </Teleport>
</template>
