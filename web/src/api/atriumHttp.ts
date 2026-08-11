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
  // Pendant un `docker compose up --build`, nginx peut deja servir le nouveau
  // SPA alors que l'ancien conteneur Atrium termine son remplacement. Une
  // nouvelle route GET repond alors 404 pendant quelques centaines de ms.
  // Les ecritures ne sont jamais rejouees, pour eviter tout double effet.
  retryStatuses: [404, 502, 503],
  makeError: (message, details) => new AtriumHttpError(message, details),
});

export const atriumGet = <T>(path: string) => request<T>("GET", path);

export const atriumPut = <T>(path: string, body?: unknown) =>
  request<T>("PUT", path, { body });
