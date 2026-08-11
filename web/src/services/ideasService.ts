import { httpDelete, httpGet, httpPatch } from "@/api/http";

export type IdeaStatus =
  | "nouvelle"
  | "en_discussion"
  | "acceptee"
  | "refusee"
  | "realisee";

export interface Idea {
  id: string;
  guild_id: string;
  title: string;
  description: string;
  status: IdeaStatus;
  category: string;
  author_id: string;
  author_name: string;
  channel_id: string | null;
  decided_by: string | null;
  decided_by_name: string | null;
  decision_reason: string | null;
  decided_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface IdeaMessage {
  id: string;
  idea_id: string;
  author_name: string;
  author_role: string;
  content: string;
  created_at: string;
}

export interface IdeaDetail {
  idea: Idea;
  messages: IdeaMessage[];
}

export interface ListIdeasParams {
  guild_id?: string;
  status?: IdeaStatus;
  category?: string;
  author_id?: string;
  search?: string;
  limit?: number;
  offset?: number;
}

/** Libellés affichables des statuts (miroir de l'enum côté Rust). */
export const IDEA_STATUS_LABELS: Record<IdeaStatus, string> = {
  nouvelle: "Nouvelle",
  en_discussion: "En discussion",
  acceptee: "Acceptée",
  refusee: "Refusée",
  realisee: "Réalisée",
};

export const IDEA_CATEGORY_LABELS: Record<string, string> = {
  evenement: "Événement",
  salon: "Salon / catégorie",
  role: "Rôle",
  bot: "Bot / fonctionnalité",
  reglement: "Règlement",
  autre: "Autre",
};

function toQuery(params: ListIdeasParams): string {
  const q = new URLSearchParams();
  Object.entries(params).forEach(([key, value]) => {
    if (value !== undefined && value !== null && value !== "") {
      q.set(key, String(value));
    }
  });
  const s = q.toString();
  return s ? `?${s}` : "";
}

export const ideasService = {
  list(params: ListIdeasParams = {}): Promise<Idea[]> {
    // La collection est montee sur `/api/ideas` sans slash final. Axum traite
    // `/api/ideas` et `/api/ideas/` comme deux chemins distincts : placer le
    // slash avant la query faisait donc repondre 404 en production.
    return httpGet(`/api/ideas${toQuery(params)}`);
  },
  get(id: string): Promise<IdeaDetail> {
    return httpGet(`/api/ideas/${id}`);
  },
  /** Décision du staff. Le bot répercute le changement dans Discord. */
  decide(id: string, status: IdeaStatus, reason?: string): Promise<Idea> {
    return httpPatch(`/api/ideas/${id}/status`, { status, reason: reason ?? null });
  },
  remove(id: string): Promise<{ deleted: boolean }> {
    return httpDelete(`/api/ideas/${id}`);
  },
};
