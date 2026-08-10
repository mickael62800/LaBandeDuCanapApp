<script setup lang="ts">
import { useRules } from "../../composables/useRules";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useToast } from "../../composables/useToast";
import type { UpdateRuleParams } from "../../types";
import RuleCard from "../organisms/RuleCard.vue";
import AdminPageShell from "../layouts/AdminPageShell.vue";

const { success, error: showError } = useToast();
const { selectedGuildId } = useGuildSelector();
const { rules, loading, toggleRule, updateRule } = useRules();

async function handleSave(params: UpdateRuleParams) {
  try {
    await updateRule(params);
    success("Regle mise a jour avec succes");
  } catch (e) {
    console.error("Erreur mise a jour regle:", e);
    showError("Erreur lors de la mise a jour de la regle");
  }
}

async function handleToggle(rule: Parameters<typeof toggleRule>[0]) {
  try {
    await toggleRule(rule);
    success(rule.enabled ? "Regle desactivee" : "Regle activee");
  } catch (e) {
    console.error("Erreur activation/desactivation regle:", e);
    showError("Erreur lors du changement d'etat de la regle");
  }
}
</script>

<template>
  <AdminPageShell title="Règles de modération" icon="🛡️" class="rules">
    <template #lede>
      Poids et seuils par type de flag. Ces règles alimentent l'automod.
    </template>

    <details class="rules-help">
      <summary>📖 Comment ca marche ?</summary>
      <div class="rules-help-body">
        <p>
          Chaque message Discord (texte ou image) est analyse par le bot et peut
          recevoir un ou plusieurs <strong>flags</strong> (spam, insulte, lien,
          phishing, nsfw, menace, rage, etc.). Chaque flag actif a un
          <strong>poids</strong> defini par sa regle ; les poids des flags
          detectes sont additionnes pour donner un <strong>score total</strong>.
        </p>
        <p>
          Selon ce score, le bot applique automatiquement la sanction qui
          correspond au seuil franchi :
        </p>
        <ul>
          <li><code>score &ge; warn</code> &rarr; avertissement</li>
          <li><code>score &ge; delete</code> &rarr; suppression du message</li>
          <li><code>score &ge; mute</code> &rarr; mute temporaire</li>
          <li><code>score &ge; ban</code> &rarr; bannissement</li>
        </ul>
        <p class="muted small">
          Exemple : <code>spam (poids 2.0) + insult (poids 2.0) = score 4.0</code>
          &rarr; si le seuil <code>delete</code> = 4.0, le message est supprime.
          Une regle peut etre desactivee (toggle) si tu ne veux pas que ce flag
          contribue au score. Si plusieurs regles sont actives sur un meme
          message, c'est le seuil le plus strict qui gagne.
        </p>
        <p class="muted small">
          Les regles par defaut sont creees automatiquement quand le bot rejoint
          un serveur. Tu peux ajuster les poids et seuils via les sliders de
          chaque carte.
        </p>
      </div>
    </details>

    <div v-if="loading" class="loading">Chargement...</div>
    <div v-else-if="!selectedGuildId" class="empty">
      Selectionne un serveur pour voir ses regles.
    </div>
    <div v-else-if="rules.length === 0" class="empty">
      Aucune regle trouvee pour ce serveur. Si le bot vient juste d'etre invite,
      attends quelques secondes puis recharge la page. Sinon redemarre l'API
      ou contacte l'administrateur.
    </div>
    <div v-else class="rules-grid">
      <RuleCard
        v-for="rule in rules"
        :key="rule.id"
        :rule="rule"
        :guild-id="selectedGuildId"
        @toggle="handleToggle"
        @save="handleSave"
      />
    </div>
  </AdminPageShell>
</template>

<style scoped>
.rules-grid {
  display: grid;
  /* 3 cols >=1900px, 2 cols 1400-1900px, 1 col <1400px.
     Les cartes contiennent un formulaire dense (5 sliders + labels),
     en dessous de 700px de largeur effective ca devient illisible. */
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 16px;
}

@media (max-width: 1900px) {
  .rules-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 1400px) {
  .rules-grid {
    grid-template-columns: 1fr;
  }
}

.loading,
.empty {
  color: var(--text-secondary);
  padding: 40px;
  text-align: center;
}

.rules-help {
  margin-bottom: 20px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 12px 16px;
}
.rules-help summary {
  cursor: pointer;
  font-weight: 600;
  user-select: none;
  list-style: none;
}
.rules-help summary::-webkit-details-marker { display: none; }
.rules-help-body {
  margin-top: 12px;
  font-size: 13px;
  line-height: 1.55;
  color: var(--text-secondary);
}
.rules-help-body p { margin: 0 0 10px; }
.rules-help-body ul { margin: 4px 0 10px 20px; padding: 0; }
.rules-help-body li { margin-bottom: 4px; }
.rules-help-body code {
  background: color-mix(in srgb, var(--accent) 8%, transparent);
  padding: 1px 6px;
  border-radius: var(--radius-sm);
  font-family: "JetBrains Mono", monospace;
  font-size: 0.92em;
  color: var(--accent);
}
.rules-help-body .muted { color: var(--text-secondary); }
.rules-help-body .small { font-size: 12px; }
</style>
