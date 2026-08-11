<script setup lang="ts">
import AppButton from "../atoms/AppButton.vue";
import AppInput from "@/components/atoms/AppInput.vue";
import { computed, reactive, watch } from "vue";
import { useWelcome } from "@/composables/useWelcome";
import { useGuildSelector } from "@/composables/useGuildSelector";
import AppToggle from "@/components/atoms/AppToggle.vue";
import ChannelSelect from "@/components/atoms/ChannelSelect.vue";
import VoiceChannelSelect from "@/components/atoms/VoiceChannelSelect.vue";
import RoleSelect from "@/components/atoms/RoleSelect.vue";
import IdsListPickerField from "@/components/molecules/IdsListPickerField.vue";
import AppTextarea from "@/components/atoms/AppTextarea.vue";
import ImagePicker from "@/components/molecules/ImagePicker.vue";

const { config, saving, saveConfig, publishRules } = useWelcome();
const { guildIdFilter } = useGuildSelector();

const draft = reactive({
  // Welcome
  welcome_enabled: false,
  welcome_channel_id: "",
  welcome_message: "",
  welcome_title: "",
  welcome_embed_color: "#5865F2",
  welcome_image_url: "",
  welcome_footer_text: "",
  welcome_dm_enabled: false,
  welcome_dm_message: "",
  // Leave
  leave_enabled: false,
  leave_channel_id: "",
  leave_message: "",
  leave_title: "",
  leave_image_url: "",
  leave_footer_text: "",
  leave_embed_color: "#e74c3c",
  // Rules
  rules_enabled: false,
  rules_channel_id: "",
  rules_message: "",
  rules_role_id: "",
  rules_button_label: "",
  rules_embed_color: "#5865f2",
  age_check_enabled: false,
  age_minimum: 20,
  unverified_role_id: "",
  age_modal_question: "",
  age_ban_message: "",
  age_min: 5,
  age_max: 120,
  age_ban_days_per_year: 365,
  age_ban_log_channel_id: "",
  // Counter
  counter_enabled: false,
  counter_channel_id: "",
  counter_format: "",
  // Voice counter
  voice_counter_enabled: false,
  voice_counter_channel_id: "",
  voice_counter_format: "",
  // Anniversary
  anniversary_enabled: false,
  anniversary_channel_id: "",
  anniversary_message: "",
  anniversary_title: "",
  anniversary_image_url: "",
  anniversary_footer_text: "",
  // Rejoin
  rejoin_message: "",
  rejoin_title: "",
  rejoin_image_url: "",
  rejoin_footer_text: "",
});

watch(
  config,
  (c) => {
    if (!c) return;
    draft.welcome_enabled = c.welcome_enabled;
    draft.welcome_channel_id = c.welcome_channel_id ?? "";
    draft.welcome_message = c.welcome_message;
    draft.welcome_title = c.welcome_title;
    draft.welcome_embed_color = c.welcome_embed_color || "#5865F2";
    draft.welcome_image_url = c.welcome_image_url;
    draft.welcome_footer_text = c.welcome_footer_text;
    draft.welcome_dm_enabled = c.welcome_dm_enabled;
    draft.welcome_dm_message = c.welcome_dm_message;
    draft.leave_enabled = c.leave_enabled;
    draft.leave_channel_id = c.leave_channel_id ?? "";
    draft.leave_message = c.leave_message;
    draft.leave_title = c.leave_title;
    draft.leave_image_url = c.leave_image_url;
    draft.leave_footer_text = c.leave_footer_text;
    draft.leave_embed_color = c.leave_embed_color || "#e74c3c";
    draft.rules_enabled = c.rules_enabled;
    draft.rules_channel_id = c.rules_channel_id ?? "";
    draft.rules_message = c.rules_message;
    draft.rules_role_id = c.rules_role_id ?? "";
    draft.rules_button_label = c.rules_button_label;
    draft.rules_embed_color = c.rules_embed_color || "#5865f2";
    draft.age_check_enabled = c.age_check_enabled;
    draft.age_minimum = c.age_minimum;
    draft.unverified_role_id = c.unverified_role_id ?? "";
    draft.age_modal_question = c.age_modal_question;
    draft.age_ban_message = c.age_ban_message;
    draft.age_min = c.age_min;
    draft.age_max = c.age_max;
    draft.age_ban_days_per_year = c.age_ban_days_per_year;
    draft.age_ban_log_channel_id = c.age_ban_log_channel_id ?? "";
    draft.counter_enabled = c.counter_enabled;
    draft.counter_channel_id = c.counter_channel_id ?? "";
    draft.counter_format = c.counter_format;
    draft.voice_counter_enabled = c.voice_counter_enabled;
    draft.voice_counter_channel_id = c.voice_counter_channel_id ?? "";
    draft.voice_counter_format = c.voice_counter_format;
    draft.anniversary_enabled = c.anniversary_enabled;
    draft.anniversary_channel_id = c.anniversary_channel_id ?? "";
    draft.anniversary_message = c.anniversary_message;
    draft.anniversary_title = c.anniversary_title;
    draft.anniversary_image_url = c.anniversary_image_url;
    draft.anniversary_footer_text = c.anniversary_footer_text;
    draft.rejoin_message = c.rejoin_message;
    draft.rejoin_title = c.rejoin_title;
    draft.rejoin_image_url = c.rejoin_image_url;
    draft.rejoin_footer_text = c.rejoin_footer_text;
  },
  { immediate: true },
);

const previewWelcomeText = computed(() => {
  return draft.welcome_message
    .replace(/\{user\}/g, "@NouveauMembre")
    .replace(/\{server\}/g, "Mon Serveur")
    .replace(/\{count\}/g, "42");
});

async function onSave() {
  await saveConfig({
    welcome_enabled: draft.welcome_enabled,
    welcome_channel_id: draft.welcome_channel_id || null,
    welcome_message: draft.welcome_message,
    welcome_title: draft.welcome_title,
    welcome_embed_color: draft.welcome_embed_color,
    welcome_image_url: draft.welcome_image_url,
    welcome_footer_text: draft.welcome_footer_text,
    welcome_dm_enabled: draft.welcome_dm_enabled,
    welcome_dm_message: draft.welcome_dm_message,
    leave_enabled: draft.leave_enabled,
    leave_channel_id: draft.leave_channel_id || null,
    leave_message: draft.leave_message,
    leave_title: draft.leave_title,
    leave_image_url: draft.leave_image_url,
    leave_footer_text: draft.leave_footer_text,
    leave_embed_color: draft.leave_embed_color,
    rules_enabled: draft.rules_enabled,
    rules_channel_id: draft.rules_channel_id || null,
    rules_message: draft.rules_message,
    rules_role_id: draft.rules_role_id || null,
    rules_button_label: draft.rules_button_label,
    rules_embed_color: draft.rules_embed_color,
    age_check_enabled: draft.age_check_enabled,
    age_minimum: draft.age_minimum,
    unverified_role_id: draft.unverified_role_id || null,
    age_modal_question: draft.age_modal_question,
    age_ban_message: draft.age_ban_message,
    age_min: draft.age_min,
    age_max: draft.age_max,
    age_ban_days_per_year: draft.age_ban_days_per_year,
    age_ban_log_channel_id: draft.age_ban_log_channel_id || null,
    counter_enabled: draft.counter_enabled,
    counter_channel_id: draft.counter_channel_id || null,
    counter_format: draft.counter_format,
    voice_counter_enabled: draft.voice_counter_enabled,
    voice_counter_channel_id: draft.voice_counter_channel_id || null,
    voice_counter_format: draft.voice_counter_format,
    anniversary_enabled: draft.anniversary_enabled,
    anniversary_channel_id: draft.anniversary_channel_id || null,
    anniversary_message: draft.anniversary_message,
    anniversary_title: draft.anniversary_title,
    anniversary_image_url: draft.anniversary_image_url,
    anniversary_footer_text: draft.anniversary_footer_text,
    rejoin_message: draft.rejoin_message,
    rejoin_title: draft.rejoin_title,
    rejoin_image_url: draft.rejoin_image_url,
    rejoin_footer_text: draft.rejoin_footer_text,
  });
}
</script>

<template>
  <form class="welcome-form" @submit.prevent="onSave">
    <!-- Welcome -->
    <fieldset class="card">
      <legend>
        <label class="toggle-row">
          <AppToggle v-model="draft.welcome_enabled" />
          <span>Message de bienvenue actif</span>
        </label>
      </legend>
      <div class="grid" :class="{ 'grid--disabled': !draft.welcome_enabled }">
        <label>Salon
          <ChannelSelect v-model="draft.welcome_channel_id" :guild-id="guildIdFilter ?? null" />
        </label>
        <label>Titre embed
          <AppInput v-model="draft.welcome_title" placeholder="Bienvenue !" />
        </label>
        <label>Couleur (hex)
          <input v-model="draft.welcome_embed_color" type="color" />
        </label>
        <label>Image
          <ImagePicker v-model="draft.welcome_image_url" />
        </label>
        <label class="full">Message
          <AppTextarea v-model="draft.welcome_message" :rows="6" />
        </label>
        <label class="full">Footer
          <AppInput v-model="draft.welcome_footer_text" />
        </label>
      </div>

      <details class="dm-details">
        <summary>DM de bienvenue (optionnel)</summary>
        <label class="toggle-row">
          <AppToggle v-model="draft.welcome_dm_enabled" />
          <span>Activer le DM</span>
        </label>
        <label class="full">Message DM
          <AppTextarea v-model="draft.welcome_dm_message" :rows="6" />
        </label>
      </details>

      <details v-if="draft.welcome_enabled" class="preview">
        <summary>👁️ Aperçu</summary>
        <div class="preview-stack">
          <!-- Un seul message : l'image est dans l'embed, sous le texte. -->
          <div class="preview-embed" :style="{ borderLeftColor: draft.welcome_embed_color }">
            <strong v-if="draft.welcome_title">{{ draft.welcome_title }}</strong>
            <p>{{ previewWelcomeText }}</p>
            <figure v-if="draft.welcome_image_url" class="preview-image">
              <img :src="draft.welcome_image_url" alt="Bannière de bienvenue" loading="lazy" />
            </figure>
            <small v-if="draft.welcome_footer_text">{{ draft.welcome_footer_text }}</small>
          </div>
        </div>
      </details>
    </fieldset>

    <!-- Verification gate (rules) -->
    <fieldset class="card">
      <legend>
        <label class="toggle-row">
          <AppToggle v-model="draft.rules_enabled" />
          <span>🔒 Verification gate (lecture des règles)</span>
        </label>
      </legend>
      <p class="hint">
        Affiche un bouton « J'ai lu les règles » dans le salon dédié.
        Le ou les rôles configurés sont attribués après acceptation.
        Après avoir enregistré, clique sur « Publier le règlement » pour
        (re)poster le message avec le bouton dans le salon.
      </p>
      <div class="grid" :class="{ 'grid--disabled': !draft.rules_enabled }">
        <label>Salon des règles
          <ChannelSelect v-model="draft.rules_channel_id" :guild-id="guildIdFilter ?? null" />
        </label>
        <label>Rôles attribués (plusieurs possibles)
          <IdsListPickerField v-model="draft.rules_role_id" :guild-id="guildIdFilter ?? null" kind="role" />
        </label>
        <label>Texte du bouton
          <AppInput v-model="draft.rules_button_label" placeholder="J'ai lu les règles" />
        </label>
        <label>Couleur du panneau (hex)
          <AppInput v-model="draft.rules_embed_color" placeholder="5865f2" />
        </label>
        <label class="full">Message
          <AppTextarea v-model="draft.rules_message" :rows="6" />
        </label>

        <!-- Vérification d'âge -->
        <label class="full toggle-row">
          <span>Vérification d'âge au règlement</span>
          <AppToggle v-model="draft.age_check_enabled" />
        </label>
        <p class="hint full">
          À l'arrivée, le membre reçoit le rôle « Membre temporaire » (qui ne voit
          que le règlement). En cliquant sur « J'accepte », un formulaire lui demande
          son âge. S'il a moins de l'âge minimum, il est banni jusqu'à l'atteindre
          ((âge min − âge) ans). Sinon il obtient le rôle Membre.
          Placeholders du message de ban : <code>{min}</code>, <code>{annees}</code>.
        </p>
        <template v-if="draft.age_check_enabled">
          <label>Âge minimum requis
            <AppInput v-model.number="draft.age_minimum" type="number" :min="13" :max="99" />
          </label>
          <label>Rôle « Membre temporaire » (à l'arrivée)
            <RoleSelect v-model="draft.unverified_role_id" :guild-id="guildIdFilter ?? null" />
          </label>
          <label class="full">Question du formulaire
            <AppInput v-model="draft.age_modal_question" placeholder="Quel âge as-tu ? (en chiffres)" />
          </label>
          <label class="full">Message de bannissement
            <AppTextarea v-model="draft.age_ban_message" :rows="3" placeholder="Tu dois avoir au moins {min} ans. Reviens dans {annees} an(s)." />
          </label>
          <label>Âge minimum saisissable
            <AppInput v-model.number="draft.age_min" type="number" :min="0" :max="120" />
          </label>
          <label>Âge maximum saisissable
            <AppInput v-model.number="draft.age_max" type="number" :min="0" :max="200" />
          </label>
          <label>Jours de ban par année manquante
            <AppInput v-model.number="draft.age_ban_days_per_year" type="number" :min="1" :max="366" />
          </label>
          <label>Salon de log des bans d'âge
            <ChannelSelect v-model="draft.age_ban_log_channel_id" :guild-id="guildIdFilter ?? null" />
          </label>
        </template>

        <div class="full">
          <button type="button" class="publish-btn" :disabled="!draft.rules_enabled || !draft.rules_channel_id" @click="publishRules">
            📢 Publier le règlement
          </button>
          <small class="hint">Poste (ou remplace) le message avec le bouton dans le salon choisi. Enregistre d'abord tes changements.</small>
        </div>
      </div>
    </fieldset>

    <!-- Compteur de membres -->
    <fieldset class="card">
      <legend>
        <label class="toggle-row">
          <AppToggle v-model="draft.counter_enabled" />
          <span>🔢 Compteur de membres</span>
        </label>
      </legend>
      <div class="grid" :class="{ 'grid--disabled': !draft.counter_enabled }">
        <label>Salon (vocal)
          <VoiceChannelSelect v-model="draft.counter_channel_id" :guild-id="guildIdFilter ?? null" />
        </label>
        <label>Format
          <AppInput v-model="draft.counter_format" placeholder="👥 {count} membres" />
        </label>
      </div>
    </fieldset>

    <!-- Compteur vocal -->
    <fieldset class="card">
      <legend>
        <label class="toggle-row">
          <AppToggle v-model="draft.voice_counter_enabled" />
          <span>🔊 Compteur de membres en vocal</span>
        </label>
      </legend>
      <p class="hint">
        Renomme un salon (idéalement verrouillé / lecture seule) avec le nombre
        de membres actuellement connectés en vocal. Utilise {count}.
      </p>
      <div class="grid" :class="{ 'grid--disabled': !draft.voice_counter_enabled }">
        <label>Salon (vocal)
          <VoiceChannelSelect v-model="draft.voice_counter_channel_id" :guild-id="guildIdFilter ?? null" />
        </label>
        <label>Format
          <AppInput v-model="draft.voice_counter_format" placeholder="En Vocal : {count}" />
        </label>
      </div>
    </fieldset>

    <!-- Anniversaire -->
    <fieldset class="card">
      <legend>
        <label class="toggle-row">
          <AppToggle v-model="draft.anniversary_enabled" />
          <span>🎂 Anniversaire d'arrivée</span>
        </label>
      </legend>
      <div class="grid" :class="{ 'grid--disabled': !draft.anniversary_enabled }">
        <label>Salon
          <ChannelSelect v-model="draft.anniversary_channel_id" :guild-id="guildIdFilter ?? null" />
        </label>
        <label>Titre
          <AppInput v-model="draft.anniversary_title" />
        </label>
        <label>Image
          <ImagePicker v-model="draft.anniversary_image_url" />
        </label>
        <label class="full">Message
          <AppTextarea v-model="draft.anniversary_message" :rows="6" />
        </label>
        <label class="full">Footer
          <AppInput v-model="draft.anniversary_footer_text" />
        </label>
      </div>
    </fieldset>

    <!-- Départ -->
    <fieldset class="card">
      <legend>
        <label class="toggle-row">
          <AppToggle v-model="draft.leave_enabled" />
          <span>👋 Message de départ</span>
        </label>
      </legend>
      <div class="grid" :class="{ 'grid--disabled': !draft.leave_enabled }">
        <label>Salon
          <ChannelSelect v-model="draft.leave_channel_id" :guild-id="guildIdFilter ?? null" />
        </label>
        <label>Titre
          <AppInput v-model="draft.leave_title" />
        </label>
        <label>Image
          <ImagePicker v-model="draft.leave_image_url" />
        </label>
        <label>Couleur embed (hex)
          <AppInput v-model="draft.leave_embed_color" placeholder="e74c3c" />
        </label>
        <label class="full">Message
          <AppTextarea v-model="draft.leave_message" :rows="6" />
        </label>
        <label class="full">Footer
          <AppInput v-model="draft.leave_footer_text" />
        </label>
      </div>
    </fieldset>

    <!-- Rejoin -->
    <fieldset class="card">
      <legend>🔁 Rejoin (membre déjà venu)</legend>
      <p class="hint">
        Affiché à la place du message de bienvenue si le membre était déjà
        passé sur le serveur.
      </p>
      <div class="grid">
        <label>Titre
          <AppInput v-model="draft.rejoin_title" />
        </label>
        <label>Image
          <ImagePicker v-model="draft.rejoin_image_url" />
        </label>
        <label class="full">Message
          <AppTextarea v-model="draft.rejoin_message" :rows="6" />
        </label>
        <label class="full">Footer
          <AppInput v-model="draft.rejoin_footer_text" />
        </label>
      </div>
    </fieldset>

    <div class="actions">
      <AppButton variant="primary" type="submit" :disabled="saving">
        {{ saving ? "Enregistrement…" : "Enregistrer" }}
      </AppButton>
    </div>
  </form>
</template>

<style scoped src="../../styles/welcome-form.css"></style>
