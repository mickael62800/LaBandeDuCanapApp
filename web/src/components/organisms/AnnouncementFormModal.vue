<script setup lang="ts">
import AppCheckbox from "../atoms/AppCheckbox.vue";
import AppSelect from "@/components/atoms/AppSelect.vue";
import { errMsg } from "@/utils/errMsg";
import AppInput from "@/components/atoms/AppInput.vue";
import { ref, computed, watch } from "vue";
import { useToast } from "@/composables/useToast";
import {
  announcementsService,
  type ScheduledAnnouncement,
  type CreateAnnouncementBody,
  type RecurrenceType,
  type ContentType,
  type AnnouncementButton,
} from "@/services/announcementsService";
import { botConfigService } from "@/services/botConfigService";
import type { DiscordTextChannel } from "@/services/guildsService";
import type { DiscordRole } from "@/types";
import AppModal from "../atoms/AppModal.vue";
import AppButton from "../atoms/AppButton.vue";
import NumberInputWithUnit from "../atoms/NumberInputWithUnit.vue";
import AppTextarea from "../atoms/AppTextarea.vue";
import ImagePicker from "@/components/molecules/ImagePicker.vue";
import AnnouncementButtonsEditor from "./announcement-form/AnnouncementButtonsEditor.vue";

const props = defineProps<{
  visible: boolean;
  /** null = mode "create", sinon mode "edit" */
  target: ScheduledAnnouncement | null;
  channels: DiscordTextChannel[];
  roles: DiscordRole[];
  guildId: string;
}>();

const emit = defineEmits<{
  close: [];
  saved: [];
}>();

const { error: toastErr, success: toastOk } = useToast();

interface FormState {
  name: string;
  recurrence_type: RecurrenceType;
  recurrence_hour: number;
  recurrence_minute: number;
  recurrence_day_of_week: number | null;
  recurrence_day_of_month: number | null;
  recurrence_month: number;
  scheduled_at: string;
  end_date: string;
  content_type: ContentType;
  content_text: string;
  embed_title: string;
  embed_color_hex: string;
  embed_image_url: string;
  embed_thumbnail_url: string;
  embed_footer_text: string;
  mention_everyone: boolean;
  mention_here: boolean;
  selected_role_ids: string[];
  selected_channel_ids: string[];
  buttons: AnnouncementButton[];
  auto_reactions_text: string;
}

function emptyForm(): FormState {
  return {
    name: "",
    recurrence_type: "daily",
    recurrence_hour: 12,
    recurrence_minute: 0,
    recurrence_day_of_week: 0,
    recurrence_day_of_month: 1,
    recurrence_month: 1,
    scheduled_at: "",
    end_date: "",
    content_type: "text",
    content_text: "",
    embed_title: "",
    embed_color_hex: "#5865f2",
    embed_image_url: "",
    embed_thumbnail_url: "",
    embed_footer_text: "",
    mention_everyone: false,
    mention_here: false,
    selected_role_ids: [],
    selected_channel_ids: [],
    buttons: [],
    auto_reactions_text: "",
  };
}

const form = ref<FormState>(emptyForm());
const saving = ref(false);
const channelSearch = ref("");
const roleSearch = ref("");
const channelPickerOpen = ref(false);
const rolePickerOpen = ref(false);

const dowLabels = ["Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi", "Samedi", "Dimanche"];
const monthLabels = [
  "Janvier", "Février", "Mars", "Avril", "Mai", "Juin",
  "Juillet", "Août", "Septembre", "Octobre", "Novembre", "Décembre",
];

const mode = computed<"create" | "edit">(() => (props.target ? "edit" : "create"));

watch(
  () => props.visible,
  (v) => {
    if (!v) return;
    if (props.target) {
      const a = props.target;
      form.value = {
        name: a.name,
        recurrence_type: a.recurrence_type,
        recurrence_hour: a.recurrence_hour,
        recurrence_minute: a.recurrence_minute,
        recurrence_day_of_week: a.recurrence_day_of_week,
        recurrence_day_of_month: a.recurrence_day_of_month,
        recurrence_month: a.recurrence_month ?? 1,
        scheduled_at: a.scheduled_at ? a.scheduled_at.slice(0, 16) : "",
        end_date: a.end_date ? a.end_date.slice(0, 16) : "",
        content_type: a.content_type,
        content_text: a.content_text,
        embed_title: a.embed_title ?? "",
        embed_color_hex: a.embed_color != null ? `#${a.embed_color.toString(16).padStart(6, "0")}` : "#5865f2",
        embed_image_url: a.embed_image_url ?? "",
        embed_thumbnail_url: a.embed_thumbnail_url ?? "",
        embed_footer_text: a.embed_footer_text ?? "",
        mention_everyone: a.mention_everyone,
        mention_here: a.mention_here,
        selected_role_ids: [...a.mention_role_ids],
        selected_channel_ids: [...a.channel_ids],
        buttons: a.buttons.map((b) => ({ ...b })),
        auto_reactions_text: a.auto_reactions.join(" "),
      };
    } else {
      form.value = emptyForm();
      // Mode "create" : applique les defauts de la config guild
      // (default_color_hex, default_mention_everyone) lus dans
      // bot_guild_config sous bot_name='announcements'.
      void applyGuildDefaults();
    }
  },
);

async function applyGuildDefaults() {
  try {
    const cfgs = await botConfigService.getGuildConfig(props.guildId);
    const ann = cfgs.filter((c) => c.bot_name === "announcements");
    const color = ann.find((c) => c.config_key === "default_color_hex")?.config_value;
    if (color && /^#?[0-9a-fA-F]{6}$/.test(color)) {
      form.value.embed_color_hex = color.startsWith("#") ? color : `#${color}`;
    }
    const mentionEveryone = ann.find((c) => c.config_key === "default_mention_everyone")?.config_value;
    if (mentionEveryone === "true" || mentionEveryone === "1") {
      form.value.mention_everyone = true;
    }
  } catch (e) {
    // Garde les defauts hardcodes du `emptyForm()` mais previens
    // l'utilisateur que les defauts du serveur n'ont pas pu etre lus.
    const msg = errMsg(e);
    toastErr(`Impossible de charger les defauts du serveur : ${msg}`);
  }
}

const availableChannels = computed(() => {
  const selected = new Set(form.value.selected_channel_ids);
  const search = channelSearch.value.toLowerCase();
  return props.channels.filter(
    (c) => !selected.has(c.id) && (search === "" || c.name.toLowerCase().includes(search)),
  );
});
const availableRoles = computed(() => {
  const selected = new Set(form.value.selected_role_ids);
  const search = roleSearch.value.toLowerCase();
  return props.roles.filter(
    (r) => !selected.has(r.id) && (search === "" || r.name.toLowerCase().includes(search)),
  );
});

const channelsById = computed(() => {
  const m: Record<string, DiscordTextChannel> = {};
  for (const c of props.channels) m[c.id] = c;
  return m;
});
const rolesById = computed(() => {
  const m: Record<string, DiscordRole> = {};
  for (const r of props.roles) m[r.id] = r;
  return m;
});

function addRole(id: string) {
  if (!form.value.selected_role_ids.includes(id)) form.value.selected_role_ids.push(id);
  roleSearch.value = "";
}
function addChannel(id: string) {
  if (!form.value.selected_channel_ids.includes(id)) form.value.selected_channel_ids.push(id);
  channelSearch.value = "";
}
function toggleChannel(id: string) {
  const arr = form.value.selected_channel_ids;
  const i = arr.indexOf(id);
  if (i >= 0) arr.splice(i, 1);
  else arr.push(id);
}
function toggleRole(id: string) {
  const arr = form.value.selected_role_ids;
  const i = arr.indexOf(id);
  if (i >= 0) arr.splice(i, 1);
  else arr.push(id);
}

function buildBody(): CreateAnnouncementBody {
  const f = form.value;
  const colorInt = parseInt(f.embed_color_hex.replace("#", ""), 16);
  const reactions = f.auto_reactions_text
    .split(/[\s,]+/)
    .map((s) => s.trim())
    .filter(Boolean)
    .slice(0, 20);
  return {
    guild_id: props.guildId,
    name: f.name,
    recurrence_type: f.recurrence_type,
    recurrence_hour: f.recurrence_hour,
    recurrence_minute: f.recurrence_minute,
    recurrence_day_of_week: f.recurrence_type === "weekly" ? f.recurrence_day_of_week : null,
    recurrence_day_of_month:
      f.recurrence_type === "monthly" || f.recurrence_type === "yearly"
        ? f.recurrence_day_of_month
        : null,
    recurrence_month: f.recurrence_type === "yearly" ? f.recurrence_month : null,
    scheduled_at:
      f.recurrence_type === "once" && f.scheduled_at ? new Date(f.scheduled_at).toISOString() : null,
    end_date: f.end_date ? new Date(f.end_date).toISOString() : null,
    content_type: f.content_type,
    content_text: f.content_text,
    embed_title: f.content_type === "embed" ? f.embed_title || null : null,
    embed_color: f.content_type === "embed" && !Number.isNaN(colorInt) ? colorInt : null,
    embed_image_url: f.content_type === "embed" ? f.embed_image_url || null : null,
    embed_thumbnail_url: f.content_type === "embed" ? f.embed_thumbnail_url || null : null,
    embed_footer_text: f.content_type === "embed" ? f.embed_footer_text || null : null,
    mention_everyone: f.mention_everyone,
    mention_here: f.mention_here,
    mention_role_ids: f.selected_role_ids,
    channel_ids: f.selected_channel_ids,
    buttons: f.buttons.filter((b) => b.label.trim()),
    auto_reactions: reactions,
  };
}

const formCanSave = computed(() => {
  const f = form.value;
  if (!f.name.trim()) return false;
  if (f.selected_channel_ids.length === 0) return false;
  if (f.recurrence_type === "once" && !f.scheduled_at) return false;
  return true;
});

async function save() {
  saving.value = true;
  try {
    const body = buildBody();
    if (mode.value === "create") {
      await announcementsService.create(body);
      toastOk("Annonce créée.");
    } else if (props.target) {
       
      const { guild_id: _gid, ...rest } = body;
      await announcementsService.update(props.target.id, rest);
      toastOk("Annonce mise à jour.");
    }
    emit("saved");
    emit("close");
  } catch (e: unknown) {
    toastErr(`Echec sauvegarde : ${errMsg(e)}`);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <AppModal
    :visible="visible"
    :title="mode === 'create' ? 'Nouvelle annonce' : `Modifier l'annonce`"
    size="xl"
    @close="emit('close')"
  >
    <div class="grid-2">
      <label>
        Nom *
        <AppInput v-model="form.name" type="text" placeholder="ex: Rappel Tournoi du dimanche" />
      </label>
      <label>
        Type de récurrence
        <AppSelect v-model="form.recurrence_type">
          <option value="once">Ponctuelle (une fois)</option>
          <option value="daily">Quotidienne</option>
          <option value="weekly">Hebdomadaire</option>
          <option value="monthly">Mensuelle</option>
          <option value="yearly">Annuelle (une fois par an)</option>
        </AppSelect>
      </label>
    </div>

    <div class="grid-2">
      <label>
        Heure (UTC)
        <NumberInputWithUnit v-model.number="form.recurrence_hour" :min="0" :max="23" />
      </label>
      <label>
        Minute
        <NumberInputWithUnit v-model.number="form.recurrence_minute" :min="0" :max="59" />
      </label>
    </div>

    <label v-if="form.recurrence_type === 'weekly'">
      Jour de la semaine
      <AppSelect v-model.number="form.recurrence_day_of_week">
        <option v-for="(d, i) in dowLabels" :key="i" :value="i">{{ d }}</option>
      </AppSelect>
    </label>

    <label v-if="form.recurrence_type === 'monthly'">
      Jour du mois (1-31, clamp si mois plus court)
      <NumberInputWithUnit v-model.number="form.recurrence_day_of_month" :min="1" :max="31" />
    </label>

    <div v-if="form.recurrence_type === 'yearly'" class="grid-2">
      <label>
        Mois
        <AppSelect v-model.number="form.recurrence_month">
          <option v-for="(m, i) in monthLabels" :key="i" :value="i + 1">{{ m }}</option>
        </AppSelect>
      </label>
      <label>
        Jour du mois (1-31, clamp si mois plus court)
        <NumberInputWithUnit v-model.number="form.recurrence_day_of_month" :min="1" :max="31" />
      </label>
    </div>

    <label v-if="form.recurrence_type === 'once'">
      Date et heure de l'annonce
      <input v-model="form.scheduled_at" type="datetime-local" />
    </label>

    <label>
      Date de fin (optionnelle, vide = indéfini)
      <input v-model="form.end_date" type="datetime-local" />
    </label>

    <hr class="sep" />

    <label>
      Format
      <AppSelect v-model="form.content_type">
        <option value="text">Texte simple</option>
        <option value="embed">Embed riche (carte)</option>
      </AppSelect>
    </label>

    <template v-if="form.content_type === 'embed'">
      <div class="grid-2">
        <label>
          Titre embed
          <AppInput v-model="form.embed_title" type="text" />
        </label>
        <label>
          Couleur
          <input v-model="form.embed_color_hex" type="color" />
        </label>
      </div>
      <label>
        Image (affichée en grand sous le texte de l'embed)
        <ImagePicker v-model="form.embed_image_url" />
      </label>
      <label>
        Vignette (petite, à droite)
        <ImagePicker v-model="form.embed_thumbnail_url" />
      </label>
      <label>
        Texte sous l'image (footer)
        <AppInput v-model="form.embed_footer_text" type="text" maxlength="2048" />
        <small class="hint">
          Seule zone de texte que Discord place sous l'image. Affichée en petit et en gris,
          sans mise en forme ni mentions.
        </small>
      </label>
    </template>

    <label>
      {{ form.content_type === "embed" ? "Description (variables : {date} {day_name} {time} ...)" : "Contenu (variables : {date} {day_name} {time} ...)" }}
      <AppTextarea v-model="form.content_text" :rows="5" />
    </label>

    <hr class="sep" />

    <div class="checkbox-row">
      <AppCheckbox v-model="form.mention_everyone">Mentionner @everyone</AppCheckbox>
      <AppCheckbox v-model="form.mention_here">Mentionner @here</AppCheckbox>
    </div>

    <!-- Picker rôles -->
    <div class="picker-section">
      <h4>
        Rôles à mentionner
        <span class="req-count">({{ form.selected_role_ids.length }} sélectionné{{ form.selected_role_ids.length > 1 ? "s" : "" }})</span>
      </h4>
      <div class="multi-picker">
        <input
          v-model="roleSearch"
          type="text"
          placeholder="🔍 Rechercher un rôle..."
          class="picker-input"
          @focus="rolePickerOpen = true"
        />
        <button
          type="button"
          class="picker-toggle"
          @click="rolePickerOpen = !rolePickerOpen"
        >{{ rolePickerOpen ? '▲' : '▼' }}</button>
        <ul v-if="rolePickerOpen" class="picker-dropdown">
          <li v-if="availableRoles.length === 0" class="picker-empty">
            {{ roleSearch ? "Aucun rôle ne correspond." : "Tous les rôles sont déjà sélectionnés." }}
          </li>
          <li
            v-for="r in availableRoles"
            :key="r.id"
            class="picker-option"
            @click="addRole(r.id)"
          >
            <span class="role-color" :style="{ background: r.color ? '#' + r.color.toString(16).padStart(6, '0') : '#888' }" />
            <span class="picker-option-label">@{{ r.name }}</span>
            <span class="picker-add">+</span>
          </li>
        </ul>
      </div>
      <div v-if="form.selected_role_ids.length > 0" class="chips">
        <span
          v-for="rid in form.selected_role_ids"
          :key="rid"
          class="chip role-chip"
          @click="toggleRole(rid)"
          title="Cliquer pour retirer"
        >
          <span
            class="role-color"
            :style="{
              background: rolesById[rid]?.color
                ? '#' + rolesById[rid].color.toString(16).padStart(6, '0')
                : '#888',
            }"
          />
          @{{ rolesById[rid]?.name ?? rid }}
          <span class="chip-remove">×</span>
        </span>
      </div>
    </div>

    <!-- Picker channels -->
    <div class="picker-section">
      <h4>
        Salons cibles *
        <span class="req-count">({{ form.selected_channel_ids.length }} sélectionné{{ form.selected_channel_ids.length > 1 ? "s" : "" }})</span>
      </h4>
      <div class="multi-picker">
        <input
          v-model="channelSearch"
          type="text"
          placeholder="🔍 Rechercher un salon..."
          class="picker-input"
          @focus="channelPickerOpen = true"
        />
        <button
          type="button"
          class="picker-toggle"
          @click="channelPickerOpen = !channelPickerOpen"
        >{{ channelPickerOpen ? '▲' : '▼' }}</button>
        <ul v-if="channelPickerOpen" class="picker-dropdown">
          <li v-if="availableChannels.length === 0" class="picker-empty">
            {{ channelSearch ? "Aucun salon ne correspond." : "Tous les salons sont déjà sélectionnés." }}
          </li>
          <li
            v-for="c in availableChannels"
            :key="c.id"
            class="picker-option"
            @click="addChannel(c.id)"
          >
            <span class="picker-option-label">#{{ c.name }}</span>
            <span class="picker-add">+</span>
          </li>
        </ul>
      </div>
      <div v-if="form.selected_channel_ids.length > 0" class="chips">
        <span
          v-for="cid in form.selected_channel_ids"
          :key="cid"
          class="chip channel-chip"
          @click="toggleChannel(cid)"
          title="Cliquer pour retirer"
        >
          #{{ channelsById[cid]?.name ?? cid }}
          <span class="chip-remove">×</span>
        </span>
      </div>
    </div>

    <hr class="sep" />

    <!-- Section Boutons -->
    <AnnouncementButtonsEditor v-model="form.buttons" />

    <hr class="sep" />

    <!-- Section Réactions -->
    <div>
      <h4>💬 Réactions automatiques (max 20)</h4>
      <p class="muted small">
        Emojis ajoutés en réaction au message. Séparés par espace ou virgule.
        Format unicode (👍) ou custom Discord (<code>&lt;:nom:id&gt;</code>).
      </p>
      <input
        v-model="form.auto_reactions_text"
        type="text"
        placeholder="👍 ❤️ 🎉 ou <:custom:1234>"
      />
    </div>

    <template #footer>
      <AppButton variant="secondary" :disabled="saving" @click="emit('close')">Annuler</AppButton>
      <AppButton variant="primary" :disabled="!formCanSave || saving" @click="save">
        {{ saving ? "Enregistrement…" : "Enregistrer" }}
      </AppButton>
    </template>
  </AppModal>
</template>

<style scoped src="../../styles/announcement-form-modal.css"></style>
