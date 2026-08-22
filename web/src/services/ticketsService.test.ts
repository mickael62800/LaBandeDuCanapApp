import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpDelete: vi.fn(),
  httpGet: vi.fn(),
  httpPatch: vi.fn(),
  httpPost: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import { ticketsService } from "./ticketsService";

describe("ticketsService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset();
  });

  it("getAll lit tous les tickets", async () => {
    const tickets = [{ id: "t1" }];
    mocks.httpGet.mockResolvedValue(tickets);

    await expect(ticketsService.getAll()).resolves.toEqual(tickets);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/tickets");
  });

  it("getDetail lit le detail d'un ticket", async () => {
    const detail = { id: "t1", messages: [] };
    mocks.httpGet.mockResolvedValue(detail);

    await expect(ticketsService.getDetail("t1")).resolves.toEqual(detail);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/tickets/t1");
  });

  it("reply envoie un message au ticket", async () => {
    mocks.httpPost.mockResolvedValue({ id: "m1" });

    await expect(ticketsService.reply("t1", "bonjour")).resolves.toEqual({
      id: "m1",
    });
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/tickets/t1/messages", {
      content: "bonjour",
    });
  });

  it("close ferme le ticket", async () => {
    mocks.httpPatch.mockResolvedValue({ closed: true });

    await expect(ticketsService.close("t1")).resolves.toEqual({ closed: true });
    expect(mocks.httpPatch).toHaveBeenCalledWith("/api/tickets/t1/close");
  });

  it("assign affecte le ticket a un agent", async () => {
    mocks.httpPatch.mockResolvedValue(undefined);

    await ticketsService.assign("t1", "u9");
    expect(mocks.httpPatch).toHaveBeenCalledWith("/api/tickets/t1/assign", {
      assignee: "u9",
    });
  });

  it("bulkDelete sans filtre exige le drapeau all", async () => {
    mocks.httpDelete.mockResolvedValue({ deleted: 3, author_id: null, from: null, to: null });

    await ticketsService.bulkDelete({ all: true });
    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/tickets/bulk?all=true");
  });

  it("bulkDelete filtre par auteur et periode", async () => {
    mocks.httpDelete.mockResolvedValue({ deleted: 1, author_id: "u2" });

    await ticketsService.bulkDelete({
      author_id: "u2",
      from: "2026-08-01T00:00:00Z",
      to: "2026-08-31T00:00:00Z",
    });
    expect(mocks.httpDelete).toHaveBeenCalledWith(
      "/api/tickets/bulk?author_id=u2&from=2026-08-01T00%3A00%3A00Z&to=2026-08-31T00%3A00%3A00Z",
    );
  });

  it("bulkDelete sans aucun parametre ne produit aucune query string", async () => {
    mocks.httpDelete.mockResolvedValue({ deleted: 0, author_id: null, from: null, to: null });

    await ticketsService.bulkDelete({});
    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/tickets/bulk");
  });
});
