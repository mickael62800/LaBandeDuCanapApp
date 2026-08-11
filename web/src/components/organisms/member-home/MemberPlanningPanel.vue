<script setup lang="ts">
import { computed, ref } from "vue";
import {
  addWeeks,
  layoutWeek,
  startOfWeek,
  weekDays,
  weekLabel,
} from "@/composables/useWeekPlanning";
import type { PublicEvent } from "@/services/publicEventsService";
import { eventAccent, formatTime } from "@/utils/publicCommunityFormat";

const props = defineProps<{ events: PublicEvent[]; loading: boolean }>();
const weekStart = ref(startOfWeek(new Date()));
const bars = computed(() => layoutWeek(props.events, weekStart.value));
const days = computed(() => weekDays(weekStart.value));
const label = computed(() => weekLabel(weekStart.value));
const rows = computed(() => Math.max(2, ...bars.value.map((bar) => bar.row), 0));

function isToday(day: Date): boolean {
  const now = new Date();
  return day.getDate() === now.getDate()
    && day.getMonth() === now.getMonth()
    && day.getFullYear() === now.getFullYear();
}
</script>

<template>
  <section class="mb-block">
    <h2>
      Le planning
      <span class="mb-count">semaine du {{ label }}</span>
      <span class="mb-nav">
        <button type="button" aria-label="Semaine précédente" @click="weekStart = addWeeks(weekStart, -1)">‹</button>
        <button type="button" @click="weekStart = startOfWeek(new Date())">Aujourd'hui</button>
        <button type="button" aria-label="Semaine suivante" @click="weekStart = addWeeks(weekStart, 1)">›</button>
      </span>
    </h2>
    <p v-if="loading" class="mb-hint">Chargement du planning…</p>
    <div v-else class="mb-cal">
      <div class="mb-cal-head">
        <div v-for="day in days" :key="day.toISOString()" :class="{ today: isToday(day) }">
          {{ day.toLocaleDateString("fr-FR", { weekday: "short" }) }}
          <b>{{ day.getDate() }}</b>
        </div>
      </div>
      <div class="mb-cal-body" :style="{ '--rows': rows }">
        <div
          v-for="bar in bars"
          :key="bar.event.id"
          class="mb-bar"
          :class="{ clipped: bar.clippedStart || bar.clippedEnd }"
          :style="{
            '--row': bar.row,
            '--from': bar.from,
            '--span': bar.span,
            '--ev': eventAccent(bar.event) || 'var(--accent)',
          }"
          :title="bar.event.title"
        >
          <strong>{{ bar.event.title }}</strong>
          <span v-if="bar.event.span_days > 1">{{ bar.event.game || "campagne" }}</span>
          <span v-else>{{ formatTime(bar.event.starts_at) }}</span>
        </div>
        <p v-if="!bars.length" class="mb-cal-vide">Rien de prévu cette semaine.</p>
      </div>
    </div>
  </section>
</template>
