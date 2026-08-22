import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ nexusGet: vi.fn(), nexusPost: vi.fn() }));

vi.mock("@/api/nexusHttp", () => mocks);

import { nexusGrandSalonService as gs } from "./nexusGrandSalonService";

describe("nexusGrandSalonService (plateau Grand Salon)", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset().mockResolvedValue({});
  });

  it("membership lit l'appartenance d'un habitue", async () => {
    const profil = { user_id: "u1" };
    mocks.nexusGet.mockResolvedValue(profil);

    await expect(gs.membership("g/9", "u&2")).resolves.toBe(profil);
    expect(mocks.nexusGet).toHaveBeenCalledWith(
      "/api/grand-salon/g%2F9/membership/u%262",
      "g/9",
    );
  });

  it("profile lit le profil habitue (route /habitues)", async () => {
    await gs.profile("g1", "u3");
    expect(mocks.nexusGet).toHaveBeenCalledWith("/api/grand-salon/g1/habitues/u3", "g1");
  });

  it("join enregistre un nouvel habitue avec son nom d'affichage", async () => {
    await gs.join("g2", "u4", "Le Comte");
    expect(mocks.nexusPost).toHaveBeenCalledWith(
      "/api/grand-salon/g2/habitues/u4",
      "g2",
      { display_name: "Le Comte" },
    );
  });

  it("daily declenche le bonus quotidien (POST sans corps)", async () => {
    await gs.daily("g3", "u5");
    expect(mocks.nexusPost).toHaveBeenCalledWith(
      "/api/grand-salon/g3/habitues/u5/daily",
      "g3",
    );
  });

  it("motions lit les motions en cours de vote", async () => {
    const motions = [{ id: "m1" }];
    mocks.nexusGet.mockResolvedValue(motions);

    await expect(gs.motions("g4")).resolves.toBe(motions);
    expect(mocks.nexusGet).toHaveBeenCalledWith("/api/grand-salon/g4/motions", "g4");
  });

  it("propose depose une motion (titre + texte)", async () => {
    await gs.propose("g5", "u6", "Titre", "Corps de la motion");
    expect(mocks.nexusPost).toHaveBeenCalledWith(
      "/api/grand-salon/g5/motions",
      "g5",
      { user_id: "u6", titre: "Titre", texte: "Corps de la motion" },
    );
  });

  it("vote enregistre un choix pour/contre sur une motion", async () => {
    await gs.vote("g6", "m7", "u8", true);
    expect(mocks.nexusPost).toHaveBeenCalledWith(
      "/api/grand-salon/g6/motions/m7/vote",
      "g6",
      { user_id: "u8", choice: true },
    );

    await gs.vote("g6", "m7", "u9", false);
    expect(mocks.nexusPost).toHaveBeenLastCalledWith(
      "/api/grand-salon/g6/motions/m7/vote",
      "g6",
      { user_id: "u9", choice: false },
    );
  });

  it("gazette lit les articles publies", async () => {
    const articles = [{ id: "a1" }];
    mocks.nexusGet.mockResolvedValue(articles);

    await expect(gs.gazette("g7")).resolves.toBe(articles);
    expect(mocks.nexusGet).toHaveBeenCalledWith("/api/grand-salon/g7/gazette", "g7");
  });

  it("cercles lit les cercles de la guilde", async () => {
    const cercles = [{ id: "ce1" }];
    mocks.nexusGet.mockResolvedValue(cercles);

    await expect(gs.cercles("g8")).resolves.toBe(cercles);
    expect(mocks.nexusGet).toHaveBeenCalledWith("/api/grand-salon/g8/cercles", "g8");
  });

  it("createCercle fonde un cercle de type bande (nom + devise)", async () => {
    await gs.createCercle("g9", "u10", "Les Canapards", "Douceur avant tout");
    expect(mocks.nexusPost).toHaveBeenCalledWith(
      "/api/grand-salon/g9/cercles",
      "g9",
      { user_id: "u10", kind: "bande", name: "Les Canapards", devise: "Douceur avant tout" },
    );
  });

  it("dossiers lit les dossiers d'un sujet (utilisateur encode)", async () => {
    const dossiers = [{ id: "d1" }];
    mocks.nexusGet.mockResolvedValue(dossiers);

    await expect(gs.dossiers("g/1", "u&2")).resolves.toBe(dossiers);
    expect(mocks.nexusGet).toHaveBeenCalledWith(
      "/api/grand-salon/g%2F1/dossiers/u%262",
      "g/1",
    );
  });

  it("investigate ouvre un dossier sur un sujet", async () => {
    await gs.investigate("g3", "u4", "disparition du coussin");
    expect(mocks.nexusPost).toHaveBeenCalledWith(
      "/api/grand-salon/g3/dossiers",
      "g3",
      { user_id: "u4", subject: "disparition du coussin" },
    );
  });

  it("reveal declassifie un dossier", async () => {
    await gs.reveal("g5", "d6", "u7");
    expect(mocks.nexusPost).toHaveBeenCalledWith(
      "/api/grand-salon/g5/dossiers/d6/reveal",
      "g5",
      { user_id: "u7" },
    );
  });

  it("propage les erreurs du client Nexus", async () => {
    const erreur = new Error("401");
    mocks.nexusGet.mockRejectedValue(erreur);
    await expect(gs.motions("gX")).rejects.toBe(erreur);
  });
});
