<script setup lang="ts">
import ActionButton from "@/components/atoms/ActionButton.vue";
import PublicMemberAvatar from "@/components/atoms/PublicMemberAvatar.vue";
import type { PublicLfgPost } from "@/services/communityLifeService";
import { relativeTime } from "@/utils/publicCommunityFormat";

defineProps<{
  posts: PublicLfgPost[];
  loading: boolean;
  authenticated: boolean;
  busyId: string | null;
  error: string | null;
}>();
const emit = defineEmits<{ join: [id: string] }>();
</script>

<template>
  <section class="mb-block">
    <h2>
      Cherche des joueurs
      <span v-if="posts.length" class="mb-count">{{ posts.length }} annonce(s) ouverte(s)</span>
    </h2>
    <p v-if="loading" class="mb-hint">Chargement des annonces…</p>
    <p v-else-if="!posts.length" class="mb-hint">
      Personne ne cherche de monde pour l'instant. Lance la première annonce&nbsp;!
    </p>
    <div v-else class="mb-lfgs">
      <article v-for="post in posts" :key="post.id" class="mb-lfg">
        <div class="mb-lfg-top">
          <PublicMemberAvatar :name="post.author_name || '?'" />
          <span class="mb-lfg-auteur">{{ post.author_name || "Un membre" }}</span>
          <span class="mb-tag">{{ post.game }}</span>
          <span class="mb-lfg-quand">{{ relativeTime(post.created_at) }}</span>
        </div>
        <p v-if="post.description" class="mb-lfg-texte">{{ post.description }}</p>
        <div class="mb-lfg-foot">
          <span class="mb-lfg-besoin">Cherche <b>{{ post.slots }}</b> joueur(s) · {{ post.when_text }}</span>
          <span class="mb-lfg-avs">
            <PublicMemberAvatar
              v-for="(name, index) in post.interested_names.slice(0, 5)"
              :key="index"
              :name="name"
              :title="name"
              size="sm"
            />
            <span v-if="post.interested_names.length" class="mb-lfg-n">
              {{ post.interested_names.length }} intéressé(s)
            </span>
            <span v-else class="mb-lfg-n muted">personne encore</span>
          </span>
          <button
            v-if="authenticated"
            type="button"
            class="mb-lfg-btn"
            :disabled="busyId === post.id"
            @click="emit('join', post.id)"
          >{{ busyId === post.id ? "…" : "Je viens" }}</button>
          <ActionButton v-else to="/login?espace=membre" variant="secondary">
            Se connecter pour répondre
          </ActionButton>
        </div>
      </article>
    </div>
    <p v-if="error" class="mb-erreur">{{ error }}</p>
  </section>
</template>
