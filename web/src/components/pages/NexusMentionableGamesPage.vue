<script setup lang="ts">
import { ref, watch, computed } from "vue";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useAuth } from "../../composables/useAuth";
import { useToast } from "../../composables/useToast";
import { useConfirm } from "../../composables/useConfirm";
import {
  nexusMentionableGamesService,
  type MentionableGame,
  type SyncDirection,
  type SyncDivergence,
  type SyncReport
} from "@/services/nexusMentionableGamesService";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import ChannelSelect from "@/components/atoms/ChannelSelect.vue";
import EmojiSelect from "@/components/atoms/EmojiSelect.vue";

const { selectedGuildId } = useGuildSelector();
const { user } = useAuth();
const { success, error: showError } = useToast();
const { confirm } = useConfirm();

const games = ref<MentionableGame[]>([]);
const loading = ref(false);

const newGameName = ref("");
const newGameEmoji = ref("");
const newGameCategory = ref("");

const deployChannelId = ref("");
const deployCategory = ref("");
const deploying = ref(false);
const creating = ref(false);

const MAX_GAMES_PER_PANEL = 25;

const availableCategories = computed(() => {
  const cats = new Set(games.value.map(g => g.category).filter(c => c !== null && c.trim() !== ""));
  return Array.from(cats) as string[];
});

const gamesForDeployment = computed(() => {
  const category = deployCategory.value.trim().toLocaleLowerCase();
  if (!category) return games.value;
  return games.value.filter(game => game.category?.trim().toLocaleLowerCase() === category);
});

async function load() {
  if (!selectedGuildId.value) {
    games.value = [];
    return;
  }
  loading.value = true;
  try {
    games.value = await nexusMentionableGamesService.listGames(selectedGuildId.value);
  } catch (e) {
    showError(e instanceof Error ? e.message : "Erreur de chargement");
  } finally {
    loading.value = false;
  }
}

async function refreshUntilRoles(guildId: string, gameIds: Set<string>) {
  const pending = new Set(gameIds);
  for (let attempt = 0; pending.size > 0 && attempt < 8; attempt += 1) {
    await new Promise<void>(resolve => window.setTimeout(resolve, 750));
    try {
      const refreshed = await nexusMentionableGamesService.listGames(guildId);
      if (selectedGuildId.value !== guildId) return;
      games.value = refreshed;
      for (const game of refreshed) {
        if (game.role_id) pending.delete(game.id);
      }
    } catch {
      // La creation Discord est asynchrone : une actualisation intermediaire
      // ratee ne transforme pas la demande deja acceptee en echec.
    }
  }
}

watch(selectedGuildId, load, { immediate: true });

async function onCreate() {
  if (!user.value || !selectedGuildId.value || creating.value) return;
  const guildId = selectedGuildId.value;
  const gameName = newGameName.value.trim();
  if (!gameName) {
    showError("Le nom du jeu est requis.");
    return;
  }
  creating.value = true;
  try {
    const created = await nexusMentionableGamesService.createGame(guildId, {
      guild_id: guildId,
      game_name: gameName,
      emoji: newGameEmoji.value.trim() || null,
      category: newGameCategory.value.trim() || null,
      created_by: user.value.id
    });
    success("Jeu créé. Son rôle Discord est en cours de création.");
    newGameName.value = "";
    newGameEmoji.value = "";
    newGameCategory.value = "";
    await load();
    await refreshUntilRoles(guildId, new Set([created.id]));
  } catch (e) {
    showError(e instanceof Error ? e.message : "Erreur création");
  } finally {
    creating.value = false;
  }
}

async function onDelete(game: MentionableGame) {
  if (!selectedGuildId.value || !user.value) return;
  if (!(await confirm({ title: "Supprimer", message: `Supprimer le jeu ${game.game_name} ? Le rôle Discord sera supprimé.` }))) return;
  
  try {
    await nexusMentionableGamesService.deleteGame(selectedGuildId.value, game.id, user.value.id);
    success("Jeu supprimé.");
    await load();
  } catch (e) {
    showError(e instanceof Error ? e.message : "Erreur suppression");
  }
}

async function onDeploy() {
  const guildId = selectedGuildId.value;
  const channelId = deployChannelId.value;
  if (!guildId || !channelId || deploying.value) return;

  const selectedGames = gamesForDeployment.value;
  if (selectedGames.length === 0) {
    showError("Aucun jeu ne correspond à ce panneau.");
    return;
  }
  const withoutEmoji = selectedGames.filter(game => !game.emoji?.trim());
  if (withoutEmoji.length > 0) {
    showError(`Ajoutez un emoji avant de déployer : ${withoutEmoji.map(game => game.game_name).join(", ")}.`);
    return;
  }
  const emojiOwners = new Map<string, string[]>();
  for (const game of selectedGames) {
    const emoji = game.emoji!.trim();
    emojiOwners.set(emoji, [...(emojiOwners.get(emoji) ?? []), game.game_name]);
  }
  const duplicates = Array.from(emojiOwners.values()).filter(names => names.length > 1);
  if (duplicates.length > 0) {
    showError(`Chaque jeu doit avoir un emoji différent : ${duplicates.map(names => names.join(" / ")).join(", ")}.`);
    return;
  }
  if (selectedGames.length > MAX_GAMES_PER_PANEL) {
    showError(`Ce panneau contient ${selectedGames.length} jeux. Limite : ${MAX_GAMES_PER_PANEL}. Utilisez une catégorie.`);
    return;
  }

  deploying.value = true;
  try {
    await nexusMentionableGamesService.deployPanel(guildId, {
      channel_id: channelId,
      category: deployCategory.value.trim() || null,
    });
    success("Demande envoyée au bot. Le panneau et les rôles sont en cours de création.");
    deployChannelId.value = "";
    deployCategory.value = "";

    // Le endpoint repond 202 avant que Discord ait termine. Quelques
    // rechargements courts rendent visibles les role_id crees par le bot sans
    // obliger l'administrateur a faire F5.
    await refreshUntilRoles(
      guildId,
      new Set(selectedGames.filter(game => !game.role_id).map(game => game.id)),
    );
  } catch (e) {
    showError(e instanceof Error ? e.message : "Erreur déploiement");
  } finally {
    deploying.value = false;
  }
}

// ── Consolidation base ↔ Discord ──
//
// Les deux mondes peuvent diverger sans que personne ne le voie : un rôle
// supprimé à la main dans Discord ne remonte nulle part, et les abonnements
// échouent ensuite en silence. Cette section montre l'écart et laisse choisir
// le côté qui fait foi — jamais l'inverse, aucune réparation n'est déduite.

const syncReport = ref<SyncReport | null>(null);
const syncLoading = ref(false);
const syncChecking = ref(false);
/** Clé de l'écart en cours de résolution, pour ne verrouiller que sa ligne. */
const resolvingKey = ref<string | null>(null);

async function loadSyncReport() {
  if (!selectedGuildId.value) {
    syncReport.value = null;
    return;
  }
  syncLoading.value = true;
  try {
    syncReport.value = await nexusMentionableGamesService.getSyncReport(selectedGuildId.value);
  } catch (e) {
    showError(e instanceof Error ? e.message : "Rapport de synchronisation indisponible");
  } finally {
    syncLoading.value = false;
  }
}

/**
 * Demande une photographie fraîche puis recharge quelques fois : le bot répond
 * de façon asynchrone, et l'API a déjà rendu la main quand il commence son
 * travail. Sans cela l'écran afficherait l'ancien rapport et donnerait
 * l'impression que la vérification n'a rien fait.
 */
async function onSyncCheck() {
  const guildId = selectedGuildId.value;
  if (!guildId || syncChecking.value) return;
  syncChecking.value = true;
  try {
    await nexusMentionableGamesService.requestSyncCheck(guildId);
    success("Vérification demandée au bot.");
    const before = syncReport.value?.inventory_taken_at ?? null;
    for (let attempt = 0; attempt < 8; attempt += 1) {
      await new Promise<void>(resolve => window.setTimeout(resolve, 900));
      if (selectedGuildId.value !== guildId) return;
      await loadSyncReport();
      if ((syncReport.value?.inventory_taken_at ?? null) !== before) break;
    }
  } catch (e) {
    showError(e instanceof Error ? e.message : "Vérification impossible");
  } finally {
    syncChecking.value = false;
  }
}

/** Ce que la résolution va faire, dit en clair AVANT de la confirmer. */
function resolutionSummary(d: SyncDivergence, direction: SyncDirection): string {
  if (direction === "discord") {
    switch (d.kind) {
      case "role_missing":
        return `Le rôle de « ${d.game_name} » n'existe plus dans Discord. Le dashboard oubliera cette liaison ; le jeu restera, sans rôle.`;
      case "role_orphan":
        return `Le rôle « ${d.role_name} » sera conservé tel quel dans Discord. Cet écart réapparaîtra tant qu'il existe.`;
      case "panel_message_missing":
        return "Le panneau disparu sera oublié côté dashboard. Aucun message ne sera republié.";
      default:
        return "";
    }
  }
  switch (d.kind) {
    case "role_missing":
      return `Un nouveau rôle sera créé dans Discord pour « ${d.game_name} », puis rattaché au jeu.`;
    case "role_unbound":
      return `Un rôle sera créé dans Discord pour « ${d.game_name} », puis rattaché au jeu.`;
    case "role_orphan":
      return `Le rôle « ${d.role_name} » sera SUPPRIMÉ de Discord. Les membres qui le portent le perdront.`;
    case "panel_message_missing":
      return "Un nouveau panneau sera publié dans le salon, et l'ancien message oublié.";
  }
}

async function onResolve(d: SyncDivergence, direction: SyncDirection) {
  const guildId = selectedGuildId.value;
  if (!guildId || resolvingKey.value) return;

  // Une résolution touche Discord ou la base : elle se confirme, et le texte
  // dit ce qui va vraiment se passer plutôt qu'un « Confirmer ? » creux.
  const confirmed = await confirm({
    title: direction === "discord" ? "Discord fait foi" : "Le dashboard fait foi",
    message: resolutionSummary(d, direction),
  });
  if (!confirmed) return;

  resolvingKey.value = d.key;
  try {
    const outcome = await nexusMentionableGamesService.resolveSync(guildId, d.key, direction);
    success(outcome.detail);
    await Promise.all([load(), loadSyncReport()]);
    if (outcome.requested_from_discord) {
      // L'effet côté Discord n'est pas encore constaté : une nouvelle
      // photographie est nécessaire pour que la ligne disparaisse vraiment.
      await onSyncCheck();
    }
  } catch (e) {
    showError(e instanceof Error ? e.message : "Résolution impossible");
  } finally {
    resolvingKey.value = null;
  }
}

function divergenceTitle(d: SyncDivergence): string {
  switch (d.kind) {
    case "role_missing":
      return `${d.game_name} — rôle introuvable dans Discord`;
    case "role_unbound":
      return `${d.game_name} — aucun rôle associé`;
    case "role_orphan":
      return `${d.role_name} — rôle sans jeu`;
    case "panel_message_missing":
      return "Panneau disparu de son salon";
  }
}

function divergenceDetail(d: SyncDivergence): string {
  switch (d.kind) {
    case "role_missing":
      return "Le dashboard pointe vers un rôle supprimé : les abonnements échouent en silence.";
    case "role_unbound":
      return "Sans rôle, ce jeu ne peut être mentionné par personne.";
    case "role_orphan":
      return "Ce rôle porte les marques d'un rôle de jeu, mais aucun jeu ne le réclame.";
    case "panel_message_missing":
      return "Le message enregistré n'existe plus : les boutons d'abonnement ont disparu avec lui.";
  }
}

/** Un jeu sans rôle ne peut pas s'aligner sur Discord : il n'y a rien à y lire. */
function canFollowDiscord(d: SyncDivergence): boolean {
  return d.kind !== "role_unbound";
}

const syncCheckedLabel = computed(() => {
  const taken = syncReport.value?.inventory_taken_at;
  if (!taken) return null;
  const date = new Date(taken);
  return Number.isNaN(date.getTime()) ? taken : date.toLocaleString("fr-FR");
});

watch(selectedGuildId, loadSyncReport, { immediate: true });

function getEmojiUrl(emojiStr: string | null): string | null {
  if (!emojiStr) return null;
  const match = emojiStr.match(/<a?:.+:(\d+)>/);
  if (match && match[1]) {
    const ext = emojiStr.startsWith('<a:') ? 'gif' : 'png';
    return `https://cdn.discordapp.com/emojis/${match[1]}.${ext}`;
  }
  return null;
}
</script>

<template>
  <AdminPageShell
    title="Jeux Mentionnables"
    description="Gérez les rôles de jeux pour que les joueurs puissent se notifier via des panneaux."
  >
    <div class="page-content">
      <div v-if="loading" class="loading">Chargement...</div>
      <div v-else>
        <section class="add-section">
          <h3>Ajouter un jeu</h3>
          <div class="form-row">
            <input v-model="newGameName" placeholder="Nom du jeu" class="input-base" />
            <EmojiSelect v-model="newGameEmoji" :guild-id="selectedGuildId" style="width: 250px" />
            <input v-model="newGameCategory" placeholder="Catégorie (optionnel)" class="input-base" />
            <button @click="onCreate" class="btn-primary" :disabled="!newGameName.trim() || creating">
              {{ creating ? 'Création…' : 'Créer' }}
            </button>
          </div>
        </section>

        <section class="deploy-section">
          <h3>Déployer un panel</h3>
          <div class="form-row">
            <ChannelSelect v-model="deployChannelId" :guild-id="selectedGuildId" style="width: 250px" />
            <select v-model="deployCategory" class="input-base">
              <option value="">— Toutes les catégories (optionnel) —</option>
              <option v-for="cat in availableCategories" :key="cat" :value="cat">{{ cat }}</option>
            </select>
            <button @click="onDeploy" class="btn-primary" :disabled="!deployChannelId || deploying">
              {{ deploying ? 'Déploiement…' : 'Déployer' }}
            </button>
          </div>
          <p class="help-text">Crée un panneau Discord avec une réaction différente par jeu. Sans catégorie, tous les jeux sont inclus.</p>
        </section>

        <section class="sync-section">
          <div class="sync-head">
            <h3>Synchronisation avec Discord</h3>
            <button @click="onSyncCheck" class="btn-secondary" :disabled="syncChecking">
              {{ syncChecking ? 'Vérification…' : 'Vérifier maintenant' }}
            </button>
          </div>

          <p v-if="syncLoading" class="help-text">Lecture du rapport…</p>

          <!-- Sans photographie, on ne sait rien. Le dire, plutôt que d'afficher
               un rassurant « tout va bien » qui serait faux. -->
          <p v-else-if="!syncReport || !syncReport.inventory_taken_at" class="sync-unknown">
            État inconnu : le bot n'a pas encore rendu compte de ce serveur Discord.
            Lance une vérification pour comparer les rôles réels au dashboard.
          </p>

          <template v-else>
            <p class="help-text">
              Dernière photographie du serveur Discord : {{ syncCheckedLabel }}.
            </p>

            <p v-if="syncReport.divergences.length === 0" class="sync-ok">
              ✅ Le dashboard et Discord sont d'accord.
            </p>

            <ul v-else class="sync-list">
              <li v-for="d in syncReport.divergences" :key="d.key" class="sync-item">
                <div class="sync-item-text">
                  <strong>{{ divergenceTitle(d) }}</strong>
                  <span class="help-text">{{ divergenceDetail(d) }}</span>
                </div>
                <div class="sync-item-actions">
                  <button
                    v-if="canFollowDiscord(d)"
                    class="btn-secondary"
                    :disabled="resolvingKey !== null"
                    @click="onResolve(d, 'discord')"
                  >
                    Discord fait foi
                  </button>
                  <button
                    class="btn-primary"
                    :disabled="resolvingKey !== null"
                    @click="onResolve(d, 'dashboard')"
                  >
                    Le dashboard fait foi
                  </button>
                </div>
              </li>
            </ul>
          </template>
        </section>

        <section class="list-section">
          <h3>Jeux configurés ({{ games.length }})</h3>
          <table v-if="games.length > 0" class="data-table">
            <thead>
              <tr>
                <th>Jeu</th>
                <th>Catégorie</th>
                <th>Rôle Discord</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="g in games" :key="g.id">
                <td>
                  <img v-if="getEmojiUrl(g.emoji)" :src="getEmojiUrl(g.emoji)!" class="table-emoji" />
                  <span v-else-if="g.emoji" class="table-emoji">{{ g.emoji }}</span>
                  <strong>{{ g.game_name }}</strong>
                </td>
                <td>{{ g.category || '—' }}</td>
                <td class="mono">{{ g.role_id || 'Aucun' }}</td>
                <td>
                  <button class="btn-danger" @click="onDelete(g)">Supprimer</button>
                </td>
              </tr>
            </tbody>
          </table>
          <div v-else class="empty-state">
            Aucun jeu configuré.
          </div>
        </section>
      </div>
    </div>
  </AdminPageShell>
</template>

<style scoped>
.page-content {
  display: flex;
  flex-direction: column;
  gap: 30px;
}

section h3 {
  margin-bottom: 15px;
  font-size: 16px;
}

.form-row {
  display: flex;
  gap: 15px;
  align-items: center;
}

.input-base {
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 14px;
}

.btn-primary {
  padding: 8px 16px;
  background: var(--accent);
  color: white;
  border: none;
  border-radius: var(--radius-md);
  cursor: pointer;
  font-weight: 600;
}
.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* Les deux directions d'une résolution ne se valent pas visuellement : celle
   qui touche Discord reste discrète, pour qu'on ne l'active pas par réflexe. */
.btn-secondary {
  padding: 8px 16px;
  background: transparent;
  color: var(--text-primary);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  cursor: pointer;
  font-weight: 600;
}
.btn-secondary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.sync-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 15px;
  flex-wrap: wrap;
}

.sync-head h3 {
  margin-bottom: 0;
}

.sync-unknown {
  margin-top: 12px;
  padding: 12px;
  border-radius: var(--radius-md);
  border-left: 3px solid var(--accent-warm);
  background: var(--accent-warm-bg);
  font-size: 13px;
}

.sync-ok {
  margin-top: 12px;
  font-size: 14px;
}

.sync-list {
  list-style: none;
  margin: 12px 0 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.sync-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 15px;
  flex-wrap: wrap;
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-card);
}

.sync-item-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.sync-item-text .help-text {
  margin-top: 0;
}

.sync-item-actions {
  display: flex;
  gap: 10px;
}

.btn-danger {
  padding: 6px 12px;
  background: var(--danger);
  color: white;
  border: none;
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: 12px;
}

.help-text {
  margin-top: 8px;
  font-size: 12px;
  color: var(--text-secondary);
}

.data-table {
  width: 100%;
  border-collapse: collapse;
}

.data-table th, .data-table td {
  padding: 12px;
  text-align: left;
  border-bottom: 1px solid var(--border);
}

.data-table th {
  font-weight: 600;
  color: var(--text-secondary);
  font-size: 13px;
}

.mono {
  font-family: monospace;
  color: var(--text-secondary);
}

.empty-state {
  color: var(--text-secondary);
  padding: 20px;
  text-align: center;
  background: var(--bg-card);
  border-radius: var(--radius-md);
}

.table-emoji {
  width: 24px;
  height: 24px;
  vertical-align: middle;
  margin-right: 8px;
}

.loading {
  padding: 40px;
  text-align: center;
  color: var(--text-secondary);
}
</style>
