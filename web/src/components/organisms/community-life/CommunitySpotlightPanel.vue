<script setup lang="ts">
import AppButton from "@/components/atoms/AppButton.vue";
import type { AdminSpotlight } from "@/services/communityAdminService";

interface SpotlightForm {
  user_id: string;
  period: string;
  reason: string;
}

defineProps<{
  items: AdminSpotlight[];
  opened: boolean;
  busy: boolean;
}>();

const form = defineModel<SpotlightForm>("form", { required: true });
const emit = defineEmits<{
  create: [];
  save: [];
  cancel: [];
  remove: [item: AdminSpotlight];
}>();
</script>

<template>
  <section class="cl-sec">
    <div class="cl-actions">
      <AppButton variant="primary" @click="emit('create')">Désigner</AppButton>
    </div>
    <form v-if="opened" class="cl-form" @submit.prevent="emit('save')">
      <h3>Désigner le membre du mois</h3>
      <label>
        Identifiant Discord du membre
        <input v-model="form.user_id" type="text" inputmode="numeric" required />
        <small class="muted">
          Le pseudo et l'avatar sont récupérés côté serveur pour rester à jour.
        </small>
      </label>
      <label>
        Période (facultatif)
        <input v-model="form.period" type="text" placeholder="2026-08" pattern="\d{4}-\d{2}" />
        <small class="muted">Vide = mois en cours. Un seul membre par mois.</small>
      </label>
      <label>
        Pourquoi lui&nbsp;?
        <textarea v-model="form.reason" rows="3" required></textarea>
        <small class="muted">Cette justification est affichée sur le site.</small>
      </label>
      <div class="cl-form-foot">
        <AppButton variant="primary" type="submit" :disabled="busy">Désigner</AppButton>
        <AppButton variant="ghost" @click="emit('cancel')">Annuler</AppButton>
      </div>
    </form>

    <p v-if="!items.length" class="muted">Personne n'a encore été désigné.</p>
    <ul v-else class="cl-list">
      <li v-for="item in items" :key="item.id" class="cl-item">
        <div class="cl-body">
          <div class="cl-line">
            <strong>{{ item.username || item.user_id }}</strong>
            <span class="pill">{{ item.period }}</span>
          </div>
          <p class="muted small">{{ item.reason }}</p>
        </div>
        <div class="cl-item-actions">
          <AppButton variant="danger" size="sm" @click="emit('remove', item)">Retirer</AppButton>
        </div>
      </li>
    </ul>
  </section>
</template>
