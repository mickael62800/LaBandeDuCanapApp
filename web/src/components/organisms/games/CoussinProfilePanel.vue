<script setup lang="ts">
import { computed } from "vue";
import type { GameCard } from "@/games/catalog";
import type { CoussinCombat, CoussinFile } from "@/services/gamesService";

const props = defineProps<{
  game: GameCard;
  file: CoussinFile | null;
  error: string | null;
}>();

const CLASSES: Record<string, { nom: string; emoji: string; trait: string }> = {
  ecraseur: { nom: "Écraseur", emoji: "🪑", trait: "S'assoit sans regarder. Ça fait mal aux deux." },
  ressort: { nom: "Ressort", emoji: "🤸", trait: "Rebondit d'un accoudoir à l'autre." },
  piegeur: { nom: "Piégeur", emoji: "🪡", trait: "Place les coussins et fouille sous ceux des autres." },
  couette: { nom: "Couette", emoji: "🛌", trait: "Roulé dans la couette. Ne bougera plus." },
};

const ITEMS: Record<string, string> = {
  rage: "🧱 Coussin Plombé",
  mindgame: "👁️ Œil sous le Plaid",
  explosion: "🍟 Renversement de Chips",
  double_coup: "🛋️ Double Coussin",
  surprise: "🪶 Bataille d'Oreillers",
  coup_traitre: "📌 Punaise dans le Coussin",
  inversion: "🔄 Retourne le Canapé",
};

const playerClass = computed(() => {
  const key = props.file?.profile.class ?? "";
  return CLASSES[key] ?? {
    nom: "Debout",
    emoji: "🧍",
    trait: "Tu n'as pas encore choisi ta place sur le canapé",
  };
});

const winRate = computed(() => {
  const profile = props.file?.profile;
  if (!profile) return null;
  const total = profile.total_wins + profile.total_losses + profile.total_draws;
  return total === 0 ? null : Math.round((profile.total_wins / total) * 100);
});

const fmtCoins = (value: number) => value.toLocaleString("fr-FR");
const fmtDate = (iso: string) => new Date(iso).toLocaleString("fr-FR", {
  day: "numeric",
  month: "short",
  hour: "2-digit",
  minute: "2-digit",
});
const itemLabel = (key: string) => ITEMS[key] ?? `📦 ${key}`;

function opponentContext(combat: CoussinCombat): { me: string; name: string } {
  const amAttacker = combat.attacker_name === props.file?.profile.username;
  return amAttacker
    ? { me: combat.attacker_id, name: combat.defender_name }
    : { me: combat.defender_id, name: combat.attacker_name };
}

function won(combat: CoussinCombat): boolean | null {
  if (!combat.winner_id) return null;
  return combat.winner_id === opponentContext(combat).me;
}
</script>

<template>
  <section class="jx-block">
    <h2>{{ game.emoji }} {{ game.nom }}</h2>
    <p v-if="error" class="jx-alerte">{{ error }}</p>
    <p v-else-if="!file" class="jx-hint">Chargement de ta fiche…</p>

    <template v-else>
      <div class="cd-fiche">
        <div class="cd-classe">
          <span class="cd-classe-emoji" aria-hidden="true">{{ playerClass.emoji }}</span>
          <div>
            <div class="cd-classe-nom">{{ playerClass.nom }}</div>
            <div class="cd-classe-trait">{{ playerClass.trait }}</div>
          </div>
        </div>
        <div class="cd-identite">
          <strong>{{ file.profile.username || "Toi" }}</strong>
          <span v-if="file.profile.title" class="cd-titre">« {{ file.profile.title }} »</span>
          <span class="cd-niveau">Niveau {{ file.profile.level }}</span>
        </div>
        <div class="cd-pv">
          <div class="cd-pv-ligne">
            <span>Confort</span>
            <span>{{ file.profile.hp_current }} / {{ file.profile.hp_max }}</span>
          </div>
          <div class="cd-jauge">
            <i :style="{ width: `${Math.max(0, Math.round((file.profile.hp_current / Math.max(1, file.profile.hp_max)) * 100))}%` }"></i>
          </div>
        </div>
        <ul class="cd-stats">
          <li><span>🧱 Impact</span><b>{{ file.profile.atk }}</b></li>
          <li><span>🪶 Moelleux</span><b>{{ file.profile.def }}</b></li>
          <li><span>✨ Expérience</span><b>{{ file.profile.xp }}</b></li>
          <li v-if="file.profile.stat_points > 0" class="cd-dispo">
            <span>🎯 Points à répartir</span><b>{{ file.profile.stat_points }}</b>
          </li>
        </ul>
        <p v-if="file.profile.stat_points > 0" class="jx-vide">
          Tu as {{ file.profile.stat_points }} point(s) à placer. Utilise <code>/train</code> sur Discord.
        </p>
      </div>

      <h3 class="cd-sous-titre">Ton palmarès</h3>
      <ul class="cd-palmares">
        <li class="gagne"><b>{{ file.profile.total_wins }}</b><span>fois assis dessus</span></li>
        <li class="perdu"><b>{{ file.profile.total_losses }}</b><span>fois piégé</span></li>
        <li><b>{{ file.profile.total_draws }}</b><span>matchs nuls</span></li>
        <li v-if="winRate !== null"><b>{{ winRate }} %</b><span>de réussite</span></li>
        <li><b>{{ fmtCoins(file.profile.total_stolen) }}</b><span>coins trouvés sous les coussins</span></li>
        <li><b>{{ file.profile.chaos_events }}</b><span>fois où le salon a dégénéré</span></li>
        <li v-if="file.profile.cowardice_count > 0" class="perdu">
          <b>{{ file.profile.cowardice_count }}</b><span>fois resté debout</span>
        </li>
      </ul>

      <h3 class="cd-sous-titre">Sous ton coussin</h3>
      <p v-if="!file.items.length" class="jx-vide">
        Rien de planqué. Le coffre à coussins s'ouvre avec <code>/shop</code>.
      </p>
      <ul v-else class="cd-objets">
        <li v-for="item in file.items" :key="item.item_key" class="cd-objet">
          <span>{{ itemLabel(item.item_key) }}</span><b>×{{ item.quantity }}</b>
        </li>
      </ul>

      <h3 class="cd-sous-titre">Tes dernières bagarres</h3>
      <p v-if="!file.combats.length" class="jx-vide">
        Aucune bagarre pour l'instant. Le premier coussin se glisse avec <code>/coussin</code> sur Discord.
      </p>
      <ul v-else class="cd-combats">
        <li
          v-for="combat in file.combats"
          :key="combat.id"
          class="cd-combat"
          :class="{ gagne: won(combat) === true, perdu: won(combat) === false }"
        >
          <div class="cd-combat-tete">
            <span class="cd-issue">{{ won(combat) === true ? "Victoire" : won(combat) === false ? "Défaite" : "Égalité" }}</span>
            <span class="cd-adversaire">contre {{ opponentContext(combat).name }}</span>
            <span v-if="combat.coins_transferred" class="cd-mise">
              {{ won(combat) ? "+" : "−" }}{{ fmtCoins(Math.abs(combat.coins_transferred)) }} coins
            </span>
            <span v-if="combat.resolved_at" class="cd-quand">{{ fmtDate(combat.resolved_at) }}</span>
          </div>
          <div v-if="combat.attacker_roll !== null" class="cd-des">
            🎲 {{ combat.attacker_name }} {{ combat.attacker_roll }} — {{ combat.defender_name }} {{ combat.defender_roll }}
          </div>
          <div v-if="combat.special_attack" class="cd-special">💫 {{ combat.special_attack }}</div>
          <div v-if="combat.chaos_event" class="cd-chaos">🌀 {{ combat.chaos_event }}</div>
          <p v-if="combat.result_message" class="cd-recit">{{ combat.result_message }}</p>
        </li>
      </ul>

      <h3 class="cd-sous-titre">Le classement</h3>
      <ol class="jx-rangs">
        <li
          v-for="(rank, index) in file.ranking"
          :key="rank.username + index"
          class="jx-rang"
          :class="{ moi: rank.username === file.profile.username }"
        >
          <span class="jx-place">{{ index + 1 }}</span>
          <span class="jx-nom">{{ rank.username || "Un membre" }}</span>
          <span class="jx-coins">niv. {{ rank.level }}</span>
        </li>
      </ol>
    </template>
  </section>
</template>
