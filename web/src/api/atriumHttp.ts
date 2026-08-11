// Client du backend Atrium, servi en meme origine par nginx.
import { BackendHttpError, createBackendClient } from "./backendHttp";

const ATRIUM_BASE = "/atrium-api";

export class AtriumHttpError extends BackendHttpError {
  constructor(message: string, status: number) {
    super(message, status, "AtriumHttpError");
  }
}

const request = createBackendClient({
  baseUrl: ATRIUM_BASE,
  errorLabel: "Atrium",
  forbiddenMessage: "Accès à Atrium refusé.",
  unavailableMessage:
    "Atrium ne répond pas. Le service est-il démarré (profil Docker « atrium ») ?",
  makeError: (message, status) => new AtriumHttpError(message, status),
});

export const atriumGet = <T>(path: string) => request<T>("GET", path);

export const atriumPut = <T>(path: string, body?: unknown) =>
  request<T>("PUT", path, { body });
