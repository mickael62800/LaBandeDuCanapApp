import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  getApiConfig: vi.fn(),
  getDiscordUser: vi.fn(),
  setDiscordUser: vi.fn(),
  clearDiscordToken: vi.fn(),
}));

vi.mock("@/api/config", () => ({
  ...mocks,
}));

import { authService } from "./authService";

describe("authService", () => {
  let originalHrefDescriptor: PropertyDescriptor | undefined;
  let hrefSetter: ReturnType<typeof vi.fn> | undefined;

  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset();
    mocks.getApiConfig.mockReturnValue(null);
    hrefSetter = undefined;
  });

  afterEach(() => {
    if (originalHrefDescriptor) {
      // Restaure le descripteur d'origine de `location.href`.
      Object.defineProperty(window.location, "href", originalHrefDescriptor);
      originalHrefDescriptor = undefined;
    }
  });

  it("getCurrentUser renvoie l'utilisateur Discord stocke", () => {
    const user = { id: "42", username: "micka" };
    mocks.getDiscordUser.mockReturnValue(user);

    expect(authService.getCurrentUser()).toBe(user);
    expect(mocks.getDiscordUser).toHaveBeenCalledOnce();
  });

  it("getCurrentUser renvoie null quand aucun utilisateur n'est stocke", () => {
    mocks.getDiscordUser.mockReturnValue(null);

    expect(authService.getCurrentUser()).toBeNull();
  });

  it("logout efface l'utilisateur et le jeton Discord", () => {
    authService.logout();

    expect(mocks.setDiscordUser).toHaveBeenCalledWith(null);
    expect(mocks.clearDiscordToken).toHaveBeenCalledOnce();
  });

  it("discordLogin redirige vers la route d'autorisation du backend", async () => {
    // happy-dom ne navigue pas : on espionne le setter de `location.href`.
    originalHrefDescriptor = Object.getOwnPropertyDescriptor(
      window.location,
      "href",
    );
    hrefSetter = vi.fn();
    Object.defineProperty(window.location, "href", {
      configurable: true,
      get() { return ""; },
      set(value: string) { hrefSetter?.(value); },
    });

    // La promesse ne resolve jamais (la page navigue) : on verifie l'effet de
    // cote sans attendre sa resolution. On laisse passer les microtaches du
    // `await getApiBaseUrl()` avant d'asserter le setter espionne.
    void authService.discordLogin();
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(hrefSetter).toHaveBeenCalledWith(
      "http://localhost:3000/auth/discord/authorize",
    );
  });
});
