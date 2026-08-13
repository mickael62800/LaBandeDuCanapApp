// Client du backend Atrium, servi en meme origine par nginx.
import { BackendHttpError, createBackendClient } from "./backendHttp";
import type { HttpErrorDetails } from "./httpError";

const ATRIUM_BASE = "/atrium-api";

export class AtriumHttpError extends BackendHttpError {
  constructor(message: string, details: HttpErrorDetails) {
    super(message, details, "AtriumHttpError");
  }
}

const request = createBackendClient({
  baseUrl: ATRIUM_BASE,
  errorLabel: "Atrium",
  forbiddenMessage: "Accès à Atrium refusé.",
  unavailableMessage:
    "Atrium ne répond pas. Le service est-il démarré (profil Docker « atrium ») ?",
  // Pendant un remplacement de conteneur, l'upstream peut etre indisponible
  // quelques centaines de ms. Un 404 reste en revanche definitif pour cette
  // version de l'API : le rejouer ne faisait que tripler les erreurs console.
  // Les ecritures ne sont jamais rejouees, pour eviter tout double effet.
  retryStatuses: [502, 503],
  makeError: (message, details) => new AtriumHttpError(message, details),
});

export const atriumGet = <T>(path: string) => request<T>("GET", path);

export const atriumPut = <T>(path: string, body?: unknown) =>
  request<T>("PUT", path, { body });

/// Le corps porte l'`actor_id` : l'API l'exige pour tracer qui a demande
/// l'effacement. `DELETE` avec corps est accepte par fetch comme par axum.
export const atriumDelete = <T>(path: string, body?: unknown) =>
  request<T>("DELETE", path, { body });
