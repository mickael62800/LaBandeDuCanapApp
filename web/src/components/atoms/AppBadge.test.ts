import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import AppBadge from "./AppBadge.vue";

describe("AppBadge", () => {
  it("affiche le libelle avec la variante par defaut", () => {
    const wrapper = mount(AppBadge, { props: { label: "En ligne" } });
    expect(wrapper.text()).toBe("En ligne");
    expect(wrapper.classes()).toContain("badge--default");
  });

  it("applique chaque variante demandee", async () => {
    for (const variant of ["info", "warn", "error", "danger", "warning", "success"] as const) {
      const wrapper = mount(AppBadge, { props: { label: "x", variant } });
      expect(wrapper.classes()).toContain(`badge--${variant}`);
    }
  });

  it("ne casse pas sans variante (prop optionnelle)", () => {
    const wrapper = mount(AppBadge);
    expect(wrapper.find(".badge").exists()).toBe(true);
  });
});
