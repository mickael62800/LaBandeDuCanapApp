// Hauts faits Nexus (cf. DOC/Nexus/haut-faits.md).
//
// Le catalogue est global a l'installation : ce sont les DEFINITIONS des hauts
// faits, pas leurs attributions. C'est ici que l'administrateur choisit l'image
// affichee sur Discord et dans le dashboard.

import { nexusGet, nexusPatch } from "@/api/nexusHttp";

export interface Achievement {
  id: string;
  /** `null` = haut fait transverse (Discord / Nexus), sinon slug du jeu. */
  game: string | null;
  code: string;
  name: string;
  description: string;
  category: string;
  icon_url: string | null;
  criteria: Record<string, unknown>;
  /** "auto" = attribuable par evenement verifie ; "manual" = validation admin. */
  verification: "auto" | "manual";
  hidden: boolean;
  enabled: boolean;
}

/**
 * Champs modifiables. `icon_url: null` EFFACE l'image ; omettre la cle la
 * laisse inchangee — d'ou le type explicite plutot qu'un `Partial` ambigu.
 */
export interface AchievementUpdate {
  icon_url?: string | null;
  name?: string;
  description?: string;
  enabled?: boolean;
  hidden?: boolean;
}

export const nexusAchievementsService = {
  /** GET /api/achievements/definitions?game=palworld */
  list(guildId: string, game?: string): Promise<Achievement[]> {
    const query = game ? `?game=${encodeURIComponent(game)}` : "";
    return nexusGet<Achievement[]>(`/api/achievements/definitions${query}`, guildId);
  },

  /** PATCH /api/achievements/definitions/{id} */
  update(guildId: string, id: string, update: AchievementUpdate): Promise<Achievement> {
    return nexusPatch<Achievement>(
      `/api/achievements/definitions/${encodeURIComponent(id)}`,
      guildId,
      update,
    );
  },
};
