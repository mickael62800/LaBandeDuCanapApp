import { beforeEach, describe, expect, it } from "vitest";

import {
  clearDiscordToken,
  getDiscordToken,
  getDiscordUser,
  setDiscordToken,
  setDiscordUser,
} from "./config";

const K_USER = "ds.discord.user";
const K_TOKEN = "ds.discord.token";

describe("identite Discord (getDiscordUser / setDiscordUser)", () => {
  beforeEach(() => {
    localStorage.clear();
    sessionStorage.clear();
  });

  it("relit l'utilisateur ecrit", () => {
    setDiscordUser({ id: "123", username: "micka" });
    expect(getDiscordUser()).toEqual({ id: "123", username: "micka" });
  });

  it("conserve les champs optionnels", () => {
    setDiscordUser({
      id: "123",
      username: "micka",
      avatar: "https://cdn.discordapp.com/a.png",
      global_name: "Micka",
      is_superadmin: true,
    });
    expect(getDiscordUser()?.is_superadmin).toBe(true);
    expect(getDiscordUser()?.global_name).toBe("Micka");
  });

  it("retourne null quand rien n'est stocke", () => {
    expect(getDiscordUser()).toBeNull();
  });

  it("rejette une entree sans id ou username et la supprime", () => {
    localStorage.setItem(K_USER, JSON.stringify({ username: "sans-id" }));
    expect(getDiscordUser()).toBeNull();
    expect(localStorage.getItem(K_USER)).toBeNull();
  });

  it("supprime la valeur quand on ecrit null", () => {
    setDiscordUser({ id: "123", username: "micka" });
    setDiscordUser(null);
    expect(getDiscordUser()).toBeNull();
    expect(localStorage.getItem(K_USER)).toBeNull();
  });
});

describe("token Discord (sessionStorage) et migration", () => {
  beforeEach(() => {
    localStorage.clear();
    sessionStorage.clear();
  });

  it("relit le token ecrit en sessionStorage", () => {
    setDiscordToken("tok-123");
    expect(getDiscordToken()).toBe("tok-123");
  });

  it("retourne une chaine vide sans token", () => {
    expect(getDiscordToken()).toBe("");
  });

  it("migre un token herite du localStorage vers sessionStorage", () => {
    localStorage.setItem(K_TOKEN, "legacy-token");
    expect(getDiscordToken()).toBe("legacy-token");
    // Apres migration, plus rien en localStorage.
    expect(localStorage.getItem(K_TOKEN)).toBeNull();
    expect(sessionStorage.getItem(K_TOKEN)).toBe("legacy-token");
  });

  it("setDiscordToken purge eventuellement un vieux token localStorage", () => {
    localStorage.setItem(K_TOKEN, "legacy-token");
    setDiscordToken("fresh-token");
    expect(getDiscordToken()).toBe("fresh-token");
    expect(localStorage.getItem(K_TOKEN)).toBeNull();
  });

  it("clearDiscordToken vide les deux stockages", () => {
    setDiscordToken("tok-123");
    localStorage.setItem(K_TOKEN, "legacy");
    clearDiscordToken();
    expect(getDiscordToken()).toBe("");
    expect(localStorage.getItem(K_TOKEN)).toBeNull();
    expect(sessionStorage.getItem(K_TOKEN)).toBeNull();
  });
});
