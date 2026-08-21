//! # nexus-bot — bot Discord de la plateforme jeux Nexus
//!
//! Serenity minimal, calque sur l'architecture de `sentinel-bot` : le bot
//! n'a AUCUN acces DB, il passe par nexus-api (client HTTP Bearer).
//!
//! Commandes :
//!   - `/roue` — 1 spin de la Roue du Destin par joueur par jour.
//!   - `/solde [membre]` — consulte son portefeuille (ou celui d'un autre).
//!   - `/donner <membre> <montant> [raison]` — transfert de coins.
//!   - `/classement` — top 10 des plus riches du serveur.
//!
//! Env :
//!   - NEXUS_DISCORD_TOKEN (sans lui : log + exit propre, comme le scaffold)
//!   - NEXUS_API_URL (defaut http://localhost:3100)
//!   - NEXUS_API_KEY (Bearer vers nexus-api)

mod achievements;
mod api_client;
mod compteurs;
mod embeds;
mod event_bus;
mod game_portal;
mod games;
mod grand_salon;
mod wheel_panel;

use std::sync::Arc;

use serenity::all::ButtonStyle;
use serenity::all::Command;
use serenity::all::CommandInteraction;
use serenity::all::CommandOptionType;
use serenity::all::Context;
use serenity::all::CreateActionRow;
use serenity::all::CreateButton;
use serenity::all::CreateCommand;
use serenity::all::CreateCommandOption;
use serenity::all::CreateInteractionResponse;
use serenity::all::CreateInteractionResponseMessage;
use serenity::all::EventHandler;
use serenity::all::GatewayIntents;
use serenity::all::Interaction;
use serenity::all::Ready;
use serenity::all::UserId;
use serenity::async_trait;
use serenity::Client;

use std::sync::atomic::{AtomicBool, Ordering};

use api_client::ApiClient;

struct Handler {
    api: Arc<ApiClient>,
    /// Garde : le consumer d'evenements ne doit demarrer qu'une fois, meme si
    /// `ready` est rejoue apres une reconnexion gateway.
    game_portal_started: AtomicBool,
}

fn option_user(cmd: &CommandInteraction, name: &str) -> Option<UserId> {
    platform_common_bot::discord_helpers::option_user(&cmd.data.options, name)
}

fn option_integer(cmd: &CommandInteraction, name: &str) -> Option<i64> {
    platform_common_bot::discord_helpers::option_i64(&cmd.data.options, name)
}

fn option_string(cmd: &CommandInteraction, name: &str) -> Option<String> {
    platform_common_bot::discord_helpers::option_str(&cmd.data.options, name).map(|s| s.to_string())
}

mod coussin_commands;
mod coussin_steal_events;
mod economy_commands;

/// La guilde est-elle celle servie par cette installation ?
///
/// `PUBLIC_GUILD_ID` absente = aucun verrou. C'est le seul defaut sur :
/// refuser par defaut ferait quitter tous ses serveurs au bot au premier
/// demarrage mal configure, et un depart ne se rattrape pas d'un clic.
pub fn is_guild_allowed(guild_id: serenity::model::id::GuildId, public_guild_id: Option<&str>, guild_id_env: Option<&str>) -> bool {
    let attendu = public_guild_id.or(guild_id_env).unwrap_or_default().trim();
    attendu.is_empty() || attendu == guild_id.to_string()
}

fn guilde_autorisee(guild_id: serenity::model::id::GuildId) -> bool {
    let pub_id = std::env::var("PUBLIC_GUILD_ID").ok();
    let g_id = std::env::var("GUILD_ID").ok();
    is_guild_allowed(guild_id, pub_id.as_deref(), g_id.as_deref())
}

#[async_trait]
impl EventHandler for Handler {
    /// Mono-serveur : le bot quitte toute autre guilde.
    ///
    /// Sans `is_new` ici : cet evenement arrive AUSSI au demarrage pour les
    /// guildes deja rejointes, et c'est precisement le cas qu'on veut
    /// nettoyer — un bot ajoute ailleurs avant la mise en place du verrou.
    async fn guild_create(
        &self,
        ctx: Context,
        guild: serenity::model::guild::Guild,
        _is_new: Option<bool>,
    ) {
        if guilde_autorisee(guild.id) {
            return;
        }
        tracing::warn!(
            guild_id = %guild.id,
            name = %guild.name,
            "mono-serveur : guilde non autorisee, le bot la quitte"
        );
        if let Err(e) = guild.id.leave(&ctx.http).await {
            tracing::error!(error = %e, guild_id = %guild.id, "echec du depart");
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        tracing::info!("nexus-bot connecte en tant que {}", ready.user.name);
        // Consumer des evenements game-portal (cycle de vie des salons de
        // session). Spawn une seule fois : `ready` peut etre rejoue apres une
        // reconnexion gateway, le consumer tourne deja.
        if !self.game_portal_started.swap(true, Ordering::SeqCst) {
            game_portal::spawn(ctx.clone(), self.api.clone());
            achievements::spawn(ctx.clone(), self.api.clone());
            games::spawn_listener(ctx.clone(), self.api.clone());
            coussin_steal_events::spawn(ctx.clone(), self.api.clone());
            // Salons compteurs (« En jeu », « Serveurs actifs »). Dans le meme
            // garde-fou que les autres taches de fond : `ready` est rejoue
            // apres une reconnexion, et une seconde boucle doublerait les
            // renommages — le quota Discord ne le supporterait pas.
            compteurs::spawn(
                ctx.clone(),
                self.api.clone(),
                ready
                    .guilds
                    .iter()
                    .map(|g| g.id)
                    .filter(|id| guilde_autorisee(*id))
                    .collect(),
            );
        }
        // Un evenement Redis publie pendant une indisponibilite peut avoir ete
        // manque. Rejouer les serveurs actifs/programmes garantit que leurs
        // salons Discord finissent quand meme par exister.
        game_portal::reconcile(
            ctx.clone(),
            self.api.clone(),
            ready
                .guilds
                .iter()
                .map(|g| g.id)
                .filter(|id| guilde_autorisee(*id))
                .collect(),
        );
        let commands = build_all_slash_commands();
        for command in commands {
            if let Err(e) = Command::create_global_command(&ctx.http, command).await {
                tracing::error!("enregistrement d'une commande slash impossible: {e}");
            }
        }
        tracing::info!(
            "commandes slash /roue /roue-panel /solde /donner /classement /game /game-admin enregistrees (globales)"
        );
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(cmd) => match cmd.data.name.as_str() {
                "roue" => self.handle_roue(&ctx, &cmd).await,
                "solde" => self.handle_solde(&ctx, &cmd).await,
                "donner" => self.handle_donner(&ctx, &cmd).await,
                "classement" => self.handle_classement(&ctx, &cmd).await,
                "coussin" => self.handle_coussin(&ctx, &cmd).await,
                "profil" => self.handle_coussin_profile(&ctx, &cmd).await,
                "classe" => self.handle_coussin_class(&ctx, &cmd).await,
                "train" => self.handle_coussin_train(&ctx, &cmd).await,
                "shop" => self.handle_coussin_shop(&ctx, &cmd).await,
                "garantie" => self.handle_coussin_insurance(&ctx, &cmd).await,
                "chiper" => self.handle_steal(&ctx, &cmd).await,
                "contrat" => self.handle_prime(&ctx, &cmd).await,
                "inventaire" => self.handle_inventory(&ctx, &cmd).await,
                "pari" => self.handle_bet(&ctx, &cmd).await,
                "roue-panel" => wheel_panel::handle_command(&ctx, &cmd).await,
                "game" | "game-admin" => games::handle_command(&self.api, &ctx, &cmd).await,
                "salon" => grand_salon::handle_command(&self.api, &ctx, &cmd).await,
                "haut-faits" => achievements::handle_command(&self.api, &ctx, &cmd).await,
                _ => {}
            },
            Interaction::Component(component) => {
                let cid = component.data.custom_id.as_str();
                if cid.starts_with("cs:") {
                    self.handle_steal_component(&ctx, &component).await;
                } else if cid.starts_with("c:") {
                    self.handle_coussin_component(&ctx, &component).await;
                } else if wheel_panel::handles_component(cid) {
                    wheel_panel::handle_spin(&self.api, &ctx, &component).await;
                } else if games::handles_component(cid) {
                    games::on_component(&self.api, &ctx, &component).await;
                } else if game_portal::handles_component(cid) {
                    game_portal::on_component(&self.api, &ctx, &component).await;
                }
            }
            _ => {}
        }
    }

    /// Un role vient de disparaitre de la guilde.
    ///
    /// C'est le chemin qui rattrape le cas courant : quelqu'un fait le menage
    /// dans les roles, et la liaison jeu -> role devient morte sans que rien ne
    /// le signale. Prevenir l'API tout de suite evite que chaque abonnement
    /// echoue jusqu'a ce qu'un humain soupconne le probleme.
    ///
    /// Aucun jeu n'est supprime ici : seule la liaison est coupee, la decision
    /// de recreer le role ou d'abandonner le jeu reste humaine.
    async fn guild_role_delete(
        &self,
        _ctx: Context,
        guild_id: serenity::all::GuildId,
        removed_role_id: serenity::all::RoleId,
        _removed_role: Option<serenity::model::guild::Role>,
    ) {
        if !guilde_autorisee(guild_id) {
            return;
        }
        games::sync::on_role_deleted(&self.api, guild_id, removed_role_id).await;
    }

    async fn reaction_add(&self, ctx: Context, add_reaction: serenity::all::Reaction) {
        games::handle_reaction(&self.api, &ctx, &add_reaction, true).await;
    }

    async fn reaction_remove(&self, ctx: Context, removed_reaction: serenity::all::Reaction) {
        games::handle_reaction(&self.api, &ctx, &removed_reaction, false).await;
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    // `env::var` renvoie Ok("") pour une variable definie mais vide — cas
    // normal quand le compose passe `${NEXUS_DISCORD_TOKEN:-}` et que le .env
    // n'est pas encore rempli. Sans ce filtre, le bot tenterait de s'authentifier
    // avec un token vide, echouerait, et repartirait en boucle de redemarrage
    // avec une erreur Discord illisible au lieu d'un message clair.
    let token = std::env::var("NEXUS_DISCORD_TOKEN")
        .ok()
        .filter(|t| !t.trim().is_empty());
    let Some(token) = token else {
        tracing::info!(
            "NEXUS_DISCORD_TOKEN absent ou vide — arret (renseigne-le dans .env \
             pour connecter le bot Discord)"
        );
        return;
    };

    let api_url =
        std::env::var("NEXUS_API_URL").unwrap_or_else(|_| "http://localhost:3100".to_string());
    let api_key = std::env::var("NEXUS_API_KEY")
        .ok()
        .filter(|k| !k.is_empty());
    let api = Arc::new(ApiClient::new(api_url, api_key));

    // GUILD_MEMBERS (privilégié) : nécessaire pour énumérer les porteurs d'un
    // rôle et afficher le nombre d'abonnés sur les boutons des panneaux de jeux.
    // À activer aussi dans le portail Discord Developer (« Server Members Intent »),
    // sinon la passerelle refuse la connexion.
    let mut intents = GatewayIntents::non_privileged() | GatewayIntents::GUILD_MEMBERS;

    // Activite de jeu des membres (« Joue a League of Legends »), pour le
    // compteur du meme nom. C'est un intent PRIVILEGIE : le demander sans
    // l'avoir coche dans le portail Discord fait REFUSER la connexion, et le
    // bot ne demarre plus du tout. D'ou cet interrupteur, eteint par defaut :
    // on ne casse pas un bot en service pour une fonction d'affichage.
    //
    // Marche a suivre : cocher « Presence Intent » dans le portail, PUIS
    // poser NEXUS_PRESENCE_INTENT=true. Dans cet ordre.
    if compteurs::intent_presence_demande() {
        intents |= GatewayIntents::GUILD_PRESENCES;
        tracing::info!(
            "NEXUS_PRESENCE_INTENT actif : l'activite de jeu des membres sera lue.              Si la passerelle refuse la connexion, c'est que « Presence Intent »              n'est pas coche dans le portail Discord."
        );
    }
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler {
            api,
            game_portal_started: AtomicBool::new(false),
        })
        .await
        .expect("creation du client serenity");
    if let Err(e) = client.start().await {
        tracing::error!("erreur client nexus-bot: {e}");
    }
}

pub fn build_all_slash_commands() -> Vec<CreateCommand> {
    let commands = vec![
        CreateCommand::new("roue")
            .description("Tire la Roue du Destin — 1 spin par jour, le destin decide."),
        CreateCommand::new("solde")
            .description("Affiche ton portefeuille (ou celui d'un autre membre)")
            .add_option(CreateCommandOption::new(
                CommandOptionType::User,
                "membre",
                "Le membre dont voir le solde (defaut : toi)",
            )),
        CreateCommand::new("donner")
            .description("Donne des coins a un autre membre")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "membre",
                    "Le membre a qui donner",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "montant",
                    "Le nombre de coins a donner",
                )
                .required(true)
                .min_int_value(1),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "raison",
                "Raison du don (optionnelle)",
            )),
        CreateCommand::new("classement").description("Top 10 des plus riches du serveur"),
        CreateCommand::new("coussin")
            .description("Glisse un coussin piege sous un membre")
            .add_option(
                CreateCommandOption::new(CommandOptionType::User, "membre", "Ta victime")
                    .required(true),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::Integer, "mise", "Mise en coins")
                    .required(true)
                    .min_int_value(1),
            ),
        CreateCommand::new("profil")
            .description("Ta place sur le canape : classe, confort, palmares")
            .add_option(CreateCommandOption::new(
                CommandOptionType::User,
                "membre",
                "Membre (defaut : toi)",
            )),
        CreateCommand::new("classe")
            .description("Choisis ta maniere d'occuper le canape")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "classe",
                    "ecraseur, ressort, piegeur ou couette",
                )
                .required(true)
                .add_string_choice("🪑 Écraseur — tu t'assois sans regarder", "ecraseur")
                .add_string_choice("🤸 Ressort — tu rebondis", "ressort")
                .add_string_choice("🪡 Piégeur — tu places les coussins", "piegeur")
                .add_string_choice("🛌 Couette — tu ne bouges plus", "couette"),
            ),
        CreateCommand::new("train")
            .description("Depense un point : plus d'impact ou plus de moelleux")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "stat",
                    "Ce que tu ameliores",
                )
                .required(true)
                .add_string_choice("Impact", "atk")
                .add_string_choice("Moelleux", "def"),
            ),
        CreateCommand::new("shop")
            .description("Le coffre a coussins")
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "objet", "Objet")
                    .required(true)
                    .add_string_choice("Coussin Plombe", "rage")
                    .add_string_choice("Oeil sous le Plaid", "mindgame")
                    .add_string_choice("Renversement de Chips", "explosion")
                    .add_string_choice("Double Coussin", "double_coup")
                    .add_string_choice("Bataille d'Oreillers", "surprise")
                    .add_string_choice("Punaise dans le Coussin", "coup_traitre")
                    .add_string_choice("Retourne le Canape", "inversion"),
            ),
        CreateCommand::new("garantie")
            .description("Garantie anti-tache : couvre tes pertes pendant 1h"),
        CreateCommand::new("chiper")
            .description("Fouille sous les coussins d'un membre")
            .add_option(
                CreateCommandOption::new(CommandOptionType::User, "membre", "Cible")
                    .required(true),
            ),
        CreateCommand::new("contrat")
            .description("Promets une recompense a qui fera lever quelqu'un")
            .add_option(
                CreateCommandOption::new(CommandOptionType::User, "membre", "Cible")
                    .required(true),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::Integer, "montant", "Montant")
                    .required(true)
                    .min_int_value(1),
            ),
        CreateCommand::new("inventaire").description("Ce que tu planques sous ton coussin"),
        CreateCommand::new("pari")
            .description("Parie sur une bagarre en cours")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "combat",
                    "ID de la bagarre",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "membre",
                    "Celui que tu soutiens",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::Integer, "montant", "Mise")
                    .required(true)
                    .min_int_value(1),
            ),
    ];
    commands
        .into_iter()
        .chain(games::register_commands())
        .chain(std::iter::once(achievements::register()))
        .chain(std::iter::once(grand_salon::register()))
        .chain(std::iter::once(wheel_panel::register()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serenity::model::id::GuildId;

    #[test]
    fn test_is_guild_allowed_no_env() {
        assert!(is_guild_allowed(GuildId::new(12345), None, None));
        assert!(is_guild_allowed(GuildId::new(67890), None, None));
    }

    #[test]
    fn test_is_guild_allowed_public_guild_id() {
        assert!(is_guild_allowed(GuildId::new(12345), Some("12345"), None));
        assert!(!is_guild_allowed(GuildId::new(67890), Some("12345"), None));
    }

    #[test]
    fn test_is_guild_allowed_guild_id() {
        assert!(is_guild_allowed(GuildId::new(999), None, Some("999")));
        assert!(!is_guild_allowed(GuildId::new(12345), None, Some("999")));
    }

    #[test]
    fn test_build_all_slash_commands() {
        let cmds = build_all_slash_commands();
        assert!(cmds.len() >= 18);
        let names: Vec<String> = cmds
            .iter()
            .map(|c| {
                let j = serde_json::to_value(c).unwrap();
                j["name"].as_str().unwrap().to_string()
            })
            .collect();
        assert!(names.contains(&"roue".to_string()));
        assert!(names.contains(&"solde".to_string()));
        assert!(names.contains(&"donner".to_string()));
        assert!(names.contains(&"classement".to_string()));
        assert!(names.contains(&"coussin".to_string()));
        assert!(names.contains(&"profil".to_string()));
        assert!(names.contains(&"classe".to_string()));
        assert!(names.contains(&"train".to_string()));
        assert!(names.contains(&"shop".to_string()));
        assert!(names.contains(&"garantie".to_string()));
        assert!(names.contains(&"chiper".to_string()));
        assert!(names.contains(&"contrat".to_string()));
        assert!(names.contains(&"inventaire".to_string()));
        assert!(names.contains(&"pari".to_string()));
        assert!(names.contains(&"game".to_string()));
        assert!(names.contains(&"game-admin".to_string()));
        assert!(names.contains(&"haut-faits".to_string()));
        assert!(names.contains(&"salon".to_string()));
        assert!(names.contains(&"roue-panel".to_string()));
    }
}
