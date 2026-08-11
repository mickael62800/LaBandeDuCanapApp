import type { PublicEvent } from "@/services/publicEventsService";

const DAY: Intl.DateTimeFormatOptions = { weekday: "short", day: "numeric", month: "short" };
const TIME: Intl.DateTimeFormatOptions = { hour: "2-digit", minute: "2-digit" };
const MEMBER_COLORS = ["#a855f7", "#22c55e", "#f39c12", "#c026d3", "#38bdf8", "#f43f5e", "#14b8a6"];

export function formatEventRange(event: PublicEvent): string {
  const start = new Date(event.starts_at);
  const end = new Date(event.ends_at);
  if (event.span_days > 1) {
    return `${start.toLocaleDateString("fr-FR", DAY)} → ${end.toLocaleDateString("fr-FR", DAY)}`;
  }
  if (event.all_day) return start.toLocaleDateString("fr-FR", DAY);
  return `${start.toLocaleDateString("fr-FR", DAY)} · ${start.toLocaleTimeString("fr-FR", TIME)}`;
}

export const formatTime = (iso: string) => new Date(iso).toLocaleTimeString("fr-FR", TIME);
export const formatDay = (iso: string) => new Date(iso).toLocaleDateString("fr-FR", {
  day: "numeric",
  month: "long",
});

export function relativeTime(iso: string): string {
  const minutes = Math.floor((Date.now() - new Date(iso).getTime()) / 60_000);
  if (minutes < 1) return "à l'instant";
  if (minutes < 60) return `il y a ${minutes} min`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `il y a ${hours} h`;
  const days = Math.floor(hours / 24);
  return days === 1 ? "hier" : `il y a ${days} jours`;
}

export const eventAccent = (event: PublicEvent) => event.color ? `#${event.color}` : undefined;
export const memberInitial = (name: string) => (name.trim()[0] ?? "?").toUpperCase();
export const publicAvatarUrl = (value: string | null) => value?.startsWith("http") ? value : null;
export const anniversaryLabel = (years: number) => years === 1 ? "1 an" : `${years} ans`;

export function memberColor(name: string): string {
  let sum = 0;
  for (const character of name) sum += character.codePointAt(0) ?? 0;
  return MEMBER_COLORS[sum % MEMBER_COLORS.length];
}
