use super::*;

impl AnalyzeMessageService {
    pub(super) async fn evaluate_flood_impl(
        &self,
        guild_id: &str,
        flood_count: i32,
    ) -> Result<crate::ports::inbound::ai::analyze_message::FloodDecision, DomainError> {
        use crate::ports::inbound::ai::analyze_message::FloodDecision;
        let entries = self
            .bot_config_repo
            .get_config(
                guild_id,
                crate::domain::entities::system::bot_names::AUTOMOD_BOT,
            )
            .await
            .unwrap_or_default();
        let num = |key: &str, default: u64| -> u64 {
            entries
                .iter()
                .find(|e| e.config_key == key)
                .and_then(|e| e.config_value.parse::<u64>().ok())
                .unwrap_or(default)
        };
        let auto_protect = entries
            .iter()
            .find(|e| e.config_key == "auto_protect_enabled")
            .map(|e| {
                let v = e.config_value.to_ascii_lowercase();
                v == "true" || v == "1"
            })
            .unwrap_or(true);
        let flood_max = num("flood_max_messages", 5);
        let severe_max = num("severe_flood_max_messages", flood_max * 2);
        let mute_dur = num("mute_duration_secs", 600);
        let severe = auto_protect && (flood_count.max(0) as u64) >= severe_max;
        // Score de confiance affiche sur la carte : fabrique cote serveur
        // (auparavant code en dur dans le bot : 0.99 severe / 0.9 sinon).
        let score = if severe { 0.99 } else { 0.9 };
        Ok(FloodDecision {
            severe,
            mute_duration_secs: mute_dur as i64,
            score,
        })
    }

    pub(super) async fn evaluate_attachments_impl(
        &self,
        guild_id: &str,
        filenames: Vec<String>,
    ) -> Result<crate::ports::inbound::ai::analyze_message::AttachmentDecision, DomainError> {
        use crate::ports::inbound::ai::analyze_message::AttachmentDecision;

        // Liste des extensions intrinsequement dangereuses (executables /
        // scripts). Auparavant codee en dur DANS le bot — la regle vit
        // desormais cote serveur.
        const DANGEROUS_EXTENSIONS: &[&str] = &[
            "exe", "bat", "cmd", "scr", "ps1", "vbs", "js", "jar", "com", "pif", "msi", "dll",
            "reg", "hta",
        ];

        let none = || AttachmentDecision {
            suspicious: false,
            action: Action::None,
            reason: String::new(),
            score: 0.0,
            filename: String::new(),
        };

        let entries = self
            .bot_config_repo
            .get_config(
                guild_id,
                crate::domain::entities::system::bot_names::AUTOMOD_BOT,
            )
            .await
            .unwrap_or_default();

        // Toggle `suspicious_files_enabled` (defaut true) : si desactive, aucune
        // piece jointe n'est jugee suspecte.
        let enabled = entries
            .iter()
            .find(|e| e.config_key == "suspicious_files_enabled")
            .map(|e| {
                let v = e.config_value.to_ascii_lowercase();
                matches!(v.as_str(), "true" | "1" | "yes")
            })
            .unwrap_or(true);
        if !enabled {
            return Ok(none());
        }

        // Extensions supplementaires configurees par serveur (CSV).
        let extra: Vec<String> = entries
            .iter()
            .find(|e| e.config_key == "suspicious_file_extensions")
            .map(|e| {
                e.config_value
                    .split(',')
                    .map(|s| s.trim().to_ascii_lowercase())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        for filename in &filenames {
            let name_lower = filename.to_lowercase();
            let ext = name_lower.rsplit('.').next().unwrap_or("");
            if DANGEROUS_EXTENSIONS.contains(&ext) || extra.iter().any(|e| e == ext) {
                return Ok(AttachmentDecision {
                    suspicious: true,
                    action: Action::Delete,
                    reason: format!("Piece jointe suspecte : {filename}"),
                    score: 1.0,
                    filename: filename.clone(),
                });
            }
        }

        Ok(none())
    }

    pub(super) async fn evaluate_caps_impl(
        &self,
        guild_id: &str,
    ) -> Result<crate::ports::inbound::ai::analyze_message::CapsDecision, DomainError> {
        use crate::ports::inbound::ai::analyze_message::CapsDecision;
        // Score de confiance affiche pour une detection de CAPS : fabrique cote
        // serveur (auparavant code en dur dans le bot : 0.8). Lu depuis la config
        // guild (`caps_confidence_score`) avec le defaut historique 0.8, borne
        // a [0.0, 1.0]. La detection (forme/longueur) reste locale au bot.
        let entries = self
            .bot_config_repo
            .get_config(
                guild_id,
                crate::domain::entities::system::bot_names::AUTOMOD_BOT,
            )
            .await
            .unwrap_or_default();
        let score = entries
            .iter()
            .find(|e| e.config_key == "caps_confidence_score")
            .and_then(|e| e.config_value.parse::<f64>().ok())
            .map(|v| v.clamp(0.0, 1.0))
            .unwrap_or(0.8);
        Ok(CapsDecision { score })
    }
}
