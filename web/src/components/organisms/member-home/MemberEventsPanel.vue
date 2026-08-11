<script setup lang="ts">
import ActionButton from "@/components/atoms/ActionButton.vue";
import type { PublicEvent } from "@/services/publicEventsService";
import { eventAccent, formatDay, formatEventRange } from "@/utils/publicCommunityFormat";

defineProps<{
  ongoing: PublicEvent[];
  nextEvent: PublicEvent | null;
  upcoming: PublicEvent[];
  authenticated: boolean;
  section: "ongoing" | "upcoming";
}>();
</script>

<template>
  <section v-if="section === 'ongoing' && ongoing.length" class="mb-block">
    <h2><span class="mb-live" aria-hidden="true"></span> En ce moment</h2>
    <ul class="mb-events">
      <li
        v-for="event in ongoing"
        :key="event.id"
        class="mb-event ongoing"
        :style="{ '--accent-event': eventAccent(event) }"
      >
        <div class="mb-event-main">
          <strong>{{ event.title }}</strong><span v-if="event.game" class="mb-tag">{{ event.game }}</span>
        </div>
        <p v-if="event.description" class="mb-event-desc">{{ event.description }}</p>
        <span class="mb-event-when">Jusqu'au {{ formatDay(event.ends_at) }}</span>
      </li>
    </ul>
  </section>

  <section v-if="section === 'upcoming'" class="mb-block">
    <h2><span class="mb-live" aria-hidden="true"></span> Le prochain rendez-vous</h2>
    <p v-if="!nextEvent" class="mb-vide">
      Rien de programmé pour l'instant. Les soirées et les campagnes de jeu s'annoncent ici.
    </p>
    <div v-else class="mb-feature" :style="{ '--accent-event': eventAccent(nextEvent) }">
      <div class="mb-feature-body">
        <div class="mb-tags">
          <span v-if="nextEvent.game" class="mb-tag">{{ nextEvent.game }}</span>
          <span class="mb-tag neutral">{{ formatEventRange(nextEvent) }}</span>
        </div>
        <h3>{{ nextEvent.title }}</h3>
        <p v-if="nextEvent.description">{{ nextEvent.description }}</p>
        <ActionButton v-if="!authenticated" to="/login?espace=membre">Se connecter pour s'inscrire</ActionButton>
        <span v-else class="mb-soon">Inscription bientôt</span>
      </div>
    </div>
    <ul v-if="upcoming.length" class="mb-events secondaires">
      <li
        v-for="event in upcoming"
        :key="event.id"
        class="mb-event"
        :style="{ '--accent-event': eventAccent(event) }"
      >
        <div class="mb-event-main">
          <strong>{{ event.title }}</strong>
          <span v-if="event.game" class="mb-tag">{{ event.game }}</span>
          <span v-if="event.span_days > 1" class="mb-tag neutral">{{ event.span_days }} jours</span>
        </div>
        <span class="mb-event-when">{{ formatEventRange(event) }}</span>
      </li>
    </ul>
  </section>
</template>
