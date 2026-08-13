import { httpGet, httpPost, httpDelete } from "@/api/http";

/// Types de salons créables depuis le constructeur. Miroir de
/// `PlannedChannelKind` (platform-core/src/sentinel/domain/entities/system/channel_plan.rs).
export type PlannedKind = "category" | "text" | "voice" | "announcement" | "stage" | "forum";

/// Salon déjà présent sur le serveur (contexte affiché à gauche du constructeur).
export interface ExistingChannel {
  id: string;
  name: string;
  kind: string;
  position: number;
}

/// Rôle du serveur, lu EN DIRECT auprès de Discord (pas la table synchronisée) :
/// un rôle créé il y a dix secondes doit être proposable.
export interface LiveRole {
  id: string;
  name: string;
  color: number;
  position: number;
  managed: boolean;
}

/// Ce qu'un rôle a le droit de faire dans un salon. Miroir de `AccessMode`
/// (platform-core/src/sentinel/domain/entities/system/channel_access.rs), qui traduit ces
/// intentions en bits de permission Discord.
export type AccessMode = "denied" | "read" | "write" | "moderate";

export interface AccessRule {
  role_id: string;
  mode: AccessMode;
}

/// Un élément du plan à créer. `key` est une clé LOCALE au plan (générée par le
/// front) : elle relie un salon à sa catégorie avant que Discord n'ait attribué
/// le moindre ID.
export interface PlanItem {
  key: string;
  name: string;
  kind: PlannedKind;
  parent_key?: string | null;
  parent_id?: string | null;
  topic?: string | null;
  slowmode?: number;
  user_limit?: number | null;
  nsfw?: boolean;
  /// Raccourci pour « @everyone : refusé ». S'exclut d'une règle @everyone
  /// explicite (l'API refuse les deux ensemble).
  private?: boolean;
  access?: AccessRule[];
}

export interface PlanItemResult {
  key: string;
  name: string;
  kind: string;
  status: "created" | "failed" | "skipped";
  channel_id: string | null;
  error: string | null;
}

export interface ApplyPlanResponse {
  created: number;
  failed: number;
  skipped: number;
  results: PlanItemResult[];
}

export const guildStructureService = {
  /// Arborescence actuelle du serveur (lue en direct auprès de Discord).
  getStructure(guildId: string): Promise<ExistingChannel[]> {
    return httpGet(`/api/guild-structure/${guildId}`);
  },

  /// Rôles du serveur, lus en direct auprès de Discord.
  getRoles(guildId: string): Promise<LiveRole[]> {
    return httpGet(`/api/guild-structure/${guildId}/roles`);
  },

  /// Applique le plan. L'API valide TOUT avant de créer quoi que ce soit, puis
  /// rapporte le sort de chaque élément.
  apply(guildId: string, items: PlanItem[]): Promise<ApplyPlanResponse> {
    return httpPost(`/api/guild-structure/${guildId}/apply`, { items });
  },

  /// Supprime un salon existant (owner requis côté API).
  removeChannel(guildId: string, channelId: string): Promise<unknown> {
    return httpDelete(`/api/guild-structure/${guildId}/channels/${channelId}`);
  },
};
