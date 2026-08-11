<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useConfirm } from "../../composables/useConfirm";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useInfractions } from "../../composables/useInfractions";
import { useModeration } from "../../composables/useModeration";
import { useRealtimeRefresh } from "../../composables/useRealtimeRefresh";
import { useToast } from "../../composables/useToast";
import { moderationService } from "@/services/moderationService";
import type { Infraction } from "../../types";
import ModerationActionModal from "./moderation-journal/ModerationActionModal.vue";
import ModerationJournalFilters from "./moderation-journal/ModerationJournalFilters.vue";
import ModerationJournalTable from "./moderation-journal/ModerationJournalTable.vue";

const emit = defineEmits<{
  "open-notes-evidence": [userId: string];
}>();

const { success, error: showError } = useToast();
const { confirm } = useConfirm();
const {
  infractions,
  loading: infractionsLoading,
  error: infractionsError,
  fetchInfractions,
  deleting,
  deleteInfraction,
  purging,
  purgeAll,
} = useInfractions();
useRealtimeRefresh(["infraction_new", "strike_added"], fetchInfractions);

const { selectedGuildId } = useGuildSelector();
const { logAction } = useModeration();

const journalSearch = ref("");
const journalType = ref("all");
const journalModerator = ref("all");
const journalStatus = ref<"all" | "detection" | "action">("all");
const journalDateFrom = ref("");
const journalDateTo = ref("");
const hideDetections = ref(true);
const bulkMenuOpen = ref(false);
const applying = ref(false);
const actionModalVisible = ref(false);

const statusOptions = [
  { value: "all", label: "Tous les statuts" },
  { value: "detection", label: "Propositions" },
  { value: "action", label: "Appliquees" },
];

const moderatorOptions = computed(() => {
  const moderators = new Set<string>();
  for (const infraction of infractions.value ?? []) {
    if (infraction.moderator) moderators.add(infraction.moderator);
  }
  return [
    { value: "all", label: "Tous les moderateurs" },
    ...Array.from(moderators).sort().map((moderator) => ({ value: moderator, label: moderator })),
  ];
});

const typeOptions = computed(() => {
  const types = new Set<string>();
  for (const infraction of infractions.value ?? []) {
    if (infraction.infraction_type) types.add(infraction.infraction_type);
  }
  return [
    { value: "all", label: "Tous les types" },
    ...Array.from(types).sort().map((type) => ({ value: type, label: type })),
  ];
});

const filteredInfractions = computed<Infraction[]>(() => {
  let rows = (infractions.value ?? []).slice();

  if (hideDetections.value) {
    rows = rows.filter((infraction) => (infraction.source ?? "detection") === "action");
  }

  rows = rows.filter((infraction) => {
    const isBan = ["ban_permanent", "ban_temp", "ban"].includes(infraction.infraction_type);
    const isApplied = (infraction.source ?? "detection") === "action";
    return !(isBan && isApplied);
  });

  const query = journalSearch.value.trim().toLowerCase();
  if (query) {
    rows = rows.filter((infraction) =>
      [
        infraction.username,
        infraction.user_id,
        infraction.reason,
        infraction.infraction_type,
        infraction.moderator,
        infraction.server,
      ].some((field) => String(field ?? "").toLowerCase().includes(query)),
    );
  }
  if (journalType.value !== "all") {
    rows = rows.filter((infraction) => infraction.infraction_type === journalType.value);
  }
  if (journalModerator.value !== "all") {
    rows = rows.filter((infraction) => infraction.moderator === journalModerator.value);
  }
  if (journalStatus.value !== "all") {
    rows = rows.filter((infraction) => (infraction.source ?? "detection") === journalStatus.value);
  }
  if (journalDateFrom.value) {
    const from = new Date(journalDateFrom.value).getTime();
    rows = rows.filter((infraction) => new Date(infraction.created_at).getTime() >= from);
  }
  if (journalDateTo.value) {
    const to = new Date(journalDateTo.value).getTime() + 86_400_000;
    rows = rows.filter((infraction) => new Date(infraction.created_at).getTime() < to);
  }

  return rows.sort(
    (left, right) => new Date(right.created_at).getTime() - new Date(left.created_at).getTime(),
  );
});

const hasActiveFilters = computed(
  () =>
    journalSearch.value !== "" ||
    journalType.value !== "all" ||
    journalModerator.value !== "all" ||
    journalStatus.value !== "all" ||
    journalDateFrom.value !== "" ||
    journalDateTo.value !== "" ||
    !hideDetections.value,
);

function resetFilters() {
  journalSearch.value = "";
  journalType.value = "all";
  journalModerator.value = "all";
  journalStatus.value = "all";
  journalDateFrom.value = "";
  journalDateTo.value = "";
  hideDetections.value = true;
}

function closeBulkMenu() {
  bulkMenuOpen.value = false;
}

onMounted(() => document.addEventListener("click", closeBulkMenu));
onBeforeUnmount(() => document.removeEventListener("click", closeBulkMenu));

async function onDeleteInfraction(row: Record<string, unknown>) {
  const id = row.id as string;
  const source = (row.source as "detection" | "action" | undefined) ?? "detection";
  const actionType = String(row.infraction_type ?? "").toLowerCase();
  const isBan = source === "action" && actionType.startsWith("ban");
  const isMute =
    source === "action" && (actionType.startsWith("mute") || actionType === "timeout");

  const message = isBan
    ? "Annuler ce BAN ? L'utilisateur sera debanni du serveur Discord et la ligne supprimee de la BDD. Cette action est irreversible."
    : isMute
      ? "Annuler ce MUTE ? Le timeout sera retire sur Discord et la ligne supprimee de la BDD. Cette action est irreversible."
      : source === "action"
        ? "Annuler cette action appliquee ? La ligne sera supprimee de la BDD. Cette action est irreversible."
        : "Annuler cette detection ? Elle sera supprimee de la BDD. Cette action est irreversible.";

  if (!(await confirm({ message }))) return;
  try {
    await deleteInfraction(id, source);
  } catch (error) {
    console.error("Erreur suppression infraction:", error);
    showError("Erreur lors de la suppression");
  }
}

async function onApplyDetection(row: Record<string, unknown>) {
  const id = row.id as string;
  const actionType = String(row.infraction_type ?? "").toLowerCase();
  const guildId = String(row.server ?? "");
  const userId = String(row.user_id ?? "");
  const username = String(row.username ?? userId);
  const reason = String(row.reason ?? "Applique depuis le panneau admin");

  if (!guildId || !userId) {
    showError("Guild ou user manquant sur cette detection");
    return;
  }

  const isBan = actionType === "ban";
  const isMute = actionType === "mute" || actionType === "timeout";
  const isWarn = actionType === "warn";
  const duration = typeof row.duration === "number" ? row.duration : undefined;
  const label = isBan ? "BAN" : isMute ? "MUTE" : isWarn ? "AVERTISSEMENT" : actionType.toUpperCase();
  const detail = isBan
    ? "L'utilisateur sera effectivement banni du serveur Discord."
    : isMute
      ? `Un timeout Discord sera applique (${duration ?? 3600}s) et l'action sera loguee en DB.`
      : "Un avertissement sera enregistre en DB.";

  if (!(await confirm({ message: `Appliquer ${label} a ${username} ?\n\n${detail}\n\nRaison : ${reason}` }))) return;

  applying.value = true;
  try {
    if (isBan) {
      await moderationService.executeBan(guildId, userId, reason);
    } else if (isMute) {
      await moderationService.executeMute(guildId, userId, reason, duration, username);
    } else {
      await logAction({
        guildId,
        channelId: "web-panel",
        moderatorId: "web-admin",
        moderatorName: "Web Admin",
        targetId: userId,
        targetName: username,
        actionType,
        reason,
        gravity: "medium",
      });
    }
    await deleteInfraction(id, "detection");
    success(`${label} applique a ${username}`);
  } catch (error) {
    console.error("Erreur apply detection:", error);
    showError("Erreur lors de l'application de la detection");
  } finally {
    applying.value = false;
  }
}

async function onPurgeAll() {
  const guildId = selectedGuildId.value;
  if (!guildId) {
    showError("Selectionnez d'abord un serveur pour purger.");
    return;
  }
  const total = infractions.value?.length ?? 0;
  const firstConfirmation = await confirm({
    message:
      `⚠️ Vider le journal (DB seule) ⚠️\n\n` +
      `Cette action supprime ${total} infraction(s) de la base de données POUR CE SERVEUR.\n\n` +
      `IMPORTANT : ça ne touche PAS Discord :\n` +
      `  • les bannissements actifs RESTENT actifs\n` +
      `  • les mutes / timeouts en cours RESTENT actifs\n` +
      `  • aucun DM de grâce n'est envoyé\n\n` +
      `Pour vraiment annuler une sanction (avec unban Discord), utilise le bouton Annuler ligne par ligne.\n\n` +
      `Cette suppression est IRRÉVERSIBLE. Continuer ?`,
  });
  if (!firstConfirmation) return;
  const finalConfirmation = await confirm({
    message:
      "Dernière confirmation : vider le journal pour ce serveur ? (les sanctions Discord ne seront PAS levées)",
  });
  if (!finalConfirmation) return;
  try {
    await purgeAll(guildId);
  } catch {
    // Le composable affiche déjà le toast.
  }
}
</script>

<template>
  <div>
    <ModerationJournalFilters
      v-model:search="journalSearch"
      v-model:type="journalType"
      v-model:moderator="journalModerator"
      v-model:status="journalStatus"
      v-model:date-from="journalDateFrom"
      v-model:date-to="journalDateTo"
      v-model:hide-detections="hideDetections"
      v-model:bulk-menu-open="bulkMenuOpen"
      :type-options="typeOptions"
      :moderator-options="moderatorOptions"
      :status-options="statusOptions"
      :has-active-filters="hasActiveFilters"
      :selected-guild-id="selectedGuildId"
      :purging="purging"
      @reset="resetFilters"
      @purge="onPurgeAll"
      @create="actionModalVisible = true"
    />

    <ModerationJournalTable
      :rows="filteredInfractions"
      :total="infractions?.length ?? 0"
      :loading="infractionsLoading"
      :error="infractionsError"
      :applying="applying"
      :deleting="deleting"
      @retry="fetchInfractions"
      @apply="onApplyDetection"
      @remove="onDeleteInfraction"
      @open-notes="emit('open-notes-evidence', $event)"
    />

    <ModerationActionModal
      :open="actionModalVisible"
      @close="actionModalVisible = false"
      @submitted="fetchInfractions"
    />
  </div>
</template>

<style src="../../styles/moderation-journal.css"></style>
