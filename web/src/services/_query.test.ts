import { describe, expect, it } from "vitest";
import { q } from "./_query";

describe("q", () => {
  it("renvoie une chaine vide quand l'objet est vide", () => {
    expect(q({})).toBe("");
  });

  it("encode les paires cle/valeur avec un point d'interrogation initial", () => {
    expect(q({ a: "1" })).toBe("?a=1");
  });

  it("joint plusieurs paires par & et encode-URI les valeurs", () => {
    expect(q({ a: "x y", b: "2&3" })).toBe("?a=x%20y&b=2%263");
  });

  it("saute les valeurs null et undefined", () => {
    expect(q({ a: "1", b: null, c: undefined })).toBe("?a=1");
  });

  it("convertit les nombres en chaine", () => {
    expect(q({ limit: 50 })).toBe("?limit=50");
  });
});
