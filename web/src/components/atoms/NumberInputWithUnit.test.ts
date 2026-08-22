import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import NumberInputWithUnit from "./NumberInputWithUnit.vue";

describe("NumberInputWithUnit — mode nombre classique", () => {
  it("rend un input + boutons -/+ et emet la valeur saisie telle quelle", async () => {
    const wrapper = mount(NumberInputWithUnit, { props: { modelValue: "3" } });
    expect(wrapper.find(".num-input").exists()).toBe(true);
    await wrapper.find(".num-btn-plus").trigger("click");
    expect(wrapper.emitted("update:modelValue")).toEqual([["4"]]);
  });

  it("decremente en respectant le pas par defaut (1)", async () => {
    const wrapper = mount(NumberInputWithUnit, { props: { modelValue: "3" } });
    await wrapper.find(".num-btn-minus").trigger("click");
    expect(wrapper.emitted("update:modelValue")).toEqual([["2"]]);
  });

  it("honore le pas fourni et bloque les boutons aux bornes", async () => {
    const wrapper = mount(NumberInputWithUnit, { props: { modelValue: "5", min: 0, max: 10, step: 3 } });
    // 5 + 3 <= 10 : bouton plus actif ; 5 - 3 >= 0 : moins actif aussi.
    expect((wrapper.find(".num-btn-plus").element as HTMLButtonElement).disabled).toBe(false);
    await wrapper.find(".num-btn-plus").trigger("click");
    expect(wrapper.emitted("update:modelValue")).toEqual([["8"]]);

    const auMax = mount(NumberInputWithUnit, { props: { modelValue: "10", min: 0, max: 10 } });
    expect((auMax.find(".num-btn-plus").element as HTMLButtonElement).disabled).toBe(true);
    expect((auMax.find(".num-btn-minus").element as HTMLButtonElement).disabled).toBe(false);

    const auMin = mount(NumberInputWithUnit, { props: { modelValue: "0", min: 0, max: 10 } });
    expect((auMin.find(".num-btn-minus").element as HTMLButtonElement).disabled).toBe(true);
  });

  it("affiche l'avertissement hors borne avec les bornes concernees", () => {
    const wrapper = mount(NumberInputWithUnit, { props: { modelValue: "42", min: 1, max: 10 } });
    expect(wrapper.find(".num-warn").text()).toContain("min 1");
    expect(wrapper.find(".num-warn").text()).toContain("max 10");
    expect(wrapper.find(".num-input-row").classes()).toContain("out-of-range");

    const minOnly = mount(NumberInputWithUnit, { props: { modelValue: "2", min: 5 } });
    expect(minOnly.find(".num-warn").text()).toBe("Hors borne (min 5)");
  });

  it("affiche l'unite en texte quand le champ n'est pas une duree", () => {
    const wrapper = mount(NumberInputWithUnit, { props: { modelValue: "10", unit: "%" } });
    expect(wrapper.find(".num-unit").text()).toBe("%");
    // Sans unite du tout : ni selecteur, ni texte d'unite.
    const plain = mount(NumberInputWithUnit, { props: { modelValue: "1" } });
    expect(plain.find("select.num-unit-select").exists()).toBe(false);
    expect(plain.find(".num-unit").exists()).toBe(false);
  });

  it("propage disabled sur input et boutons", () => {
    const wrapper = mount(NumberInputWithUnit, { props: { modelValue: "1", disabled: true } });
    expect((wrapper.find(".num-input").element as HTMLInputElement).disabled).toBe(true);
    expect((wrapper.find(".num-btn-minus").element as HTMLButtonElement).disabled).toBe(true);
    expect(wrapper.find(".num-input-row").classes()).toContain("is-disabled");
  });

  it("propage id, placeholder et required sur l'input", () => {
    const wrapper = mount(NumberInputWithUnit, {
      props: { modelValue: "1", id: "champ-x", placeholder: "?", required: true },
    });
    expect(wrapper.find(".num-input").attributes("id")).toBe("champ-x");
    expect(wrapper.find(".num-input").attributes("placeholder")).toBe("?");
    expect((wrapper.find(".num-input").element as HTMLInputElement).required).toBe(true);
  });

  it("ne se plaint pas de borne quand la valeur est vide", () => {
    const wrapper = mount(NumberInputWithUnit, { props: { modelValue: "", min: 1 } });
    expect(wrapper.find(".num-warn").exists()).toBe(false);
    // Valeur nulle : les deux boutons restent actifs (pas de calcul possible).
    expect((wrapper.find(".num-btn-minus").element as HTMLButtonElement).disabled).toBe(false);
    expect((wrapper.find(".num-btn-plus").element as HTMLButtonElement).disabled).toBe(false);
  });

  it("ignore une valeur non numerique pour les bornes", () => {
    const wrapper = mount(NumberInputWithUnit, { props: { modelValue: "abc", min: 1 } });
    expect(wrapper.find(".num-warn").exists()).toBe(false);
  });
});

describe("NumberInputWithUnit — mode duree (unit en secondes)", () => {
  it("auto-choisit la plus grande unite qui tombe juste et re-exprime l'affichage", () => {
    // 3600 s = exactement 1 heure.
    const wrapper = mount(NumberInputWithUnit, { props: { modelValue: "3600", unit: "s" } });
    expect((wrapper.find("select.num-unit-select").element as HTMLSelectElement).value).toBe("heure");
    expect(wrapper.find(".num-input").attributes("value")).toBe("1");

    // 95 s : aucune unite plus grande ne tombe juste -> reste en sec.
    const odd = mount(NumberInputWithUnit, { props: { modelValue: "95", unit: "s" } });
    expect((odd.find("select.num-unit-select").element as HTMLSelectElement).value).toBe("sec");
  });

  it("n'offre que les unites >= a l'unite native (champ en minutes)", () => {
    const wrapper = mount(NumberInputWithUnit, { props: { modelValue: "90", unit: "min" } });
    const options = wrapper.findAll(".num-unit-select option");
    expect(options.map((o) => o.attributes("value"))).toEqual(["min", "heure", "jour", "semaine"]);
    // 90 min stockees -> affichees en minutes (pas de fraction possible).
    expect(wrapper.find(".num-input").attributes("value")).toBe("90");
  });

  it("re-convertit la saisie dans l'unite native et borne le resultat", async () => {
    const wrapper = mount(NumberInputWithUnit, { props: { modelValue: "1800", unit: "s", min: 60, max: 3600 } });
    // Affiche en minutes (1800 s / 60). Saisir 99 -> 5940 s > max 3600.
    expect(wrapper.find(".num-input").attributes("value")).toBe("30");
    const input = wrapper.find(".num-input");
    (input.element as HTMLInputElement).value = "99";
    await input.trigger("input");
    expect(wrapper.emitted("update:modelValue")).toEqual([["3600"]]);

    // Saisir 0 -> sous le min : bornee au minimum.
    const low = mount(NumberInputWithUnit, { props: { modelValue: "60", unit: "s", min: 50 } });
    expect(low.find(".num-input").attributes("value")).toBe("1"); // 60 s en minutes
    (low.find(".num-input").element as HTMLInputElement).value = "0";
    await low.find(".num-input").trigger("input");
    expect(low.emitted("update:modelValue")).toEqual([["50"]]);

    // Saisie vide -> valeur vide.
    const empty = mount(NumberInputWithUnit, { props: { modelValue: "60", unit: "s" } });
    (empty.find(".num-input").element as HTMLInputElement).value = "";
    await empty.find(".num-input").trigger("input");
    expect(empty.emitted("update:modelValue")).toEqual([[""]]);

    // Saisie non numerique : le navigateur (et happy-dom) la vide sur un input type=number.
    const bad = mount(NumberInputWithUnit, { props: { modelValue: "60", unit: "s" } });
    (bad.find(".num-input").element as HTMLInputElement).value = "abc";
    await bad.find(".num-input").trigger("input");
    expect(bad.emitted("update:modelValue")).toEqual([[""]]);
  });

  it("garde l'unite choisie par l'utilisateur quand la valeur change", async () => {
    const wrapper = mount(NumberInputWithUnit, { props: { modelValue: "3600", unit: "s" } });
    // L'utilisateur force "jour".
    await wrapper.find("select.num-unit-select").setValue("jour");
    expect((wrapper.find("select.num-unit-select").element as HTMLSelectElement).value).toBe("jour");

    // La valeur change : l'auto-pick ne doit PAS repasser en heure.
    await wrapper.setProps({ modelValue: "7200" });
    expect((wrapper.find("select.num-unit-select").element as HTMLSelectElement).value).toBe("jour");
  });

  it("affiche une valeur vide quand rien n'est saisi", () => {
    const wrapper = mount(NumberInputWithUnit, { props: { modelValue: "", unit: "s" } });
    expect(wrapper.find(".num-input").attributes("value")).toBe("");
  });

  it("propage disabled sur le selecteur d'unite en mode temps", () => {
    const wrapper = mount(NumberInputWithUnit, { props: { modelValue: "60", unit: "s", disabled: true } });
    expect((wrapper.find(".num-unit-select").element as HTMLSelectElement).disabled).toBe(true);
  });

  it("re-exprime la duree quand on change d'unite sans changer le stock", async () => {
    const wrapper = mount(NumberInputWithUnit, { props: { modelValue: "3600", unit: "s" } });
    expect(wrapper.find(".num-input").attributes("value")).toBe("1"); // 1 heure
    await wrapper.find("select.num-unit-select").setValue("semaine");
    // Meme duree stockee, re-exprimee en semaines : 3600 / 604800.
    expect(wrapper.find(".num-input").attributes("value")).toBe(String(Number((3600 / 604800).toFixed(4))));
  });

  it("accepte les variantes d'orthographe de l'unite native", () => {
    for (const u of ["sec", "secs", "seconde", "secondes"]) {
      const wrapper = mount(NumberInputWithUnit, { props: { modelValue: "60", unit: u } });
      expect(wrapper.find("select.num-unit-select").exists()).toBe(true);
    }
  });
});
