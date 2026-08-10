// Client HTTP de l'exploitation de la machine hôte.
//
// Quatrième backend, quatrième client — pour la même raison que les trois
// autres : des modèles d'authentification différents. Les fusionner
// obligerait à paramétrer les services existants pour un seul cas.
//
// Chemin : `/ops-api/...` en même origine, proxifié par nginx vers ops-api.
//   - le jeton n'est JAMAIS embarqué ici : nginx l'injecte côté serveur ;
//   - `auth_request` a déjà vérifié la session Discord en amont, donc ops-api
//     n'a aucune notion d'utilisateur.

import { getDiscordToken } from "./config";

const OPS_BASE = "/ops-api";

export class OpsHttpError extends Error {
  constructor(
    message: string,
    public status: number,
  ) {
    super(message);
    this.name = "OpsHttpError";
  }
}

function headers(): Record<string, string> {
  const h: Record<string, string> = { "Content-Type": "application/json" };
  const tok = getDiscordToken();
  if (tok) h["X-Discord-Token"] = tok;
  return h;
}

async function request<T>(
  method: string,
  path: string,
  body?: unknown,
): Promise<T> {
  const res = await fetch(`${OPS_BASE}${path}`, {
    method,
    headers: headers(),
    credentials: "include",
    body: body === undefined ? undefined : JSON.stringify(body),
  });

  if (!res.ok) {
    // 401/403 viennent de la passerelle, pas d'ops-api.
    if (res.status === 401) {
      throw new OpsHttpError("Session expirée — reconnecte-toi.", 401);
    }
    if (res.status === 403) {
      throw new OpsHttpError("Accès à l'exploitation refusé.", 403);
    }
    if (res.status === 502 || res.status === 503) {
      throw new OpsHttpError(
        "L'API d'exploitation ne répond pas. Le service est-il démarré ?",
        res.status,
      );
    }
    const detail = await res
      .json()
      .then((b: { error?: string }) => b?.error)
      .catch(() => null);
    throw new OpsHttpError(detail ?? `Erreur exploitation (${res.status})`, res.status);
  }

  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}

export const opsGet = <T>(path: string) => request<T>("GET", path);

export const opsPatch = <T>(path: string, body?: unknown) =>
  request<T>("PATCH", path, body);
