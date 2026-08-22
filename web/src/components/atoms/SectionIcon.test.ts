import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import SectionIcon from "./SectionIcon.vue";

const ICONS = [
  "grid", "list", "alert-triangle", "shield", "award", "target", "zap",
  "gavel", "user-x", "ticket", "lightbulb", "mic", "cpu", "users",
  "trending-up", "clipboard", "activity", "layers", "settings",
  "dollar-sign", "edit-3", "clock", "paperclip", "check-circle",
  "bar-chart-2", "user-plus", "sliders", "user-check", "refresh-cw",
  "server", "heart", "send", "save",
];

describe("SectionIcon", () => {
  it.each(ICONS)("rend un svg non vide pour l'icone %s", (name) => {
    const wrapper = mount(SectionIcon, { props: { name } });
    expect(wrapper.find(".section-icon").exists()).toBe(true);
    // Chaque branche dessine au moins une forme.
    expect(wrapper.html()!.length).toBeGreaterThan("<svg".length + 100);
  });

  it("ne dessine aucune forme pour un nom inconnu", () => {
    const wrapper = mount(SectionIcon, { props: { name: "inexistant" } });
    expect(wrapper.find(".section-icon").exists()).toBe(true);
    // Le svg est la : mais aucun <path>/<rect> a l'interieur.
    expect(wrapper.html()!.match(/<(path|rect|line|circle|polygon|polyline)/g) ?? []).toHaveLength(0);
  });

  it("change de dessin quand le nom change", async () => {
    const wrapper = mount(SectionIcon, { props: { name: "grid" } });
    expect(wrapper.html()!.includes("<rect")).toBe(true);
    await wrapper.setProps({ name: "zap" });
    expect(wrapper.html()!.includes("<polygon")).toBe(true);
  });
});
