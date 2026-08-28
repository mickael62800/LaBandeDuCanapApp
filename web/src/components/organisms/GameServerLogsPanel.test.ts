import { describe, expect, it, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";

vi.mock("@/services/nexusGamesService", () => ({
  nexusGamesService: {
    logs: vi.fn(),
  },
}));

const successToast = vi.fn();
const errorToast = vi.fn();
vi.mock("@/composables/useToast", () => ({
  useToast: () => ({
    success: (...a: unknown[]) => successToast(...a),
    error: (...a: unknown[]) => errorToast(...a),
  }),
}));

import GameServerLogsPanel from "./GameServerLogsPanel.vue";
import { nexusGamesService } from "@/services/nexusGamesService";

const mockLogsService = vi.mocked(nexusGamesService);

describe("GameServerLogsPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("charge et affiche les logs au montage", async () => {
    mockLogsService.logs.mockResolvedValue([
      "[INFO] Server starting...",
      "[WARN] High memory load",
      "[ERROR] Database connection lost",
      "Player 'Steve' joined the game",
    ]);

    const wrapper = mount(GameServerLogsPanel, {
      props: {
        guildId: "g1",
        serverId: "srv1",
        serverName: "Valheim Canap",
        isRunning: true,
      },
    });

    await flushPromises();

    expect(mockLogsService.logs).toHaveBeenCalledWith("g1", "srv1", 300);
    expect(wrapper.text()).toContain("Console des Logs");
    expect(wrapper.text()).toContain("Server starting...");
    expect(wrapper.text()).toContain("High memory load");
    expect(wrapper.text()).toContain("Database connection lost");
    expect(wrapper.text()).toContain("Steve");

    // Vérifie les compteurs de statistiques
    expect(wrapper.text()).toContain("Erreurs");
    expect(wrapper.text()).toContain("Alertes");
    expect(wrapper.text()).toContain("Joueurs");
  });

  it("filtre les logs par niveau quand on clique sur le filtre d'erreurs", async () => {
    mockLogsService.logs.mockResolvedValue([
      "[INFO] Normal line",
      "[ERROR] Critical failure",
      "[WARN] Be careful",
    ]);

    const wrapper = mount(GameServerLogsPanel, {
      props: {
        guildId: "g1",
        serverId: "srv1",
      },
    });

    await flushPromises();

    const errorPill = wrapper.findAll(".gsl-pill.pill-error")[0];
    expect(errorPill).toBeDefined();
    await errorPill!.trigger("click");

    expect(wrapper.text()).toContain("Critical failure");
    expect(wrapper.text()).not.toContain("Normal line");
  });

  it("filtre les logs via le champ de recherche", async () => {
    mockLogsService.logs.mockResolvedValue([
      "Binding port 16261",
      "Player 'Alice' joined",
      "Autosave finished",
    ]);

    const wrapper = mount(GameServerLogsPanel, {
      props: {
        guildId: "g1",
        serverId: "srv1",
      },
    });

    await flushPromises();

    const searchInput = wrapper.find(".gsl-search-input");
    await searchInput.setValue("Alice");

    expect(wrapper.text()).toContain("Alice");
    expect(wrapper.text()).not.toContain("Binding port");
    expect(wrapper.text()).not.toContain("Autosave finished");
  });

  it("affiche un message d'erreur si la récupération échoue", async () => {
    mockLogsService.logs.mockRejectedValue(new Error("Serveur injoignable"));

    const wrapper = mount(GameServerLogsPanel, {
      props: {
        guildId: "g1",
        serverId: "srv1",
      },
    });

    await flushPromises();

    expect(wrapper.text()).toContain("Serveur injoignable");
  });
});
