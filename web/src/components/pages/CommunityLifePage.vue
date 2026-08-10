<script setup lang="ts">
import AppCheckbox from "../atoms/AppCheckbox.vue";
import AppButton from "../atoms/AppButton.vue";
import AdminPageShell from "../layouts/AdminPageShell.vue";
// Back-office de la vie communautaire.
//
// Quatre entités sur un seul écran à onglets : elles alimentent la même page
// membre et un modérateur passe naturellement de l'une à l'autre.
//
// Deux d'entre elles se REDIGENT ici (sondages, nouvelles), les deux autres
// se MODERENT (annonces de recherche, écrites par les membres) ou se
// DESIGNENT (membre du mois). D'où des onglets qui ne se ressemblent pas :
// forcer une table uniforme aurait masqué cette différence de nature.

import { computed, ref } from "vue";

import AppTabs from "../molecules/AppTabs.vue";
import ImagePicker from "../molecules/ImagePicker.vue";
import ConfirmDialog from "../molecules/ConfirmDialog.vue";
import { useCommunityLife, type LifeTab } from "@/composables/useCommunityLife";
import { useConfirm } from "@/composables/useConfirm";
import { useToast } from "@/composables/useToast";
import { errMsg } from "@/utils/errMsg";
import {
  communityAdminService,
  type CreatePollInput,
  type UpsertNewsInput,
} from "@/services/communityAdminService";

const { tab, showArchived, lfg, polls, spotlight, news, loading, guildId, refresh } =
  useCommunityLife();
const { success, error: toastErr } = useToast();
const { confirm } = useConfirm();

const TABS = [
  { key: "news", label: "Annonces du site", icon: "📰" },
  { key: "polls", label: "Sondages", icon: "🗳️" },
  { key: "spotlight", label: "Membre du mois", icon: "⭐" },
  { key: "lfg", label: "Recherche de joueurs", icon: "🎮" },
];

const busy = ref(false);

/// Exécute une action puis recharge. Centralisé : sans ça, chaque bouton
/// répéterait le même try/catch/toast/refresh, et l'un finirait par oublier
/// le rechargement.
async function agir(action: () => Promise<unknown>, message: string) {
  busy.value = true;
  try {
    await action();
    success(message);
    await refresh();
  } catch (e: unknown) {
    toastErr(errMsg(e));
  } finally {
    busy.value = false;
  }
}

// ── Confirmation de suppression ──

/// Suppression toujours confirmée : ces contenus sont visibles publiquement,
/// un clic malheureux se verrait immédiatement sur le site.
async function supprimer(label: string, run: () => Promise<unknown>) {
  const ok = await confirm({
    title: `Supprimer ${label} ?`,
    message: "Cette action est définitive.",
  });
  if (ok) await agir(run, "Supprimé.");
}

// ── Nouvelles ──

/// `null` = aucun formulaire ouvert. Une chaîne vide = création ; un
/// identifiant = modification.
const newsEdite = ref<string | null>(null);
const formNews = ref<UpsertNewsInput>(nouvelleVierge());

function nouvelleVierge(): UpsertNewsInput {
  return { title: "", body: "", image_url: "", is_pinned: false, is_public: true };
}

function ouvrirNews(id?: string) {
  const existante = id ? news.value.find((n) => n.id === id) : undefined;
  formNews.value = existante
    ? {
        title: existante.title,
        body: existante.body,
        image_url: existante.image_url ?? "",
        is_pinned: existante.is_pinned,
        is_public: existante.is_public,
      }
    : nouvelleVierge();
  newsEdite.value = id ?? "";
}

async function enregistrerNews() {
  const g = guildId.value;
  if (!g) return;

  // Le chemin vide doit partir en `null` : le serveur refuse toute valeur qui
  // n'est pas un chemin relatif, et une chaîne vide en est une.
  const payload: UpsertNewsInput = {
    ...formNews.value,
    image_url: formNews.value.image_url?.trim() || null,
  };

  const id = newsEdite.value;
  await agir(
    () =>
      id
        ? communityAdminService.updateNews(id, payload)
        : communityAdminService.createNews(g, payload),
    id ? "Annonce mise à jour." : "Annonce publiée.",
  );
  newsEdite.value = null;
}

// ── Sondages ──

const sondageOuvert = ref(false);
const formPoll = ref<CreatePollInput>(sondageVierge());

function sondageVierge(): CreatePollInput {
  // Deux options d'emblée : c'est le minimum accepté par le serveur, autant
  // que le formulaire le montre plutôt que de le refuser à l'envoi.
  return {
    question: "",
    description: "",
    closes_at: dansJours(7),
    is_public: true,
    options: [{ label: "" }, { label: "" }],
  };
}

/// Valeur par défaut de la date de clôture, au format attendu par
/// `<input type="datetime-local">` (heure locale, sans fuseau).
function dansJours(n: number): string {
  const d = new Date();
  d.setDate(d.getDate() + n);
  d.setSeconds(0, 0);
  const p = (v: number) => String(v).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}`;
}

function ouvrirSondage() {
  formPoll.value = sondageVierge();
  sondageOuvert.value = true;
}

const optionsValides = computed(
  () => formPoll.value.options.filter((o) => o.label.trim()).length >= 2,
);

async function enregistrerSondage() {
  const g = guildId.value;
  if (!g) return;

  await agir(
    () =>
      communityAdminService.createPoll(g, {
        ...formPoll.value,
        description: formPoll.value.description?.trim() || null,
        // `datetime-local` donne une heure locale sans fuseau : on la
        // convertit en instant absolu, sinon le serveur l'interpréterait en
        // UTC et clôturerait le sondage avec une heure de décalage.
        closes_at: new Date(formPoll.value.closes_at).toISOString(),
        options: formPoll.value.options.filter((o) => o.label.trim()),
      }),
    "Sondage ouvert.",
  );
  sondageOuvert.value = false;
}

// ── Membre du mois ──

const designationOuverte = ref(false);
const formSpot = ref({ user_id: "", period: "", reason: "" });

function ouvrirDesignation() {
  formSpot.value = { user_id: "", period: "", reason: "" };
  designationOuverte.value = true;
}

async function enregistrerDesignation() {
  const g = guildId.value;
  if (!g) return;

  await agir(
    () =>
      communityAdminService.designate(g, {
        user_id: formSpot.value.user_id.trim(),
        period: formSpot.value.period.trim() || undefined,
        reason: formSpot.value.reason,
      }),
    "Membre du mois désigné.",
  );
  designationOuverte.value = false;
}

// ── Formats ──

function fmt(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR", {
    day: "numeric",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function fmtJour(iso: string): string {
  return new Date(iso).toLocaleDateString("fr-FR", { day: "numeric", month: "long" });
}

const expire = (iso: string) => new Date(iso) <= new Date();
</script>

<template>
  <AdminPageShell title="Vie de la communauté" icon="🛋️" class="cl-page">
    <template #lede>
      Ce qui alimente l'espace membre du site. Les annonces de recherche de
      joueurs sont écrites par les membres&nbsp;: on les modère, on ne les
      rédige pas.
    </template>
    <template #actions>
      <AppCheckbox v-model="showArchived">Afficher les éléments clos et brouillons</AppCheckbox>
    </template>

    <!-- `AppTabs` parle en `string` ; l'onglet est un type fermé. On projette
         dans un sens et on restreint dans l'autre plutôt que d'élargir le
         type de l'état. -->
    <AppTabs :model-value="tab" :tabs="TABS" @update:model-value="tab = $event as LifeTab" />

    <p v-if="!guildId" class="muted">Sélectionne un serveur pour commencer.</p>
    <p v-else-if="loading" class="muted">Chargement…</p>

    <template v-else>
      <!-- ── Annonces du site ── -->
      <section v-if="tab === 'news'" class="cl-sec">
        <div class="cl-actions">
          <AppButton variant="primary" @click="ouvrirNews()">
            Nouvelle annonce
          </AppButton>
        </div>

        <form v-if="newsEdite !== null" class="cl-form" @submit.prevent="enregistrerNews">
          <h3>{{ newsEdite ? "Modifier l'annonce" : "Nouvelle annonce" }}</h3>

          <label>
            Titre
            <input v-model="formNews.title" type="text" maxlength="160" required />
          </label>

          <label>
            Texte
            <textarea v-model="formNews.body" rows="5" required></textarea>
          </label>

          <label>
            Image
            <ImagePicker
              :model-value="formNews.image_url ?? ''"
              mode="relative"
              @update:model-value="formNews.image_url = $event || null"
            />
            <!-- Le serveur refuse toute URL absolue : elle figerait le domaine
                 en base et ouvrirait la porte à un `javascript:` dans un src. -->
            <small class="muted">
              Chemin relatif seulement, depuis <code>web/public/</code>. Les URL
              complètes sont refusées.
            </small>
          </label>

          <div class="cl-checks">
            <AppCheckbox v-model="formNews.is_pinned">Épingler en tête de liste</AppCheckbox>
            <AppCheckbox v-model="formNews.is_public">Visible par les visiteurs non connectés</AppCheckbox>
          </div>

          <div class="cl-form-foot">
            <AppButton variant="primary" type="submit" :disabled="busy">Enregistrer</AppButton>
            <AppButton variant="ghost" @click="newsEdite = null">Annuler</AppButton>
          </div>
        </form>

        <p v-if="!news.length" class="muted">Aucune annonce pour l'instant.</p>

        <ul v-else class="cl-list">
          <li v-for="n in news" :key="n.id" class="cl-item">
            <img v-if="n.image_url" :src="n.image_url" alt="" class="cl-thumb" />
            <div class="cl-body">
              <div class="cl-line">
                <strong>{{ n.title }}</strong>
                <span v-if="n.is_pinned" class="pill">épinglée</span>
                <span v-if="!n.is_public" class="pill warn">non publique</span>
              </div>
              <p class="muted small">{{ n.body.slice(0, 160) }}</p>
              <span class="muted small">{{ fmt(n.published_at) }}</span>
            </div>
            <div class="cl-item-actions">
              <AppButton variant="ghost" size="sm" @click="ouvrirNews(n.id)">Modifier</AppButton>
              <AppButton variant="danger" size="sm" @click="supprimer(n.title, () => communityAdminService.deleteNews(n.id))"
              >
                Supprimer
              </AppButton>
            </div>
          </li>
        </ul>
      </section>

      <!-- ── Sondages ── -->
      <section v-else-if="tab === 'polls'" class="cl-sec">
        <div class="cl-actions">
          <AppButton variant="primary" @click="ouvrirSondage">
            Nouveau sondage
          </AppButton>
        </div>

        <form v-if="sondageOuvert" class="cl-form" @submit.prevent="enregistrerSondage">
          <h3>Nouveau sondage</h3>

          <label>
            Question
            <input v-model="formPoll.question" type="text" maxlength="200" required />
          </label>

          <label>
            Précision (facultatif)
            <textarea v-model="formPoll.description" rows="2"></textarea>
          </label>

          <label>
            Clôture
            <input v-model="formPoll.closes_at" type="datetime-local" required />
          </label>

          <fieldset class="cl-options">
            <legend>Choix</legend>
            <div v-for="(o, i) in formPoll.options" :key="i" class="cl-option">
              <input v-model="o.label" type="text" maxlength="120" :placeholder="`Choix ${i + 1}`" />
              <button
                v-if="formPoll.options.length > 2"
                type="button"
                class="btn small"
                @click="formPoll.options.splice(i, 1)"
              >
                ✕
              </button>
            </div>
            <AppButton variant="ghost" size="sm" v-if="formPoll.options.length < 10"
              
              
              @click="formPoll.options.push({ label: '' })">
              Ajouter un choix
            </AppButton>
            <p v-if="!optionsValides" class="muted small">
              Il faut au moins deux choix renseignés.
            </p>
          </fieldset>

          <AppCheckbox v-model="formPoll.is_public">Visible par les visiteurs non connectés</AppCheckbox>

          <div class="cl-form-foot">
            <AppButton variant="primary" type="submit" :disabled="busy || !optionsValides">
              Ouvrir le sondage
            </AppButton>
            <AppButton variant="ghost" @click="sondageOuvert = false">Annuler</AppButton>
          </div>
        </form>

        <p v-if="!polls.length" class="muted">Aucun sondage.</p>

        <ul v-else class="cl-list">
          <li v-for="p in polls" :key="p.id" class="cl-item bloc">
            <div class="cl-body">
              <div class="cl-line">
                <strong>{{ p.question }}</strong>
                <span v-if="!p.is_open" class="pill warn">clos</span>
              </div>

              <ul class="cl-bars">
                <li v-for="o in p.options" :key="o.id">
                  <span class="cl-bar-label">{{ o.label }}</span>
                  <span class="cl-bar">
                    <i :style="{ width: `${o.share}%`, background: `#${o.color}` }"></i>
                  </span>
                  <span class="muted small">{{ o.votes }} · {{ o.share }} %</span>
                </li>
              </ul>

              <span class="muted small">
                {{ p.total_votes }} vote(s) · clôture le {{ fmtJour(p.closes_at) }}
              </span>
            </div>

            <div class="cl-item-actions">
              <AppButton variant="ghost" size="sm" v-if="p.is_open"
                
                
                :disabled="busy"
                @click="agir(() => communityAdminService.closePoll(p.id), 'Sondage clos.')"
              >
                Clore
              </AppButton>
              <AppButton variant="danger" size="sm" @click="supprimer(p.question, () => communityAdminService.deletePoll(p.id))"
              >
                Supprimer
              </AppButton>
            </div>
          </li>
        </ul>
      </section>

      <!-- ── Membre du mois ── -->
      <section v-else-if="tab === 'spotlight'" class="cl-sec">
        <div class="cl-actions">
          <AppButton variant="primary" @click="ouvrirDesignation">
            Désigner
          </AppButton>
        </div>

        <form v-if="designationOuverte" class="cl-form" @submit.prevent="enregistrerDesignation">
          <h3>Désigner le membre du mois</h3>

          <label>
            Identifiant Discord du membre
            <input v-model="formSpot.user_id" type="text" inputmode="numeric" required />
            <small class="muted">
              Le pseudo et l'avatar sont récupérés côté serveur&nbsp;: recopiés
              à la main, ils deviendraient faux au prochain changement de pseudo.
            </small>
          </label>

          <label>
            Période (facultatif)
            <input v-model="formSpot.period" type="text" placeholder="2026-08" pattern="\d{4}-\d{2}" />
            <small class="muted">Vide = mois en cours. Un seul membre par mois.</small>
          </label>

          <label>
            Pourquoi lui&nbsp;?
            <textarea v-model="formSpot.reason" rows="3" required></textarea>
            <!-- Obligatoire en base, pas seulement ici : sans le pourquoi, la
                 section du site n'afficherait qu'un nom. -->
            <small class="muted">
              C'est ce qui donne son sens à la distinction. Affiché sur le site.
            </small>
          </label>

          <div class="cl-form-foot">
            <AppButton variant="primary" type="submit" :disabled="busy">Désigner</AppButton>
            <AppButton variant="ghost" @click="designationOuverte = false">Annuler</AppButton>
          </div>
        </form>

        <p v-if="!spotlight.length" class="muted">Personne n'a encore été désigné.</p>

        <ul v-else class="cl-list">
          <li v-for="s in spotlight" :key="s.id" class="cl-item">
            <div class="cl-body">
              <div class="cl-line">
                <strong>{{ s.username || s.user_id }}</strong>
                <span class="pill">{{ s.period }}</span>
              </div>
              <p class="muted small">{{ s.reason }}</p>
            </div>
            <div class="cl-item-actions">
              <AppButton variant="danger" size="sm" @click="
                  supprimer(
                    `${s.username} (${s.period})`,
                    () => communityAdminService.deleteSpotlight(guildId!, s.id),
                  )
                "
              >
                Retirer
              </AppButton>
            </div>
          </li>
        </ul>
      </section>

      <!-- ── Recherche de joueurs ── -->
      <section v-else class="cl-sec">
        <p class="muted small">
          Ces annonces sont publiées par les membres depuis le site. Le rôle du
          staff est de fermer celles qui traînent et de retirer les abusives.
        </p>

        <p v-if="!lfg.length" class="muted">Aucune annonce.</p>

        <ul v-else class="cl-list">
          <li v-for="a in lfg" :key="a.id" class="cl-item">
            <div class="cl-body">
              <div class="cl-line">
                <strong>{{ a.author_name || a.author_id }}</strong>
                <span class="pill">{{ a.game }}</span>
                <span v-if="!a.is_open" class="pill warn">fermée</span>
                <span v-else-if="expire(a.expires_at)" class="pill warn">expirée</span>
              </div>
              <p v-if="a.description" class="muted small">{{ a.description }}</p>
              <span class="muted small">
                Cherche {{ a.slots }} joueur(s) · {{ a.when_text }} ·
                {{ a.interested.length }} intéressé(s) · expire le {{ fmt(a.expires_at) }}
              </span>
            </div>
            <div class="cl-item-actions">
              <AppButton variant="ghost" size="sm" v-if="a.is_open"
                
                
                :disabled="busy"
                @click="agir(() => communityAdminService.closeLfg(a.id), 'Annonce fermée.')"
              >
                Fermer
              </AppButton>
              <AppButton variant="danger" size="sm" @click="supprimer(`l'annonce de ${a.author_name}`, () => communityAdminService.deleteLfg(a.id))"
              >
                Supprimer
              </AppButton>
            </div>
          </li>
        </ul>
      </section>
    </template>

    <ConfirmDialog />
  </AdminPageShell>
</template>

<style scoped>
.cl-page {
  padding: 0;
}

.muted {
  color: var(--text-secondary);
}

.small {
  font-size: 12px;
}


.cl-sec {
  margin-top: 16px;
}

.cl-actions {
  margin-bottom: 12px;
}

.btn {
  background: transparent;
  border: 1px solid var(--border);
  color: var(--text-primary);
  border-radius: var(--radius-sm);
  padding: 6px 14px;
  font: inherit;
  font-size: 13px;
  cursor: pointer;
}

.btn:hover:not(:disabled) {
  border-color: var(--accent);
}

.btn:disabled {
  opacity: 0.5;
  cursor: default;
}

.btn.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
  font-weight: 600;
}

.btn.small {
  padding: 3px 10px;
  font-size: 12px;
}

.btn.danger:hover:not(:disabled) {
  border-color: var(--danger);
  color: var(--danger);
}

/* ── Formulaires ── */
.cl-form {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
  margin-bottom: 16px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-elevated, rgba(255, 255, 255, 0.02));
}

.cl-form h3 {
  margin: 0;
  font-size: 15px;
}

.cl-form label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 13px;
}

.cl-form label.cb {
  flex-direction: row;
  align-items: center;
}

.cl-form input[type="text"],
.cl-form input[type="datetime-local"],
.cl-form textarea {
  background: var(--bg-input, rgba(0, 0, 0, 0.25));
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 7px 10px;
  color: var(--text-primary);
  font: inherit;
  font-size: 13px;
}

.cl-form textarea {
  resize: vertical;
}

.cl-checks {
  display: flex;
  flex-wrap: wrap;
  gap: 1rem;
}

.cl-options {
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.cl-options legend {
  font-size: 12px;
  color: var(--text-secondary);
  padding: 0 4px;
}

.cl-option {
  display: flex;
  gap: 6px;
}

.cl-option input {
  flex: 1;
}

.cl-form-foot {
  display: flex;
  gap: 8px;
}

/* ── Listes ── */
.cl-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.cl-item {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 12px 14px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
}

.cl-thumb {
  width: 92px;
  aspect-ratio: 16 / 9;
  object-fit: cover;
  border-radius: var(--radius-sm);
  flex: none;
}

.cl-body {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.cl-line {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

.cl-body p {
  margin: 0;
}

.cl-item-actions {
  display: flex;
  gap: 6px;
  flex: none;
}

.pill {
  font-size: 11px;
  padding: 1px 8px;
  border-radius: var(--radius-pill);
  background: rgba(168, 85, 247, 0.16);
  color: var(--text-secondary);
}

.pill.warn {
  background: rgba(245, 158, 11, 0.16);
  color: #fbbf24;
}

/* ── Barres de sondage ── */
.cl-bars {
  list-style: none;
  margin: 6px 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.cl-bars li {
  display: grid;
  grid-template-columns: minmax(6rem, 12rem) 1fr auto;
  align-items: center;
  gap: 8px;
  font-size: 12px;
}

.cl-bar-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cl-bar {
  height: 7px;
  border-radius: var(--radius-pill);
  background: rgba(255, 255, 255, 0.08);
  overflow: hidden;
}

.cl-bar i {
  display: block;
  height: 100%;
  border-radius: var(--radius-pill);
}

@media (max-width: 700px) {
  /* L'empilement de l'en-tete en petite largeur est desormais gere par
     `AdminPageShell`, commun a toutes les pages. */
  .cl-item {
    flex-direction: column;
  }

  .cl-bars li {
    grid-template-columns: 1fr;
  }
}
</style>
