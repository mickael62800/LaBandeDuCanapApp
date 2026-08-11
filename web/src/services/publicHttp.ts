// Accès aux endpoints publics de Sentinel — sans connexion.
//
// Distinct de `api/http.ts` : ce client n'envoie AUCUNE credential et ne
// redirige jamais vers /login. Un visiteur doit pouvoir consulter la vie du
// serveur sans compte, et une erreur 401 sur une section ne doit pas éjecter
// quelqu'un de la page.

/** GET JSON anonyme vers une URL publique complete. */
export async function anonymousJsonGet<T>(url: string): Promise<T> {
  const res = await fetch(url, {
    credentials: "omit",
    headers: { Accept: "application/json" },
  });
  if (!res.ok) throw new Error(`Erreur ${res.status}`);
  return (await res.json()) as T;
}

/** GET sur `/api/public{path}`. */
export function publicGet<T>(path: string): Promise<T> {
  return anonymousJsonGet<T>(`/api/public${path}`);
}

/** Construit une query string en ignorant les paramètres absents. */
export function query(params: Record<string, string | number | undefined>): string {
  const pairs = Object.entries(params)
    .filter(([, v]) => v !== undefined && v !== "")
    .map(([k, v]) => `${k}=${encodeURIComponent(String(v))}`);
  return pairs.length ? `?${pairs.join("&")}` : "";
}
