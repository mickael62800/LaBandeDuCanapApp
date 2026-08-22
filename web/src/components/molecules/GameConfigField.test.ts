import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";

import GameConfigField from "./GameConfigField.vue";
import type { TemplateField } from "@/services/nexusGamesService";

function champ(over: Partial<TemplateField> = {}): TemplateField {
  return { key: "MOTD", label: "Message d'accueil", type: "text", ...over } as TemplateField;
}

function monter(field: TemplateField, modelValue: string | undefined = "") {
  return mount(GameConfigField, { props: { field, modelValue } });
}

describe("structure de la cellule", () => {
  it("ne rend QU'UN seul element racine", () => {
    // Le formulaire est une grille CSS : un composant multi-racines verrait
    // chacun de ses noeuds devenir une cellule INDEPENDANTE, et le libelle se
    // retrouverait dans une colonne, son controle dans une autre.
    const wrapper = monter(champ({ type: "number", min: 1, max: 10 }));
    expect(wrapper.element.nodeType).toBe(Node.ELEMENT_NODE);
    expect(wrapper.element.tagName).toBe("LABEL");
  });

  it("garde libelle, controle, description et avertissement dans cette meme cellule", () => {
    const wrapper = monter(
      champ({ description: "Ce que fait le reglage", warning: "Ce qu'il casse" }),
    );
    const racine = wrapper.find("label.gcf");
    expect(racine.find(".gcf-label").exists()).toBe(true);
    expect(racine.find(".gcf-input").exists()).toBe(true);
    expect(racine.find(".gcf-note").text()).toBe("Ce que fait le reglage");
    expect(racine.find(".gcf-warning").text()).toContain("Ce qu'il casse");
  });

  it("porte une classe derivee du type, dont depend la mise en page", () => {
    // `.gcf--text` s'etale sur toute la largeur, `.gcf--boolean` se compacte
    // en ligne : perdre la classe casse la grille sans rien signaler.
    expect(monter(champ({ type: "text" })).classes()).toContain("gcf--text");
    expect(monter(champ({ type: "boolean" })).classes()).toContain("gcf--boolean");
  });

});

describe("collision entre le modificateur de type et les classes utilitaires", () => {
  /// Les classes utilitaires du composant, celles qui portent une regle de
  /// mise en page. Le modificateur de type ne doit JAMAIS en produire une.
  const UTILITAIRES = [
    "gcf-label",
    "gcf-input",
    "gcf-note",
    "gcf-warning",
    "gcf-slider",
    "gcf-range",
  ];

  it("le modificateur de type ne peut heurter aucune classe utilitaire", () => {
    // LE defaut : `gcf-${field.type}` donnait `gcf-number` pour un reglage
    // chiffre — le nom exact d'une classe utilitaire posee sur le petit champ
    // de saisie. Sa regle `width: 6.5rem` bridait donc la CELLULE entiere : le
    // curseur debordait, et description comme avertissement se retrouvaient
    // compresses sur une centaine de pixels, un mot par ligne.
    for (const type of ["text", "number", "boolean", "enum", "input", "note", "slider"]) {
      const classes = monter(champ({ type: type as TemplateField["type"] })).classes();
      for (const utilitaire of UTILITAIRES) {
        expect(classes, `type « ${type} »`).not.toContain(utilitaire);
      }
    }
  });

  it("la largeur reduite ne vise que la saisie a cote du curseur", () => {
    // Ciblee par sa PLACE (`.gcf-slider .gcf-input`) et non par une classe que
    // le modificateur pourrait reproduire.
    const wrapper = monter(champ({ type: "number", min: 1, max: 200 }), "20");
    const dansCurseur = wrapper.find(".gcf-slider .gcf-input");
    expect(dansCurseur.exists()).toBe(true);
    // La racine n'est pas, elle, un `.gcf-input`.
    expect(wrapper.classes()).not.toContain("gcf-input");
  });
});

describe("controle rendu selon le type", () => {
  it("rend une liste deroulante pour un enum, avec ses options", () => {
    const wrapper = monter(
      champ({ type: "enum", options: ["easy", "normal", "hard"] }),
      "normal",
    );
    const options = wrapper.findAll("option").map((o) => o.text());
    expect(options).toEqual(["easy", "normal", "hard"]);
  });

  it("rend un interrupteur pour un booleen, et non un champ texte", () => {
    // La page de detail rendait autrefois les booleens en texte : l'admin
    // devait y taper litteralement « true ».
    const wrapper = monter(champ({ type: "boolean" }), "true");
    expect(wrapper.find('input[type="text"]').exists()).toBe(false);
    expect(wrapper.findComponent({ name: "AppToggle" }).exists()).toBe(true);
  });

  it("rend un curseur quand les deux bornes sont connues", () => {
    const wrapper = monter(champ({ type: "number", min: 1, max: 32 }), "16");
    expect(wrapper.find('input[type="range"]').exists()).toBe(true);
    expect(wrapper.find('input[type="number"]').exists()).toBe(true);
  });

  it("retombe sur une saisie chiffree quand une borne manque", () => {
    // Sans echelle, un curseur n'a pas de position a montrer.
    const wrapper = monter(champ({ type: "number", min: 1000 }), "29999984");
    expect(wrapper.find('input[type="range"]').exists()).toBe(false);
    expect(wrapper.find('input[type="number"]').exists()).toBe(true);
  });
});

describe("correspondance avec les types acceptes par l'API", () => {
  /// `ConfigFieldType` cote Rust
  /// (platform-core/src/nexus/domain/entities/game/template.rs) n'accepte que
  /// ces quatre-la, et rejette le reste a la deserialisation — un type inconnu
  /// fait repondre 500 a TOUT l'endpoint des modeles, il ne peut donc pas
  /// arriver ici en silence.
  ///
  /// Le risque restant est l'inverse : un type AJOUTE cote Rust sans controle
  /// correspondant ici tomberait dans le `v-else` et s'afficherait en champ
  /// texte libre. L'admin taperait alors « true » a la main la ou un
  /// interrupteur etait attendu — c'est exactement ce que faisait la page de
  /// detail avant que ce composant n'existe.
  const TYPES_DE_L_API = ["text", "number", "enum", "boolean"] as const;

  it("rend un controle dedie pour chacun des quatre types", () => {
    const controle: Record<string, (w: ReturnType<typeof monter>) => boolean> = {
      text: (w) => w.find('input[type="text"]').exists(),
      number: (w) => w.find('input[type="number"]').exists(),
      enum: (w) => w.find("select").exists(),
      boolean: (w) => w.findComponent({ name: "AppToggle" }).exists(),
    };
    for (const type of TYPES_DE_L_API) {
      const f = champ({
        type: type as TemplateField["type"],
        ...(type === "enum" ? { options: ["a", "b"] } : {}),
      });
      expect(controle[type]!(monter(f)), `type « ${type} »`).toBe(true);
    }
  });

  it("ne rend un champ texte libre QUE pour le type texte", () => {
    // Le `v-else` est le filet de securite : qu'il attrape autre chose que
    // `text` signifie qu'un type n'a pas de controle.
    for (const type of TYPES_DE_L_API) {
      const f = champ({
        type: type as TemplateField["type"],
        ...(type === "enum" ? { options: ["a", "b"] } : {}),
      });
      const texteLibre = monter(f).find('input[type="text"]').exists();
      expect(texteLibre, `type « ${type} »`).toBe(type === "text");
    }
  });
});

describe("valeur", () => {
  it("lit un booleen quelle que soit la casse", () => {
    // L'EULA de Minecraft vaut « TRUE » en majuscules dans les schemas d'origine.
    for (const v of ["true", "TRUE", "True"]) {
      const wrapper = monter(champ({ type: "boolean" }), v);
      expect(wrapper.findComponent({ name: "AppToggle" }).props("modelValue"), v).toBe(true);
    }
    expect(
      monter(champ({ type: "boolean" }), "false")
        .findComponent({ name: "AppToggle" })
        .props("modelValue"),
    ).toBe(false);
  });

  it("remonte toujours une chaine, car c'est ce que la base stocke", async () => {
    const wrapper = monter(champ({ type: "boolean" }), "false");
    await wrapper.findComponent({ name: "AppToggle" }).vm.$emit("update:modelValue", true);
    expect(wrapper.emitted("update:modelValue")?.[0]).toEqual(["true"]);
  });

  it("accepte un pas decimal quand les bornes ou le defaut le sont", () => {
    // La moitie des taux Palworld sont decimaux : un pas entier rendait
    // « 1,5 » inatteignable.
    const wrapper = monter(champ({ type: "number", min: 0.1, max: 20, default: 1 }), "1");
    expect(wrapper.find('input[type="range"]').attributes("step")).toBe("0.1");
  });

  it("elargit le pas sur une grande amplitude", () => {
    // Un curseur de 600 a 86400 secondes ne se parcourt pas pixel par pixel.
    const wrapper = monter(champ({ type: "number", min: 600, max: 86400 }), "7200");
    expect(Number(wrapper.find('input[type="range"]').attributes("step"))).toBeGreaterThan(1);
  });

  it("affiche le libelle, ou la cle brute a defaut", () => {
    expect(monter(champ({ label: "" })).find(".gcf-label").text()).toBe("MOTD");
  });
});

/// Ce defaut-la ne casse aucun rendu : jsdom ne calcule pas de mise en page,
/// et le composant s'affiche « correctement » en test tout en etant disloque a
/// l'ecran. La feuille de style est donc la seule trace verifiable en CI.
describe("mise en page de la grille (non-regression)", () => {
  const bloc = (css: string, selecteur: string) => {
    const debut = css.indexOf(selecteur);
    expect(debut, `${selecteur} introuvable`).toBeGreaterThan(-1);
    return css.slice(debut, css.indexOf("}", debut));
  };

  it("la grille de la page de detail n'etire pas ses cellules", async () => {
    // Sans `align-items: start`, une rangee prend la hauteur de sa cellule la
    // plus haute — un avertissement long suffit — et les controles voisins
    // partent au fond de leur cellule, loin de leur libelle.
    const { readFileSync } = await import("node:fs");
    const css = readFileSync("src/styles/nexus-server-detail.css", "utf8");
    expect(bloc(css, ".sd-form {")).toContain("align-items: start");
  });

  it("la grille de la page de creation non plus", async () => {
    const { readFileSync } = await import("node:fs");
    const vue = readFileSync("src/components/pages/NexusServerCreatePage.vue", "utf8");
    expect(bloc(vue, ".nc-form {")).toContain("align-items: start");
  });

  it("le libelle ne reclame plus l'espace libre de la cellule", async () => {
    // `flex-grow: 1` etait l'autre moitie du defaut : c'est lui qui convertissait
    // la hauteur excedentaire de la cellule en un vide entre libelle et controle.
    const { readFileSync } = await import("node:fs");
    const vue = readFileSync("src/components/molecules/GameConfigField.vue", "utf8");
    expect(bloc(vue, ".gcf-label {")).not.toContain("flex-grow");
  });
});
