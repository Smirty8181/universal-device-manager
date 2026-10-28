<script setup lang="ts">
import { computed } from "vue";
import { getLayout, isEmphasized, keyLabel, isModifier, type LayoutSize, type LabelLang } from "../lib/keyboard";

const props = withDefaults(
  defineProps<{
    layout: LayoutSize;
    iso?: boolean;
    pressed?: string[];
    labelLang?: LabelLang;
    accent?: string;
    pressedColor?: string;
    heldColor?: string;
    modifierColor?: string;
    glow?: number;
    animationMs?: number;
    heatmap?: Record<string, number> | null;
    tracked?: "all" | "gaming" | "typing";
    scale?: number;
  }>(),
  {
    iso: false,
    pressed: () => [],
    labelLang: "auto",
    accent: "#ff4fa3",
    pressedColor: "#ff4fa3",
    heldColor: "#a78bfa",
    modifierColor: "#6ee7f9",
    glow: 70,
    animationMs: 160,
    heatmap: null,
    tracked: "all",
    scale: 1,
  }
);

const board = computed(() => getLayout(props.layout, props.iso));
const pressedSet = computed(() => new Set(props.pressed));
const maxHeat = computed(() => {
  if (!props.heatmap) return 1;
  return Math.max(1, ...Object.values(props.heatmap));
});

function heat(id: string): number {
  if (!props.heatmap) return 0;
  return (props.heatmap[id] ?? 0) / maxHeat.value;
}

function styleFor(key: { id: string; x: number; y: number; w: number; h: number }) {
  const layout = board.value;
  return {
    left: `${(key.x / layout.width) * 100}%`,
    top: `${(key.y / layout.height) * 100}%`,
    width: `${(key.w / layout.width) * 100}%`,
    height: `${(key.h / layout.height) * 100}%`,
  };
}

function keyVars(key: { id: string; x: number; y: number; w: number; h: number }) {
  const id = key.id;
  const active = pressedSet.value.has(id);
  const heatValue = heat(id);
  const accent = props.accent;
  const vars: Record<string, string> = {
    "--cap-accent": accent,
    "--cap-pressed": props.pressedColor,
    "--cap-held": props.heldColor,
    "--cap-modifier": props.modifierColor,
    "--cap-glow": String(props.glow / 100),
    "--cap-anim": `${Math.max(60, props.animationMs)}ms`,
    "--heat": heatValue ? String(0.1 + heatValue * 0.72) : "0",
  };
  if (!active && heatValue) vars["--heat-color"] = accent;
  return vars;
}

const scaleStyle = computed(() => ({
  width: `${Math.min(100, 100 * props.scale)}%`,
}));
</script>

<template>
  <div class="keyboard-shell" :style="scaleStyle">
    <div
      class="keyboard-board"
      :style="{
        aspectRatio: `${board.width} / ${board.height}`,
        '--cap-accent': accent,
      }"
    >
      <div class="board-grid">
        <div
          v-for="key in board.keys"
          :key="`${key.id}-${key.x}-${key.y}`"
          class="cap-wrap"
          :class="{
            'cap-dim': tracked !== 'all' && !isEmphasized(key.id, tracked),
          }"
          :style="styleFor(key)"
        >
          <div
            class="cap"
            :class="{
              'is-pressed': pressedSet.has(key.id),
              'is-modifier': isModifier(key.id),
              'has-heat': heatmap && heat(key.id) > 0,
            }"
            :style="keyVars(key)"
            :title="`${keyLabel(key.id, 'en')}\nVK ${key.vks[0].toString(16).toUpperCase()}`"
          >
            <span v-if="key.sub" class="cap-sub">{{ key.sub }}</span>
            <span class="cap-label">{{ keyLabel(key.id, labelLang) }}</span>
            <span v-if="heatmap && heat(key.id) > 0" class="cap-heat mono">{{ heatmap[key.id] }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.keyboard-shell {
  margin-inline: auto;
  transition: width 220ms var(--ease-out);
}

.keyboard-board {
  position: relative;
  width: 100%;
  container-type: inline-size;
  border-radius: 18px;
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.035), rgba(255, 255, 255, 0.008)),
    radial-gradient(120% 130% at 50% 0%, color-mix(in srgb, var(--cap-accent) 6%, transparent), transparent 62%);
  border: 1px solid rgba(255, 255, 255, 0.055);
}

.board-grid {
  position: absolute;
  inset: 8px;
}

.cap-wrap {
  position: absolute;
  padding: 2.5px;
  transition: opacity 220ms ease;
}

.cap-wrap.cap-dim {
  opacity: 0.28;
}

.cap {
  position: relative;
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.5px;
  border-radius: 7px;
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.055), rgba(255, 255, 255, 0.012)),
    #17121f;
  border: 1px solid rgba(255, 255, 255, 0.075);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.05),
    0 1.5px 0 rgba(0, 0, 0, 0.45);
  color: color-mix(in srgb, var(--color-text) 78%, transparent);
  overflow: hidden;
  user-select: none;
  transition:
    background-color 70ms ease,
    border-color 70ms ease,
    box-shadow 120ms ease,
    color 70ms ease,
    transform 90ms var(--ease-spring);
}

.cap.has-heat {
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.045), rgba(255, 255, 255, 0.012)),
    color-mix(in srgb, var(--cap-accent) calc(var(--heat) * 100%), #17121f);
  border-color: color-mix(in srgb, var(--cap-accent) calc(var(--heat) * 55%), rgba(255, 255, 255, 0.07));
}

.cap.is-pressed {
  color: #fff;
  border-color: color-mix(in srgb, var(--cap-held) 62%, transparent);
  background:
    linear-gradient(180deg, color-mix(in srgb, var(--cap-pressed) 40%, #17121f), color-mix(in srgb, var(--cap-held) 26%, #17121f));
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.15),
    0 0 calc(var(--cap-glow) * 26px) color-mix(in srgb, var(--cap-pressed) calc(var(--cap-glow) * 75%), transparent);
  transform: translateY(1px) scale(0.985);
  animation: cap-flash var(--cap-anim) ease-out both;
}

.cap.is-pressed.is-modifier {
  border-color: color-mix(in srgb, var(--cap-modifier) 70%, transparent);
  background:
    linear-gradient(180deg, color-mix(in srgb, var(--cap-modifier) 42%, #17121f), color-mix(in srgb, var(--cap-modifier) 22%, #17121f));
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.16),
    0 0 calc(var(--cap-glow) * 24px) color-mix(in srgb, var(--cap-modifier) calc(var(--cap-glow) * 70%), transparent);
}

@keyframes cap-flash {
  0% {
    background-color: color-mix(in srgb, var(--cap-pressed) 70%, #17121f);
    box-shadow: 0 0 calc(var(--cap-glow) * 34px) color-mix(in srgb, var(--cap-pressed) 85%, transparent);
  }
  100% {
    background-color: transparent;
  }
}

.cap-label {
  font-family: var(--font-display);
  font-size: clamp(7px, 1.6cqw, 15px);
  font-weight: 600;
  line-height: 1.05;
  letter-spacing: 0.01em;
  text-align: center;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 96%;
}

.cap-sub {
  position: absolute;
  top: 3px;
  right: 4px;
  font-size: clamp(5.5px, 1.06cqw, 10px);
  color: color-mix(in srgb, var(--color-text) 45%, transparent);
  line-height: 1;
}

.cap-heat {
  position: absolute;
  bottom: 2px;
  right: 4px;
  font-size: clamp(5.5px, 1cqw, 9px);
  color: color-mix(in srgb, var(--color-text) 55%, transparent);
  line-height: 1;
}

.is-pressed .cap-sub,
.is-pressed .cap-heat {
  color: rgba(255, 255, 255, 0.72);
}
</style>
