<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { ExternalLink, Gauge, Loader2, Mouse, RefreshCw, RotateCcw, Timer, Trash2 } from "lucide-vue-next";
import type { MouseSystem } from "../../lib/types";
import { openExternal } from "../../lib/api";
import { useI18n } from "../../lib/i18n";
import { useInputStore } from "../../stores/input";
import Segmented from "../Segmented.vue";
import Slider from "../Slider.vue";
import Toggle from "../Toggle.vue";

const { t } = useI18n();
const input = useInputStore();

const draft = ref<MouseSystem>({ pointerSpeed: 10, acceleration: true, wheelLines: 3, swapButtons: false });
const dirty = computed(() => JSON.stringify(draft.value) !== JSON.stringify(input.status.system));

function syncDraft() {
  draft.value = { ...input.status.system };
}

const speedRejected = ref(false);

async function apply() {
  const ok = await input.applySystem(draft.value);
  if (ok) syncDraft();
  speedRejected.value = !ok && draft.value.pointerSpeed !== input.status.system.pointerSpeed;
}

const windows = computed(() => [
  { value: "500", label: t("input.window500") },
  { value: "1000", label: t("input.window1000") },
  { value: "3000", label: t("input.window3000") },
]);
const windowMs = ref("1000");
const auto = ref(false);
const polling = ref(false);

async function poll() {
  polling.value = true;
  await input.pollStats(Number(windowMs.value));
  polling.value = false;
}

const rows = computed(() =>
  [...input.stats]
    .sort((a, b) => b.rate - a.rate)
    .map((row) => ({ ...row, kindLabel: row.kind === "mouse" ? t("input.mouse") : row.kind === "keyboard" ? t("input.keyboard") : "HID" }))
);

const maxRate = computed(() => Math.max(1, ...rows.value.map((r) => r.rate)));

let timer: number | null = null;
watch([auto, windowMs], () => {
  if (timer !== null) {
    window.clearInterval(timer);
    timer = null;
  }
  if (auto.value) {
    poll();
    timer = window.setInterval(poll, Math.max(500, Number(windowMs.value)));
  }
});

onMounted(() => {
  input.refresh().then(syncDraft);
  poll();
});

onBeforeUnmount(() => {
  if (timer !== null) window.clearInterval(timer);
  timer = null;
});
</script>

<template>
  <div class="grid grid-cols-12 gap-4">
    <div class="col-span-12 lg:col-span-5">
      <div class="card p-5 h-full">
        <div class="flex items-center justify-between mb-1">
          <div class="section-title">{{ t("input.mouseSystem") }}</div>
          <Mouse :size="14" class="text-dim" />
        </div>
        <div class="text-[11px] text-dim mb-4 leading-relaxed">{{ t("input.mouseSystemHint") }}</div>

        <div class="space-y-4">
          <div>
            <div class="flex items-center justify-between text-[11.5px] mb-1.5">
              <span class="text-muted">{{ t("input.pointerSpeed") }}</span>
              <span class="text-dim mono">{{ draft.pointerSpeed }} / 20</span>
            </div>
            <Slider v-model="draft.pointerSpeed" :min="1" :max="20" :step="1" />
            <button
              v-if="speedRejected"
              class="btn btn-ghost !h-7 mt-2 text-[11px]"
              @click="openExternal('ms-settings:mousetouchpad')"
            >
              <ExternalLink :size="12" />
              {{ t("input.windowsMouseSettings") }}
            </button>
          </div>

          <div>
            <div class="flex items-center justify-between text-[11.5px] mb-1.5">
              <span class="text-muted">{{ t("input.wheelLines") }}</span>
              <span class="text-dim mono">{{ draft.wheelLines }}</span>
            </div>
            <Slider v-model="draft.wheelLines" :min="1" :max="12" :step="1" />
          </div>

          <div class="flex items-center justify-between gap-4 pt-1">
            <div class="min-w-0">
              <div class="text-[12.5px] font-medium">{{ t("input.acceleration") }}</div>
              <div class="text-[11px] text-dim mt-0.5">{{ t("input.accelerationHint") }}</div>
            </div>
            <Toggle v-model="draft.acceleration" />
          </div>

          <div class="flex items-center justify-between gap-4">
            <div class="min-w-0">
              <div class="text-[12.5px] font-medium">{{ t("input.swapButtons") }}</div>
              <div class="text-[11px] text-dim mt-0.5">{{ t("input.swapButtonsHint") }}</div>
            </div>
            <Toggle v-model="draft.swapButtons" />
          </div>
        </div>

        <div class="flex items-center gap-2 mt-5 pt-4 border-t border-white/[0.05]">
          <button class="btn btn-primary !h-8 text-[12px]" :disabled="!dirty || input.busy || !input.available" @click="apply">
            <Loader2 v-if="input.busy" :size="13" class="animate-spin" />
            {{ t("input.apply") }}
          </button>
          <button class="btn !h-8 text-[12px]" :disabled="!dirty" @click="syncDraft">
            <RotateCcw :size="13" />
            {{ t("input.revert") }}
          </button>
        </div>
        <div v-if="dirty" class="text-[11px] text-warn mt-2">{{ t("input.unsaved") }}</div>
      </div>
    </div>

    <div class="col-span-12 lg:col-span-7">
      <div class="card p-5 h-full">
        <div class="flex items-center justify-between mb-1 flex-wrap gap-2">
          <div class="section-title">{{ t("input.polling") }}</div>
          <div class="flex items-center gap-2">
            <Segmented v-model="windowMs" :options="windows" />
            <label class="flex items-center gap-1.5 text-[11px] text-muted cursor-pointer">
              <input v-model="auto" type="checkbox" class="accent-[var(--color-accent)]" />
              {{ t("input.autoRefresh") }}
            </label>
            <button class="icon-btn" :title="t('common.refresh')" :disabled="polling" @click="poll">
              <RefreshCw :size="13" :class="polling ? 'animate-spin' : ''" />
            </button>
            <button class="icon-btn" :title="t('input.resetStats')" @click="input.resetStats()">
              <Trash2 :size="13" />
            </button>
          </div>
        </div>
        <div class="text-[11px] text-dim mb-4 leading-relaxed">{{ t("input.pollingHint") }}</div>

        <div v-if="rows.length" class="space-y-2.5">
          <div v-for="row in rows" :key="row.transport" class="card-flat px-3 py-2.5">
            <div class="flex items-center gap-2 text-[11.5px]">
              <span class="chip !text-[9.5px] !px-1.5">{{ row.kindLabel }}</span>
              <span class="mono truncate">{{ row.device }}</span>
              <span class="ml-auto mono tabular-nums" :class="row.rate > 0 ? 'text-ok' : 'text-dim'">
                {{ row.rate > 0 ? row.rate.toFixed(0) + " Hz" : t("input.idle") }}
              </span>
              <span class="text-dim mono tabular-nums w-[62px] text-right">{{ row.reports }} {{ t("input.reports") }}</span>
            </div>
            <div class="h-[3px] rounded-full bg-white/[0.05] mt-2 overflow-hidden">
              <span
                class="block h-full rounded-full transition-[width] duration-300"
                style="background: linear-gradient(90deg, var(--color-accent), var(--color-accent-2))"
                :style="{ width: Math.max(2, (row.rate / maxRate) * 100) + '%' }"
              ></span>
            </div>
          </div>
        </div>
        <div v-else class="text-[11.5px] text-dim py-8 text-center leading-relaxed">
          <Timer :size="18" class="mx-auto mb-2 opacity-60" />
          {{ t("input.noPolling") }}
        </div>

        <div class="flex items-start gap-2 mt-4 pt-3 border-t border-white/[0.05] text-[10.5px] text-dim leading-relaxed">
          <Gauge :size="13" class="shrink-0 mt-px" />
          <span>{{ t("input.pollingNote") }}</span>
        </div>
      </div>
    </div>
  </div>
</template>
