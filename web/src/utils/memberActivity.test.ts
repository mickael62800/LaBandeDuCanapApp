import { describe, expect, it } from "vitest";
import type { UserActivity, UserDossier } from "../types";
import {
  activityAttachmentsCount,
  activityCount,
  activityLabel,
  activityLinksCount,
  activityVariant,
  activityWithin,
  attachmentCounts,
  avatarDiff,
  automodCount,
  burstCount,
  countByCategory,
  editedBeforeAfter,
  eventCategory,
  formatDuration,
  heatColor,
  heatmapData,
  metaArr,
  metaStr,
  profileDiff,
  rolesCount,
  rolesDiff,
  topChannels,
  topVoiceCompanions,
  voiceChannelLabel,
  voiceHours,
  watchSplitStats,
} from "./memberActivity";

// ── Fabriques ────────────────
function act(partial: Partial<UserActivity> = {}): UserActivity {
  return {
    id: "a1",
    guild_id: "g1",
    user_id: "u1",
    event_type: "message_sent",
    channel_id: "c1",
    channel_name: "general",
    content: null,
    metadata: {},
    created_at: new Date().toISOString(),
    ...partial,
  };
}

describe("formatage", () => {
  it("formatDuration en heures + minutes", () => {
    expect(formatDuration(3600)).toBe("1h 0m");
    expect(formatDuration(7500)).toBe("2h 5m");
    expect(formatDuration(59)).toBe("0m");
  });

  it("rolesCount compte les roles", () => {
    expect(rolesCount(["a", "b", "c"])).toBe(3);
    expect(rolesCount([])).toBe(0);
  });
});

describe("categorisation", () => {
  it("eventCategory", () => {
    expect(eventCategory("message_sent")).toBe("text");
    expect(eventCategory("message_edited")).toBe("text");
    expect(eventCategory("voice_join")).toBe("vocal");
    expect(eventCategory("voice_leave")).toBe("vocal");
    expect(eventCategory("member_join")).toBe("other");
  });

  it("activityCount filtre par type", () => {
    const list = [act({ id: "1" }), act({ id: "2", event_type: "voice_join" })];
    expect(activityCount(list, "message_sent")).toBe(1);
    expect(activityCount(list, "voice_join")).toBe(1);
    expect(activityCount(list, "autre")).toBe(0);
  });

  it("activityLinksCount detecte les urls dans le contenu", () => {
    const list = [
      act({ id: "1", content: "regarde https://exemple.fr" }),
      act({ id: "2", content: "rien ici" }),
      act({ id: "3", content: null }),
    ];
    expect(activityLinksCount(list)).toBe(1);
  });

  it("activityAttachmentsCount compte les evenements avec pieces jointes", () => {
    const list = [
      act({ id: "1", metadata: { attachments: ["a.png"] } }),
      act({ id: "2", metadata: { attachments: [] } }),
      act({ id: "3", metadata: {} }),
    ];
    expect(activityAttachmentsCount(list)).toBe(1);
  });

  it("activityLabel traduit puis retombe sur le type", () => {
    expect(activityLabel("message_sent")).toBe("Message");
    expect(activityLabel("voice_move")).toBe("Move vocal");
    expect(activityLabel("custom_evt")).toBe("custom_evt");
  });

  it("activityVariant colore les evenements", () => {
    expect(activityVariant("message_deleted")).toBe("danger");
    expect(activityVariant("message_edited")).toBe("warning");
    expect(activityVariant("voice_leave")).toBe("warning");
    expect(activityVariant("member_join")).toBe("success");
    expect(activityVariant("voice_join")).toBe("success");
    expect(activityVariant("voice_move")).toBe("info");
    expect(activityVariant("message_sent")).toBe("info");
    expect(activityVariant("autre")).toBe("default");
  });
});

describe("helpers metadata", () => {
  it("metaStr prend la premiere valeur non vide", () => {
    expect(metaStr(null, "a")).toBeNull();
    expect(metaStr({}, "a")).toBeNull();
    expect(metaStr({ a: "  " }, "a", "b")).toBeNull();
    expect(metaStr({ a: "  ", b: "v" }, "a", "b")).toBe("v");
    expect(metaStr({ a: 42, b: "v" }, "a", "b")).toBe("v");
    expect(metaStr({ a: "ok" }, "a")).toBe("ok");
  });

  it("metaArr filtre les chaines", () => {
    expect(metaArr(null, "k")).toEqual([]);
    expect(metaArr({}, "k")).toEqual([]);
    expect(metaArr({ k: "x" }, "k")).toEqual([]);
    expect(metaArr({ k: ["a", 1, "b"] }, "k")).toEqual(["a", "b"]);
  });

  it("voiceChannelLabel combine nom et id", () => {
    expect(voiceChannelLabel({ channel_name: "Général", channel_id: "c1", metadata: {} })).toBe("🔊 Général (c1)");
    expect(voiceChannelLabel({ channel_name: "Général", metadata: {} })).toBe("🔊 Général");
    expect(voiceChannelLabel({ channel_id: "c1", metadata: {} })).toBe("🔊 c1");
    expect(voiceChannelLabel({ metadata: { channel_name: "Vocal", channel_id: "c2" } })).toBe("🔊 Vocal (c2)");
    expect(voiceChannelLabel({ metadata: {} })).toBe("");
  });

  it("editedBeforeAfter lit les variantes de metadata", () => {
    expect(editedBeforeAfter({ content: "apres", metadata: { old_content: "avant" } })).toEqual({ before: "avant", after: "apres" });
    expect(editedBeforeAfter({ content: "apres", metadata: { before: "avant" } })).toEqual({ before: "avant", after: "apres" });
    expect(editedBeforeAfter({ content: "apres", metadata: { content_before: "avant" } })).toEqual({ before: "avant", after: "apres" });
    expect(editedBeforeAfter({ content: "apres", metadata: {} })).toEqual({ before: null, after: "apres" });
    expect(editedBeforeAfter({ content: null, metadata: { new_content: "apres" } })).toEqual({ before: null, after: "apres" });
  });

  it("rolesDiff lit added/removed puis roles_added/roles_removed", () => {
    expect(rolesDiff({ metadata: { added: ["a"], removed: ["r"] } })).toEqual({ added: ["a"], removed: ["r"] });
    expect(rolesDiff({ metadata: { roles_added: ["a"], roles_removed: ["r"] } })).toEqual({ added: ["a"], removed: ["r"] });
    expect(rolesDiff({ metadata: {} })).toEqual({ added: [], removed: [] });
  });

  it("profileDiff lit les variantes avant/apres", () => {
    expect(profileDiff({ content: "Nouveau", metadata: { old: "Ancien" } })).toEqual({ before: "Ancien", after: "Nouveau" });
    expect(profileDiff({ content: "Nouveau", metadata: { old_nickname: "Ancien" } })).toEqual({ before: "Ancien", after: "Nouveau" });
    expect(profileDiff({ content: "Nouveau", metadata: {} })).toEqual({ before: null, after: "Nouveau" });
  });

  it("avatarDiff lit les urls avant/apres", () => {
    expect(avatarDiff({ metadata: { old_avatar_url: "o.png", new_avatar_url: "n.png" } })).toEqual({ before: "o.png", after: "n.png" });
    expect(avatarDiff({ metadata: { old_avatar: "o.png", new_avatar: "n.png" } })).toEqual({ before: "o.png", after: "n.png" });
    expect(avatarDiff({ metadata: {} })).toEqual({ before: null, after: null });
  });
});

describe("surveillance enrichie", () => {
  const now = Date.now();
  const iso = (daysAgo: number) => new Date(now - daysAgo * 86_400_000).toISOString();

  it("activityWithin filtre par recence", () => {
    const list = [act({ id: "recent", created_at: iso(1) }), act({ id: "old", created_at: iso(30) })];
    expect(activityWithin(list, 7).map((e) => e.id)).toEqual(["recent"]);
    // days <= 0 : pas de filtre
    expect(activityWithin(list, 0)).toHaveLength(2);
  });

  it("countByCategory compte par categorie", () => {
    const list = [
      act({ id: "1", event_type: "message_sent", created_at: iso(1) }),
      act({ id: "2", event_type: "voice_join", created_at: iso(1) }),
      act({ id: "3", event_type: "member_join", created_at: iso(1) }),
    ];
    expect(countByCategory(list, 7, "text")).toBe(1);
    expect(countByCategory(list, 7, "vocal")).toBe(1);
    expect(countByCategory(list, 7, "other")).toBe(1);
  });

  it("voiceHours somme les durees voice_leave/voice_move", () => {
    const list = [
      act({ id: "1", event_type: "voice_leave", metadata: { duration_secs: 3600 } }),
      act({ id: "2", event_type: "voice_move", metadata: { duration_secs: "1800" } }),
      act({ id: "3", event_type: "voice_join", metadata: { duration_secs: 9999 } }),
    ];
    expect(voiceHours(list, 0)).toBe(1.5);
  });

  it("attachmentCounts distingue images, videos, fichiers et liens", () => {
    const list = [
      act({ id: "1", content: "https://exemple.fr", metadata: { attachments: ["a.png", "b.mp4", "c.pdf"] } }),
      act({ id: "2", metadata: { attachments: [{ content_type: "image/gif" }, { content_type: "video/webm" }, { content_type: "text/plain" }] } }),
      act({ id: "3", metadata: { attachments: ["x.png", "y.jpg", "z.gif", "w.webp", "v.mp4", "u.mov"] } }),
    ];
    expect(attachmentCounts(list)).toEqual({ images: 6, videos: 4, files: 2, links: 1 });
  });

  it("topChannels classe les canaux de messages", () => {
    const list = [
      act({ id: "1", channel_id: "c1", channel_name: "general" }),
      act({ id: "2", channel_id: "c1", channel_name: "general" }),
      act({ id: "3", channel_id: "c2", channel_name: "dev" }),
      act({ id: "4", event_type: "voice_join", channel_id: "v1" }),
      act({ id: "5", channel_id: null }),
    ];
    expect(topChannels(list, 2)).toEqual([
      { name: "general", id: "c1", count: 2 },
      { name: "dev", id: "c2", count: 1 },
    ]);
  });

  it("topVoiceCompanions agrège les compagnons (objets et chaines)", () => {
    const list = [
      act({ id: "1", event_type: "voice_join", metadata: { companions: [{ user_id: "u2", username: "Bob" }, { user_id: "u3" }] } }),
      act({ id: "2", event_type: "voice_move", metadata: { companions: [{ user_id: "u2", username: "Bob" }, "u4", null, 42] } }),
      act({ id: "3", event_type: "voice_join", metadata: { companions: "pas-un-tableau" } }),
    ];
    const top = topVoiceCompanions(list, 3);
    expect(top).toEqual([
      { user_id: "u2", username: "Bob", count: 2 },
      { user_id: "u3", username: "u3", count: 1 },
      { user_id: "u4", username: "u4", count: 1 },
    ]);
  });

  it("heatmapData construit la grille jour x heure", () => {
    const monday9 = new Date(2026, 7, 17, 9, 30).toISOString(); // lundi 9h
    const monday10 = new Date(2026, 7, 17, 10, 0).toISOString();
    const sunday22 = new Date(2026, 7, 16, 22, 0).toISOString(); // dimanche 22h
    const list = [act({ id: "1", created_at: monday9 }), act({ id: "2", created_at: monday10 }), act({ id: "3", created_at: sunday22 })];
    const h = heatmapData(list);
    expect(h.days).toEqual(["Lun", "Mar", "Mer", "Jeu", "Ven", "Sam", "Dim"]);
    expect(h.grid[0][9]).toBe(1);
    expect(h.grid[0][10]).toBe(1);
    expect(h.grid[6][22]).toBe(1);
    expect(h.max).toBe(1);
  });

  it("heatColor retourne des intensites proportionnelles", () => {
    expect(heatColor(0, 10)).toBe("rgba(88, 101, 242, 0.05)");
    expect(heatColor(5, 10)).toBe("rgba(88, 101, 242, 0.5)");
    expect(heatColor(10, 10)).toBe("rgba(88, 101, 242, 0.9)");
    expect(heatColor(99, 10)).toBe("rgba(88, 101, 242, 0.9)");
    expect(heatColor(1, 0)).toBe("rgba(88, 101, 242, 0.9)");
  });

  it("burstCount detecte les rafales de 10 messages en 1 minute", () => {
    const base = Date.now() - 86_400_000;
    const burst = Array.from({ length: 12 }, (_, i) =>
      act({ id: `b${i}`, event_type: "message_sent", created_at: new Date(base + i * 4000).toISOString() }),
    );
    expect(burstCount(burst)).toBe(1);
    expect(burstCount(burst.slice(0, 9))).toBe(0);
    expect(burstCount([])).toBe(0);
  });

  it("automodCount compte les evenements securite automod", () => {
    const dossier: UserDossier = {
      user: {
        user_id: "u1", username: "m", guild_id: "g1", guild_name: "serveur",
        risk_level: "high", total_warns: 0, total_mutes: 0, total_bans: 0,
        last_incident_at: null, security_events_count: 3, first_seen_at: "2026-01-01T00:00:00Z",
      },
      infractions: [],
      moderation_actions: [],
      security_events: [
        { id: "1", guild_id: "g1", event_type: "automod_spam", severity: "high", description: "d", user_ids: ["u1"], created_at: "2026-01-02T00:00:00Z" },
        { id: "2", guild_id: "g1", event_type: "AUTOMOD_PHISHING", severity: "high", description: "d", user_ids: ["u1"], created_at: "2026-01-03T00:00:00Z" },
        { id: "3", guild_id: "g1", event_type: "raid", severity: "high", description: "d", user_ids: ["u1"], created_at: "2026-01-04T00:00:00Z" },
      ],
    };
    expect(automodCount(dossier)).toBe(2);
    expect(automodCount(null)).toBe(0);
  });

  it("watchSplitStats compare infractions avant/apres premier signalement", () => {
    const user = {
      user_id: "u1", username: "m", guild_id: "g1", guild_name: "serveur",
      risk_level: "high", total_warns: 0, total_mutes: 0, total_bans: 0,
      last_incident_at: null, security_events_count: 0, first_seen_at: "2026-06-01T00:00:00Z",
    };
    const dossier: UserDossier = {
      user,
      infractions: [
        { id: "1", user_id: "u1", username: "m", server: "s", infraction_type: "warn", reason: "r", created_at: "2026-05-01T00:00:00Z", moderator: "mod" },
        { id: "2", user_id: "u1", username: "m", server: "s", infraction_type: "warn", reason: "r", created_at: "2026-07-01T00:00:00Z", moderator: "mod" },
      ],
      moderation_actions: [],
      security_events: [],
    };
    expect(watchSplitStats(dossier)).toEqual({
      sinceTs: new Date("2026-06-01T00:00:00Z").getTime(),
      beforeIncidents: 1,
      afterIncidents: 1,
    });
    expect(watchSplitStats(null)).toBeNull();
    // first_seen_at manquant : la fonction retombe sur null.
    const noFirstSeen = { ...(dossier as object), user: { ...(user as object), first_seen_at: "" } } as unknown as UserDossier;
    expect(watchSplitStats(noFirstSeen)).toBeNull();
  });
});
