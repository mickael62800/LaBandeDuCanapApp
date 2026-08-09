// Client HTTP de la plateforme jeux Nexus.
//
// Volontairement separe de `api/http.ts` : ce sont deux backends distincts,
// avec deux modeles d'authentification differents. Les melanger reviendrait a
// parametrer les 40 services existants pour une seule raison.
//
// Chemin : tout part sur `/nexus-api/...` en meme origine, proxifie par nginx
// vers nexus-api. Deux consequences importantes :
//   - la cle d'API Nexus n'est JAMAIS embarquee ici. nginx l'injecte cote
//     serveur. La mettre dans le SPA donnerait un acces admin complet.
//   - chaque requete porte `X-Guild-Id` : nginx la transmet a sentinel-api
//     (auth_request) qui verifie le gate RBAC `nexus.access` pour cette guild.
//     Sans cet en-tete, l'acces est refuse.

import { getDiscordToken } from "./config";

/// Prefixe servi par nginx. Relatif : toujours la meme origine que le SPA.
const NEXUS_BASE = "/nexus-api";

export class NexusHttpError extends Error {
  constructor(
    message: string,
    public status: number,
  ) {
    super(message);
    this.name = "NexusHttpError";
  }
}

function headers(guildId: string | null, extra?: Record<string, string>): Record<string, string> {
  const h: Record<string, string> = { "Content-Type": "application/json", ...extra };
  const tok = getDiscordToken();
  if (tok) h["X-Discord-Token"] = tok;
  if (guildId) h["X-Guild-Id"] = guildId;
  return h;
}

async function request<T>(
  method: string,
  path: string,
  guildId: string | null,
  body?: unknown,
): Promise<T> {
  const res = await fetch(`${NEXUS_BASE}${path}`, {
    method,
    headers: headers(guildId),
    credentials: "include",
    body: body === undefined ? undefined : JSON.stringify(body),
  });

  if (!res.ok) {
    // 401/403 viennent de la passerelle (session expiree ou gate RBAC
    // `nexus.access` refuse), pas de nexus-api lui-meme.
    if (res.status === 401) {
      throw new NexusHttpError("Session expiree — reconnecte-toi.", 401);
    }
    if (res.status === 403) {
      throw new NexusHttpError("Acces a la plateforme jeux refuse.", 403);
    }
    const detail = await res
      .json()
      .then((b: { error?: string }) => b?.error)
      .catch(() => null);
    throw new NexusHttpError(detail ?? `Erreur Nexus (${res.status})`, res.status);
  }

  if (res.status === 204 || res.status === 202) return undefined as T;
  return (await res.json()) as T;
}

export const nexusGet = <T>(path: string, guildId: string | null) =>
  request<T>("GET", path, guildId);

export const nexusPost = <T>(path: string, guildId: string | null, body?: unknown) =>
  request<T>("POST", path, guildId, body);

export const nexusPut = <T>(path: string, guildId: string | null, body?: unknown) =>
  request<T>("PUT", path, guildId, body);

export const nexusPatch = <T>(path: string, guildId: string | null, body?: unknown) =>
  request<T>("PATCH", path, guildId, body);

export const nexusDelete = <T>(path: string, guildId: string | null) =>
  request<T>("DELETE", path, guildId);
