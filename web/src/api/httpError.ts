export interface HttpErrorDetails {
  status: number;
  code?: string;
  body?: unknown;
  requestId?: string;
  backend?: string;
}

/** Erreur HTTP commune à tous les backends du Web. */
export class HttpError extends Error {
  readonly status: number;
  readonly code?: string;
  readonly body?: unknown;
  readonly requestId?: string;
  readonly backend?: string;

  constructor(message: string, details: HttpErrorDetails, name = "HttpError") {
    super(message);
    this.name = name;
    this.status = details.status;
    this.code = details.code;
    this.body = details.body;
    this.requestId = details.requestId;
    this.backend = details.backend;
  }
}

export class HttpTimeoutError extends Error {
  constructor(public readonly timeoutMs: number) {
    super(`La requête a dépassé le délai de ${timeoutMs} ms.`);
    this.name = "HttpTimeoutError";
  }
}

const PREFIXES_TECHNIQUES = [
  "Données invalides : ",
  "Données invalides: ",
  "Donnees invalides : ",
  "Donnees invalides: ",
  "Conflit : ",
  "Conflit: ",
  "Validation : ",
  "Validation: ",
];

export function messageFromErrorBody(status: number, body: unknown): string {
  let message = "";
  if (body && typeof body === "object") {
    const record = body as Record<string, unknown>;
    const candidate = record.error ?? record.message;
    if (typeof candidate === "string") message = candidate;
  } else if (typeof body === "string") {
    message = body.slice(0, 200);
  }

  message = message.trim();
  for (const prefix of PREFIXES_TECHNIQUES) {
    if (message.startsWith(prefix)) {
      message = message.slice(prefix.length);
      break;
    }
  }
  return message || `Erreur ${status}`;
}

export function errorDetails(
  response: Response,
  body: unknown,
  backend?: string,
): HttpErrorDetails {
  const record = body && typeof body === "object"
    ? body as Record<string, unknown>
    : undefined;
  const code = typeof record?.code === "string" ? record.code : undefined;
  const bodyRequestId = typeof record?.request_id === "string"
    ? record.request_id
    : undefined;
  return {
    status: response.status,
    code,
    body,
    requestId: response.headers.get("X-Request-Id") ?? bodyRequestId,
    backend,
  };
}
