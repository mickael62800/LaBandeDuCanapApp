import { beforeEach, describe, expect, it } from "vitest";

import { getApiConfig, setApiConfig } from "./config";

const K_API = "ds.api.config";

describe("getApiConfig", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("relit la configuration ecrite", () => {
    setApiConfig({ api_url: window.location.origin });
    expect(getApiConfig()).toEqual({ api_url: window.location.origin });
  });

  // Le coeur de W4. Retirer `api_key` du contrat ne l'efface pas des postes qui
  // l'ont deja stockee : plus personne ne l'envoie, mais une XSS peut toujours
  // la lire dans le localStorage. La lecture doit donc reecrire l'entree.
  it("purge une cle API heritee du localStorage", () => {
    localStorage.setItem(
      K_API,
      JSON.stringify({ api_url: window.location.origin, api_key: "secret-interne" }),
    );

    const cfg = getApiConfig();

    expect(cfg).toEqual({ api_url: window.location.origin });
    expect(cfg).not.toHaveProperty("api_key");
    // Et surtout : la valeur ne doit plus etre lisible sur le poste.
    expect(localStorage.getItem(K_API)).not.toContain("secret-interne");
    expect(localStorage.getItem(K_API)).not.toContain("api_key");
  });

  it("rejette une entree sans api_url et la supprime", () => {
    localStorage.setItem(K_API, JSON.stringify({ api_key: "secret-interne" }));

    expect(getApiConfig()).toBeNull();
    expect(localStorage.getItem(K_API)).toBeNull();
  });

  // L'assainissement d'origine existait avant W4 : on verifie qu'il survit a la
  // reecriture, sinon la purge aurait pu figer une URL empoisonnee.
  it("ramene une api_url d'origine inconnue sur l'origin courant", () => {
    localStorage.setItem(
      K_API,
      JSON.stringify({ api_url: "https://evil.example", api_key: "secret-interne" }),
    );

    expect(getApiConfig()).toEqual({ api_url: window.location.origin });
    expect(localStorage.getItem(K_API)).not.toContain("secret-interne");
  });
});
