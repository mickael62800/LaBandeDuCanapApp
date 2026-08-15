import { httpGet, httpPost, httpPatch, httpDelete } from "@/api/http";
import type { DiscordRole } from "@/types";

export interface CreateRoleParams {
  name: string;
  color: number;
  permissions?: string | null;
}

export interface EditRoleParams {
  name?: string | null;
  color?: number;
  permissions?: string;
  mentionable?: boolean;
  hoist?: boolean;
}

/**
 * Requêtes en cours, par serveur.
 *
 * Un écran peut afficher des dizaines de sélecteurs de rôle — les paliers de
 * niveaux en alignent une vingtaine. Chacun chargeait la liste à son montage :
 * autant de requêtes identiques, lancées dans la même milliseconde, pour la
 * même réponse. On partage la promesse plutôt que de les multiplier.
 *
 * La promesse est retenue brièvement puis oubliée : assez pour couvrir le
 * montage d'un écran, trop peu pour masquer un rôle cree entre-temps.
 */
const enCours = new Map<string, Promise<DiscordRole[]>>();
const DUREE_PARTAGE_MS = 3000;

type LiveDiscordRole = Pick<DiscordRole, "id" | "name" | "color" | "position" | "managed">;

/**
 * La table `discord_roles` est alimentee par une synchronisation du bot. Elle
 * peut etre vide au demarrage ou apres une interruption de cette synchro.
 * Dans ce cas, lire directement Discord evite de rendre tous les selecteurs
 * de roles inutilisables. Les champs absents de la reponse live ne sont pas
 * necessaires aux selecteurs ; on les complete pour conserver le contrat
 * partage `DiscordRole`.
 */
async function chargerRoles(guildId: string): Promise<DiscordRole[]> {
  const synchronises = await httpGet<DiscordRole[]>(`/api/discord-roles/${guildId}`);
  if (synchronises.length > 0) return synchronises;

  const directs = await httpGet<LiveDiscordRole[]>(`/api/guild-structure/${guildId}/roles`);
  return directs.map((role) => ({
    ...role,
    guild_id: guildId,
    permissions: "0",
    mentionable: false,
    icon: null,
    member_count: 0,
    synced_at: "",
  }));
}

export const discordRolesService = {
  getAll(guildId: string): Promise<DiscordRole[]> {
    const partagee = enCours.get(guildId);
    if (partagee) return partagee;

    const promesse = chargerRoles(guildId);
    enCours.set(guildId, promesse);
    // Un echec ne doit pas rester en cache : le prochain appel doit reessayer.
    promesse.catch(() => enCours.delete(guildId));
    setTimeout(() => enCours.delete(guildId), DUREE_PARTAGE_MS);
    return promesse;
  },

  /// A appeler apres toute modification de role, pour que le prochain
  /// selecteur reparte de la liste reelle.
  invalider(guildId: string) {
    enCours.delete(guildId);
  },
  create(guildId: string, params: CreateRoleParams): Promise<unknown> {
    enCours.delete(guildId);
    return httpPost(`/api/discord-roles/${guildId}/create`, params);
  },
  edit(guildId: string, roleId: string, params: EditRoleParams): Promise<unknown> {
    enCours.delete(guildId);
    return httpPatch(`/api/discord-roles/${guildId}/${roleId}`, params);
  },
  remove(guildId: string, roleId: string): Promise<unknown> {
    enCours.delete(guildId);
    return httpDelete(`/api/discord-roles/${guildId}/${roleId}`);
  },
};
