//! Vitrine PUBLIQUE des serveurs de jeu — accessible sans connexion.
//!
//! # Regle de securite
//!
//! Monte hors de toute authentification. Chaque champ expose ici est lisible
//! par n'importe qui sur Internet, donc le DTO est ecrit a la main, champ par
//! champ, en partant de rien. Ce qui n'y figure PAS est aussi important que ce
//! qui y figure : ni mot de passe RCON, ni port RCON, ni identifiant de
//! conteneur, ni proprietaire, ni salon Discord.
//!
//! # Adresse de connexion
//!
//! L'adresse n'est publiee que si `ip_revealed` est vrai. Le game-portal a un
//! mecanisme de revelation differee (`ip_reveal_at`) : le contourner ici
//! reviendrait a annuler la fonctionnalite. Tant que l'IP est masquee, on
//! annonce que le serveur existe, pas comment s'y connecter.

use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;

use crate::adapters::inbound::http::handlers::ApiError;
use crate::bootstrap::AppState;
use nexus_core::domain::entities::game::server::GameServerStatus;
use nexus_core::domain::errors::DomainError;

#[derive(Debug, Serialize)]
pub struct PublicGameServerDto {
    pub id: String,
    pub name: String,
    /// Nom lisible du jeu (resolu depuis le template), pas son slug technique.
    pub game: String,
    pub icon: Option<String>,
    /// Jaquette du jeu, chemin RELATIF (`/imgs/...`) resolu depuis le
    /// template. C'est ce que la page membre affiche en grille ; l'emoji
    /// `icon` ne sert plus que de repli.
    ///
    /// Relatif et non absolu : le site le sert tel quel, et une URL absolue
    /// figerait le domaine en base.
    pub cover_image_url: Option<String>,
    /// `running` | `stopped` — les etats transitoires sont ramenes a l'un des
    /// deux : un visiteur n'a que faire de « stopping ».
    pub online: bool,
    pub player_count: i32,
    /// Port public, uniquement si l'adresse a ete revelee.
    pub port: Option<u16>,
    /// Adresse complete hote:port, uniquement apres revelation et si l'hote
    /// public est configure pour cette guild.
    pub address: Option<String>,
    /// Vrai quand l'adresse est publiable (fin du delai de revelation).
    pub address_revealed: bool,
}

/// GET /api/public/games/{guild_id}/servers
pub async fn public_servers(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<PublicGameServerDto>>, ApiError> {
    // Endpoint non authentifie : validation stricte, il est expose au balayage.
    if guild_id.len() > 20 || !guild_id.chars().all(|c| c.is_ascii_digit()) {
        return Err(ApiError(DomainError::ValidationError(
            "guild_id invalide".into(),
        )));
    }

    let servers = state.game_servers_uc.list_for_guild(&guild_id).await?;
    let templates = state.game_template_repo.list().await.unwrap_or_default();
    let public_host = super::servers::hote_public(&state, &guild_id).await;

    let out = servers
        .into_iter()
        // Un serveur supprime n'a rien a faire dans une vitrine publique.
        .filter(|s| !matches!(s.status, GameServerStatus::Deleted))
        .map(|s| {
            let tpl = templates.iter().find(|t| t.id == s.template_id);
            PublicGameServerDto {
                id: s.id.to_string(),
                name: s.name,
                game: tpl.map(|t| t.name.clone()).unwrap_or_else(|| "Jeu".into()),
                icon: tpl.and_then(|t| t.icon.clone()),
                cover_image_url: tpl.and_then(|t| t.cover_image_url.clone()),
                online: matches!(s.status, GameServerStatus::Running),
                player_count: s.last_player_count,
                port: if s.ip_revealed { s.host_port } else { None },
                address: if s.ip_revealed {
                    public_host
                        .as_deref()
                        .zip(s.host_port)
                        .map(|(host, port)| format!("{}:{}", host.trim(), port))
                } else {
                    None
                },
                address_revealed: s.ip_revealed,
            }
        })
        .collect();

    Ok(Json(out))
}
