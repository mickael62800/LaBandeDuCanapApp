<script setup lang="ts">
import PublicMemberAvatar from "@/components/atoms/PublicMemberAvatar.vue";
import type { Anniversary, Newcomer, NewsItem, Spotlight } from "@/services/communityLifeService";
import {
  anniversaryLabel,
  formatDay,
  publicAvatarUrl,
  relativeTime,
} from "@/utils/publicCommunityFormat";

defineProps<{
  spotlight: Spotlight | null;
  anniversaries: Anniversary[];
  newcomers: Newcomer[];
  news: NewsItem[];
}>();
</script>

<template>
  <section class="mb-block mb-duo">
    <article class="mb-panel">
      <h3>Membre du mois</h3>
      <p v-if="!spotlight" class="mb-vide">
        Personne n'est encore mis en avant. Le staff distingue chaque mois quelqu'un qui a fait vivre le serveur.
      </p>
      <div v-else class="mb-mom">
        <img v-if="publicAvatarUrl(spotlight.avatar)" :src="publicAvatarUrl(spotlight.avatar)!" alt="" class="mb-mom-av" />
        <PublicMemberAvatar v-else :name="spotlight.username" size="lg" />
        <div>
          <div class="mb-mom-nom">{{ spotlight.username }}</div>
          <div class="mb-mom-quoi">{{ spotlight.reason }}</div>
        </div>
      </div>
    </article>

    <article class="mb-panel">
      <h3>Anniversaires à venir</h3>
      <p v-if="!anniversaries.length" class="mb-vide">Aucun anniversaire d'arrivée dans les deux prochaines semaines.</p>
      <ul v-else class="mb-annivs">
        <li v-for="anniversary in anniversaries" :key="anniversary.username + anniversary.joined_at" class="mb-anniv">
          <PublicMemberAvatar :name="anniversary.username" />
          <span class="mb-anniv-nom">{{ anniversary.username }}</span>
          <span class="mb-anniv-age">{{ anniversaryLabel(anniversary.years) }}</span>
          <span class="mb-anniv-date">le {{ formatDay(anniversary.joined_at) }}</span>
        </li>
      </ul>
    </article>
  </section>

  <section class="mb-block">
    <h2>Ils nous ont rejoints cette semaine <span class="mb-count">{{ newcomers.length }} nouveau(x)</span></h2>
    <p v-if="!newcomers.length" class="mb-vide">Personne de nouveau cette semaine.</p>
    <div v-else class="mb-nouveaux">
      <span v-for="newcomer in newcomers" :key="newcomer.username" class="mb-nv">
        <PublicMemberAvatar :name="newcomer.username" /><span>{{ newcomer.username }}</span>
      </span>
    </div>
  </section>

  <section class="mb-block">
    <h2>Les dernières annonces</h2>
    <p v-if="!news.length" class="mb-vide">Rien à annoncer pour le moment. Les nouvelles du serveur s'afficheront ici.</p>
    <div v-else class="mb-anns">
      <article v-for="item in news" :key="item.id" class="mb-ann" :class="{ pinned: item.is_pinned }">
        <img v-if="item.image_url" :src="item.image_url" alt="" class="mb-ann-img" />
        <div>
          <h3>{{ item.title }}</h3><p>{{ item.excerpt }}</p>
          <span class="mb-ann-when">{{ relativeTime(item.published_at) }}</span>
        </div>
      </article>
    </div>
  </section>
</template>
