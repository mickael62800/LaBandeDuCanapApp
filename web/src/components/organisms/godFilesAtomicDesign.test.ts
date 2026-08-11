import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import CommunityLfgPanel from "./community-life/CommunityLfgPanel.vue";
import CommunityNewsPanel from "./community-life/CommunityNewsPanel.vue";
import CommunityPollsPanel from "./community-life/CommunityPollsPanel.vue";
import ModerationJournalFilters from "./moderation-journal/ModerationJournalFilters.vue";
import type {
  AdminLfgPost,
  AdminNewsItem,
  AdminPoll,
  CreatePollInput,
  UpsertNewsInput,
} from "@/services/communityAdminService";

describe("découpage Atomic Design des god files", () => {
  it("ModerationJournalFilters ne possède pas l'état des filtres", async () => {
    const wrapper = mount(ModerationJournalFilters, {
      props: {
        search: "",
        type: "all",
        moderator: "all",
        status: "all",
        dateFrom: "",
        dateTo: "",
        hideDetections: true,
        typeOptions: [{ value: "all", label: "Tous" }],
        moderatorOptions: [{ value: "all", label: "Tous" }],
        statusOptions: [{ value: "all", label: "Tous" }],
        hasActiveFilters: true,
        selectedGuildId: "guild-1",
        purging: false,
        bulkMenuOpen: false,
      },
    });

    await wrapper.find('input[type="text"]').setValue("spam");
    await wrapper.find(".reset-btn").trigger("click");

    expect(wrapper.emitted("update:search")).toEqual([["spam"]]);
    expect(wrapper.emitted("reset")).toHaveLength(1);
  });

  it("CommunityNewsPanel délègue édition et suppression", async () => {
    const item: AdminNewsItem = {
      id: "news-1",
      title: "Soirée communautaire",
      body: "Rendez-vous vendredi.",
      image_url: null,
      is_pinned: false,
      is_public: true,
      published_at: "2026-08-11T18:00:00Z",
      created_by: "admin",
    };
    const form: UpsertNewsInput = {
      title: "",
      body: "",
      is_pinned: false,
      is_public: true,
    };
    const wrapper = mount(CommunityNewsPanel, {
      props: { items: [item], editing: null, busy: false, form },
    });

    const buttons = wrapper.findAll("button");
    await buttons.find((button) => button.text() === "Modifier")?.trigger("click");
    await buttons.find((button) => button.text() === "Supprimer")?.trigger("click");

    expect(wrapper.emitted("edit")).toEqual([["news-1"]]);
    expect(wrapper.emitted("remove")).toEqual([[item]]);
  });

  it("CommunityPollsPanel délègue la clôture d'un sondage", async () => {
    const item: AdminPoll = {
      id: "poll-1",
      question: "Quel jeu ?",
      description: null,
      closes_at: "2026-08-20T18:00:00Z",
      is_closed: false,
      is_open: true,
      total_votes: 0,
      options: [],
      my_vote: null,
    };
    const form: CreatePollInput = {
      question: "",
      closes_at: "2026-08-20T18:00",
      is_public: true,
      options: [{ label: "A" }, { label: "B" }],
    };
    const wrapper = mount(CommunityPollsPanel, {
      props: { items: [item], opened: false, busy: false, optionsValid: true, form },
    });

    await wrapper.findAll("button").find((button) => button.text() === "Clore")?.trigger("click");
    expect(wrapper.emitted("close")).toEqual([[item]]);
  });

  it("CommunityLfgPanel délègue fermeture et suppression", async () => {
    const item: AdminLfgPost = {
      id: "lfg-1",
      author_id: "user-1",
      author_name: "Joueur",
      game: "Minecraft",
      game_server_id: null,
      slots: 2,
      when_text: "Ce soir",
      description: null,
      is_open: true,
      expires_at: "2099-08-20T18:00:00Z",
      created_at: "2026-08-11T18:00:00Z",
      interested: [],
      remaining_slots: 2,
      is_full: false,
    };
    const wrapper = mount(CommunityLfgPanel, { props: { items: [item], busy: false } });

    const buttons = wrapper.findAll("button");
    await buttons.find((button) => button.text() === "Fermer")?.trigger("click");
    await buttons.find((button) => button.text() === "Supprimer")?.trigger("click");

    expect(wrapper.emitted("close")).toEqual([[item]]);
    expect(wrapper.emitted("remove")).toEqual([[item]]);
  });
});
