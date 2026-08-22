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

import IdMultiplierMapField from "./IdMultiplierMapField.vue";
import { guildChannelsService } from "@/services/guildChannelsService";
import { discordRolesService } from "@/services/discordRolesService";

const listText = (guildChannelsService as any).listTextChannels;
const listAll = (guildChannelsService as any).listAllChannels;
const getAll = (discordRolesService as any).getAll;

beforeEach(() => {
  vi.clearAllMocks();
});

function mountField(props: Record<string, unknown> = {}) {
  return mount(IdMultiplierMapField, {
    props: { modelValue: "", guildId: "g1", kind: "channel", ...props },
  });
}

describe("chargement des options par type de champ", () => {
  it("appelle listTextChannels pour un salon texte et liste les salons avec leur icone", async () => {
    listText.mockResolvedValue([
      { id: "c1", name: "general", kind: "text" },
      { id: "v1", name: "Lounge", kind: "voice" },
      { id: "s1", name: "Stage", kind: "stage" },
      { id: "a1", name: "News", kind: "announcement" },
    ]);
    const wrapper = mountField();
    await flushPromises();

    expect(listText).toHaveBeenCalledWith("g1");
    const options = wrapper.findAll(".picker-select option").slice(1);
    expect(options.map((o) => o.text())).toEqual([
      "# general",
      "🔊 Lounge",
      "🎙️ Stage",
      "📢 News",
    ]);
  });

  it("appelle listAllChannels en mode channel-all et getAll pour les roles (tries par position)", async () => {
    const w1 = mountField({ kind: "channel-all" });
    await flushPromises();
    expect(listAll).toHaveBeenCalledWith("g1");

    getAll.mockResolvedValue([
      { id: "r2", name: "Modo", color: 0x57f287, position: 3 },
      { id: "r1", name: "Membre", color: undefined, position: 1 },
    ]);
    const w2 = mountField({ kind: "role" });
    await flushPromises();

    expect(getAll).toHaveBeenCalledWith("g1");
    // r2 (position 3) avant r1.
    const options = w2.findAll(".picker-select option").slice(1);
    expect(options.map((o) => o.text())).toEqual(["@Modo", "@Membre"]);
    // Couleur hexadecimale paddee a 6 chiffres.
    expect(options[0].attributes("style")).toContain("#57f287");
    expect(options[1].attributes("style") ?? "").not.toContain("color");

    const placeholder = w2.find(".picker-select option").text();
    expect(placeholder).toContain("rôle"); // "— Choisir un rôle —"
  });

  it("sans guildId : aucune requete, selecteur desactive et message vide", async () => {
    listText.mockReset().mockResolvedValue([]);
    const wrapper = mountField({ guildId: null });
    await flushPromises();
    expect(listText).not.toHaveBeenCalled();
    expect((wrapper.find(".picker-select").element as HTMLSelectElement).disabled).toBe(true);
  });

  it("affiche l'erreur du service quand le chargement echoue", async () => {
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

describe("parsing du modelValue (id:valeur par ligne)", () => {
  it("ignore les lignes invalides et deduplique via usedIds", async () => {
    listText.mockResolvedValue([{ id: "c1", name: "A", kind: "text" }, { id: "c2", name: "B", kind: "text" }]);
    const wrapper = mountField({ modelValue: "c1:0.5\nbadline\nc2:x\n c3 : 2 \n" });
    await flushPromises();

    // c3 inconnu mais valeur valide -> garde (label fallback). "c2:x" rejetee (NaN),
    // tandis qu'une ligne sans ":" devient {id: ligne, value: 0} car Number("") === 0.
    const entries = wrapper.findAll(".entry");
    expect(entries.length).toBe(3);
    expect(wrapper.find(".entry-label").text()).toBe("# A");
  });

  it("affiche le label reel quand l'option est connue, sinon ID <id>", async () => {
    listText.mockResolvedValue([{ id: "c1", name: "A", kind: "text" }]);
    const wrapper = mountField({ modelValue: "c1:2\nzzz:3" });
    await flushPromises();

    const labels = wrapper.findAll(".entry-label");
    expect(labels[0].text()).toBe("# A");
    expect(labels[1].text()).toBe("ID zzz");
  });

  it("exclut du selecteur les ids deja utilises", async () => {
    listText.mockResolvedValue([{ id: "c1", name: "A", kind: "text" }, { id: "c2", name: "B", kind: "text" }]);
    const wrapper = mountField({ modelValue: "c1:1.5" });
    await flushPromises();

    const options = wrapper.findAll(".picker-select option").slice(1);
    expect(options.map((o) => o.attributes("value"))).toEqual(["c2"]);
  });
});

describe("ajout / retrait / mise a jour des entrees", () => {
  it("ajoute une entree avec la valeur choisie puis reinitialise le picker", async () => {
    listText.mockResolvedValue([{ id: "c1", name: "A", kind: "text" }]);
    const wrapper = mountField({ valueDefault: 2 });
    await flushPromises();

    // Valeur par defaut pre-remplie.
    expect((wrapper.find(".value-input").element as HTMLInputElement).placeholder).toBe("2");

    (await wrapper.find(".picker-select") as any).setValue("c1");
    const valueInput = wrapper.find(".value-input") as any;
    valueInput.element.value = "3";
    await valueInput.trigger("input");

    expect((wrapper.find(".btn-add").element as HTMLButtonElement).disabled).toBe(false);
    await wrapper.find(".btn-add").trigger("click");

    expect(wrapper.emitted("update:modelValue")).toEqual([["c1:3"]]);
    // Picker reinitialise.
    expect((wrapper.find(".picker-select").element as HTMLSelectElement).value).toBe("");
  });

  it("refuse d'ajouter sans id choisi", async () => {
    listText.mockResolvedValue([{ id: "c1", name: "A", kind: "text" }]);
    const wrapper = mountField();
    await flushPromises();
    expect((wrapper.find(".btn-add").element as HTMLButtonElement).disabled).toBe(true);
  });

  it("retire une entree a la demande", async () => {
    listText.mockResolvedValue([{ id: "c1", name: "A", kind: "text" }, { id: "c2", name: "B", kind: "text" }]);
    const wrapper = mountField({ modelValue: "c1:1\nc2:4" });
    await flushPromises();

    expect(wrapper.findAll(".entry").length).toBe(2);
    await wrapper.findAll(".btn-remove")[0].trigger("click");
    expect(wrapper.emitted("update:modelValue")).toEqual([["c2:4"]]);
  });

  it("met a jour la valeur d'une entree (et ignore les valeurs non finies)", async () => {
    listText.mockResolvedValue([{ id: "c1", name: "A", kind: "text" }, { id: "c2", name: "B", kind: "text" }]);
    const wrapper = mountField({ modelValue: "c1:1\nc2:4" });
    await flushPromises();

    const entryInput = wrapper.findAll(".entry-value")[0] as any;
    entryInput.element.value = "9.5";
    await entryInput.trigger("change");
    expect(wrapper.emitted("update:modelValue")).toEqual([["c1:9.5\nc2:4"]]);

    // Valeur non numerique : happy-dom vide le champ -> Number("") === 0 (fini) donc emis.
    entryInput.element.value = "abc";
    await entryInput.trigger("change");
    expect(wrapper.emitted("update:modelValue")).toEqual([["c1:9.5\nc2:4"], ["c1:0\nc2:4"]]);
  });

  it("affiche le message vide quand aucune entree", async () => {
    listText.mockResolvedValue([]);
    const wrapper = mountField({ kind: "role" });
    await flushPromises();
    expect(wrapper.find(".empty").text()).toContain("rôle");
  });

  it("propage les bornes et le pas sur l'input de valeur", async () => {
    listText.mockResolvedValue([{ id: "c1", name: "A", kind: "text" }]);
    const wrapper = mountField({ valueMin: 0.5, valueMax: 8, valueStep: 0.5 });
    await flushPromises();

    const input = wrapper.find(".value-input").attributes()!;
    expect(input.min).toBe("0.5");
    expect(input.max).toBe("8");
    expect(input.step).toBe("0.5");
  });
});
