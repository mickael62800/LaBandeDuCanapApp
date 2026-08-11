<script setup lang="ts">
import AppButton from "@/components/atoms/AppButton.vue";
import type { AdminLfgPost } from "@/services/communityAdminService";

defineProps<{
  items: AdminLfgPost[];
  busy: boolean;
}>();

const emit = defineEmits<{
  close: [item: AdminLfgPost];
  remove: [item: AdminLfgPost];
}>();

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR", {
    day: "numeric",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function isExpired(iso: string): boolean {
  return new Date(iso) <= new Date();
}
</script>

<template>
  <section class="cl-sec">
    <p class="muted small">
      Ces annonces sont publiées par les membres depuis le site. Le staff ferme celles qui
      traînent et retire les contenus abusifs.
    </p>
    <p v-if="!items.length" class="muted">Aucune annonce.</p>
    <ul v-else class="cl-list">
      <li v-for="item in items" :key="item.id" class="cl-item">
        <div class="cl-body">
          <div class="cl-line">
            <strong>{{ item.author_name || item.author_id }}</strong>
            <span class="pill">{{ item.game }}</span>
            <span v-if="!item.is_open" class="pill warn">fermée</span>
            <span v-else-if="isExpired(item.expires_at)" class="pill warn">expirée</span>
          </div>
          <p v-if="item.description" class="muted small">{{ item.description }}</p>
          <span class="muted small">
            Cherche {{ item.slots }} joueur(s) · {{ item.when_text }} ·
            {{ item.interested.length }} intéressé(s) · expire le {{ formatDate(item.expires_at) }}
          </span>
        </div>
        <div class="cl-item-actions">
          <AppButton
            v-if="item.is_open"
            variant="ghost"
            size="sm"
            :disabled="busy"
            @click="emit('close', item)"
          >Fermer</AppButton>
          <AppButton variant="danger" size="sm" @click="emit('remove', item)">Supprimer</AppButton>
        </div>
      </li>
    </ul>
  </section>
</template>
