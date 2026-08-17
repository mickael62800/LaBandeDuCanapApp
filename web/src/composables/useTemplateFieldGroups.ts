import { computed, type Ref } from "vue";
import type { TemplateField } from "@/services/nexusGamesService";

/// Nom de la section fourre-tout, pour les champs sans `group` dans le schéma.
export const SECTION_PAR_DEFAUT = "Réglages généraux";

/// Ordre d'affichage à l'intérieur d'une section.
///
/// Les interrupteurs se lisent d'un coup d'œil, les champs de saisie demandent
/// de s'arrêter. Les alterner obligeait l'œil à changer de mode à chaque ligne
/// — d'où l'impression de fouillis sur une section de vingt réglages.
///
/// Regrouper par nature met les interrupteurs en tête, en bloc compact, puis
/// les listes, puis les nombres, puis les textes libres qui prennent le plus
/// de place.
const ORDRE_TYPES: Record<string, number> = {
  boolean: 0,
  enum: 1,
  number: 2,
  text: 3,
};

export interface GroupeChamps {
  nom: string;
  champs: TemplateField[];
}

/**
 * Champs d'un `config_schema` regroupés par section, puis triés par nature à
 * l'intérieur de chacune. Un jeu peut avoir cinquante réglages : sans cela, le
 * formulaire est illisible.
 *
 * Partagé par la création et l'édition d'un serveur : les deux écrans lisent le
 * même schéma et doivent présenter les mêmes sections dans le même ordre.
 */
export function useTemplateFieldGroups(
  schema: Ref<TemplateField[] | undefined | null>,
) {
  return computed<GroupeChamps[]>(() => {
    const out: GroupeChamps[] = [];
    for (const f of schema.value ?? []) {
      const nom = f.group?.trim() || SECTION_PAR_DEFAUT;
      let g = out.find((x) => x.nom === nom);
      if (!g) {
        g = { nom, champs: [] };
        out.push(g);
      }
      g.champs.push(f);
    }

    // Tri STABLE : à nature égale, l'ordre du schéma est conservé. C'est lui
    // qui porte l'intention de celui qui a écrit les réglages — un tri
    // alphabétique séparerait `SPAWN_ANIMALS` de `SPAWN_MONSTERS`.
    for (const g of out) {
      g.champs = g.champs
        .map((f, i) => ({ f, i }))
        .sort((a, b) => {
          const ta = ORDRE_TYPES[a.f.type] ?? 9;
          const tb = ORDRE_TYPES[b.f.type] ?? 9;
          return ta !== tb ? ta - tb : a.i - b.i;
        })
        .map(({ f }) => f);
    }

    // Les sections nommées passent devant le fourre-tout : elles portent une
    // intention, lui n'est qu'un reste. Les jeux les plus anciens n'ont aucun
    // `group` dans leur schéma et tomberaient donc entièrement dedans.
    return out.sort((a, b) => {
      const da = a.nom === SECTION_PAR_DEFAUT ? 1 : 0;
      const db = b.nom === SECTION_PAR_DEFAUT ? 1 : 0;
      return da - db;
    });
  });
}
