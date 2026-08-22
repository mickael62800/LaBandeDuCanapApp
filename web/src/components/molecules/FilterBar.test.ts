import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import FilterBar from "./FilterBar.vue";

const FILTERS = [
  { modelValue: "", options: [{ value: "all", label: "Tous" }, { value: "open", label: "Ouverts" }] },
  { modelValue: "x", options: [{ value: "a", label: "A" }, { value: "b", label: "B" }] },
];

describe("FilterBar", () => {
  it("rend un select par filtre, avec leurs options et valeurs", () => {
    const wrapper = mount(FilterBar, { props: { filters: FILTERS } });
    const selects = wrapper.findAll("select");
    expect(selects).toHaveLength(2);
    expect(wrapper.find(".filter-bar").exists()).toBe(true);
  });

  it("emets update:filter avec l'index du filtre change", async () => {
    const wrapper = mount(FilterBar, { props: { filters: FILTERS } });
    await wrapper.findAll("select")[1].setValue("b");
    expect(wrapper.emitted("update:filter")).toEqual([[1, "b"]]);
  });

  it("ne rend aucun select sans filtre", () => {
    const wrapper = mount(FilterBar, { props: { filters: [] } });
    expect(wrapper.findAll("select")).toHaveLength(0);
  });
});
