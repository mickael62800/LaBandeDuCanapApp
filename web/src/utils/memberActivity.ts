/**
 * Fonctions pures de calcul/agregation pour le drawer de detail membre.
 * Extraites de MemberDetailDrawer.vue — comportement byte-identique.
 * Toutes prennent la donnee en argument (aucun acces a un state reactif).
 */
import type { UserActivity, UserDossier } from "../types";
export { formatShortMonthDate as formatMemberDate } from "../composables/useFormatDate";

export type Meta = Record<string, unknown> | null | undefined;

export const URL_RE = /https?:\/\/[^\s]+/i;
export const TEXT_EVENTS = ["message_sent", "message_edited", "message_deleted"];
export const VOCAL_EVENTS = ["voice_join", "voice_leave", "voice_move"];
const ONE_DAY = 86_400_000;

// ── Formatage ────────────────
export function formatDuration(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

export function rolesCount(roles: string[]): number {
  return roles.length;
}

// ── Categorisation des evenements ────────────────
export function eventCategory(t: string): "text" | "vocal" | "other" {
  if (TEXT_EVENTS.includes(t)) return "text";
  if (VOCAL_EVENTS.includes(t)) return "vocal";
  return "other";
}

export function activityCount(list: UserActivity[], type: string): number {
  return list.filter((e) => e.event_type === type).length;
}

export function activityLinksCount(list: UserActivity[]): number {
  return list.filter((e) => typeof e.content === "string" && URL_RE.test(e.content)).length;
}

export function activityAttachmentsCount(list: UserActivity[]): number {
  return list.filter((e) => {
    const m = e.metadata as Record<string, unknown> | null | undefined;
    const att = m?.attachments;
    return Array.isArray(att) && att.length > 0;
  }).length;
}

export function activityLabel(t: string): string {
  return ({
    message_sent: "Message",
    message_edited: "Edite",
    message_deleted: "Supprime",
    voice_join: "Entree vocal",
    voice_leave: "Sortie vocal",
    voice_move: "Move vocal",
    roles_changed: "Roles",
    nickname_changed: "Pseudo",
    avatar_changed: "Avatar",
    member_join: "Arrivee",
    member_leave: "Depart",
  } as Record<string, string>)[t] ?? t;
}

export function activityVariant(t: string): "default" | "warning" | "danger" | "info" | "success" {
  if (t === "message_deleted") return "danger";
  if (t === "message_edited" || t === "voice_leave" || t === "member_leave") return "warning";
  if (t === "member_join" || t === "voice_join") return "success";
  if (t.startsWith("voice_") || t === "message_sent") return "info";
  return "default";
}

// ── Helpers metadata ────────────────
export function metaStr(m: Meta, ...keys: string[]): string | null {
  if (!m) return null;
  for (const k of keys) {
    const v = m[k];
    if (typeof v === "string" && v.trim() !== "") return v;
  }
  return null;
}

export function metaArr(m: Meta, key: string): string[] {
  if (!m) return [];
  const v = m[key];
  return Array.isArray(v) ? v.filter((x): x is string => typeof x === "string") : [];
}

export function voiceChannelLabel(evt: { channel_name?: string | null; channel_id?: string | null; metadata: Meta }): string {
  const name = evt.channel_name || metaStr(evt.metadata, "channel_name", "voice_channel_name");
  const id = evt.channel_id || metaStr(evt.metadata, "channel_id", "voice_channel_id");
  if (name && id) return `🔊 ${name} (${id})`;
  if (name) return `🔊 ${name}`;
  if (id) return `🔊 ${id}`;
  return "";
}

export function editedBeforeAfter(evt: { content: string | null; metadata: Meta }): { before: string | null; after: string | null } {
  const before = metaStr(evt.metadata, "old_content", "before", "content_before", "previous_content");
  const after = metaStr(evt.metadata, "new_content", "after", "content_after") || evt.content;
  return { before, after };
}

export function rolesDiff(evt: { metadata: Meta }): { added: string[]; removed: string[] } {
  const added = metaArr(evt.metadata, "added").length > 0
    ? metaArr(evt.metadata, "added")
    : metaArr(evt.metadata, "roles_added");
  const removed = metaArr(evt.metadata, "removed").length > 0
    ? metaArr(evt.metadata, "removed")
    : metaArr(evt.metadata, "roles_removed");
  return { added, removed };
}

export function profileDiff(evt: { content: string | null; metadata: Meta }): { before: string | null; after: string | null } {
  const before = metaStr(evt.metadata, "old", "old_value", "before", "from", "old_nickname", "old_username");
  const after =
    metaStr(evt.metadata, "new", "new_value", "after", "to", "new_nickname", "new_username") || evt.content;
  return { before, after };
}

export function avatarDiff(evt: { metadata: Meta }): { before: string | null; after: string | null } {
  return {
    before: metaStr(evt.metadata, "old_avatar_url", "old_avatar", "before"),
    after: metaStr(evt.metadata, "new_avatar_url", "new_avatar", "after"),
  };
}

// ── Surveillance enrichie ────────────────
export function activityWithin(list: UserActivity[], days: number): UserActivity[] {
  if (days <= 0) return list;
  const since = Date.now() - days * ONE_DAY;
  return list.filter((e) => new Date(e.created_at).getTime() >= since);
}

export function countByCategory(list: UserActivity[], days: number, cat: "text" | "vocal" | "other"): number {
  return activityWithin(list, days).filter((e) => eventCategory(e.event_type) === cat).length;
}

export function voiceHours(list: UserActivity[], days: number): number {
  let total = 0;
  for (const e of activityWithin(list, days)) {
    if (e.event_type !== "voice_leave" && e.event_type !== "voice_move") continue;
    const m = e.metadata as Meta;
    const d = m?.duration_secs;
    if (typeof d === "number") total += d;
    else if (typeof d === "string") total += parseInt(d, 10) || 0;
  }
  return Math.round((total / 3600) * 10) / 10;
}

export function attachmentCounts(list: UserActivity[]): { images: number; videos: number; files: number; links: number } {
  let images = 0, videos = 0, files = 0, links = 0;
  for (const e of list) {
    if (typeof e.content === "string" && URL_RE.test(e.content)) links++;
    const m = e.metadata as Meta;
    const att = m?.attachments;
    if (Array.isArray(att)) {
      for (const a of att) {
        if (typeof a === "string") {
          if (/\.(png|jpg|jpeg|gif|webp)/i.test(a)) images++;
          else if (/\.(mp4|webm|mov)/i.test(a)) videos++;
          else files++;
        } else if (a && typeof a === "object") {
          const ct = (a as Record<string, unknown>).content_type as string | undefined;
          if (ct?.startsWith("image/")) images++;
          else if (ct?.startsWith("video/")) videos++;
          else files++;
        }
      }
    }
  }
  return { images, videos, files, links };
}

export function topChannels(list: UserActivity[], limit = 5): Array<{ name: string; id: string; count: number }> {
  const counts = new Map<string, { name: string; id: string; count: number }>();
  for (const e of list) {
    if (!TEXT_EVENTS.includes(e.event_type)) continue;
    const id = e.channel_id ?? "";
    const name = e.channel_name ?? id ?? "?";
    if (!id) continue;
    const cur = counts.get(id) ?? { name, id, count: 0 };
    cur.count++;
    counts.set(id, cur);
  }
  return [...counts.values()].sort((a, b) => b.count - a.count).slice(0, limit);
}

export function topVoiceCompanions(list: UserActivity[], limit = 5): Array<{ user_id: string; username: string; count: number }> {
  const counts = new Map<string, { user_id: string; username: string; count: number }>();
  for (const e of list) {
    if (!VOCAL_EVENTS.includes(e.event_type)) continue;
    const m = e.metadata as Meta;
    const companions = m?.companions;
    if (Array.isArray(companions)) {
      for (const c of companions) {
        if (c && typeof c === "object") {
          const cid = (c as Record<string, unknown>).user_id as string | undefined;
          const cname = (c as Record<string, unknown>).username as string | undefined;
          if (!cid) continue;
          const cur = counts.get(cid) ?? { user_id: cid, username: cname ?? cid, count: 0 };
          cur.count++;
          counts.set(cid, cur);
        } else if (typeof c === "string") {
          const cur = counts.get(c) ?? { user_id: c, username: c, count: 0 };
          cur.count++;
          counts.set(c, cur);
        }
      }
    }
  }
  return [...counts.values()].sort((a, b) => b.count - a.count).slice(0, limit);
}

export function heatmapData(list: UserActivity[]): { days: string[]; grid: number[][]; max: number } {
  const days = ["Lun", "Mar", "Mer", "Jeu", "Ven", "Sam", "Dim"];
  const grid: number[][] = Array.from({ length: 7 }, () => Array(24).fill(0));
  let max = 0;
  for (const e of list) {
    if (!TEXT_EVENTS.includes(e.event_type)) continue;
    const d = new Date(e.created_at);
    const dow = (d.getDay() + 6) % 7;
    const h = d.getHours();
    grid[dow][h]++;
    if (grid[dow][h] > max) max = grid[dow][h];
  }
  return { days, grid, max };
}

export function heatColor(value: number, max: number): string {
  if (value === 0) return "rgba(88, 101, 242, 0.05)";
  const intensity = Math.min(value / Math.max(1, max), 1);
  return `rgba(88, 101, 242, ${0.1 + intensity * 0.8})`;
}

export function burstCount(list: UserActivity[]): number {
  const ts = list
    .filter((e) => e.event_type === "message_sent")
    .map((e) => new Date(e.created_at).getTime())
    .sort((a, b) => a - b);
  if (ts.length < 10) return 0;
  let bursts = 0;
  for (let i = 0; i + 9 < ts.length; i++) {
    if (ts[i + 9] - ts[i] <= 60_000) {
      bursts++;
      i += 9;
    }
  }
  return bursts;
}

export function automodCount(dossier: UserDossier | null): number {
  if (!dossier) return 0;
  return dossier.security_events.filter(
    (e) => typeof e.event_type === "string" && e.event_type.toLowerCase().includes("automod"),
  ).length;
}

export function watchSplitStats(dossier: UserDossier | null): { sinceTs: number; beforeIncidents: number; afterIncidents: number } | null {
  if (!dossier) return null;
  const since = dossier.user.first_seen_at;
  if (!since) return null;
  const sinceTs = new Date(since).getTime();
  let beforeIncidents = 0, afterIncidents = 0;
  for (const inf of dossier.infractions) {
    const ts = new Date(inf.created_at).getTime();
    if (ts < sinceTs) beforeIncidents++;
    else afterIncidents++;
  }
  return { sinceTs, beforeIncidents, afterIncidents };
}
