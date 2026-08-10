<script setup lang="ts">
import ActionButton from "../atoms/ActionButton.vue";
import LevelRoleTiersCard from "../organisms/LevelRoleTiersCard.vue";
import LevelsManualXpCard from "../organisms/LevelsManualXpCard.vue";
</script>

<template>
  <!-- Contenu d'onglet : l'en-tete de page appartient a `LevelsHubPage`. -->
  <div class="levels-config-tab">
    <!--
      Le reste de la configuration vit sur la page du composant, pas ici.
      Cet encart le dit ET y mene : la mention seule laissait chercher la
      page « progression-bot » a la main, sans dire ou elle se trouve.
    -->
    <section class="lc-ailleurs">
      <div class="lc-texte">
        <h2>Les autres réglages d'XP</h2>
        <p>
          Gains par message et par minute de vocal, cooldown anti-farm,
          message de level-up, salons et rôles ignorés — sur la page du
          composant Progression.
        </p>
      </div>
      <ActionButton to="/component-config?bot=progression-bot" variant="primary">
        Ouvrir la configuration
      </ActionButton>
    </section>

    <div class="grid">
      <LevelRoleTiersCard />
      <LevelsManualXpCard />
    </div>
  </div>
</template>

<style scoped>
@import "./_admin-page-shared.css";
.grid { display: grid; grid-template-columns: 1fr; gap: 16px; }

.lc-ailleurs {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-left: 3px solid var(--accent);
  border-radius: var(--radius-md);
  padding: 16px 20px;
  margin-bottom: 16px;
}

.lc-texte { flex: 1 1 320px; }
.lc-texte h2 { font-size: 15px; color: var(--text-primary); margin-bottom: 4px; }
.lc-texte p { font-size: 13px; color: var(--text-secondary); line-height: 1.5; }
</style>
