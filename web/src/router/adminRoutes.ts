// Racine ADMINISTRATION : le back-office, organise par univers.
//
// Toute route de ce fichier passe par `inUniverse(...)`, qui pose
// `meta.universe`. C'est la seule source de verite de l'univers courant
// (cf. `universes.ts`) : l'oublier donnerait une barre laterale incoherente
// avec la page affichee.
//
// Ces routes ne declarent pas `meta.layout` : l'absence vaut « back-office »,
// le cas de loin le plus frequent.

import type { RouteRecordRaw } from "vue-router";
import type { UniverseKey } from "@/universes";

// Eager : 1ere page apres connexion, autant l'avoir dans le bundle initial
// pour eviter un flash de loader. Tout le reste est lazy-loaded -> chaque page
// = un chunk separe, charge a la 1ere navigation.
import DashboardPage from "@/components/pages/DashboardPage.vue";

/// Applique l'univers a un lot de routes. Sucre volontaire : il rend l'oubli
/// de `meta.universe` difficile, la ou une route isolee l'oubliait facilement.
function inUniverse(
  universe: UniverseKey,
  records: RouteRecordRaw[],
): RouteRecordRaw[] {
  return records.map((r) => ({ ...r, meta: { ...r.meta, universe } }));
}

export const adminRoutes: RouteRecordRaw[] = [
  // ── Univers Sentinel : moderation et communaute ──
  ...inUniverse("sentinel", [
    // Le nom de route reste "dashboard" : toutes les redirections internes
    // continuent de fonctionner malgre le changement de chemin.
    { path: "/dashboard", name: "dashboard", component: DashboardPage },

    // ── Stats / Audit ──
    // Statistiques : serveur + modération réunies en onglets (StatsHubPage).
    { path: "/stats", name: "stats", component: () => import("@/components/pages/StatsHubPage.vue") },
    { path: "/name-history", name: "name-history", component: () => import("@/components/pages/NameHistoryPage.vue") },

    // ── Modération ──
    { path: "/moderation", name: "moderation", component: () => import("@/components/pages/ModerationHubPage.vue") },
    { path: "/rules", name: "rules", component: () => import("@/components/pages/RulesPage.vue") },
    { path: "/automod", name: "automod", component: () => import("@/components/pages/AutomodPage.vue") },

    // ── Sécurité ──
    { path: "/security", name: "security", component: () => import("@/components/pages/SecurityPage.vue") },

    // ── Communauté ──
    { path: "/welcome", name: "welcome", component: () => import("@/components/pages/WelcomePage.vue") },
    { path: "/announcements", name: "announcements", component: () => import("@/components/pages/AnnouncementsPage.vue") },
    { path: "/embeds", name: "embeds", component: () => import("@/components/pages/EmbedBuilderPage.vue") },
    { path: "/message", name: "send-message", component: () => import("@/components/pages/SendMessagePage.vue") },
    { path: "/vie-communaute", name: "community-life", component: () => import("@/components/pages/CommunityLifePage.vue") },
    { path: "/confessions", name: "confessions", component: () => import("@/components/pages/ConfessionsPage.vue") },
    { path: "/tickets", name: "tickets", component: () => import("@/components/pages/TicketsPage.vue") },
    { path: "/ideas", name: "ideas", component: () => import("@/components/pages/IdeasPage.vue") },
    // Vocaux : salons + thèmes réunis en onglets (VoiceHubPage).
    { path: "/voice-channels", name: "voice-channels", component: () => import("@/components/pages/VoiceHubPage.vue") },
    // Rôles : panneaux + rôles Discord réunis en onglets (RolesHubPage). Les deux
    // chemins pointent le même hub, qui choisit l'onglet selon l'URL (le lien
    // croisé "Voir tous les rôles" reste fonctionnel).
    { path: "/role-panels", name: "role-panels", component: () => import("@/components/pages/RolesHubPage.vue") },
    { path: "/role-panels/new", name: "role-panel-new", component: () => import("@/components/pages/RolePanelEditPage.vue") },
    { path: "/discord-roles", name: "discord-roles", component: () => import("@/components/pages/RolesHubPage.vue") },
    // Niveaux : classement + configuration réunis en onglets (LevelsHubPage).
    { path: "/levels", name: "levels", component: () => import("@/components/pages/LevelsHubPage.vue") },
    { path: "/sponsorships", name: "sponsorships", component: () => import("@/components/pages/SponsorshipsPage.vue") },
    { path: "/temp-roles", name: "temp-roles", component: () => import("@/components/pages/TempRolesPage.vue") },
    { path: "/members", name: "members", component: () => import("@/components/pages/MembersPage.vue") },

    // ── Configuration du serveur Discord ──
    { path: "/component-config", name: "component-config", component: () => import("@/components/pages/ComponentConfigPage.vue") },
    { path: "/server-builder", name: "server-builder", component: () => import("@/components/pages/ServerBuilderPage.vue") },
    { path: "/guild-backup", name: "guild-backup", component: () => import("@/components/pages/GuildBackupPage.vue") },
    { path: "/ai-dataset", name: "ai-dataset", component: () => import("@/components/pages/AiDatasetPage.vue") },
  ]),

  // ── Univers Nexus : plateforme jeux ──
  // Backend distinct (nexus-api) derriere la passerelle /nexus-api/. L'acces
  // est garde cote serveur par le gate `nexus.access`.
  ...inUniverse("nexus", [
    { path: "/nexus/servers", name: "nexus-servers", component: () => import("@/components/pages/NexusServersPage.vue") },
    { path: "/nexus/servers/nouveau", name: "nexus-server-create", component: () => import("@/components/pages/NexusServerCreatePage.vue") },
    { path: "/nexus/servers/:id", name: "nexus-server-detail", component: () => import("@/components/pages/NexusServerDetailPage.vue") },
    { path: "/nexus/ressources", name: "nexus-resources", component: () => import("@/components/pages/NexusResourcesDocPage.vue") },
    { path: "/nexus/economie", name: "nexus-economy", component: () => import("@/components/pages/NexusEconomyPage.vue") },
    { path: "/nexus/grand-salon", name: "nexus-grand-salon", component: () => import("@/components/pages/NexusGrandSalonPage.vue") },
    { path: "/nexus/roue", name: "nexus-roue", component: () => import("@/components/pages/NexusWheelPage.vue") },
    { path: "/nexus/coussin", name: "nexus-coussin", component: () => import("@/components/pages/NexusCoussinPage.vue") },
    { path: "/nexus/haut-faits", name: "nexus-achievements", component: () => import("@/components/pages/NexusAchievementsPage.vue") },
    { path: "/nexus/jeux", name: "nexus-games", component: () => import("@/components/pages/NexusMentionableGamesPage.vue") },
    { path: "/nexus/config", name: "nexus-config", component: () => import("@/components/pages/NexusConfigPage.vue") },
  ]),

  // ── Univers Exploitation : la machine hote ──
  // Docker, disques, TLS, IP bannies, logs des services. Transverse aux trois
  // plateformes : ces ecrans ne parlent pas de Discord.
  ...inUniverse("ops", [
    { path: "/server-health", name: "server-health", component: () => import("@/components/pages/ServerHealthPage.vue") },
    { path: "/system/operations", name: "system-ops", component: () => import("@/components/pages/SystemOpsPage.vue") },
    { path: "/server-security", name: "server-security", component: () => import("@/components/pages/ServerSecurityPage.vue") },
    { path: "/alert-rules", name: "alert-rules", component: () => import("@/components/pages/AlertRulesPage.vue") },
    { path: "/system-logs", name: "system-logs", component: () => import("@/components/pages/SystemLogsPage.vue") },
  ]),

  // ── Univers Atrium : accueil assiste par IA ──
  // Backend distinct (atrium-api) derriere la passerelle /atrium-api/, sur le
  // meme montage que Nexus : `auth_request` vers sentinel-api, puis jeton
  // injecte cote serveur. atrium-api n'est pas publie sur l'hote.
  ...inUniverse("atrium", [
    { path: "/atrium", name: "atrium", component: () => import("@/components/pages/AtriumPage.vue") },
  ]),
];
