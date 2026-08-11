<script setup lang="ts">
import ActionButton from "@/components/atoms/ActionButton.vue";
import SiteHero from "@/components/molecules/SiteHero.vue";
import MemberCommunityHighlights from "@/components/organisms/member-home/MemberCommunityHighlights.vue";
import MemberEventsPanel from "@/components/organisms/member-home/MemberEventsPanel.vue";
import MemberGameServersPanel from "@/components/organisms/member-home/MemberGameServersPanel.vue";
import MemberLfgPanel from "@/components/organisms/member-home/MemberLfgPanel.vue";
import MemberPlanningPanel from "@/components/organisms/member-home/MemberPlanningPanel.vue";
import MemberPollsPanel from "@/components/organisms/member-home/MemberPollsPanel.vue";
import MemberPresencePanel from "@/components/organisms/member-home/MemberPresencePanel.vue";
import { discordInvite } from "@/branding";
import { useMemberHomePage } from "@/composables/useMemberHomePage";

const {
  guildId,
  user,
  hasAdminAccess,
  allFailed,
  events,
  servers,
  lfg,
  polls,
  spotlight,
  anniversaries,
  newcomers,
  news,
  presence,
  loadingEvents,
  loadingServers,
  loadingLfg,
  busyLfg,
  lfgError,
  busyVote,
  playersOnline,
  serversOnline,
  ongoing,
  nextEvent,
  upcoming,
  joinLfg,
  vote,
} = useMemberHomePage();
</script>

<template>
  <div class="mb theme-communaute">
    <SiteHero
      :tagline="user
        ? `Content de te revoir, ${user.username}. Voici ce qui se passe.`
        : 'Ce qui se passe en ce moment. Connecte-toi pour participer.'"
    >
      <template #info>
        <span v-if="serversOnline" class="mb-chip">
          <span class="mb-pip on"></span><b>{{ serversOnline }}</b> serveur(s) en ligne
        </span>
        <span v-if="playersOnline" class="mb-chip"><b>{{ playersOnline }}</b> joueur(s) en jeu</span>
        <span v-if="lfg.length" class="mb-chip"><b>{{ lfg.length }}</b> recherche(s) de joueurs</span>
      </template>
      <template #actions>
        <ActionButton to="/jeux" variant="secondary">🎡 Les jeux</ActionButton>
        <ActionButton v-if="discordInvite()" :href="discordInvite()">Rejoindre le Discord</ActionButton>
      </template>
    </SiteHero>

    <section v-if="!guildId" class="mb-block">
      <p class="mb-config">
        Le serveur à afficher n'est pas configuré. Définis <code>PUBLIC_GUILD_ID</code>
        dans <code>infrastructure/docker/.env</code>, puis redémarre le conteneur web.
      </p>
    </section>
    <section v-if="allFailed" class="mb-block">
      <p class="mb-config">
        Le site n'arrive pas à joindre l'API. Vérifie que les conteneurs
        <code>api</code> et <code>nexus-api</code> tournent.
      </p>
    </section>

    <MemberEventsPanel
      section="ongoing"
      :ongoing="ongoing"
      :next-event="nextEvent"
      :upcoming="upcoming"
      :authenticated="!!user"
    />
    <MemberGameServersPanel :servers="servers" :loading="loadingServers" />
    <MemberLfgPanel
      :posts="lfg"
      :loading="loadingLfg"
      :authenticated="!!user"
      :busy-id="busyLfg"
      :error="lfgError"
      @join="joinLfg"
    />
    <MemberPlanningPanel :events="events" :loading="loadingEvents" />
    <MemberEventsPanel
      section="upcoming"
      :ongoing="ongoing"
      :next-event="nextEvent"
      :upcoming="upcoming"
      :authenticated="!!user"
    />
    <MemberPresencePanel :presence="presence" />
    <MemberPollsPanel :polls="polls" :authenticated="!!user" :busy-id="busyVote" @vote="vote" />
    <MemberCommunityHighlights
      :spotlight="spotlight"
      :anniversaries="anniversaries"
      :newcomers="newcomers"
      :news="news"
    />

    <footer class="mb-footer">
      <RouterLink v-if="hasAdminAccess" to="/dashboard" class="mb-admin-link">
        🛡️ Accéder à l'administration
      </RouterLink>
    </footer>
  </div>
</template>
<style src="../../../styles/member-home.css"></style>
