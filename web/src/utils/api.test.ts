import { beforeEach, describe, expect, it } from "vitest";

import { getApiBaseUrl } from "./api";

// `getApiConfig` lit la config dans localStorage (cle K_API) et applique une
// whitelist d'origines : en dev elle tolere localhost/127.0.0.1 sur n'importe
// quel port. On s'appuie dessus pour piloter chaque branche de getApiBaseUrl.
const K_API = "ds.api.config";

describe("getApiBaseUrl", () => {
  beforeEach(() => {
    localStorage.clear();
    delete import.meta.env.VITE_API_URL;
  });

  it("prefere l'api_url de la config locale (branche 1)", async () => {
    // localhost:4567 passe la whitelist dev -> renvoye tel quel.
    localStorage.setItem(K_API, JSON.stringify({ api_url: "http://localhost:4567" }));
    expect(await getApiBaseUrl()).toBe("http://localhost:4567");
  });

  it("tombe sur VITE_API_URL quand la config est vide (branche 2)", async () => {
    import.meta.env.VITE_API_URL = "http://vite.local:3001";
    expect(await getApiBaseUrl()).toBe("http://vite.local:3001");
  });

  it("tombe sur localhost en dev quand rien d'autre n'est defini (branche 3)", async () => {
    // import.meta.env.PROD est faux sous vitest -> fallback dev.
    expect(await getApiBaseUrl()).toBe("http://localhost:3000");
  });
});
