// Back-office de la vie communautaire.
//
// Distinct de `communityLifeService`, qui sert la page publique sans
// credential. Ici tout passe par `api/http.ts` : ces routes exigent une
// session et `Moderator+` pour écrire.
//
// Les vues d'administration portent des champs que les DTO publics
// n'exposent pas — auteur, état de publication, sondages clos — parce que
// modérer suppose de voir ce qu'on ne publie pas.

import { httpDelete, httpGet, httpPost, httpPut } from "@/api/http";

// ── Recherche de joueurs ──

export interface AdminLfgPost {
  id: string;
  author_id: string;
  author_name: string;
  game: string;
  game_server_id: string | null;
  slots: number;
  when_text: string;
  description: string | null;
  is_open: boolean;
  expires_at: string;
  created_at: string;
  interested: Array<{ user_id: string; username: string }>;
  remaining_slots: number;
  is_full: boolean;
}

// ── Sondages ──

export interface AdminPollOption {
  id: string;
  label: string;
  color: string;
  votes: number;
  share: number;
}

export interface AdminPoll {
  id: string;
  question: string;
  description: string | null;
  closes_at: string;
  is_closed: boolean;
  is_open: boolean;
  total_votes: number;
  options: AdminPollOption[];
  my_vote: string | null;
}

export interface CreatePollInput {
  question: string;
  description?: string | null;
  /// RFC3339.
  closes_at: string;
  is_public: boolean;
  options: Array<{ label: string; color?: string | null }>;
}

// ── Membre du mois ──

export interface AdminSpotlight {
  id: string;
  user_id: string;
  username: string;
  avatar: string | null;
  period: string;
  reason: string;
}

export interface DesignateInput {
  user_id: string;
  /// Repli seulement : le serveur résout le pseudo depuis `guild_members`.
  username?: string;
  /// `AAAA-MM`. Absent = mois courant.
  period?: string;
  reason: string;
}

// ── Nouvelles ──

export interface AdminNewsItem {
  id: string;
  title: string;
  body: string;
  image_url: string | null;
  is_pinned: boolean;
  is_public: boolean;
  published_at: string;
  created_by: string;
}

export interface UpsertNewsInput {
  title: string;
  body: string;
  image_url?: string | null;
  is_pinned: boolean;
  is_public: boolean;
  /// RFC3339. Absent à la création = maintenant ; absent en modification =
  /// la date existante est conservée.
  published_at?: string | null;
}

/// `?all=1` demande à voir aussi ce qui est clos, expiré ou en brouillon.
function scope(all: boolean): string {
  return all ? "?all=1" : "";
}

export const communityAdminService = {
  // ── Recherche de joueurs ──

  listLfg(guildId: string, all = true): Promise<AdminLfgPost[]> {
    return httpGet<AdminLfgPost[]>(`/api/lfg/${encodeURIComponent(guildId)}${scope(all)}`);
  },

  /// Fermeture, pas suppression : l'annonce reste consultable un temps par
  /// ceux qui s'y étaient inscrits.
  closeLfg(id: string): Promise<{ ok: boolean }> {
    return httpPost<{ ok: boolean }>(`/api/lfg/detail/${encodeURIComponent(id)}/close`, {});
  },

  deleteLfg(id: string): Promise<{ deleted: boolean }> {
    return httpDelete<{ deleted: boolean }>(`/api/lfg/detail/${encodeURIComponent(id)}`);
  },

  // ── Sondages ──

  listPolls(guildId: string, all = true): Promise<AdminPoll[]> {
    return httpGet<AdminPoll[]>(`/api/polls/${encodeURIComponent(guildId)}${scope(all)}`);
  },

  createPoll(guildId: string, input: CreatePollInput): Promise<AdminPoll> {
    return httpPost<AdminPoll>(`/api/polls/${encodeURIComponent(guildId)}`, input);
  },

  closePoll(id: string): Promise<{ ok: boolean }> {
    return httpPost<{ ok: boolean }>(`/api/polls/detail/${encodeURIComponent(id)}/close`, {});
  },

  deletePoll(id: string): Promise<{ deleted: boolean }> {
    return httpDelete<{ deleted: boolean }>(`/api/polls/detail/${encodeURIComponent(id)}`);
  },

  // ── Membre du mois ──

  listSpotlight(guildId: string): Promise<AdminSpotlight[]> {
    return httpGet<AdminSpotlight[]>(`/api/spotlight/${encodeURIComponent(guildId)}`);
  },

  /// Désigner. Une période déjà pourvue est remplacée : un seul membre du
  /// mois par mois.
  designate(guildId: string, input: DesignateInput): Promise<AdminSpotlight> {
    return httpPost<AdminSpotlight>(`/api/spotlight/${encodeURIComponent(guildId)}`, input);
  },

  deleteSpotlight(guildId: string, id: string): Promise<{ deleted: boolean }> {
    return httpDelete<{ deleted: boolean }>(
      `/api/spotlight/${encodeURIComponent(guildId)}/detail/${encodeURIComponent(id)}`,
    );
  },

  // ── Nouvelles ──

  listNews(guildId: string, all = true): Promise<AdminNewsItem[]> {
    return httpGet<AdminNewsItem[]>(`/api/news/${encodeURIComponent(guildId)}${scope(all)}`);
  },

  createNews(guildId: string, input: UpsertNewsInput): Promise<AdminNewsItem> {
    return httpPost<AdminNewsItem>(`/api/news/${encodeURIComponent(guildId)}`, input);
  },

  updateNews(id: string, input: UpsertNewsInput): Promise<AdminNewsItem> {
    return httpPut<AdminNewsItem>(`/api/news/detail/${encodeURIComponent(id)}`, input);
  },

  // ── Evénements ──

  createEvent(guildId: string, input: CreateEventInput): Promise<{ id: string }> {
    return httpPost<{ id: string }>(`/api/events/${encodeURIComponent(guildId)}`, input);
  },

  deleteNews(id: string): Promise<{ deleted: boolean }> {
    return httpDelete<{ deleted: boolean }>(`/api/news/detail/${encodeURIComponent(id)}`);
  },
};

export interface CreateEventInput {
  title: string;
  description?: string | null;
  game?: string | null;
  color?: string | null;
  starts_at: string;
  ends_at: string;
  all_day?: boolean;
  is_public?: boolean;
}
