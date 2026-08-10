<script setup lang="ts">
// Espace membre — la vie du serveur, consultable SANS connexion.
//
// Choix structurant : cette page est publique. Un visiteur doit pouvoir voir
// ce qui se passe — le planning, les serveurs en ligne, qui cherche des
// joueurs — avant de décider de créer un compte. Demander la connexion à
// l'entrée revenait à mettre un videur devant une vitrine.
//
// La connexion n'est requise que pour AGIR : s'inscrire, voter, dire « je
// viens ». Les boutons d'action affichent alors une invitation à se connecter
// plutôt que de disparaître — l'utilisateur comprend ce qu'il gagnerait.
//
// Chaque section charge indépendamment : une plateforme jeux indisponible ne
// doit pas priver la page de son planning.
//
// Les sections restent AFFICHÉES même vides, avec un texte qui annonce ce qui
// s'y trouvera. Les masquer paraissait plus propre, mais sur une communauté
// qui démarre — base neuve, aucun contenu — il ne restait que le logo, et la
// page semblait cassée alors qu'elle fonctionnait.
//
// Rendue hors de `MainLayout` : un membre n'a rien à faire dans la barre
// latérale d'administration.

import { computed, onMounted, onUnmounted, ref } from "vue";
import { useAuth } from "@/composables/useAuth";
import {
  addWeeks,
  layoutWeek,
  startOfWeek,
  weekDays,
  weekLabel,
} from "@/composables/useWeekPlanning";
import ActionButton from "@/components/atoms/ActionButton.vue";
import SiteHero from "@/components/molecules/SiteHero.vue";
import { discordInvite } from "@/branding";
import { siteConfig } from "@/siteConfig";
import {
  isOngoing,
  publicEventsService,
  type PublicEvent,
} from "@/services/publicEventsService";
import {
  publicGamesService,
  type PublicGameServer,
} from "@/services/publicGamesService";
import {
  communityActionsService,
  communityLifeService,
  type Anniversary,
  type Newcomer,
  type NewsItem,
  type Poll,
  type Presence,
  type PublicLfgPost,
  type Spotlight,
} from "@/services/communityLifeService";

/// Serveur dont on affiche la vie. Lu à l'exécution depuis `site-config.json`
/// (écrit par l'entrypoint nginx), avec repli sur la variable de build pour le
/// développement local.
///
/// Sans lui, la page ne sait pas de quel serveur parler : elle n'affiche que
/// l'accueil, et l'explique au lieu de rester muette.
const guildId =
  siteConfig().guildId ||
  ((import.meta.env.VITE_PUBLIC_GUILD_ID as string | undefined) ?? "");

// Avatar et deconnexion sont desormais rendus par `SiteHeader` : la page ne
// lit plus que l'identite, pour personnaliser son accueil.
const { user } = useAuth();

// Le lien vers l'administration n'apparait que pour un superadmin (flag pose a
// l'OAuth / au refresh, source de verite = SUPERADMIN_USER_IDS cote API). Un
// visiteur anonyme ou un membre ordinaire ne le voit pas ; l'acces reel reste
// de toute facon tranche cote serveur (403) sur chaque route d'admin.
const hasAdminAccess = computed(() => user.value?.is_superadmin === true);

// ── État ──

const events = ref<PublicEvent[]>([]);
const servers = ref<PublicGameServer[]>([]);
const lfg = ref<PublicLfgPost[]>([]);
const polls = ref<Poll[]>([]);
const spotlight = ref<Spotlight | null>(null);
const anniversaries = ref<Anniversary[]>([]);
const newcomers = ref<Newcomer[]>([]);
const news = ref<NewsItem[]>([]);
const presence = ref<Presence>({ voice: [], voice_total: 0, text: [] });

/// Rafraîchissement de la présence. Elle est la seule donnée qui change à la
/// minute : tout recharger pour elle serait du gâchis, la laisser figée
/// afficherait un salon vide comme occupé.
const RAFRAICHISSEMENT_MS = 20_000;
let timerPresence: number | undefined;

/// Nombre de requêtes publiques en échec.
///
/// Sans lui, une base vide et une API en panne donnent exactement la même
/// page — et on cherche le problème du mauvais côté. Ce compteur permet de
/// dire lequel des deux on regarde.
const echecs = ref(0);
const APPELS_PUBLICS = 6;

function echec() {
  echecs.value += 1;
}

/// Tout est tombé : c'est une panne, pas un serveur calme.
const toutEnEchec = computed(() => echecs.value >= APPELS_PUBLICS);

const loadingEvents = ref(true);
const loadingServers = ref(true);
const loadingLfg = ref(true);

/// Semaine affichée dans le calendrier. Décalable sans recharger : la fenêtre
/// interrogée couvre déjà deux mois.
const weekStart = ref(startOfWeek(new Date()));

onMounted(() => {
  if (!guildId) {
    loadingEvents.value = false;
    loadingServers.value = false;
    loadingLfg.value = false;
    return;
  }

  // Fenêtre large : ce qui est en cours (commencé avant aujourd'hui) et ce qui
  // arrive dans les deux mois. Permet de naviguer de semaine en semaine sans
  // nouvel appel.
  const from = new Date();
  from.setDate(from.getDate() - 30);
  const to = new Date();
  to.setDate(to.getDate() + 60);

  // Appels indépendants : l'échec de l'un ne prive pas la page des autres.
  publicEventsService
    .list(guildId, from, to)
    .then((r) => (events.value = r))
    .catch(() => {
      echec();
      events.value = [];
    })
    .finally(() => (loadingEvents.value = false));

  publicGamesService
    .listServers(guildId)
    .then((r) => (servers.value = r))
    .catch(() => {
      echec();
      servers.value = [];
    })
    .finally(() => (loadingServers.value = false));

  communityLifeService
    .lfg(guildId)
    .then((r) => (lfg.value = r))
    .catch(() => {
      echec();
      lfg.value = [];
    })
    .finally(() => (loadingLfg.value = false));

  // Un membre connecté voit son propre vote pré-coché, ce que la surface
  // publique ne peut pas renseigner.
  const chargerSondages = user.value
    ? communityActionsService.myPolls(guildId)
    : communityLifeService.polls(guildId);
  chargerSondages
    .then((r) => (polls.value = r.filter((p) => p.is_open).slice(0, 2)))
    .catch(() => {
      echec();
      polls.value = [];
    });

  communityLifeService
    .spotlight(guildId)
    .then((r) => (spotlight.value = r))
    .catch(() => {
      echec();
      spotlight.value = null;
    });

  communityLifeService
    .pulse(guildId)
    .then((r) => {
      anniversaries.value = r.anniversaries;
      newcomers.value = r.newcomers;
    })
    .catch(() => {
      echec();
      anniversaries.value = [];
      newcomers.value = [];
    });

  communityLifeService
    .news(guildId)
    .then((r) => (news.value = r))
    .catch(() => {
      echec();
      news.value = [];
    });

  chargerPresence();
  timerPresence = window.setInterval(chargerPresence, RAFRAICHISSEMENT_MS);
});

/// Le timer survivrait à la navigation et continuerait d'interroger l'API
/// depuis une page démontée.
onUnmounted(() => {
  if (timerPresence) window.clearInterval(timerPresence);
});

function chargerPresence() {
  if (!guildId) return;
  communityLifeService
    .presence(guildId)
    // En cas d'échec on VIDE plutôt que de garder l'état précédent : afficher
    // « 11 en vocal » figé depuis dix minutes est pire que ne rien afficher.
    .then((r) => (presence.value = r))
    .catch(() => (presence.value = { voice: [], voice_total: 0, text: [] }));
}

// ── Serveurs de jeu ──

/// En ligne d'abord : c'est ce qu'on vient chercher.
const sortedServers = computed(() =>
  [...servers.value].sort((a, b) => Number(b.online) - Number(a.online)),
);

const playersOnline = computed(() =>
  servers.value.reduce((n, s) => n + (s.online ? s.player_count : 0), 0),
);

const serversOnline = computed(() => servers.value.filter((s) => s.online).length);

// ── Planning ──

const weekBars = computed(() => layoutWeek(events.value, weekStart.value));
const days = computed(() => weekDays(weekStart.value));
const label = computed(() => weekLabel(weekStart.value));

/// Nombre de lignes de la grille. Minimum 2, sinon une semaine à un seul
/// événement donnerait un calendrier écrasé sur une bande.
const weekRows = computed(() =>
  Math.max(2, ...weekBars.value.map((b) => b.row), 0),
);

/// Recalculée à chaque clic plutôt que figée au chargement : un onglet laissé
/// ouvert une nuit ramènerait sinon à la semaine de la veille.
function semaineCourante(): Date {
  return startOfWeek(new Date());
}

function isToday(d: Date): boolean {
  const now = new Date();
  return (
    d.getDate() === now.getDate() &&
    d.getMonth() === now.getMonth() &&
    d.getFullYear() === now.getFullYear()
  );
}

const ongoing = computed(() => events.value.filter((e) => isOngoing(e)));

/// Le prochain rendez-vous, mis en avant. Le premier à venir, pas le plus
/// proche d'aujourd'hui : une campagne en cours n'est pas un rendez-vous.
const nextEvent = computed(() => {
  const now = new Date();
  return (
    events.value
      .filter((e) => new Date(e.starts_at) > now)
      .sort((a, b) => a.starts_at.localeCompare(b.starts_at))[0] ?? null
  );
});

const upcoming = computed(() => {
  const now = new Date();
  return events.value
    .filter((e) => new Date(e.starts_at) > now)
    .sort((a, b) => a.starts_at.localeCompare(b.starts_at))
    .slice(1, 5);
});

// ── Actions ──

const busyLfg = ref<string | null>(null);
const lfgError = ref<string | null>(null);

/// « Je viens ». Recharge la liste publique après coup : la réponse
/// authentifiée porte les identifiants, la vue publique n'en veut pas.
async function joinLfg(id: string) {
  if (!user.value || !guildId) return;
  busyLfg.value = id;
  lfgError.value = null;
  try {
    await communityActionsService.joinLfg(id);
    lfg.value = await communityLifeService.lfg(guildId);
  } catch (e) {
    lfgError.value = e instanceof Error ? e.message : "Impossible de rejoindre.";
  } finally {
    busyLfg.value = null;
  }
}

const busyVote = ref<string | null>(null);

async function vote(pollId: string, optionId: string) {
  if (!user.value) return;
  busyVote.value = pollId;
  try {
    const maj = await communityActionsService.vote(pollId, optionId);
    polls.value = polls.value.map((p) => (p.id === pollId ? maj : p));
  } catch {
    // Un vote qui échoue laisse les barres inchangées : rien à annoncer de
    // plus que l'absence de mouvement.
  } finally {
    busyVote.value = null;
  }
}

// ── Formats ──

const JOUR: Intl.DateTimeFormatOptions = { weekday: "short", day: "numeric", month: "short" };
const HEURE: Intl.DateTimeFormatOptions = { hour: "2-digit", minute: "2-digit" };

function fmtRange(e: PublicEvent): string {
  const start = new Date(e.starts_at);
  const end = new Date(e.ends_at);

  // Une campagne s'annonce par ses dates, une soirée par son horaire :
  // afficher « 21:00 » pour un événement de trois semaines n'aurait aucun sens.
  if (e.span_days > 1) {
    return `${start.toLocaleDateString("fr-FR", JOUR)} → ${end.toLocaleDateString("fr-FR", JOUR)}`;
  }
  if (e.all_day) return start.toLocaleDateString("fr-FR", JOUR);
  return `${start.toLocaleDateString("fr-FR", JOUR)} · ${start.toLocaleTimeString("fr-FR", HEURE)}`;
}

function fmtHeure(iso: string): string {
  return new Date(iso).toLocaleTimeString("fr-FR", HEURE);
}

function fmtJour(iso: string): string {
  return new Date(iso).toLocaleDateString("fr-FR", { day: "numeric", month: "long" });
}

/// Ancienneté en clair. Les repères courts d'abord : c'est ce qu'on regarde.
function depuis(iso: string): string {
  const ms = Date.now() - new Date(iso).getTime();
  const minutes = Math.floor(ms / 60000);
  if (minutes < 1) return "à l'instant";
  if (minutes < 60) return `il y a ${minutes} min`;
  const heures = Math.floor(minutes / 60);
  if (heures < 24) return `il y a ${heures} h`;
  const jours = Math.floor(heures / 24);
  return jours === 1 ? "hier" : `il y a ${jours} jours`;
}

function accent(e: PublicEvent): string | undefined {
  return e.color ? `#${e.color}` : undefined;
}

/// Palette des pastilles d'avatar, choisie de façon stable à partir du pseudo
/// pour qu'une même personne garde sa couleur d'un chargement à l'autre.
const PALETTE = ["#a855f7", "#22c55e", "#f39c12", "#c026d3", "#38bdf8", "#f43f5e", "#14b8a6"];

function couleurDe(nom: string): string {
  let somme = 0;
  for (const c of nom) somme += c.codePointAt(0) ?? 0;
  return PALETTE[somme % PALETTE.length];
}

function initiale(nom: string): string {
  return (nom.trim()[0] ?? "?").toUpperCase();
}

/// Les surfaces publiques ne publient pas les identifiants Discord — c'est
/// délibéré, ils permettraient de retrouver quelqu'un hors du serveur. Or une
/// URL d'avatar Discord se construit à partir de cet identifiant. On affiche
/// donc toujours la pastille à initiale, sauf si l'API a fourni une URL
/// complète (cas d'un avatar hébergé ailleurs).
function avatarUrlDe(hash: string | null): string | null {
  return hash?.startsWith("http") ? hash : null;
}

const anneesLabel = (n: number) => (n === 1 ? "1 an" : `${n} ans`);
</script>

<template>
  <div class="mb theme-communaute">
    <!-- ── Accueil ──
         Le wordmark est une illustration complète, avec son propre décor : il
         occupe seul le héros. Le poser sur une photo ferait deux images qui
         se coupent. -->
    <SiteHero
      :tagline="
        user
          ? `Content de te revoir, ${user.username}. Voici ce qui se passe.`
          : 'Ce qui se passe en ce moment. Connecte-toi pour participer.'
      "
    >
      <template #info>
        <span v-if="serversOnline" class="mb-chip">
          <span class="mb-pip on"></span><b>{{ serversOnline }}</b> serveur(s) en ligne
        </span>
        <span v-if="playersOnline" class="mb-chip"><b>{{ playersOnline }}</b> joueur(s) en jeu</span>
        <span v-if="lfg.length" class="mb-chip">
          <b>{{ lfg.length }}</b> recherche(s) de joueurs
        </span>
      </template>

      <template #actions>
        <ActionButton to="/jeux" variant="secondary">🎡 Les jeux</ActionButton>
        <ActionButton v-if="discordInvite()" :href="discordInvite()">
          Rejoindre le Discord
        </ActionButton>
      </template>
    </SiteHero>

    <!-- Sans identifiant de serveur, la page n'a rien à interroger. Le dire
         franchement plutôt que d'afficher un héros seul : la page paraîtrait
         cassée sans qu'on sache pourquoi. -->
    <section v-if="!guildId" class="mb-block">
      <p class="mb-config">
        Le serveur à afficher n'est pas configuré. Définis
        <code>PUBLIC_GUILD_ID</code> dans <code>infrastructure/docker/.env</code>,
        puis redémarre le conteneur web.
      </p>
    </section>

    <!-- Base vide et API en panne donnent la même page. Le dire évite de
         chercher le problème du mauvais côté. -->
    <section v-if="toutEnEchec" class="mb-block">
      <p class="mb-config">
        Le site n'arrive pas à joindre l'API. Les sections restent vides tant
        que la connexion n'est pas rétablie — vérifie que les conteneurs
        <code>api</code> et <code>nexus-api</code> tournent.
      </p>
    </section>

    <!-- ── En cours ── -->
    <section v-if="ongoing.length" class="mb-block">
      <h2><span class="mb-live" aria-hidden="true"></span> En ce moment</h2>
      <ul class="mb-events">
        <li
          v-for="e in ongoing"
          :key="e.id"
          class="mb-event ongoing"
          :style="{ '--accent-event': accent(e) }"
        >
          <div class="mb-event-main">
            <strong>{{ e.title }}</strong>
            <span v-if="e.game" class="mb-tag">{{ e.game }}</span>
          </div>
          <p v-if="e.description" class="mb-event-desc">{{ e.description }}</p>
          <span class="mb-event-when">Jusqu'au {{ fmtJour(e.ends_at) }}</span>
        </li>
      </ul>
    </section>

    <!-- ── Serveurs de jeu ── -->
    <section class="mb-block">
      <h2>
        Nos serveurs de jeu
        <span v-if="playersOnline" class="mb-count">{{ playersOnline }} joueur(s) en ligne</span>
      </h2>

      <p v-if="loadingServers" class="mb-hint">Chargement des serveurs…</p>

      <p v-else-if="!servers.length" class="mb-vide">
        Aucun serveur de jeu déclaré. Ils apparaîtront ici avec leur jaquette
        et le nombre de joueurs connectés.
      </p>

      <ul v-else class="mb-games">
        <li
          v-for="sv in sortedServers"
          :key="sv.id"
          class="mb-game"
          :class="{ off: !sv.online }"
        >
          <span v-if="sv.online && sv.player_count" class="mb-badge">
            {{ sv.player_count }} EN JEU
          </span>

          <img v-if="sv.cover_image_url" :src="sv.cover_image_url" :alt="sv.game" />
          <!-- Sans jaquette, l'emoji du template tient lieu de visuel plutôt
               qu'une carte vide. -->
          <div v-else class="mb-game-fallback" aria-hidden="true">{{ sv.icon || "🎮" }}</div>

          <div class="mb-game-in">
            <strong>{{ sv.name }}</strong>
            <span class="mb-game-state">
              <span class="mb-pip" :class="sv.online ? 'on' : 'off'"></span>
              {{ sv.online ? sv.game : "Hors ligne" }}
            </span>
            <span v-if="sv.online && sv.port" class="mb-game-addr">Port {{ sv.port }}</span>
            <span v-else-if="sv.online" class="mb-game-addr muted">Adresse bientôt révélée</span>
          </div>
        </li>
      </ul>
    </section>

    <!-- ── Cherche des joueurs ──
         Placée haut : c'est la section qui fait revenir les gens chaque jour. -->
    <section class="mb-block">
      <h2>
        Cherche des joueurs
        <span v-if="lfg.length" class="mb-count">{{ lfg.length }} annonce(s) ouverte(s)</span>
      </h2>

      <p v-if="loadingLfg" class="mb-hint">Chargement des annonces…</p>

      <p v-else-if="!lfg.length" class="mb-hint">
        Personne ne cherche de monde pour l'instant. Lance la première annonce&nbsp;!
      </p>

      <div v-else class="mb-lfgs">
        <article v-for="a in lfg" :key="a.id" class="mb-lfg">
          <div class="mb-lfg-top">
            <span class="mb-av" :style="{ '--c': couleurDe(a.author_name) }">
              {{ initiale(a.author_name || "?") }}
            </span>
            <span class="mb-lfg-auteur">{{ a.author_name || "Un membre" }}</span>
            <span class="mb-tag">{{ a.game }}</span>
            <span class="mb-lfg-quand">{{ depuis(a.created_at) }}</span>
          </div>

          <p v-if="a.description" class="mb-lfg-texte">{{ a.description }}</p>

          <div class="mb-lfg-foot">
            <span class="mb-lfg-besoin">
              Cherche <b>{{ a.slots }}</b> joueur(s) · {{ a.when_text }}
            </span>

            <span class="mb-lfg-avs">
              <span
                v-for="(nom, i) in a.interested_names.slice(0, 5)"
                :key="i"
                class="mb-av sm"
                :style="{ '--c': couleurDe(nom) }"
                :title="nom"
              >{{ initiale(nom) }}</span>
              <span v-if="a.interested_names.length" class="mb-lfg-n">
                {{ a.interested_names.length }} intéressé(s)
              </span>
              <span v-else class="mb-lfg-n muted">personne encore</span>
            </span>

            <button
              v-if="user"
              type="button"
              class="mb-lfg-btn"
              :disabled="busyLfg === a.id"
              @click="joinLfg(a.id)"
            >
              {{ busyLfg === a.id ? "…" : "Je viens" }}
            </button>
            <ActionButton v-else to="/login?espace=membre" variant="secondary">
              Se connecter pour répondre
            </ActionButton>
          </div>
        </article>
      </div>

      <p v-if="lfgError" class="mb-erreur">{{ lfgError }}</p>
    </section>

    <!-- ── Le planning ── -->
    <section class="mb-block">
      <h2>
        Le planning
        <span class="mb-count">semaine du {{ label }}</span>
        <span class="mb-nav">
          <button type="button" @click="weekStart = addWeeks(weekStart, -1)" aria-label="Semaine précédente">‹</button>
          <button type="button" @click="weekStart = semaineCourante()">Aujourd'hui</button>
          <button type="button" @click="weekStart = addWeeks(weekStart, 1)" aria-label="Semaine suivante">›</button>
        </span>
      </h2>

      <p v-if="loadingEvents" class="mb-hint">Chargement du planning…</p>

      <div v-else class="mb-cal">
        <div class="mb-cal-head">
          <div v-for="d in days" :key="d.toISOString()" :class="{ today: isToday(d) }">
            {{ d.toLocaleDateString("fr-FR", { weekday: "short" }) }}
            <b>{{ d.getDate() }}</b>
          </div>
        </div>

        <div class="mb-cal-body" :style="{ '--rows': weekRows }">
          <div
            v-for="b in weekBars"
            :key="b.event.id"
            class="mb-bar"
            :class="{ clipped: b.clippedStart || b.clippedEnd }"
            :style="{
              '--row': b.row,
              '--from': b.from,
              '--span': b.span,
              '--ev': accent(b.event) || 'var(--accent)',
            }"
            :title="b.event.title"
          >
            <strong>{{ b.event.title }}</strong>
            <span v-if="b.event.span_days > 1">
              {{ b.event.game || "campagne" }}
            </span>
            <span v-else>{{ fmtHeure(b.event.starts_at) }}</span>
          </div>

          <p v-if="!weekBars.length" class="mb-cal-vide">Rien de prévu cette semaine.</p>
        </div>
      </div>
    </section>

    <!-- ── Le prochain rendez-vous ── -->
    <section class="mb-block">
      <h2><span class="mb-live" aria-hidden="true"></span> Le prochain rendez-vous</h2>

      <p v-if="!nextEvent" class="mb-vide">
        Rien de programmé pour l'instant. Les soirées et les campagnes de jeu
        s'annoncent ici.
      </p>

      <div v-else class="mb-feature" :style="{ '--accent-event': accent(nextEvent) }">
        <div class="mb-feature-body">
          <div class="mb-tags">
            <span v-if="nextEvent.game" class="mb-tag">{{ nextEvent.game }}</span>
            <span class="mb-tag neutral">{{ fmtRange(nextEvent) }}</span>
          </div>
          <h3>{{ nextEvent.title }}</h3>
          <p v-if="nextEvent.description">{{ nextEvent.description }}</p>

          <ActionButton v-if="!user" to="/login?espace=membre">
            Se connecter pour s'inscrire
        </ActionButton>
          <span v-else class="mb-soon">Inscription bientôt</span>
        </div>
      </div>

      <ul v-if="upcoming.length" class="mb-events secondaires">
        <li
          v-for="e in upcoming"
          :key="e.id"
          class="mb-event"
          :style="{ '--accent-event': accent(e) }"
        >
          <div class="mb-event-main">
            <strong>{{ e.title }}</strong>
            <span v-if="e.game" class="mb-tag">{{ e.game }}</span>
            <span v-if="e.span_days > 1" class="mb-tag neutral">{{ e.span_days }} jours</span>
          </div>
          <span class="mb-event-when">{{ fmtRange(e) }}</span>
        </li>
      </ul>
    </section>

    <!-- ── En vocal maintenant ──
         Ne s'affiche que si quelqu'un y est vraiment : un cadre « personne en
         vocal » occuperait un écran entier pour dire qu'il ne se passe rien.
         L'API ne publie que les salons visibles par @everyone. -->
    <section class="mb-block">
      <h2>
        <span class="mb-live" aria-hidden="true"></span> En vocal maintenant
        <span v-if="presence.voice_total" class="mb-count">
          {{ presence.voice_total }} personne(s)
        </span>
      </h2>

      <p v-if="!presence.voice.length" class="mb-vide">
        Personne en vocal pour le moment.
      </p>

      <div v-else class="mb-vocaux">
        <article v-for="c in presence.voice" :key="c.channel_name" class="mb-vc">
          <header class="mb-vc-head">
            <span aria-hidden="true">{{ c.restricted ? "🔒" : "🔊" }}</span>
            <span class="mb-vc-nom">{{ c.channel_name }}</span>
            <span
              v-if="c.restricted"
              class="mb-vc-prive"
              title="Salon réservé : visible parce que tu es connecté"
            >
              privé
            </span>
            <span class="mb-vc-n">{{ c.members.length }}</span>
          </header>

          <ul class="mb-vc-list">
            <li v-for="m in c.members" :key="m.username" class="mb-vm">
              <span class="mb-av sm" :style="{ '--c': couleurDe(m.username) }">
                {{ initiale(m.username) }}
              </span>
              <span class="mb-vm-nom">{{ m.username }}</span>
              <span v-if="m.streaming" class="mb-vm-ico" title="Partage son écran">🖥️</span>
              <span v-else-if="m.video" class="mb-vm-ico" title="Caméra activée">📹</span>
              <span v-if="m.muted" class="mb-vm-ico" title="Micro coupé">🔇</span>
            </li>
          </ul>
        </article>
      </div>
    </section>

    <!-- ── Ça discute à l'écrit ── -->
    <section class="mb-block">
      <h2>Ça discute aussi à l'écrit</h2>

      <p v-if="!presence.text.length" class="mb-vide">
        Aucun salon actif dans le dernier quart d'heure.
      </p>

      <ul v-else class="mb-textes">
        <li v-for="t in presence.text" :key="t.channel_name" class="mb-tc">
          <span class="mb-tc-hash" aria-hidden="true">#</span>
          <span class="mb-tc-nom">{{ t.channel_name }}</span>
          <span class="mb-tc-avs">
            <span
              v-for="a in t.recent_authors.slice(0, 6)"
              :key="a"
              class="mb-av sm"
              :style="{ '--c': couleurDe(a) }"
              :title="a"
            >{{ initiale(a) }}</span>
          </span>
          <span class="mb-tc-when">{{ depuis(t.last_message_at) }}</span>
        </li>
      </ul>
    </section>

    <!-- ── On vote ── -->
    <section class="mb-block">
      <h2>On vote</h2>

      <article v-for="p in polls" :key="p.id" class="mb-poll">
        <h3>{{ p.question }}</h3>
        <p v-if="p.description" class="mb-poll-desc">{{ p.description }}</p>

        <ul class="mb-poll-list">
          <li v-for="o in p.options" :key="o.id" class="mb-poll-opt">
            <button
              type="button"
              class="mb-poll-line"
              :class="{ mine: p.my_vote === o.id, votable: !!user }"
              :disabled="!user || busyVote === p.id"
              @click="vote(p.id, o.id)"
            >
              <span>{{ o.label }}</span>
              <span class="mb-poll-pct">{{ o.share }} %</span>
            </button>
            <div class="mb-poll-bar">
              <i :style="{ width: `${o.share}%`, background: `#${o.color}` }"></i>
            </div>
            <span class="mb-poll-n">{{ o.votes }} voix</span>
          </li>
        </ul>

        <p class="mb-poll-foot">
          {{ p.total_votes }} vote(s) · se termine le {{ fmtJour(p.closes_at) }}
        </p>
        <ActionButton v-if="!user" to="/login?espace=membre">
          Se connecter pour voter
        </ActionButton>
      </article>

      <p v-if="!polls.length" class="mb-vide">
        Aucun vote en cours. Le staff en ouvre pour choisir les prochains jeux
        ou les horaires des soirées.
      </p>
    </section>

    <!-- ── Membre du mois et anniversaires ── -->
    <section class="mb-block mb-duo">
      <article class="mb-panel">
        <h3>Membre du mois</h3>
        <p v-if="!spotlight" class="mb-vide">
          Personne n'est encore mis en avant. Le staff distingue chaque mois
          quelqu'un qui a fait vivre le serveur.
        </p>
        <div v-else class="mb-mom">
          <img
            v-if="avatarUrlDe(spotlight.avatar)"
            :src="avatarUrlDe(spotlight.avatar)!"
            alt=""
            class="mb-mom-av"
          />
          <span v-else class="mb-av lg" :style="{ '--c': couleurDe(spotlight.username) }">
            {{ initiale(spotlight.username) }}
          </span>
          <div>
            <div class="mb-mom-nom">{{ spotlight.username }}</div>
            <!-- La raison est ce qui donne son sens à la distinction : sans
                 elle, la section n'afficherait qu'un nom. -->
            <div class="mb-mom-quoi">{{ spotlight.reason }}</div>
          </div>
        </div>
      </article>

      <article class="mb-panel">
        <h3>Anniversaires à venir</h3>
        <p v-if="!anniversaries.length" class="mb-vide">
          Aucun anniversaire d'arrivée dans les deux prochaines semaines.
        </p>
        <ul v-else class="mb-annivs">
          <li v-for="a in anniversaries" :key="a.username + a.joined_at" class="mb-anniv">
            <span class="mb-av" :style="{ '--c': couleurDe(a.username) }">
              {{ initiale(a.username) }}
            </span>
            <span class="mb-anniv-nom">{{ a.username }}</span>
            <span class="mb-anniv-age">{{ anneesLabel(a.years) }}</span>
            <span class="mb-anniv-date">le {{ fmtJour(a.joined_at) }}</span>
          </li>
        </ul>
      </article>
    </section>

    <!-- ── Nouveaux venus ── -->
    <section class="mb-block">
      <h2>
        Ils nous ont rejoints cette semaine
        <span class="mb-count">{{ newcomers.length }} nouveau(x)</span>
      </h2>
      <p v-if="!newcomers.length" class="mb-vide">
        Personne de nouveau cette semaine.
      </p>

      <div v-else class="mb-nouveaux">
        <span v-for="n in newcomers" :key="n.username" class="mb-nv">
          <span class="mb-av" :style="{ '--c': couleurDe(n.username) }">
            {{ initiale(n.username) }}
          </span>
          <span>{{ n.username }}</span>
        </span>
      </div>
    </section>

    <!-- ── Annonces ── -->
    <section class="mb-block">
      <h2>Les dernières annonces</h2>
      <p v-if="!news.length" class="mb-vide">
        Rien à annoncer pour le moment. Les nouvelles du serveur s'afficheront ici.
      </p>

      <div v-else class="mb-anns">
        <article v-for="n in news" :key="n.id" class="mb-ann" :class="{ pinned: n.is_pinned }">
          <img v-if="n.image_url" :src="n.image_url" alt="" class="mb-ann-img" />
          <div>
            <h3>{{ n.title }}</h3>
            <p>{{ n.excerpt }}</p>
            <span class="mb-ann-when">{{ depuis(n.published_at) }}</span>
          </div>
        </article>
      </div>
    </section>

    <footer class="mb-footer">
      <RouterLink v-if="hasAdminAccess" to="/dashboard" class="mb-admin-link">
        🛡️ Accéder à l'administration
      </RouterLink>
    </footer>
  </div>
</template>

<style scoped>
.mb {
  flex: 1;
  position: relative;
  overflow-x: hidden;
  overflow-y: auto;
  padding: clamp(1rem, 3vh, 2rem) clamp(1rem, 4vw, 3rem) 3rem;
  display: flex;
  flex-direction: column;
  gap: clamp(1.75rem, 4vh, 2.75rem);
}

.mb-hero,
.mb-block,
.mb-footer {
  position: relative;
  z-index: 1;
  width: 100%;
  max-width: 68rem;
  margin: 0 auto;
}

/* La barre de session (avatar, deconnexion, « Se connecter ») vivait ici en
   `position: sticky`. Elle est passee dans `SiteHeader`, partagee par les
   trois pages publiques.

   Effet de bord bienvenu : `.mb-bar` designait DEUX choses dans ce fichier —
   cette barre et les barres d'evenement du calendrier plus bas — avec deux
   regles homonymes dans la meme feuille scoped, qui se contaminaient
   mutuellement (grid-column applique a l'en-tete, sticky applique aux
   evenements). Le nom ne designe plus qu'une seule chose. */



/* ── Accueil ── */




.mb-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.35rem 0.9rem;
  border-radius: var(--radius-pill);
  background: var(--bg-card);
  border: 1px solid var(--border);
  font-size: 0.85rem;
  color: var(--text-secondary);
}

.mb-chip b {
  color: #fff;
  font-variant-numeric: tabular-nums;
}

.mb-chip.link:hover {
  border-color: var(--accent);
  color: #fff;
}

/* ── Communs ── */
.mb-block h2 {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.5rem;
  margin: 0 0 0.9rem;
  font-size: 1.15rem;
}

.mb-count {
  font-size: 0.8rem;
  font-weight: 400;
  color: var(--site-ink-4);
  font-variant-numeric: tabular-nums;
}

.mb-nav {
  margin-left: auto;
  display: flex;
  gap: 0.3rem;
}

.mb-nav button {
  background: none;
  border: 1px solid var(--border);
  color: var(--site-ink-3);
  border-radius: var(--radius-pill);
  padding: 0.15rem 0.7rem;
  font: inherit;
  font-size: 0.8rem;
  cursor: pointer;
}

.mb-nav button:hover {
  border-color: var(--accent);
  color: #fff;
}

.mb-live {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  background: var(--site-live);
  box-shadow: 0 0 10px var(--site-live);
  animation: pulse 2.2s ease-in-out infinite;
}

@keyframes pulse {
  50% {
    opacity: 0.35;
  }
}

.mb-pip {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex: none;
  display: inline-block;
}

.mb-pip.on {
  background: var(--site-live);
  box-shadow: 0 0 8px var(--site-live);
}

.mb-pip.off {
  background: var(--site-off);
}

.mb-hint {
  color: var(--site-ink-4);
  margin: 0;
}

.mb-erreur {
  margin: 0.6rem 0 0;
  color: #fca5a5;
  font-size: 0.86rem;
}

.mb-tag {
  font-size: 0.74rem;
  padding: 1px 9px;
  border-radius: var(--radius-pill);
  background: rgba(168, 85, 247, 0.16);
  color: var(--text-secondary);
}

.mb-tag.neutral {
  background: rgba(255, 255, 255, 0.08);
}

.mb-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}

.mb-av {
  width: 26px;
  height: 26px;
  border-radius: 50%;
  flex: none;
  display: grid;
  place-items: center;
  background: var(--c);
  color: var(--bg-primary);
  font-size: 0.75rem;
  font-weight: 700;
}

.mb-av.sm {
  width: 20px;
  height: 20px;
  font-size: 0.66rem;
}

.mb-av.lg {
  width: 46px;
  height: 46px;
  font-size: 1.1rem;
}


.mb-soon {
  align-self: flex-start;
  font-size: 0.74rem;
  color: var(--site-ink-4);
  border: 1px solid var(--border);
  border-radius: var(--radius-pill);
  padding: 2px 10px;
}

/* ── Serveurs de jeu ── */
.mb-games {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(16.5rem, 1fr));
  gap: 1.1rem;
  list-style: none;
  margin: 0;
  padding: 0;
}

.mb-game {
  position: relative;
  border-radius: var(--radius-xl);
  overflow: hidden;
  border: 1px solid var(--border);
  background: var(--bg-card);
}

.mb-game img,
.mb-game-fallback {
  display: block;
  width: 100%;
  aspect-ratio: 1;
  object-fit: cover;
}

.mb-game-fallback {
  display: grid;
  place-items: center;
  font-size: 4rem;
  background: rgba(168, 85, 247, 0.08);
}

.mb-game::after {
  content: "";
  position: absolute;
  inset: 0;
  background: linear-gradient(180deg, transparent 42%, rgba(10, 4, 20, 0.94) 92%);
}

.mb-game-in {
  position: absolute;
  inset: auto 0 0 0;
  z-index: 1;
  padding: 0.9rem 1rem;
}

.mb-game-in strong {
  display: block;
  font-size: 1.08rem;
}

.mb-game-state {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 0.85rem;
  color: var(--site-ink-3);
}

/* Un serveur éteint reste visible mais s'efface : il informe sans attirer. */
.mb-game.off img,
.mb-game.off .mb-game-fallback {
  filter: grayscale(0.85) brightness(0.55);
}

.mb-game-addr {
  display: block;
  margin-top: 0.2rem;
  font-family: ui-monospace, "Cascadia Mono", Menlo, monospace;
  font-size: 0.78rem;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.mb-game-addr.muted {
  color: var(--site-ink-4);
  font-style: italic;
}

.mb-badge {
  position: absolute;
  top: 0.7rem;
  right: 0.7rem;
  z-index: 1;
  background: rgba(34, 197, 94, 0.92);
  color: #04220f;
  font-size: 0.76rem;
  font-weight: 700;
  padding: 3px 10px;
  border-radius: var(--radius-pill);
}

/* ── Cherche des joueurs ── */
.mb-lfgs {
  display: flex;
  flex-direction: column;
  gap: 0.8rem;
}

.mb-lfg {
  padding: 0.85rem 1.05rem;
  border-radius: var(--radius-lg);
  background: var(--bg-card);
  border: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.mb-lfg-top {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  flex-wrap: wrap;
}

.mb-lfg-auteur {
  font-weight: 600;
}

.mb-lfg-quand {
  margin-left: auto;
  font-size: 0.76rem;
  color: var(--site-ink-4);
}

.mb-lfg-texte {
  margin: 0;
  font-size: 0.89rem;
  color: var(--site-ink-3);
}

.mb-lfg-foot {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.6rem;
}

.mb-lfg-besoin {
  font-size: 0.83rem;
  color: var(--text-secondary);
}

.mb-lfg-besoin b {
  color: var(--accent);
}

.mb-lfg-avs {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  margin-left: auto;
}

.mb-lfg-avs .mb-av {
  margin-left: -6px;
  border: 2px solid var(--bg-primary);
}

.mb-lfg-n {
  font-size: 0.78rem;
  color: var(--site-ink-3);
}

.mb-lfg-n.muted {
  color: var(--site-ink-4);
  font-style: italic;
}

.mb-lfg-btn {
  background: rgba(168, 85, 247, 0.18);
  border: 1px solid var(--border-strong);
  color: var(--text-primary);
  font: inherit;
  font-size: 0.82rem;
  font-weight: 600;
  border-radius: var(--radius-pill);
  padding: 0.28rem 0.95rem;
  cursor: pointer;
  text-decoration: none;
}

.mb-lfg-btn:hover:not(:disabled) {
  background: rgba(168, 85, 247, 0.3);
}

.mb-lfg-btn:disabled {
  opacity: 0.5;
  cursor: default;
}

/* ── Calendrier ── */
.mb-cal {
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  overflow: hidden;
  background: var(--bg-card);
}

.mb-cal-head {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  border-bottom: 1px solid var(--border);
}

.mb-cal-head div {
  padding: 0.6rem 0.4rem;
  text-align: center;
  font-size: 0.78rem;
  color: var(--site-ink-4);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.mb-cal-head div.today {
  color: var(--accent);
  font-weight: 700;
}

.mb-cal-head b {
  display: block;
  font-size: 1.05rem;
  color: var(--text-primary);
  letter-spacing: 0;
}

.mb-cal-body {
  position: relative;
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  grid-template-rows: repeat(var(--rows, 2), minmax(2.6rem, auto));
  gap: 0.3rem;
  padding: 0.6rem;
  /* Filets verticaux dessinés en dégradé plutôt qu'en éléments : sept bordures
     de plus alourdiraient l'arbre sans rien apporter. */
  background-image: repeating-linear-gradient(
    to right,
    transparent 0,
    transparent calc(100% / 7 - 1px),
    rgba(168, 85, 247, 0.08) calc(100% / 7 - 1px),
    rgba(168, 85, 247, 0.08) calc(100% / 7)
  );
}

.mb-bar {
  grid-row: var(--row);
  grid-column: var(--from) / span var(--span);
  border-radius: var(--radius-sm);
  padding: 0.4rem 0.6rem;
  font-size: 0.8rem;
  background: color-mix(in srgb, var(--ev) 26%, transparent);
  border: 1px solid color-mix(in srgb, var(--ev) 55%, transparent);
  border-left: 3px solid var(--ev);
  overflow: hidden;
}

/* Un événement qui déborde de la semaine perd son arrondi côté tronqué :
   le lecteur voit qu'il continue au-delà. */
.mb-bar.clipped {
  border-radius: var(--radius-sm) 0.15rem 0.15rem 0.55rem;
}

.mb-bar strong {
  display: block;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.mb-bar span {
  color: var(--site-ink-3);
  font-size: 0.74rem;
  font-variant-numeric: tabular-nums;
}

.mb-cal-vide {
  grid-column: 1 / -1;
  grid-row: 1;
  margin: 0;
  align-self: center;
  text-align: center;
  color: var(--site-ink-4);
  font-size: 0.88rem;
}

/* ── Prochain rendez-vous ── */
.mb-feature {
  --accent-event: var(--accent);
  padding: 1.2rem 1.3rem;
  border-radius: var(--radius-lg);
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-left: 3px solid var(--accent-event);
}

.mb-feature-body {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
}

.mb-feature-body h3 {
  margin: 0;
  font-size: 1.15rem;
}

.mb-feature-body p {
  margin: 0;
  font-size: 0.9rem;
  color: var(--site-ink-3);
}

/* ── Événements en liste ── */
.mb-events {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.7rem;
}

.mb-events.secondaires {
  margin-top: 0.8rem;
}

.mb-event {
  --accent-event: var(--accent);
  padding: 0.9rem 1.1rem;
  border-radius: var(--radius-lg);
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-left: 3px solid var(--accent-event);
}

.mb-event.ongoing {
  background: rgba(34, 197, 94, 0.07);
  border-color: rgba(34, 197, 94, 0.25);
}

.mb-event-main {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
}

.mb-event-desc {
  margin: 0.35rem 0 0;
  font-size: 0.88rem;
  color: var(--site-ink-3);
}

.mb-event-when {
  display: inline-block;
  margin-top: 0.35rem;
  font-size: 0.82rem;
  color: var(--site-ink-4);
}

/* État vide : présent mais discret. Il informe de ce qui viendra sans
   occuper la place d'un vrai contenu. */
.mb-vide {
  margin: 0;
  padding: 0.85rem 1.05rem;
  border-radius: var(--radius-lg);
  background: rgba(255, 255, 255, 0.025);
  border: 1px dashed var(--border);
  color: var(--site-ink-4);
  font-size: 0.88rem;
  line-height: 1.5;
}

/* ── Message de configuration ── */
.mb-config {
  margin: 0;
  padding: 0.9rem 1.1rem;
  border-radius: var(--radius-lg);
  background: rgba(245, 158, 11, 0.1);
  border: 1px solid rgba(245, 158, 11, 0.35);
  color: #f8d9a0;
  font-size: 0.9rem;
}

.mb-config code {
  font-family: ui-monospace, "Cascadia Mono", Menlo, monospace;
  font-size: 0.85em;
  padding: 1px 5px;
  border-radius: var(--radius-sm);
  background: rgba(0, 0, 0, 0.3);
}

/* ── Vocal ── */
.mb-vocaux {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(15rem, 1fr));
  gap: 0.9rem;
}

.mb-vc {
  border-radius: var(--radius-lg);
  background: var(--bg-card);
  border: 1px solid var(--border);
  overflow: hidden;
}

.mb-vc-head {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  padding: 0.6rem 0.85rem;
  background: rgba(168, 85, 247, 0.09);
  border-bottom: 1px solid var(--border);
  font-size: 0.9rem;
}

.mb-vc-nom {
  font-weight: 600;
}

/* Discret : le salon réservé n'est pas une alerte, juste une précision. */
.mb-vc-prive {
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--site-ink-3);
  border: 1px solid var(--site-ink-4);
  border-radius: var(--radius-xs);
  padding: 0 0.3em;
}

.mb-vc-n {
  margin-left: auto;
  font-size: 0.78rem;
  color: var(--site-ink-4);
  font-variant-numeric: tabular-nums;
}

.mb-vc-list {
  list-style: none;
  margin: 0;
  padding: 0.5rem;
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
}

.mb-vm {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  padding: 0.3rem 0.45rem;
  border-radius: var(--radius-sm);
  font-size: 0.88rem;
}

.mb-vm-nom {
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mb-vm-ico {
  font-size: 0.78rem;
  opacity: 0.75;
}

/* Le premier pousse les suivants à droite ; sans ça, deux icônes se
   colleraient au pseudo au lieu de s'aligner en bout de ligne. */
.mb-vm-ico:first-of-type {
  margin-left: auto;
}

/* ── Écrit ── */
.mb-textes {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.mb-tc {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.6rem 0.9rem;
  border-radius: var(--radius-md);
  background: var(--bg-card);
  border: 1px solid var(--border);
  font-size: 0.9rem;
}

.mb-tc-hash {
  color: var(--site-ink-4);
  font-weight: 700;
}

.mb-tc-nom {
  font-weight: 600;
}

.mb-tc-avs {
  display: flex;
  margin-left: auto;
}

.mb-tc-avs .mb-av {
  margin-left: -6px;
  border: 2px solid var(--bg-primary);
}

.mb-tc-when {
  font-size: 0.78rem;
  color: var(--site-ink-4);
  white-space: nowrap;
}

/* ── Sondages ── */
.mb-poll {
  padding: 1.1rem 1.2rem;
  border-radius: var(--radius-lg);
  background: var(--bg-card);
  border: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.mb-poll + .mb-poll {
  margin-top: 0.8rem;
}

.mb-poll h3 {
  margin: 0;
  font-size: 1.02rem;
}

.mb-poll-desc {
  margin: 0;
  font-size: 0.87rem;
  color: var(--site-ink-3);
}

.mb-poll-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
}

.mb-poll-line {
  width: 100%;
  display: flex;
  justify-content: space-between;
  gap: 0.6rem;
  background: none;
  border: none;
  padding: 0;
  font: inherit;
  font-size: 0.88rem;
  color: var(--text-primary);
  text-align: left;
  cursor: default;
}

.mb-poll-line.votable {
  cursor: pointer;
}

.mb-poll-line.votable:hover {
  color: #fff;
}

/* Le choix du lecteur se distingue par la graisse, pas par une couleur : la
   couleur est déjà porteuse de sens sur les barres. */
.mb-poll-line.mine {
  font-weight: 700;
}

.mb-poll-pct {
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
}

.mb-poll-bar {
  height: 8px;
  border-radius: var(--radius-pill);
  background: rgba(255, 255, 255, 0.07);
  margin: 0.2rem 0 0.1rem;
  overflow: hidden;
}

.mb-poll-bar i {
  display: block;
  height: 100%;
  border-radius: var(--radius-pill);
  transition: width 0.35s ease;
}

.mb-poll-n {
  font-size: 0.75rem;
  color: var(--site-ink-4);
  font-variant-numeric: tabular-nums;
}

.mb-poll-foot {
  margin: 0;
  font-size: 0.8rem;
  color: var(--site-ink-4);
}

/* ── Deux colonnes ── */
.mb-duo {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(20rem, 1fr));
  gap: 1.1rem;
}

.mb-panel {
  padding: 1.1rem 1.2rem;
  border-radius: var(--radius-lg);
  background: var(--bg-card);
  border: 1px solid var(--border);
}

.mb-panel h3 {
  margin: 0 0 0.7rem;
  font-size: 1.02rem;
}

.mb-mom {
  display: flex;
  align-items: center;
  gap: 0.8rem;
}

.mb-mom-av {
  width: 46px;
  height: 46px;
  border-radius: 50%;
}

.mb-mom-nom {
  font-weight: 700;
  font-size: 1.05rem;
}

.mb-mom-quoi {
  font-size: 0.86rem;
  color: var(--site-ink-3);
}

.mb-annivs {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.mb-anniv {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  font-size: 0.9rem;
}

.mb-anniv-nom {
  font-weight: 600;
}

.mb-anniv-age {
  font-size: 0.76rem;
  padding: 1px 9px;
  border-radius: var(--radius-pill);
  background: rgba(168, 85, 247, 0.18);
  color: var(--text-secondary);
}

.mb-anniv-date {
  margin-left: auto;
  font-size: 0.8rem;
  color: var(--site-ink-4);
}

/* ── Nouveaux venus ── */
.mb-nouveaux {
  display: flex;
  flex-wrap: wrap;
  gap: 0.55rem;
}

.mb-nv {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.25rem 0.7rem 0.25rem 0.25rem;
  border-radius: var(--radius-pill);
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid var(--border);
  font-size: 0.85rem;
  color: var(--text-secondary);
}

/* ── Annonces ── */
.mb-anns {
  display: flex;
  flex-direction: column;
  gap: 0.7rem;
}

.mb-ann {
  display: grid;
  grid-template-columns: 8rem 1fr;
  gap: 0.9rem;
  padding: 0.75rem;
  border-radius: var(--radius-lg);
  background: var(--bg-card);
  border: 1px solid var(--border);
}

.mb-ann.pinned {
  border-color: var(--border-strong);
}

.mb-ann-img {
  width: 100%;
  aspect-ratio: 16 / 9;
  object-fit: cover;
  border-radius: var(--radius-sm);
}

.mb-ann h3 {
  margin: 0 0 0.2rem;
  font-size: 0.98rem;
}

.mb-ann p {
  margin: 0 0 0.3rem;
  font-size: 0.86rem;
  color: var(--site-ink-3);
}

.mb-ann-when {
  font-size: 0.76rem;
  color: var(--site-ink-4);
}

/* ── Pied ── */
.mb-footer {
  margin-top: auto;
  padding-top: 1rem;
  text-align: center;
}

.mb-admin-link {
  color: var(--site-ink-4);
  font-size: 0.9rem;
}

.mb-admin-link:hover {
  color: var(--text-secondary);
}

@media (max-width: 760px) {
  .mb-ann {
    grid-template-columns: 1fr;
  }

  .mb-nav {
    margin-left: 0;
    width: 100%;
  }
}

@media (prefers-reduced-motion: reduce) {
  .mb-live {
    animation: none;
  }

  .mb-poll-bar i {
    transition: none;
  }
}
</style>
