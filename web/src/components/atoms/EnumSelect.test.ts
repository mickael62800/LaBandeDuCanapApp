import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import EnumSelect from "./EnumSelect.vue";

const OPTIONS = [
  { value: "a", label: "Alpha" },
  { value: "b", label: "Beta" },
];

describe("EnumSelect", () => {
  it("liste les options et preselectionne la valeur courante", async () => {
    const wrapper = mount(EnumSelect, { props: { modelValue: "a", options: OPTIONS } });
    const select = wrapper.find("select");
    expect(select.findAll("option")).toHaveLength(2);
    await select.setValue("b");
  });

  it("emets update:modelValue avec la nouvelle valeur au changement", async () => {
    const wrapper = mount(EnumSelect, { props: { modelValue: "a", options: OPTIONS } });
    await wrapper.find("select").setValue("b");
    expect(wrapper.emitted("update:modelValue")).toEqual([["b"]]);
  });

  it("affiche une option placeholder desactivee quand demandee", () => {
    const wrapper = mount(EnumSelect, { props: { modelValue: "", options: OPTIONS, placeholder: "Choisir…" } });
    const first = wrapper.find("option");
    expect(first.text()).toBe("Choisir…");
    expect((first.element as HTMLOptionElement).disabled).toBe(true);
  });

  it("propage l'id sur le select", () => {
    const wrapper = mount(EnumSelect, { props: { modelValue: "", options: OPTIONS, id: "mon-id" } });
    expect(wrapper.find("#mon-id").exists()).toBe(true);
  });
});
