<script setup lang="ts">
import { useI18n } from "../lib/i18n";
import LogoMark from "./LogoMark.vue";

withDefaults(defineProps<{ progress?: number; step?: string; leaving?: boolean }>(), {
  progress: 0,
  step: "",
  leaving: false,
});

const { t } = useI18n();
</script>

<template>
  <div class="boot-screen" :class="{ 'boot-leave': leaving }" role="status" aria-live="polite">
    <div class="boot-aurora boot-aurora-a"></div>
    <div class="boot-aurora boot-aurora-b"></div>
    <div class="boot-grid"></div>

    <div class="boot-inner">
      <div class="boot-mark">
        <span class="boot-halo"></span>
        <span class="boot-halo boot-halo-late"></span>
        <LogoMark :size="76" :interactive="false" />
      </div>

      <div class="boot-word">UNIVERSAL DEVICE MANAGER</div>
      <div class="boot-tagline">{{ t("boot.tagline") }}</div>

      <div class="boot-track">
        <span class="boot-fill" :style="{ width: `${progress}%` }"></span>
      </div>

      <div class="boot-row">
        <span class="boot-dot"></span>
        <Transition name="fade" mode="out-in">
          <span :key="step" class="boot-step">{{ step }}</span>
        </Transition>
        <span class="boot-percent mono">{{ Math.round(progress) }}%</span>
      </div>
    </div>

    <div class="boot-foot">{{ t("boot.foot") }}</div>
  </div>
</template>

<style scoped>
.boot-screen {
  position: fixed;
  inset: 0;
  z-index: 200;
  display: grid;
  place-items: center;
  overflow: hidden;
  background: #08060d;
  transition: opacity 420ms var(--ease-out), transform 420ms var(--ease-out);
}

.boot-leave {
  opacity: 0;
  transform: scale(1.02);
  pointer-events: none;
}

.boot-aurora {
  position: absolute;
  width: 62vw;
  height: 62vw;
  border-radius: 999px;
  filter: blur(110px);
  opacity: 0.3;
  will-change: transform;
}

.boot-aurora-a {
  top: -24vw;
  left: 8vw;
  background: radial-gradient(circle, var(--color-accent) 0%, transparent 65%);
  animation: boot-float-a 9s var(--ease-in-out) infinite;
}

.boot-aurora-b {
  bottom: -30vw;
  right: 4vw;
  background: radial-gradient(circle, var(--color-accent-2) 0%, transparent 65%);
  animation: boot-float-b 11s var(--ease-in-out) infinite;
}

.boot-grid {
  position: absolute;
  inset: 0;
  background-image:
    linear-gradient(rgba(255, 255, 255, 0.035) 1px, transparent 1px),
    linear-gradient(90deg, rgba(255, 255, 255, 0.035) 1px, transparent 1px);
  background-size: 56px 56px;
  mask-image: radial-gradient(52% 46% at 50% 48%, #000 20%, transparent 78%);
  -webkit-mask-image: radial-gradient(52% 46% at 50% 48%, #000 20%, transparent 78%);
  animation: boot-grid-drift 14s linear infinite;
}

.boot-inner {
  position: relative;
  z-index: 10;
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding-bottom: 24px;
  animation: boot-enter 620ms var(--ease-out) both;
}

.boot-mark {
  position: relative;
  display: grid;
  place-items: center;
  margin-bottom: 26px;
}

.boot-halo {
  position: absolute;
  inset: -14px;
  border-radius: 26px;
  border: 1px solid color-mix(in srgb, var(--color-accent) 45%, transparent);
  animation: boot-halo 2.4s var(--ease-out) infinite;
}

.boot-halo-late {
  animation-delay: 1.2s;
}

.boot-word {
  font-family: var(--font-display, inherit);
  font-size: 13px;
  letter-spacing: 0.34em;
  text-indent: 0.34em;
  font-weight: 700;
  color: rgba(255, 255, 255, 0.92);
}

.boot-tagline {
  margin-top: 9px;
  font-size: 11.5px;
  color: var(--color-dim, #8b8797);
}

.boot-track {
  position: relative;
  width: 264px;
  height: 3px;
  margin-top: 34px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.08);
  overflow: hidden;
}

.boot-fill {
  position: absolute;
  inset: 0 auto 0 0;
  border-radius: inherit;
  background: linear-gradient(90deg, var(--color-accent), var(--color-accent-2));
  box-shadow: 0 0 16px color-mix(in srgb, var(--color-accent) 55%, transparent);
  transition: width 460ms var(--ease-out);
}

.boot-track::after {
  content: "";
  position: absolute;
  inset: 0;
  background: linear-gradient(100deg, transparent 30%, rgba(255, 255, 255, 0.35) 50%, transparent 70%);
  background-size: 220% 100%;
  animation: boot-sheen 1.5s linear infinite;
}

.boot-row {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 18px;
  margin-top: 14px;
  font-size: 11px;
  color: var(--color-muted, #b6b2c2);
}

.boot-step {
  display: inline-block;
}

.boot-percent {
  font-size: 10px;
  color: var(--color-dim, #8b8797);
  margin-left: 2px;
  font-variant-numeric: tabular-nums;
}

.boot-dot {
  width: 5px;
  height: 5px;
  border-radius: 999px;
  background: var(--color-accent);
  box-shadow: 0 0 10px color-mix(in srgb, var(--color-accent) 70%, transparent);
  animation: boot-blink 1.1s ease-in-out infinite;
}

.boot-foot {
  position: absolute;
  bottom: 30px;
  left: 0;
  right: 0;
  text-align: center;
  font-size: 10px;
  letter-spacing: 0.04em;
  color: var(--color-dim, #8b8797);
  opacity: 0.7;
}

@keyframes boot-enter {
  from { opacity: 0; transform: translateY(10px) scale(0.985); }
  to { opacity: 1; transform: translateY(0) scale(1); }
}

@keyframes boot-float-a {
  0%, 100% { transform: translate3d(0, 0, 0) scale(1); }
  50% { transform: translate3d(6vw, 4vw, 0) scale(1.08); }
}

@keyframes boot-float-b {
  0%, 100% { transform: translate3d(0, 0, 0) scale(1.04); }
  50% { transform: translate3d(-5vw, -3vw, 0) scale(0.96); }
}

@keyframes boot-grid-drift {
  from { background-position: 0 0, 0 0; }
  to { background-position: 56px 56px, 56px 56px; }
}

@keyframes boot-halo {
  0% { opacity: 0.7; transform: scale(0.96); }
  70%, 100% { opacity: 0; transform: scale(1.24); }
}

@keyframes boot-sheen {
  to { background-position: -120% 0; }
}

@keyframes boot-blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.35; }
}
</style>
