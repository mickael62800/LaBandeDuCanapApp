// Client HTTP de l'accueil IA Atrium.
//
// Volontairement separe de `api/http.ts` et de `api/nexusHttp.ts` : trois
// backends distincts, trois modeles d'authentification. Les fusionner
// obligerait a parametrer les services existants pour une seule raison.
//
// Chemin : tout part sur `/atrium-api/...` en meme origine, proxifie par nginx
// vers atrium-api. Deux consequences importantes :
//   - le jeton d'API Atrium n'est JAMAIS embarque ici. nginx l'injecte cote
//     serveur. Le mettre dans le SPA donnerait un acces complet a l'API.
//   - atrium-api n'est pas publie sur l'hote : cette passerelle est le SEUL
//     chemin du back-office vers lui.
//
// L'acces est garde en amont par `auth_request` : nginx interroge sentinel-api
// pour verifier que la session Discord est celle d'un superadmin. atrium-api
// n'a aucune notion d'utilisateur.

import { getDiscordToken } from "./config";

/// Prefixe servi par nginx. Relatif : toujours la meme origine que le SPA.
const ATRIUM_BASE = "/atrium-api";

export class AtriumHttpError extends Error {
  constructor(
    message: string,
    public status: number,
  ) {
    super(message);
    this.name = "AtriumHttpError";
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
  const res = await fetch(`${ATRIUM_BASE}${path}`, {
    method,
    headers: headers(),
    credentials: "include",
    body: body === undefined ? undefined : JSON.stringify(body),
  });

  if (!res.ok) {
    // 401/403 viennent de la passerelle (session expiree ou droit refuse),
    // pas d'atrium-api lui-meme.
    if (res.status === 401) {
      throw new AtriumHttpError("Session expirée — reconnecte-toi.", 401);
    }
    if (res.status === 403) {
      throw new AtriumHttpError("Accès à Atrium refusé.", 403);
    }
    // 502/503 : atrium-api tourne derriere le profil Docker `atrium`. Absent,
    // nginx ne joint rien — c'est un cas NORMAL d'installation, pas une panne,
    // et le message doit le dire plutot que d'afficher une erreur brute.
    if (res.status === 502 || res.status === 503) {
      throw new AtriumHttpError(
        "Atrium ne répond pas. Le service est-il démarré (profil Docker « atrium ») ?",
        res.status,
      );
    }
    const detail = await res
      .json()
      .then((b: { error?: string }) => b?.error)
      .catch(() => null);
    throw new AtriumHttpError(detail ?? `Erreur Atrium (${res.status})`, res.status);
  }

  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}

export const atriumGet = <T>(path: string) => request<T>("GET", path);

export const atriumPut = <T>(path: string, body?: unknown) =>
  request<T>("PUT", path, body);
