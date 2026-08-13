use crate::sentinel::adapters::outbound::postgres::pg_ctx;
use crate::sentinel::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use sqlx::PgPool;

use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::community::welcome_config_repository::WelcomeConfigData;
use platform_core::sentinel::ports::outbound::community::welcome_config_repository::WelcomeConfigRepository;
pub struct PgWelcomeConfigRepository {
    pool: PgPool,
}

impl PgWelcomeConfigRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct Row {
    guild_id: String,
    welcome_enabled: bool,
    welcome_channel_id: Option<String>,
    welcome_message: String,
    welcome_embed_color: String,
    welcome_dm_enabled: bool,
    welcome_dm_message: String,
    leave_enabled: bool,
    leave_channel_id: Option<String>,
    leave_message: String,
    rules_enabled: bool,
    rules_channel_id: Option<String>,
    rules_message: String,
    rules_role_id: Option<String>,
    rules_button_label: String,
    counter_enabled: bool,
    counter_channel_id: Option<String>,
    counter_format: String,
    anniversary_enabled: bool,
    anniversary_channel_id: Option<String>,
    anniversary_message: String,
    rejoin_message: String,
}

impl From<Row> for WelcomeConfigData {
    fn from(r: Row) -> Self {
        Self {
            guild_id: r.guild_id.into(),
            welcome_enabled: r.welcome_enabled,
            welcome_channel_id: r.welcome_channel_id,
            welcome_message: r.welcome_message,
            welcome_embed_color: r.welcome_embed_color,
            welcome_dm_enabled: r.welcome_dm_enabled,
            welcome_dm_message: r.welcome_dm_message,
            leave_enabled: r.leave_enabled,
            leave_channel_id: r.leave_channel_id,
            leave_message: r.leave_message,
            rules_enabled: r.rules_enabled,
            rules_channel_id: r.rules_channel_id,
            rules_message: r.rules_message,
            rules_role_id: r.rules_role_id,
            rules_button_label: r.rules_button_label,
            rules_embed_color: "5865f2".into(),
            // Verification d'age : Row legacy ne porte pas ces colonnes,
            // defaults ici (lecture reelle via overlay_with_bot_config).
            age_check_enabled: false,
            age_minimum: 20,
            unverified_role_id: None,
            age_modal_question: "Quel age as-tu ? (en chiffres)".into(),
            age_ban_message:
                "Tu dois avoir au moins {min} ans pour rejoindre ce serveur. Tu pourras revenir dans {annees} an(s)."
                    .into(),
            // Bornes/ban de la verification d'age : Row legacy ne porte pas ces
            // colonnes, defaults ici (lecture reelle via overlay_with_bot_config).
            age_min: 5,
            age_max: 120,
            age_ban_days_per_year: 365,
            age_ban_log_channel_id: None,
            counter_enabled: r.counter_enabled,
            counter_channel_id: r.counter_channel_id,
            counter_format: r.counter_format,
            // Row legacy (table welcome_config) ne contient pas ces colonnes :
            // defaults, la lecture reelle passe par overlay_with_bot_config.
            voice_counter_enabled: false,
            voice_counter_channel_id: None,
            voice_counter_format: "En Vocal : {count}".into(),
            anniversary_enabled: r.anniversary_enabled,
            anniversary_channel_id: r.anniversary_channel_id,
            anniversary_message: r.anniversary_message,
            rejoin_message: r.rejoin_message,
            // Row ne contient pas les champs embed enrichi (legacy welcome_config
            // table, desormais inutilisee en lecture). Valeurs par defaut.
            welcome_title: "Bienvenue !".into(),
            welcome_image_url: "".into(),
            welcome_footer_text: "{count} membres".into(),
            rejoin_title: "Bon retour !".into(),
            rejoin_image_url: "".into(),
            rejoin_footer_text: "{count} membres".into(),
            leave_title: "Au revoir...".into(),
            leave_image_url: "".into(),
            leave_footer_text: "{count} membres".into(),
            leave_embed_color: "e74c3c".into(),
            anniversary_title: "Joyeux anniversaire !".into(),
            anniversary_image_url: "".into(),
            anniversary_footer_text: "{count} membres".into(),
        }
    }
}

fn default_config(guild_id: &str) -> WelcomeConfigData {
    WelcomeConfigData {
        guild_id: guild_id.to_string().into(),
        welcome_enabled: true,
        welcome_channel_id: None,
        welcome_message: "Bienvenue {user} sur **{server}** ! Tu es le **{count}e** membre.".into(),
        welcome_embed_color: "3498db".into(),
        welcome_dm_enabled: false,
        welcome_dm_message: "Bienvenue sur **{server}** !".into(),
        leave_enabled: true,
        leave_channel_id: None,
        leave_message: "{user} nous a quittes. Nous sommes maintenant **{count}** membres.".into(),
        rules_enabled: false,
        rules_channel_id: None,
        rules_message: "Lis les regles et clique sur le bouton pour acceder au serveur.".into(),
        rules_role_id: None,
        rules_button_label: "J'accepte les regles".into(),
        rules_embed_color: "5865f2".into(),
        age_check_enabled: false,
        age_minimum: 20,
        unverified_role_id: None,
        age_modal_question: "Quel age as-tu ? (en chiffres)".into(),
        age_ban_message:
            "Tu dois avoir au moins {min} ans pour rejoindre ce serveur. Tu pourras revenir dans {annees} an(s)."
                .into(),
        age_min: 5,
        age_max: 120,
        age_ban_days_per_year: 365,
        age_ban_log_channel_id: None,
        counter_enabled: false,
        counter_channel_id: None,
        counter_format: "Membres : {count}".into(),
        voice_counter_enabled: false,
        voice_counter_channel_id: None,
        voice_counter_format: "En Vocal : {count}".into(),
        anniversary_enabled: false,
        anniversary_channel_id: None,
        anniversary_message:
            "Felicitations {user}, ca fait **{years} an(s)** que tu es sur **{server}** !".into(),
        rejoin_message: "Content de te revoir {user} ! Tu nous avais manque.".into(),
        welcome_title: "Bienvenue !".into(),
        welcome_image_url: "".into(),
        welcome_footer_text: "{count} membres".into(),
        rejoin_title: "Bon retour !".into(),
        rejoin_image_url: "".into(),
        rejoin_footer_text: "{count} membres".into(),
        leave_title: "Au revoir...".into(),
        leave_image_url: "".into(),
        leave_footer_text: "{count} membres".into(),
        leave_embed_color: "e74c3c".into(),
        anniversary_title: "Joyeux anniversaire !".into(),
        anniversary_image_url: "".into(),
        anniversary_footer_text: "{count} membres".into(),
    }
}

fn parse_bool(v: &str, default: bool) -> bool {
    matches!(v, "true" | "1" | "yes" | "on") || (v.is_empty() && default)
}

fn overlay_with_bot_config(
    base: WelcomeConfigData,
    kvs: Vec<(String, String)>,
) -> WelcomeConfigData {
    let mut d = base;
    for (k, v) in kvs {
        match k.as_str() {
            "welcome_enabled" => d.welcome_enabled = parse_bool(&v, d.welcome_enabled),
            "welcome_channel_id" => {
                d.welcome_channel_id = if v.is_empty() { None } else { Some(v) }
            }
            "welcome_message" if !v.is_empty() => {
                d.welcome_message = v;
            }
            "welcome_embed_color" if !v.is_empty() => {
                d.welcome_embed_color = v;
            }
            "welcome_dm_enabled" => d.welcome_dm_enabled = parse_bool(&v, d.welcome_dm_enabled),
            "welcome_dm_message" if !v.is_empty() => {
                d.welcome_dm_message = v;
            }
            "rejoin_message" if !v.is_empty() => {
                d.rejoin_message = v;
            }
            "leave_enabled" => d.leave_enabled = parse_bool(&v, d.leave_enabled),
            "leave_channel_id" => d.leave_channel_id = if v.is_empty() { None } else { Some(v) },
            "leave_message" if !v.is_empty() => {
                d.leave_message = v;
            }
            "rules_enabled" => d.rules_enabled = parse_bool(&v, d.rules_enabled),
            "rules_channel_id" => d.rules_channel_id = if v.is_empty() { None } else { Some(v) },
            "rules_message" if !v.is_empty() => {
                d.rules_message = v;
            }
            "rules_role_id" => d.rules_role_id = if v.is_empty() { None } else { Some(v) },
            "rules_button_label" if !v.is_empty() => {
                d.rules_button_label = v;
            }
            "age_check_enabled" => d.age_check_enabled = parse_bool(&v, d.age_check_enabled),
            "age_minimum" => {
                if let Ok(n) = v.parse::<i32>() {
                    d.age_minimum = n;
                }
            }
            "unverified_role_id" => {
                d.unverified_role_id = if v.is_empty() { None } else { Some(v) }
            }
            "age_modal_question" if !v.is_empty() => {
                d.age_modal_question = v;
            }
            "age_ban_message" if !v.is_empty() => {
                d.age_ban_message = v;
            }
            "age_min" => {
                if let Ok(n) = v.parse::<i32>() {
                    d.age_min = n;
                }
            }
            "age_max" => {
                if let Ok(n) = v.parse::<i32>() {
                    d.age_max = n;
                }
            }
            "age_ban_days_per_year" => {
                if let Ok(n) = v.parse::<i32>() {
                    d.age_ban_days_per_year = n;
                }
            }
            "age_ban_log_channel_id" => {
                d.age_ban_log_channel_id = if v.is_empty() { None } else { Some(v) }
            }
            "leave_embed_color" if !v.is_empty() => {
                d.leave_embed_color = v;
            }
            "rules_embed_color" if !v.is_empty() => {
                d.rules_embed_color = v;
            }
            "counter_enabled" => d.counter_enabled = parse_bool(&v, d.counter_enabled),
            "counter_channel_id" => {
                d.counter_channel_id = if v.is_empty() { None } else { Some(v) }
            }
            "counter_format" if !v.is_empty() => {
                d.counter_format = v;
            }
            "voice_counter_enabled" => {
                d.voice_counter_enabled = parse_bool(&v, d.voice_counter_enabled)
            }
            "voice_counter_channel_id" => {
                d.voice_counter_channel_id = if v.is_empty() { None } else { Some(v) }
            }
            "voice_counter_format" if !v.is_empty() => {
                d.voice_counter_format = v;
            }
            "anniversary_enabled" => d.anniversary_enabled = parse_bool(&v, d.anniversary_enabled),
            "anniversary_channel_id" => {
                d.anniversary_channel_id = if v.is_empty() { None } else { Some(v) }
            }
            "anniversary_message" if !v.is_empty() => {
                d.anniversary_message = v;
            }
            "welcome_title" if !v.is_empty() => {
                d.welcome_title = v;
            }
            "welcome_image_url" => d.welcome_image_url = v,
            "welcome_footer_text" if !v.is_empty() => {
                d.welcome_footer_text = v;
            }
            "rejoin_title" if !v.is_empty() => {
                d.rejoin_title = v;
            }
            "rejoin_image_url" => d.rejoin_image_url = v,
            "rejoin_footer_text" if !v.is_empty() => {
                d.rejoin_footer_text = v;
            }
            "leave_title" if !v.is_empty() => {
                d.leave_title = v;
            }
            "leave_image_url" => d.leave_image_url = v,
            "leave_footer_text" if !v.is_empty() => {
                d.leave_footer_text = v;
            }
            "anniversary_title" if !v.is_empty() => {
                d.anniversary_title = v;
            }
            "anniversary_image_url" => d.anniversary_image_url = v,
            "anniversary_footer_text" if !v.is_empty() => {
                d.anniversary_footer_text = v;
            }
            _ => {}
        }
    }
    d
}

#[async_trait]
impl WelcomeConfigRepository for PgWelcomeConfigRepository {
    /// Lit la config welcome depuis `bot_guild_config` (migration 148).
    /// Fallback sur les defaults si aucune cle n est configuree pour ce
    /// serveur. L ancienne table `welcome_config` n est plus lue.
    async fn get_config(&self, guild_id: &str) -> Result<WelcomeConfigData, DomainError> {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT config_key, config_value FROM bot_guild_config \
             WHERE guild_id = $1 AND bot_name = 'welcome-bot'",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(overlay_with_bot_config(default_config(guild_id), rows))
    }

    /// Ecrit la config welcome dans `bot_guild_config` (migration 148).
    ///
    /// Historique : on ecrivait dans l'ancienne table `welcome_config`, mais
    /// `get_config` lit desormais depuis `bot_guild_config`. Pour eviter la
    /// desynchronisation save/get, save_config route sa sortie vers la meme
    /// table que get_config.
    async fn save_config(
        &self,
        guild_id: &str,
        d: &WelcomeConfigData,
    ) -> Result<WelcomeConfigData, DomainError> {
        let kvs = build_welcome_config_kvs(d);

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(pg_ctx("welcome save tx begin"))?;

        for (k, v) in &kvs {
            sqlx::query(
                "INSERT INTO bot_guild_config (id, guild_id, bot_name, config_key, config_value, updated_at) \
                 VALUES (gen_random_uuid(), $1, 'welcome-bot', $2, $3, NOW()) \
                 ON CONFLICT (guild_id, bot_name, config_key) DO UPDATE SET \
                   config_value = EXCLUDED.config_value, updated_at = NOW()",
            )
            .bind(guild_id)
            .bind(k)
            .bind(v)
            .execute(&mut *tx)
            .await
            .map_err(|e| DomainError::Internal(format!("welcome save {}: {e}", k)))?;
        }

        tx.commit()
            .await
            .map_err(pg_ctx("welcome save tx commit"))?;

        // Relit la config apres upsert pour garantir que get_config renverra bien
        // ce qui vient d'etre ecrit.
        self.get_config(guild_id).await
    }
}

/// Convertit une `WelcomeConfigData` en paires (config_key, config_value)
/// compatibles avec `overlay_with_bot_config`. Pur helper sans IO pour tests.
pub(super) fn build_welcome_config_kvs(d: &WelcomeConfigData) -> Vec<(&'static str, String)> {
    fn b(v: bool) -> String {
        if v {
            "true".into()
        } else {
            "false".into()
        }
    }
    fn opt(v: &Option<String>) -> String {
        v.clone().unwrap_or_default()
    }
    vec![
        ("welcome_enabled", b(d.welcome_enabled)),
        ("welcome_channel_id", opt(&d.welcome_channel_id)),
        ("welcome_message", d.welcome_message.clone()),
        ("welcome_embed_color", d.welcome_embed_color.clone()),
        ("welcome_dm_enabled", b(d.welcome_dm_enabled)),
        ("welcome_dm_message", d.welcome_dm_message.clone()),
        ("leave_enabled", b(d.leave_enabled)),
        ("leave_channel_id", opt(&d.leave_channel_id)),
        ("leave_message", d.leave_message.clone()),
        ("rules_enabled", b(d.rules_enabled)),
        ("rules_channel_id", opt(&d.rules_channel_id)),
        ("rules_message", d.rules_message.clone()),
        ("rules_role_id", opt(&d.rules_role_id)),
        ("rules_button_label", d.rules_button_label.clone()),
        ("age_check_enabled", b(d.age_check_enabled)),
        ("age_minimum", d.age_minimum.to_string()),
        ("unverified_role_id", opt(&d.unverified_role_id)),
        ("age_modal_question", d.age_modal_question.clone()),
        ("age_ban_message", d.age_ban_message.clone()),
        ("age_min", d.age_min.to_string()),
        ("age_max", d.age_max.to_string()),
        ("age_ban_days_per_year", d.age_ban_days_per_year.to_string()),
        ("age_ban_log_channel_id", opt(&d.age_ban_log_channel_id)),
        ("leave_embed_color", d.leave_embed_color.clone()),
        ("rules_embed_color", d.rules_embed_color.clone()),
        ("counter_enabled", b(d.counter_enabled)),
        ("counter_channel_id", opt(&d.counter_channel_id)),
        ("counter_format", d.counter_format.clone()),
        ("voice_counter_enabled", b(d.voice_counter_enabled)),
        ("voice_counter_channel_id", opt(&d.voice_counter_channel_id)),
        ("voice_counter_format", d.voice_counter_format.clone()),
        ("anniversary_enabled", b(d.anniversary_enabled)),
        ("anniversary_channel_id", opt(&d.anniversary_channel_id)),
        ("anniversary_message", d.anniversary_message.clone()),
        ("rejoin_message", d.rejoin_message.clone()),
        ("welcome_title", d.welcome_title.clone()),
        ("welcome_image_url", d.welcome_image_url.clone()),
        ("welcome_footer_text", d.welcome_footer_text.clone()),
        ("rejoin_title", d.rejoin_title.clone()),
        ("rejoin_image_url", d.rejoin_image_url.clone()),
        ("rejoin_footer_text", d.rejoin_footer_text.clone()),
        ("leave_title", d.leave_title.clone()),
        ("leave_image_url", d.leave_image_url.clone()),
        ("leave_footer_text", d.leave_footer_text.clone()),
        ("anniversary_title", d.anniversary_title.clone()),
        ("anniversary_image_url", d.anniversary_image_url.clone()),
        ("anniversary_footer_text", d.anniversary_footer_text.clone()),
    ]
}
