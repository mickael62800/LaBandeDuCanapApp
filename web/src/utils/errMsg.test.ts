import { describe, expect, it } from "vitest";
import { errMsg } from "./errMsg";

describe("errMsg", () => {
  it("extrait le message d'une Error", () => {
    expect(errMsg(new Error("boom"))).toBe("boom");
    expect(errMsg(new TypeError("typage"))).toBe("typage");
  });

  it("convertit une valeur non-Error en chaine", () => {
    expect(errMsg("chaine brute")).toBe("chaine brute");
    expect(errMsg(42)).toBe("42");
    expect(errMsg(null)).toBe("null");
    expect(errMsg(undefined)).toBe("undefined");
    expect(errMsg({ code: "invalid" })).toBe("[object Object]");
  });
});
