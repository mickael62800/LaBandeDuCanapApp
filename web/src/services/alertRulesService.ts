// Les règles d'alerte pilotent la supervision de la MACHINE : elles sont
// servies par `ops-api`, pas par sentinel-api. L'URL de la page
// (/alert-rules) ne change pas — seul le backend appelé change.
import { opsGet, opsPatch } from "@/api/opsHttp";

export interface AlertRule {
  id: string;
  label: string;
  metric: string;
  comparator: string; // 'gt' | 'lt'
  threshold: number | null;
  enabled: boolean;
  severity: string; // 'info' | 'warning' | 'critical'
  cooldown_secs: number;
}

export interface UpdateAlertRule {
  enabled?: boolean;
  threshold?: number;
  severity?: string;
  cooldown_secs?: number;
}

export const alertRulesService = {
  /** GET /ops-api/alert-rules — règles de supervision de la machine. */
  list(): Promise<AlertRule[]> {
    return opsGet("/alert-rules");
  },
  /** PATCH /ops-api/alert-rules/{id} — met à jour une règle. */
  update(id: string, patch: UpdateAlertRule): Promise<AlertRule> {
    return opsPatch(`/alert-rules/${id}`, patch);
  },
};
