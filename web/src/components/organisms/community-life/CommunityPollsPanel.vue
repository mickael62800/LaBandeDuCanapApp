<script setup lang="ts">
import AppButton from "@/components/atoms/AppButton.vue";
import AppCheckbox from "@/components/atoms/AppCheckbox.vue";
import type { AdminPoll, CreatePollInput } from "@/services/communityAdminService";

defineProps<{
  items: AdminPoll[];
  opened: boolean;
  busy: boolean;
  optionsValid: boolean;
}>();

const form = defineModel<CreatePollInput>("form", { required: true });
const emit = defineEmits<{
  create: [];
  save: [];
  cancel: [];
  close: [item: AdminPoll];
  remove: [item: AdminPoll];
}>();

function formatDay(iso: string): string {
  return new Date(iso).toLocaleDateString("fr-FR", { day: "numeric", month: "long" });
}
</script>

<template>
  <section class="cl-sec">
    <div class="cl-actions">
      <AppButton variant="primary" @click="emit('create')">Nouveau sondage</AppButton>
    </div>

    <form v-if="opened" class="cl-form" @submit.prevent="emit('save')">
      <h3>Nouveau sondage</h3>
      <label>
        Question
        <input v-model="form.question" type="text" maxlength="200" required />
      </label>
      <label>
        Précision (facultatif)
        <textarea v-model="form.description" rows="2"></textarea>
      </label>
      <label>
        Clôture
        <input v-model="form.closes_at" type="datetime-local" required />
      </label>
      <fieldset class="cl-options">
        <legend>Choix</legend>
        <div v-for="(option, index) in form.options" :key="index" class="cl-option">
          <input
            v-model="option.label"
            type="text"
            maxlength="120"
            :placeholder="`Choix ${index + 1}`"
          />
          <button
            v-if="form.options.length > 2"
            type="button"
            class="btn small"
            @click="form.options.splice(index, 1)"
          >✕</button>
        </div>
        <AppButton
          v-if="form.options.length < 10"
          variant="ghost"
          size="sm"
          @click="form.options.push({ label: '' })"
        >Ajouter un choix</AppButton>
        <p v-if="!optionsValid" class="muted small">Il faut au moins deux choix renseignés.</p>
      </fieldset>
      <AppCheckbox v-model="form.is_public">Visible par les visiteurs non connectés</AppCheckbox>
      <div class="cl-form-foot">
        <AppButton variant="primary" type="submit" :disabled="busy || !optionsValid">
          Ouvrir le sondage
        </AppButton>
        <AppButton variant="ghost" @click="emit('cancel')">Annuler</AppButton>
      </div>
    </form>

    <p v-if="!items.length" class="muted">Aucun sondage.</p>
    <ul v-else class="cl-list">
      <li v-for="item in items" :key="item.id" class="cl-item bloc">
        <div class="cl-body">
          <div class="cl-line">
            <strong>{{ item.question }}</strong>
            <span v-if="!item.is_open" class="pill warn">clos</span>
          </div>
          <ul class="cl-bars">
            <li v-for="option in item.options" :key="option.id">
              <span class="cl-bar-label">{{ option.label }}</span>
              <span class="cl-bar">
                <i :style="{ width: `${option.share}%`, background: `#${option.color}` }"></i>
              </span>
              <span class="muted small">{{ option.votes }} · {{ option.share }} %</span>
            </li>
          </ul>
          <span class="muted small">
            {{ item.total_votes }} vote(s) · clôture le {{ formatDay(item.closes_at) }}
          </span>
        </div>
        <div class="cl-item-actions">
          <AppButton
            v-if="item.is_open"
            variant="ghost"
            size="sm"
            :disabled="busy"
            @click="emit('close', item)"
          >Clore</AppButton>
          <AppButton variant="danger" size="sm" @click="emit('remove', item)">Supprimer</AppButton>
        </div>
      </li>
    </ul>
  </section>
</template>
