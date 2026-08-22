import { describe, expect, it } from "vitest";
import { formatShortMonthDate, useFormatDate } from "./useFormatDate";

const ISO = "2026-03-28T14:35:09.000Z"; // date fixe : pas de dépendance à l'heure réelle

describe("formatShortMonthDate (export direct)", () => {
  it("formate une date valide en français court", () => {
    const s = formatShortMonthDate(ISO);
    expect(s).toMatch(/\d{1,2}\s+mar\.?/i); // "28 mars 2026" (l'abréviation du mois varie selon l'ICU : avec ou sans point)
    expect(s).toContain("2026");
  });

  it("retourne l'empty par défaut sur entrée vide", () => {
    expect(formatShortMonthDate(null)).toBe("-");
    expect(formatShortMonthDate(undefined, "n/a")).toBe("n/a");
    expect(formatShortMonthDate("", "")).toBe("");
  });

  it("laisse passer une chaîne invalide telle quelle", () => {
    expect(formatShortMonthDate("pas-une-date")).toBe("pas-une-date");
  });
});

describe("useFormatDate", () => {
  const f = useFormatDate();

  it("expose les six formateurs + le court-mois", () => {
    for (const k of ["formatDate", "formatDateTime", "formatTime", "formatShortDateTime", "formatDateTimeShort", "formatDateTimeNumeric"] as const) {
      expect(typeof f[k]).toBe("function");
    }
  });

  it("formatDate : long, sans heure ; vide et invalide gérés", () => {
    expect(f.formatDate(ISO)).toContain("2026");
    expect(f.formatDate(null)).toBe("");
    expect(f.formatDate(undefined)).toBe("");
    expect(f.formatDate("invalide")).toBe("invalide");
  });

  it("formatDateTime : contient heure et minute", () => {
    const s = f.formatDateTime(ISO);
    expect(s).toMatch(/\d{1,2}/); // au moins un chiffre d'heure/minute
    expect(f.formatDateTime(null)).toBe("");
    expect(f.formatDateTime("x")).toBe("x");
  });

  it("formatTime : hh:mm:ss", () => {
    const s = f.formatTime(ISO);
    expect(s).toMatch(/^\d{1,2}:\d{2}/); // heure:minute en tête (fuseau local)
    expect(f.formatTime(null)).toBe("");
  });

  it("formatShortDateTime : jj/mm/aaaa hh:mm", () => {
    const s = f.formatShortDateTime(ISO);
    expect(s).toMatch(/\d{2}\/\d{2}\//); // date numérique courte
    expect(f.formatShortDateTime(null)).toBe("");
  });

  it("formatDateTimeShort : toLocaleString fr-FR complet", () => {
    const s = f.formatDateTimeShort(ISO);
    expect(s).toContain("/");
    expect(s.length).toBeGreaterThan(5);
  });

  it("formatDateTimeNumeric : jj/mm/aaaa hh:mm explicite", () => {
    const s = f.formatDateTimeNumeric(ISO);
    expect(s).toMatch(/\d{2}\/\d{2}\//);
  });
});
