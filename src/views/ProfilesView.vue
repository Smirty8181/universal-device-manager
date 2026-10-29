<script setup lang="ts">
import { computed, ref } from "vue";
import {
  AppWindow, Box, Briefcase, CheckCircle2, Copy, Crosshair, Gamepad2, Layers, LoaderCircle,
  MonitorPlay, Pencil, Plus, Target, Trash2, Volume2, X,
} from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import EmptyState from "../components/EmptyState.vue";
import Modal from "../components/Modal.vue";
import Toggle from "../components/Toggle.vue";
import Slider from "../components/Slider.vue";
import ColorField from "../components/ColorField.vue";
import { useProfilesStore } from "../stores/profiles";
import { useAppStore } from "../stores/app";
import { useI18n } from "../lib/i18n";
import { formatDateTime } from "../lib/format";
import type { Profile } from "../lib/types";

const { t, locale } = useI18n();
const profiles = useProfilesStore();
const app = useAppStore();

const editOpen = ref(false);
const isNew = ref(false);
const draft = ref<Profile | null>(null);
const deleteTarget = ref<Profile | null>(null);
const appInput = ref("");
const switchingId = ref<string | null>(null);

const effects = ["static", "breathing", "rainbow", "wave", "reactive", "ripple", "gradient", "spectrum", "fire", "custom"];
const pollingOptions = [125, 250, 500, 1000, 2000, 4000, 8000];

const templateDefs = [
  { key: "gaming", i18n: "profiles.tplGaming", icon: Gamepad2, accent: "#ff4fa3", apps: [] as string[] },
  { key: "work", i18n: "profiles.tplWork", icon: Briefcase, accent: "#6ee7f9", apps: [] as string[] },
  { key: "streaming", i18n: "profiles.tplStreaming", icon: MonitorPlay, accent: "#a78bfa", apps: ["obs64.exe"] },
  { key: "minecraft", i18n: "profiles.tplMinecraft", icon: Box, accent: "#7dd3a7", apps: ["javaw.exe", "minecraft.exe"] },
  { key: "cs2", i18n: "profiles.tplCs2", icon: Crosshair, accent: "#f0a35e", apps: ["cs2.exe"] },
  { key: "valorant", i18n: "profiles.tplValorant", icon: Target, accent: "#ff6b6b", apps: ["valorant.exe"] },
  { key: "custom", i18n: "profiles.tplCustom", icon: Layers, accent: "#9a8cff", apps: [] as string[] },
];

const templates = computed(() => templateDefs.map((d) => ({ ...d, name: t(d.i18n) })));

function identityFor(p: Profile) {
  return templates.value.find((tpl) => p.name.startsWith(tpl.name)) ?? templates.value[templates.value.length - 1];
}

const batteryLightOptions = computed(() => [
  { value: "", label: t("common.none") },
  { value: "color", label: t("keyboard.modeColor") },
  { value: "indicator", label: t("keyboard.modeIndicator") },
  { value: "pulse", label: t("keyboard.modePulse") },
  { value: "eco", label: t("keyboard.modeEco") },
  { value: "offlow", label: t("keyboard.modeOffLow") },
]);

function createProfile(template?: { name: string; apps: string[] }) {
  const profile = profiles.emptyProfile(template?.name ?? t("profiles.create"));
  if (template) profile.apps = [...template.apps];
  draft.value = profile;
  isNew.value = true;
  editOpen.value = true;
}

function openEditor(profile: Profile) {
  draft.value = JSON.parse(JSON.stringify(profile)) as Profile;
  isNew.value = false;
  editOpen.value = true;
}

async function saveDraft() {
  if (!draft.value || !draft.value.name.trim()) return;
  await profiles.save(draft.value);
  editOpen.value = false;
  draft.value = null;
}

async function duplicate(profile: Profile) {
  const copy = JSON.parse(JSON.stringify(profile)) as Profile;
  copy.id = `profile-${Date.now()}`;
  copy.name = `${profile.name} (${t("profiles.copyOf")})`;
  copy.createdAt = Date.now();
  await profiles.save(copy);
}

async function removeProfile() {
  if (!deleteTarget.value) return;
  await profiles.remove(deleteTarget.value.id);
  deleteTarget.value = null;
}

function addApp() {
  if (!draft.value) return;
  const process = appInput.value.trim().toLowerCase().replace(/\s+/g, "");
  if (!process) return;
  if (!draft.value.apps.includes(process)) draft.value.apps.push(process);
  appInput.value = "";
}

function removeApp(name: string) {
  if (!draft.value) return;
  draft.value.apps = draft.value.apps.filter((x) => x !== name);
}

async function toggleAutoSwitch(value: boolean) {
  await app.patchSettings({ autoSwitchProfiles: value });
}

async function activateProfile(p: Profile) {
  if (switchingId.value || profiles.activeId === p.id) return;
  switchingId.value = p.id;
  try {
    await profiles.activate(p.id);
  } finally {
    window.setTimeout(() => {
      switchingId.value = null;
    }, 700);
  }
}
</script>

<template>
  <div>
    <PageHeader :title="t('profiles.title')" :subtitle="t('profiles.subtitle')" :overline="t('nav.profiles')">
      <template #actions>
        <button class="btn btn-primary" @click="createProfile()">
          <Plus :size="13" />
          {{ t("profiles.create") }}
        </button>
      </template>
    </PageHeader>

    <div class="grid grid-cols-12 gap-4">
      <div class="col-span-8">
        <EmptyState v-if="!profiles.profiles.length" :icon="Layers" :title="t('profiles.emptyTitle')" :body="t('profiles.emptyBody')" />
        <div v-else class="grid grid-cols-2 gap-4">
          <article
            v-for="(p, idx) in profiles.profiles"
            :key="p.id"
            class="profile-card panel relative overflow-hidden p-5 flex flex-col reveal"
            :class="{ 'is-active': profiles.activeId === p.id, 'is-switching': switchingId === p.id }"
            :style="{ '--id-accent': identityFor(p).accent, '--i': idx }"
          >
            <span class="identity-wash" aria-hidden="true"></span>
            <span v-if="switchingId === p.id" class="switch-sweep" aria-hidden="true"></span>
            <span v-if="profiles.activeId === p.id" class="active-hairline" aria-hidden="true"></span>

            <div class="flex items-start gap-3 relative">
              <span class="identity-tile shrink-0 w-10 h-10 rounded-xl grid place-items-center">
                <component :is="identityFor(p).icon" :size="17" />
              </span>
              <div class="min-w-0 flex-1">
                <div class="flex items-center gap-2">
                  <h3 class="text-[14px] font-semibold truncate">{{ p.name }}</h3>
                  <span v-if="profiles.activeId === p.id" class="chip chip-ok !h-5 !text-[9px] shrink-0 active-chip">
                    <CheckCircle2 :size="9" />
                    {{ t("profiles.active") }}
                  </span>
                </div>
                <div class="text-[10px] text-dim mt-1">{{ t("profiles.updated") }} {{ formatDateTime(p.updatedAt, locale) }}</div>
              </div>
              <span class="identity-label">{{ identityFor(p).name }}</span>
            </div>

            <div class="grid grid-cols-3 gap-2 mt-4 relative">
              <div class="profile-stat">
                <span class="label">{{ t("profiles.rgb") }}</span>
                <span class="value">
                  <span class="w-2 h-2 rounded-full shrink-0" :style="{ background: p.rgb.color }"></span>
                  <span class="truncate">{{ t("rgb.effects." + p.rgb.effect) }}</span>
                </span>
              </div>
              <div class="profile-stat">
                <span class="label">{{ t("profiles.mouse") }}</span>
                <span class="value mono !text-[11px]">{{ p.mouse.dpi }} DPI</span>
              </div>
              <div class="profile-stat">
                <span class="label">{{ t("profiles.audio") }}</span>
                <span class="value mono !text-[11px]">{{ p.audio.volume ?? "—" }}%</span>
              </div>
            </div>

            <div class="mt-3 flex flex-wrap gap-1 relative min-h-[20px] items-center">
              <span v-for="a in p.apps" :key="a" class="chip !h-5 mono !text-[9.5px]">
                <AppWindow :size="9" />
                {{ a }}
              </span>
              <span v-if="!p.apps.length" class="text-[10px] text-dim">{{ t("profiles.autoSwitch") }} — {{ t("common.none") }}</span>
            </div>

            <div class="flex items-center gap-1.5 mt-auto pt-4 relative">
              <button
                class="btn flex-1 !h-8 !text-[11px]"
                :class="profiles.activeId === p.id ? '' : 'btn-primary'"
                :disabled="profiles.activeId === p.id || Boolean(switchingId)"
                @click="activateProfile(p)"
              >
                <LoaderCircle v-if="switchingId === p.id" :size="12" class="animate-spin" />
                <CheckCircle2 v-else-if="profiles.activeId === p.id" :size="12" />
                {{ switchingId === p.id ? t("profiles.activating") : profiles.activeId === p.id ? t("profiles.active") : t("profiles.apply") }}
              </button>
              <button class="btn !h-8 !px-2" :title="t('common.edit')" @click="openEditor(p)"><Pencil :size="12" /></button>
              <button class="btn !h-8 !px-2" :title="t('profiles.duplicate')" @click="duplicate(p)"><Copy :size="12" /></button>
              <button class="btn !h-8 !px-2 text-dim hover:text-bad" :title="t('common.delete')" @click="deleteTarget = p"><Trash2 :size="12" /></button>
            </div>
          </article>
        </div>
      </div>

      <div class="col-span-4 space-y-4">
        <div class="panel p-5">
          <div class="flex items-center justify-between mb-2">
            <div class="section-title">{{ t("profiles.autoSwitch") }}</div>
            <span class="flex items-center gap-2">
              <span v-if="app.settings.autoSwitchProfiles" class="live-dot"></span>
              <Toggle :model-value="app.settings.autoSwitchProfiles" @update:model-value="toggleAutoSwitch" />
            </span>
          </div>
          <div class="text-[11px] text-dim leading-relaxed">{{ t("profiles.autoSwitchNote") }}</div>
          <div class="divider my-3"></div>
          <div class="text-[11px] text-muted leading-relaxed">{{ t("profiles.activateHint") }}</div>
        </div>

        <div class="panel p-5">
          <div class="section-title mb-3">{{ t("profiles.templates") }}</div>
          <div class="grid grid-cols-2 gap-1.5">
            <button
              v-for="tpl in templates"
              :key="tpl.name"
              class="template-chip group"
              :style="{ '--tpl-accent': tpl.accent }"
              @click="createProfile(tpl)"
            >
              <span class="w-6 h-6 rounded-md grid place-items-center shrink-0" :style="{ background: `color-mix(in srgb, ${tpl.accent} 14%, transparent)` }">
                <component :is="tpl.icon" :size="12" :style="{ color: tpl.accent }" />
              </span>
              <span class="truncate">{{ tpl.name }}</span>
              <Plus :size="11" class="ml-auto text-dim group-hover:text-text transition-colors" />
            </button>
          </div>
          <div class="text-[10.5px] text-dim mt-3 leading-relaxed">{{ t("profiles.emptyBody") }}</div>
        </div>

        <div class="panel p-5">
          <div class="flex items-center gap-2 mb-2">
            <Volume2 :size="13" class="text-dim" />
            <div class="text-[11.5px]">{{ t("profiles.audio") }}</div>
          </div>
          <div class="text-[10.5px] text-dim leading-relaxed">{{ t("headset.volumeNote") }}</div>
        </div>
      </div>
    </div>

    <Modal :open="editOpen" :title="isNew ? t('profiles.create') : t('profiles.title')" width="680px" @close="editOpen = false">
      <template v-if="draft">
        <label class="block text-[11px] text-muted mb-1.5">{{ t("common.name") }}</label>
        <input v-model="draft.name" class="input" :placeholder="t('profiles.namePlaceholder')" />

        <div class="text-[11px] text-muted mt-4 mb-1.5">{{ t("profiles.apps") }}</div>
        <div class="flex flex-wrap gap-1.5 mb-2">
          <span v-for="a in draft.apps" :key="a" class="chip !h-6 mono !text-[10px]">
            {{ a }}
            <button class="text-dim hover:text-bad" @click="removeApp(a)"><X :size="9" /></button>
          </span>
          <span v-if="!draft.apps.length" class="text-[10.5px] text-dim self-center">{{ t("common.none") }}</span>
        </div>
        <div class="flex gap-2">
          <input v-model="appInput" class="input flex-1 mono !text-[11.5px]" :placeholder="t('profiles.appPlaceholder')" @keydown.enter.prevent="addApp" />
          <button class="btn" @click="addApp"><Plus :size="12" /> {{ t("common.add") }}</button>
        </div>

        <div class="grid grid-cols-2 gap-5 mt-5">
          <div>
            <div class="section-title mb-2">{{ t("profiles.rgb") }}</div>
            <label class="block text-[10.5px] text-dim mb-1">{{ t("rgb.effect") }}</label>
            <select v-model="draft.rgb.effect" class="input !h-8 !text-[11.5px]">
              <option v-for="e in effects" :key="e" :value="e">{{ t("rgb.effects." + e) }}</option>
            </select>
            <div class="h-2"></div>
            <ColorField v-model="draft.rgb.color" :label="t('rgb.hex')" />
            <div class="h-2"></div>
            <ColorField v-model="draft.rgb.secondary" :label="t('rgb.secondary')" />
            <div class="h-2"></div>
            <div class="text-[10.5px] text-dim mb-1">{{ t("rgb.brightness") }}</div>
            <Slider v-model="draft.rgb.brightness" :min="1" :max="100" unit="%" />
            <div class="h-2"></div>
            <div class="text-[10.5px] text-dim mb-1">{{ t("rgb.speed") }}</div>
            <Slider v-model="draft.rgb.speed" :min="0" :max="100" unit="%" />
          </div>

          <div>
            <div class="section-title mb-2">{{ t("profiles.mouse") }}</div>
            <label class="block text-[10.5px] text-dim mb-1">{{ t("mouse.dpi") }}</label>
            <input v-model.number="draft.mouse.dpi" type="number" min="100" max="26000" class="input !h-8 mono !text-[11.5px]" />
            <div class="grid grid-cols-2 gap-2 mt-2">
              <div>
                <label class="block text-[10.5px] text-dim mb-1">{{ t("mouse.polling") }}</label>
                <select v-model.number="draft.mouse.polling" class="input !h-8 !text-[11.5px]">
                  <option v-for="p in pollingOptions" :key="p" :value="p">{{ p }} Hz</option>
                </select>
              </div>
              <div>
                <label class="block text-[10.5px] text-dim mb-1">{{ t("mouse.lod") }}</label>
                <select v-model.number="draft.mouse.lod" class="input !h-8 !text-[11.5px]">
                  <option :value="0.7">0.7 mm</option>
                  <option :value="1">1.0 mm</option>
                  <option :value="2">2.0 mm</option>
                  <option :value="3">3.0 mm</option>
                </select>
              </div>
            </div>
            <div class="text-[10.5px] text-dim mt-2 mb-1">{{ t("mouse.debounce") }}</div>
            <Slider v-model="draft.mouse.debounce" :min="0" :max="20" unit=" ms" />

            <div class="section-title mt-4 mb-2">{{ t("profiles.keyboard") }}</div>
            <div class="flex items-center justify-between">
              <span class="text-[11px]">{{ t("keyboard.gameMode") }}</span>
              <Toggle v-model="draft.keyboard.gameMode" />
            </div>
            <label class="block text-[10.5px] text-dim mt-2 mb-1">{{ t("keyboard.batteryModes") }}</label>
            <select
              class="input !h-8 !text-[11.5px]"
              :value="draft.keyboard.batteryLight ?? ''"
              @change="draft.keyboard.batteryLight = ($event.target as HTMLSelectElement).value || null"
            >
              <option v-for="o in batteryLightOptions" :key="o.value" :value="o.value">{{ o.label }}</option>
            </select>
          </div>
        </div>

        <div class="section-title mt-5 mb-2">{{ t("profiles.audio") }}</div>
        <div class="grid grid-cols-2 gap-5">
          <div>
            <div class="text-[10.5px] text-dim mb-1">{{ t("headset.volume") }}</div>
            <Slider v-model="draft.audio.volume" :min="0" :max="100" unit="%" />
            <div class="flex items-center justify-between mt-2">
              <span class="text-[11px]">{{ t("audio.mute") }}</span>
              <Toggle v-model="draft.audio.muted" />
            </div>
          </div>
          <div>
            <div class="text-[10.5px] text-dim mb-1">{{ t("headset.micGain") }}</div>
            <Slider v-model="draft.audio.micGain" :min="0" :max="100" unit="%" />
          </div>
        </div>

        <div class="text-[10.5px] text-dim mt-4 leading-relaxed">{{ t("keyboard.lightingNote") }}</div>
      </template>
      <template #footer>
        <button class="btn" @click="editOpen = false">{{ t("common.cancel") }}</button>
        <button class="btn btn-primary" :disabled="!draft?.name.trim()" @click="saveDraft">{{ t("common.save") }}</button>
      </template>
    </Modal>

    <Modal :open="Boolean(deleteTarget)" :title="t('profiles.deleteConfirm')" width="380px" @close="deleteTarget = null">
      <div class="text-[12px] text-muted">{{ deleteTarget?.name }}</div>
      <template #footer>
        <button class="btn" @click="deleteTarget = null">{{ t("common.cancel") }}</button>
        <button class="btn btn-danger" @click="removeProfile">{{ t("common.delete") }}</button>
      </template>
    </Modal>
  </div>
</template>

<style scoped>
.profile-card {
  transition: border-color 200ms ease, transform 220ms var(--ease-out);
}
.profile-card:hover { transform: translateY(-2px); border-color: rgba(255, 255, 255, 0.12); }
.profile-card.is-active { border-color: color-mix(in srgb, var(--id-accent) 36%, transparent); }

.identity-wash {
  position: absolute;
  top: 0;
  right: 0;
  width: 190px;
  height: 150px;
  background: radial-gradient(circle at 85% 0%, color-mix(in srgb, var(--id-accent) 14%, transparent), transparent 68%);
  pointer-events: none;
}

.identity-tile {
  color: var(--id-accent);
  background: linear-gradient(135deg, color-mix(in srgb, var(--id-accent) 18%, transparent), transparent);
  border: 1px solid color-mix(in srgb, var(--id-accent) 24%, transparent);
}

.identity-label {
  font-size: 8.5px;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  font-weight: 600;
  color: color-mix(in srgb, var(--id-accent) 72%, #ffffff 12%);
  padding-top: 3px;
}

.profile-stat {
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
  padding: 7px 9px;
  border-radius: 10px;
  border: 1px solid rgba(255, 255, 255, 0.055);
  background: rgba(255, 255, 255, 0.018);
}
.profile-stat .label {
  font-size: 8.5px;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--color-dim);
}
.profile-stat .value {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11.5px;
  font-weight: 500;
  color: var(--color-text);
}

.active-hairline {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 2px;
  background: linear-gradient(90deg, var(--id-accent), transparent 88%);
}

.switch-sweep {
  position: absolute;
  inset: 0;
  transform: translateX(-110%);
  background: linear-gradient(105deg, transparent 32%, color-mix(in srgb, var(--id-accent) 20%, transparent) 50%, transparent 68%);
  animation: switch-sweep 900ms var(--ease-out) both;
  pointer-events: none;
}

.active-chip { animation: chip-pop 320ms var(--ease-spring) both; }

.template-chip {
  display: flex;
  align-items: center;
  gap: 7px;
  min-width: 0;
  height: 32px;
  padding: 0 8px;
  border-radius: 9px;
  border: 1px solid rgba(255, 255, 255, 0.07);
  background: rgba(255, 255, 255, 0.02);
  font-size: 11px;
  font-weight: 500;
  color: var(--color-muted);
  text-align: left;
  transition: border-color 150ms ease, color 150ms ease, background 150ms ease;
}
.template-chip:hover {
  color: var(--color-text);
  border-color: color-mix(in srgb, var(--tpl-accent) 32%, transparent);
  background: color-mix(in srgb, var(--tpl-accent) 6%, transparent);
}

@keyframes switch-sweep {
  to { transform: translateX(110%); }
}

@keyframes chip-pop {
  from { opacity: 0; transform: scale(0.85); }
  to { opacity: 1; transform: scale(1); }
}

@media (prefers-reduced-motion: reduce) {
  .profile-card { transition: none; }
  .switch-sweep { animation: none; opacity: 0; }
  .active-chip { animation: none; }
}
</style>
