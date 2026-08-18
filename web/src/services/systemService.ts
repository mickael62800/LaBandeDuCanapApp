import { httpGet, httpPost } from "@/api/http";

export interface ServiceStatus {
  name: string;
  online: boolean;
}

export interface HostMetrics {
  cpu_percent: number;
  cpu_cores: number;
  mem_used_mb: number;
  mem_total_mb: number;
  /**
   * Débit réseau instantané de l'hôte, en octets par seconde.
   *
   * Un débit, pas un compteur : les octets cumulés depuis le démarrage ne
   * disent rien à qui les lit. Les interfaces virtuelles (boucle locale,
   * ponts Docker) sont exclues — elles compteraient deux fois le même paquet.
   */
  net_rx_bytes_per_sec: number;
  net_tx_bytes_per_sec: number;
  /** Joignabilité des services dont la plateforme dépend. */
  internet: InternetProbe[];
}

/**
 * Résultat d'une sonde vers l'extérieur.
 *
 * Une connexion TCP, pas un ping ICMP : beaucoup de réseaux filtrent l'ICMP,
 * si bien qu'un ping perdu ne prouverait rien. On ouvre le port réellement
 * utilisé.
 */
export interface InternetProbe {
  label: string;
  target: string;
  reachable: boolean;
  /** `null` si injoignable : 0 laisserait croire à une latence parfaite. */
  latency_ms: number | null;
}

export interface ProcessMetrics {
  cpu_percent: number;
  mem_used_mb: number;
}

export interface RedisMetrics {
  used_memory_mb: number;
  connected_clients: number;
  total_keys: number;
  uptime_seconds: number;
}

export interface DiskInfo {
  name: string;
  mount_point: string;
  fs_type: string;
  total_gb: number;
  used_gb: number;
  available_gb: number;
  usage_percent: number;
  is_removable: boolean;
}

export interface HealthChecks {
  api_responding: boolean;
  postgres_responding: boolean;
  redis_responding: boolean;
}

export interface SystemInfo {
  bots: ServiceStatus[];
  workers: ServiceStatus[];
  host: HostMetrics;
  process: ProcessMetrics;
  redis: RedisMetrics;
  disks: DiskInfo[];
  health: HealthChecks;
  uptime_seconds: number;
  db_size_mb: number;
}

export interface GuildResetResult {
  tables_wiped: number;
  total_rows: number;
}

export const systemService = {
  getInfo(): Promise<SystemInfo> {
    return httpGet("/api/system/info");
  },

  /**
   * ⚠️ DANGER — Factory reset d'un serveur. IRREVERSIBLE.
   * Efface toutes les donnees du serveur + demande au bot d'annuler l'etat
   * Discord (deban / unmute / retrait des roles temp+quarantaine).
   * Reserve a l'owner ; `confirmation` doit etre le nom EXACT du serveur.
   */
  resetGuild(
    guildId: string,
    confirmation: string,
    options: { unban?: boolean; unmute?: boolean; remove_roles?: boolean } = {},
  ): Promise<GuildResetResult> {
    return httpPost(`/api/system/guild-reset/${guildId}`, {
      confirmation,
      unban: options.unban ?? true,
      unmute: options.unmute ?? true,
      remove_roles: options.remove_roles ?? true,
    });
  },
};
