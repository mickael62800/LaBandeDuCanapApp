import { computed, type Ref } from "vue";
import { useBotEnabledStatus } from "@/composables/useBotEnabledStatus";
import { UNIVERSE_ORDER, UNIVERSES, type UniverseKey } from "@/universes";

/// Registre unique de la navigation du back-office : la barre laterale et les
/// tuiles du tableau de bord lisent toutes les deux ce tableau.
///
/// `universe` : OBLIGATOIRE. Il n'y a plus de defaut implicite — une entree
///   sans univers appartenait auparavant a Sentinel, ce qui rendait Sentinel
///   indistinguable du « reste » et interdisait tout 3e univers.
/// `requiredBot` : la tuile est cachee quand ce bot est desactive pour la
///   guild courante.
/// `requiredAnyBot` : cachee seulement quand TOUS ces bots sont desactives
///   (visible des qu'au moins un est actif).

/// Re-export : de nombreux appelants importent `Universe` depuis ce module.
export type Universe = UniverseKey;

export type DashboardSection = {
  key: string;
  path: string;
  label: string;
  icon: string;
  universe: UniverseKey;
  requiredBot?: string;
  requiredAnyBot?: string[];
};

const ALL_SECTIONS: DashboardSection[] = [
  // ── Plateforme jeux Nexus ──
  // Backend distinct (nexus-api), accessible via la passerelle /nexus-api/.
  // L'acces est garde cote serveur par la passerelle nginx (auth_request ->
  // sentinel-api verifie l'appartenance a SUPERADMIN_USER_IDS).
  { key: "nexus.servers", path: "/nexus/servers", label: "Serveurs de jeu", icon: "server", universe: "nexus" },
  { key: "nexus.economy", path: "/nexus/economie", label: "Economie", icon: "trending-up", universe: "nexus" },
  { key: "nexus.grand-salon", path: "/nexus/grand-salon", label: "Le Grand Salon", icon: "users", universe: "nexus" },
  { key: "nexus.roue", path: "/nexus/roue", label: "Roue du Destin", icon: "zap", universe: "nexus" },
  { key: "nexus.coussin", path: "/nexus/coussin", label: "Coussin Piégé", icon: "gavel", universe: "nexus" },
  { key: "nexus.games", path: "/nexus/jeux", label: "Jeux mentionnables", icon: "target", universe: "nexus" },
  { key: "nexus.achievements", path: "/nexus/haut-faits", label: "Hauts faits", icon: "award", universe: "nexus" },
  { key: "nexus.config", path: "/nexus/config", label: "Configuration", icon: "sliders", universe: "nexus" },

  // ── Sentinel : general ──
  // Statistiques serveur + moderation reunies (onglets). Visible si au moins
  // un des deux bots concernes est actif.
  { key: "general.stats", path: "/stats", label: "Statistiques", icon: "bar-chart-2", universe: "sentinel", requiredAnyBot: ["audit-bot", "moderation-bot"] },

  // ── Sentinel : moderation ──
  { key: "moderation.hub", path: "/moderation", label: "Modération", icon: "gavel", universe: "sentinel", requiredBot: "moderation-bot" },
  { key: "moderation.members", path: "/members", label: "Membres", icon: "users", universe: "sentinel" },
  // Regles de scoring : poids + seuils par type de flag. Elles alimentent
  // l'AUTOMOD (spam, insulte, lien, phishing, nsfw, menace…), donc la tuile
  // doit s'afficher des qu'automod-bot est actif — pas seulement moderation-bot.
  { key: "moderation.rules", path: "/rules", label: "Règles", icon: "shield", universe: "sentinel", requiredAnyBot: ["automod-bot", "moderation-bot"] },
  { key: "moderation.name-history", path: "/name-history", label: "Historique pseudos", icon: "user-x", universe: "sentinel", requiredBot: "audit-bot" },

  // ── Sentinel : communaute ──
  { key: "community.welcome", path: "/welcome", label: "Bienvenue", icon: "user-plus", universe: "sentinel", requiredBot: "welcome-bot" },
  { key: "community.announcements", path: "/announcements", label: "Annonces planifiées", icon: "clock", universe: "sentinel" },
  { key: "community.embeds", path: "/embeds", label: "Embed builder", icon: "edit-3", universe: "sentinel" },
  { key: "community.message", path: "/message", label: "Envoyer un message", icon: "send", universe: "sentinel" },
  // Ce qui alimente l'espace membre du site : nouvelles, sondages, membre du
  // mois, moderation des annonces de recherche de joueurs.
  { key: "community.life", path: "/vie-communaute", label: "Vie de la communauté", icon: "heart", universe: "sentinel" },
  { key: "community.confessions", path: "/confessions", label: "Confessions", icon: "edit-3", universe: "sentinel" },
  { key: "community.tickets", path: "/tickets", label: "Tickets", icon: "ticket", universe: "sentinel", requiredBot: "ticket-bot" },
  { key: "community.ideas", path: "/ideas", label: "Idées", icon: "lightbulb", universe: "sentinel", requiredBot: "idea-bot" },
  { key: "community.voice-channels", path: "/voice-channels", label: "Vocaux", icon: "mic", universe: "sentinel", requiredBot: "voice-bot" },
  { key: "community.role-panels", path: "/role-panels", label: "Panneaux de rôles", icon: "users", universe: "sentinel", requiredBot: "community-bot" },
  { key: "community.levels", path: "/levels", label: "Niveaux", icon: "trending-up", universe: "sentinel", requiredBot: "progression-bot" },
  { key: "community.sponsorships", path: "/sponsorships", label: "Parrainages", icon: "user-check", universe: "sentinel", requiredBot: "community-bot" },
  { key: "community.temp-roles", path: "/temp-roles", label: "Rôles temporaires", icon: "clock", universe: "sentinel", requiredBot: "community-bot" },

  // ── Sentinel : securite ──
  { key: "security.hub", path: "/security", label: "Menaces & alertes", icon: "zap", universe: "sentinel", requiredBot: "security-bot" },
  { key: "security.automod", path: "/automod", label: "Automod", icon: "shield", universe: "sentinel", requiredBot: "automod-bot" },

  // ── Sentinel : configuration ──
  { key: "config.components", path: "/component-config", label: "Composants", icon: "cpu", universe: "sentinel" },
  { key: "config.server-builder", path: "/server-builder", label: "Constructeur de salons", icon: "grid", universe: "sentinel" },
  { key: "config.guild-backup", path: "/guild-backup", label: "Sauvegardes du serveur Discord", icon: "save", universe: "sentinel" },
  { key: "config.ai-dataset", path: "/ai-dataset", label: "Dataset IA", icon: "cpu", universe: "sentinel" },
  // Module sans page dediee : la tuile ouvre directement sa config (lien
  // profond ?bot= gere par ComponentConfigPage). Masquee si le bot est off.
  { key: "config.nasa-apod", path: "/component-config?bot=nasa-apod-bot", label: "Photo de l'espace", icon: "image", universe: "sentinel", requiredBot: "nasa-apod-bot" },

  // ── Exploitation : la machine hote ──
  // Ces ecrans ne parlent pas de Discord : Docker, disques, CPU, certificats
  // TLS, IP bannies, logs des services. Ils concernent autant Nexus et Atrium
  // que Sentinel, d'ou leur univers propre. Les libelles disent « machine » ou
  // « hote » — jamais « serveur » seul, qui designerait aussi bien la guilde
  // Discord qu'un serveur de jeu Nexus.
  { key: "ops.health", path: "/server-health", label: "État de la machine", icon: "server", universe: "ops" },
  { key: "ops.services", path: "/system/operations", label: "Opérations système", icon: "activity", universe: "ops" },
  { key: "ops.security", path: "/server-security", label: "Sécurité de l'hôte", icon: "shield", universe: "ops" },
  { key: "ops.alert-rules", path: "/alert-rules", label: "Règles d'alerte", icon: "zap", universe: "ops" },
  { key: "ops.logs", path: "/system-logs", label: "Logs techniques", icon: "cpu", universe: "ops" },

  // ── Accueil IA Atrium ──
  // Backend distinct (atrium-api) derriere la passerelle /atrium-api/.
  { key: "atrium.home", path: "/atrium", label: "Accueil IA", icon: "cpu", universe: "atrium" },
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
  { prefix: "nexus", label: "Plateforme jeux" },
  { prefix: "atrium", label: "Accueil IA" },
  { prefix: "ops", label: "Exploitation" },
];

/// Filtre les tuiles selon l'univers courant puis l'etat des bots :
/// - `requiredBot` : visible seulement si le bot est actif
/// - `requiredAnyBot` : visible si AU MOINS UN bot de la liste est actif
/// - aucun des deux : toujours visible (autonome)
export function useDashboardSections(universe?: Ref<UniverseKey>) {
  const { isBotEnabled } = useBotEnabledStatus();

  function botsAllow(s: DashboardSection): boolean {
    if (s.requiredBot && !isBotEnabled(s.requiredBot)) return false;
    if (s.requiredAnyBot && s.requiredAnyBot.length > 0) {
      return s.requiredAnyBot.some((b) => isBotEnabled(b));
    }
    return true;
  }

  const sections = computed<DashboardSection[]>(() =>
    ALL_SECTIONS.filter(
      (s) => (!universe || s.universe === universe.value) && botsAllow(s),
    ),
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

  /// Univers reellement navigables : ceux qui ont au moins une section
  /// visible. Evite d'offrir dans la bascule un univers qui n'amenerait que
  /// sur une barre laterale vide.
  const availableUniverses = computed(() =>
    UNIVERSE_ORDER.filter((k) =>
      ALL_SECTIONS.some((s) => s.universe === k && botsAllow(s)),
    ).map((k) => UNIVERSES[k]),
  );

  return { sections, groups, availableUniverses };
}
