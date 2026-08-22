import { describe, expect, it } from "vitest";

import type { PublicEvent } from "@/services/publicEventsService";

import {
  anniversaryLabel,
  eventAccent,
  formatDay,
  formatEventRange,
  formatTime,
  memberColor,
  memberInitial,
  publicAvatarUrl,
  relativeTime,
} from "./publicCommunityFormat";

const ev = (over: Partial<PublicEvent> = {}): PublicEvent => ({
  id: "e1",
  title: "Soiree",
  description: null,
  game: null,
  color: null,
  starts_at: "2026-08-22T20:00:00Z",
  ends_at: "2026-08-23T01:00:00Z",
  all_day: false,
  span_days: 1,
  ...over,
});

describe("formatEventRange", () => {
  it("affiche une plage de dates quand l'event dure plusieurs jours", () => {
    const s = formatEventRange(ev({ span_days: 3 }));
    expect(s).toContain("\u2192"); // flèche entre debut et fin
  });

  it("affiche seulement la date pour un event toute la journee", () => {
    const s = formatEventRange(ev({ all_day: true }));
    expect(s).not.toContain("\u00b7"); // pas d'heure
    expect(s).toContain("ao\u00fbt");
  });

  it("affiche date + heure pour un event ponctuel", () => {
    const s = formatEventRange(ev());
    expect(s).toContain("\u00b7"); // separateur avant l'heure
  });
});

describe("formatTime / formatDay", () => {
  it("formate une heure et un jour en fr-FR", () => {
    expect(formatTime("2026-08-22T14:35:00Z")).toMatch(/\d{2}:\d{2}/);
    expect(formatDay("2026-08-22T00:00:00Z")).toContain("ao\u00fbt");
  });
});

describe("relativeTime", () => {
  it("couvre les paliers instant / min / h / hier / jours", () => {
    const now = Date.now();
    expect(relativeTime(new Date(now).toISOString())).toBe("\u00e0 l'instant");
    expect(relativeTime(new Date(now - 5 * 60_000).toISOString())).toContain("min");
    expect(relativeTime(new Date(now - 3 * 3_600_000).toISOString())).toContain("h");
    // ~24 h -> hier (juste sous le palier des jours multiples)
    expect(relativeTime(new Date(now - 25 * 3_600_000).toISOString())).toBe("hier");
    expect(relativeTime(new Date(now - 7 * 86_400_000).toISOString())).toContain("jours");
  });
});

describe("helpers membres / event", () => {
  it("eventAccent : # + couleur, ou undefined sans", () => {
    expect(eventAccent(ev({ color: "a855f7" }))).toBe("#a855f7");
    expect(eventAccent(ev())).toBeUndefined();
  });

  it("memberInitial : premiere lettre majuscule, ? si vide", () => {
    expect(memberInitial("alice")).toBe("A");
    expect(memberInitial("  bob  ")).toBe("B");
    expect(memberInitial("")).toBe("?");
  });

  it("publicAvatarUrl : garde les http, null sinon", () => {
    expect(publicAvatarUrl("https://x/a.png")).toBe("https://x/a.png");
    expect(publicAvatarUrl("/local/a.png")).toBeNull();
    expect(publicAvatarUrl(null)).toBeNull();
  });

  it("anniversaryLabel : singulier au pluriel", () => {
    expect(anniversaryLabel(1)).toBe("1 an");
    expect(anniversaryLabel(3)).toBe("3 ans");
  });

  it("memberColor : deterministe et dans la palette", () => {
    const c = memberColor("micka");
    expect(c).toMatch(/^#[0-9a-f]{6}$/);
    // meme nom -> meme couleur (fonction pure, pas de hasard)
    expect(memberColor("micka")).toBe(c);
  });
});
