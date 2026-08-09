<script setup lang="ts">
import { ref, watch, computed } from "vue";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useAuth } from "../../composables/useAuth";
import { useToast } from "../../composables/useToast";
import { useConfirm } from "../../composables/useConfirm";
import {
  nexusMentionableGamesService,
  type MentionableGame
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

const availableCategories = computed(() => {
  const cats = new Set(games.value.map(g => g.category).filter(c => c !== null && c.trim() !== ""));
  return Array.from(cats) as string[];
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

watch(selectedGuildId, load, { immediate: true });

async function onCreate() {
  if (!user.value || !selectedGuildId.value) return;
  try {
    await nexusMentionableGamesService.createGame(selectedGuildId.value, {
      guild_id: selectedGuildId.value,
      game_name: newGameName.value,
      emoji: newGameEmoji.value,
      category: newGameCategory.value,
      created_by: user.value.id
    });
    success("Jeu créé !");
    newGameName.value = "";
    newGameEmoji.value = "";
    newGameCategory.value = "";
    await load();
  } catch (e) {
    showError(e instanceof Error ? e.message : "Erreur création");
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
  if (!selectedGuildId.value || !deployChannelId.value) return;
  try {
    await nexusMentionableGamesService.deployPanel(selectedGuildId.value, {
      channel_id: deployChannelId.value,
      category: deployCategory.value || null,
    });
    success("Panel déployé !");
    deployChannelId.value = "";
    deployCategory.value = "";
  } catch (e) {
    showError(e instanceof Error ? e.message : "Erreur déploiement");
  }
}

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
            <button @click="onCreate" class="btn-primary" :disabled="!newGameName">Créer</button>
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
            <button @click="onDeploy" class="btn-primary" :disabled="!deployChannelId">Déployer</button>
          </div>
          <p class="help-text">Déploie ou rafraîchit le panneau Discord contenant les boutons d'abonnement aux jeux.</p>
        </section>

        <section class="list-section">
          <h3>Jeux configurés ({{ games.length }})</h3>
          <table v-if="games.length > 0" class="data-table">
            <thead>
              <tr>
                <th>Jeu</th>
                <th>Catégorie</th>
                <th>Rôle Discord</th>
                <th>Abonnés</th>
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
                <td>{{ g.subscriber_count }}</td>
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
