import { describe, expect, it, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";

vi.mock("@/services/guildChannelsService", () => ({
  guildChannelsService: {
    listTextChannels: vi.fn(),
    listAllChannels: vi.fn(),
  },
}));
vi.mock("@/services/discordRolesService", () => ({
  discordRolesService: { getAll: vi.fn() },
}));

import IdsListPickerField from "./IdsListPickerField.vue";
import { guildChannelsService } from "@/services/guildChannelsService";
import { discordRolesService } from "@/services/discordRolesService";

const listText = (guildChannelsService as any).listTextChannels;
const listAll = (guildChannelsService as any).listAllChannels;
const getAll = (discordRolesService as any).getAll;

beforeEach(() => vi.clearAllMocks());

function mountField(props: Record<string, unknown> = {}) {
  return mount(IdsListPickerField, {
    props: { modelValue: "", guildId: "g1", kind: "channel", ...props },
  });
}

describe("chargement des options par type de champ", () => {
  it("liste les salons texte avec le prefixe #", async () => {
    listText.mockResolvedValue([{ id: "c1", name: "general", kind: "text" }]);
    const wrapper = mountField();
    await flushPromises();

    expect(listText).toHaveBeenCalledWith("g1");
    const options = wrapper.findAll(".picker-select option").slice(1);
    expect(options.map((o) => o.text())).toEqual(["# general"]);
  });

  it("filtre voice/stage en mode channel-voice avec le prefixe haut-parleur", async () => {
    listAll.mockResolvedValue([
      { id: "t1", name: "text-only", kind: "text" },
      { id: "v1", name: "Lounge", kind: "voice" },
      { id: "s1", name: "Stage", kind: "stage" },
    ]);
    const wrapper = mountField({ kind: "channel-voice" });
    await flushPromises();

    expect(listAll).toHaveBeenCalledWith("g1");
    const options = wrapper.findAll(".picker-select option").slice(1);
    expect(options.map((o) => o.text())).toEqual(["🔊 Lounge", "🔊 Stage"]);
  });

  it("liste les roles tries par position avec couleur paddee", async () => {
    getAll.mockResolvedValue([
      { id: "r2", name: "Modo", color: 0x57f287, position: 3 },
      { id: "r1", name: "Membre", color: undefined, position: 1 },
    ]);
    const wrapper = mountField({ kind: "role" });
    await flushPromises();

    expect(getAll).toHaveBeenCalledWith("g1");
    const options = wrapper.findAll(".picker-select option").slice(1);
    expect(options.map((o) => o.text())).toEqual(["@Modo", "@Membre"]);
    expect(options[0].attributes("style")).toContain("#57f287");

    // Placeholder specifique aux roles.
    expect(wrapper.find(".picker-select option").text()).toContain("rôle");
  });

  it("sans guildId : aucune requete et selecteur desactive", async () => {
    const wrapper = mountField({ guildId: null });
    await flushPromises();
    expect(listText).not.toHaveBeenCalled();
    expect((wrapper.find(".picker-select").element as HTMLSelectElement).disabled).toBe(true);
  });

  it("affiche l'erreur du service", async () => {
    listText.mockRejectedValue(new Error("boom"));
    const wrapper = mountField();
    await flushPromises();
    expect(wrapper.find(".err").text()).toBe("boom");
  });

  it("recharge quand guildId ou kind change", async () => {
    listText.mockResolvedValue([]);
    getAll.mockResolvedValue([]);
    const wrapper = mountField({ kind: "channel" });
    await flushPromises();
    expect(listText).toHaveBeenCalledTimes(1);

    await wrapper.setProps({ guildId: "g2" });
    await flushPromises();
    expect(listText).toHaveBeenLastCalledWith("g2");

    await wrapper.setProps({ kind: "role" });
    await flushPromises();
    expect(getAll).toHaveBeenCalledWith("g2");
  });
});

describe("selection (chips)", () => {
  it("parse le modelValue sur les delimitateurs virgule/point-virgule/saut de ligne", async () => {
    listText.mockResolvedValue([
      { id: "c1", name: "A", kind: "text" },
      { id: "c2", name: "B", kind: "text" },
      { id: "c3", name: "C", kind: "text" },
    ]);
    const wrapper = mountField({ modelValue: " c1 , c2 ;\nc3 " });
    await flushPromises();

    expect(wrapper.findAll(".chip").length).toBe(3);
  });

  it("affiche le label reel des chips, avec couleur pour les roles", async () => {
    listText.mockResolvedValue([{ id: "c1", name: "A", kind: "text" }]);
    const wrapper = mountField({ modelValue: "c1\nzzz" });
    await flushPromises();

    const chips = wrapper.findAll(".chip");
    expect(chips[0].find(".chip-label").text()).toBe("# A");
    // id inconnu : fallback ID <id>.
    expect(chips[1].find(".chip-label").text()).toBe("ID zzz");

    getAll.mockResolvedValue([{ id: "r1", name: "Modo", color: 0x57f287, position: 1 }]);
    const roles = mountField({ kind: "role", modelValue: "r1" });
    await flushPromises();
    expect(roles.find(".chip").attributes("style")).toContain("#57f287");
  });

  it("exclut du selecteur les ids deja selectionnes et affiche 'Tout est ajoute'", async () => {
    listText.mockResolvedValue([{ id: "c1", name: "A", kind: "text" }]);
    const wrapper = mountField({ modelValue: "c1" });
    await flushPromises();

    expect(wrapper.findAll(".picker-select option").length).toBe(1); // placeholder seul
    expect((wrapper.find(".picker-select").element as HTMLSelectElement).disabled).toBe(true);
  });

  it("ajoute l'id choisi au modelValue puis reinitialise le selecteur", async () => {
    listText.mockResolvedValue([{ id: "c1", name: "A", kind: "text" }, { id: "c2", name: "B", kind: "text" }]);
    const wrapper = mountField({ modelValue: "c1" });
    await flushPromises();

    (await wrapper.find(".picker-select") as any).setValue("c2");
    expect(wrapper.emitted("update:modelValue")).toEqual([["c1,c2"]]);
  });

  it("retire une chip a la demande", async () => {
    listText.mockResolvedValue([{ id: "c1", name: "A", kind: "text" }, { id: "c2", name: "B", kind: "text" }]);
    const wrapper = mountField({ modelValue: "c1,c2" });
    await flushPromises();

    expect(wrapper.findAll(".chip").length).toBe(2);
    await wrapper.find(".chip-remove").trigger("click");
    expect(wrapper.emitted("update:modelValue")).toEqual([["c2"]]);
  });

  it("affiche le message vide quand rien n'est selectionne", async () => {
    listText.mockResolvedValue([]);
    const wrapper = mountField({ kind: "role" });
    await flushPromises();
    expect(wrapper.find(".empty").text()).toContain("rôle");
  });
});
