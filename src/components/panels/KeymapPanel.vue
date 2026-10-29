<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { ArrowRight, Ban, Keyboard, Loader2, MousePointer2, Plus, Radio, RotateCcw, ShieldAlert, Trash2 } from "lucide-vue-next";
import type { ButtonRemap, InputMap } from "../../lib/types";
import { useI18n } from "../../lib/i18n";
import { useInputStore } from "../../stores/input";
import { listen } from "../../lib/api";
import Toggle from "../Toggle.vue";

const props = defineProps<{ kind: "keyboard" | "mouse" | "all" }>();
const { t } = useI18n();
const input = useInputStore();

const showKeys = computed(() => props.kind === "keyboard" || props.kind === "all");
const showButtons = computed(() => props.kind === "mouse" || props.kind === "all");

const draft = ref<InputMap>({ enabled: false, keys: [], buttons: [] });
const dirty = computed(() => JSON.stringify(draft.value) !== JSON.stringify(input.status.map));

const names = computed(() => {
  const map = new Map<number, string>();
  for (const entry of input.catalog) map.set(entry.vk, entry.name);
  return map;
});

const buttons = computed(() => [
  { id: 0, label: t("input.lmb") },
  { id: 1, label: t("input.rmb") },
  { id: 2, label: t("input.mmb") },
  { id: 3, label: t("input.side1") },
  { id: 4, label: t("input.side2") },
]);

function keyLabel(vk: number): string {
  return names.value.get(vk) ?? `VK ${vk}`;
}
function buttonLabel(id: number): string {
  return buttons.value.find((b) => b.id === id)?.label ?? `#${id}`;
}

const catalogOptions = computed(() => input.catalog.map((entry) => ({ vk: entry.vk, name: entry.name })));

const ruleFilter = ref("");
const filteredCatalog = computed(() => {
  const needle = ruleFilter.value.trim().toLowerCase();
  if (!needle) return catalogOptions.value;
  return catalogOptions.value.filter(
    (entry) => entry.name.toLowerCase().includes(needle) || String(entry.vk) === needle
  );
});

function pick(value: string): number | null {
  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : null;
}

function pickButton(value: string): number | null {
  if (value === "") return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed >= 0 ? parsed : null;
}

const newKey = ref<number | null>(null);
const targetPick = ref("");
const disableNewKey = ref(false);
const buttonPick = ref("");
const newButtonAction = ref<ButtonRemap["action"]>("key");
const buttonKeyPick = ref("");
const buttonTargetPick = ref("");

const newTarget = computed(() => pick(targetPick.value));

const capturing = ref(false);

const DUAL_CODE_VK: Record<string, number> = {
  ControlLeft: 161,
  ControlRight: 163,
  ShiftLeft: 160,
  ShiftRight: 162,
  AltLeft: 164,
  AltRight: 165,
};

const CODE_VK: Record<string, number> = {
  Escape: 27,
  Tab: 9,
  Enter: 13,
  Space: 32,
  Backspace: 8,
  Delete: 46,
  Insert: 45,
  Home: 36,
  End: 35,
  PageUp: 33,
  PageDown: 34,
  ArrowLeft: 37,
  ArrowUp: 38,
  ArrowRight: 39,
  ArrowDown: 40,
  ShiftLeft: 16,
  ShiftRight: 16,
  ControlLeft: 17,
  ControlRight: 17,
  AltLeft: 18,
  AltRight: 18,
  CapsLock: 20,
  NumLock: 144,
  ScrollLock: 145,
  PrintScreen: 44,
  ContextMenu: 93,
  Minus: 189,
  Equal: 187,
  BracketLeft: 219,
  BracketRight: 221,
  Backslash: 220,
  Semicolon: 186,
  Quote: 222,
  Backquote: 192,
  Comma: 188,
  Period: 190,
  Slash: 191,
};

function vkFromCode(code: string): number | null {
  if (code in CODE_VK) return CODE_VK[code];
  if (/^Key[A-Z]$/.test(code) || /^Digit\d$/.test(code)) return code.charCodeAt(code.length - 1);
  const fn = /^F(\d{1,2})$/.exec(code);
  if (fn) {
    const n = Number(fn[1]);
    if (n >= 1 && n <= 24) return 111 + n;
  }
  const numpad = /^Numpad(\d)$/.exec(code);
  if (numpad) return 96 + Number(numpad[1]);
  return null;
}

function vkFromEvent(ev: KeyboardEvent): number | null {
  const dual = DUAL_CODE_VK[ev.code];
  if (dual) return dual;
  if (ev.keyCode > 0) return ev.keyCode;
  return vkFromCode(ev.code);
}

function onCaptureKey(ev: KeyboardEvent) {
  if (!capturing.value) return;
  ev.preventDefault();
  ev.stopPropagation();
  if (ev.repeat) return;
  const vk = vkFromEvent(ev);
  if (vk === null) return;
  newKey.value = vk;
  if (newTarget.value === vk) targetPick.value = "";
  capturing.value = false;
}

function startCapture() {
  capturing.value = true;
}

function stopCapture() {
  capturing.value = false;
}

async function applyMap(next: InputMap) {
  const ok = await input.commit(next);
  if (ok) syncDraft();
}

function addKeyRule() {
  const source = newKey.value;
  if (source === null) return;
  const target = disableNewKey.value ? null : newTarget.value;
  if (target === null && !disableNewKey.value) return;
  if (target !== null && target === source) return;
  const keys = draft.value.keys.filter((rule) => rule.vk !== source);
  keys.push({ vk: source, target });
  draft.value = { ...draft.value, keys };
  newKey.value = null;
  targetPick.value = "";
  disableNewKey.value = false;
}

function removeKeyRule(vk: number) {
  draft.value = { ...draft.value, keys: draft.value.keys.filter((rule) => rule.vk !== vk) };
}

function addTargetKeyRule() {
  const source = newKey.value;
  const target = newTarget.value;
  if (source === null || target === null) return;
  const keys = draft.value.keys.filter((rule) => rule.vk !== target);
  keys.push({ vk: target, target: source });
  draft.value = { ...draft.value, keys };
}

function addButtonRule() {
  const source = pickButton(buttonPick.value);
  if (source === null) return;
  const action = newButtonAction.value;
  const target =
    action === "key" ? pick(buttonKeyPick.value) : action === "button" ? pickButton(buttonTargetPick.value) : 0;
  if (action !== "disable" && (target === null || target === source)) return;
  const rules = draft.value.buttons.filter((rule) => rule.button !== source);
  rules.push({ button: source, action, target: target ?? 0 });
  draft.value = { ...draft.value, buttons: rules };
  buttonPick.value = "";
  buttonKeyPick.value = "";
  buttonTargetPick.value = "";
}

function removeButtonRule(button: number) {
  draft.value = { ...draft.value, buttons: draft.value.buttons.filter((rule) => rule.button !== button) };
}

function buttonRuleText(rule: ButtonRemap): string {
  if (rule.action === "disable") return t("input.disabled");
  if (rule.action === "button") return buttonLabel(rule.target);
  return keyLabel(rule.target);
}

function syncDraft() {
  draft.value = {
    enabled: input.status.map.enabled,
    keys: input.status.map.keys.map((rule) => ({ ...rule })),
    buttons: input.status.map.buttons.map((rule) => ({ ...rule })),
  };
}

function resetDraft() {
  syncDraft();
}

async function toggleEnabled(next: boolean) {
  draft.value = { ...draft.value, enabled: next };
  if (!dirty.value) return;
  await applyMap(draft.value);
}

const recent = computed(() => input.status.recent.slice(0, 6));
const totals = computed(() => {
  const s = input.status;
  return {
    keys: `${s.swallowedKeys} / ${s.seenKeys}`,
    buttons: `${s.swallowedButtons} / ${s.seenButtons}`,
  };
});

let unlisten: (() => void) | null = null;
let timer: number | null = null;

onMounted(async () => {
  await input.refresh();
  syncDraft();
  input.loadCatalog();
  unlisten = await listen("input:changed", () => {
    input.refresh().then(syncDraft);
  });
  timer = window.setInterval(() => input.refresh(), 2500);
});

onBeforeUnmount(() => {
  if (unlisten) unlisten();
  unlisten = null;
  if (timer !== null) window.clearInterval(timer);
  timer = null;
  window.removeEventListener("keydown", onCaptureKey, true);
});

watch(capturing, (active) => {
  if (active) window.addEventListener("keydown", onCaptureKey, true);
  else window.removeEventListener("keydown", onCaptureKey, true);
});
</script>

<template>
  <div class="grid grid-cols-12 gap-4">
    <div class="col-span-12 xl:col-span-7 space-y-4">
      <div class="card p-5">
        <div class="flex items-start justify-between gap-4 mb-4">
          <div>
            <div class="section-title">{{ t("input.title") }}</div>
            <div class="text-[11px] text-dim mt-1 max-w-[520px] leading-relaxed">{{ t("input.osLevelHint") }}</div>
          </div>
          <div class="flex items-center gap-2 shrink-0">
            <span class="chip" :class="input.status.engineActive ? 'chip-ok' : 'chip-bad'">
              <Radio :size="11" />
              {{ input.status.engineActive ? t("input.engineOn") : t("input.engineOff") }}
            </span>
            <Toggle :model-value="draft.enabled" :disabled="input.busy || !input.available" @update:model-value="toggleEnabled" />
          </div>
        </div>

        <div class="grid grid-cols-2 gap-2 mb-1">
          <div class="card-flat px-3 py-2.5">
            <div class="text-[10px] uppercase tracking-[0.1em] text-dim">{{ t("input.seenKeys") }}</div>
            <div class="mono text-[13px] mt-1 tabular-nums">{{ totals.keys }}</div>
          </div>
          <div class="card-flat px-3 py-2.5">
            <div class="text-[10px] uppercase tracking-[0.1em] text-dim">{{ t("input.seenButtons") }}</div>
            <div class="mono text-[13px] mt-1 tabular-nums">{{ totals.buttons }}</div>
          </div>
        </div>

        <div class="flex items-start gap-2 mt-3 text-[10.5px] text-dim leading-relaxed">
          <ShieldAlert :size="13" class="text-warn shrink-0 mt-px" />
          <span>{{ t("input.panicHint") }}</span>
        </div>
      </div>

      <div v-if="showKeys" class="card p-5">
        <div class="flex items-center justify-between mb-3">
          <div class="section-title">{{ t("input.keyRules") }}</div>
          <span class="chip mono !text-[10px]">{{ draft.keys.length }}</span>
        </div>

        <div class="grid grid-cols-[1fr_auto_1fr] gap-2 items-stretch mb-2">
          <button
            class="card-flat px-3 py-2.5 text-left hover:border-white/15 transition-colors"
            :class="capturing ? 'border-accent/40' : ''"
            @click="capturing ? stopCapture() : startCapture()"
          >
            <div class="text-[10px] uppercase tracking-[0.1em] text-dim">{{ t("input.source") }}</div>
            <div class="text-[13px] mt-1 flex items-center gap-2">
              <Keyboard :size="13" class="text-dim" />
              <span v-if="newKey !== null" class="mono">{{ keyLabel(newKey) }}</span>
              <span v-else-if="capturing" class="text-accent animate-pulse">{{ t("input.pressKey") }}</span>
              <span v-else class="text-dim">{{ t("input.captureKey") }}</span>
            </div>
          </button>
          <div class="grid place-items-center text-dim"><ArrowRight :size="14" /></div>
          <select v-model="targetPick" class="card-flat px-3 py-2.5 bg-transparent text-[12.5px] outline-none" :disabled="disableNewKey">
            <option value="" disabled>{{ t("input.pickTarget") }}</option>
            <option v-for="entry in filteredCatalog" :key="entry.vk" :value="String(entry.vk)">{{ entry.name }}</option>
          </select>
        </div>

        <div class="flex items-center gap-3 flex-wrap">
          <label class="flex items-center gap-2 text-[11.5px] text-muted cursor-pointer">
            <input v-model="disableNewKey" type="checkbox" class="accent-[var(--color-accent)]" />
            {{ t("input.disableKey") }}
          </label>
          <input
            v-model="ruleFilter"
            class="card-flat px-2.5 py-1.5 bg-transparent text-[11.5px] w-[180px] outline-none"
            :placeholder="t('input.searchKey')"
          />
          <button
            v-if="newKey !== null && newTarget !== null && !disableNewKey"
            class="btn !h-7 text-[11px]"
            @click="addTargetKeyRule"
          >
            {{ t("input.swap") }}
          </button>
          <button
            class="btn btn-primary !h-7 text-[11px] ml-auto"
            :disabled="newKey === null || (!disableNewKey && newTarget === null)"
            @click="addKeyRule"
          >
            <Plus :size="12" />
            {{ t("input.addRule") }}
          </button>
        </div>

        <div v-if="draft.keys.length" class="mt-4 space-y-1.5 max-h-[280px] overflow-y-auto scroll-region pr-1">
          <div v-for="rule in draft.keys" :key="rule.vk" class="flex items-center gap-2 card-flat px-3 py-2">
            <span class="mono text-[12px]">{{ keyLabel(rule.vk) }}</span>
            <ArrowRight :size="12" class="text-dim" />
            <span v-if="rule.target === null" class="chip chip-bad !text-[10px]"><Ban :size="10" /> {{ t("input.disabled") }}</span>
            <span v-else class="mono text-[12px]">{{ keyLabel(rule.target) }}</span>
            <button class="icon-btn ml-auto" :title="t('common.delete')" @click="removeKeyRule(rule.vk)">
              <Trash2 :size="12" />
            </button>
          </div>
        </div>
        <div v-else class="text-[11.5px] text-dim mt-4">{{ t("input.noKeyRules") }}</div>

        <div class="flex items-center gap-2 mt-4 pt-4 border-t border-white/[0.05]">
          <button class="btn btn-primary !h-8 text-[12px]" :disabled="!dirty || input.busy" @click="applyMap(draft)">
            <Loader2 v-if="input.busy" :size="13" class="animate-spin" />
            {{ t("input.apply") }}
          </button>
          <button class="btn !h-8 text-[12px]" :disabled="!dirty" @click="resetDraft">
            <RotateCcw :size="13" />
            {{ t("input.revert") }}
          </button>
          <span v-if="dirty" class="text-[11px] text-warn ml-auto">{{ t("input.unsaved") }}</span>
        </div>
      </div>

      <div v-if="showButtons" class="card p-5">
        <div class="flex items-center justify-between mb-3">
          <div class="section-title">{{ t("input.buttonRules") }}</div>
          <span class="chip mono !text-[10px]">{{ draft.buttons.length }}</span>
        </div>

        <div class="grid grid-cols-3 gap-2 mb-2">
          <select v-model="buttonPick" class="card-flat px-3 py-2 bg-transparent text-[12px] outline-none">
            <option value="" disabled>{{ t("input.source") }}</option>
            <option v-for="b in buttons" :key="b.id" :value="String(b.id)">{{ b.label }}</option>
          </select>
          <select v-model="newButtonAction" class="card-flat px-3 py-2 bg-transparent text-[12px] outline-none">
            <option value="key">{{ t("input.actKey") }}</option>
            <option value="button">{{ t("input.actButton") }}</option>
            <option value="disable">{{ t("input.disabled") }}</option>
          </select>
          <select
            v-if="newButtonAction === 'key'"
            v-model="buttonKeyPick"
            class="card-flat px-3 py-2 bg-transparent text-[12px] outline-none"
          >
            <option value="" disabled>{{ t("input.pickTarget") }}</option>
            <option v-for="entry in filteredCatalog" :key="entry.vk" :value="String(entry.vk)">{{ entry.name }}</option>
          </select>
          <select
            v-else-if="newButtonAction === 'button'"
            v-model="buttonTargetPick"
            class="card-flat px-3 py-2 bg-transparent text-[12px] outline-none"
          >
            <option value="" disabled>{{ t("input.pickTarget") }}</option>
            <option v-for="b in buttons" :key="b.id" :value="String(b.id)">{{ b.label }}</option>
          </select>
          <div v-else class="card-flat px-3 py-2 text-[12px] text-dim flex items-center gap-2">
            <Ban :size="12" /> {{ t("input.disabled") }}
          </div>
        </div>

        <input
          v-model="ruleFilter"
          class="card-flat px-2.5 py-1.5 bg-transparent text-[11.5px] w-[180px] outline-none mb-3"
          :placeholder="t('input.searchKey')"
        />
        <button
          class="btn btn-primary !h-7 text-[11px] block"
          :disabled="
            pickButton(buttonPick) === null ||
            (newButtonAction === 'key' && pick(buttonKeyPick) === null) ||
            (newButtonAction === 'button' && pickButton(buttonTargetPick) === null)
          "
          @click="addButtonRule"
        >
          <Plus :size="12" />
          {{ t("input.addRule") }}
        </button>

        <div v-if="draft.buttons.length" class="mt-4 space-y-1.5">
          <div v-for="rule in draft.buttons" :key="rule.button" class="flex items-center gap-2 card-flat px-3 py-2">
            <MousePointer2 :size="12" class="text-dim" />
            <span class="text-[12px]">{{ buttonLabel(rule.button) }}</span>
            <ArrowRight :size="12" class="text-dim" />
            <span class="text-[12px]">{{ buttonRuleText(rule) }}</span>
            <button class="icon-btn ml-auto" :title="t('common.delete')" @click="removeButtonRule(rule.button)">
              <Trash2 :size="12" />
            </button>
          </div>
        </div>
        <div v-else class="text-[11.5px] text-dim mt-4">{{ t("input.noButtonRules") }}</div>

        <div class="flex items-center gap-2 mt-4 pt-4 border-t border-white/[0.05]">
          <button class="btn btn-primary !h-8 text-[12px]" :disabled="!dirty || input.busy" @click="applyMap(draft)">
            <Loader2 v-if="input.busy" :size="13" class="animate-spin" />
            {{ t("input.apply") }}
          </button>
          <button class="btn !h-8 text-[12px]" :disabled="!dirty" @click="resetDraft">
            <RotateCcw :size="13" />
            {{ t("input.revert") }}
          </button>
          <span v-if="dirty" class="text-[11px] text-warn ml-auto">{{ t("input.unsaved") }}</span>
        </div>
      </div>
    </div>

    <div class="col-span-12 xl:col-span-5 space-y-4">
      <div class="card p-5">
        <div class="flex items-center justify-between mb-3">
          <div class="section-title">{{ t("input.intercepted") }}</div>
          <span class="chip !text-[10px]">{{ input.status.remapping ? t("input.armed") : t("input.idle") }}</span>
        </div>
        <div v-if="recent.length" class="space-y-1.5">
          <div v-for="(ev, index) in recent" :key="`${ev.at}-${index}`" class="flex items-center gap-2 text-[11.5px] card-flat px-3 py-2">
            <span class="mono text-dim">{{ ((ev.at % 60000) / 1000).toFixed(1) }}s</span>
            <span class="mono">{{ ev.kind === "key" ? keyLabel(ev.from) : buttonLabel(ev.from) }}</span>
            <ArrowRight :size="11" class="text-dim" />
            <span v-if="ev.to === null" class="text-bad">{{ t("input.disabled") }}</span>
            <span v-else class="mono">{{ ev.kind === "key" ? keyLabel(ev.to) : buttonLabel(ev.to) }}</span>
            <span class="text-dim ml-auto">{{ ev.action }}</span>
          </div>
        </div>
        <div v-else class="text-[11.5px] text-dim leading-relaxed">{{ t("input.noInterceptions") }}</div>
        <div class="text-[10.5px] text-dim mt-3 leading-relaxed">{{ t("input.interceptHint") }}</div>
      </div>

      <div class="card p-5">
        <div class="section-title mb-3">{{ t("input.rulesSummary") }}</div>
        <div class="text-[11.5px] text-dim leading-relaxed space-y-2">
          <p>{{ t("input.ruleNote1") }}</p>
          <p>{{ t("input.ruleNote2") }}</p>
          <p>{{ t("input.ruleNote3") }}</p>
        </div>
      </div>
    </div>
  </div>
</template>
