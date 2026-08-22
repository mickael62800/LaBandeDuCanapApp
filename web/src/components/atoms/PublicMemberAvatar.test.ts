import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import PublicMemberAvatar from "./PublicMemberAvatar.vue";

describe("PublicMemberAvatar", () => {
  it("affiche l'initiale du nom avec une couleur derivee de ce nom", () => {
    const wrapper = mount(PublicMemberAvatar, { props: { name: "Micka" } });
    expect(wrapper.text()).toBe("M");
    // La couleur est stable pour un meme nom (hash des codepoints).
    const style1 = (wrapper.element as HTMLElement).style.getPropertyValue("--c");
    const wrapper2 = mount(PublicMemberAvatar, { props: { name: "Micka" } });
    expect((wrapper2.element as HTMLElement).style.getPropertyValue("--c")).toBe(style1);
  });

  it("gagne la classe de taille demandee", async () => {
    const wrapper = mount(PublicMemberAvatar, { props: { name: "Ana", size: "sm" } });
    expect(wrapper.classes()).toContain("sm");
    await wrapper.setProps({ size: "lg" });
    expect(wrapper.classes()).toContain("lg");
  });

  it("propage le titre d'info-bulle quand fourni", () => {
    const wrapper = mount(PublicMemberAvatar, { props: { name: "Ana", title: "Membre depuis 2019" } });
    expect(wrapper.attributes("title")).toBe("Membre depuis 2019");
  });

  it("tombe sur '?' pour un nom vide", () => {
    const wrapper = mount(PublicMemberAvatar, { props: { name: "   " } });
    expect(wrapper.text()).toBe("?");
  });
});
