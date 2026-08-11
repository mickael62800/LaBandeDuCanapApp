/**
 * Formate les dates en français : "28 mars 2026 à 14h35"
 * Utilisable partout dans l'application.
 */
export function formatShortMonthDate(
  raw: string | undefined | null,
  empty = "-",
): string {
  if (!raw) return empty;
  const date = new Date(raw);
  if (isNaN(date.getTime())) return raw;
  return date.toLocaleDateString("fr-FR", {
    day: "numeric",
    month: "short",
    year: "numeric",
  });
}

export function useFormatDate() {
  function formatDate(raw: string | undefined | null): string {
    if (!raw) return "";
    const d = new Date(raw);
    if (isNaN(d.getTime())) return raw;
    return d.toLocaleDateString("fr-FR", {
      day: "numeric",
      month: "long",
      year: "numeric",
    });
  }

  function formatDateTime(raw: string | undefined | null): string {
    if (!raw) return "";
    const d = new Date(raw);
    if (isNaN(d.getTime())) return raw;
    return d.toLocaleDateString("fr-FR", {
      day: "numeric",
      month: "long",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  function formatTime(raw: string | undefined | null): string {
    if (!raw) return "";
    const d = new Date(raw);
    if (isNaN(d.getTime())) return raw;
    return d.toLocaleTimeString("fr-FR", {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  }

  function formatShortDateTime(raw: string | undefined | null): string {
    if (!raw) return "";
    const d = new Date(raw);
    if (isNaN(d.getTime())) return raw;
    return d.toLocaleDateString("fr-FR", {
      day: "2-digit",
      month: "2-digit",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  // Reproduit exactement `new Date(x).toLocaleString("fr-FR")` (variante la
  // plus réinventée localement). Pas de garde : comportement byte-identique
  // aux copies locales pour toute entrée valide.
  function formatDateTimeShort(raw: string): string {
    return new Date(raw).toLocaleString("fr-FR");
  }

  // Reproduit `toLocaleString("fr-FR", { jj/mm/aaaa hh:mm })`.
  function formatDateTimeNumeric(raw: string): string {
    return new Date(raw).toLocaleString("fr-FR", {
      day: "2-digit",
      month: "2-digit",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  return {
    formatDate,
    formatDateTime,
    formatTime,
    formatShortDateTime,
    formatDateTimeShort,
    formatDateTimeNumeric,
    formatShortMonthDate,
  };
}
