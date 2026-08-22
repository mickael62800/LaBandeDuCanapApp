<script setup lang="ts">
import AppButton from "../atoms/AppButton.vue";
import GameConfigField from "../molecules/GameConfigField.vue";
// Création d'un serveur de jeu — choix du jeu puis réglages.
//
// Le formulaire est ENTIÈREMENT piloté par le `config_schema` du template
// choisi. Ajouter une option à Minecraft ou Palworld se fait donc en base,
// sans toucher à ce fichier — c'est ce qui permettra d'ajouter de nouveaux
// jeux sans redéployer le front.
//
// Deux étapes délibérées : choisir le jeu, puis le configurer. Tout afficher
// d'un coup noierait l'utilisateur sous des dizaines de champs dont la moitié
// dépendent du jeu retenu.

import { computed, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useAuth } from "../../composables/useAuth";
import { useToast } from "../../composables/useToast";
import { nexusGamesService, type GameTemplate } from "@/services/nexusGamesService";
import { useTemplateFieldGroups } from "@/composables/useTemplateFieldGroups";
import { communityAdminService } from "@/services/communityAdminService";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import GameResourcesGuide from "../organisms/GameResourcesGuide.vue";

const router = useRouter();
const { selectedGuildId, selectedGuild } = useGuildSelector();
const { user } = useAuth();
const { success, error: showError } = useToast();

const templates = ref<GameTemplate[]>([]);
const loading = ref(false);
const errorMessage = ref("");

const chosen = ref<GameTemplate | null>(null);
const name = ref("");
const memoryMb = ref<number>(0);
const cpuLimit = ref<number>(2);
/// Date et heure d'ouverture (révélation de l'IP & démarrage programmé)
const openAt = ref("");
/// Date et heure de fermeture (utilisée pour le calendrier)
const closeAt = ref("");
/// Valeurs des champs du template, indexées par clé.
const values = ref<Record<string, string>>({});
const submitting = ref(false);

async function loadTemplates() {
  if (!selectedGuildId.value) return;
  loading.value = true;
  errorMessage.value = "";
  try {
    templates.value = await nexusGamesService.listTemplates(selectedGuildId.value);
  } catch (e) {
    errorMessage.value = e instanceof Error ? e.message : "Chargement impossible";
    templates.value = [];
  } finally {
    loading.value = false;
  }
}

/// Sélectionner un jeu pré-remplit tous ses champs avec les valeurs par
/// défaut : l'utilisateur n'a plus qu'à ajuster ce qui l'intéresse.
function choose(t: GameTemplate) {
  chosen.value = t;
  memoryMb.value = t.default_memory_mb;
  const initial: Record<string, string> = {};
  for (const f of t.config_schema ?? []) {
    initial[f.key] = f.default === undefined ? "" : String(f.default);
  }
  values.value = initial;
  if (!name.value) name.value = t.slug;

  // Pré-remplissage des dates : ouverture à demain 20h00, fermeture à 23h00
  const tomorrow = new Date();
  tomorrow.setDate(tomorrow.getDate() + 1);
  tomorrow.setHours(20, 0, 0, 0);
  openAt.value = tomorrow.toISOString().slice(0, 16);

  const end = new Date(tomorrow);
  end.setHours(23, 0, 0, 0);
  closeAt.value = end.toISOString().slice(0, 16);
}

/// Le nom devient celui du conteneur : on impose ce que la base accepte
/// (lettres, chiffres, espaces, tirets, underscores).
const nameError = computed(() => {
  const v = name.value.trim();
  if (!v) return "Donne un nom au serveur.";
  if (v.length > 64) return "64 caractères maximum.";
  if (!/^[a-zA-Z0-9 _-]+$/.test(v)) {
    return "Lettres, chiffres, espaces, tirets et underscores uniquement.";
  }
  return "";
});

const memoryError = computed(() => {
  const t = chosen.value;
  if (!t) return "";
  if (memoryMb.value < t.min_memory_mb) return `Minimum ${t.min_memory_mb} Mo pour ce jeu.`;
  if (memoryMb.value > t.max_memory_mb) return `Maximum ${t.max_memory_mb} Mo pour ce jeu.`;
  return "";
});

const dateError = computed(() => {
  if (!openAt.value) return "Indique la date et l'heure d'ouverture.";
  if (!closeAt.value) return "Indique la date et l'heure de fermeture.";
  const openDate = new Date(openAt.value);
  const closeDate = new Date(closeAt.value);
  if (isNaN(openDate.getTime())) return "Date d'ouverture invalide.";
  if (isNaN(closeDate.getTime())) return "Date de fermeture invalide.";
  if (closeDate <= openDate) return "La date de fermeture doit être postérieure à l'ouverture.";
  return "";
});

const canSubmit = computed(
  () =>
    !!chosen.value &&
    !nameError.value &&
    !memoryError.value &&
    !dateError.value &&
    !submitting.value,
);

async function submit() {
  if (!canSubmit.value || !selectedGuildId.value || !chosen.value) return;
  submitting.value = true;
  try {
    const openDate = new Date(openAt.value);
    const closeDate = new Date(closeAt.value);

    // Calcul du nombre de jours relatifs pour ip_reveal_days
    const now = new Date();
    const diffMs = openDate.getTime() - now.getTime();
    const days = Math.max(0, Math.round(diffMs / (1000 * 60 * 60 * 24)));

    const created = await nexusGamesService.create(selectedGuildId.value, {
      template_slug: chosen.value.slug,
      name: name.value.trim(),
      memory_mb: memoryMb.value,
      cpu_limit: cpuLimit.value,
      owner_user_id: user.value?.id ?? "",
      config: values.value,
      ip_reveal_days: days,
    });

    // La creation seule persiste un serveur a l'arret. C'est la programmation
    // qui publie `game_server_scheduled` vers nexus-bot : le bot cree alors
    // immediatement les salons Discord et le panneau d'inscription, tandis que
    // le conteneur attend l'heure d'ouverture.
    // La date de fermeture part avec la programmation : elle ne servait
    // jusqu'ici qu'au calendrier communautaire, alors que c'est elle qui
    // permet de distinguer « la soirée continue » de « c'est fini ».
    await nexusGamesService.schedule(
      selectedGuildId.value,
      created.id,
      openDate.toISOString(),
      closeDate.toISOString(),
    );

    // Remplissage automatique du calendrier communautaire avec l'événement
    try {
      await communityAdminService.createEvent(selectedGuildId.value, {
        title: `Session ${chosen.value.name} - ${name.value.trim()}`,
        description: `Serveur de jeu Nexus « ${name.value.trim()} ». Ouverture et accès au serveur à ${openDate.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}.`,
        game: chosen.value.name,
        starts_at: openDate.toISOString(),
        ends_at: closeDate.toISOString(),
        is_public: true,
      });
    } catch {
      // Si la création d'événement échoue (ex: permissions), le serveur est quand même créé
    }

    success(`Serveur « ${created.name} » programmé, salons Discord demandés et événement ajouté au calendrier !`);
    router.push(`/nexus/servers/${created.id}`);
  } catch (e) {
    showError(e instanceof Error ? e.message : "Création impossible");
  } finally {
    submitting.value = false;
  }
}

/// Champs regroupés par section, puis par nature à l'intérieur de chacune.
/// Même découpage que la page de détail : voir `useTemplateFieldGroups`.
const groupes = useTemplateFieldGroups(computed(() => chosen.value?.config_schema));

watch(selectedGuildId, loadTemplates, { immediate: true });
</script>

<template>
  <AdminPageShell
    title="Nouveau serveur de jeu"
    :subtitle="selectedGuild?.name ?? 'Aucun serveur sélectionné'"
  >
    <p v-if="!selectedGuildId" class="nc-hint">
      Sélectionne un serveur Discord pour créer un serveur de jeu.
    </p>

    <p v-else-if="errorMessage" class="nc-error">{{ errorMessage }}</p>

    <p v-else-if="loading" class="nc-hint">Chargement du catalogue…</p>

    <template v-else>
      <!-- Étape 1 : le jeu -->
      <h2 class="nc-step">1. Choisis le jeu</h2>
      <div class="nc-games">
        <button
          v-for="t in templates"
          :key="t.id"
          type="button"
          class="nc-game"
          :class="{ active: chosen?.id === t.id }"
          :style="t.accent_color ? { '--accent-game': `#${t.accent_color}` } : undefined"
          @click="choose(t)"
        >
          <img
            v-if="t.cover_image_url"
            :src="t.cover_image_url"
            :alt="t.name"
            class="nc-game-cover"
            loading="lazy"
          />
          <span v-else class="nc-game-icon">{{ t.icon || "🎮" }}</span>
          <span class="nc-game-name">{{ t.name }}</span>
          <span v-if="t.category" class="nc-game-cat">{{ t.category }}</span>
          <span class="nc-game-ram">{{ t.default_memory_mb }} Mo conseillés</span>
        </button>
      </div>

      <p v-if="!templates.length" class="nc-hint">
        Aucun jeu autorisé pour ce serveur. Vérifie la liste
        <code>allowed_templates</code> dans la configuration Nexus.
      </p>

      <!-- Étape 2 : les réglages -->
      <template v-if="chosen">
        <p v-if="chosen.description" class="nc-desc">{{ chosen.description }}</p>

        <GameResourcesGuide :slug="chosen.slug" />

        <h2 class="nc-step">2. Règle le serveur</h2>

        <div class="nc-form">
          <label class="nc-field">
            <span>Nom du serveur</span>
            <input v-model="name" type="text" maxlength="64" />
            <small v-if="nameError" class="nc-err">{{ nameError }}</small>
          </label>

          <label class="nc-field">
            <span>Mémoire allouée (Mo)</span>
            <span class="nc-slider">
              <input
                v-model.number="memoryMb"
                type="range"
                class="nc-range"
                :min="chosen.min_memory_mb"
                :max="chosen.max_memory_mb"
                step="512"
              />
              <input
                v-model.number="memoryMb"
                type="number"
                class="nc-slider-value"
                :min="chosen.min_memory_mb"
                :max="chosen.max_memory_mb"
                step="512"
              />
            </span>
            <small v-if="memoryError" class="nc-err">{{ memoryError }}</small>
            <small v-else class="nc-note">
              Entre {{ chosen.min_memory_mb }} et {{ chosen.max_memory_mb }} Mo.
            </small>
          </label>

          <label class="nc-field">
            <span>Cœurs processeur</span>
            <span class="nc-slider">
              <input
                v-model.number="cpuLimit"
                type="range"
                class="nc-range"
                min="0.5"
                max="6"
                step="0.5"
              />
              <input
                v-model.number="cpuLimit"
                type="number"
                class="nc-slider-value"
                min="0.5"
                max="6"
                step="0.5"
              />
            </span>
            <small class="nc-note">
              Plafond (0.5 à 6 cœurs). Minecraft n'exploite quasiment qu'un
              cœur : 2 suffisent. Palworld est multithreadé : 4 sont utiles.
            </small>
          </label>

          <label class="nc-field">
            <span>Date et heure d'ouverture *</span>
            <input v-model="openAt" type="datetime-local" required />
            <small class="nc-note">
              Heure de démarrage et de publication automatique de l'IP du serveur.
            </small>
          </label>

          <label class="nc-field">
            <span>Date et heure de fermeture *</span>
            <input v-model="closeAt" type="datetime-local" required />
            <small class="nc-note">
              Heure de fin pour alimenter automatiquement le calendrier de la communauté.
            </small>
          </label>

          <div v-if="dateError" class="nc-field nc-field-full">
            <small class="nc-err">{{ dateError }}</small>
          </div>

        </div>

        <!-- Champs propres au jeu, générés depuis le schéma et regroupés. -->
        <details v-for="g in groupes" :key="g.nom" class="nc-group" open>
          <summary>
            {{ g.nom }}
            <span class="nc-group-count">{{ g.champs.length }}</span>
          </summary>
          <div class="nc-form">
            <GameConfigField
              v-for="f in g.champs"
              :key="f.key"
              :field="f"
              v-model="values[f.key]"
            />
          </div>
        </details>

        <div class="nc-actions">
          <AppButton variant="primary" :disabled="!canSubmit" @click="submit">
            {{ submitting ? "Création…" : "Créer le serveur" }}
          </AppButton>
          <RouterLink to="/nexus/servers" class="nc-cancel">Annuler</RouterLink>
        </div>

        <p class="nc-warn">
          Le conteneur est créé à l'arrêt. Il faudra le démarrer depuis la liste
          des serveurs — la première image peut mettre plusieurs minutes à se
          télécharger.
        </p>
      </template>
    </template>
  </AdminPageShell>
</template>

<style scoped>
.nc-hint,
.nc-desc,
.nc-note {
  color: var(--text-secondary);
}

.nc-error,
.nc-err {
  color: var(--danger);
}

.nc-step {
  font-size: 1.05rem;
  margin: var(--space-lg) 0 var(--space-sm);
}

.nc-step:first-of-type {
  margin-top: 0;
}

.nc-games {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(11rem, 1fr));
  gap: var(--space-sm);
}

.nc-game {
  --accent-game: var(--accent);
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 2px;
  padding: var(--space-md);
  background: var(--bg-card);
  border: 1px solid var(--bg-hover);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  cursor: pointer;
  text-align: left;
  transition: var(--transition-fast);
}

.nc-game:hover {
  border-color: var(--accent-game);
}

.nc-game.active {
  border-color: var(--accent-game);
  box-shadow: 0 0 0 1px var(--accent-game) inset;
}

.nc-game-icon {
  font-size: 1.5rem;
}

.nc-game-cover {
  width: 100%;
  aspect-ratio: 1;
  object-fit: cover;
  border-radius: var(--radius-sm);
  margin-bottom: 4px;
}

.nc-game-name {
  font-weight: 600;
}

.nc-game-cat,
.nc-game-ram {
  font-size: 0.78rem;
  color: var(--text-secondary);
}

.nc-desc {
  margin: var(--space-sm) 0 0;
  font-size: 0.9rem;
}

/* Colonnes un peu plus larges que le strict necessaire : a 16 rem, les
   libelles longs passaient a trois lignes et hachaient la grille. */
.nc-form {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr));
  gap: var(--space-md) var(--space-lg);
}

.nc-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 0.9rem;
}

.nc-field > span {
  color: var(--text-secondary);
}

.nc-field input,
.nc-field select {
  background: var(--bg-card);
  border: 1px solid var(--bg-hover);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  padding: 6px 10px;
}

.nc-field input:focus,
.nc-field select:focus {
  outline: none;
  border-color: var(--accent);
}


.nc-field small {
  font-size: 0.76rem;
}

.nc-group {
  margin-top: var(--space-md);
  border: 1px solid var(--bg-hover);
  border-radius: var(--radius-md);
  padding: var(--space-sm) var(--space-md);
}

.nc-group > summary {
  cursor: pointer;
  font-weight: 600;
  color: var(--text-primary);
}

.nc-group > .nc-form {
  margin-top: var(--space-md);
}

/* Combien de reglages se cachent derriere une section repliee. */
.nc-group-count {
  margin-left: var(--space-sm);
  padding: 1px 8px;
  border-radius: 999px;
  background: var(--bg-hover);
  color: var(--text-secondary);
  font-size: 0.72rem;
  font-weight: 500;
}

/* Curseur + valeur chiffree, comme les reglages du jeu (GameConfigField) :
   la plage autorisee se lit d'un coup d'oeil, la saisie exacte reste possible. */
.nc-slider {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}

.nc-range {
  flex: 1;
  accent-color: var(--accent);
  cursor: pointer;
  padding: 0;
  border: none;
  background: none;
}

.nc-slider-value {
  width: 6.5rem;
  flex-shrink: 0;
}

.nc-actions {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  margin-top: var(--space-lg);
}



.nc-cancel {
  color: var(--text-secondary);
}

.nc-warn {
  margin-top: var(--space-md);
  font-size: 0.84rem;
  color: var(--text-secondary);
}

</style>
