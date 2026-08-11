// Client du backend d'exploitation, servi en meme origine par nginx.
import { BackendHttpError, createBackendClient } from "./backendHttp";

const OPS_BASE = "/ops-api";

export class OpsHttpError extends BackendHttpError {
  constructor(message: string, status: number) {
    super(message, status, "OpsHttpError");
  }
}

const request = createBackendClient({
  baseUrl: OPS_BASE,
  errorLabel: "exploitation",
  forbiddenMessage: "Accès à l'exploitation refusé.",
  unavailableMessage:
    "L'API d'exploitation ne répond pas. Le service est-il démarré ?",
  makeError: (message, status) => new OpsHttpError(message, status),
});

export const opsGet = <T>(path: string) => request<T>("GET", path);

export const opsPatch = <T>(path: string, body?: unknown) =>
  request<T>("PATCH", path, { body });

export const opsPost = <T>(path: string, body?: unknown) =>
  request<T>("POST", path, { body });

export const opsDelete = <T>(path: string, body?: unknown) =>
  request<T>("DELETE", path, { body });
