<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { parseBoolConfig } from "@/utils/configFlags";
import { botConfigService } from "@/services/botConfigService";
import { useToast } from "../../composables/useToast";
import { clampNumberValue } from "../../utils/clampNumber";
import type { BotDefinition, BotGuildConfig, ConfigField } from "../../types";
import { RouterLink } from "vue-router";
import AppToggle from "../atoms/AppToggle.vue";
import ConfigFieldRow from "../molecules/ConfigFieldRow.vue";

/**
 * Persistance de la configuration. Par defaut sentinel-api ; la plateforme
 * Nexus fournit la sienne, car c'est un autre backend (autre base, autre
 * passerelle). Le rendu du formulaire, lui, est identique : il est pilote par
 * `definition.config_schema`.
 */
export interface ConfigPersistence {
  set(guildId: string, botName: string, key: string, value: string): Promise<unknown>;
  remove(guildId: string, botName: string, key: string): Promise<unknown>;
}

const props = withDefaults(
  defineProps<{
    definition: BotDefinition;
    configs: BotGuildConfig[];
    guildId: string;
    persistence?: ConfigPersistence;
  }>(),
  { persistence: undefined },
);

const persist = computed<ConfigPersistence>(() => props.persistence ?? botConfigService);

const emit = defineEmits<{
  (e: "saved"): void;
}>();

const { success, error: showError } = useToast();

const formValues = ref<Record<string, string>>({});
/** Ce qui est REELLEMENT stocke en base (sert a distinguer « configure » de
 * « valeur par defaut » dans l'etat affiche sous chaque champ). */
const dbValues = ref<Record<string, string>>({});
/** Etat EFFECTIF : base + defauts du schema. Reference de comparaison. */
const savedValues = ref<Record<string, string>>({});
const saving = ref(false);
const successMessage = ref("");

const isWorker = computed(() => props.definition.bot_name.endsWith("-worker"));

// Modules dont la config DETAILLEE vit sur une page dediee (UX riche). Ici on
// n'affiche QUE l'interrupteur `enabled` pour eviter le doublon de reglages
// (le reste se configure sur la page dediee -> voir DEDICATED_PAGE).
const DEDICATED_CONFIG: Record<string, { label: string; path: string }> = {
  "welcome-bot": { label: "la page Bienvenue", path: "/welcome" },
  "security-bot": { label: "la page Sécurité", path: "/security" },
};
const dedicated = computed(() => DEDICATED_CONFIG[props.definition.bot_name] ?? null);

const configFields = computed<ConfigField[]>(() => {
  const schema = props.definition.config_schema;
  const all = Array.isArray(schema) ? schema : [];
  // Module a page dediee : on ne garde que l'interrupteur principal.
  if (dedicated.value) {
    return all.filter((f) => f.key === "enabled");
  }
  return all;
});

const booleanFields = computed(() => configFields.value.filter((f) => f.type === "boolean"));
const isAutomod = computed(() => props.definition.bot_name === "automod-bot");
// En mode IA texte exclusif, ces détecteurs restent visibles (pour expliquer
// ce qui est suspendu) mais ne sont plus modifiables. Les protections contre
// phishing et fichiers dangereux restent volontairement disponibles.
const AI_ONLY_LOCAL_FIELDS = new Set([
  "spam_detection_enabled", "caps_warning_enabled", "insult_detection_enabled",
  "link_detection_enabled", "emoji_spam_enabled", "mentions_enabled",
  "unicode_detection_enabled", "flood_review_mode", "caps_review_mode",
]);

/**
 * AutoMod comporte maintenant plusieurs niveaux de decision (detecteurs,
 * analyse IA/tension, protection et review). Les regrouper evite une grille
 * plate de plusieurs dizaines d'interrupteurs, sans changer le schema DB.
 */
function automodToggleGroup(key: string): string {
  if (key === "enabled") return "Module";
  if (/^(spam|caps|insult|link|phishing|emoji|mentions|suspicious_)/.test(key)) return "Détecteurs";
  if (/^(text_|channel_tension|context_)/.test(key)) return "Analyse IA et tension";
  if (/^(auto_protect|auto_delete|human_only|sanction_)/.test(key)) return "Réponse automatique";
  if (/^(review|vote|discussion)/.test(key)) return "Validation des modérateurs";
  return "Autres options";
}

const booleanSections = computed(() => {
  if (!isAutomod.value) return [{ title: "Fonctionnalités", fields: booleanFields.value }];
  const order = ["Module", "Détecteurs", "Analyse IA et tension", "Réponse automatique", "Validation des modérateurs", "Autres options"];
  return order
    .map((title) => ({ title, fields: booleanFields.value.filter((field) => automodToggleGroup(field.key) === title) }))
    .filter((section) => section.fields.length > 0);
});
/// Champs de scoring de la moderation (`score_weight_*`, `score_threshold_*`) :
/// sortis dans LEUR PROPRE section pour ne pas etre noyes parmi les dizaines
/// d'autres nombres d'automod (l'utilisateur les cherchait sans les trouver).
const isScoringKey = (key: string) => key.startsWith("score_");
const scoringFields = computed(() =>
  configFields.value.filter((f) => f.type === "number" && isScoringKey(f.key)),
);
const numberFields = computed(() =>
  configFields.value.filter((f) => f.type === "number" && !isScoringKey(f.key)),
);
const channelFields = computed(() => configFields.value.filter((f) => f.type === "channel"));
const categoryFields = computed(() => configFields.value.filter((f) => f.type === "category"));
const roleFields = computed(() => configFields.value.filter((f) => f.type === "role"));
const enumFields = computed(() => configFields.value.filter((f) => f.type === "enum"));
const voiceFields = computed(() => configFields.value.filter((f) => f.type === "voice"));
const listFields = computed(() =>
  configFields.value.filter((f) =>
    f.type === "channel_list" || f.type === "role_list" || f.type === "voice_list"
    || f.type === "channel_schedule_list",
  ),
);

function isMultilineKey(k: string): boolean {
  return k.endsWith("_message") || k.endsWith("_multipliers");
}
const longTextFields = computed(() =>
  configFields.value.filter((f) => f.type === "text" && isMultilineKey(f.key)),
);
const shortTextFields = computed(() =>
  configFields.value.filter((f) => f.type === "text" && !isMultilineKey(f.key)),
);

/**
 * Types deja repartis dans une section nommee. Sert au filet de securite
 * ci-dessous : tout ce qui n'est pas ici doit quand meme s'afficher.
 */
const TYPES_CLASSES = new Set([
  "boolean", "number", "enum", "channel", "voice", "category", "role",
  "channel_list", "role_list", "voice_list", "channel_schedule_list", "text",
]);

/**
 * Champs d'un type qu'aucune section ne connait.
 *
 * Sans ce filet, un type ajoute cote base disparaissait PUREMENT du
 * formulaire : le reglage existait, le bot le lisait, mais personne ne
 * pouvait le renseigner et rien ne le signalait. C'est ce qui est arrive aux
 * lobbies vocaux — sans eux, le module ne savait pas quel salon declenche la
 * creation d'un vocal temporaire.
 */
const unclassifiedFields = computed(() =>
  configFields.value.filter((f) => !TYPES_CLASSES.has(f.type)),
);

const visibleSections = computed(() => {
  const all = [
    { title: "Scoring — poids & seuils (modération)", fields: scoringFields.value, wide: true },
    { title: "Valeurs", fields: numberFields.value, wide: false },
    { title: "Choix", fields: enumFields.value, wide: false },
    { title: "Salons", fields: channelFields.value, wide: false },
    { title: "Salons vocaux", fields: voiceFields.value, wide: false },
    { title: "Categories", fields: categoryFields.value, wide: false },
    { title: "Roles", fields: roleFields.value, wide: false },
    { title: "Listes", fields: listFields.value, wide: true },
    { title: "Textes courts", fields: shortTextFields.value, wide: false },
    { title: "Textes longs", fields: longTextFields.value, wide: true },
    { title: "Autres reglages", fields: unclassifiedFields.value, wide: true },
  ];
  return all.filter((s) => s.fields.length > 0);
});

const allTogglesOn = computed(() =>
  booleanFields.value.length > 0
  && booleanFields.value.every((f) => parseBoolConfig(formValues.value[f.key])),
);

function enableAllToggles() {
  for (const field of booleanFields.value) formValues.value[field.key] = "true";
}
function disableAllToggles() {
  for (const field of booleanFields.value) formValues.value[field.key] = "false";
}

function isFieldModified(key: string): boolean {
  return (formValues.value[key] ?? "") !== (savedValues.value[key] ?? "");
}

/**
 * Une cle est "disabled" quand son `depends_on.key` n'a pas la valeur
 * `equals` requise. Ex : tous les champs avec `depends_on:{key:"enabled",
 * equals:"true"}` sont grises tant que `enabled` est OFF.
 *
 * Cas speciaux :
 *  - `equals:""` (chaine vide) signifie "le parent a une valeur non-zero
 *    et non-vide" — utile pour les champs numeriques ou 0 = desactive
 *    (ex: scan interval depend de timeout > 0).
 *
 * Recursivite : si le parent est lui-meme disabled (chaine de depends_on
 * type enabled -> sub_feature_enabled -> sub_feature_channel_id),
 * l'enfant l'est aussi. On remonte la chaine jusqu'a un noeud sans
 * depends_on. Detection des cycles via un Set de cles deja visitees.
 */
function isFieldDisabled(field: ConfigField, visited: Set<string> = new Set()): boolean {
  if (
    isAutomod.value
    && ["auto_warn_enabled", "auto_delete_enabled", "auto_mute_enabled", "auto_kick_enabled", "auto_ban_enabled"].includes(field.key)
    && !parseBoolConfig(formValues.value.auto_actions_selective_enabled)
  ) return true;
  if (
    isAutomod.value
    && parseBoolConfig(formValues.value.ai_only_enabled)
    && AI_ONLY_LOCAL_FIELDS.has(field.key)
  ) return true;
  const dep = field.depends_on as { key: string; equals: string } | undefined;
  if (!dep) return false;
  if (visited.has(field.key)) return false; // garde-fou cycle
  visited.add(field.key);

  // 1) check direct sur la valeur du parent
  const v = formValues.value[dep.key];
  let directlyDisabled: boolean;
  if (dep.equals === "true") directlyDisabled = !parseBoolConfig(v);
  else if (dep.equals === "false") directlyDisabled = parseBoolConfig(v);
  else if (dep.equals === "") directlyDisabled = v === undefined || v === "" || v === "0" || v === "false";
  else directlyDisabled = v !== dep.equals;

  if (directlyDisabled) return true;

  // 2) check transitif : si le parent est lui-meme disabled (cascade),
  // on l'est aussi.
  const parent = configFields.value.find((f) => f.key === dep.key);
  if (parent && isFieldDisabled(parent, visited)) return true;

  return false;
}

const hasChanges = computed(() =>
  configFields.value.some((f) => isFieldModified(f.key)),
);

const changesCount = computed(() =>
  configFields.value.filter((f) => isFieldModified(f.key)).length,
);

function fieldStatus(field: ConfigField): { text: string; source: "db" | "default" | "none" } {
  const dbValue = dbValues.value[field.key];

  if (isWorker.value) {
    if (dbValue !== undefined && dbValue !== "") {
      const unit = field.label.includes("heure") ? "heure(s)" : "minute(s)";
      return { text: `Valeur actuelle : ${dbValue} ${unit}`, source: "db" };
    }
    if (field.default !== undefined && field.default !== "") {
      const unit = field.label.includes("heure") ? "heure(s)" : "minute(s)";
      return { text: `Valeur par defaut : ${field.default} ${unit}`, source: "default" };
    }
    return { text: "Non configure", source: "none" };
  }

  const typeLabel =
    field.type === "channel" ? "ID du salon"
    : field.type === "role" ? "ID du role"
    : field.type === "number" ? "nombre"
    : field.type === "boolean" ? "true/false"
    : "texte";

  if (dbValue !== undefined && dbValue !== "") {
    return { text: `Configure : ${dbValue}`, source: "db" };
  }
  if (field.default !== undefined && field.default !== "") {
    return { text: `Par defaut : ${field.default} (${typeLabel})`, source: "default" };
  }
  return { text: `Non configure (${typeLabel})`, source: "none" };
}

function loadFormValues() {
  const stored: Record<string, string> = {};
  for (const cfg of props.configs.filter((c) => c.bot_name === props.definition.bot_name)) {
    stored[cfg.config_key] = cfg.config_value;
  }
  dbValues.value = { ...stored };

  // Le formulaire doit montrer l'etat EFFECTIF, pas seulement ce qui est
  // stocke : cote backend, une cle absente vaut le `default` du schema.
  //
  // Sans ce remplissage, un interrupteur dont le defaut est `true` s'affichait
  // ETEINT alors que le service le considerait ALLUME — et l'eteindre pour de
  // bon etait impossible : la valeur affichee etant deja « off », rien
  // n'apparaissait modifie, donc rien n'etait ecrit. C'est ce qui faisait
  // persister les annonces de level-up malgre un interrupteur visuellement
  // sur off.
  const effective = { ...stored };
  for (const field of configFields.value) {
    if (
      effective[field.key] === undefined
      && field.default !== undefined
      && field.default !== ""
    ) {
      effective[field.key] = field.default;
    }
  }
  savedValues.value = { ...effective };
  formValues.value = { ...effective };
}

function cancelChanges() {
  formValues.value = { ...savedValues.value };
}

async function save() {
  saving.value = true;
  successMessage.value = "";
  try {
    for (const field of configFields.value) {
      if (!isFieldModified(field.key)) continue;
      let value = formValues.value[field.key] ?? "";
      if (field.type === "number" && value) {
        value = clampNumberValue(value, field.min, field.max);
        formValues.value[field.key] = value;
      }
      if (value) {
        await persist.value.set(props.guildId, props.definition.bot_name, field.key, String(value));
      } else {
        await persist.value.remove(props.guildId, props.definition.bot_name, field.key);
      }
    }
    const count = changesCount.value;
    successMessage.value = `${count} parametre(s) enregistre(s)`;
    success(`${count} parametre(s) enregistre(s)`);
    emit("saved");
    setTimeout(() => (successMessage.value = ""), 3000);
  } catch (e) {
    console.error("Erreur sauvegarde:", e);
    showError("Erreur lors de la sauvegarde de la configuration");
  } finally {
    saving.value = false;
  }
}

// Recharge les valeurs quand le composant selectionne ou les configs changent
watch(() => [props.definition.bot_name, props.configs], loadFormValues, { immediate: true });
</script>

<template>
  <div class="config-form">
    <div class="config-form-header">
      <h2>{{ definition.display_name }}</h2>
    </div>

    <div v-if="configFields.length === 0" class="no-params">
      Ce composant n'a pas de parametres configurables par serveur.
    </div>

    <template v-else>
      <!-- Notice worker : reglages lus au demarrage uniquement -->
      <div v-if="isWorker" class="worker-notice">
        Les réglages des workers sont lus au démarrage — un changement prend
        effet au prochain redémarrage du worker.
      </div>

      <!-- Notice module a page dediee : ici juste l'interrupteur, le reste ailleurs -->
      <div v-if="dedicated" class="worker-notice">
        Ici tu actives/désactives le module. Toute la configuration détaillée
        se fait sur <RouterLink :to="dedicated.path" class="dedicated-link">{{ dedicated.label }}</RouterLink>.
      </div>

      <!-- Section toggles -->
      <div v-if="booleanFields.length > 0" class="toggles-section">
        <div class="section-title-row">
          <h3 class="section-title">{{ isAutomod ? 'Paramètres AutoMod' : 'Fonctionnalités' }}</h3>
          <button
            class="btn-toggle-all"
            @click="allTogglesOn ? disableAllToggles() : enableAllToggles()"
          >
            {{ allTogglesOn ? 'Tout desactiver' : 'Tout activer' }}
          </button>
        </div>
        <div v-for="section in booleanSections" :key="section.title" class="toggle-group">
          <h4 v-if="isAutomod" class="toggle-group-title">{{ section.title }}</h4>
          <div class="toggles-grid">
            <div
              v-for="field in section.fields"
              :key="field.key"
              class="toggle-card"
              :class="{ modified: isFieldModified(field.key), 'field-disabled': isFieldDisabled(field) }"
              :title="isFieldDisabled(field) ? 'Depend d\'une autre option desactivee' : undefined"
            >
              <div class="toggle-card-header">
                <span class="toggle-card-label" :title="field.label">{{ field.label }}</span>
                <span v-if="field.description" class="tooltip-wrap">
                  <span class="info-icon">i</span>
                  <span class="tooltip-text">{{ field.description }}</span>
                </span>
                <span v-if="isFieldModified(field.key)" class="modified-dot"></span>
              </div>
              <div class="toggle-card-control">
                <AppToggle
                  :model-value="formValues[field.key] === 'true' || formValues[field.key] === '1'"
                  @update:model-value="formValues[field.key] = $event ? 'true' : 'false'"
                />
                <span class="toggle-state" :class="{ active: formValues[field.key] === 'true' || formValues[field.key] === '1' }">
                  {{ formValues[field.key] === 'true' || formValues[field.key] === '1' ? 'ON' : 'OFF' }}
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Sections non-boolean -->
      <div class="sections-flow">
        <div
          v-for="section in visibleSections"
          :key="section.title"
          class="inputs-section"
          :class="[
            section.fields.length >= 4 || section.wide ? 'section-full' : 'section-auto',
            section.wide ? 'section-textareas' : '',
          ]"
          :style="
            !section.wide && section.fields.length < 4
              ? { flexGrow: section.fields.length }
              : undefined
          "
        >
          <h3 class="section-title">{{ section.title }}</h3>
          <div class="fields-grid-2col">
            <ConfigFieldRow
              v-for="field in section.fields"
              :key="field.key"
              :field="field"
              :model-value="formValues[field.key] ?? ''"
              :guild-id="guildId"
              :modified="isFieldModified(field.key)"
              :hint="fieldStatus(field).text"
              :hint-source="fieldStatus(field).source"
              :disabled="isFieldDisabled(field)"
              @update:model-value="formValues[field.key] = $event"
            />
          </div>
        </div>
      </div>

      <div class="form-actions">
        <button class="btn-save" :disabled="saving || !hasChanges" @click="save">
          {{ saving ? "Enregistrement..." : hasChanges ? `Enregistrer (${changesCount})` : "Aucune modification" }}
        </button>
        <button v-if="hasChanges" class="btn-cancel" @click="cancelChanges">Annuler</button>
        <span v-if="successMessage" class="success-msg">{{ successMessage }}</span>
      </div>
    </template>
  </div>
</template>

<style scoped src="../../styles/component-config-form.css"></style>
