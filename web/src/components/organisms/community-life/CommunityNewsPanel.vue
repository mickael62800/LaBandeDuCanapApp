<script setup lang="ts">
import AppButton from "@/components/atoms/AppButton.vue";
import AppCheckbox from "@/components/atoms/AppCheckbox.vue";
import ImagePicker from "@/components/molecules/ImagePicker.vue";
import type { AdminNewsItem, UpsertNewsInput } from "@/services/communityAdminService";

defineProps<{
  items: AdminNewsItem[];
  editing: string | null;
  busy: boolean;
}>();

const form = defineModel<UpsertNewsInput>("form", { required: true });
const emit = defineEmits<{
  create: [];
  edit: [id: string];
  save: [];
  cancel: [];
  remove: [item: AdminNewsItem];
}>();

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR", {
    day: "numeric",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  });
}
</script>

<template>
  <section class="cl-sec">
    <div class="cl-actions">
      <AppButton variant="primary" @click="emit('create')">Nouvelle annonce</AppButton>
    </div>

    <form v-if="editing !== null" class="cl-form" @submit.prevent="emit('save')">
      <h3>{{ editing ? "Modifier l'annonce" : "Nouvelle annonce" }}</h3>
      <label>
        Titre
        <input v-model="form.title" type="text" maxlength="160" required />
      </label>
      <label>
        Texte
        <textarea v-model="form.body" rows="5" required></textarea>
      </label>
      <label>
        Image
        <ImagePicker
          :model-value="form.image_url ?? ''"
          mode="relative"
          @update:model-value="form.image_url = $event || null"
        />
        <small class="muted">
          Chemin relatif seulement, depuis <code>web/public/</code>. Les URL complètes sont refusées.
        </small>
      </label>
      <div class="cl-checks">
        <AppCheckbox v-model="form.is_pinned">Épingler en tête de liste</AppCheckbox>
        <AppCheckbox v-model="form.is_public">Visible par les visiteurs non connectés</AppCheckbox>
      </div>
      <div class="cl-form-foot">
        <AppButton variant="primary" type="submit" :disabled="busy">Enregistrer</AppButton>
        <AppButton variant="ghost" @click="emit('cancel')">Annuler</AppButton>
      </div>
    </form>

    <p v-if="!items.length" class="muted">Aucune annonce pour l'instant.</p>
    <ul v-else class="cl-list">
      <li v-for="item in items" :key="item.id" class="cl-item">
        <img v-if="item.image_url" :src="item.image_url" alt="" class="cl-thumb" />
        <div class="cl-body">
          <div class="cl-line">
            <strong>{{ item.title }}</strong>
            <span v-if="item.is_pinned" class="pill">épinglée</span>
            <span v-if="!item.is_public" class="pill warn">non publique</span>
          </div>
          <p class="muted small">{{ item.body.slice(0, 160) }}</p>
          <span class="muted small">{{ formatDate(item.published_at) }}</span>
        </div>
        <div class="cl-item-actions">
          <AppButton variant="ghost" size="sm" @click="emit('edit', item.id)">Modifier</AppButton>
          <AppButton variant="danger" size="sm" @click="emit('remove', item)">Supprimer</AppButton>
        </div>
      </li>
    </ul>
  </section>
</template>
