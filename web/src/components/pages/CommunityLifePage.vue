<script setup lang="ts">
import { computed, ref } from "vue";
import AppCheckbox from "../atoms/AppCheckbox.vue";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import AppTabs from "../molecules/AppTabs.vue";
import ConfirmDialog from "../molecules/ConfirmDialog.vue";
import CommunityLfgPanel from "../organisms/community-life/CommunityLfgPanel.vue";
import CommunityNewsPanel from "../organisms/community-life/CommunityNewsPanel.vue";
import CommunityPollsPanel from "../organisms/community-life/CommunityPollsPanel.vue";
import CommunitySpotlightPanel from "../organisms/community-life/CommunitySpotlightPanel.vue";
import { useCommunityLife, type LifeTab } from "@/composables/useCommunityLife";
import { useConfirm } from "@/composables/useConfirm";
import { useToast } from "@/composables/useToast";
import {
  communityAdminService,
  type AdminLfgPost,
  type AdminNewsItem,
  type AdminPoll,
  type AdminSpotlight,
  type CreatePollInput,
  type UpsertNewsInput,
} from "@/services/communityAdminService";
import { errMsg } from "@/utils/errMsg";

const { tab, showArchived, lfg, polls, spotlight, news, loading, guildId, refresh } =
  useCommunityLife();
const { success, error: toastError } = useToast();
const { confirm } = useConfirm();

const tabs = [
  { key: "news", label: "Annonces du site", icon: "📰" },
  { key: "polls", label: "Sondages", icon: "🗳️" },
  { key: "spotlight", label: "Membre du mois", icon: "⭐" },
  { key: "lfg", label: "Recherche de joueurs", icon: "🎮" },
];

const busy = ref(false);
const newsEditing = ref<string | null>(null);
const newsForm = ref<UpsertNewsInput>(emptyNews());
const pollOpened = ref(false);
const pollForm = ref<CreatePollInput>(emptyPoll());
const spotlightOpened = ref(false);
const spotlightForm = ref({ user_id: "", period: "", reason: "" });

async function runAction(action: () => Promise<unknown>, message: string) {
  busy.value = true;
  try {
    await action();
    success(message);
    await refresh();
  } catch (error: unknown) {
    toastError(errMsg(error));
  } finally {
    busy.value = false;
  }
}

async function remove(label: string, action: () => Promise<unknown>) {
  const confirmed = await confirm({
    title: `Supprimer ${label} ?`,
    message: "Cette action est définitive.",
  });
  if (confirmed) await runAction(action, "Supprimé.");
}

function emptyNews(): UpsertNewsInput {
  return { title: "", body: "", image_url: "", is_pinned: false, is_public: true };
}

function openNews(id?: string) {
  const existing = id ? news.value.find((item) => item.id === id) : undefined;
  newsForm.value = existing
    ? {
        title: existing.title,
        body: existing.body,
        image_url: existing.image_url ?? "",
        is_pinned: existing.is_pinned,
        is_public: existing.is_public,
      }
    : emptyNews();
  newsEditing.value = id ?? "";
}

async function saveNews() {
  const currentGuildId = guildId.value;
  if (!currentGuildId) return;
  const payload: UpsertNewsInput = {
    ...newsForm.value,
    image_url: newsForm.value.image_url?.trim() || null,
  };
  const id = newsEditing.value;
  await runAction(
    () =>
      id
        ? communityAdminService.updateNews(id, payload)
        : communityAdminService.createNews(currentGuildId, payload),
    id ? "Annonce mise à jour." : "Annonce publiée.",
  );
  newsEditing.value = null;
}

function removeNews(item: AdminNewsItem) {
  return remove(item.title, () => communityAdminService.deleteNews(item.id));
}

function inDays(days: number): string {
  const date = new Date();
  date.setDate(date.getDate() + days);
  date.setSeconds(0, 0);
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

function emptyPoll(): CreatePollInput {
  return {
    question: "",
    description: "",
    closes_at: inDays(7),
    is_public: true,
    options: [{ label: "" }, { label: "" }],
  };
}

function openPoll() {
  pollForm.value = emptyPoll();
  pollOpened.value = true;
}

const validPollOptions = computed(
  () => pollForm.value.options.filter((option) => option.label.trim()).length >= 2,
);

async function savePoll() {
  const currentGuildId = guildId.value;
  if (!currentGuildId) return;
  await runAction(
    () =>
      communityAdminService.createPoll(currentGuildId, {
        ...pollForm.value,
        description: pollForm.value.description?.trim() || null,
        closes_at: new Date(pollForm.value.closes_at).toISOString(),
        options: pollForm.value.options.filter((option) => option.label.trim()),
      }),
    "Sondage ouvert.",
  );
  pollOpened.value = false;
}

function closePoll(item: AdminPoll) {
  return runAction(() => communityAdminService.closePoll(item.id), "Sondage clos.");
}

function removePoll(item: AdminPoll) {
  return remove(item.question, () => communityAdminService.deletePoll(item.id));
}

function openSpotlight() {
  spotlightForm.value = { user_id: "", period: "", reason: "" };
  spotlightOpened.value = true;
}

async function saveSpotlight() {
  const currentGuildId = guildId.value;
  if (!currentGuildId) return;
  await runAction(
    () =>
      communityAdminService.designate(currentGuildId, {
        user_id: spotlightForm.value.user_id.trim(),
        period: spotlightForm.value.period.trim() || undefined,
        reason: spotlightForm.value.reason,
      }),
    "Membre du mois désigné.",
  );
  spotlightOpened.value = false;
}

function removeSpotlight(item: AdminSpotlight) {
  const currentGuildId = guildId.value;
  if (!currentGuildId) return;
  return remove(`${item.username} (${item.period})`, () =>
    communityAdminService.deleteSpotlight(currentGuildId, item.id),
  );
}

function closeLfg(item: AdminLfgPost) {
  return runAction(() => communityAdminService.closeLfg(item.id), "Annonce fermée.");
}

function removeLfg(item: AdminLfgPost) {
  return remove(`l'annonce de ${item.author_name}`, () => communityAdminService.deleteLfg(item.id));
}
</script>

<template>
  <AdminPageShell title="Vie de la communauté" icon="🛋️" class="cl-page">
    <template #lede>
      Ce qui alimente l'espace membre du site. Les annonces de recherche de joueurs sont écrites
      par les membres&nbsp;: on les modère, on ne les rédige pas.
    </template>
    <template #actions>
      <AppCheckbox v-model="showArchived">Afficher les éléments clos et brouillons</AppCheckbox>
    </template>

    <AppTabs :model-value="tab" :tabs="tabs" @update:model-value="tab = $event as LifeTab" />
    <p v-if="!guildId" class="muted">Sélectionne un serveur pour commencer.</p>
    <p v-else-if="loading" class="muted">Chargement…</p>

    <template v-else>
      <CommunityNewsPanel
        v-if="tab === 'news'"
        v-model:form="newsForm"
        :items="news"
        :editing="newsEditing"
        :busy="busy"
        @create="openNews()"
        @edit="openNews"
        @save="saveNews"
        @cancel="newsEditing = null"
        @remove="removeNews"
      />
      <CommunityPollsPanel
        v-else-if="tab === 'polls'"
        v-model:form="pollForm"
        :items="polls"
        :opened="pollOpened"
        :busy="busy"
        :options-valid="validPollOptions"
        @create="openPoll"
        @save="savePoll"
        @cancel="pollOpened = false"
        @close="closePoll"
        @remove="removePoll"
      />
      <CommunitySpotlightPanel
        v-else-if="tab === 'spotlight'"
        v-model:form="spotlightForm"
        :items="spotlight"
        :opened="spotlightOpened"
        :busy="busy"
        @create="openSpotlight"
        @save="saveSpotlight"
        @cancel="spotlightOpened = false"
        @remove="removeSpotlight"
      />
      <CommunityLfgPanel
        v-else
        :items="lfg"
        :busy="busy"
        @close="closeLfg"
        @remove="removeLfg"
      />
    </template>

    <ConfirmDialog />
  </AdminPageShell>
</template>

<style src="../../styles/community-life.css"></style>
