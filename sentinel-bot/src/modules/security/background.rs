//! Background tasks du module security : slowmode revert / lockdown revert.
//!
//! L'expulsion des quarantaines expirees ne vit plus ici : elle est decidee par
//! `platform-api` (`sentinel::jobs::security::kick_expired_quarantine`), qui
//! publie un evenement, et le bot l'execute via `quarantine_expired_consumer`.
//! Le crate `sentinel-worker` que mentionnait ce commentaire n'existe plus.

use serenity::all::Context;
use serenity::model::id::{GuildId, UserId};

use super::{LockdownKey, QuarantineKey, SecurityConfigKey, SlowmodeKey};

/// Spawn les background tasks security restantes : slowmode + lockdown
/// revert (encore en RAM tant que les `PermissionOverwrite` originaux
/// ne sont pas persistes en DB).
pub fn spawn_background(ctx: Context) {
    // 1. Rehydratation des quarantaines actives depuis la DB (post-reboot) : sans
    // ca, apres un redemarrage un user quarantine ne peut plus se verifier
    // (is_quarantined=false) et sa quarantaine ne peut plus etre levee cote bot.
    let ctx_q = ctx.clone();
    tokio::spawn(async move {
        let data = ctx_q.data.read().await;
        let (Some(sec_api), Some(quarantine)) = (
            data.get::<super::SecurityApiKey>(),
            data.get::<QuarantineKey>(),
        ) else {
            return;
        };
        match sec_api.list_active_quarantines().await {
            Ok(list) => {
                let mut n = 0u32;
                for (g, u) in list {
                    if let (Ok(gid), Ok(uid)) = (g.parse::<u64>(), u.parse::<u64>()) {
                        quarantine.rehydrate(GuildId::new(gid), UserId::new(uid));
                        n += 1;
                    }
                }
                tracing::info!(count = n, "Quarantaines actives rehydratees au demarrage");
            }
            Err(e) => tracing::warn!(error = %e, "Echec rehydratation des quarantaines"),
        }
    });

    // 2. Slowmode revert (15s loop)
    let ctx_s = ctx.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;

            let data = ctx_s.data.read().await;
            let Some(slowmode) = data.get::<SlowmodeKey>() else {
                continue;
            };
            let duration = data
                .get::<SecurityConfigKey>()
                .map(|c| c.slowmode_duration_secs)
                .unwrap_or(300);

            let expired = slowmode.expired_guilds(duration);
            for guild_id in expired {
                slowmode.deactivate_with_http(&ctx_s.http, guild_id).await;
            }
        }
    });

    // 3. Lockdown revert (15s loop)
    let ctx_l = ctx;
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;

            let data = ctx_l.data.read().await;
            let Some(lockdown) = data.get::<LockdownKey>() else {
                continue;
            };
            let duration = data
                .get::<SecurityConfigKey>()
                .map(|c| c.lockdown_duration_secs)
                .unwrap_or(300);

            let expired = lockdown.expired_guilds(duration);
            for guild_id in expired {
                lockdown.deactivate_with_http(&ctx_l.http, guild_id).await;
            }
        }
    });
}
