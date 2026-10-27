import { defineStore } from "pinia";
import { ref } from "vue";

// Preview colors of the lighting editor. Hardware state lives in LightingStudio.
export const useRgbStore = defineStore("rgb", () => {
  const primary = ref("#ff4fa3");
  const secondary = ref("#a78bfa");

  function setColors(nextPrimary: string, nextSecondary?: string) {
    primary.value = nextPrimary;
    if (nextSecondary) secondary.value = nextSecondary;
  }

  return { primary, secondary, setColors };
});
