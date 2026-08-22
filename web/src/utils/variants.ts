export type BadgeVariant = "danger" | "warning" | "info" | "success" | "default";

export function severityVariant(severity: string): BadgeVariant {
  switch (severity) {
    case "critical":
    case "urgent":
      return "danger";
    case "high":
      return "warning";
    case "medium":
      return "info";
    case "low":
      return "default";
    default:
      return "default";
  }
}

export function actionVariant(action: string): BadgeVariant {
  switch (action) {
    case "ban":
    case "ban_permanent":
    case "ban_temp":
    case "lockdown":
      return "danger";
    case "mute":
    case "mute_permanent":
    case "mute_temp":
    case "delete":
      return "warning";
    case "warn":
      return "info";
    case "none":
      return "default";
    case "unban":
    case "unmute":
      return "success";
    default:
      return "default";
  }
}

export function statusVariant(status: string): BadgeVariant {
  switch (status) {
    case "open":
      return "info";
    case "pending":
      return "warning";
    case "closed":
      return "success";
    default:
      return "default";
  }
}

export function priorityVariant(priority: string): BadgeVariant {
  switch (priority) {
    case "urgent":
      return "danger";
    case "high":
      return "warning";
    case "medium":
      return "info";
    case "low":
      return "default";
    default:
      return "default";
  }
}

export function levelVariant(level: string): BadgeVariant {
  if (level === "info") return "info";
  if (level === "warn") return "warning";
  if (level === "error") return "danger";
  return "default";
}

export function infractionTypeVariant(type: string): BadgeVariant {
  switch (type) {
    case "ban":
      return "danger";
    case "mute":
      return "warning";
    case "warn":
      return "info";
    default:
      return "default";
  }
}

// --- Labels (centralized from RuleCard, AuditPage, etc.) ---

export function actionLabel(action: string): string {
  const labels: Record<string, string> = {
    ban: "Bannissement",
    mute: "Sourdine",
    delete: "Suppression",
    warn: "Avertissement",
    lockdown: "Verrouillage",
    // Le flag ne suffit pas seul a franchir un seuil : il lui faut un autre
    // signal sur le meme message.
    none: "Seul : rien",
  };
  return labels[action] ?? action;
}

export function typeLabel(type: string): string {
  const labels: Record<string, string> = {
    spam: "Spam",
    insult: "Insulte",
    link: "Lien",
    phishing: "Hameconnage",
    nsfw: "NSFW",
    illicit: "Illicite",
    anger: "Colere",
    rage: "Rage",
    threat: "Menace",
    harassment: "Harcelement",
  };
  return labels[type] ?? type;
}

export function eventVariant(type: string): BadgeVariant {
  switch (type) {
    case "member_ban":
    case "channel_delete":
      return "danger";
    case "member_leave":
    case "message_delete":
    case "member_roles_update":
      return "warning";
    case "member_join":
    case "member_unban":
    case "channel_create":
      return "success";
    case "voice_join":
    case "voice_leave":
    case "voice_move":
      return "info";
    default:
      return "default";
  }
}

export function eventLabel(type: string): string {
  const labels: Record<string, string> = {
    // Messages
    message_delete: "Message supprimé",
    message_edit: "Message édité",
    message_pin: "Message épinglé",
    message_unpin: "Message désépinglé",
    // Membres
    member_join: "Arrivée d'un membre",
    member_leave: "Départ d'un membre",
    member_ban: "Membre banni",
    member_unban: "Membre débanni",
    member_kick: "Membre expulsé",
    member_timeout: "Membre mis en sourdine",
    member_untimeout: "Sourdine retirée",
    member_update: "Profil modifié",
    member_nickname_update: "Pseudo modifié",
    member_roles_update: "Rôles modifiés",
    // Vocal
    voice_join: "Entrée en vocal",
    voice_leave: "Sortie de vocal",
    voice_move: "Changement de salon vocal",
    voice_mute: "Micro coupé",
    voice_unmute: "Micro réactivé",
    voice_deafen: "Casque coupé",
    voice_undeafen: "Casque réactivé",
    voice_channel_created: "Salon vocal créé",
    voice_channel_updated: "Salon vocal modifié",
    voice_channel_closed: "Salon vocal fermé",
    // Salons
    channel_create: "Salon créé",
    channel_delete: "Salon supprimé",
    channel_update: "Salon modifié",
    // Rôles
    role_create: "Rôle créé",
    role_delete: "Rôle supprimé",
    role_update: "Rôle modifié",
    // Modération
    moderation_action: "Action de modération",
    warn_added: "Avertissement ajouté",
    strike_added: "Strike ajouté",
    // Sécurité
    raid_detected: "Raid détecté",
    suspicious_account: "Compte suspect",
    mass_ban: "Bannissement en masse",
    alt_account: "Compte alternatif suspect",
    // Tickets
    ticket_create: "Ticket créé",
    ticket_close: "Ticket fermé",
    ticket_assign: "Ticket assigné",
    // Invites
    invite_create: "Invitation créée",
    invite_delete: "Invitation supprimée",
    invite_use: "Invitation utilisée",
  };
  return labels[type] ?? humanize(type);
}

/**
 * Fallback : transforme un event_type inconnu en libellé lisible.
 * `member_ban_xyz` → `Member ban xyz`
 */
function humanize(type: string): string {
  const cleaned = type.replace(/_/g, " ").trim();
  if (!cleaned) return type;
  return cleaned.charAt(0).toUpperCase() + cleaned.slice(1);
}

export function eventIcon(type: string): string {
  const icons: Record<string, string> = {
    message_delete: "X",
    message_edit: "E",
    member_join: "+",
    member_leave: "-",
    member_ban: "B",
    member_unban: "U",
    member_roles_update: "R",
    voice_join: "V",
    voice_leave: "V",
    voice_move: "M",
    channel_create: "#",
    channel_delete: "#",
  };
  return icons[type] ?? "?";
}

/** Couleur du badge de statut d'une idee (voir IdeaStatus cote Rust). */
export function ideaStatusVariant(status: string): BadgeVariant {
  switch (status) {
    case "nouvelle":
      return "info";
    case "en_discussion":
      return "warning";
    case "acceptee":
      return "success";
    case "refusee":
      return "danger";
    case "realisee":
      return "success";
    default:
      return "default";
  }
}
