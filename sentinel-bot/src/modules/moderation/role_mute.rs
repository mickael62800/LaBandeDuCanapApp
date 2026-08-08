//! Mute par role : permet de conserver un salon d'appel accessible quand les
//! messages prives sont fermes. Les expirations sont persistees dans
//! `temp_roles`, donc survivent aux redemarrages.

use crate::shared::api_client::BaseApiClient;
use crate::shared::heartbeat::ApiClientKey;
use serenity::all::{Context, GuildId, RoleId, UserId};

/// Retourne le role de mute lorsque ce mode est explicitement active.
pub async fn configured_role(ctx: &Context, guild_id: GuildId) -> Option<RoleId> {
    let api = {
        let data = ctx.data.read().await;
        data.get::<ApiClientKey>().cloned()
    }?;
    let config = api
        .get_guild_config_for(&guild_id.to_string(), super::MODULE_BOT_NAME)
        .await
        .ok()?;
    if !BaseApiClient::config_bool(&config, "mute_uses_role", false) {
        return None;
    }
    config
        .get("mute_role_id")
        .and_then(|id| id.parse::<u64>().ok())
        .filter(|id| *id > 0)
        .map(RoleId::new)
}

/// Ajoute le role configure et enregistre son expiration avant de toucher
/// Discord. En cas d'echec Discord, la reservation est retiree afin qu'un
/// membre ne soit jamais demute de facon inattendue plus tard.
pub async fn apply(
    ctx: &Context,
    guild_id: GuildId,
    user_id: UserId,
    duration_secs: u64,
) -> Result<bool, String> {
    let Some(role_id) = configured_role(ctx, guild_id).await else {
        return Ok(false);
    };

    // Dans le salon d'appel, AutoMod peut encore supprimer un message
    // inacceptable. En revanche, un membre qui porte deja ce role ne doit pas
    // etre mute une seconde fois ni voir son echeance repoussee.
    let member = guild_id
        .member(&ctx.http, user_id)
        .await
        .map_err(|e| format!("membre introuvable : {e}"))?;
    if member.roles.contains(&role_id) {
        tracing::debug!(guild = %guild_id, user = %user_id, role = %role_id, "Mute par role deja actif, echeance inchangee");
        return Ok(true);
    }

    let expires_at =
        (chrono::Utc::now() + chrono::Duration::seconds(duration_secs as i64)).to_rfc3339();
    let roles_api = {
        let data = ctx.data.read().await;
        data.get::<crate::modules::community::RolesApiKey>()
            .cloned()
    }
    .ok_or_else(|| "service de roles temporairement indisponible".to_string())?;

    roles_api
        .create_temp_role(
            &guild_id.to_string(),
            &user_id.to_string(),
            &role_id.to_string(),
            &expires_at,
        )
        .await?;

    if let Err(e) = member.add_role(&ctx.http, role_id).await {
        roles_api
            .delete_temp_role(
                &guild_id.to_string(),
                &user_id.to_string(),
                &role_id.to_string(),
            )
            .await;
        return Err(format!("impossible d'ajouter le role de mute : {e}"));
    }

    tracing::info!(guild = %guild_id, user = %user_id, role = %role_id, "Mute applique via role temporaire");
    Ok(true)
}

/// Retire le role de mute et annule son expiration persistée. Retourne `true`
/// si le mode role est configure, meme si le membre ne portait deja plus le role.
pub async fn remove(ctx: &Context, guild_id: GuildId, user_id: UserId) -> Result<bool, String> {
    let Some(role_id) = configured_role(ctx, guild_id).await else {
        return Ok(false);
    };
    let member = guild_id
        .member(&ctx.http, user_id)
        .await
        .map_err(|e| format!("membre introuvable : {e}"))?;
    member
        .remove_role(&ctx.http, role_id)
        .await
        .map_err(|e| format!("impossible de retirer le role de mute : {e}"))?;
    let api = {
        let data = ctx.data.read().await;
        data.get::<crate::modules::community::RolesApiKey>()
            .cloned()
    };
    if let Some(api) = api {
        api.delete_temp_role(
            &guild_id.to_string(),
            &user_id.to_string(),
            &role_id.to_string(),
        )
        .await;
    }
    tracing::info!(guild = %guild_id, user = %user_id, role = %role_id, "Mute par role retire");
    Ok(true)
}
