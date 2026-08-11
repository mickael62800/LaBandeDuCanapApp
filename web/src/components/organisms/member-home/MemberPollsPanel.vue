<script setup lang="ts">
import ActionButton from "@/components/atoms/ActionButton.vue";
import type { Poll } from "@/services/communityLifeService";
import { formatDay } from "@/utils/publicCommunityFormat";

defineProps<{
  polls: Poll[];
  authenticated: boolean;
  busyId: string | null;
}>();
const emit = defineEmits<{ vote: [pollId: string, optionId: string] }>();
</script>

<template>
  <section class="mb-block">
    <h2>On vote</h2>
    <article v-for="poll in polls" :key="poll.id" class="mb-poll">
      <h3>{{ poll.question }}</h3>
      <p v-if="poll.description" class="mb-poll-desc">{{ poll.description }}</p>
      <ul class="mb-poll-list">
        <li v-for="option in poll.options" :key="option.id" class="mb-poll-opt">
          <button
            type="button"
            class="mb-poll-line"
            :class="{ mine: poll.my_vote === option.id, votable: authenticated }"
            :disabled="!authenticated || busyId === poll.id"
            @click="emit('vote', poll.id, option.id)"
          >
            <span>{{ option.label }}</span><span class="mb-poll-pct">{{ option.share }} %</span>
          </button>
          <div class="mb-poll-bar"><i :style="{ width: `${option.share}%`, background: `#${option.color}` }"></i></div>
          <span class="mb-poll-n">{{ option.votes }} voix</span>
        </li>
      </ul>
      <p class="mb-poll-foot">{{ poll.total_votes }} vote(s) · se termine le {{ formatDay(poll.closes_at) }}</p>
      <ActionButton v-if="!authenticated" to="/login?espace=membre">Se connecter pour voter</ActionButton>
    </article>
    <p v-if="!polls.length" class="mb-vide">
      Aucun vote en cours. Le staff en ouvre pour choisir les prochains jeux ou les horaires des soirées.
    </p>
  </section>
</template>
