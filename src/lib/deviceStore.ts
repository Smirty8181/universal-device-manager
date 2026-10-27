import { ref, watch, type Ref } from "vue";

export function useDeviceState<T extends object>(deviceId: string, defaults: T): Ref<T> {
  const key = `udm.device.${deviceId}`;
  let initial: T = defaults;
  try {
    const raw = localStorage.getItem(key);
    if (raw) initial = { ...defaults, ...(JSON.parse(raw) as Partial<T>) };
  } catch {}
  const state = ref(initial) as Ref<T>;
  watch(
    state,
    (value) => {
      try {
        localStorage.setItem(key, JSON.stringify(value));
      } catch {}
    },
    { deep: true }
  );
  return state;
}
