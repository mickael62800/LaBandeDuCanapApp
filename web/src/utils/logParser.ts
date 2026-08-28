/**
 * Utilitaire de parsing et traitement des logs de serveurs de jeux.
 *
 * Gère :
 * - Le nettoyage et la conversion des codes couleur ANSI en HTML sécurisé (anti-XSS).
 * - La détection automatique des niveaux de sévérité (ERROR, WARN, SUCCESS, INFO, DEBUG).
 * - La détection des catégories métier (Joueurs, Sauvegardes, Réseau, Général).
 * - L'extraction propre des horodatages (ISO, brackets, etc.).
 * - La surbrillance sécurisée des termes de recherche.
 */

export type LogLevel = "error" | "warn" | "success" | "info" | "debug";
export type LogCategory = "player" | "save" | "network" | "general";

export interface ParsedLogLine {
  id: number;
  raw: string;
  level: LogLevel;
  category: LogCategory;
  timestamp: string | null;
  message: string;
  html: string;
}

/** Échappe les caractères HTML spéciaux pour éviter toute injection XSS */
export function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

/** Supprime les codes d'échappement ANSI d'une chaîne */
export function stripAnsi(text: string): string {
  // eslint-disable-next-line no-control-regex
  return text.replace(/\u001b\[[0-9;]*[a-zA-Z]/g, "");
}

/** Table de correspondance des couleurs ANSI standard vers des classes CSS */
const ANSI_CLASSES: Record<number, string> = {
  1: "ansi-bold",
  2: "ansi-dim",
  3: "ansi-italic",
  4: "ansi-underline",
  30: "ansi-black",
  31: "ansi-red",
  32: "ansi-green",
  33: "ansi-yellow",
  34: "ansi-blue",
  35: "ansi-magenta",
  36: "ansi-cyan",
  37: "ansi-white",
  90: "ansi-bright-black",
  91: "ansi-bright-red",
  92: "ansi-bright-green",
  93: "ansi-bright-yellow",
  94: "ansi-bright-blue",
  95: "ansi-bright-magenta",
  96: "ansi-bright-cyan",
  97: "ansi-bright-white",
};

/**
 * Convertit une ligne avec codes ANSI en HTML sécurisé enrichi de balises <span>
 */
export function ansiToHtml(text: string): string {
  // eslint-disable-next-line no-control-regex
  const ansiRegex = /\u001b\[([0-9;]*)m/g;
  let result = "";
  let lastIndex = 0;
  const activeClasses = new Set<string>();

  let match: RegExpExecArray | null;
  while ((match = ansiRegex.exec(text)) !== null) {
    const chunk = text.slice(lastIndex, match.index);
    if (chunk) {
      const escaped = escapeHtml(chunk);
      if (activeClasses.size > 0) {
        result += `<span class="${Array.from(activeClasses).join(" ")}">${escaped}</span>`;
      } else {
        result += escaped;
      }
    }

    const codeStr = match[1] ?? "";
    const codes = codeStr ? codeStr.split(";").map(Number) : [0];

    for (const code of codes) {
      if (code === 0) {
        activeClasses.clear();
      } else if (ANSI_CLASSES[code]) {
        activeClasses.add(ANSI_CLASSES[code]!);
      }
    }

    lastIndex = ansiRegex.lastIndex;
  }

  const remaining = text.slice(lastIndex);
  if (remaining) {
    const escaped = escapeHtml(remaining);
    if (activeClasses.size > 0) {
      result += `<span class="${Array.from(activeClasses).join(" ")}">${escaped}</span>`;
    } else {
      result += escaped;
    }
  }

  return result;
}

/**
 * Détecte le niveau de sévérité à partir du contenu brut de la ligne
 */
export function detectLogLevel(line: string): LogLevel {
  const clean = stripAnsi(line);

  // Erreurs critiques et exceptions
  if (
    /\b(error|fatal|panic|severe|critical|crash|crashed|failed|failure|stacktrace|errno|sigsegv|sigterm)\b/i.test(clean) ||
    /exception/i.test(clean) ||
    /\[(error|fatal|severe|err)\]/i.test(clean) ||
    /^(?:.*[:]\s*)?(?:error|fatal)[:]/i.test(clean)
  ) {
    return "error";
  }

  // Avertissements
  if (
    /\b(warn|warning|deprecated)\b/i.test(clean) ||
    /\[(warn|warning)\]/i.test(clean)
  ) {
    return "warn";
  }

  // Succès / Initialisations réussies / Connexions
  if (
    /\b(success|successfully|ready|online|listening on|listening at|server started|game started|done \(|fully initialized)\b/i.test(clean) ||
    /\[(success|ready)\]/i.test(clean)
  ) {
    return "success";
  }

  // Debug / Traces
  if (
    /\b(debug|trace|verbose)\b/i.test(clean) ||
    /\[(debug|trace)\]/i.test(clean)
  ) {
    return "debug";
  }

  return "info";
}

/**
 * Détecte la catégorie métier de l'événement de log
 */
export function detectLogCategory(line: string): LogCategory {
  const clean = stripAnsi(line);

  if (
    /\b(player|user|joined|left|disconnect|disconnected|connecting|connected|logged in|kicked|banned|chat|death|killed|respawn|whisper)\b/i.test(clean) ||
    /^<[a-zA-Z0-9_-]+>\s+/i.test(clean)
  ) {
    return "player";
  }

  if (
    /\b(save|saving|saved|backup|autosave|world|chunk|snapshot|writing to disk)\b/i.test(clean)
  ) {
    return "save";
  }

  if (
    /\b(port|bind|tcp|udp|steam|socket|rcon|http|query|network|ip|listening|packet)\b/i.test(clean)
  ) {
    return "network";
  }

  return "general";
}

/**
 * Extrait l'horodatage en tête de ligne s'il existe
 */
export function extractTimestamp(line: string): { timestamp: string | null; message: string } {
  const clean = stripAnsi(line);

  // Format ISO : 2026-08-28T21:15:30.123Z ou 2026-08-28 21:15:30
  const isoMatch = clean.match(/^(\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})?)\s*(?:[|:-]|\s)\s*(.*)$/);
  if (isoMatch && isoMatch[1]) {
    return {
      timestamp: isoMatch[1],
      message: isoMatch[2] || clean,
    };
  }

  // Format Brackets : [21:15:30] ou [08/28/26 21:15:30] ou (21:15:30)
  const bracketMatch = clean.match(/^[[(](\d{1,4}[-/:]\d{1,2}[-/:]\d{1,4}\s+)?(\d{2}:\d{2}:\d{2}(?:\.\d+)?)[\])]\s*(.*)$/);
  if (bracketMatch && bracketMatch[2]) {
    const fullTs = bracketMatch[1] ? `${bracketMatch[1].trim()} ${bracketMatch[2]}` : bracketMatch[2];
    return {
      timestamp: fullTs,
      message: bracketMatch[3] || clean,
    };
  }

  return {
    timestamp: null,
    message: clean,
  };
}

/**
 * Applique la surbrillance d'un terme de recherche dans un fragment HTML déjà échappé
 */
export function highlightSearchInHtml(html: string, search: string): string {
  if (!search.trim()) return html;
  const escapedSearch = escapeHtml(search.trim());
  const regex = new RegExp(`(${escapedSearch.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")})`, "gi");
  return html.replace(regex, '<mark class="log-match">$1</mark>');
}

/**
 * Parse une liste de lignes de logs brutes en structures enrichies
 */
export function parseLogLines(rawLines: string[], search = ""): ParsedLogLine[] {
  return rawLines.map((raw, index) => {
    const level = detectLogLevel(raw);
    const category = detectLogCategory(raw);
    const { timestamp, message } = extractTimestamp(raw);
    let html = ansiToHtml(raw);
    if (search.trim()) {
      html = highlightSearchInHtml(html, search);
    }

    return {
      id: index + 1,
      raw,
      level,
      category,
      timestamp,
      message,
      html,
    };
  });
}
