import { computed, type Ref } from "vue";
import { useBotEnabledStatus } from "@/composables/useBotEnabledStatus";

/// Tile affichee sur la page d accueil.
///
/// `requiredBot` : si defini, la tuile est cachee quand ce bot est
///   desactive pour la guild courante.
/// `requiredAnyBot` : si defini, la tuile est cachee uniquement quand
///   TOUS ces bots sont desactives (visible si au moins un actif).
/// Univers applicatif d'une entree. Deux produits distincts partagent ce
/// dashboard : Sentinel (moderation/communaute) et Nexus (plateforme jeux).
/// La barre laterale n'affiche que l'univers courant.
export type Universe = "sentinel" | "nexus";

export type DashboardSection = {
  key: string;
  path: string;
  label: string;
  icon: string;
  requiredBot?: string;
  requiredAnyBot?: string[];
  /// Absent = "sentinel" (l'immense majorite des entrees existantes).
  universe?: Universe;
};

const ALL_SECTIONS: DashboardSection[] = [
  // ── Plateforme jeux Nexus ──
  // Backend distinct (nexus-api), accessible via la passerelle /nexus-api/.
  // L'acces est garde cote serveur par la passerelle nginx (auth_request ->
  // sentinel-api verifie l'appartenance a SUPERADMIN_USER_IDS).
  {
    key: "nexus.servers",
    path: "/nexus/servers",
    label: "Serveurs de jeu",
    icon: "server",
    universe: "nexus",
  },
  {
    key: "nexus.economy",
    path: "/nexus/economie",
    label: "Economie",
    icon: "trending-up",
    universe: "nexus",
  },
  {
    key: "nexus.roue",
    path: "/nexus/roue",
    label: "Roue du Destin",
    icon: "zap",
    universe: "nexus",
  },
  {
    key: "nexus.coussin",
    path: "/nexus/coussin",
    label: "Coussin Piégé",
    icon: "gavel",
    universe: "nexus",
  },
  {
    key: "nexus.games",
    path: "/nexus/jeux",
    label: "Jeux mentionnables",
    icon: "target",
    universe: "nexus",
    requiredBot: "game-bot",
  },
  {
    key: "nexus.config",
    path: "/nexus/config",
    label: "Configuration",
    icon: "sliders",
    universe: "nexus",
  },
  // Statistiques serveur + modération réunies (onglets). Visible si au moins un
  // des deux bots concernés est actif.
  { key: "general.stats", path: "/stats", label: "Statistiques", icon: "bar-chart-2", requiredAnyBot: ["audit-bot", "moderation-bot"] },

  { key: "moderation.hub", path: "/moderation", label: "Modération", icon: "gavel", requiredBot: "moderation-bot" },
  { key: "moderation.members", path: "/members", label: "Membres", icon: "users" },
  // Règles de scoring : poids + seuils par type de flag. Elles alimentent
  // l'AUTOMOD (spam, insulte, lien, phishing, nsfw, menace…), donc la tuile
  // doit s'afficher dès qu'automod-bot est actif — pas seulement moderation-bot.
  { key: "moderation.rules", path: "/rules", label: "Règles", icon: "shield", requiredAnyBot: ["automod-bot", "moderation-bot"] },
  { key: "moderation.name-history", path: "/name-history", label: "Historique pseudos", icon: "user-x", requiredBot: "audit-bot" },

  { key: "community.welcome", path: "/welcome", label: "Bienvenue", icon: "user-plus", requiredBot: "welcome-bot" },
  { key: "community.announcements", path: "/announcements", label: "Annonces planifiées", icon: "clock" },
  { key: "community.embeds", path: "/embeds", label: "Embed builder", icon: "edit-3" },
  { key: "community.message", path: "/message", label: "Envoyer un message", icon: "send" },
  // Ce qui alimente l'espace membre du site : nouvelles, sondages, membre du
  // mois, modération des annonces de recherche de joueurs.
  { key: "community.life", path: "/vie-communaute", label: "Vie de la communauté", icon: "heart" },
  { key: "community.confessions", path: "/confessions", label: "Confessions", icon: "edit-3" },
  { key: "community.tickets", path: "/tickets", label: "Tickets", icon: "ticket", requiredBot: "ticket-bot" },
  { key: "community.ideas", path: "/ideas", label: "Idées", icon: "lightbulb", requiredBot: "idea-bot" },
  { key: "community.voice-channels", path: "/voice-channels", label: "Vocaux", icon: "mic", requiredBot: "voice-bot" },
  { key: "community.role-panels", path: "/role-panels", label: "Panneaux de rôles", icon: "users", requiredBot: "community-bot" },
  { key: "community.levels", path: "/levels", label: "Niveaux", icon: "trending-up", requiredBot: "progression-bot" },
  { key: "community.sponsorships", path: "/sponsorships", label: "Parrainages", icon: "user-check", requiredBot: "community-bot" },
  { key: "community.temp-roles", path: "/temp-roles", label: "Rôles temporaires", icon: "clock", requiredBot: "community-bot" },

  { key: "security.hub", path: "/security", label: "Menaces & alertes", icon: "zap", requiredBot: "security-bot" },
  { key: "security.automod", path: "/automod", label: "Automod", icon: "shield", requiredBot: "automod-bot" },


  // Observabilité : journaux métier + système + audit réunis (onglets).
  { key: "logs.system", path: "/system-logs", label: "Logs techniques", icon: "cpu" },


  { key: "config.components", path: "/component-config", label: "Composants", icon: "cpu" },
  { key: "config.system-ops", path: "/system/operations", label: "Opérations système", icon: "activity" },
  { key: "config.server-health", path: "/server-health", label: "État serveur", icon: "server" },
  { key: "config.alert-rules", path: "/alert-rules", label: "Règles d'alerte", icon: "zap" },
  { key: "config.server-security", path: "/server-security", label: "Sécurité serveur", icon: "shield" },
  { key: "config.server-builder", path: "/server-builder", label: "Constructeur de salons", icon: "grid" },
  { key: "config.guild-backup", path: "/guild-backup", label: "Sauvegardes serveur", icon: "save" },
  { key: "config.ai-dataset", path: "/ai-dataset", label: "Dataset IA", icon: "cpu" },
  // Module sans page dediee : la tuile ouvre directement sa config (lien
  // profond ?bot= gere par ComponentConfigPage). Masquee si le bot est off.
  { key: "config.nasa-apod", path: "/component-config?bot=nasa-apod-bot", label: "Photo de l'espace", icon: "image", requiredBot: "nasa-apod-bot" },
];

/// Un groupe de tuiles regroupees par domaine (prefixe de `key`).
export type DashboardGroup = {
  prefix: string;
  label: string;
  sections: DashboardSection[];
};

/// Ordre d'affichage des groupes + libelles FR. Le prefixe correspond a
/// la partie de `key` avant le premier point (ex. "community.welcome").
/// Tout prefixe non liste ici est ignore du regroupement (ne devrait pas
/// arriver ; garde-fou en cas d'ajout futur non declare).
const GROUP_ORDER: { prefix: string; label: string }[] = [
  { prefix: "general", label: "Général" },
  { prefix: "moderation", label: "Modération" },
  { prefix: "community", label: "Communauté" },
  { prefix: "security", label: "Sécurité" },
  { prefix: "config", label: "Configuration" },
  { prefix: "logs", label: "Journaux" },
  // ── Univers Nexus ──
  { prefix: "nexus", label: "Plateforme jeux" },
];

/// Filtre les tuiles dashboard selon :
/// - `requiredBot` : visible seulement si le bot est actif (single dep)
/// - `requiredAnyBot` : visible si AU MOINS UN bot de la liste est actif
/// - aucun des deux : toujours visible (autonome)
export function useDashboardSections(universe?: Ref<Universe>) {
  const { isBotEnabled } = useBotEnabledStatus();

  const sections = computed<DashboardSection[]>(() =>
    ALL_SECTIONS.filter((s) => {
      // Univers : une entree sans `universe` appartient a Sentinel.
      const u = s.universe ?? "sentinel";
      if (universe && u !== universe.value) return false;
      if (s.requiredBot && !isBotEnabled(s.requiredBot)) return false;
      if (s.requiredAnyBot && s.requiredAnyBot.length > 0) {
        const anyActive = s.requiredAnyBot.some((b) => isBotEnabled(b));
        if (!anyActive) return false;
      }
      return true;
    }),
  );

  /// Tuiles visibles regroupees par domaine, dans l'ordre de `GROUP_ORDER`.
  /// Les groupes vides (aucune tuile visible) sont omis.
  const groups = computed<DashboardGroup[]>(() =>
    GROUP_ORDER.map((g) => ({
      prefix: g.prefix,
      label: g.label,
      sections: sections.value.filter((s) => s.key.split(".")[0] === g.prefix),
    })).filter((g) => g.sections.length > 0),
  );

  return { sections, groups };
}
