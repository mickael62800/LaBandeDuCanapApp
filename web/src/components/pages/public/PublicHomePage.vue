<script setup lang="ts">
// Accueil PUBLIC du site communautaire — visible sans connexion.
//
// Page longue, assumée : un visiteur doit comprendre ce qu'est La Bande du
// Canapé, ce qu'on y fait, et avoir envie d'entrer. Le défilement est donc
// normal ici — contrairement au reste du site, cette page raconte quelque
// chose.
//
// Rendue hors de `MainLayout` (ni barre latérale, ni sélecteur de serveur) :
// c'est une vitrine, pas un back-office. Elle n'appelle QUE `/api/public/*`,
// hors de la pile d'authentification : aucun token, aucune donnée personnelle.
//
// Composition : le heros, les sections et les actions sont des composants
// partagés (`SiteHero`, `FeatureSection`, `ActionButton`). Cette page ne
// porte donc plus que son CONTENU et son enchaînement — l'apparence vit dans
// les briques, où elle est corrigée une fois pour les trois pages publiques.

import { onMounted, ref } from "vue";

import ActionButton from "@/components/atoms/ActionButton.vue";
import FeatureSection from "@/components/molecules/FeatureSection.vue";
import SiteHero from "@/components/molecules/SiteHero.vue";
import {
  guildIconUrl,
  publicSiteService,
  type PublicGuild,
} from "@/services/publicSiteService";
import { COMMUNITY, discordInvite } from "@/branding";
import { siteConfig } from "@/siteConfig";

/// Serveur mis en avant. Lu à l'exécution (cf. `siteConfig.ts`), avec repli
/// sur la variable de build pour le développement local.
const guildId =
  siteConfig().guildId ||
  ((import.meta.env.VITE_PUBLIC_GUILD_ID as string | undefined) ?? "");

const guild = ref<PublicGuild | null>(null);
const iconUrl = ref<string | null>(null);

onMounted(async () => {
  if (!guildId) return;
  try {
    const g = await publicSiteService.guild(guildId);
    guild.value = g;
    iconUrl.value = guildIconUrl(g);
  } catch {
    // Vitrine indisponible : on n'affiche pas d'erreur technique à un
    // visiteur, la page garde tout son sens sans ce bloc.
    guild.value = null;
  }
});

/// Sections alternées : l'illustration passe à droite puis à gauche.
const SECTIONS = [
  {
    id: "jeux",
    surtitre: "Nos serveurs",
    titre: "On monte les serveurs, tu joues",
    texte:
      "Minecraft, Palworld et d'autres : nos serveurs se créent en quelques clics et tournent sur notre propre machine. Pas de file d'attente, pas de publicité, pas de loyer mensuel à payer pour jouer entre nous.",
    points: [
      "Serveurs montés à la demande, éteints quand personne ne joue",
      "Sauvegardes automatiques du monde",
      "Un salon Discord dédié créé pour chaque session",
    ],
    image: "/site/section-jeux.jpg",
    alt: "Serveurs de jeu de la communauté",
  },
  {
    id: "vocal",
    surtitre: "La vie du serveur",
    titre: "Des vocaux ouverts, tout le temps",
    texte:
      "Le cœur de la bande, c'est le vocal. Il y a presque toujours quelqu'un. On y parle de tout, on y joue, ou on laisse simplement tourner en fond pendant qu'on fait autre chose.",
    points: [
      "Salons vocaux créés automatiquement quand il en manque",
      "Personne n'est obligé de parler : venir écouter, ça compte",
      "Des salons privés à la demande pour les petits groupes",
    ],
    image: "/site/section-vocal.jpg",
    alt: "Salons vocaux de la communauté",
  },
  {
    id: "planning",
    surtitre: "Le planning",
    titre: "Ce qui arrive, et quand",
    texte:
      "Une soirée un mardi soir, une saison Minecraft qui dure trois semaines, une campagne Palworld sur un mois : le planning affiche les deux, en vue semaine ou en vue mois. Tu vois d'un coup d'œil ce qui tourne et ce qui se prépare.",
    points: [
      "Vue semaine pour les soirées, vue mois pour les campagnes",
      "Les événements longs restent visibles toute leur durée",
      "Inscription en un clic, avec les « peut-être » qui comptent aussi",
    ],
    image: "/site/section-planning.jpg",
    alt: "Planning des événements et campagnes",
  },
  {
    id: "animation",
    surtitre: "Concours",
    titre: "Tirages au sort et petits jeux",
    texte:
      "Des giveaways réguliers, une monnaie maison à dépenser, et quelques jeux internes qui tournent directement sur Discord. Rien d'obligatoire, tout est là pour l'ambiance — et pour le plaisir de gagner un truc de temps en temps.",
    points: [
      "Tirages au sort transparents, gagnants annoncés publiquement",
      "Une monnaie du serveur à gagner et à dépenser",
      "Des jeux internes jouables sans rien installer",
    ],
    image: "/site/section-animation.jpg",
    alt: "Concours et tirages au sort",
  },
  {
    id: "classements",
    surtitre: "Classements",
    titre: "Qui traîne vraiment le plus sur le canapé",
    texte:
      "Temps passé en vocal, messages échangés, niveaux gagnés : tout est compté, et affiché. Sans prise de tête, juste de quoi savoir qui squatte le plus et se chambrer en connaissance de cause.",
    points: [
      "Classement du temps en vocal et des messages",
      "Niveaux et expérience gagnés en participant",
      "Statistiques du serveur, mois par mois",
    ],
    image: "/site/section-classements.jpg",
    alt: "Classements de la communauté",
  },
  {
    id: "moderation",
    surtitre: "Un cadre sain",
    titre: "Modéré, sans être fliqué",
    texte:
      "On tient à ce que le canapé reste confortable. La modération est présente et outillée, mais discrète : elle intervient sur ce qui gêne réellement, pas sur les blagues.",
    points: [
      "Règles claires, affichées, appliquées de la même façon pour tous",
      "Détection automatique du spam et des raids",
      "Chaque décision est tracée et peut être contestée",
    ],
    image: "/site/section-moderation.jpg",
    alt: "Une communauté modérée",
  },
];

/// Réglages d'apparition partagés : léger décalage vers le haut, une seule
/// fois, au passage dans le champ de vision.
const APPEAR = {
  initial: { opacity: 0, y: 28 },
  visibleOnce: { opacity: 1, y: 0, transition: { duration: 550 } },
};
</script>

<template>
  <div class="ph theme-communaute">
    <SiteHero
      v-motion
      :initial="{ opacity: 0, scale: 0.94 }"
      :enter="{ opacity: 1, scale: 1, transition: { duration: 700 } }"
      :tagline="COMMUNITY.tagline"
    >
      <template v-if="guild" #info>
        <span class="ph-guild">
          <img v-if="iconUrl" :src="iconUrl" :alt="guild.name" class="ph-guild-icon" />
          <span>
            <strong>{{ guild.member_count.toLocaleString("fr-FR") }}</strong>
            membres sur {{ guild.name }}
          </span>
        </span>
      </template>

      <!-- Trois actions DISTINCTES : entrer sur Discord (la vraie
           conversion), regarder ce qui s'y passe sans compte, ou parcourir
           la page. -->
      <template #actions>
        <ActionButton v-if="discordInvite()" :href="discordInvite()" size="lg">
          Rejoindre Discord
        </ActionButton>
        <ActionButton to="/membre" variant="secondary" size="lg">
          Voir la vie du serveur
        </ActionButton>
        <ActionButton href="#jeux" variant="ghost" size="lg">Découvrir</ActionButton>
      </template>
    </SiteHero>

    <!-- ── Présentation ── -->
    <section v-motion="APPEAR" class="ph-about">
      <h2>C'est quoi, La Bande du Canapé&nbsp;?</h2>
      <p>
        Un serveur Discord sans prise de tête. On s'y retrouve pour jouer, pour
        discuter, ou juste pour avoir un peu de bruit de fond pendant qu'on fait
        autre chose. Pas de niveau minimum, pas d'audition&nbsp;: si tu poses tes
        fesses sur le canapé, tu fais partie de la bande.
      </p>
    </section>

    <!-- ── Sections alternées ── -->
    <FeatureSection
      v-for="(s, i) in SECTIONS"
      :id="s.id"
      :key="s.id"
      v-motion="APPEAR"
      :surtitre="s.surtitre"
      :titre="s.titre"
      :texte="s.texte"
      :points="s.points"
      :image="s.image"
      :alt="s.alt"
      :inverse="i % 2 === 1"
    />

    <!-- ── Appel final ── -->
    <section v-motion="APPEAR" class="ph-cta">
      <h2>Le canapé est large, il reste de la place</h2>
      <p>Regarde ce qui s'y passe, sans compte et sans engagement.</p>

      <ActionButton v-if="discordInvite()" :href="discordInvite()" size="lg">
        Rejoindre Discord
      </ActionButton>
      <ActionButton v-else to="/membre" size="lg">Voir la vie du serveur</ActionButton>
    </section>

    <!-- L'administration ne concerne qu'une poignée de personnes : un lien
         discret en pied de page, pas une porte au même rang que l'entrée. -->
    <footer class="ph-footer">
      <RouterLink to="/login?espace=admin">Administration</RouterLink>
    </footer>
  </div>
</template>

<style scoped>
/* Cette page ne définit plus aucune couleur : tout vient de
   `.theme-communaute` (cf. styles/global.css). Il ne reste ici que la mise en
   page propre à l'accueil. */
.ph {
  flex: 1;
  position: relative;
  /* Le heros déborde volontairement : sans ça, il créerait une barre de
     défilement horizontale. */
  overflow-x: hidden;
  overflow-y: auto;
  padding: clamp(2rem, 6vh, 4rem) var(--space-lg) clamp(2rem, 5vh, 4rem);
  display: flex;
  flex-direction: column;
  gap: clamp(3rem, 9vh, 6rem);
}

.ph > * {
  width: 100%;
  max-width: 68rem;
  margin: 0 auto;
}

/* ── Chiffre du serveur ── */
.ph-guild {
  display: inline-flex;
  align-items: center;
  gap: var(--space-sm);
  padding: var(--space-sm) var(--space-lg);
  border-radius: var(--radius-pill);
  background: var(--bg-card);
  border: 1px solid var(--border);
  color: var(--text-secondary);
  font-size: 0.92rem;
}

.ph-guild strong {
  color: var(--text-primary);
  font-variant-numeric: tabular-nums;
}

.ph-guild-icon {
  width: 26px;
  height: 26px;
  border-radius: 50%;
}

/* ── Présentation ── */
.ph-about {
  max-width: 46rem;
  text-align: center;
}

.ph-about h2 {
  margin: 0 0 var(--space-lg);
  font-size: clamp(1.5rem, 3.5vw, 2rem);
  text-wrap: balance;
}

.ph-about p {
  margin: 0;
  color: var(--text-secondary);
  font-size: 1.02rem;
  line-height: 1.7;
}

/* ── Appel final ── */
.ph-cta {
  max-width: 46rem;
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-lg);
  padding: clamp(2rem, 5vh, 3rem) var(--space-xl);
  border-radius: var(--radius-xl);
  background: var(--bg-card);
  border: 1px solid var(--border);
}

.ph-cta h2 {
  margin: 0;
  font-size: clamp(1.4rem, 3vw, 1.85rem);
  text-wrap: balance;
}

.ph-cta p {
  margin: 0;
  color: var(--text-secondary);
}

/* ── Pied de page ── */
.ph-footer {
  text-align: center;
  padding-top: var(--space-lg);
}

.ph-footer a {
  color: var(--site-ink-4);
  font-size: 0.88rem;
  text-decoration: none;
}

.ph-footer a:hover {
  color: var(--text-secondary);
}
</style>
