<script setup lang="ts">
withDefaults(
  defineProps<{
    title?: string;
    /** Emoji ou caractere prefixe affiche avant le titre. */
    icon?: string;
    /** Largeur de page : constrained (defaut), wide pour dashboards/tables, narrow pour login. */
    width?: "constrained" | "wide" | "narrow";
  }>(),
  { title: "", icon: "", width: "constrained" },
);
</script>

<template>
  <div :class="['admin-page', `page--${width}`]">
    <header v-if="title || icon || $slots.lede || $slots.actions" class="admin-page-header" :class="{ 'has-actions': !!$slots.actions }">
      <div v-if="title || icon || $slots.lede" class="admin-page-title-block">
        <h1 v-if="title || icon" class="admin-page-title">
          <span v-if="icon" class="admin-page-icon">{{ icon }}</span>
          {{ title }}
        </h1>
        <p v-if="$slots.lede" class="admin-page-lede">
          <slot name="lede" />
        </p>
      </div>
      <div v-if="$slots.actions" class="admin-page-actions">
        <slot name="actions" />
      </div>
    </header>

    <slot />
  </div>
</template>

<style scoped>
.admin-page-header {
  margin-bottom: 24px;
  padding-bottom: 18px;
  /* Bordure douce degradee sous le header pour structurer la page (meme
     effet que StatsPage). */
  border-bottom: 1px solid transparent;
  background:
    linear-gradient(to right,
      transparent 0%,
      color-mix(in srgb, var(--universe-accent, var(--accent)) 35%, transparent) 30%,
      color-mix(in srgb, var(--universe-accent, var(--accent)) 35%, transparent) 70%,
      transparent 100%) bottom / 100% 1px no-repeat;
}
.admin-page-header.has-actions {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}
.admin-page-title-block { flex: 1; min-width: 0; }
.admin-page-title {
  margin: 0 0 8px 0;
  font-size: 22px;
  font-weight: 700;
  display: flex;
  align-items: center;
  gap: 10px;
  /* Gradient text + shimmer animation (meme effet que StatsPage h1). */
  background: linear-gradient(
    90deg,
    var(--text-primary) 0%,
    color-mix(in srgb, var(--universe-accent, var(--accent)) 60%, var(--text-primary)) 50%,
    var(--text-primary) 100%
  );
  background-size: 200% auto;
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
  color: transparent;
  animation: admin-page-title-shimmer 10s linear infinite;
  letter-spacing: 0.3px;
}
.admin-page-icon {
  /* Garde la couleur d'origine de l'emoji (sinon il devient transparent
     a cause du background-clip: text). */
  -webkit-text-fill-color: initial;
  color: initial;
}
@keyframes admin-page-title-shimmer {
  0%   { background-position: 200% center; }
  100% { background-position: -200% center; }
}
.admin-page-icon { font-size: 1.2em; flex-shrink: 0; }
.admin-page-lede {
  color: var(--text-secondary);
  margin: 0;
  font-size: 13px;
  line-height: 1.5;
}
.admin-page-lede :deep(code) {
  background: var(--bg-card);
  border: 1px solid var(--border);
  padding: 1px 6px;
  border-radius: 6px;
  font-size: 0.9em;
  font-family: "JetBrains Mono", monospace;
  color: var(--universe-accent, var(--accent));
}
.admin-page-lede :deep(a) {
  color: var(--universe-accent, var(--accent));
  text-decoration: none;
}
.admin-page-lede :deep(a:hover) { text-decoration: underline; }
.admin-page-actions {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-shrink: 0;
  flex-wrap: wrap;
}

@media (max-width: 768px) {
  .admin-page-header.has-actions {
    flex-direction: column;
    align-items: stretch;
  }
  .admin-page-actions { width: 100%; }
  .admin-page-actions :deep(> *) { flex: 1; }
}
</style>
