import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpDelete: vi.fn(),
  httpGet: vi.fn(),
  httpPost: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import { notesService } from "./notesService";

describe("notesService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset();
  });

  it("list lit les notes d'un membre du serveur", async () => {
    const notes = [{ id: "n1" }];
    mocks.httpGet.mockResolvedValue(notes);

    await expect(
      notesService.list("g1", "u2"),
    ).resolves.toEqual([{ id: "n1" }]);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/notes/g1/u2");
  });

  it("add envoie le corps de la nouvelle note", async () => {
    const body = {
      guild_id: "g1",
      user_id: "u2",
      author_id: "a9",
      author_name: "micka",
      content: "bien",
    };
    mocks.httpPost.mockResolvedValue({ id: "n9", ...body });

    await expect(notesService.add(body)).resolves.toEqual({
      id: "n9",
      ...body,
    });
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/notes", body);
  });

  it("remove supprime la note par identifiant", async () => {
    mocks.httpDelete.mockResolvedValue(undefined);

    await notesService.remove("n1");
    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/notes/n1");
  });
});
