import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import Segmented from "../src/components/Segmented.vue";
import Toggle from "../src/components/Toggle.vue";

describe("Toggle", () => {
  it("flips the bound value on click", async () => {
    const wrapper = mount(Toggle, { props: { modelValue: false } });
    await wrapper.get("button").trigger("click");
    expect(wrapper.emitted("update:modelValue")).toEqual([[true]]);
  });

  it("reflects the pressed state through aria", () => {
    const wrapper = mount(Toggle, { props: { modelValue: true } });
    expect(wrapper.get("button").attributes("aria-pressed")).toBe("true");
  });

  it("ignores clicks while disabled", async () => {
    const wrapper = mount(Toggle, { props: { modelValue: false, disabled: true } });
    await wrapper.get("button").trigger("click");
    expect(wrapper.emitted("update:modelValue")).toBeUndefined();
  });
});

describe("Segmented", () => {
  const options = [
    { value: "slow", label: "Slow" },
    { value: "fast", label: "Fast" },
  ];

  it("renders every option", () => {
    const wrapper = mount(Segmented, { props: { modelValue: "slow", options } });
    expect(wrapper.findAll("button")).toHaveLength(2);
  });

  it("emits the selected value", async () => {
    const wrapper = mount(Segmented, { props: { modelValue: "slow", options } });
    await wrapper.findAll("button")[1].trigger("click");
    expect(wrapper.emitted("update:modelValue")).toEqual([["fast"]]);
  });

  it("does not emit when disabled", async () => {
    const wrapper = mount(Segmented, {
      props: { modelValue: "slow", options, disabled: true },
    });
    await wrapper.findAll("button")[1].trigger("click");
    expect(wrapper.emitted("update:modelValue")).toBeUndefined();
  });
});
