import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import GamesCatalogNavigation from "./games/GamesCatalogNavigation.vue";
import WheelGamePanel from "./games/WheelGamePanel.vue";
import MemberPollsPanel from "./member-home/MemberPollsPanel.vue";
import { GAMES } from "@/games/catalog";
import type { Poll } from "@/services/communityLifeService";

describe("organisms publics Atomic Design", () => {
  it("GamesCatalogNavigation expose la sélection et la navigation sans posséder l'état", async () => {
    const wrapper = mount(GamesCatalogNavigation, {
      props: { games: GAMES, activeKey: "roue" },
    });

    await wrapper.findAll(".jx-vignette")[1]?.trigger("click");
    await wrapper.find('[aria-label="Jeu suivant"]').trigger("click");

    expect(wrapper.emitted("select")).toEqual([["coussin"]]);
    expect(wrapper.emitted("shift")).toEqual([[1]]);
    expect(wrapper.find('[aria-current="true"]').text()).toContain("La Roue du Destin");
  });

  it("WheelGamePanel rend le résultat serveur et délègue le tirage", async () => {
    const wrapper = mount(WheelGamePanel, {
      props: {
        cases: [{ key: "jackpot", emoji: "🎰" }],
        sector: 360,
        background: "conic-gradient(red 0deg 360deg)",
        angle: 720,
        spinning: false,
        alreadyPlayed: false,
        error: null,
        result: {
          case_key: "jackpot",
          case_label: "Jackpot",
          payout: 5000,
          balance_after: 7000,
          is_memorable: true,
        },
      },
    });

    await wrapper.find("button").trigger("click");
    expect(wrapper.emitted("spin")).toHaveLength(1);
    expect(wrapper.text()).toContain("Jackpot");
    expect(wrapper.text()).toContain("5 000 coins");
    expect(wrapper.find(".jx-resultat").classes()).toContain("rare");
  });

  it("MemberPollsPanel interdit le vote anonyme et émet un vote authentifié", async () => {
    const poll: Poll = {
      id: "poll-1",
      question: "On joue à quoi ?",
      description: null,
      closes_at: "2026-08-15T18:00:00Z",
      is_closed: false,
      is_open: true,
      total_votes: 3,
      my_vote: null,
      options: [{ id: "option-1", label: "Minecraft", color: "22c55e", votes: 3, share: 100 }],
    };
    const wrapper = mount(MemberPollsPanel, {
      props: { polls: [poll], authenticated: true, busyId: null },
    });

    await wrapper.find(".mb-poll-line").trigger("click");
    expect(wrapper.emitted("vote")).toEqual([["poll-1", "option-1"]]);

    await wrapper.setProps({ authenticated: false });
    expect(wrapper.find(".mb-poll-line").attributes("disabled")).toBeDefined();
  });
});
