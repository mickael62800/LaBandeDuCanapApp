<script setup lang="ts">
/**
 * Panneau d'administration d'un serveur de jeu.
 *
 * Chaque jeu parle sa propre langue : Palworld bannit avec `BanPlayer`,
 * Minecraft avec `ban`. Demander à un administrateur de retenir trois syntaxes
 * revient à lui demander de ne pas se tromper au moment où il est pressé.
 *
 * L'écran se construit donc à partir du catalogue déclaré par le jeu, comme le
 * formulaire de réglages se construit à partir de son schéma. Ajouter une
 * commande se fait en base.
 *
 * Ce composant n'envoie JAMAIS de commande : il envoie une clé et des
 * paramètres. Le gabarit RCON reste côté serveur, qui compose et valide.
 */
import { computed, ref, watch } from "vue";
import AppButton from "@/components/atoms/AppButton.vue";
import { useConfirm } from "@/composables/useConfirm";
import { useToast } from "@/composables/useToast";
import {
  nexusGamesService,
  type GameCommand,
  type OnlinePlayer,
} from "@/services/nexusGamesService";

const props = defineProps<{
  guildId: string;
  serverId: string;
  /** Les commandes ne partent que vers un serveur en marche. */
  running: boolean;
}>();

const commands = ref<GameCommand[]>([]);
const players = ref<OnlinePlayer[]>([]);
const loading = ref(false);
const refreshingPlayers = ref(false);
/** Valeurs saisies, par commande puis par paramètre. */
const values = ref<Record<string, Record<string, string>>>({});
const busyKey = ref<string | null>(null);
const output = ref("");

const { confirm } = useConfirm();
const { success, error: showError } = useToast();

/** Commandes groupées par section, dans l'ordre du catalogue. */
const groups = computed(() => {
  const out: { nom: string; commandes: GameCommand[] }[] = [];
  for (const command of commands.value) {
    const nom = command.group?.trim() || "Commandes";
    let g = out.find((x) => x.nom === nom);
    if (!g) {
      g = { nom, commandes: [] };
      out.push(g);
    }
    g.commandes.push(command);
  }
  return out;
});

/** Un joueur se désigne par son identifiant de jeu ; à défaut, par son nom. */
function playerValue(player: OnlinePlayer): string {
  return player.game_player_id || player.name;
}

async function load() {
  loading.value = true;
  try {
    commands.value = await nexusGamesService.commands(props.guildId, props.serverId);
    const initial: Record<string, Record<string, string>> = {};
    for (const command of commands.value) {
      initial[command.key] = {};
      for (const param of command.params ?? []) {
        initial[command.key][param.key] = "";
      }
    }
    values.value = initial;
  } catch (e) {
    showError(e instanceof Error ? e.message : "Catalogue de commandes indisponible");
  } finally {
    loading.value = false;
  }
}

/**
 * Interroge le serveur de jeu en direct. Passe par RCON : on ne le fait donc
 * pas tout seul en boucle, seulement à la demande et à l'ouverture.
 */
async function loadPlayers() {
  if (!props.running || refreshingPlayers.value) return;
  refreshingPlayers.value = true;
  try {
    players.value = await nexusGamesService.onlinePlayers(props.guildId, props.serverId);
  } catch (e) {
    showError(e instanceof Error ? e.message : "Liste des joueurs indisponible");
  } finally {
    refreshingPlayers.value = false;
  }
}

async function run(command: GameCommand, overrides?: Record<string, string>) {
  if (busyKey.value) return;

  const params = { ...(values.value[command.key] ?? {}), ...(overrides ?? {}) };

  // Confirmation avant les gestes irréversibles ou visibles de tous. Le texte
  // dit ce qui va se passer plutôt qu'un « Confirmer ? » creux.
  if (command.confirm) {
    const cible = params.steamid || params.joueur || "";
    const ok = await confirm({
      title: command.label,
      message: [
        command.description,
        cible ? `Cible : ${cible}` : "",
        command.warning ? `⚠️ ${command.warning}` : "",
      ]
        .filter(Boolean)
        .join("\n\n"),
    });
    if (!ok) return;
  }

  busyKey.value = command.key;
  try {
    const res = await nexusGamesService.runCommand(
      props.guildId,
      props.serverId,
      command.key,
      params,
    );
    output.value = `> ${command.label}\n${res.response || "(aucune réponse)"}\n\n${output.value}`;
    success(`${command.label} : envoyé.`);
    // Les commandes qui touchent aux joueurs changent la liste : la relire
    // évite d'agir ensuite sur quelqu'un qui vient d'être expulsé.
    if ((command.params ?? []).some((p) => p.type === "player")) {
      await loadPlayers();
    }
  } catch (e) {
    const message = e instanceof Error ? e.message : "Commande refusée";
    output.value = `> ${command.label}\n[erreur] ${message}\n\n${output.value}`;
    showError(message);
  } finally {
    busyKey.value = null;
  }
}

/** Commandes applicables directement à un joueur de la liste. */
const playerCommands = computed(() =>
  commands.value.filter((c) => (c.params ?? []).some((p) => p.type === "player")),
);

/** Une commande est prête si tous ses paramètres obligatoires sont remplis. */
function isReady(command: GameCommand): boolean {
  return (command.params ?? [])
    .filter((p) => p.required)
    .every((p) => (values.value[command.key]?.[p.key] ?? "").trim() !== "");
}

watch(
  () => [props.serverId, props.running] as const,
  () => {
    void load();
    void loadPlayers();
  },
  { immediate: true },
);
</script>

<template>
  <div class="gcp">
    <p v-if="loading" class="sd-hint">Chargement des commandes…</p>

    <p v-else-if="!commands.length" class="sd-hint">
      Ce jeu ne déclare pas encore de commandes. La console libre reste disponible.
    </p>

    <template v-else>
      <p v-if="!running" class="gcp-off">
        Le serveur est arrêté : les commandes ne partiront qu'une fois démarré.
      </p>

      <!-- Joueurs connectés, avec leurs actions directes -->
      <section class="gcp-players">
        <header class="gcp-players-head">
          <h3>Joueurs connectés ({{ players.length }})</h3>
          <AppButton
            variant="secondary"
            size="xs"
            :disabled="!running || refreshingPlayers"
            @click="loadPlayers"
          >
            {{ refreshingPlayers ? "Lecture…" : "Actualiser" }}
          </AppButton>
        </header>

        <p v-if="!running" class="sd-hint">Serveur arrêté.</p>
        <p v-else-if="!players.length" class="sd-hint">Personne en jeu pour l'instant.</p>

        <ul v-else class="gcp-player-list">
          <li v-for="player in players" :key="playerValue(player)" class="gcp-player">
            <div class="gcp-player-id">
              <strong>{{ player.name }}</strong>
              <code v-if="player.game_player_id">{{ player.game_player_id }}</code>
            </div>
            <div class="gcp-player-actions">
              <AppButton
                v-for="command in playerCommands"
                :key="command.key"
                :variant="command.danger ? 'warning' : 'secondary'"
                size="xs"
                :disabled="!running || busyKey !== null"
                @click="
                  run(command, {
                    [(command.params ?? []).find((p) => p.type === 'player')!.key]:
                      playerValue(player),
                  })
                "
              >
                {{ command.label }}
              </AppButton>
            </div>
          </li>
        </ul>
      </section>

      <!-- Catalogue complet, par section -->
      <section v-for="g in groups" :key="g.nom" class="gcp-group">
        <h3>{{ g.nom }}</h3>
        <div class="gcp-cards">
          <article
            v-for="command in g.commandes"
            :key="command.key"
            class="gcp-card"
            :class="{ danger: command.danger }"
          >
            <header class="gcp-card-head">
              <strong>{{ command.label }}</strong>
              <span v-if="command.danger" class="gcp-tag">irréversible</span>
            </header>

            <p v-if="command.description" class="gcp-desc">{{ command.description }}</p>

            <label v-for="param in command.params ?? []" :key="param.key" class="gcp-field">
              <span>{{ param.label }}<em v-if="!param.required"> — facultatif</em></span>

              <!-- Un joueur se choisit dans la liste plutôt qu'il ne se recopie :
                   un identifiant Steam saisi à la main est une faute qui attend. -->
              <select
                v-if="param.type === 'player'"
                v-model="values[command.key][param.key]"
                :disabled="!running"
              >
                <option value="">— choisir un joueur —</option>
                <option
                  v-for="player in players"
                  :key="playerValue(player)"
                  :value="playerValue(player)"
                >
                  {{ player.name }}
                </option>
              </select>

              <select
                v-else-if="param.type === 'enum'"
                v-model="values[command.key][param.key]"
                :disabled="!running"
              >
                <option v-for="option in param.options ?? []" :key="option" :value="option">
                  {{ option }}
                </option>
              </select>

              <input
                v-else-if="param.type === 'number'"
                v-model="values[command.key][param.key]"
                type="number"
                :min="param.min"
                :max="param.max"
                :disabled="!running"
              />

              <input
                v-else
                v-model="values[command.key][param.key]"
                type="text"
                :maxlength="param.max_length"
                :disabled="!running"
              />

              <small v-if="param.description" class="gcp-note">{{ param.description }}</small>
            </label>

            <small v-if="command.warning" class="gcp-warning">
              <span aria-hidden="true">⚠️</span> {{ command.warning }}
            </small>

            <AppButton
              :variant="command.danger ? 'warning' : 'primary'"
              size="sm"
              :disabled="!running || busyKey !== null || !isReady(command)"
              @click="run(command)"
            >
              {{ busyKey === command.key ? "Envoi…" : "Exécuter" }}
            </AppButton>
          </article>
        </div>
      </section>

      <section class="gcp-group">
        <h3>Réponses du serveur</h3>
        <pre class="sd-logs">{{ output || "Aucune commande envoyée." }}</pre>
      </section>
    </template>
  </div>
</template>

<style scoped>
.gcp {
  display: flex;
  flex-direction: column;
  gap: var(--space-lg);
}

.gcp h3 {
  font-size: 0.98rem;
  margin: 0 0 var(--space-sm);
}

.gcp-off {
  padding: var(--space-sm) var(--space-md);
  border-radius: var(--radius-sm);
  background: var(--accent-warm-bg);
  border-left: 3px solid var(--accent-warm);
  font-size: 0.84rem;
}

.gcp-players-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-md);
}

.gcp-player-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
}

.gcp-player {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-md);
  flex-wrap: wrap;
  padding: var(--space-sm) var(--space-md);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-card);
}

.gcp-player-id {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.gcp-player-id code {
  font-size: 0.72rem;
  color: var(--text-secondary);
}

.gcp-player-actions {
  display: flex;
  gap: var(--space-sm);
  flex-wrap: wrap;
}

.gcp-cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr));
  gap: var(--space-md);
}

.gcp-card {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
  padding: var(--space-md);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-card);
}

/* Une commande irréversible ne doit pas ressembler aux autres : la couleur
   fait hésiter une demi-seconde, ce qui est précisément le but. */
.gcp-card.danger {
  border-color: var(--accent-warm);
}

.gcp-card-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-sm);
}

.gcp-tag {
  padding: 1px 8px;
  border-radius: 999px;
  background: var(--accent-warm-bg);
  color: var(--accent-warm);
  font-size: 0.68rem;
  text-transform: uppercase;
}

.gcp-desc,
.gcp-note {
  color: var(--text-secondary);
  font-size: 0.78rem;
  margin: 0;
}

.gcp-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 0.86rem;
}

.gcp-field > span {
  color: var(--text-secondary);
}

.gcp-field em {
  font-style: normal;
  opacity: 0.7;
}

.gcp-field input,
.gcp-field select {
  background: var(--bg-hover);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  padding: 6px 10px;
}

.gcp-warning {
  display: flex;
  align-items: flex-start;
  gap: var(--space-sm);
  padding: var(--space-sm) var(--space-md);
  border-radius: var(--radius-sm);
  background: var(--accent-warm-bg);
  border-left: 3px solid var(--accent-warm);
  font-size: 0.74rem;
  line-height: 1.5;
}
</style>
