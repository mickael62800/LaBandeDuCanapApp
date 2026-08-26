//! Game Portal : projection Discord d'un serveur de jeu.
//!
//! Le module est piloté par les événements publiés par `nexus-api` sur la
//! stream Redis `nexus:events` :
//!   - `game_server_scheduled` : ouverture programmee — cree les memes salons et
//!     le panneau que `game_server_started`, mais le conteneur reste eteint (le
//!     worker le demarre ~5 min avant l'ouverture) ;
//!   - `game_server_started` : cree un salon texte + un salon vocal PRIVES
//!     (visibles du seul role du jeu) dans la categorie configuree, epingle un
//!     panneau avec bouton d'inscription et ping le role ;
//!   - `game_server_stopped` : arret temporaire — les salons, le role et le
//!     panneau sont CONSERVES (pour pouvoir redemarrer sans tout reconstruire) ;
//!   - `game_server_deleted` : suppression du jeu — supprime salons et role ;
//!   - `game_ip_reveal` : poste l'adresse de connexion et rafraichit le panneau ;
//!   - `game_daily_ping` : rappelle l'ouverture a venir au role du jeu.
//!
//! Les joueurs consultent les reglages d'un serveur via la commande ephemere
//! `/game parametres` (voir `params_embeds_for_channel`), pas via une carte
//! epinglee.
//!
//! La configuration par guild (categorie, hote public) est lue via
//! `GET /api/config/{guild_id}/game-portal`.

use std::sync::Arc;

use serenity::all::{
    ButtonStyle, ChannelId, ChannelType, Colour, ComponentInteraction, Context, CreateActionRow,
    CreateButton, CreateChannel, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseFollowup, CreateInteractionResponseMessage, CreateMessage,
    EditChannel, EditInteractionResponse, EditMessage, EditRole, GetMessages, GuildId, MessageId,
    PermissionOverwrite, PermissionOverwriteType, Permissions, RoleId,
};

use crate::api_client::{ApiClient, GameServer};

/// Nom du module dans `bot_guild_config` (cle de lecture de la config guild).
const MODULE_BOT_NAME: &str = "game-portal";

/// custom_id du bouton d'inscription : `gp_register:{server_id}`.
pub const REGISTER_PREFIX: &str = "gp_register:";
pub const UNREGISTER_PREFIX: &str = "gp_unregister:";
pub const REVEAL_IP_PREFIX: &str = "gp_reveal_ip:";

pub fn handles_component(custom_id: &str) -> bool {
    custom_id.starts_with(REGISTER_PREFIX)
        || custom_id.starts_with(UNREGISTER_PREFIX)
        || custom_id.starts_with(REVEAL_IP_PREFIX)
}

pub async fn on_component(api: &ApiClient, ctx: &Context, component: &ComponentInteraction) {
    if let Some(server_id) = component.data.custom_id.strip_prefix(REVEAL_IP_PREFIX) {
        on_reveal_ip_component(api, ctx, component, server_id).await;
        return;
    }

    let mut is_register = true;
    let server_id = if let Some(id) = component.data.custom_id.strip_prefix(REGISTER_PREFIX) {
        id
    } else if let Some(id) = component.data.custom_id.strip_prefix(UNREGISTER_PREFIX) {
        is_register = false;
        id
    } else {
        return;
    };

    // ACCUSE IMMEDIAT, avant le moindre appel reseau.
    //
    // Discord ferme l'interaction au bout de 3 secondes. Or reconstruire le
    // panneau demande une inscription, la liste des inscrits, le serveur, son
    // modele et l'attribution d'un role : bien plus que 3 s des que l'API
    // repond lentement, et le clic echouait alors en « NexusBot n'a pas
    // repondu a temps » alors que l'inscription, elle, avait bien eu lieu.
    //
    // `Acknowledge` plutot que `Defer` : le message garde son apparence, sans
    // etat de chargement, et reste modifiable ensuite.
    if let Err(error) = component
        .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
        .await
    {
        tracing::warn!(%error, server_id, "game-portal: accuse de reception impossible");
        return;
    }

    let reg_result = if is_register {
        api.register_to_server(server_id, &component.user.id.to_string())
            .await
    } else {
        api.unregister_from_server(server_id, &component.user.id.to_string())
            .await
    };

    // L'API peut refuser (serveur ferme, capacite, etc.) : on ne pretend pas
    // que l'inscription a reussi -> message ephemere et on s'arrete.
    if let Err(e) = reg_result {
        let content = format_registration_error_content(is_register, &e);
        let _ = component
            .create_followup(
                &ctx.http,
                CreateInteractionResponseFollowup::new()
                    .content(content)
                    .ephemeral(true),
            )
            .await;
        return;
    }

    // Re-fetch inscrits + serveur pour reconstruire le panneau.
    let user_ids: Vec<String> = api
        .list_server_registrations(server_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.user_id)
        .collect();

    if let Ok(detail) = api.get_game_server(server_id).await {
        let template = api.get_game_template(&detail.server.template_id).await.ok();
        let game_name = template
            .as_ref()
            .map(|t| t.name.clone())
            .unwrap_or_else(|| "Jeu".into());
        let cover_url = public_cover_url_for_status(
            template
                .as_ref()
                .and_then(|template| template.cover_image_url.as_deref()),
            etat_affiche(&detail.server),
        );
        grant_session_role(ctx, component, server_id, &game_name).await;
        let embed = build_public_panel_embed(
            &game_name,
            &detail.server.name,
            &user_ids,
            detail.server.ip_reveal_at.as_deref(),
            detail.server.ip_revealed,
            cover_url.as_deref(),
        );
        // Le panneau est le message d'origine : on l'edite, l'accuse de
        // reception ayant deja consomme la reponse initiale.
        let _ = component
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .embed(embed)
                    .components(panel_rows(server_id, detail.server.ip_revealed)),
            )
            .await;
        return;
    }

    // Fallback : simple accuse ephemere.
    let action_msg = format_registration_ack_content(is_register);
    let _ = component
        .create_followup(
            &ctx.http,
            CreateInteractionResponseFollowup::new()
                .content(format!("✅ {action_msg}."))
                .ephemeral(true),
        )
        .await;
}

fn session_suffix(server_id: &str) -> String {
    server_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(8)
        .collect()
}

fn session_role_name(game_name: &str, server_id: &str) -> String {
    format!("{}_{}", slugify(game_name), session_suffix(server_id))
}

/// Resout les trois noms de salons d'une session.
///
/// Le calcul vit dans `platform-core` : il doit donner exactement le meme
/// resultat a la creation d'une session et lors d'un renommage ulterieur. Deux
/// implementations auraient fini par diverger, et un salon aurait porte un nom
/// que plus aucun nettoyage ne reconnaitrait.
fn noms_des_salons(
    server: &GameServer,
    game_name: &str,
    cfg: &std::collections::HashMap<String, String>,
) -> (String, String, String) {
    use platform_core::nexus::domain::entities::game::channel_names as noms;

    let modele = |cle: &str| cfg.get(cle).map(String::as_str);
    let resoudre = |libre: Option<&str>, cle: &str, defaut: &str, genre| {
        noms::nom_de_salon(libre, modele(cle), defaut, game_name, &server.name, genre)
    };

    (
        resoudre(
            server.channel_name_registration.as_deref(),
            "channel_name_registration_template",
            noms::MODELE_INSCRIPTION_PAR_DEFAUT,
            noms::TypeDeSalon::Ecrit,
        ),
        resoudre(
            server.channel_name_private.as_deref(),
            "channel_name_private_template",
            noms::MODELE_PRIVE_PAR_DEFAUT,
            noms::TypeDeSalon::Ecrit,
        ),
        resoudre(
            server.channel_name_voice.as_deref(),
            "channel_name_voice_template",
            noms::MODELE_VOCAL_PAR_DEFAUT,
            noms::TypeDeSalon::Vocal,
        ),
    )
}

fn legacy_private_text_name(server_id: &str) -> String {
    format!("joueurs-{}", session_suffix(server_id))
}

fn private_text_topic(server_id: &str) -> String {
    format!("Nexus Game Portal | session:{server_id} | private")
}

fn is_player_password_key(key: &str) -> bool {
    matches!(
        key.to_ascii_uppercase().as_str(),
        "PASSWORD" | "SERVER_PASS" | "SERVER_PASSWORD" | "SERVERCONFIG_SERVERPASSWORD"
    )
}

fn is_safe_game_option(key: &str) -> bool {
    let key = key.to_ascii_uppercase();
    if is_player_password_key(&key) {
        return true;
    }
    ![
        "ADMIN",
        "RCON",
        "TOKEN",
        "SECRET",
        "PRIVATE_KEY",
        "API_KEY",
        "ACCESS_KEY",
        "OPERATOR",
        "OP_PERMISSION",
    ]
    .iter()
    .any(|forbidden| key.contains(forbidden))
        && key != "OPS"
}

/// Nomme un reglage en francais, d'apres le modele du jeu.
///
/// Sans cela, la carte affichait `SPAWN_MONSTERS` ou `DEATH_PENALTY` : des
/// cles techniques, en anglais, qui ne disent rien a un joueur venu savoir
/// comment se joue la partie. Le libelle existe deja dans le schema du jeu, il
/// suffisait de s'en servir.
///
/// Repli sur la cle brute si le modele ne decrit pas ce reglage : mieux vaut
/// un nom technique qu'une ligne disparue.
fn nom_du_reglage(schema: &[crate::api_client::TemplateField], key: &str) -> String {
    schema
        .iter()
        .find(|f| f.key.eq_ignore_ascii_case(key))
        .map(|f| f.label.clone())
        .unwrap_or_else(|| key.to_string())
}

/// Section d'un reglage, pour regrouper la carte.
fn section_du_reglage(schema: &[crate::api_client::TemplateField], key: &str) -> String {
    schema
        .iter()
        .find(|f| f.key.eq_ignore_ascii_case(key))
        .and_then(|f| f.group.clone())
        .filter(|g| !g.trim().is_empty())
        .unwrap_or_else(|| "Reglages generaux".to_string())
}

/// Rend une valeur lisible : un `true` brut n'apprend rien a personne.
fn valeur_lisible(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" => "Oui".to_string(),
        "false" | "0" => "Non".to_string(),
        "" => "—".to_string(),
        _ => value.trim().to_string(),
    }
}

/// Une ligne de la carte des parametres, prete a etre triee.
struct LigneReglage {
    section: String,
    texte: String,
}

fn lignes_reglages(
    config: &std::collections::HashMap<String, String>,
    schema: &[crate::api_client::TemplateField],
    avec_mot_de_passe: bool,
) -> Vec<String> {
    let mut lignes: Vec<LigneReglage> = config
        .iter()
        .filter(|(key, _)| is_safe_game_option(key))
        .filter(|(key, _)| avec_mot_de_passe || !is_player_password_key(key))
        .map(|(key, value)| {
            let affichee = if is_player_password_key(key) && value.trim().is_empty() {
                "Aucun (accès libre)".to_string()
            } else {
                valeur_lisible(value)
            };
            LigneReglage {
                section: section_du_reglage(schema, key),
                texte: format!("**{}** : `{}`", nom_du_reglage(schema, key), affichee),
            }
        })
        .collect();

    // Par section, puis par libelle a l'interieur : la carte se lit alors comme
    // la page de configuration, et non comme un vidage de base de donnees.
    lignes.sort_by(|a, b| a.section.cmp(&b.section).then(a.texte.cmp(&b.texte)));

    let mut sortie = Vec::new();
    let mut section_courante = String::new();
    for ligne in lignes {
        if ligne.section != section_courante {
            section_courante = ligne.section.clone();
            sortie.push(format!("__**{section_courante}**__"));
        }
        sortie.push(ligne.texte);
    }
    sortie
}

fn public_game_options(
    config: &std::collections::HashMap<String, String>,
    schema: &[crate::api_client::TemplateField],
) -> Vec<String> {
    lignes_reglages(config, schema, true)
}

/// Footer des embeds de la commande `/game parametres`.
const OPTIONS_FOOTER: &str = "Game Portal | Paramètres";

/// Options affichées hors salon privé (ex. salon d'inscription) : comme
/// `public_game_options` mais SANS le mot de passe. Ce salon est visible de tout
/// le rôle du jeu ; le mot de passe ne doit apparaître qu'au salon privé des
/// inscrits.
fn registration_options(
    config: &std::collections::HashMap<String, String>,
    schema: &[crate::api_client::TemplateField],
) -> Vec<String> {
    lignes_reglages(config, schema, false)
}

/// Construit la carte des paramètres (un ou plusieurs embeds, un par chunk).
/// Bornée à 10 embeds (limite Discord par message).
fn build_options_embeds(
    game_name: &str,
    server_name: &str,
    options: &[String],
) -> Vec<CreateEmbed> {
    let mut chunks = chunk_options(options);
    if chunks.is_empty() {
        chunks.push("_Aucune option publique configurée._".into());
    }
    chunks.truncate(10);
    chunks
        .into_iter()
        .enumerate()
        .map(|(index, chunk)| {
            let mut embed = CreateEmbed::new()
                .title(if index == 0 {
                    format!("⚙️ Paramètres — {game_name} · {server_name}")
                } else {
                    "⚙️ Paramètres (suite)".to_string()
                })
                .field("Options de la partie", chunk, false)
                .color(0x2ecc71)
                .footer(CreateEmbedFooter::new(OPTIONS_FOOTER));
            if index == 0 {
                embed = embed.description("Réglages actuels de ce serveur.");
            }
            embed
        })
        .collect()
}

/// Extrait l'ID de serveur du topic d'un salon de session. Les salons
/// d'inscription et privés portent tous deux `... | session:{id} | ...` dans
/// leur topic (posé à la création). `None` si le salon n'est pas un salon de
/// session.
pub(crate) fn server_id_from_topic(topic: &str) -> Option<&str> {
    let after = topic.split("session:").nth(1)?;
    let id = after.split([' ', '|']).next()?.trim();
    (!id.is_empty()).then_some(id)
}

/// Session de jeu du salon courant, retrouvee par le sujet du salon.
///
/// Le sujet porte `session:{id}` depuis la creation et ne change jamais, meme
/// quand le salon est renomme : c'est le seul repere fiable.
pub(crate) async fn session_du_salon(ctx: &Context, salon: ChannelId) -> Option<String> {
    let sujet = salon.to_channel(&ctx).await.ok()?.guild()?.topic?;
    server_id_from_topic(&sujet).map(str::to_string)
}

/// Construit les embeds de paramètres à afficher en réponse ÉPHÉMÈRE à la
/// commande `/game parametres`, à partir du salon d'où elle est lancée.
///
/// Contextuel : dans le salon privé des inscrits (topic `| private`) le mot de
/// passe est inclus (comme à la révélation) ; ailleurs (salon d'inscription) il
/// est masqué. Retourne un message d'erreur affichable si le salon n'est pas un
/// salon de session.
pub async fn params_embeds_for_channel(
    ctx: &Context,
    api: &ApiClient,
    channel: ChannelId,
) -> Result<Vec<CreateEmbed>, &'static str> {
    let topic = match channel.to_channel(&ctx.http).await {
        Ok(ch) => ch.guild().and_then(|g| g.topic),
        Err(_) => None,
    }
    .ok_or(
        "Utilise cette commande dans le salon d'un jeu (inscription ou salon privé des inscrits).",
    )?;

    let server_id =
        server_id_from_topic(&topic).ok_or("Ce salon n'est pas rattaché à un serveur de jeu.")?;
    let is_private = topic.contains("| private");

    let detail = api
        .get_game_server(server_id)
        .await
        .map_err(|_| "Serveur introuvable.")?;
    let template = api.get_game_template(&detail.server.template_id).await.ok();
    Ok(build_options_embeds_for_server(
        &detail.server,
        template.as_ref(),
        &detail.config,
        is_private,
    ))
}

/// Decoupe les options en blocs tenant dans la VALEUR d'un champ d'embed
/// Discord, dont la limite dure est 1024 caracteres. On borne a 1000 pour
/// garder une marge (chaque option est deja une ligne markdown courte).
fn chunk_options(options: &[String]) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for line in options {
        if !current.is_empty() && current.len() + line.len() + 1 > 1000 {
            chunks.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(line);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

#[allow(dead_code)]
fn public_cover_url(path: Option<&str>) -> Option<String> {
    public_cover_url_for_status(path, "running")
}

/// Jaquette correspondant a l'etat ANNONCE de la session.
///
/// L'etat vient de l'API (`display_state`), qui l'a calcule a partir de la
/// fenetre horaire ET du conteneur — voir `session_state` cote domaine. Le bot
/// ne rejoue pas cette regle : quand chacun avait la sienne, Discord et le site
/// racontaient la meme session differemment.
///
/// Repli sur le statut brut si l'API ne renseigne pas l'etat (reponse d'une
/// version anterieure) : mieux vaut l'ancienne approximation qu'aucune image.
fn public_cover_url_for_status(path: Option<&str>, etat_ou_statut: &str) -> Option<String> {
    let path = path?.trim();
    if path.is_empty() {
        return None;
    }
    let full_url = if path.starts_with("https://") || path.starts_with("http://") {
        path.to_string()
    } else {
        let base = std::env::var("WEB_FRONT_URL").ok()?;
        let base = base.trim().trim_end_matches('/');
        if base.is_empty() {
            return None;
        }
        format!("{base}/{}", path.trim_start_matches('/'))
    };

    let suffixe = match etat_ou_statut {
        // Etats de session (source de verite).
        "open" => "",
        "waiting" => "_attente",
        "closed" => "_offline",
        // Repli : statuts bruts du conteneur.
        "running" => "",
        "scheduled" | "starting" => "_attente",
        _ => "_offline",
    };

    if suffixe.is_empty() {
        return Some(full_url);
    }

    let dot = full_url.rfind('.')?;
    let base = strip_status_suffix(&full_url[..dot]);
    let ext = &full_url[dot..];

    // Les jaquettes livrees dans `web/public/imgs/` portent le suffixe
    // `_attente` / `_offline`.
    Some(format!("{base}{suffixe}{ext}"))
}

/// Etat a utiliser pour choisir la jaquette d'un serveur.
fn etat_affiche(server: &GameServer) -> &str {
    server
        .display_state
        .as_deref()
        .unwrap_or(server.status.as_str())
}

pub fn format_registration_error_content(is_register: bool, err: &str) -> String {
    let action = if is_register {
        "Inscription"
    } else {
        "Désinscription"
    };
    format!("❌ {action} impossible : {err}")
}

pub fn format_registration_ack_content(is_register: bool) -> &'static str {
    if is_register {
        "Inscription enregistrée"
    } else {
        "Désinscription enregistrée"
    }
}

pub fn format_owner_only_reveal_error() -> &'static str {
    "⛔ Seul le propriétaire du serveur peut révéler son adresse."
}

pub async fn execute_reveal_ip_logic(
    api: &ApiClient,
    server_id: &str,
    user_id: &str,
) -> Result<(crate::api_client::RevealRequest, String), String> {
    let detail = api
        .get_game_server(server_id)
        .await
        .map_err(|e| format!("❌ Serveur introuvable : {e}"))?;
    if detail.server.owner_user_id != user_id {
        return Err(format_owner_only_reveal_error().into());
    }
    let outcome = api
        .request_reveal_ip(server_id, user_id)
        .await
        .map_err(|e| format!("❌ Ouverture impossible : {e}"))?;
    let game_name = api
        .get_game_template(&detail.server.template_id)
        .await
        .map(|t| t.name)
        .unwrap_or_else(|_| "Le serveur".into());
    Ok((outcome, game_name))
}

pub fn strip_status_suffix(base: &str) -> &str {
    for suffix in ["_attente", "_waiting", "_offline"] {
        if let Some(stripped) = base.strip_suffix(suffix) {
            return stripped;
        }
    }
    base
}

async fn grant_session_role(
    ctx: &Context,
    component: &ComponentInteraction,
    server_id: &str,
    game_name: &str,
) {
    let Some(guild_id) = component.guild_id else {
        return;
    };
    let expected = session_role_name(game_name, server_id);
    let role_id = match guild_id.roles(&ctx.http).await {
        Ok(roles) => roles.values().find(|r| r.name == expected).map(|r| r.id),
        Err(e) => {
            tracing::warn!(error = %e, server_id, "game-portal: lecture roles impossible");
            None
        }
    };
    let Some(role_id) = role_id else {
        tracing::warn!(
            server_id,
            role = expected,
            "game-portal: role de session introuvable"
        );
        return;
    };
    match guild_id.member(&ctx.http, component.user.id).await {
        Ok(member) => {
            if let Err(e) = member.add_role(&ctx.http, role_id).await {
                tracing::warn!(error = %e, server_id, user = %component.user.id, "game-portal: attribution role de session impossible");
            }
        }
        Err(e) => tracing::warn!(error = %e, server_id, "game-portal: membre introuvable"),
    }
}

async fn on_reveal_ip_component(
    api: &ApiClient,
    ctx: &Context,
    component: &ComponentInteraction,
    server_id: &str,
) {
    // Defer ephemere IMMEDIAT : le demarrage du conteneur (allocation de ports,
    // reseau, pull d'image, start) depasse largement les 3 s d'ack imposees par
    // Discord. Sans ce defer, l'interaction echoue en « n'a pas repondu a
    // temps » avant meme la fin de l'appel API.
    if let Err(e) = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, server_id, "game-portal: defer reveal-ip impossible");
        return;
    }

    let (outcome, game_name) =
        match execute_reveal_ip_logic(api, server_id, &component.user.id.to_string()).await {
            Ok(res) => res,
            Err(err_msg) => {
                edit_deferred(ctx, component, err_msg).await;
                return;
            }
        };

    // Accuse ephemere au proprietaire.
    let minutes = outcome.delay_minutes;
    edit_deferred(ctx, component, format_reveal_ack(outcome.started, minutes)).await;

    // Annonce publique dans le panneau d'inscription : tout le monde voit que la
    // session ouvre bientot. L'adresse, elle, ne parait qu'au salon prive a
    // l'echeance (publiee par le worker reveal-ip).
    let _ = component
        .channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(build_opening_soon_embed(&game_name, minutes)),
        )
        .await;
}

/// Edite la reponse ephemere DEJA deferee (voir le `Defer` en tete de
/// `on_reveal_ip_component`). A n'appeler qu'apres ce defer.
async fn edit_deferred(
    ctx: &Context,
    component: &ComponentInteraction,
    content: impl Into<String>,
) {
    let _ = component
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new().content(content.into()),
        )
        .await;
}

// ── Panneau ──

/// Composants du panneau d'inscription.
///
/// Les boutons de liaison de compte de jeu (ID Steam / ID Xbox) ont ete retires
/// avec le catalogue Palworld : les hauts faits sont desormais Discord. Le
/// backend de liaison reste en place (routes, table `game_player_links`) — les
/// remettre revient a rajouter une rangee de boutons ici.
pub fn panel_rows(server_id: &str, ip_revealed: bool) -> Vec<CreateActionRow> {
    let mut buttons = vec![
        CreateButton::new(format!("{REGISTER_PREFIX}{server_id}"))
            .label("Je m'inscris")
            .emoji('✅')
            .style(ButtonStyle::Success),
        CreateButton::new(format!("{UNREGISTER_PREFIX}{server_id}"))
            .label("Me désinscrire")
            .emoji('❌')
            .style(ButtonStyle::Secondary),
    ];
    if !ip_revealed {
        buttons.push(
            CreateButton::new(format!("{REVEAL_IP_PREFIX}{server_id}"))
                .label("Révéler l'adresse IP")
                .emoji('🔓')
                .style(ButtonStyle::Danger),
        );
    }

    vec![CreateActionRow::Buttons(buttons)]
}

pub fn build_panel_embed(
    game_name: &str,
    server_name: &str,
    inscrits: &[String],
    ip_reveal_at: Option<&str>,
    ip_revealed: bool,
    public_host: Option<&str>,
    host_port: Option<u16>,
) -> CreateEmbed {
    let inscrits_txt = if inscrits.is_empty() {
        "_Personne pour l'instant — sois le premier !_".to_string()
    } else {
        inscrits
            .iter()
            .map(|u| format!("<@{u}>"))
            .collect::<Vec<_>>()
            .join(", ")
    };

    let ip_txt = if ip_revealed {
        match (public_host.filter(|h| !h.trim().is_empty()), host_port) {
            (Some(host), Some(p)) => format!("**Serveur ouvert !** `{host}:{p}`"),
            _ => "**Adresse indisponible : configuration incomplete.**".to_string(),
        }
    } else {
        match ip_reveal_at {
            Some(d) => {
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(d) {
                    let ts = dt.timestamp();
                    format!("🔒 Masquee — revelee le <t:{ts}:F> (<t:{ts}:R>)")
                } else {
                    format!("🔒 Masquee — revelee le **{}**", &d[..10.min(d.len())])
                }
            }
            None => "🔒 Masquee".to_string(),
        }
    };

    CreateEmbed::new()
        .title(format!("🎮 {game_name} — {server_name}"))
        .description(
            "Un serveur de jeu est en preparation. Inscris-toi pour etre prevenu a l'ouverture !",
        )
        .field(
            format!("Inscrits ({})", inscrits.len()),
            inscrits_txt,
            false,
        )
        .field("Adresse (IP)", ip_txt, false)
        .field(
            "Réglages",
            "Tape `/game parametres` pour voir tous les réglages du serveur (réponse privée).",
            false,
        )
        .color(0x5865f2)
        .footer(CreateEmbedFooter::new("Game Portal | Nexus"))
        .timestamp(serenity::model::Timestamp::now())
}

/// Panneau du salon d'inscription : il indique l'ouverture, mais ne contient
/// jamais l'adresse, meme apres sa revelation dans le salon prive.
fn build_public_panel_embed(
    game_name: &str,
    server_name: &str,
    inscrits: &[String],
    ip_reveal_at: Option<&str>,
    ip_revealed: bool,
    cover_url: Option<&str>,
) -> CreateEmbed {
    let mut embed = build_panel_embed(
        game_name,
        server_name,
        inscrits,
        ip_reveal_at,
        ip_revealed,
        None,
        None,
    );
    if let Some(url) = cover_url {
        embed = embed.image(url);
    }
    embed
}

pub fn format_reveal_ack(started: bool, delay_minutes: i64) -> String {
    let debut = if started {
        "🚀 Le serveur démarre."
    } else {
        "🚀 Le serveur est déjà en ligne."
    };
    format!("{debut} L'adresse de connexion sera révélée dans le salon privé des inscrits dans **{delay_minutes} minute(s)**.")
}

pub fn build_opening_soon_embed(game_name: &str, minutes: i64) -> CreateEmbed {
    CreateEmbed::new()
        .title(format!("⏳ {game_name} ouvre bientôt !"))
        .description(format!(
            "Le serveur démarre. L'adresse de connexion sera révélée dans le **salon privé des inscrits** dans **{minutes} minute(s)**.\n\nPas encore inscrit ? Clique sur **Je m'inscris** ci-dessus."
        ))
        .color(0x5865f2)
        .footer(CreateEmbedFooter::new("Game Portal | Nexus"))
        .timestamp(serenity::model::Timestamp::now())
}

pub fn build_private_reveal_card(
    game_name: &str,
    server_name: &str,
    public_host: Option<&str>,
    host_port: Option<u16>,
    cover_url: Option<&str>,
) -> CreateEmbed {
    let address = match (public_host.filter(|h| !h.trim().is_empty()), host_port) {
        (Some(host), Some(port)) => format!("`{host}:{port}`"),
        _ => "_Adresse indisponible, contacte le propriétaire._".to_string(),
    };
    let mut card = CreateEmbed::new()
        .title(format!("🎮 {game_name} — {server_name}"))
        .description(format!(
            "🔓 **Serveur ouvert**\nConnexion : {address}\n\nTape `/game parametres` ici pour voir tous les réglages (mot de passe inclus)."
        ))
        .color(0x5865f2)
        .footer(CreateEmbedFooter::new("Game Portal | Accès privé"))
        .timestamp(serenity::model::Timestamp::now());
    if let Some(url) = cover_url {
        card = card.image(url);
    }
    card
}

pub fn format_daily_ping_content(role_id: RoleId, game_name: &str, when: &str) -> String {
    format!("<@&{role_id}> Le serveur **{game_name}** ouvre {when} ! Inscris-toi sur le panneau.")
}

pub fn format_when_timestamp(ip_reveal_at: Option<&str>) -> String {
    match ip_reveal_at.and_then(|d| chrono::DateTime::parse_from_rfc3339(d).ok()) {
        Some(dt) => format!("<t:{}:R>", dt.timestamp()),
        None => "bientôt".to_string(),
    }
}

// ── Consumer d'evenements ──

/// Spawn le consumer durable de la stream Redis. Appele une fois au `ready`.
///
/// Sans REDIS_URL joignable, `listen_stream_group` boucle en reconnexion : les
/// salons ne sont simplement pas crees, le reste du bot fonctionne.
pub fn spawn(ctx: Context, api: Arc<ApiClient>) {
    tokio::spawn(async move {
        let consumer = crate::event_bus::default_consumer_name();
        crate::event_bus::listen_stream_group(
            "nexus-bot-game-portal".to_string(),
            consumer,
            move |payload_json| {
                let ctx = ctx.clone();
                let api = api.clone();
                async move { handle_event(&ctx, &api, &payload_json).await }
            },
        )
        .await;
    });
}

/// Rattrape les evenements Redis manques pendant que le bot etait hors ligne.
/// Les serveurs deja munis de salons sortent immediatement dans `on_started` ;
/// l'operation est donc idempotente et peut etre rejouee a chaque `ready`.
pub fn reconcile(ctx: Context, api: Arc<ApiClient>, guild_ids: Vec<GuildId>) {
    tokio::spawn(async move {
        for guild_id in guild_ids {
            let guild_key = guild_id.to_string();
            let servers = match api.list_game_servers(&guild_key).await {
                Ok(servers) => servers,
                Err(e) => {
                    tracing::warn!(error = %e, guild = %guild_id, "game-portal: reconciliation impossible");
                    continue;
                }
            };

            for server in servers {
                if matches!(server.status.as_str(), "scheduled" | "starting" | "running") {
                    on_started(&ctx, &api, guild_id, &server.id).await;
                }
                if server.status == "scheduled" && !server.ip_revealed {
                    if let Some(reveal_at) = server.ip_reveal_at.as_deref() {
                        let (game_name, _) = game_name_and_role(&api, &server).await;
                        schedule_opening_soon(
                            ctx.clone(),
                            guild_id,
                            server.id.clone(),
                            game_name,
                            reveal_at.to_string(),
                        );
                    }
                }
            }
        }
    });
}

pub fn parse_portal_event(payload_json: &str) -> Option<(String, String, u64)> {
    let env = serde_json::from_str::<serde_json::Value>(payload_json).ok()?;
    let event = env.get("event").and_then(|v| v.as_str())?.to_string();
    let data = env.get("data")?;
    let server_id = data.get("server_id").and_then(|v| v.as_str())?.to_string();
    let guild_id = data
        .get("guild_id")
        .and_then(|v| v.as_str())?
        .parse::<u64>()
        .ok()?;
    Some((event, server_id, guild_id))
}

/// Extrait le preavis d'un evenement de redemarrage.
///
/// Un preavis illisible retombe sur zero minute plutot que de faire disparaitre
/// l'annonce : mieux vaut un message imprecis que pas de message du tout quand
/// le serveur va couper.
pub fn parse_restart_warning(payload_json: &str) -> (u16, Option<String>) {
    let Ok(env) = serde_json::from_str::<serde_json::Value>(payload_json) else {
        return (0, None);
    };
    let Some(data) = env.get("data") else {
        return (0, None);
    };
    let minutes = data
        .get("minutes_left")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
        .min(u16::MAX as u64) as u16;
    let restart_at = data
        .get("restart_at")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    (minutes, restart_at)
}

/// Annonce Discord d'un redemarrage a venir.
///
/// Elle double le message envoye DANS le jeu, et ne le remplace pas : le
/// message RCON touche ceux qui jouent, celui-ci touche ceux qui s'appretent a
/// se connecter et trouveraient sinon porte close sans explication.
pub fn build_restart_warning_content(
    role_id: Option<RoleId>,
    game_name: &str,
    minutes_left: u16,
    restart_at: Option<&str>,
) -> String {
    let mention = match role_id {
        Some(id) => format!("<@&{id}> "),
        None => String::new(),
    };
    // Horodatage Discord : chacun le lit dans son propre fuseau, ce qu'une
    // heure ecrite en dur ne permet pas.
    let quand = restart_at
        .and_then(|iso| chrono::DateTime::parse_from_rfc3339(iso).ok())
        .map(|t| format!(" (<t:{}:t>)", t.timestamp()))
        .unwrap_or_default();
    format!(
        "🔄 {mention}**{game_name}** redemarre dans **{minutes_left} minutes**{quand}.\n\
         Mettez-vous a l'abri et sauvegardez votre progression."
    )
}

/// Annonce de fin de redemarrage. Sans elle, les joueurs prevenus du
/// redemarrage n'ont aucun moyen de savoir quand revenir.
pub fn build_restarted_content(role_id: Option<RoleId>, game_name: &str) -> String {
    let mention = match role_id {
        Some(id) => format!("<@&{id}> "),
        None => String::new(),
    };
    format!("✅ {mention}**{game_name}** est de nouveau en ligne.")
}

/// Reconnait une annonce de redemarrage deja postee par le bot.
///
/// Le prefixe seul ne suffit pas : d'autres messages du portail commencent par
/// une coche. On exige donc le prefixe ET la tournure propre a ces deux
/// annonces, pour ne jamais emporter un message voisin.
pub fn est_annonce_de_redemarrage(contenu: &str) -> bool {
    (contenu.starts_with("🔄 ") && contenu.contains("redemarre dans"))
        || (contenu.starts_with("✅ ") && contenu.contains("est de nouveau en ligne"))
}

/// Efface les annonces de redemarrage precedentes du salon.
///
/// Un serveur qui redemarre toutes les six heures laissait deux messages par
/// cycle — « redemarre dans quinze minutes », puis « de nouveau en ligne ».
/// Au bout de quelques jours le salon de session ne contenait plus que cela,
/// et la conversation des joueurs disparaissait dessous.
///
/// On ne garde donc que la derniere annonce : les precedentes sont supprimees
/// juste AVANT de poster la nouvelle. Dans l'autre ordre on effacerait le
/// message qu'on vient d'envoyer.
///
/// Best-effort : un echec de suppression (message trop vieux, permission
/// retiree) ne doit jamais empecher l'annonce elle-meme de partir. Prevenir
/// d'un redemarrage compte plus que la proprete du salon.
async fn purger_annonces_de_redemarrage(ctx: &Context, salon: ChannelId) {
    let bot_id = ctx.cache.current_user().id;
    let anciennes: Vec<MessageId> = match salon
        .messages(&ctx.http, GetMessages::new().limit(50))
        .await
    {
        Ok(messages) => messages
            .into_iter()
            .filter(|m| m.author.id == bot_id && est_annonce_de_redemarrage(&m.content))
            .map(|m| m.id)
            .collect(),
        Err(e) => {
            tracing::warn!(error = %e, "game-portal: lecture du salon pour purge impossible");
            return;
        }
    };
    for id in anciennes {
        let _ = salon.delete_message(&ctx.http, id).await;
    }
}

/// Poste une annonce dans le salon de session du serveur.
async fn annoncer_dans_la_session(
    ctx: &Context,
    api: &ApiClient,
    server_id: &str,
    contenu: impl FnOnce(Option<RoleId>, &str) -> String,
) {
    let Ok(detail) = api.get_game_server(server_id).await else {
        return;
    };
    let Some(text_ch) = parse_channel(detail.server.text_channel_id.as_ref()) else {
        return;
    };
    purger_annonces_de_redemarrage(ctx, text_ch).await;
    let (game_name, role_id) = game_name_and_role(api, &detail.server).await;
    let _ = text_ch
        .send_message(
            &ctx.http,
            CreateMessage::new().content(contenu(role_id, &game_name)),
        )
        .await;
}

async fn handle_event(ctx: &Context, api: &ApiClient, payload_json: &str) {
    let Some((event, server_id, guild_id)) = parse_portal_event(payload_json) else {
        return;
    };

    use platform_core::nexus::ports::outbound::events::game_events as ev;
    match event.as_str() {
        // Programmation ET demarrage creent les salons/panneau. La programmation
        // ouvre les inscriptions a l'avance ; le garde anti-doublon de
        // `on_started` evite qu'un demarrage ulterieur ne recree quoi que ce soit.
        ev::SERVER_SCHEDULED | ev::SERVER_STARTED => {
            on_started(ctx, api, GuildId::new(guild_id), &server_id).await;
            if let Ok(detail) = api.get_game_server(&server_id).await {
                if detail.server.status == "scheduled" && !detail.server.ip_revealed {
                    if let Some(reveal_at) = detail.server.ip_reveal_at.as_deref() {
                        let (game_name, _) = game_name_and_role(api, &detail.server).await;
                        schedule_opening_soon(
                            ctx.clone(),
                            GuildId::new(guild_id),
                            server_id,
                            game_name,
                            reveal_at.to_string(),
                        );
                    }
                }
            }
        }
        ev::SERVER_STOPPED => on_stopped(ctx, api, &server_id).await,
        ev::SERVER_DELETED => on_deleted(ctx, api, &server_id, payload_json).await,
        ev::SESSION_CHANNELS_RENAMED => renommer_les_salons(ctx, api, &server_id).await,
        ev::SESSION_ANNOUNCEMENT_ABANDONED => {
            signaler_abandon_d_annonce(ctx, api, payload_json).await
        }
        // Reprise : la sequence est la meme qu'a l'ouverture, et le garde
        // « deja annoncee » la rend sans effet sur une session deja servie.
        ev::SESSION_ANNOUNCEMENT_RETRY => publier_annonce_puis_panneau(ctx, api, &server_id).await,
        ev::IP_REVEAL => on_ip_reveal(ctx, api, &server_id).await,
        ev::DAILY_PING => on_daily_ping(ctx, api, &server_id).await,
        ev::SERVER_RESTART_WARNING => {
            // Le preavis et l'heure viennent de l'API : le bot ne recalcule pas
            // un creneau dont le fuseau vit ailleurs.
            let (minutes_left, restart_at) = parse_restart_warning(payload_json);
            annoncer_dans_la_session(ctx, api, &server_id, |role, nom| {
                build_restart_warning_content(role, nom, minutes_left, restart_at.as_deref())
            })
            .await;
        }
        ev::SERVER_RESTARTED => {
            annoncer_dans_la_session(ctx, api, &server_id, build_restarted_content).await;
        }
        _ => {}
    }

    // Un serveur qui s'allume ou s'eteint change le compte affiche : on
    // rafraichit sans attendre le prochain passage periodique, comme le
    // compteur vocal reagit a une arrivee en salon plutot que de la decouvrir
    // dix minutes plus tard.
    //
    // Le renommage n'a lieu que si le nom change vraiment : un evenement qui
    // ne deplace aucun chiffre ne consomme donc pas le quota Discord.
    if matches!(
        event.as_str(),
        ev::SERVER_SCHEDULED | ev::SERVER_STARTED | ev::SERVER_STOPPED | ev::SERVER_DELETED
    ) {
        crate::compteurs::rafraichir(ctx, api, GuildId::new(guild_id)).await;
    }
}

// ── Helpers partages ──

/// Resout le role Discord a pinguer pour le template d'un serveur.
///
/// Un role configure explicitement pour le template reste prioritaire. A
/// defaut, on reutilise le role deja cree par le module "Jeux
/// mentionnables" : d'abord par nom de template, puis par slug/base de slug
/// (`minecraft-vanilla` retrouve ainsi le jeu `Minecraft`).
pub(crate) async fn resolve_role(
    api: &ApiClient,
    guild_id: &str,
    slug: &str,
    game_name: &str,
) -> Option<RoleId> {
    let configured = api
        .list_template_settings(guild_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .find(|s| s.template_slug == slug)
        .and_then(|s| s.discord_role_id)
        .and_then(|r| r.parse::<u64>().ok())
        .map(RoleId::new);
    if configured.is_some() {
        return configured;
    }

    let slug_base = slug.split(['-', '_']).next().unwrap_or(slug);
    for candidate in [game_name, slug_base, slug] {
        if let Ok(Some(game)) = api.get_game_by_name(guild_id, candidate).await {
            if let Some(role_id) = game
                .role_id
                .as_deref()
                .and_then(|role| role.parse::<u64>().ok())
                .map(RoleId::new)
            {
                return Some(role_id);
            }
        }
    }
    None
}

/// Nom lisible du jeu + role a pinguer, depuis le template du serveur.
pub(crate) async fn game_name_and_role(
    api: &ApiClient,
    server: &GameServer,
) -> (String, Option<RoleId>) {
    let template = api.get_game_template(&server.template_id).await.ok();
    let game_name = template
        .as_ref()
        .map(|t| t.name.clone())
        .unwrap_or_else(|| "Jeu".into());
    let role_id = match template.as_ref().map(|t| t.slug.clone()) {
        Some(slug) => resolve_role(api, &server.guild_id, &slug, &game_name).await,
        None => None,
    };
    (game_name, role_id)
}

pub fn build_options_embeds_for_server(
    server: &GameServer,
    template: Option<&crate::api_client::GameTemplate>,
    config: &std::collections::HashMap<String, String>,
    is_private: bool,
) -> Vec<CreateEmbed> {
    let game_name = template
        .map(|t| t.name.clone())
        .unwrap_or_else(|| "Jeu".into());
    let schema = template
        .map(|t| t.config_schema.as_slice())
        .unwrap_or_default();
    let options = if is_private {
        public_game_options(config, schema)
    } else {
        registration_options(config, schema)
    };
    build_options_embeds(&game_name, &server.name, &options)
}

pub(crate) fn parse_channel(id: Option<&String>) -> Option<ChannelId> {
    id.and_then(|s| s.parse::<u64>().ok()).map(ChannelId::new)
}

/// Nom de la categorie creee au premier demarrage si aucune n'est configuree.
const DEFAULT_SESSION_CATEGORY: &str = "Sessions de jeu";

/// Resout la categorie ou creer les salons de session, en la creant au besoin.
///
/// Trois etapes, de la moins couteuse a la plus couteuse :
///   1. `session_category_id` deja en config -> on verifie que la categorie
///      existe encore et qu'elle est bien de type Category ;
///   2. sinon, on adopte une categorie existante portant le nom attendu (cas
///      d'un admin qui l'a creee a la main) ;
///   3. sinon, on la cree.
///
/// Dans les cas 2 et 3, l'ID est PERSISTE via `set_config` : les demarrages
/// suivants sortent a l'etape 1 sans aucun appel Discord supplementaire.
/// C'est ce que sentinel-bot ne fait pas (`help_panel` recherche la categorie
/// par nom a chaque boot), d'ou une recreation si la categorie est renommee.
async fn ensure_session_category(
    ctx: &Context,
    api: &ApiClient,
    guild_id: GuildId,
    cfg: &std::collections::HashMap<String, String>,
) -> Option<ChannelId> {
    let guild_key = guild_id.to_string();

    // 1. Config existante — on ne fait confiance qu'apres verification.
    if let Some(id) = cfg
        .get("session_category_id")
        .and_then(|s| s.parse::<u64>().ok())
        .map(ChannelId::new)
    {
        match id.to_channel(&ctx).await {
            Ok(ch) => {
                if ch.guild().map(|g| g.kind) == Some(ChannelType::Category) {
                    return Some(id);
                }
                tracing::warn!(
                    %id,
                    "game-portal: session_category_id ne pointe pas sur une categorie -> resolution"
                );
            }
            Err(e) => {
                // Erreur reseau/rate limit : on garde la valeur configuree
                // plutot que de creer une categorie en double.
                if !is_not_found(&e) {
                    tracing::warn!(error = %e, %id, "game-portal: verification categorie impossible");
                    return Some(id);
                }
                tracing::warn!(%id, "game-portal: categorie de session disparue -> recreation");
            }
        }
    }

    // 2. Adoption d'une categorie existante portant le nom attendu.
    if let Ok(channels) = guild_id.channels(&ctx.http).await {
        if let Some(ch) = channels
            .values()
            .find(|c| c.kind == ChannelType::Category && c.name == DEFAULT_SESSION_CATEGORY)
        {
            persist_category(api, &guild_key, ch.id).await;
            return Some(ch.id);
        }
    }

    // 3. Creation.
    let created = guild_id
        .create_channel(
            &ctx.http,
            CreateChannel::new(DEFAULT_SESSION_CATEGORY).kind(ChannelType::Category),
        )
        .await;
    match created {
        Ok(ch) => {
            tracing::info!(guild = %guild_id, category = %ch.id, "game-portal: categorie de sessions creee");
            persist_category(api, &guild_key, ch.id).await;
            Some(ch.id)
        }
        Err(e) => {
            // Sans categorie les salons sont crees a la racine : degrade mais
            // fonctionnel, on ne bloque pas l'ouverture de session.
            tracing::warn!(error = %e, guild = %guild_id, "game-portal: creation de la categorie impossible");
            None
        }
    }
}

pub(crate) async fn persist_category(api: &ApiClient, guild_id: &str, category: ChannelId) {
    if let Err(e) = api
        .set_config(
            guild_id,
            MODULE_BOT_NAME,
            "session_category_id",
            &category.to_string(),
        )
        .await
    {
        // Non bloquant : la categorie sera re-resolue par son nom au prochain
        // demarrage (etape 2), simplement moins efficacement.
        tracing::warn!(error = %e, guild_id, "game-portal: memorisation de la categorie impossible");
    }
}

/// L'erreur serenity correspond-elle a un 404 Discord ?
fn is_not_found(e: &serenity::Error) -> bool {
    matches!(
        e,
        serenity::Error::Http(serenity::http::HttpError::UnsuccessfulRequest(res))
            if res.status_code == serenity::http::StatusCode::NOT_FOUND
    )
}

/// Le salon existe-t-il encore cote Discord ?
///
/// Passe par le cache puis l'API HTTP. En cas d'erreur autre qu'un 404 (panne
/// reseau, rate limit), on repond `true` : mieux vaut ne rien faire que
/// recreer des salons en double sur une erreur transitoire.
async fn channel_exists(ctx: &Context, channel_id: ChannelId) -> bool {
    match channel_id.to_channel(&ctx).await {
        Ok(_) => true,
        Err(e) if is_not_found(&e) => false,
        Err(e) => {
            tracing::warn!(error = %e, %channel_id, "game-portal: verification du salon impossible");
            true
        }
    }
}

/// Salon prive : @everyone ne voit rien, le role du jeu voit et participe.
///
/// Les permissions accordees dependent du TYPE de salon. Discord refuse une
/// creation (50013) quand l'overwrite accorde une permission que le bot ne
/// possede pas lui-meme : demander CONNECT et SPEAK sur un salon textuel, ou
/// SEND_MESSAGES sur un vocal, expose a un echec pour une permission dont le
/// salon n'a de toute facon aucun usage.
fn build_overwrites(
    guild_id: GuildId,
    role_id: Option<RoleId>,
    kind: ChannelType,
) -> Vec<PermissionOverwrite> {
    // @everyone porte le meme ID que la guild.
    let mut ows = vec![PermissionOverwrite {
        allow: Permissions::empty(),
        deny: Permissions::VIEW_CHANNEL,
        kind: PermissionOverwriteType::Role(RoleId::new(guild_id.get())),
    }];
    if let Some(rid) = role_id {
        let specifiques = if kind == ChannelType::Voice {
            Permissions::CONNECT | Permissions::SPEAK
        } else {
            Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY
        };
        ows.push(PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL | specifiques,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Role(rid),
        });
    }
    ows
}

pub fn build_create_channel_request<'a>(
    name: &'a str,
    kind: ChannelType,
    category: Option<ChannelId>,
    overwrites: &[PermissionOverwrite],
    topic: Option<&'a str>,
) -> CreateChannel<'a> {
    let mut b = CreateChannel::new(name)
        .kind(kind)
        .permissions(overwrites.to_vec());
    if let Some(c) = category {
        b = b.category(c);
    }
    if let Some(t) = topic {
        b = b.topic(t);
    }
    b
}

async fn create_channel(
    ctx: &Context,
    guild_id: GuildId,
    name: &str,
    kind: ChannelType,
    category: Option<ChannelId>,
    overwrites: &[PermissionOverwrite],
    topic: Option<&str>,
) -> Option<ChannelId> {
    let premiere = match guild_id
        .create_channel(
            &ctx.http,
            build_create_channel_request(name, kind, category, overwrites, topic),
        )
        .await
    {
        Ok(ch) => return Some(ch.id),
        Err(e) => e,
    };

    // Une categorie Discord plafonne a 50 salons. Le vocal etant cree apres le
    // textuel, c'est lui qui bute en premier sur la limite — le textuel passe
    // et le vocal manque, sans que rien ne le signale.
    //
    // Plutot que d'abandonner la session, on recree hors categorie : le salon
    // est moins bien range mais il existe, ce qui est preferable a une session
    // muette. Le log dit pourquoi.
    if category.is_some() {
        tracing::warn!(
            error = %premiere,
            name,
            ?kind,
            "game-portal: echec creation salon dans la categorie -> nouvel essai hors categorie"
        );
        match guild_id
            .create_channel(
                &ctx.http,
                build_create_channel_request(name, kind, None, overwrites, topic),
            )
            .await
        {
            Ok(ch) => return Some(ch.id),
            Err(e) => {
                tracing::error!(
                    error = %e,
                    name,
                    ?kind,
                    "game-portal: echec creation salon, y compris hors categorie"
                );
                return None;
            }
        }
    }

    tracing::error!(error = %premiere, name, ?kind, "game-portal: echec creation salon");
    None
}

/// Nom de salon Discord valide : minuscules, tirets, sans accents.
fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_dash = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_end_matches('-').to_string();
    if trimmed.is_empty() {
        "serveur".to_string()
    } else {
        trimmed
    }
}

// ── Demarrage d'un serveur -> creation des salons ──

async fn on_started(ctx: &Context, api: &ApiClient, guild_id: GuildId, server_id: &str) {
    let detail = match api.get_game_server(server_id).await {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!(error = %e, server_id, "game-portal: echec lecture serveur");
            return;
        }
    };
    let server = detail.server;
    // Salons deja enregistres : soit l'evenement est rejoue (salon bien vivant,
    // rien a faire), soit le salon a disparu cote Discord et la base ment.
    //
    // On VERIFIE plutot que de faire confiance : un salon peut avoir ete
    // supprime par un wipe de guilde (module guild_backup de sentinel-bot),
    // par un admin a la main, ou par n'importe quel futur nettoyage. Sans cette
    // verification, la garde ci-dessous bloquerait la recreation pour toujours
    // et le game-portal resterait casse en silence.
    if let Some(existing) = parse_channel(server.text_channel_id.as_ref()) {
        if channel_exists(ctx, existing).await {
            // Salon existant : on en profite pour forcer la mise a jour du panel
            // s'il y a eu un changement de code ou une desynchro.
            let (game_name, _) = game_name_and_role(api, &server).await;
            let cover_url = api
                .get_game_template(&server.template_id)
                .await
                .ok()
                .and_then(|template| {
                    public_cover_url_for_status(
                        template.cover_image_url.as_deref(),
                        etat_affiche(&server),
                    )
                });
            let registered_user_ids: Vec<String> = api
                .list_server_registrations(server_id)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|r| r.user_id)
                .collect();
            let embed = build_public_panel_embed(
                &game_name,
                &server.name,
                &registered_user_ids,
                server.ip_reveal_at.as_deref(),
                server.ip_revealed,
                cover_url.as_deref(),
            );
            if let Ok(pins) = existing.pins(&ctx.http).await {
                for mut msg in pins {
                    let is_panel = msg.components.iter().any(|row| {
                        row.components.iter().any(|c| {
                            if let serenity::model::application::ActionRowComponent::Button(b) = c {
                                if let serenity::all::ButtonKind::NonLink { custom_id, .. } =
                                    &b.data
                                {
                                    return custom_id.starts_with(REGISTER_PREFIX)
                                        && custom_id.contains(server_id);
                                }
                            }
                            false
                        })
                    });
                    if is_panel {
                        let _ = msg
                            .edit(
                                &ctx.http,
                                serenity::builder::EditMessage::new()
                                    .embed(embed)
                                    .components(panel_rows(server_id, server.ip_revealed)),
                            )
                            .await;
                        break;
                    }
                }
            }
            return;
        }
        // Salon fantome : on libere les references avant de recreer, sinon le
        // claim `set_session_channels` plus bas refuserait (garde anti-doublon).
        tracing::warn!(
            server_id,
            channel_id = %existing,
            "game-portal: salon de session disparu cote Discord -> recreation"
        );
        if let Err(e) = api.set_session_channels(server_id, None, None).await {
            tracing::warn!(error = %e, server_id, "game-portal: echec liberation des salons fantomes");
            return;
        }
    }

    let (game_name, role_id) = game_name_and_role(api, &server).await;

    let cfg = api
        .get_guild_config(&server.guild_id, MODULE_BOT_NAME)
        .await
        .unwrap_or_default();
    let category = ensure_session_category(ctx, api, guild_id, &cfg).await;
    let (nom_inscription, nom_prive, nom_vocal) = noms_des_salons(&server, &game_name, &cfg);

    let session_role = match guild_id
        .create_role(
            &ctx.http,
            EditRole::new()
                .name(session_role_name(&game_name, server_id))
                .colour(Colour::new(0x5865f2))
                .mentionable(false)
                .hoist(false),
        )
        .await
    {
        Ok(role) => role,
        Err(e) => {
            tracing::error!(error = %e, server_id, "game-portal: creation role de session impossible");
            return;
        }
    };

    let text_ch = create_channel(
        ctx,
        guild_id,
        &nom_inscription,
        ChannelType::Text,
        category,
        &build_overwrites(guild_id, role_id, ChannelType::Text),
        Some(&format!(
            "Nexus Game Portal | session:{server_id} | registration"
        )),
    )
    .await;
    let private_text_ch = create_channel(
        ctx,
        guild_id,
        &nom_prive,
        ChannelType::Text,
        category,
        &build_overwrites(guild_id, Some(session_role.id), ChannelType::Text),
        Some(&private_text_topic(server_id)),
    )
    .await;
    let voice_ch = create_channel(
        ctx,
        guild_id,
        &nom_vocal,
        ChannelType::Voice,
        category,
        &build_overwrites(guild_id, Some(session_role.id), ChannelType::Voice),
        None,
    )
    .await;

    let (Some(text_ch), Some(private_text_ch), Some(voice_ch)) =
        (text_ch, private_text_ch, voice_ch)
    else {
        if let Some(ch) = text_ch {
            let _ = ch.delete(&ctx.http).await;
        }
        if let Some(ch) = private_text_ch {
            let _ = ch.delete(&ctx.http).await;
        }
        if let Some(ch) = voice_ch {
            let _ = ch.delete(&ctx.http).await;
        }
        let _ = guild_id.delete_role(&ctx.http, session_role.id).await;
        return;
    };

    // Enregistrement cote API : le claim sert de garde anti-doublon. Si le
    // claim echoue (claimed=false), des salons etaient deja enregistres
    // (evenement rejoue) -> on supprime ceux qu'on vient de creer. Une erreur
    // reseau laisse les salons en place (pas de suppression a tort).
    match api
        .set_session_channels(
            server_id,
            Some(&text_ch.to_string()),
            Some(&voice_ch.to_string()),
        )
        .await
    {
        Ok(false) => {
            let _ = text_ch.delete(&ctx.http).await;
            let _ = private_text_ch.delete(&ctx.http).await;
            let _ = voice_ch.delete(&ctx.http).await;
            let _ = guild_id.delete_role(&ctx.http, session_role.id).await;
            tracing::warn!(
                server_id,
                "game-portal: salons deja enregistres (evenement rejoue) -> doublons supprimes"
            );
            return;
        }
        Ok(true) => {}
        Err(e) => {
            tracing::warn!(error = %e, server_id, "game-portal: echec enregistrement salons (salons conserves)");
        }
    }

    let registered_user_ids: Vec<String> = api
        .list_server_registrations(server_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.user_id)
        .collect();
    for user_id in &registered_user_ids {
        if let Ok(user_num) = user_id.parse::<u64>() {
            if let Ok(member) = guild_id.member(&ctx.http, user_num).await {
                let _ = member.add_role(&ctx.http, session_role.id).await;
            }
        }
    }

    publier_annonce_puis_panneau(ctx, api, server_id).await;
}

/// Publie l'annonce d'Atrium, PUIS le panneau d'inscription.
///
/// L'ORDRE EST LA REGLE, ET L'ANNONCE EST UN PREALABLE. Quand Atrium ne peut
/// rien ecrire, on ne publie RIEN — ni annonce, ni panneau — et la reprise
/// repassera. C'est un choix assume : personne ne s'inscrit tant que la panne
/// dure, mais la soiree ne s'ouvre jamais sur un message que personne n'a
/// voulu.
///
/// UNE SEULE IMPLEMENTATION pour l'ouverture et pour la reprise. Deux copies
/// auraient fini par diverger, et la reprise aurait publie un panneau
/// legerement different de celui de l'ouverture.
///
/// Le marquage a lieu DES QUE L'ANNONCE EST PARTIE, avant le panneau : un
/// panneau rate se rejoue sans dommage, une annonce publiee deux fois se voit.

/// Publie le panneau d'inscription dans un salon et l'epingle.
///
/// UNE SEULE IMPLEMENTATION pour l'ouverture d'une session et pour sa remise en
/// etat : un panneau republie par la commande de resynchronisation doit etre
/// identique a celui de l'ouverture, sinon les deux divergeraient au fil des
/// evolutions et personne ne saurait lequel fait foi.
///
/// Rend le nom du jeu et le role du jeu, que l'appelant reutilise pour la
/// mention qui suit.
pub(crate) async fn poster_le_panneau(
    ctx: &Context,
    api: &ApiClient,
    text_ch: ChannelId,
    server: &GameServer,
) -> (String, Option<RoleId>) {
    let (game_name, role_id) = game_name_and_role(api, server).await;
    let cover_url = api
        .get_game_template(&server.template_id)
        .await
        .ok()
        .and_then(|template| {
            public_cover_url_for_status(template.cover_image_url.as_deref(), etat_affiche(server))
        });
    let registered_user_ids: Vec<String> = api
        .list_server_registrations(&server.id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.user_id)
        .collect();

    let embed = build_public_panel_embed(
        &game_name,
        &server.name,
        &registered_user_ids,
        server.ip_reveal_at.as_deref(),
        server.ip_revealed,
        cover_url.as_deref(),
    );
    let msg = text_ch
        .send_message(
            &ctx.http,
            CreateMessage::new()
                .embed(embed)
                .components(panel_rows(&server.id, server.ip_revealed)),
        )
        .await;
    if let Ok(m) = &msg {
        let _ = text_ch.pin(&ctx.http, m.id).await;
    }

    (game_name, role_id)
}
pub(crate) async fn publier_annonce_puis_panneau(ctx: &Context, api: &ApiClient, server_id: &str) {
    let Ok(detail) = api.get_game_server(server_id).await else {
        return;
    };
    let server = detail.server;

    // Deja annoncee : la reprise ne doit pas repasser dessus.
    if server.announcement_posted_at.is_some() {
        return;
    }
    let Some(text_ch) = parse_channel(server.text_channel_id.as_ref()) else {
        return;
    };

    let annonce = match api.annonce_de_session(server_id).await {
        Ok(Some(texte)) => texte,
        Ok(None) => {
            tracing::warn!(
                server_id,
                "game-portal: Atrium indisponible, panneau differe (la reprise repassera)"
            );
            return;
        }
        Err(e) => {
            tracing::error!(
                error = %e,
                server_id,
                "game-portal: annonce refusee, panneau non publie"
            );
            return;
        }
    };

    // LE REGLEMENT VOYAGE AVEC L'ANNONCE, DANS LE MEME MESSAGE.
    //
    // Un cartouche separe plutot que du texte courant : le ton d'Atrium et les
    // regles ne se lisent pas de la meme facon, et les melanger ferait passer
    // le reglement pour une plaisanterie de plus.
    //
    // Le texte est celui de l'exploitant, MOT POUR MOT. Atrium l'a recu comme
    // contexte pour ne rien annoncer qu'il interdise, mais ce qui s'affiche
    // n'est jamais passe par le modele : un reglement reformule est un
    // reglement qui change de sens sans que personne ne s'en apercoive.
    let mut message = CreateMessage::new().content(annonce);
    if let Some(reglement) = server.rules.as_deref().map(str::trim) {
        if !reglement.is_empty() {
            // LE TEXTE PART BRUT, SANS ECHAPPEMENT, ET C'EST VOULU.
            //
            // Une description d'embed rend le Markdown de Discord : gras,
            // italique, listes, titres, citations, blocs de code, liens.
            // L'exploitant peut donc mettre en forme son reglement, et
            // echapper le texte le priverait de tout cela.
            //
            // L'embed apporte en prime une propriete de surete : les mentions
            // qu'il contient ne PINGUENT PAS. Un « @everyone respectez les
            // regles » ecrit dans un reglement ne reveillera donc pas le
            // serveur — ce qui ne serait pas vrai dans le corps du message.
            message = message.embed(
                CreateEmbed::new()
                    .title("📜 Reglement de la soiree")
                    .description(reglement)
                    .colour(Colour::new(0x5865f2)),
            );
        }
    }

    if let Err(e) = text_ch.send_message(&ctx.http, message).await {
        tracing::warn!(error = %e, server_id, "game-portal: annonce non publiee");
        return;
    }
    if let Err(e) = api.marquer_annonce_publiee(server_id).await {
        // L'annonce EST publiee. Ne pas avoir pu l'ecrire en base fera
        // repasser la reprise et republier : desagreable, mais moins grave que
        // d'interrompre ici et de laisser la session sans panneau.
        tracing::warn!(error = %e, server_id, "game-portal: annonce publiee mais non marquee");
    }

    let (game_name, role_id) = poster_le_panneau(ctx, api, text_ch, &server).await;

    if let Some(rid) = role_id {
        let _ = text_ch
            .send_message(
                &ctx.http,
                CreateMessage::new().content(format!(
                    "<@&{rid}> un serveur **{game_name}** ouvre bientot ! Inscris-toi ci-dessus."
                )),
            )
            .await;
    }

    tracing::info!(server_id, "game-portal: annonce puis panneau publies");
}

// ── Arret -> on CONSERVE les salons ──

/// Serveur arrete (mais pas supprime) : les salons de session, le role et le
/// panneau d'inscription sont CONSERVES. Un arret est temporaire — on veut
/// pouvoir redemarrer sans reconstruire salons ni inscriptions, et sans que le
/// role de session (donc l'acces des inscrits) ne saute a chaque pause. La
/// suppression effective des salons n'a lieu qu'a la suppression du jeu
/// (`on_deleted`, evenement `game_server_deleted`).
pub(crate) async fn on_stopped(_ctx: &Context, _api: &ApiClient, server_id: &str) {
    tracing::info!(server_id, "game-portal: session arretee (salons conserves)");
}

/// Renomme les salons DEJA CREES d'une session.
///
/// Sans cela, changer un modele ou un nom libre n'aurait rien fait aux salons
/// existants : le nouveau nom ne serait apparu qu'a la session suivante, et
/// l'administrateur aurait cru le reglage sans effet.
///
/// LE SALON PRIVE SE RETROUVE PAR SON SUJET. Son identifiant n'est pas
/// enregistre — seuls l'inscription et le vocal le sont — et son nom vient
/// justement de changer : le comparer au modele courant serait circulaire.
///
/// Best-effort salon par salon : Discord limite fortement les renommages (deux
/// par salon et par dix minutes). Un refus sur l'un ne doit pas empecher les
/// autres, et l'echec est journalise plutot qu'avale.
async fn renommer_les_salons(ctx: &Context, api: &ApiClient, server_id: &str) {
    let Ok(detail) = api.get_game_server(server_id).await else {
        tracing::warn!(
            server_id,
            "game-portal: serveur introuvable, renommage abandonne"
        );
        return;
    };
    let server = detail.server;
    let (game_name, _) = game_name_and_role(api, &server).await;
    let cfg = api
        .get_guild_config(&server.guild_id, MODULE_BOT_NAME)
        .await
        .unwrap_or_default();

    let (nom_inscription, nom_prive, nom_vocal) = noms_des_salons(&server, &game_name, &cfg);

    let mut cibles: Vec<(ChannelId, String)> = Vec::new();
    if let Some(ch) = parse_channel(server.text_channel_id.as_ref()) {
        cibles.push((ch, nom_inscription));
    }
    if let Some(ch) = parse_channel(server.voice_channel_id.as_ref()) {
        cibles.push((ch, nom_vocal));
    }
    if let Ok(guild_num) = server.guild_id.parse::<u64>() {
        let sujet = private_text_topic(server_id);
        if let Ok(salons) = GuildId::new(guild_num).channels(&ctx.http).await {
            if let Some(id) = salons
                .iter()
                .find(|(_, c)| c.topic.as_deref() == Some(sujet.as_str()))
                .map(|(id, _)| *id)
            {
                cibles.push((id, nom_prive));
            }
        }
    }

    for (salon, nom) in cibles {
        // Ne rien demander quand le nom ne bouge pas : chaque appel consomme le
        // quota de renommage de Discord, et l'epuiser bloquerait les salons qui
        // ont vraiment change.
        let inchange = salons_deja_nomme(ctx, salon, &nom).await;
        if inchange {
            continue;
        }
        if let Err(e) = salon
            .edit(&ctx.http, EditChannel::new().name(nom.clone()))
            .await
        {
            tracing::warn!(error = %e, %salon, nom, "game-portal: renommage refuse");
        }
    }
    tracing::info!(server_id, "game-portal: salons renommes");
}

/// Le salon porte-t-il deja ce nom ? Une lecture ratee repond « non » : mieux
/// vaut une tentative de renommage inutile qu'un renommage jamais fait.
async fn salons_deja_nomme(ctx: &Context, salon: ChannelId, nom: &str) -> bool {
    salon
        .to_channel(&ctx)
        .await
        .ok()
        .and_then(|c| c.guild().map(|g| g.name == nom))
        .unwrap_or(false)
}

/// Signale dans le salon de logs qu'une session n'aura pas son annonce.
///
/// POURQUOI CETTE ALERTE EXISTE. L'annonce d'Atrium precede le panneau
/// d'inscription : tant qu'elle manque, personne ne peut s'inscrire. Passe le
/// plafond de tentatives, la reprise cesse — et sans ce message, une soiree
/// resterait sans panneau, decouverte par les joueurs plutot que par
/// l'exploitant.
///
/// Elle dit aussi COMMENT s'en sortir : le blocage se leve en publiant le
/// panneau a la main ou en recreant la session, pas en attendant.
///
/// Sans salon de logs configure, rien n'est publie. C'est le reglage
/// `log_channel_id` du module, et l'absence de salon est un choix, pas une
/// panne : on ne va pas ecrire ailleurs faute de mieux.
async fn signaler_abandon_d_annonce(ctx: &Context, api: &ApiClient, payload_json: &str) {
    let Some((_, server_id, guild_id)) = parse_portal_event(payload_json) else {
        return;
    };
    let donnees = serde_json::from_str::<serde_json::Value>(payload_json).ok();
    let nom = donnees
        .as_ref()
        .and_then(|env| env.get("data"))
        .and_then(|d| d.get("nom"))
        .and_then(|v| v.as_str())
        .unwrap_or("un serveur")
        .to_string();

    let cfg = api
        .get_guild_config(&guild_id.to_string(), MODULE_BOT_NAME)
        .await
        .unwrap_or_default();
    let Some(salon) = cfg
        .get("log_channel_id")
        .and_then(|s| s.parse::<u64>().ok())
        .map(ChannelId::new)
    else {
        tracing::warn!(
            server_id,
            "game-portal: abandon d'annonce non signale (aucun salon de logs configure)"
        );
        return;
    };

    let message = format!(
        "⚠️ **{nom}** : Atrium n'a pas pu rediger l'annonce d'ouverture apres \
plusieurs tentatives, la reprise s'arrete.\n\
Les salons existent, mais **le panneau d'inscription n'a pas ete publie** : \
personne ne peut s'inscrire.\n\
Verifie la disponibilite d'Atrium et son quota, puis recree la session pour \
relancer l'ouverture."
    );
    if let Err(e) = salon
        .send_message(&ctx.http, CreateMessage::new().content(message))
        .await
    {
        tracing::warn!(error = %e, server_id, "game-portal: alerte d'abandon non publiee");
    }
}
// ── Suppression du jeu -> suppression des salons ──

/// Salons et role a nettoyer, tels que l'evenement de suppression les porte.
///
/// `None` pour un message ancien, emis avant que l'API n'enrichisse la charge
/// utile — encore possible pendant un deploiement, le temps que le stream se
/// vide.
fn parse_deleted_payload(payload_json: &str) -> Option<(Option<String>, Option<String>, String)> {
    let env = serde_json::from_str::<serde_json::Value>(payload_json).ok()?;
    let data = env.get("data")?;
    let template_id = data.get("template_id")?.as_str()?.to_string();
    let lire = |cle: &str| data.get(cle).and_then(|v| v.as_str()).map(str::to_string);
    Some((
        lire("text_channel_id"),
        lire("voice_channel_id"),
        template_id,
    ))
}

async fn on_deleted(ctx: &Context, api: &ApiClient, server_id: &str, payload_json: &str) {
    // LES IDENTIFIANTS VIENNENT DU MESSAGE, PAS D'UNE RELECTURE.
    //
    // Cette fonction commencait par redemander le serveur a l'API. Or la fiche
    // est deja soft-deleted quand l'evenement arrive, et `find_by_id` filtre
    // `deleted_at IS NULL` : la reponse etait un 404, la fonction sortait a sa
    // premiere ligne sans un mot, et les salons du jeu supprime restaient en
    // place pour toujours.
    let (text_channel_id, voice_channel_id, template_id, guild_id) =
        match parse_deleted_payload(payload_json) {
            Some((text, voice, template)) => {
                let guild = crate::game_portal::parse_portal_event(payload_json).map(|(_, _, g)| g);
                (text, voice, template, guild)
            }
            // Message d'avant l'enrichissement : on tente la relecture, qui ne
            // reussira que si la suppression n'est pas encore allee au bout.
            None => match api.get_game_server(server_id).await {
                Ok(detail) => (
                    detail.server.text_channel_id,
                    detail.server.voice_channel_id,
                    detail.server.template_id,
                    detail.server.guild_id.parse::<u64>().ok(),
                ),
                Err(_) => {
                    tracing::warn!(
                        server_id,
                        "game-portal: suppression sans salons connus, rien a nettoyer"
                    );
                    return;
                }
            },
        };

    for ch in [
        parse_channel(text_channel_id.as_ref()),
        parse_channel(voice_channel_id.as_ref()),
    ]
    .into_iter()
    .flatten()
    {
        let _ = ch.delete(&ctx.http).await;
    }

    if let Some(guild_num) = guild_id {
        let guild_id = GuildId::new(guild_num);
        let game_name = api
            .get_game_template(&template_id)
            .await
            .map(|t| t.name)
            .unwrap_or_else(|_| "jeu".into());
        // REPERAGE PAR LE SUJET, ET NON PAR LE NOM.
        //
        // La condition exigeait que le nom du salon corresponde AUSSI au
        // modele courant. Depuis que les noms sont personnalisables, un salon
        // cree sous un ancien modele — ou renomme a la main — ne
        // correspondait plus, et survivait a la suppression du jeu. Le sujet,
        // lui, porte `session:{id}` depuis la creation et ne change jamais.
        let legacy_private_name = legacy_private_text_name(server_id);
        let private_topic = private_text_topic(server_id);
        if let Ok(channels) = guild_id.channels(&ctx.http).await {
            for channel in channels.values().filter(|c| {
                c.topic.as_deref() == Some(private_topic.as_str()) || c.name == legacy_private_name
            }) {
                let _ = channel.delete(&ctx.http).await;
            }
        }
        let role_name = session_role_name(&game_name, server_id);
        if let Ok(roles) = guild_id.roles(&ctx.http).await {
            for role in roles.values().filter(|r| r.name == role_name) {
                let _ = guild_id.delete_role(&ctx.http, role.id).await;
            }
        }
    }
    // Libere les salons cote API (sinon un futur demarrage se croirait rejoue).
    if let Err(e) = api.set_session_channels(server_id, None, None).await {
        tracing::warn!(error = %e, server_id, "game-portal: echec liberation des salons");
    }
    tracing::info!(server_id, "game-portal: session fermee (salons supprimes)");
}

// ── Revelation d'IP ──

async fn on_ip_reveal(ctx: &Context, api: &ApiClient, server_id: &str) {
    let Ok(detail) = api.get_game_server(server_id).await else {
        return;
    };
    let server = detail.server;
    let Some(text_ch) = parse_channel(server.text_channel_id.as_ref()) else {
        return;
    };

    let template = api.get_game_template(&server.template_id).await.ok();
    let game_name = template
        .as_ref()
        .map(|t| t.name.clone())
        .unwrap_or_else(|| "Jeu".into());
    let cover_url = public_cover_url_for_status(
        template
            .as_ref()
            .and_then(|template| template.cover_image_url.as_deref()),
        etat_affiche(&server),
    );

    // Publie l'adresse uniquement dans le salon textuel prive des inscrits.
    if let Ok(guild_num) = server.guild_id.parse::<u64>() {
        let guild_id = GuildId::new(guild_num);
        // Meme raison qu'a la suppression : le sujet identifie le salon, le
        // nom ne le peut plus depuis qu'il est personnalisable.
        let legacy_private_name = legacy_private_text_name(server_id);
        let private_topic = private_text_topic(server_id);
        if let Ok(channels) = guild_id.channels(&ctx.http).await {
            if let Some(private_ch) = channels.values().find(|c| {
                c.topic.as_deref() == Some(private_topic.as_str()) || c.name == legacy_private_name
            }) {
                let card = build_private_reveal_card(
                    &game_name,
                    &server.name,
                    server.public_host.as_deref(),
                    server.host_port,
                    cover_url.as_deref(),
                );
                let _ = private_ch
                    .send_message(&ctx.http, CreateMessage::new().embed(card))
                    .await;
            }
        }
    }

    // Le panneau public indique seulement que le serveur est ouvert.
    let user_ids: Vec<String> = api
        .list_server_registrations(server_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.user_id)
        .collect();
    let embed = build_public_panel_embed(
        &game_name,
        &server.name,
        &user_ids,
        None,
        true,
        cover_url.as_deref(),
    );
    if let Ok(pins) = text_ch.pins(&ctx.http).await {
        if let Some(m) = pins.into_iter().find(|m| !m.embeds.is_empty()) {
            let _ = text_ch
                .edit_message(
                    &ctx.http,
                    m.id,
                    EditMessage::new()
                        .embed(embed)
                        .components(panel_rows(server_id, true)),
                )
                .await;
        }
    }

    tracing::info!(server_id, "game-portal: IP revelee");
}

// ── Ping quotidien ──

async fn on_daily_ping(ctx: &Context, api: &ApiClient, server_id: &str) {
    let Ok(detail) = api.get_game_server(server_id).await else {
        return;
    };
    let server = detail.server;
    let Some(text_ch) = parse_channel(server.text_channel_id.as_ref()) else {
        return;
    };
    let (game_name, role_id) = game_name_and_role(api, &server).await;
    let Some(rid) = role_id else { return };

    let when = format_when_timestamp(server.ip_reveal_at.as_deref());

    let _ = text_ch
        .send_message(
            &ctx.http,
            CreateMessage::new().content(format_daily_ping_content(rid, &game_name, &when)),
        )
        .await;
}

static SCHEDULED_PINGS: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<String>>> =
    std::sync::OnceLock::new();

fn get_scheduled_pings() -> &'static std::sync::Mutex<std::collections::HashSet<String>> {
    SCHEDULED_PINGS.get_or_init(|| std::sync::Mutex::new(std::collections::HashSet::new()))
}

fn schedule_opening_soon(
    ctx: Context,
    guild_id: GuildId,
    server_id: String,
    game_name: String,
    reveal_at: String,
) {
    let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&reveal_at) else {
        return;
    };
    let dt_utc = dt.with_timezone(&chrono::Utc);
    let now = chrono::Utc::now();
    let ping_time = dt_utc - chrono::Duration::hours(1);

    if ping_time > now {
        let mut pings = get_scheduled_pings().lock().unwrap();
        if !pings.insert(server_id.clone()) {
            return;
        }
        drop(pings);

        tokio::spawn(async move {
            let sleep_dur = (ping_time - chrono::Utc::now())
                .to_std()
                .unwrap_or(std::time::Duration::from_secs(0));
            tokio::time::sleep(sleep_dur).await;

            // Le sujet suffit et reste vrai quel que soit le nom : voir la
            // meme correction dans `on_deleted`.
            let channels = guild_id.channels(&ctx.http).await.unwrap_or_default();
            let mut private_ch_id = None;
            for (id, ch) in channels {
                if ch
                    .topic
                    .as_deref()
                    .unwrap_or("")
                    .contains(&format!("session:{server_id}"))
                {
                    private_ch_id = Some(id);
                    break;
                }
            }

            if let Some(ch_id) = private_ch_id {
                let _ = ch_id
                    .send_message(
                        &ctx.http,
                        CreateMessage::new().content(format!(
                            "@everyone Le serveur **{game_name}** ouvre dans moins d'une heure !"
                        )),
                    )
                    .await;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_client::TemplateField;

    fn champ(key: &str, label: &str, group: Option<&str>) -> TemplateField {
        TemplateField {
            key: key.into(),
            label: label.into(),
            group: group.map(str::to_string),
        }
    }

    #[test]
    fn les_reglages_prennent_leur_nom_francais() {
        let schema = vec![champ("SPAWN_MONSTERS", "Spawn monstres", Some("Monde"))];
        let config =
            std::collections::HashMap::from([("SPAWN_MONSTERS".to_string(), "true".to_string())]);

        let lignes = lignes_reglages(&config, &schema, true);
        assert!(lignes.iter().any(|l| l.contains("Spawn monstres")));
        assert!(
            !lignes.iter().any(|l| l.contains("SPAWN_MONSTERS")),
            "la cle technique ne doit plus apparaitre"
        );
        assert!(lignes.iter().any(|l| l.contains("Monde")));
    }

    #[test]
    fn un_reglage_inconnu_du_schema_reste_affiche() {
        let config = std::collections::HashMap::from([("MYSTERE".to_string(), "42".to_string())]);
        let lignes = lignes_reglages(&config, &[], true);
        assert!(lignes.iter().any(|l| l.contains("MYSTERE")));
    }

    #[test]
    fn les_valeurs_booleennes_se_lisent_en_francais() {
        assert_eq!(valeur_lisible("true"), "Oui");
        assert_eq!(valeur_lisible("false"), "Non");
        assert_eq!(valeur_lisible("1"), "Oui");
        assert_eq!(valeur_lisible("0"), "Non");
        assert_eq!(valeur_lisible(""), "—");
        assert_eq!(valeur_lisible("normal"), "normal");
    }

    #[test]
    fn le_mot_de_passe_ne_sort_pas_du_salon_prive() {
        let config = std::collections::HashMap::from([(
            "SERVER_PASSWORD".to_string(),
            "secret".to_string(),
        )]);
        let prive = lignes_reglages(&config, &[], true);
        let inscription = lignes_reglages(&config, &[], false);
        assert!(prive.iter().any(|l| l.contains("secret")));
        assert!(!inscription.iter().any(|l| l.contains("secret")));
    }

    /// Sans rien de configure, les salons gardent les noms historiques : c'est
    /// ce que voient les guildes qui n'ont touche a aucun modele.
    #[test]
    fn session_channels_use_the_game_name() {
        use platform_core::nexus::domain::entities::game::channel_names as noms;

        assert_eq!(
            noms::nom_de_salon(
                None,
                None,
                noms::MODELE_INSCRIPTION_PAR_DEFAUT,
                "7 Days to Die",
                "Le Canap",
                noms::TypeDeSalon::Ecrit
            ),
            "inscription-7-days-to-die"
        );
        assert_eq!(
            noms::nom_de_salon(
                None,
                None,
                noms::MODELE_PRIVE_PAR_DEFAUT,
                "7 Days to Die",
                "Le Canap",
                noms::TypeDeSalon::Ecrit
            ),
            "salon-7-days-to-die"
        );
    }

    #[test]
    fn le_preavis_se_lit_dans_l_evenement() {
        let payload = r#"{"event":"game_server_restart_warning","data":{
            "server_id":"s1","guild_id":"1","minutes_left":15,
            "restart_at":"2026-08-19T13:00:00Z"}}"#;
        let (minutes, quand) = parse_restart_warning(payload);
        assert_eq!(minutes, 15);
        assert_eq!(quand.as_deref(), Some("2026-08-19T13:00:00Z"));
    }

    #[test]
    fn un_preavis_illisible_ne_fait_pas_disparaitre_l_annonce() {
        // Mieux vaut un message imprecis que pas de message quand le serveur
        // va couper.
        assert_eq!(parse_restart_warning("pas du json"), (0, None));
        assert_eq!(parse_restart_warning(r#"{"event":"x"}"#), (0, None));
        let (minutes, quand) = parse_restart_warning(r#"{"data":{"server_id":"s1"}}"#);
        assert_eq!(minutes, 0);
        assert!(quand.is_none());
    }

    #[test]
    fn l_annonce_de_redemarrage_mentionne_le_role_et_l_heure() {
        let avec = build_restart_warning_content(
            Some(RoleId::new(42)),
            "Valheim",
            15,
            Some("2026-08-19T13:00:00Z"),
        );
        assert!(avec.contains("<@&42>"));
        assert!(avec.contains("Valheim"));
        assert!(avec.contains("15 minutes"));
        // Horodatage Discord : chacun le lit dans son fuseau.
        assert!(avec.contains("<t:"));

        // Sans role configure, l'annonce part quand meme — sans mention vide.
        let sans = build_restart_warning_content(None, "Valheim", 15, None);
        assert!(!sans.contains("<@&"));
        assert!(!sans.contains("<t:"));
        assert!(sans.contains("Valheim"));
    }

    #[test]
    fn l_annonce_de_retour_dit_quand_revenir() {
        let avec = build_restarted_content(Some(RoleId::new(7)), "Palworld");
        assert!(avec.contains("<@&7>"));
        assert!(avec.contains("Palworld"));
        assert!(build_restarted_content(None, "Palworld").contains("Palworld"));
    }

    #[test]
    fn une_heure_de_redemarrage_illisible_est_simplement_omise() {
        let contenu = build_restart_warning_content(None, "Jeu", 5, Some("pas une date"));
        assert!(!contenu.contains("<t:"));
        assert!(contenu.contains("5 minutes"));
    }

    #[test]
    fn test_handles_component() {
        assert!(handles_component("gp_register:123"));
        assert!(handles_component("gp_unregister:123"));
        assert!(handles_component("gp_reveal_ip:123"));
        assert!(!handles_component("other_custom_id"));
    }

    #[test]
    fn test_is_player_password_key() {
        assert!(is_player_password_key("PASSWORD"));
        assert!(is_player_password_key("server_pass"));
        assert!(is_player_password_key("SERVER_PASSWORD"));
        assert!(is_player_password_key("ServerConfig_ServerPassword"));
        assert!(!is_player_password_key("ADMIN_PASSWORD"));
    }

    #[test]
    fn test_is_safe_game_option() {
        assert!(is_safe_game_option("SERVER_PASSWORD"));
        assert!(is_safe_game_option("MAX_PLAYERS"));
        assert!(!is_safe_game_option("OPS"));
        assert!(!is_safe_game_option("ADMIN_PASSWORD"));
        assert!(!is_safe_game_option("RCON_PORT"));
        assert!(!is_safe_game_option("API_KEY"));
        assert!(!is_safe_game_option("SECRET_TOKEN"));
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Minecraft 1.20"), "minecraft-1-20");
        assert_eq!(slugify("ARK: Survival Evolved"), "ark-survival-evolved");
        assert_eq!(slugify(""), "serveur");
    }

    #[test]
    fn test_server_id_from_topic() {
        let topic = "Nexus Game Portal | session:abc-123-def | registration";
        assert_eq!(server_id_from_topic(topic), Some("abc-123-def"));
        assert_eq!(server_id_from_topic("aucun"), None);
    }

    #[test]
    fn test_chunk_options() {
        let options = vec!["Option 1".to_string(), "Option 2".to_string()];
        let chunks = chunk_options(&options);
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].contains("Option 1"));
        assert!(chunks[0].contains("Option 2"));

        let long_opt = "A".repeat(600);
        let chunks_multi = chunk_options(&[long_opt.clone(), long_opt.clone()]);
        assert_eq!(chunks_multi.len(), 2);
    }

    #[test]
    fn test_public_cover_url() {
        assert_eq!(public_cover_url(None), None);
        assert_eq!(public_cover_url(Some("")), None);
        assert_eq!(
            public_cover_url_for_status(Some("https://example.com/cover.jpg"), "open"),
            Some("https://example.com/cover.jpg".into())
        );
        assert_eq!(
            public_cover_url_for_status(Some("https://example.com/cover.jpg"), "waiting"),
            Some("https://example.com/cover_attente.jpg".into())
        );
        assert_eq!(
            public_cover_url_for_status(Some("https://example.com/cover.jpg"), "closed"),
            Some("https://example.com/cover_offline.jpg".into())
        );

        std::env::set_var("WEB_FRONT_URL", "https://app.canap.fr");
        assert_eq!(
            public_cover_url_for_status(Some("/imgs/game.png"), "open"),
            Some("https://app.canap.fr/imgs/game.png".into())
        );
        std::env::remove_var("WEB_FRONT_URL");
    }

    #[test]
    fn test_panel_rows_and_embeds() {
        let rows_revealed = panel_rows("srv_1", true);
        assert_eq!(rows_revealed.len(), 1);

        let rows_unrevealed = panel_rows("srv_1", false);
        assert_eq!(rows_unrevealed.len(), 1);

        let embed = build_panel_embed(
            "Valheim",
            "Serveur du Canapé",
            &["user_1".into()],
            Some("2026-01-01T20:00:00Z"),
            false,
            Some("play.canap.fr"),
            Some(2456),
        );
        let json_embed = serde_json::to_value(&embed).unwrap();
        assert_eq!(json_embed["fields"].as_array().unwrap().len(), 3);

        let public_embed = build_public_panel_embed(
            "Valheim",
            "Serveur du Canapé",
            &[],
            None,
            true,
            Some("https://example.com/valheim.jpg"),
        );
        let json_pub = serde_json::to_value(&public_embed).unwrap();
        assert_eq!(json_pub["fields"].as_array().unwrap().len(), 3);

        let opts_embeds = build_options_embeds("Valheim", "Serveur", &[]);
        assert_eq!(opts_embeds.len(), 1);
    }

    #[test]
    fn test_build_overwrites() {
        let g = GuildId::new(12345);
        let r = Some(RoleId::new(67890));
        let ow_text = build_overwrites(g, r, ChannelType::Text);
        assert_eq!(ow_text.len(), 2);

        let ow_voice = build_overwrites(g, r, ChannelType::Voice);
        assert_eq!(ow_voice.len(), 2);

        let req_text = build_create_channel_request(
            "salon-test",
            ChannelType::Text,
            Some(ChannelId::new(111)),
            &ow_text,
            Some("topic test"),
        );
        let j_req_t = serde_json::to_value(&req_text).unwrap();
        assert_eq!(j_req_t["name"], "salon-test");
        assert_eq!(j_req_t["type"], 0); // Text
        assert_eq!(j_req_t["parent_id"], "111");
        assert_eq!(j_req_t["topic"], "topic test");

        let req_voice =
            build_create_channel_request("vocal-test", ChannelType::Voice, None, &ow_voice, None);
        let j_req_v = serde_json::to_value(&req_voice).unwrap();
        assert_eq!(j_req_v["name"], "vocal-test");
        assert_eq!(j_req_v["type"], 2); // Voice
    }

    #[test]
    fn test_session_helpers_and_names() {
        assert_eq!(session_suffix("srv-1234-5678-abc"), "srv12345");
        assert_eq!(
            session_role_name("Valheim RPG", "srv-123"),
            "valheim-rpg_srv123"
        );
        assert_eq!(slugify("Valheim RPG"), "valheim-rpg");

        assert_eq!(legacy_private_text_name("srv-123"), "joueurs-srv123");
        assert_eq!(
            private_text_topic("srv-123"),
            "Nexus Game Portal | session:srv-123 | private"
        );
    }

    #[test]
    fn test_valeur_lisible() {
        assert_eq!(valeur_lisible("true"), "Oui");
        assert_eq!(valeur_lisible("1"), "Oui");
        assert_eq!(valeur_lisible("false"), "Non");
        assert_eq!(valeur_lisible("0"), "Non");
        assert_eq!(valeur_lisible(""), "—");
        assert_eq!(valeur_lisible("10"), "10");
    }

    #[test]
    fn test_lignes_and_sections_reglages() {
        let schema = [TemplateField {
            key: "PVP".into(),
            label: "Combat PvP".into(),
            group: Some("Gameplay".into()),
        }];
        assert_eq!(nom_du_reglage(&schema, "PVP"), "Combat PvP");
        assert_eq!(nom_du_reglage(&schema, "UNKNOWN"), "UNKNOWN");
        assert_eq!(section_du_reglage(&schema, "PVP"), "Gameplay");
        assert_eq!(section_du_reglage(&schema, "UNKNOWN"), "Reglages generaux");

        let mut config = std::collections::HashMap::new();
        config.insert("PVP".into(), "true".into());
        config.insert("PASSWORD".into(), "secret".into());
        config.insert("ADMIN_PASS".into(), "supersecret".into());

        let lines_public = lignes_reglages(&config, &schema, false);
        assert_eq!(lines_public.len(), 2); // 1 header + PVP

        let lines_private = lignes_reglages(&config, &schema, true);
        assert_eq!(lines_private.len(), 4); // 1 header Gameplay + PVP + 1 header Reglages generaux + PASSWORD

        let pub_opts = public_game_options(&config, &schema);
        assert_eq!(pub_opts.len(), 4);

        let reg_opts = registration_options(&config, &schema);
        assert_eq!(reg_opts.len(), 2);
    }

    #[test]
    fn test_status_and_cover_helpers() {
        assert_eq!(strip_status_suffix("cover_attente"), "cover");
        assert_eq!(strip_status_suffix("cover_waiting"), "cover");
        assert_eq!(strip_status_suffix("cover_offline"), "cover");
        assert_eq!(strip_status_suffix("cover_normal"), "cover_normal");

        assert_eq!(
            public_cover_url_for_status(Some("https://example.com/cover.jpg"), "open"),
            Some("https://example.com/cover.jpg".into())
        );
        assert_eq!(
            public_cover_url_for_status(Some("https://example.com/cover.jpg"), "waiting"),
            Some("https://example.com/cover_attente.jpg".into())
        );
        assert_eq!(
            public_cover_url_for_status(Some("https://example.com/cover.jpg"), "closed"),
            Some("https://example.com/cover_offline.jpg".into())
        );
        assert_eq!(
            public_cover_url_for_status(Some("https://example.com/cover.jpg"), "scheduled"),
            Some("https://example.com/cover_attente.jpg".into())
        );
        assert_eq!(
            public_cover_url_for_status(Some("https://example.com/cover.jpg"), "stopped"),
            Some("https://example.com/cover_offline.jpg".into())
        );

        let srv = GameServer {
            id: "s1".into(),
            guild_id: "g1".into(),
            template_id: "t1".into(),
            name: "Server".into(),
            status: "Running".into(),
            owner_user_id: "u1".into(),
            host_port: Some(25565),
            public_host: Some("play.com".into()),
            ip_reveal_at: None,
            ip_revealed: true,
            display_state: Some("open".into()),
            announcement_posted_at: None,
            rules: None,
            channel_name_registration: None,
            channel_name_private: None,
            channel_name_voice: None,
            text_channel_id: None,
            voice_channel_id: None,
            last_player_count: 0,
        };
        assert_eq!(etat_affiche(&srv), "open");

        let ch_str = "12345".to_string();
        assert_eq!(parse_channel(Some(&ch_str)), Some(ChannelId::new(12345)));
        assert_eq!(parse_channel(None), None);
    }

    #[test]
    fn test_is_not_found() {
        assert!(!is_not_found(&serenity::Error::Other("something")));
    }

    #[test]
    fn test_parse_portal_event() {
        let valid_json =
            r#"{"event":"server.started","data":{"server_id":"srv_123","guild_id":"98765"}}"#;
        let parsed = parse_portal_event(valid_json);
        assert_eq!(
            parsed,
            Some(("server.started".into(), "srv_123".into(), 98765))
        );

        assert_eq!(parse_portal_event("invalid json"), None);
        assert_eq!(parse_portal_event(r#"{"event":"server.started"}"#), None);
        assert_eq!(
            parse_portal_event(r#"{"event":"server.started","data":{"server_id":"s1"}}"#),
            None
        );
        assert_eq!(
            parse_portal_event(
                r#"{"event":"server.started","data":{"server_id":"s1","guild_id":"not_a_num"}}"#
            ),
            None
        );
    }

    #[test]
    fn test_reveal_and_ping_helpers() {
        let ack_start = format_reveal_ack(true, 5);
        assert!(ack_start.contains("démarre"));
        assert!(ack_start.contains("5 minute(s)"));

        let ack_online = format_reveal_ack(false, 2);
        assert!(ack_online.contains("déjà en ligne"));

        let embed_soon = build_opening_soon_embed("Minecraft", 10);
        let j_soon = serde_json::to_value(&embed_soon).unwrap();
        assert!(j_soon["title"].as_str().unwrap().contains("Minecraft"));
        assert!(j_soon["description"]
            .as_str()
            .unwrap()
            .contains("10 minute(s)"));

        let card = build_private_reveal_card(
            "Minecraft",
            "Serveur Canap",
            Some("play.net"),
            Some(25565),
            Some("https://example.com/mc.jpg"),
        );
        let j_card = serde_json::to_value(&card).unwrap();
        assert!(j_card["title"]
            .as_str()
            .unwrap()
            .contains("Minecraft — Serveur Canap"));
        assert!(j_card["description"]
            .as_str()
            .unwrap()
            .contains("`play.net:25565`"));
        assert_eq!(j_card["image"]["url"], "https://example.com/mc.jpg");

        let card_none = build_private_reveal_card("Minecraft", "Serveur Canap", None, None, None);
        let j_card_none = serde_json::to_value(&card_none).unwrap();
        assert!(j_card_none["description"]
            .as_str()
            .unwrap()
            .contains("Adresse indisponible"));

        let ping_msg = format_daily_ping_content(RoleId::new(1234), "Minecraft", "<t:100:R>");
        assert_eq!(
            ping_msg,
            "<@&1234> Le serveur **Minecraft** ouvre <t:100:R> ! Inscris-toi sur le panneau."
        );

        assert_eq!(format_when_timestamp(None), "bientôt");
        assert_eq!(
            format_when_timestamp(Some("2026-01-01T00:00:00Z")),
            "<t:1767225600:R>"
        );
    }

    #[tokio::test]
    async fn test_resolve_role_and_options_embeds() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{}", addr);

        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]);

                let body = if req.contains("/api/games/templates/tpl_1") {
                    r#"{"name":"Minecraft","slug":"minecraft","config_schema":[{"key":"pvp","label":"PvP"}]}"#
                } else if req.contains("/template-settings") {
                    r#"[{"template_slug":"minecraft","discord_role_id":"99999"}]"#
                } else if req.contains("/api/games/") {
                    r#"{"id":"g1","game_name":"Minecraft","role_id":"88888"}"#
                } else {
                    r#"{"ok":true}"#
                };

                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes()).await;
            }
        });

        let client = ApiClient::new(base_url, Some("token".into()));

        let role = resolve_role(&client, "g1", "minecraft", "Minecraft").await;
        assert_eq!(role, Some(RoleId::new(99999)));

        let srv = GameServer {
            id: "s1".into(),
            guild_id: "g1".into(),
            template_id: "tpl_1".into(),
            name: "Mon Serveur".into(),
            status: "Running".into(),
            owner_user_id: "u1".into(),
            host_port: Some(25565),
            public_host: Some("play.net".into()),
            ip_reveal_at: None,
            ip_revealed: true,
            display_state: Some("open".into()),
            announcement_posted_at: None,
            rules: None,
            channel_name_registration: None,
            channel_name_private: None,
            channel_name_voice: None,
            text_channel_id: None,
            voice_channel_id: None,
            last_player_count: 0,
        };

        let (gname, grole) = game_name_and_role(&client, &srv).await;
        assert_eq!(gname, "Minecraft");
        assert_eq!(grole, Some(RoleId::new(99999)));

        let tpl = crate::api_client::GameTemplate {
            name: "Minecraft".into(),
            slug: "minecraft".into(),
            cover_image_url: Some("https://example.com/mc.jpg".into()),
            config_schema: vec![crate::api_client::TemplateField {
                key: "pvp".into(),
                label: "PvP".into(),
                group: None,
            }],
        };
        let mut cfg = std::collections::HashMap::new();
        cfg.insert("pvp".into(), "true".into());

        let embeds_pub = build_options_embeds_for_server(&srv, Some(&tpl), &cfg, false);
        assert_eq!(embeds_pub.len(), 1);
        let j_pub = serde_json::to_value(&embeds_pub[0]).unwrap();
        assert!(j_pub["title"].as_str().unwrap().contains("Minecraft"));

        let embeds_priv = build_options_embeds_for_server(&srv, Some(&tpl), &cfg, true);
        assert_eq!(embeds_priv.len(), 1);
        let j_priv = serde_json::to_value(&embeds_priv[0]).unwrap();
        assert!(j_priv["title"].as_str().unwrap().contains("Minecraft"));

        // Test persist_category
        persist_category(&client, "g1", ChannelId::new(55555)).await;
    }

    #[tokio::test]
    async fn test_portal_registration_and_reveal_logic() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        assert_eq!(strip_status_suffix("image_attente"), "image");
        assert_eq!(strip_status_suffix("image_waiting"), "image");
        assert_eq!(strip_status_suffix("image_offline"), "image");
        assert_eq!(strip_status_suffix("image_normal"), "image_normal");

        assert_eq!(
            format_registration_error_content(true, "quota"),
            "❌ Inscription impossible : quota"
        );
        assert_eq!(
            format_registration_error_content(false, "introuvable"),
            "❌ Désinscription impossible : introuvable"
        );
        assert_eq!(
            format_registration_ack_content(true),
            "Inscription enregistrée"
        );
        assert_eq!(
            format_registration_ack_content(false),
            "Désinscription enregistrée"
        );
        assert_eq!(
            format_owner_only_reveal_error(),
            "⛔ Seul le propriétaire du serveur peut révéler son adresse."
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{}", addr);

        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]);

                let body = if req.contains("/api/games/servers/s1") && req.contains("/reveal-ip") {
                    r#"{"started":true,"delay_minutes":5}"#
                } else if req.contains("/api/games/servers/s1") {
                    r#"{"server":{"id":"s1","guild_id":"g1","template_id":"t1","name":"Serveur Test","status":"running","owner_user_id":"u1","ip_revealed":false,"last_player_count":0},"config":{}}"#
                } else if req.contains("/api/games/templates/t1") {
                    r#"{"name":"Template Test","slug":"test","config_schema":[]}"#
                } else {
                    r#"{"ok":true}"#
                };

                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes()).await;
            }
        });

        let client = ApiClient::new(base_url, Some("token".into()));

        // Success reveal logic by owner
        let res = execute_reveal_ip_logic(&client, "s1", "u1").await;
        assert!(res.is_ok());
        let (outcome, gname) = res.unwrap();
        assert_eq!(outcome.delay_minutes, 5);
        assert_eq!(gname, "Template Test");

        // Error reveal logic by non-owner
        let err_res = execute_reveal_ip_logic(&client, "s1", "u2").await;
        assert!(err_res.is_err());
        assert_eq!(err_res.unwrap_err(), format_owner_only_reveal_error());
    }
}

/// Suppression d'un jeu : ce que l'evenement doit transporter.
///
/// Ces tests verrouillent le contrat entre l'API, qui construit la charge utile
/// via `payload_serveur_supprime`, et le bot, qui la relit. C'est exactement le
/// joint qui avait lache : le bot allait rechercher les salons aupres d'une
/// fiche deja supprimee, recevait un 404, et laissait les salons Discord en
/// place sans que rien ne le signale.
#[cfg(test)]
mod tests_suppression {
    use super::parse_deleted_payload;
    use platform_core::nexus::ports::outbound::events::game_events;

    /// Reconstitue l'enveloppe telle que le publieur la met sur le stream.
    fn enveloppe(data: serde_json::Value) -> String {
        serde_json::json!({ "event": game_events::SERVER_DELETED, "data": data }).to_string()
    }

    #[test]
    fn les_salons_traversent_l_evenement() {
        let payload = game_events::payload_serveur_supprime(
            "11111111-1111-1111-1111-111111111111",
            "123456789012345678",
            Some("222222222222222222"),
            Some("333333333333333333"),
            "44444444-4444-4444-4444-444444444444",
        );

        let (texte, vocal, template) =
            parse_deleted_payload(&enveloppe(payload)).expect("charge utile lisible");

        assert_eq!(texte.as_deref(), Some("222222222222222222"));
        assert_eq!(vocal.as_deref(), Some("333333333333333333"));
        assert_eq!(template, "44444444-4444-4444-4444-444444444444");
    }

    /// Un serveur cree sans salons Discord — le bouton « sans salon » existe —
    /// doit rester lisible : il n'y a rien a supprimer, ce n'est pas une erreur.
    #[test]
    fn un_serveur_sans_salons_reste_lisible() {
        let payload = game_events::payload_serveur_supprime(
            "11111111-1111-1111-1111-111111111111",
            "123456789012345678",
            None,
            None,
            "44444444-4444-4444-4444-444444444444",
        );

        let (texte, vocal, _) =
            parse_deleted_payload(&enveloppe(payload)).expect("charge utile lisible");

        assert!(texte.is_none());
        assert!(vocal.is_none());
    }

    /// Message emis par une version anterieure, encore en vol dans le stream
    /// pendant un deploiement : il n'a pas les nouveaux champs. Le bot doit le
    /// reconnaitre comme tel pour retomber sur la relecture, et non l'accepter
    /// avec des salons vides — ce qui ne supprimerait rien du tout.
    #[test]
    fn un_message_d_avant_l_enrichissement_est_refuse() {
        let ancien = enveloppe(serde_json::json!({
            "server_id": "11111111-1111-1111-1111-111111111111",
            "guild_id": "123456789012345678",
        }));

        assert!(parse_deleted_payload(&ancien).is_none());
    }

    #[test]
    fn une_charge_utile_illisible_ne_panique_pas() {
        assert!(parse_deleted_payload("pas du json").is_none());
        assert!(parse_deleted_payload("{}").is_none());
    }
}

/// Ce que la purge du salon a le droit d'effacer.
///
/// Un predicat trop large emporterait des messages voisins ; trop etroit, il
/// laisserait le salon se remplir. Ces tests fixent la frontiere sur le
/// contenu REELLEMENT produit par les deux constructeurs.
#[cfg(test)]
mod tests_purge_redemarrage {
    use super::{
        build_restart_warning_content, build_restarted_content, est_annonce_de_redemarrage,
    };
    use serenity::all::RoleId;

    #[test]
    fn les_deux_annonces_sont_reconnues() {
        let preavis = build_restart_warning_content(None, "Palworld", 15, None);
        let retour = build_restarted_content(None, "Palworld");

        assert!(est_annonce_de_redemarrage(&preavis));
        assert!(est_annonce_de_redemarrage(&retour));
    }

    /// Avec mention de role, le contenu change apres l'emoji : le predicat ne
    /// doit pas dependre de ce qui suit immediatement le prefixe.
    #[test]
    fn la_mention_de_role_ne_gene_pas() {
        let role = Some(RoleId::new(123456789012345678));
        assert!(est_annonce_de_redemarrage(&build_restart_warning_content(
            role,
            "Minecraft",
            15,
            None
        )));
        assert!(est_annonce_de_redemarrage(&build_restarted_content(
            role,
            "Minecraft"
        )));
    }

    /// La coche ouvre d'autres messages du portail. Les emporter ferait
    /// disparaitre des informations que personne n'a demande a effacer.
    #[test]
    fn un_autre_message_a_coche_est_epargne() {
        assert!(!est_annonce_de_redemarrage(
            "✅ **Palworld** : inscription enregistree."
        ));
        assert!(!est_annonce_de_redemarrage("✅ Adresse revelee."));
        assert!(!est_annonce_de_redemarrage(
            "🔄 Synchronisation des roles terminee."
        ));
    }

    #[test]
    fn un_message_de_joueur_est_epargne() {
        assert!(!est_annonce_de_redemarrage(
            "il redemarre dans combien de temps ?"
        ));
        assert!(!est_annonce_de_redemarrage(""));
    }
}

// ── Reconciliation : salons orphelins ──────────────────────────────────────

/// Ce role appartient-il a la session `server_id` ?
///
/// Le nom d'un role de session est `{slug_du_jeu}_{suffixe}`, ou le suffixe est
/// fait des huit premiers caracteres alphanumeriques du `server_id`. On ne peut
/// donc pas reconstruire le nom complet sans connaitre le jeu — dont la fiche a
/// justement disparu — mais le suffixe, lui, se deduit du seul identifiant.
///
/// LE GARDE-FOU SUR LA LONGUEUR N'EST PAS DECORATIF. Un identifiant sans huit
/// caracteres alphanumeriques donnerait un suffixe court, voire vide : la
/// comparaison se reduirait a « le nom finit par `_` » et emporterait des roles
/// de la guilde qui n'ont rien a voir. En dessous de huit, on ne reconnait
/// rien, et le role orphelin survit — ce qui se repare a la main, contrairement
/// a un role supprime par erreur.
pub fn role_de_session(nom_du_role: &str, server_id: &str) -> bool {
    let suffixe = session_suffix(server_id);
    if suffixe.len() < 8 {
        return false;
    }
    nom_du_role.ends_with(&format!("_{suffixe}"))
}

/// Supprime les salons et le role d'une session dont le serveur n'existe plus.
///
/// Le salon vocal n'a pas de sujet — Discord n'en accepte pas sur un vocal —
/// et son nom reprend celui du serveur, que l'on ne connait plus. Il se
/// retrouve donc par son role : les trois salons d'une session partagent une
/// permission pour le meme role, dont le nom porte le suffixe de
/// l'identifiant. Le role est supprime en dernier, apres avoir servi de fil.
async fn supprimer_session_orpheline(ctx: &Context, guild_id: GuildId, server_id: &str) {
    let Ok(salons) = guild_id.channels(&ctx.http).await else {
        return;
    };

    for (id, salon) in salons.iter() {
        let vise = salon
            .topic
            .as_deref()
            .and_then(server_id_from_topic)
            .is_some_and(|vu| vu == server_id);
        if vise {
            let _ = id.delete(&ctx.http).await;
        }
    }

    let Ok(roles) = guild_id.roles(&ctx.http).await else {
        return;
    };
    for role in roles
        .values()
        .filter(|r| role_de_session(&r.name, server_id))
    {
        for (id, salon) in salons.iter() {
            let porte_le_role = salon
                .permission_overwrites
                .iter()
                .any(|ow| matches!(ow.kind, PermissionOverwriteType::Role(rid) if rid == role.id));
            if porte_le_role {
                let _ = id.delete(&ctx.http).await;
            }
        }
        let _ = guild_id.delete_role(&ctx.http, role.id).await;
    }

    tracing::info!(server_id, "game-portal: session orpheline nettoyee");
}

/// Supprime les salons des sessions dont le serveur n'existe plus en base.
///
/// POURQUOI CE NETTOYAGE EXISTE. Jusqu'ici, la suppression d'un jeu laissait
/// ses salons en place (le bot interrogeait une fiche deja effacee et
/// renoncait). Le correctif empeche d'en creer de nouveaux, mais ne fait rien
/// des salons deja abandonnes. Celui-ci les rattrape, et rattrapera aussi ceux
/// qu'un incident laisserait derriere lui.
///
/// CE QU'IL NE TOUCHE PAS. Un serveur ARRETE conserve ses salons : c'est le
/// comportement voulu, un serveur de soiree passe l'essentiel de sa vie
/// eteint. Le seul critere de suppression est l'absence de fiche vivante.
///
/// FERME EN CAS DE DOUTE. Si l'API ne repond pas, ou repond autre chose qu'un
/// 404 franc, la session est laissee intacte. Un salon orphelin de trop se
/// supprime a la main ; un salon vivant supprime par erreur emporte une
/// conversation.
pub async fn reconcilier_salons_orphelins(ctx: &Context, api: &ApiClient, guild_id: &str) {
    let Ok(numerique) = guild_id.parse::<u64>() else {
        return;
    };
    let guild = GuildId::new(numerique);
    let Ok(salons) = guild.channels(&ctx.http).await else {
        tracing::warn!(guild_id, "game-portal: inventaire des salons impossible");
        return;
    };

    let mut sessions: Vec<String> = salons
        .values()
        .filter_map(|c| c.topic.as_deref().and_then(server_id_from_topic))
        .map(str::to_string)
        .collect();
    sessions.sort();
    sessions.dedup();

    for server_id in sessions {
        match api.game_server_existe(&server_id).await {
            Ok(true) => {}
            Ok(false) => supprimer_session_orpheline(ctx, guild, &server_id).await,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    server_id,
                    "game-portal: existence indeterminee, salons conserves"
                );
            }
        }
    }
}

/// Reconnaissance d'un role de session, sur laquelle repose la suppression du
/// salon vocal orphelin — le seul des trois que Discord ne laisse pas marquer.
#[cfg(test)]
mod tests_reconciliation {
    use super::{role_de_session, server_id_from_topic, session_role_name};

    #[test]
    fn le_role_de_sa_propre_session_est_reconnu() {
        let id = "3f2a91bc-4d5e-4a7b-9c1d-2e3f4a5b6c7d";
        assert!(role_de_session(&session_role_name("Palworld", id), id));
        assert!(role_de_session(
            &session_role_name("Project Zomboid", id),
            id
        ));
    }

    #[test]
    fn le_role_d_une_autre_session_est_epargne() {
        let mien = "3f2a91bc-4d5e-4a7b-9c1d-2e3f4a5b6c7d";
        let autre = "99887766-4d5e-4a7b-9c1d-2e3f4a5b6c7d";
        assert!(!role_de_session(
            &session_role_name("Palworld", autre),
            mien
        ));
    }

    /// Le vrai danger : un identifiant sans huit caracteres alphanumeriques
    /// donne un suffixe court, et la comparaison degenere en « finit par `_` ».
    /// Elle emporterait alors des roles de la guilde qui n'ont aucun rapport.
    #[test]
    fn un_identifiant_trop_court_ne_reconnait_rien() {
        // Chaque paire est un role de la guilde que le suffixe court FERAIT
        // correspondre si le garde-fou tombait : c'est la que se joue le
        // risque, pas sur des noms qui ne correspondent de toute facon pas.
        for (role, court) in [
            ("moderateur_abc", "abc"),
            ("staff_a1", "a1"),
            ("vip_2e3f4a5", "2e3f4a5"),
            ("quelquechose_", ""),
            ("autre_", "---"),
        ] {
            assert!(
                !role_de_session(role, court),
                "role {role:?} emporte par l'identifiant trop court {court:?}"
            );
        }
    }

    #[test]
    fn les_roles_ordinaires_sont_epargnes() {
        let id = "3f2a91bc-4d5e-4a7b-9c1d-2e3f4a5b6c7d";
        for nom in ["Moderateur", "@everyone", "admin_3f2a91b", "3f2a91bc"] {
            assert!(!role_de_session(nom, id), "role {nom:?} emporte a tort");
        }
    }

    /// Le recensement des sessions part du sujet des salons : c'est lui qui
    /// decide quels identifiants seront verifies aupres de l'API.
    #[test]
    fn le_sujet_livre_l_identifiant_de_session() {
        let id = "3f2a91bc-4d5e-4a7b-9c1d-2e3f4a5b6c7d";
        assert_eq!(
            server_id_from_topic(&format!("Nexus Game Portal | session:{id} | registration")),
            Some(id)
        );
        assert_eq!(
            server_id_from_topic(&format!("Nexus Game Portal | session:{id} | private")),
            Some(id)
        );
        assert_eq!(server_id_from_topic("Salon de discussion general"), None);
    }
}

// ── Remise en etat d'une session ───────────────────────────────────────────

/// Ce que la remise en etat a trouve et repare.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct RapportResync {
    /// Inscrits connus de l'API.
    pub inscrits: usize,
    /// Inscrits a qui le role manquait, et qui l'ont recu.
    pub roles_poses: usize,
    /// Inscrits introuvables dans la guilde : partis, ou identifiant illisible.
    pub absents: usize,
    /// Le role de session avait disparu et a ete recree.
    pub role_recree: bool,
    /// Le panneau d'inscription manquait et a ete republie.
    pub panneau_republie: bool,
}

impl RapportResync {
    /// Rien n'a bouge : tout etait deja en place.
    pub fn rien_a_faire(&self) -> bool {
        self.roles_poses == 0 && !self.role_recree && !self.panneau_republie
    }

    /// Compte rendu lisible, tel qu'il est renvoye a l'administrateur.
    ///
    /// Il DIT CE QUI A ETE FAIT, pas « termine ». Une commande de reparation
    /// qui repond seulement « ok » n'apprend rien : on relance une
    /// resynchronisation justement parce qu'on ne sait pas ce qui manque.
    pub fn resume(&self) -> String {
        if self.rien_a_faire() {
            return format!(
                "✅ Rien a reparer. {} inscrit(s), role et panneau en place.",
                self.inscrits
            );
        }
        let mut lignes = vec![format!(
            "✅ Session resynchronisee ({} inscrits).",
            self.inscrits
        )];
        if self.role_recree {
            lignes.push("• Role de session recree.".into());
        }
        if self.roles_poses > 0 {
            lignes.push(format!("• Role rendu a {} membre(s).", self.roles_poses));
        }
        if self.panneau_republie {
            lignes.push("• Panneau d'inscription republie.".into());
        }
        if self.absents > 0 {
            lignes.push(format!(
                "• {} inscrit(s) introuvable(s) dans le serveur (partis ?).",
                self.absents
            ));
        }
        lignes.join("\n")
    }
}

/// Le salon contient-il deja un panneau d'inscription ?
///
/// On cherche un message DU BOT porteur de composants : le panneau est le seul
/// a en avoir dans un salon de session. Comparer un titre d'embed serait plus
/// fragile — il change avec le nom du jeu.
///
/// Une lecture impossible repond « oui » : republier a l'aveugle poserait un
/// second panneau a cote du premier, et deux panneaux valent moins qu'un.
async fn le_panneau_existe(ctx: &Context, salon: ChannelId) -> bool {
    let bot_id = ctx.cache.current_user().id;
    match salon
        .messages(&ctx.http, GetMessages::new().limit(50))
        .await
    {
        Ok(messages) => messages
            .iter()
            .any(|m| m.author.id == bot_id && !m.components.is_empty()),
        Err(e) => {
            tracing::warn!(error = %e, %salon, "resync : lecture du salon impossible");
            true
        }
    }
}

/// Remet une session d'aplomb : role, inscrits, panneau.
///
/// POURQUOI CETTE COMMANDE EXISTE. Une session vit dans deux mondes — la base,
/// qui sait qui est inscrit, et Discord, qui distribue les acces. Ils se
/// desynchronisent : un role supprime a la main, un panneau efface, un membre
/// revenu apres un depart, une panne pendant l'ouverture. Rien ne rattrapait
/// ces ecarts, et il fallait recreer la session entiere pour un role manquant.
///
/// ELLE AJOUTE, ELLE NE RETIRE JAMAIS. Un membre porteur du role mais absent
/// des inscrits n'est pas touche : il peut l'avoir recu a la main, pour une
/// bonne raison qu'on ignore. Une commande de reparation qui retire des acces
/// serait bien plus dangereuse que le desordre qu'elle corrige.
pub(crate) async fn resynchroniser_session(
    ctx: &Context,
    api: &ApiClient,
    guild_id: GuildId,
    server_id: &str,
) -> Result<RapportResync, String> {
    let detail = api
        .get_game_server(server_id)
        .await
        .map_err(|e| format!("serveur introuvable : {e}"))?;
    let server = detail.server;
    let mut rapport = RapportResync::default();

    let game_name = api
        .get_game_template(&server.template_id)
        .await
        .map(|t| t.name)
        .unwrap_or_else(|_| "Jeu".into());
    let nom_du_role = session_role_name(&game_name, server_id);

    // 1. Le role de session, recree s'il a disparu.
    let roles = guild_id
        .roles(&ctx.http)
        .await
        .map_err(|e| format!("roles illisibles : {e}"))?;
    let role_id = match roles.values().find(|r| r.name == nom_du_role) {
        Some(role) => role.id,
        None => {
            let cree = guild_id
                .create_role(
                    &ctx.http,
                    EditRole::new()
                        .name(nom_du_role.clone())
                        .colour(Colour::new(0x5865f2))
                        .mentionable(false)
                        .hoist(false),
                )
                .await
                .map_err(|e| format!("role de session non recree : {e}"))?;
            rapport.role_recree = true;
            cree.id
        }
    };

    // 2. Les inscrits recoivent le role qui leur manque.
    let inscrits = api
        .list_server_registrations(server_id)
        .await
        .map_err(|e| format!("inscriptions illisibles : {e}"))?;
    rapport.inscrits = inscrits.len();

    for inscription in &inscrits {
        let Ok(user_num) = inscription.user_id.parse::<u64>() else {
            rapport.absents += 1;
            continue;
        };
        let Ok(membre) = guild_id.member(&ctx.http, user_num).await else {
            // Parti du serveur : ce n'est pas une erreur, mais l'administrateur
            // doit le savoir — c'est souvent la raison du desordre constate.
            rapport.absents += 1;
            continue;
        };
        if membre.roles.contains(&role_id) {
            continue;
        }
        if let Err(e) = membre.add_role(&ctx.http, role_id).await {
            tracing::warn!(error = %e, user = %inscription.user_id, "resync : role non pose");
            rapport.absents += 1;
        } else {
            rapport.roles_poses += 1;
        }
    }

    // 3. Le panneau d'inscription, republie s'il manque.
    if let Some(text_ch) = parse_channel(server.text_channel_id.as_ref()) {
        if !le_panneau_existe(ctx, text_ch).await {
            poster_le_panneau(ctx, api, text_ch, &server).await;
            rapport.panneau_republie = true;
        }
    }

    tracing::info!(server_id, ?rapport, "resync : session remise en etat");
    Ok(rapport)
}

/// Le compte rendu de la remise en etat.
///
/// C'est la seule chose que l'administrateur voit : une commande de reparation
/// qui repond « ok » n'apprend rien, puisqu'on la lance justement parce qu'on
/// ignore ce qui manque.
#[cfg(test)]
mod tests_resync {
    use super::RapportResync;

    #[test]
    fn une_session_intacte_le_dit_sans_ambiguite() {
        let r = RapportResync {
            inscrits: 7,
            ..Default::default()
        };
        assert!(r.rien_a_faire());

        let texte = r.resume();
        assert!(texte.contains("Rien a reparer"));
        assert!(texte.contains('7'));
    }

    #[test]
    fn chaque_reparation_est_nommee() {
        let r = RapportResync {
            inscrits: 5,
            roles_poses: 3,
            absents: 1,
            role_recree: true,
            panneau_republie: true,
        };
        assert!(!r.rien_a_faire());

        let texte = r.resume();
        assert!(texte.contains("Role de session recree"));
        assert!(texte.contains("3 membre(s)"));
        assert!(texte.contains("Panneau d'inscription republie"));
        assert!(texte.contains("1 inscrit(s) introuvable"));
    }

    /// Un role pose est une reparation, meme sans role recree ni panneau
    /// republie : c'est le cas le plus courant, et le rapport ne doit pas
    /// annoncer « rien a reparer » alors qu'on vient de rendre des acces.
    #[test]
    fn un_role_rendu_suffit_a_ne_pas_dire_rien_a_faire() {
        let r = RapportResync {
            inscrits: 4,
            roles_poses: 1,
            ..Default::default()
        };
        assert!(!r.rien_a_faire());
        assert!(r.resume().contains("1 membre(s)"));
    }

    /// Des absents SEULS ne sont pas une reparation : on n'a rien change, on a
    /// seulement constate. Le dire autrement laisserait croire a une action.
    #[test]
    fn des_absents_seuls_ne_valent_pas_reparation() {
        let r = RapportResync {
            inscrits: 4,
            absents: 2,
            ..Default::default()
        };
        assert!(r.rien_a_faire());
        assert!(r.resume().contains("Rien a reparer"));
    }
}
