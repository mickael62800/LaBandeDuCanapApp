<script setup lang="ts">
import IconButton from "@/components/atoms/IconButton.vue";

defineProps<{
  title: string;
  description: string;
  position: number;
  total: number;
}>();

defineEmits<{
  previous: [];
  next: [];
}>();
</script>

<template>
  <nav class="profile-nav" :aria-label="`Profil ${title}`">
    <IconButton label="Profil précédent" class="arrow" @click="$emit('previous')">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
        <path d="m15 18-6-6 6-6" />
      </svg>
    </IconButton>

    <div class="profile-copy" aria-live="polite">
      <span class="eyebrow">Profil {{ position }} / {{ total }}</span>
      <strong>{{ title }}</strong>
      <span class="description">{{ description }}</span>
    </div>

    <IconButton label="Profil suivant" class="arrow" @click="$emit('next')">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
        <path d="m9 18 6-6-6-6" />
      </svg>
    </IconButton>
  </nav>
</template>

<style scoped>
.profile-nav {
  display: grid;
  grid-template-columns: 42px minmax(0, 1fr) 42px;
  align-items: center;
  gap: 14px;
  padding: 12px 16px;
  margin-bottom: 16px;
  background: color-mix(in srgb, var(--universe-accent, var(--accent)) 8%, var(--bg-card));
  border: 1px solid color-mix(in srgb, var(--universe-accent, var(--accent)) 32%, var(--border));
  border-radius: var(--radius-lg);
}

.arrow {
  width: 42px;
  height: 42px;
  border-radius: 50%;
  color: var(--text-primary);
}

.arrow svg {
  width: 20px;
  height: 20px;
}

.profile-copy {
  display: flex;
  flex-direction: column;
  align-items: center;
  min-width: 0;
  text-align: center;
}

.eyebrow {
  color: var(--universe-accent, var(--accent));
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.profile-copy strong {
  margin-top: 2px;
  color: var(--text-primary);
  font-size: 17px;
}

.description {
  margin-top: 2px;
  color: var(--text-secondary);
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

@media (max-width: 640px) {
  .description { white-space: normal; }
}
</style>
