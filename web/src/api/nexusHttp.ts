// Client du backend Nexus, servi en meme origine par nginx.
import { BackendHttpError, createBackendClient } from "./backendHttp";
import type { HttpErrorDetails } from "./httpError";

const NEXUS_BASE = "/nexus-api";

export class NexusHttpError extends BackendHttpError {
  constructor(message: string, details: HttpErrorDetails) {
    super(message, details, "NexusHttpError");
  }
}

const request = createBackendClient({
  baseUrl: NEXUS_BASE,
  errorLabel: "Nexus",
  forbiddenMessage: "Accès à la plateforme jeux refusé.",
  emptyStatuses: [202, 204],
  makeError: (message, details) => new NexusHttpError(message, details),
});

const guildHeaders = (guildId: string | null) =>
  guildId ? { "X-Guild-Id": guildId } : undefined;

export const nexusGet = <T>(path: string, guildId: string | null) =>
  request<T>("GET", path, { headers: guildHeaders(guildId) });

export const nexusPost = <T>(path: string, guildId: string | null, body?: unknown) =>
  request<T>("POST", path, { body, headers: guildHeaders(guildId) });

export const nexusPut = <T>(path: string, guildId: string | null, body?: unknown) =>
  request<T>("PUT", path, { body, headers: guildHeaders(guildId) });

export const nexusPatch = <T>(path: string, guildId: string | null, body?: unknown) =>
  request<T>("PATCH", path, { body, headers: guildHeaders(guildId) });

export const nexusDelete = <T>(path: string, guildId: string | null) =>
  request<T>("DELETE", path, { headers: guildHeaders(guildId) });
