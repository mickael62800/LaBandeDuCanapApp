import { describe, expect, it } from "vitest";
import { errorDetails, HttpError, HttpTimeoutError, messageFromErrorBody } from "./httpError";

describe("HttpError (erreur HTTP commune)", () => {
  const details = { status: 429, code: "RATE_LIMITED", body: null, requestId: "r-1" };

  it("expose statut/code/corps/requestId et le nom par defaut", () => {
    const erreur = new HttpError("Trop de requetes.", details);

    expect(erreur).toBeInstanceOf(Error);
    expect(erreur.name).toBe("HttpError");
    expect(erreur.message).toBe("Trop de requetes.");
    expect(erreur.status).toBe(429);
    expect(erreur.code).toBe("RATE_LIMITED");
    expect(erreur.requestId).toBe("r-1");
  });

  it("accepte un nom d'erreur surcharge (sous-classe)", () => {
    const erreur = new HttpError("Acces refuse.", { status: 403 }, "NexusHttpError");
    expect(erreur.name).toBe("NexusHttpError");
  });

  it("laisse les champs optionnels vides quand absents", () => {
    const erreur = new HttpError("Erreur 502", { status: 502, backend: "LaBande" });
    expect(erreur.code).toBeUndefined();
    expect(erreur.body).toBeUndefined();
    expect(erreur.requestId).toBeUndefined();
    expect(erreur.backend).toBe("LaBande");
  });
});

describe("HttpTimeoutError", () => {
  it("decrit le depassement de delai avec la duree fournie", () => {
    const erreur = new HttpTimeoutError(1500);
    expect(erreur).toBeInstanceOf(Error);
    expect(erreur.name).toBe("HttpTimeoutError");
    expect(erreur.message).toContain("1500 ms");
  });
});

describe("messageFromErrorBody (extraction du message lisible)", () => {
  it("prefere le champ error puis message d'un objet JSON", () => {
    expect(messageFromErrorBody(422, { error: "Champ requis manquant." })).toBe("Champ requis manquant.");
    expect(messageFromErrorBody(500, { message: "Boom interne." })).toBe("Boom interne.");
  });

  it("ignore les champs non-strings et retombe sur Erreur {status}", () => {
    expect(messageFromErrorBody(418, { error: 7 })).toBe("Erreur 418");
    expect(messageFromErrorBody(503, null)).toBe("Erreur 503");
    expect(messageFromErrorBody(204, undefined)).toBe("Erreur 204");
  });

  it("tronque un corps texte brut a 200 caracteres", () => {
    const long = "x".repeat(300);
    const message = messageFromErrorBody(500, long);
    expect(message).toHaveLength(200);
  });

  it("retire les prefixes techniques (accents ou non)", () => {
    for (const prefix of [
      "Données invalides : ",
      "Conflit: ",
      "Validation: ",
    ]) {
      expect(messageFromErrorBody(409, `${prefix}le nom existe deja`)).toBe("le nom existe deja");
    }
  });

  it("ne retire qu'un seul prefixe et ignore les corps vides", () => {
    expect(messageFromErrorBody(422, "   ")).toBe("Erreur 422");
    expect(messageFromErrorBody(409, "Conflit : Conflit : double")).toBe("Conflit : double");
  });
});

describe("errorDetails (details d'erreur depuis une reponse)", () => {
  function fakeResponse(status: number, headers?: Record<string, string>) {
    return { status, headers: new Headers(headers) } as Response;
  }

  it("lit code + request_id du corps JSON", () => {
    const details = errorDetails(
      fakeResponse(409),
      { code: "ALREADY_EXISTS", request_id: "body-1" },
      "LaBande",
    );

    expect(details).toEqual({
      status: 409,
      code: "ALREADY_EXISTS",
      body: { code: "ALREADY_EXISTS", request_id: "body-1" },
      requestId: "body-1", // pas d'en-tete : le corps fournit l'identifiant
      backend: "LaBande",
    });
  });

  it("prefere l'en-tete X-Request-Id sur celui du corps", () => {
    const details = errorDetails(
      fakeResponse(403, { "X-Request-Id": "header-9" }),
      { code: "FORBIDDEN", request_id: "body-2" },
    );

    expect(details.requestId).toBe("header-9");
  });

  it("ignore les corps non-objets et les champs mal formes", () => {
    const texte = errorDetails(fakeResponse(500), "<html>oops</html>", "Nexus");
    expect(texte.code).toBeUndefined();
    expect(texte.requestId).toBeUndefined();

    const malforme = errorDetails(fakeResponse(422), { code: 12, request_id: 3 });
    expect(malforme.code).toBeUndefined();
    expect(malforme.requestId).toBeUndefined();
  });
});
