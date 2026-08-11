// Vitrine publique des serveurs de jeu — accessible SANS connexion.
//
// Passe par `/nexus-public/`, une location nginx distincte de `/nexus-api/` :
// celle-ci n'exige pas de session. La clé d'API Nexus reste injectée côté
// serveur et ne parvient jamais au navigateur.

import { anonymousJsonGet } from "./publicHttp";

export interface PublicGameServer {
  id: string;
  name: string;
  /// Nom lisible du jeu, pas son slug technique.
  game: string;
  icon: string | null;
  /// Jaquette du jeu, chemin relatif servi par le site (`/imgs/...`).
  /// Absente : la page retombe sur l'emoji `icon`.
  cover_image_url: string | null;
  online: boolean;
  player_count: number;
  /// Renseigné uniquement si l'adresse a été révélée.
  port: number | null;
  address_revealed: boolean;
}

export const publicGamesService = {
  /** GET /api/public/games/{guild}/servers */
  listServers(guildId: string): Promise<PublicGameServer[]> {
    return anonymousJsonGet<PublicGameServer[]>(
      `/nexus-public/api/public/games/${encodeURIComponent(guildId)}/servers`,
    );
  },
};
