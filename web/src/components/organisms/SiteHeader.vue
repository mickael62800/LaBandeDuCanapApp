<script setup lang="ts">
// En-tete du SITE PUBLIC communautaire : marque, navigation, session.
//
// Pendant de `TopBar` pour le back-office. Meme granularite : c'est un
// organism, compose par une template (`PublicLayout`), jamais utilise
// directement par une page.
//
// POURQUOI IL EXISTE
//
// Chaque page publique portait sa propre barre : `.mb-bar` (connexion et
// deconnexion) dans l'espace membre, `.jx-bar` (retour + solde) dans les jeux,
// et RIEN sur l'accueil — un visiteur arrivant sur la page d'accueil n'avait
// donc aucun moyen de se connecter ni de naviguer. Trois pages, trois
// en-tetes, une navigation absente.
//
// L'identite affichee est celle de la COMMUNAUTE, pas celle d'un univers
// d'administration : un visiteur arrive chez la communaute, pas chez un outil.

import { computed } from "vue";
import { RouterLink, useRoute } from "vue-router";
import ActionButton from "../atoms/ActionButton.vue";
import { useAuth } from "../../composables/useAuth";
import { COMMUNITY, discordInvite, onLogoError } from "@/branding";

const route = useRoute();
const { user, logout, avatarUrl } = useAuth();

const NAV = [
  { to: "/", label: "Accueil" },
  { to: "/membre", label: "Espace membre" },
  { to: "/jeux", label: "Jeux" },
];

function isActive(to: string): boolean {
  return to === "/" ? route.path === "/" : route.path.startsWith(to);
}

/// Passerelle vers le back-office, montree seulement a qui peut s'en servir.
/// Sans ce lien, un administrateur connecte cote site public n'a aucun moyen
/// d'atteindre l'administration sans taper l'URL a la main.
const canAdminister = computed(() => user.value?.is_superadmin === true);
</script>

<template>
  <header class="site-bar">
    <RouterLink to="/" class="site-brand">
      <img
        :src="COMMUNITY.mark"
        :alt="COMMUNITY.name"
        class="site-logo"
        @error="onLogoError"
      />
      <span class="site-name">{{ COMMUNITY.name }}</span>
    </RouterLink>

    <nav class="site-nav" aria-label="Navigation du site">
      <RouterLink
        v-for="n in NAV"
        :key="n.to"
        :to="n.to"
        class="site-link"
        :class="{ active: isActive(n.to) }"
      >
        {{ n.label }}
      </RouterLink>
    </nav>

    <div class="site-spacer" />

    <ActionButton
      v-if="discordInvite()"
      :href="discordInvite()"
      variant="secondary"
      size="md"
    >
      Rejoindre Discord
    </ActionButton>

    <RouterLink v-if="canAdminister" to="/dashboard" class="site-link admin-link">
      Administration
    </RouterLink>

    <div v-if="user" class="site-user">
      <img :src="avatarUrl(user)" alt="" class="site-avatar" />
      <span class="site-username">{{ user.global_name ?? user.username }}</span>
      <ActionButton variant="ghost" size="md" @click="logout">
        Déconnexion
      </ActionButton>
    </div>
    <ActionButton v-else to="/login?espace=membre" variant="ghost" size="md">
      Se connecter
    </ActionButton>
  </header>
</template>

<style scoped>
.site-bar {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 10px 20px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-secondary);
  position: sticky;
  top: 0;
  z-index: 20;
}

.site-brand {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  text-decoration: none;
  color: var(--text-primary);
  font-weight: 700;
}
.site-logo {
  width: 26px;
  height: 26px;
  object-fit: contain;
}
.site-name {
  white-space: nowrap;
}

.site-nav {
  display: flex;
  gap: 4px;
}

.site-link {
  padding: 6px 12px;
  border-radius: var(--radius-pill);
  color: var(--text-secondary);
  text-decoration: none;
  font-size: 0.9rem;
  font-weight: 500;
}
.site-link:hover {
  color: var(--text-primary);
  background: color-mix(in srgb, var(--accent) 10%, transparent);
}
.site-link.active {
  color: var(--text-primary);
  background: color-mix(in srgb, var(--accent) 16%, transparent);
  font-weight: 600;
}

.admin-link {
  border: 1px solid var(--border);
}

.site-spacer {
  flex: 1;
}

.site-user {
  display: inline-flex;
  align-items: center;
  gap: 8px;
}
.site-avatar {
  width: 28px;
  height: 28px;
  border-radius: 50%;
}
.site-username {
  font-size: 0.9rem;
  color: var(--text-secondary);
  white-space: nowrap;
}

/* Mobile : la navigation reste, le superflu disparait. La barre ne doit
   jamais pousser le viewport horizontalement. */
@media (max-width: 760px) {
  .site-bar {
    gap: 8px;
    padding: 8px 12px;
    flex-wrap: wrap;
  }
  .site-name,
  .site-username {
    display: none;
  }
}
</style>
