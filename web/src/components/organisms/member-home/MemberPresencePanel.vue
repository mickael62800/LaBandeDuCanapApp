<script setup lang="ts">
import PublicMemberAvatar from "@/components/atoms/PublicMemberAvatar.vue";
import type { Presence } from "@/services/communityLifeService";
import { relativeTime } from "@/utils/publicCommunityFormat";

defineProps<{ presence: Presence }>();
</script>

<template>
  <section class="mb-block">
    <h2>
      <span class="mb-live" aria-hidden="true"></span> En vocal maintenant
      <span v-if="presence.voice_total" class="mb-count">{{ presence.voice_total }} personne(s)</span>
    </h2>
    <p v-if="!presence.voice.length" class="mb-vide">Personne en vocal pour le moment.</p>
    <div v-else class="mb-vocaux">
      <article v-for="channel in presence.voice" :key="channel.channel_name" class="mb-vc">
        <header class="mb-vc-head">
          <span aria-hidden="true">{{ channel.restricted ? "🔒" : "🔊" }}</span>
          <span class="mb-vc-nom">{{ channel.channel_name }}</span>
          <span v-if="channel.restricted" class="mb-vc-prive" title="Salon réservé">privé</span>
          <span class="mb-vc-n">{{ channel.members.length }}</span>
        </header>
        <ul class="mb-vc-list">
          <li v-for="member in channel.members" :key="member.username" class="mb-vm">
            <PublicMemberAvatar :name="member.username" size="sm" />
            <span class="mb-vm-nom">{{ member.username }}</span>
            <span v-if="member.streaming" class="mb-vm-ico" title="Partage son écran">🖥️</span>
            <span v-else-if="member.video" class="mb-vm-ico" title="Caméra activée">📹</span>
            <span v-if="member.muted" class="mb-vm-ico" title="Micro coupé">🔇</span>
          </li>
        </ul>
      </article>
    </div>
  </section>

  <section class="mb-block">
    <h2>Ça discute aussi à l'écrit</h2>
    <p v-if="!presence.text.length" class="mb-vide">Aucun salon actif dans le dernier quart d'heure.</p>
    <ul v-else class="mb-textes">
      <li v-for="channel in presence.text" :key="channel.channel_name" class="mb-tc">
        <span class="mb-tc-hash" aria-hidden="true">#</span>
        <span class="mb-tc-nom">{{ channel.channel_name }}</span>
        <span class="mb-tc-avs">
          <PublicMemberAvatar
            v-for="author in channel.recent_authors.slice(0, 6)"
            :key="author"
            :name="author"
            :title="author"
            size="sm"
          />
        </span>
        <span class="mb-tc-when">{{ relativeTime(channel.last_message_at) }}</span>
      </li>
    </ul>
  </section>
</template>
