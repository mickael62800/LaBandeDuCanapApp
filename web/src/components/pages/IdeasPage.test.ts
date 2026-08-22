import { describe, expect, it } from "vitest";
import { shallowMount } from "@vue/test-utils";
import IdeasPage from "./IdeasPage.vue";
import IdeasListPanel from "../organisms/IdeasListPanel.vue";
import IdeaDetailPanel from "../organisms/IdeaDetailPanel.vue";

describe("IdeasPage", () => {
  it("affiche la liste au depart, sans detail", () => {
    const wrapper = shallowMount(IdeasPage);
    expect(wrapper.findComponent(IdeasListPanel).exists()).toBe(true);
    expect(wrapper.findComponent(IdeaDetailPanel).exists()).toBe(false);
  });

  it("passe au detail quand une idee est choisie dans la liste", async () => {
    const wrapper = shallowMount(IdeasPage);
    await wrapper.findComponent(IdeasListPanel).vm.$emit("select", "idea-1");
    expect(wrapper.findComponent(IdeasListPanel).exists()).toBe(false);
    // Le detail recoit bien l'id choisi.
    expect(wrapper.findComponent(IdeaDetailPanel).props().ideaId).toBe("idea-1");
  });

  it("retourne a la liste quand on demande le retour depuis le detail", async () => {
    const wrapper = shallowMount(IdeasPage);
    await wrapper.findComponent(IdeasListPanel).vm.$emit("select", "idea-1");
    expect(wrapper.findComponent(IdeaDetailPanel).exists()).toBe(true);
    await wrapper.findComponent(IdeaDetailPanel).vm.$emit("back");
    expect(wrapper.findComponent(IdeasListPanel).exists()).toBe(true);
  });
});
