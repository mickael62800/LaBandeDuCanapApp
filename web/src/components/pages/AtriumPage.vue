<script setup lang="ts">
// Accueil IA Atrium — premier ecran d'administration de l'univers.
//
// Atrium etait jusqu'ici pilotable uniquement depuis Discord : son etat, ses
// quotas et sa base de connaissances n'existaient qu'en gRPC, a l'usage
// d'`atrium-bot`. Cette page est la premiere fenetre du back-office dessus,
// via la passerelle nginx `/atrium-api/`.
//
// Tout ce qui est affiche ici est desormais reglable PAR SERVEUR : les quotas
// sont passes des variables d'environnement a `bot_guild_config`, conformement
// a la regle du depot. Un serveur qui n'a rien regle retombe sur les valeurs
// d'environnement — l'ecran indique laquelle des deux s'applique.

import { computed, ref, watch } from "vue";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import DashboardHero from "../organisms/DashboardHero.vue";
import AppToggle from "../atoms/AppToggle.vue";
import AppButton from "../atoms/AppButton.vue";
import AppInput from "../atoms/AppInput.vue";
import AppTextarea from "../atoms/AppTextarea.vue";
import ErrorState from "../atoms/ErrorState.vue";
import EmptyState from "../atoms/EmptyState.vue";
import LoadingState from "../atoms/LoadingState.vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useAuth } from "@/composables/useAuth";
import { useToast } from "@/composables/useToast";
import { useConfirm } from "@/composables/useConfirm";
import { errMsg } from "@/utils/errMsg";
import {
  atriumService,
  type AtriumDocument,
  type AtriumUsage,
} from "@/services/atriumService";

// Longueur alignée sur la validation du domaine côté API (2 000 caractères).
const CONTEXT_MAX = 2000;

const { selectedGuildId } = useGuildSelector();
const { user } = useAuth();
const { success, error: toastError } = useToast();
const { confirm } = useConfirm();

const loading = ref(false);
const loadError = ref<string | null>(null);
const saving = ref(false);

const enabled = ref(false);
const usage = ref<AtriumUsage | null>(null);
const documents = ref<AtriumDocument[]>([]);

// Formulaire des quotas. En chaines : `AppInput` travaille en `string`, et
// convertir a la volee ferait disparaitre un champ vide en cours de saisie.
const form = ref({
  user_daily_limit: "",
  user_cooldown_secs: "",
  global_daily_limit: "",
});
// Copie de reference pour savoir ce qui a change et n'envoyer QUE cela.
const saved = ref({ ...form.value });
const savingConfig = ref(false);

const dirty = computed(() =>
  (Object.keys(form.value) as (keyof typeof form.value)[]).some(
    (k) => form.value[k] !== saved.value[k],
  ),
);

// Consignes de ton (welcome_context / conflict_context). Séparées des quotas :
// ce sont des textes libres, enregistrés dans le même `bot_guild_config` mais
// via un bloc distinct pour n'envoyer que ce que l'admin a réellement modifié.
const contextForm = ref({ welcome_context: "", conflict_context: "" });
const savedContext = ref({ ...contextForm.value });
const savingContext = ref(false);
const dirtyContext = computed(() =>
  (Object.keys(contextForm.value) as (keyof typeof contextForm.value)[]).some(
    (k) => contextForm.value[k] !== savedContext.value[k],
  ),
);

// Départ éclair. Bloc à part des deux précédents : ce n'est ni un quota ni une
// consigne de ton, et un seul « Enregistrer » pour trois natures de réglages
// obligerait à réécrire des clés que l'admin n'a pas touchées.
const ghostForm = ref({ welcome_ghost_minutes: "" });
const savedGhost = ref({ ...ghostForm.value });
const savingGhost = ref(false);
const dirtyGhost = computed(
  () => ghostForm.value.welcome_ghost_minutes !== savedGhost.value.welcome_ghost_minutes,
);

async function saveGhost() {
  const guildId = selectedGuildId.value;
  if (!guildId || !dirtyGhost.value) return;
  savingGhost.value = true;
  try {
    await atriumService.setConfig(guildId, {
      welcome_ghost_minutes: ghostForm.value.welcome_ghost_minutes.trim(),
    });
    savedGhost.value = { ...ghostForm.value };
    success("Départ éclair enregistré.");
  } catch (e: unknown) {
    toastError(errMsg(e));
  } finally {
    savingGhost.value = false;
  }
}

function resetGhost() {
  ghostForm.value = { ...savedGhost.value };
}

async function saveContext() {
  const guildId = selectedGuildId.value;
  if (!guildId) return;
  const values: Record<string, string> = {};
  for (const k of Object.keys(contextForm.value) as (keyof typeof contextForm.value)[]) {
    if (contextForm.value[k] !== savedContext.value[k]) values[k] = contextForm.value[k];
  }
  if (Object.keys(values).length === 0) return;

  savingContext.value = true;
  try {
    await atriumService.setConfig(guildId, values);
    savedContext.value = { ...contextForm.value };
    success("Contexte enregistré.");
  } catch (e: unknown) {
    toastError(errMsg(e));
  } finally {
    savingContext.value = false;
  }
}

function resetContext() {
  contextForm.value = { ...savedContext.value };
}

function fillForm(u: AtriumUsage) {
  form.value = {
    user_daily_limit: String(u.user_daily_limit),
    user_cooldown_secs: String(u.user_cooldown_secs),
    global_daily_limit: String(u.global_daily_limit),
  };
  saved.value = { ...form.value };
}

function resetForm() {
  form.value = { ...saved.value };
}

async function saveConfig() {
  const guildId = selectedGuildId.value;
  if (!guildId) return;
  // N'envoyer que les cles modifiees : ecrire les trois a chaque fois
  // materialiserait en base des valeurs que l'admin n'a jamais choisies, et
  // ferait perdre le repli sur la configuration d'installation.
  const values: Record<string, string> = {};
  for (const k of Object.keys(form.value) as (keyof typeof form.value)[]) {
    if (form.value[k] !== saved.value[k]) values[k] = form.value[k].trim();
  }
  if (Object.keys(values).length === 0) return;

  savingConfig.value = true;
  try {
    await atriumService.setConfig(guildId, values);
    saved.value = { ...form.value };
    success("Quotas enregistrés.");
    // Relecture : les compteurs du jour et les limites appliquees viennent du
    // serveur, pas du formulaire.
    usage.value = await atriumService.usage(guildId);
  } catch (e: unknown) {
    toastError(errMsg(e));
  } finally {
    savingConfig.value = false;
  }
}

/// Part du quota global consommee aujourd'hui, bornee a 100 %.
/// Une limite a zero signifie « pas de plafond » : afficher une jauge dans ce
/// cas ferait croire a une saturation imminente.
const globalPct = computed(() => {
  const u = usage.value;
  if (!u || u.global_daily_limit <= 0) return null;
  return Math.min(100, Math.round((u.global_used_today / u.global_daily_limit) * 100));
});

async function load() {
  const guildId = selectedGuildId.value;
  if (!guildId) return;
  loading.value = true;
  loadError.value = null;
  try {
    // En parallele : les trois lectures sont independantes, et l'ecran n'a
    // d'interet qu'une fois les trois disponibles.
    const [state, stats, docs, context] = await Promise.all([
      atriumService.state(guildId),
      atriumService.usage(guildId),
      atriumService.knowledge(guildId),
      atriumService.context(guildId),
    ]);
    enabled.value = state.enabled;
    usage.value = stats;
    documents.value = docs;
    fillForm(stats);
    contextForm.value = {
      welcome_context: context.welcome_context,
      conflict_context: context.conflict_context,
    };
    savedContext.value = { ...contextForm.value };
    ghostForm.value = { welcome_ghost_minutes: context.welcome_ghost_minutes };
    savedGhost.value = { ...ghostForm.value };
  } catch (e: unknown) {
    loadError.value = errMsg(e);
  } finally {
    loading.value = false;
  }
}

// ── Effacement de la memoire d'un membre ──
const forgetMemberId = ref("");
const forgetting = ref(false);

// Meme regle que `valider_member_id` cote API : un identifiant Discord est un
// entier decimal d'au plus 20 chiffres. Le verifier ici evite un aller-retour
// pour une faute de frappe ; l'API reste l'autorite.
const forgetMemberIdValide = computed(() => {
  const v = forgetMemberId.value.trim();
  return v.length > 0 && v.length <= 20 && /^\d+$/.test(v);
});

async function forgetMember() {
  const guildId = selectedGuildId.value;
  const memberId = forgetMemberId.value.trim();
  if (!guildId || !user.value || !forgetMemberIdValide.value || forgetting.value) return;

  const ok = await confirm({
    title: "Effacer la mémoire de ce membre ?",
    message:
      `Tout ce qu'Atrium a retenu des échanges du membre ${memberId} sur ce ` +
      `serveur sera supprimé. Cette action est immédiate et irréversible.`,
  });
  if (!ok) return;

  forgetting.value = true;
  try {
    const res = await atriumService.forgetMember(guildId, memberId, user.value.id);
    // Le decompte vient du serveur : « 0 message » est une reponse valable
    // (membre qui n'a jamais parle a Atrium, ou effacement deja fait), et la
    // dire evite de laisser croire a un echec.
    success(
      res.deleted === 0
        ? "Aucun message retenu pour ce membre — rien à effacer."
        : `${res.deleted} message${res.deleted > 1 ? "s" : ""} effacé${res.deleted > 1 ? "s" : ""}.`,
    );
    forgetMemberId.value = "";
  } catch (e: unknown) {
    toastError(errMsg(e));
  } finally {
    forgetting.value = false;
  }
}

async function toggleEnabled(next: boolean) {
  const guildId = selectedGuildId.value;
  // Garde ici et pas via une prop `disabled` : `AppToggle` n'en expose pas,
  // l'attribut retomberait sur le <label> sans rien empecher. Deux clics
  // rapides enverraient alors deux bascules concurrentes.
  if (!guildId || !user.value || saving.value) return;
  saving.value = true;
  // Optimiste : l'interrupteur suit le doigt, et revient en arriere si l'API
  // refuse. Attendre l'aller-retour donnait un bouton qui semble ne rien faire.
  const previous = enabled.value;
  enabled.value = next;
  try {
    await atriumService.setState(guildId, next, user.value.id);
    success(next ? "Atrium activé sur ce serveur." : "Atrium désactivé sur ce serveur.");
  } catch (e: unknown) {
    enabled.value = previous;
    toastError(errMsg(e));
  } finally {
    saving.value = false;
  }
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR", {
    dateStyle: "short",
    timeStyle: "short",
  });
}

// Au rechargement complet, le store restaure la guilde selectionnee apres le
// montage de la page. `onMounted(load)` partait alors avec un ID vide, quittait
// sans requete et laissait tous les formulaires a leur valeur initiale. La
// watcher immediate couvre a la fois l'arrivee tardive de l'ID et un changement
// de serveur depuis le selecteur.
watch(selectedGuildId, load, { immediate: true });
</script>

<template>
  <AdminPageShell class="atrium-page">
    <DashboardHero
      title="Atrium"
      subtitle="Accueil assisté par Intelligence Artificielle et base de connaissances communautaire."
      logo="/atrium_logo.png"
      universe="atrium"
    >
      <template #actions>
        <AppButton variant="secondary" :disabled="loading" @click="load">
          Actualiser
        </AppButton>
      </template>
    </DashboardHero>

    <ErrorState
      v-if="loadError"
      :message="loadError"
      :retryable="true"
      @retry="load"
    />

    <LoadingState v-else-if="loading" />

    <template v-else>
      <section class="card at-switch">
        <div class="at-switch-text">
          <h2>Atrium sur ce serveur</h2>
          <p>
            Désactivé, le bot continue de tourner mais répond qu'il est hors
            service au lieu d'appeler le modèle — aucun quota n'est consommé.
          </p>
        </div>
        <AppToggle :model-value="enabled" @update:model-value="toggleEnabled" />
      </section>

      <section v-if="usage" class="card">
        <h2>Consommation du jour</h2>
        <div class="at-stats">
          <div class="at-stat">
            <span class="at-value">{{ usage.guild_used_today }}</span>
            <span class="at-label">requêtes sur ce serveur</span>
          </div>
          <div class="at-stat">
            <span class="at-value">{{ usage.guild_active_users_today }}</span>
            <span class="at-label">membres actifs</span>
          </div>
          <div class="at-stat">
            <span class="at-value">
              {{ usage.global_used_today }}
              <small v-if="usage.global_daily_limit > 0">
                / {{ usage.global_daily_limit }}
              </small>
            </span>
            <span class="at-label">requêtes toutes guildes</span>
          </div>
        </div>

        <div v-if="globalPct !== null" class="at-gauge" :title="`${globalPct} % du quota global`">
          <i :style="{ width: `${globalPct}%` }" :class="{ hot: globalPct >= 80 }"></i>
        </div>

      </section>

      <section class="card">
        <h2>Quotas de ce serveur</h2>
        <p class="at-note">
          Ces limites protègent la facture du fournisseur de modèle.
          <strong>0 signifie « illimité ».</strong> Non renseignées, elles
          retombent sur les valeurs d'installation
          (<code>ATRIUM_USER_DAILY_LIMIT</code>,
          <code>ATRIUM_USER_COOLDOWN_SECS</code>,
          <code>ATRIUM_GLOBAL_DAILY_LIMIT</code>).
        </p>

        <div class="at-form">
          <label class="at-field">
            <span>Requêtes par membre et par jour</span>
            <AppInput v-model="form.user_daily_limit" type="number" :min="0" />
          </label>
          <label class="at-field">
            <span>Délai entre deux questions (s)</span>
            <AppInput v-model="form.user_cooldown_secs" type="number" :min="0" />
          </label>
          <label class="at-field">
            <span>Plafond quotidien du serveur</span>
            <AppInput v-model="form.global_daily_limit" type="number" :min="0" />
          </label>
        </div>

        <div class="at-form-actions">
          <AppButton
            variant="primary"
            :disabled="!dirty || savingConfig"
            @click="saveConfig"
          >
            Enregistrer
          </AppButton>
          <AppButton variant="secondary" :disabled="!dirty" @click="resetForm">
            Annuler
          </AppButton>
        </div>
      </section>

      <section class="card">
        <h2>Message d'accueil</h2>
        <p class="at-note">
          Un membre qui rejoint puis quitte le serveur dans ce délai n'a jamais
          vraiment été là : son message d'accueil est supprimé du salon général.
          Passé ce délai, le message reste. <strong>0 désactive</strong> la
          suppression. Sentinel a son propre réglage équivalent, à aligner
          depuis la page Bienvenue.
        </p>

        <div class="at-form">
          <label class="at-field">
            <span>Départ éclair : délai (minutes)</span>
            <AppInput
              v-model="ghostForm.welcome_ghost_minutes"
              type="number"
              :min="0"
              :max="1440"
            />
          </label>
        </div>

        <div class="at-form-actions">
          <AppButton
            variant="primary"
            :disabled="!dirtyGhost || savingGhost"
            @click="saveGhost"
          >
            Enregistrer
          </AppButton>
          <AppButton variant="secondary" :disabled="!dirtyGhost" @click="resetGhost">
            Annuler
          </AppButton>
        </div>
      </section>

      <section class="card">
        <h2>Comportement de l'IA</h2>
        <p class="at-note">
          Ces consignes ajustent le <strong>ton et la personnalité</strong>
          d'Atrium. Elles ne remplacent pas la base de connaissances : les
          règles, salons et rôles restent tirés des documents indexés. Laissées
          vides, Atrium garde son comportement par défaut.
        </p>

        <div class="at-context">
          <label class="at-field">
            <span>Contexte d'accueil (réponses aux membres)</span>
            <AppTextarea
              v-model="contextForm.welcome_context"
              :rows="4"
              :maxlength="CONTEXT_MAX"
              placeholder="Ex. Reste très chaleureux, tutoie les membres, glisse une touche d'humour."
            />
          </label>
          <label class="at-field">
            <span>Contexte d'apaisement (messages de conflit)</span>
            <AppTextarea
              v-model="contextForm.conflict_context"
              :rows="4"
              :maxlength="CONTEXT_MAX"
              placeholder="Ex. Ton ferme mais bienveillant, rappelle la règle sans accuser personne."
            />
          </label>
        </div>

        <div class="at-form-actions">
          <AppButton
            variant="primary"
            :disabled="!dirtyContext || savingContext"
            @click="saveContext"
          >
            Enregistrer
          </AppButton>
          <AppButton
            variant="secondary"
            :disabled="!dirtyContext"
            @click="resetContext"
          >
            Annuler
          </AppButton>
        </div>
      </section>

      <section class="card">
        <h2>Base de connaissances</h2>
        <EmptyState
          v-if="documents.length === 0"
          message="Aucun document indexé pour ce serveur."
        />
        <table v-else class="at-table">
          <thead>
            <tr>
              <th>Document</th>
              <th>Fragments</th>
              <th>État</th>
              <th>Mis à jour</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="d in documents" :key="d.id">
              <td>
                <span class="at-title">{{ d.title }}</span>
                <span v-if="d.source_url" class="at-source">{{ d.source_url }}</span>
              </td>
              <td>
                <!-- Zero fragment = document enregistre mais jamais vectorise,
                     donc invisible pour les reponses. C'est la panne
                     silencieuse la plus probable ici : on la signale. -->
                <span :class="{ 'at-warn': d.chunk_count === 0 }">
                  {{ d.chunk_count }}
                </span>
              </td>
              <td>{{ d.enabled ? "Actif" : "Inactif" }}</td>
              <td>{{ formatDate(d.updated_at) }}</td>
            </tr>
          </tbody>
        </table>
      </section>

      <!-- Effacement sur demande. Volontairement en bas de page et sans
           surlignage : c'est une action rare et irreversible, pas un reglage.
           La confirmation rappelle le nombre de messages concernes plutot que
           « etes-vous sur ? », qui n'apprend rien a personne. -->
      <section class="card at-danger">
        <h2>Effacer la mémoire d'un membre</h2>
        <p class="at-note">
          Supprime tout ce qu'Atrium a retenu des échanges d'un membre sur ce
          serveur. Répond à une demande d'effacement : c'est immédiat et
          définitif. La base de connaissances et les résumés d'ambiance ne sont
          pas concernés.
        </p>
        <div class="at-forget">
          <AppInput
            v-model="forgetMemberId"
            placeholder="Identifiant Discord du membre (18 à 20 chiffres)"
            :disabled="forgetting"
          />
          <AppButton
            variant="danger"
            :disabled="!forgetMemberIdValide || forgetting"
            @click="forgetMember"
          >
            {{ forgetting ? "Effacement…" : "Effacer" }}
          </AppButton>
        </div>
        <p v-if="forgetMemberId && !forgetMemberIdValide" class="at-warn">
          Un identifiant Discord ne contient que des chiffres.
        </p>
      </section>
    </template>
  </AdminPageShell>
</template>

<style scoped>
@import "./_admin-page-shared.css";

.at-switch {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
}
.at-switch-text p {
  color: var(--text-secondary);
  font-size: 13px;
  margin: 4px 0 0;
  max-width: 60ch;
}

.at-stats {
  display: flex;
  flex-wrap: wrap;
  gap: 32px;
  margin: 16px 0;
}
.at-stat {
  display: flex;
  flex-direction: column;
}
.at-value {
  font-size: 26px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  color: var(--universe-accent, var(--accent));
}
.at-value small {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-secondary);
}
.at-label {
  font-size: 12px;
  color: var(--text-secondary);
}

.at-gauge {
  height: 6px;
  border-radius: var(--radius-pill);
  background: var(--bg-card);
  overflow: hidden;
}
.at-gauge i {
  display: block;
  height: 100%;
  background: var(--universe-accent, var(--accent));
  border-radius: var(--radius-pill);
}
.at-gauge i.hot {
  background: var(--accent-warm, #e67e22);
}

.at-note {
  color: var(--text-secondary);
  font-size: 12px;
  line-height: 1.6;
  margin: 0 0 16px;
}

.at-form {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 16px;
}
.at-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 13px;
  color: var(--text-secondary);
}
.at-form-actions {
  display: flex;
  gap: 8px;
  margin-top: 16px;
}

.at-context {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 16px;
}
.at-context textarea {
  resize: vertical;
  min-height: 96px;
}

.at-table {
  width: 100%;
  border-collapse: collapse;
}
.at-table th,
.at-table td {
  text-align: left;
  padding: 8px 10px;
  border-bottom: 1px solid var(--border);
  font-size: 13px;
}
.at-title {
  display: block;
  font-weight: 600;
}
.at-source {
  display: block;
  font-size: 11px;
  color: var(--text-secondary);
}
.at-warn {
  color: var(--accent-warm, #e67e22);
  font-weight: 700;
}

/* Bordure teintee plutot qu'un fond rouge : la section doit se distinguer sans
   crier. Le rouge appuye est reserve au bouton, seul element qui agit. */
.at-danger {
  border-color: color-mix(in srgb, var(--danger, #e74c3c) 35%, var(--border));
}
.at-forget {
  display: flex;
  gap: 10px;
  align-items: center;
  flex-wrap: wrap;
  margin-top: 12px;
}
.at-forget :deep(input) {
  flex: 1;
  min-width: 260px;
}

@media (max-width: 700px) {
  .at-switch {
    flex-direction: column;
    align-items: flex-start;
  }
  /* Le bouton passe pleine largeur sous le champ : cote a cote, il devenait
     une cible de quelques pixels a cote d'un champ long. */
  .at-forget :deep(button) {
    width: 100%;
  }
}
</style>
