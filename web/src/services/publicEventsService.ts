// Planning public — accessible SANS connexion.
//
// Même principe que `publicSiteService` : aucune credential envoyée, aucune
// redirection vers /login en cas d'erreur. Un visiteur doit pouvoir consulter
// ce qui se passe sans avoir de compte.

export interface PublicEvent {
  id: string;
  title: string;
  description: string | null;
  game: string | null;
  color: string | null;
  starts_at: string;
  ends_at: string;
  all_day: boolean;
  /// Nombre de jours couverts : sert à distinguer une soirée d'une campagne.
  span_days: number;
  /// Serveur de jeu Nexus à l'origine de cet événement, s'il y en a un.
  source_server_id?: string | null;
}

import { publicGet } from "./publicHttp";

export const publicEventsService = {
  /**
   * GET /api/public/events/{guild}?from=&to=
   *
   * Les bornes sont explicites : l'API renvoie les événements qui CHEVAUCHENT
   * la fenêtre, donc une campagne en cours ressort même si elle a commencé
   * avant `from`.
   */
  list(guildId: string, from: Date, to: Date): Promise<PublicEvent[]> {
    const q = `?from=${encodeURIComponent(from.toISOString())}&to=${encodeURIComponent(
      to.toISOString(),
    )}`;
    return publicGet<PublicEvent[]>(`/events/${encodeURIComponent(guildId)}${q}`);
  },
};

/// Un événement est-il en cours à cet instant ?
export function isOngoing(e: PublicEvent, now = new Date()): boolean {
  return new Date(e.starts_at) <= now && new Date(e.ends_at) >= now;
}
