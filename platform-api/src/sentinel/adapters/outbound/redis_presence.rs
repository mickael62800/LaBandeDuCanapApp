//! Lecture de la presence publiee par le bot.
//!
//! Symetrique de `sentinel-bot/src/shared/presence.rs` : les cles et le
//! format sont definis la-bas, ici on ne fait que lire. Toute evolution du
//! format doit toucher les deux fichiers — d'ou la mention explicite.
//!
//! Redis indisponible n'est PAS une erreur remontee a l'appelant : la presence
//! est un agrement, pas une donnee critique. Une panne Redis doit vider la
//! section, pas casser la page membre.

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use redis::AsyncCommands;
use serde::Deserialize;

use platform_core::sentinel::domain::entities::community::presence::{
    TextChannelActivity, VoiceChannelPresence, VoiceMember, VoicePresence, TEXT_WINDOW_SECONDS,
};
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::community::presence_repository::PresenceRepository;

pub struct RedisPresenceRepository {
    client: redis::Client,
}

impl RedisPresenceRepository {
    pub fn new(client: redis::Client) -> Self {
        Self { client }
    }

    async fn conn(&self) -> Option<redis::aio::MultiplexedConnection> {
        match self.client.get_multiplexed_async_connection().await {
            Ok(c) => Some(c),
            Err(e) => {
                tracing::warn!(error = %e, "presence : connexion Redis indisponible");
                None
            }
        }
    }
}

fn voice_key(guild_id: &str) -> String {
    format!("sentinel:presence:voice:{guild_id}")
}

fn text_index_key(guild_id: &str) -> String {
    format!("sentinel:presence:text:{guild_id}")
}

fn text_channel_key(guild_id: &str, channel_id: &str) -> String {
    format!("sentinel:presence:text:{guild_id}:{channel_id}")
}

#[derive(Deserialize)]
struct VoiceMemberRaw {
    user_id: String,
    username: String,
    self_mute: bool,
    self_deaf: bool,
    server_mute: bool,
    streaming: bool,
    video: bool,
}

#[derive(Deserialize)]
struct VoiceChannelRaw {
    channel_id: String,
    channel_name: String,
    members: Vec<VoiceMemberRaw>,
    /// Absent des instantanes publies avant l'introduction du marqueur. Sans
    /// defaut, ils deviendraient illisibles et la presence disparaitrait le
    /// temps que le bot republie.
    #[serde(default)]
    restreint: bool,
}

#[derive(Deserialize)]
struct VoiceSnapshotRaw {
    channels: Vec<VoiceChannelRaw>,
    updated_at: String,
}

#[derive(Deserialize)]
struct TextIndexRaw {
    channel_name: String,
    last_message_at: i64,
}

#[async_trait]
impl PresenceRepository for RedisPresenceRepository {
    async fn voice(&self, guild_id: &str) -> Result<Option<VoicePresence>, DomainError> {
        let Some(mut conn) = self.conn().await else {
            return Ok(None);
        };

        let brut: Option<String> = conn.get(voice_key(guild_id)).await.unwrap_or(None);
        let Some(brut) = brut else {
            return Ok(None);
        };

        // Un instantane illisible vaut un instantane absent : la section se
        // masque au lieu de faire echouer toute la page.
        let Ok(snapshot) = serde_json::from_str::<VoiceSnapshotRaw>(&brut) else {
            tracing::warn!(guild_id, "presence vocale illisible, ignoree");
            return Ok(None);
        };

        let Ok(updated_at) = DateTime::parse_from_rfc3339(&snapshot.updated_at) else {
            return Ok(None);
        };

        Ok(Some(VoicePresence {
            updated_at: updated_at.with_timezone(&Utc),
            channels: snapshot
                .channels
                .into_iter()
                .map(|c| VoiceChannelPresence {
                    channel_id: c.channel_id,
                    channel_name: c.channel_name,
                    restreint: c.restreint,
                    members: c
                        .members
                        .into_iter()
                        .map(|m| VoiceMember {
                            user_id: m.user_id,
                            username: m.username,
                            self_mute: m.self_mute,
                            self_deaf: m.self_deaf,
                            server_mute: m.server_mute,
                            streaming: m.streaming,
                            video: m.video,
                        })
                        .collect(),
                })
                .collect(),
        }))
    }

    async fn text_activity(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<TextChannelActivity>, DomainError> {
        let Some(mut conn) = self.conn().await else {
            return Ok(vec![]);
        };

        let index: std::collections::HashMap<String, String> = conn
            .hgetall(text_index_key(guild_id))
            .await
            .unwrap_or_default();

        let seuil = Utc::now().timestamp() - TEXT_WINDOW_SECONDS;

        // On trie AVANT d'aller chercher les auteurs : sans ca, un serveur a
        // cinquante salons declencherait cinquante requetes pour n'en garder
        // que huit.
        let mut salons: Vec<(String, TextIndexRaw)> = index
            .into_iter()
            .filter_map(|(id, brut)| {
                serde_json::from_str::<TextIndexRaw>(&brut)
                    .ok()
                    .map(|v| (id, v))
            })
            .filter(|(_, v)| v.last_message_at >= seuil)
            .collect();

        salons.sort_by_key(|(_, meta)| std::cmp::Reverse(meta.last_message_at));
        salons.truncate(limit.max(0) as usize);

        let mut out = Vec::with_capacity(salons.len());
        for (channel_id, meta) in salons {
            // Du plus recent au plus ancien : c'est l'ordre d'affichage.
            let auteurs: Vec<String> = conn
                .zrevrange(text_channel_key(guild_id, &channel_id), 0, -1)
                .await
                .unwrap_or_default();

            // Un salon dont tous les auteurs ont expire n'a plus rien a
            // montrer, meme si son entree d'index survit encore.
            if auteurs.is_empty() {
                continue;
            }

            let Some(last_message_at) = Utc.timestamp_opt(meta.last_message_at, 0).single() else {
                continue;
            };

            out.push(TextChannelActivity {
                channel_id,
                channel_name: meta.channel_name,
                recent_authors: auteurs,
                last_message_at,
            });
        }

        Ok(out)
    }
}
