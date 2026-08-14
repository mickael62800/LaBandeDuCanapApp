<script setup lang="ts">
/**
 * Catalogue des hauts faits : c'est ici que l'administrateur choisit l'IMAGE
 * de chaque haut fait, celle que le bot affiche dans l'annonce Discord.
 *
 * La page ne decide d'aucune regle d'attribution : elle edite des definitions.
 * L'API tranche ce qui est valide (URL, longueurs) et ce qui est autorise.
 */
import { computed, ref, watch } from "vue";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useToast } from "../../composables/useToast";
import {
  nexusAchievementsService,
  type Achievement,
} from "@/services/nexusAchievementsService";
import { imagesPourJeu, nomFichier } from "@/services/achievementImages";
import AdminPageShell from "../layouts/AdminPageShell.vue";

const { selectedGuildId } = useGuildSelector();
const { success, error: showError } = useToast();

const achievements = ref<Achievement[]>([]);
const loading = ref(false);
const savingId = ref<string | null>(null);

/** Brouillons d'URL par haut fait : on n'ecrit qu'a la validation. */
const drafts = ref<Record<string, string>>({});

const gameFilter = ref<string>("palworld");
const categoryFilter = ref<string>("");
const search = ref("");

const games = computed(() => {
  const set = new Set<string>();
  for (const a of achievements.value) if (a.game) set.add(a.game);
  return Array.from(set).sort();
});

const categories = computed(() => {
  const set = new Set<string>();
  for (const a of achievements.value) if (a.category) set.add(a.category);
  return Array.from(set).sort();
});

const visible = computed(() => {
  const term = search.value.trim().toLocaleLowerCase();
  return achievements.value.filter((a) => {
    if (categoryFilter.value && a.category !== categoryFilter.value) return false;
    if (!term) return true;
    return (
      a.name.toLocaleLowerCase().includes(term)
      || a.code.toLocaleLowerCase().includes(term)
      || a.description.toLocaleLowerCase().includes(term)
    );
  });
});

const withImage = computed(() => achievements.value.filter((a) => a.icon_url).length);

/** Images livrees pour le jeu d'un haut fait (vide si le jeu n'en fournit pas). */
function imagesDisponibles(a: Achievement): string[] {
  return imagesPourJeu(a.game);
}

/** Galerie ouverte pour ce haut fait (choix visuel plutot qu'a l'aveugle). */
const galerieOuverte = ref<string | null>(null);

function basculerGalerie(id: string) {
  galerieOuverte.value = galerieOuverte.value === id ? null : id;
}

/** Choisit une image depuis la galerie et enregistre dans la foulee. */
async function choisirImage(a: Achievement, chemin: string) {
  drafts.value[a.id] = chemin;
  galerieOuverte.value = null;
  await saveImage(a);
}

async function load() {
  if (!selectedGuildId.value) {
    achievements.value = [];
    return;
  }
  loading.value = true;
  try {
    const game = gameFilter.value || undefined;
    achievements.value = await nexusAchievementsService.list(selectedGuildId.value, game);
    drafts.value = Object.fromEntries(
      achievements.value.map((a) => [a.id, a.icon_url ?? ""]),
    );
  } catch (e) {
    showError(e instanceof Error ? e.message : "Erreur de chargement");
  } finally {
    loading.value = false;
  }
}

function isModified(a: Achievement): boolean {
  return (drafts.value[a.id] ?? "") !== (a.icon_url ?? "");
}

/** Enregistre l'image. Une valeur vide efface l'image (envoi de `null`). */
async function saveImage(a: Achievement) {
  if (!selectedGuildId.value || savingId.value) return;
  const draft = (drafts.value[a.id] ?? "").trim();
  savingId.value = a.id;
  try {
    const updated = await nexusAchievementsService.update(selectedGuildId.value, a.id, {
      icon_url: draft === "" ? null : draft,
    });
    const index = achievements.value.findIndex((x) => x.id === a.id);
    if (index !== -1) achievements.value[index] = updated;
    drafts.value[a.id] = updated.icon_url ?? "";
    success(`Image mise a jour : ${updated.name}`);
  } catch (e) {
    showError(e instanceof Error ? e.message : "Enregistrement impossible");
  } finally {
    savingId.value = null;
  }
}

async function toggleEnabled(a: Achievement) {
  if (!selectedGuildId.value || savingId.value) return;
  savingId.value = a.id;
  try {
    const updated = await nexusAchievementsService.update(selectedGuildId.value, a.id, {
      enabled: !a.enabled,
    });
    const index = achievements.value.findIndex((x) => x.id === a.id);
    if (index !== -1) achievements.value[index] = updated;
  } catch (e) {
    showError(e instanceof Error ? e.message : "Enregistrement impossible");
  } finally {
    savingId.value = null;
  }
}

watch([selectedGuildId, gameFilter], load, { immediate: true });
</script>

<template>
  <AdminPageShell title="Hauts faits" icon="🏆" width="wide">
    <p class="hf-intro">
      Catalogue des hauts faits. L'image choisie ici est celle que le bot
      affiche dans l'annonce Discord et dans <code>/haut-faits</code>.
    </p>
    <div class="hf-toolbar">
      <label>
        Jeu
        <select v-model="gameFilter">
          <option value="palworld">Palworld</option>
          <option v-for="g in games" :key="g" :value="g">{{ g }}</option>
          <option value="">Tous</option>
        </select>
      </label>
      <label>
        Catégorie
        <select v-model="categoryFilter">
          <option value="">Toutes</option>
          <option v-for="c in categories" :key="c" :value="c">{{ c }}</option>
        </select>
      </label>
      <label class="hf-search">
        Rechercher
        <input v-model="search" type="search" placeholder="nom, code, description" />
      </label>
      <span class="hf-count">
        {{ withImage }} / {{ achievements.length }} avec image
      </span>
    </div>

    <p v-if="loading" class="hf-empty">Chargement…</p>
    <p v-else-if="visible.length === 0" class="hf-empty">
      Aucun haut fait pour ce filtre.
    </p>

    <ul v-else class="hf-list">
      <li v-for="a in visible" :key="a.id" class="hf-item" :class="{ off: !a.enabled }">
        <div class="hf-preview">
          <!-- L'image vient d'une URL saisie par l'admin : si elle ne charge
               pas, on garde la vignette de repli plutot qu'une icone cassee. -->
          <img
            v-if="a.icon_url"
            :src="a.icon_url"
            :alt="a.name"
            loading="lazy"
            @error="($event.target as HTMLImageElement).style.display = 'none'"
          />
          <span v-else class="hf-placeholder">🏆</span>
        </div>

        <div class="hf-body">
          <div class="hf-head">
            <strong>{{ a.name }}</strong>
            <code class="hf-code">{{ a.code }}</code>
            <span class="hf-badge" :class="a.verification">
              {{ a.verification === "auto" ? "auto" : "validation admin" }}
            </span>
            <span v-if="a.category" class="hf-cat">{{ a.category }}</span>
          </div>
          <p class="hf-desc">{{ a.description }}</p>

          <div class="hf-row">
            <!-- Choix dans les images livrees pour ce jeu. La valeur est un
                 chemin local stable ; « Autre / URL » laisse la saisie libre. -->
            <select
              v-if="imagesDisponibles(a).length > 0"
              class="hf-select"
              :value="imagesDisponibles(a).includes(drafts[a.id] ?? '') ? drafts[a.id] : ''"
              @change="drafts[a.id] = ($event.target as HTMLSelectElement).value"
            >
              <option value="">— Aucune / URL libre —</option>
              <option v-for="img in imagesDisponibles(a)" :key="img" :value="img">
                {{ nomFichier(img) }}
              </option>
            </select>
            <button
              v-if="imagesDisponibles(a).length > 0"
              class="hf-toggle"
              type="button"
              @click="basculerGalerie(a.id)"
            >
              {{ galerieOuverte === a.id ? "Fermer" : "Parcourir" }}
            </button>
            <input
              v-model="drafts[a.id]"
              type="text"
              class="hf-input"
              placeholder="/Achievement/… ou https://…"
              @keyup.enter="saveImage(a)"
            />
            <button
              class="hf-save"
              :disabled="!isModified(a) || savingId === a.id"
              @click="saveImage(a)"
            >
              {{ savingId === a.id ? "…" : "Enregistrer" }}
            </button>
            <button class="hf-toggle" :disabled="savingId === a.id" @click="toggleEnabled(a)">
              {{ a.enabled ? "Désactiver" : "Activer" }}
            </button>
          </div>

          <!-- Galerie : cliquer une vignette choisit ET enregistre. -->
          <div v-if="galerieOuverte === a.id" class="hf-gallery">
            <button
              v-for="img in imagesDisponibles(a)"
              :key="img"
              type="button"
              class="hf-thumb"
              :class="{ active: drafts[a.id] === img }"
              :title="nomFichier(img)"
              @click="choisirImage(a, img)"
            >
              <img :src="img" :alt="nomFichier(img)" loading="lazy" />
            </button>
          </div>
        </div>
      </li>
    </ul>
  </AdminPageShell>
</template>

<style scoped>
.hf-intro {
  margin: 0 0 1rem;
  color: var(--text-muted, #9aa4bf);
  font-size: 0.9rem;
}
.hf-toolbar {
  display: flex;
  flex-wrap: wrap;
  gap: 1rem;
  align-items: flex-end;
  margin-bottom: 1.25rem;
}
.hf-toolbar label {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  font-size: 0.85rem;
  color: var(--text-muted, #9aa4bf);
}
.hf-toolbar select,
.hf-toolbar input {
  padding: 0.4rem 0.6rem;
  border-radius: 6px;
  border: 1px solid var(--border, #2c3350);
  background: var(--surface, #161b2e);
  color: inherit;
}
.hf-search {
  min-width: 240px;
  flex: 1;
}
.hf-count {
  margin-left: auto;
  font-size: 0.85rem;
  color: var(--text-muted, #9aa4bf);
}
.hf-empty {
  color: var(--text-muted, #9aa4bf);
}
.hf-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
.hf-item {
  display: flex;
  gap: 1rem;
  padding: 0.75rem;
  border: 1px solid var(--border, #2c3350);
  border-radius: 8px;
  background: var(--surface, #161b2e);
}
.hf-item.off {
  opacity: 0.55;
}
.hf-preview {
  width: 64px;
  height: 64px;
  flex: 0 0 64px;
  display: grid;
  place-items: center;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.04);
  overflow: hidden;
}
.hf-preview img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.hf-placeholder {
  font-size: 1.6rem;
}
.hf-body {
  flex: 1;
  min-width: 0;
}
.hf-head {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  align-items: center;
}
.hf-code {
  font-size: 0.75rem;
  color: var(--text-muted, #9aa4bf);
}
.hf-badge {
  font-size: 0.7rem;
  padding: 0.1rem 0.4rem;
  border-radius: 4px;
  background: rgba(241, 196, 15, 0.15);
  color: #f1c40f;
}
.hf-badge.auto {
  background: rgba(46, 204, 113, 0.15);
  color: #2ecc71;
}
.hf-cat {
  font-size: 0.7rem;
  color: var(--text-muted, #9aa4bf);
}
.hf-desc {
  margin: 0.25rem 0 0.5rem;
  font-size: 0.85rem;
  color: var(--text-muted, #9aa4bf);
}
.hf-row {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
}
.hf-input {
  flex: 1;
  min-width: 220px;
  padding: 0.4rem 0.6rem;
  border-radius: 6px;
  border: 1px solid var(--border, #2c3350);
  background: var(--bg, #0f1320);
  color: inherit;
}
.hf-row button {
  padding: 0.4rem 0.8rem;
  border-radius: 6px;
  border: 1px solid var(--border, #2c3350);
  background: var(--surface-2, #1d2338);
  color: inherit;
  cursor: pointer;
}
.hf-row button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.hf-select {
  padding: 0.4rem 0.6rem;
  border-radius: 6px;
  border: 1px solid var(--border, #2c3350);
  background: var(--bg, #0f1320);
  color: inherit;
  max-width: 200px;
}
.hf-gallery {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(72px, 1fr));
  gap: 0.5rem;
  margin-top: 0.75rem;
  padding: 0.5rem;
  border: 1px solid var(--border, #2c3350);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.02);
  max-height: 260px;
  overflow-y: auto;
}
.hf-thumb {
  padding: 0;
  border: 2px solid transparent;
  border-radius: 6px;
  background: none;
  cursor: pointer;
  overflow: hidden;
  aspect-ratio: 1;
}
.hf-thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.hf-thumb.active {
  border-color: #5865f2;
}
.hf-thumb:hover {
  border-color: #8b95f5;
}
.hf-save:not(:disabled) {
  background: #5865f2;
  border-color: #5865f2;
  color: #fff;
}
</style>
