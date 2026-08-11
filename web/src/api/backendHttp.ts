import { getDiscordToken } from "./config";

export class BackendHttpError extends Error {
  constructor(
    message: string,
    public status: number,
    name = "BackendHttpError",
  ) {
    super(message);
    this.name = name;
  }
}

interface BackendClientOptions {
  baseUrl: string;
  errorLabel: string;
  forbiddenMessage: string;
  unavailableMessage?: string;
  emptyStatuses?: readonly number[];
  makeError: (message: string, status: number) => Error;
}

interface RequestOptions {
  body?: unknown;
  headers?: Record<string, string>;
}

/** Transport JSON commun aux backends internes servis par la passerelle. */
export function createBackendClient(options: BackendClientOptions) {
  const emptyStatuses = new Set(options.emptyStatuses ?? [204]);

  return async function request<T>(
    method: string,
    path: string,
    requestOptions: RequestOptions = {},
  ): Promise<T> {
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
      ...requestOptions.headers,
    };
    const token = getDiscordToken();
    if (token) headers["X-Discord-Token"] = token;

    const response = await fetch(`${options.baseUrl}${path}`, {
      method,
      headers,
      credentials: "include",
      body:
        requestOptions.body === undefined
          ? undefined
          : JSON.stringify(requestOptions.body),
    });

    if (!response.ok) {
      if (response.status === 401) {
        throw options.makeError("Session expirée — reconnecte-toi.", 401);
      }
      if (response.status === 403) {
        throw options.makeError(options.forbiddenMessage, 403);
      }
      if (
        options.unavailableMessage &&
        (response.status === 502 || response.status === 503)
      ) {
        throw options.makeError(options.unavailableMessage, response.status);
      }
      const detail = await response
        .json()
        .then((body: { error?: string }) => body?.error)
        .catch(() => null);
      throw options.makeError(
        detail ?? `Erreur ${options.errorLabel} (${response.status})`,
        response.status,
      );
    }

    if (emptyStatuses.has(response.status)) return undefined as T;
    return (await response.json()) as T;
  };
}
