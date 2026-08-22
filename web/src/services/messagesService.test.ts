import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ httpPost: vi.fn() }));

vi.mock("@/api/http", () => mocks);

import { MAX_MESSAGE_LENGTH, messagesService } from "./messagesService";

describe("messagesService (envoi de message par le bot)", () => {
  beforeEach(() => {
    mocks.httpPost.mockReset().mockResolvedValue({ queued: true });
  });

  it("expose la limite Discord dupliquee pour le compteur", () => {
    expect(MAX_MESSAGE_LENGTH).toBe(2000);
  });

  it("send envoie du texte seul (image_url null)", async () => {
    await messagesService.send("g1", "c9", "Salut la bande !");

    expect(mocks.httpPost).toHaveBeenCalledWith("/api/messages/g1/c9", {
      content: "Salut la bande !",
      image_url: null,
    });
  });

  it("send accepte une image seule (contenu vide)", async () => {
    await messagesService.send("g2", "c8", "", "https://cdn.example.com/img.png");

    expect(mocks.httpPost).toHaveBeenCalledWith("/api/messages/g2/c8", {
      content: "",
      image_url: "https://cdn.example.com/img.png",
    });
  });

  it("send nettoie l'URL d'image (espaces) et la rend null si vide apres trim", async () => {
    await messagesService.send("g3", "c7", "texte", "   https://x.fr/a.png  ");
    expect(mocks.httpPost).toHaveBeenLastCalledWith("/api/messages/g3/c7", {
      content: "texte",
      image_url: "https://x.fr/a.png",
    });

    await messagesService.send("g4", "c6", "texte", "   ");
    expect(mocks.httpPost).toHaveBeenLastCalledWith("/api/messages/g4/c6", {
      content: "texte",
      image_url: null,
    });
  });

  it("propage les erreurs du transport (le bot n'a pas encore poste)", async () => {
    const erreur = new Error("502");
    mocks.httpPost.mockRejectedValue(erreur);
    await expect(messagesService.send("gX", "cY", "bonjour")).rejects.toBe(erreur);
  });
});
