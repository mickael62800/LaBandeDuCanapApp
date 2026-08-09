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

mod api_client;
mod embeds;
mod event_bus;
mod game_portal;
mod games;
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

impl Handler {
    /// Reponse ephemere avec l'embed d'erreur standard.
    async fn reply_error(&self, ctx: &Context, cmd: &CommandInteraction, message: &str) {
        let msg = CreateInteractionResponseMessage::new()
            .embed(embeds::build_error_embed(message))
            .ephemeral(true);
        let _ = cmd
            .create_response(&ctx.http, CreateInteractionResponse::Message(msg))
            .await;
    }

    /// Exige un serveur : retourne le guild_id ou repond une erreur ephemere.
    async fn require_guild(&self, ctx: &Context, cmd: &CommandInteraction) -> Option<String> {
        let Some(g) = cmd.guild_id else {
            self.reply_error(
                ctx,
                cmd,
                "Cette commande s'utilise sur un serveur, pas en MP.",
            )
            .await;
            return None;
        };

        // Mono-serveur. Ce controle vit ici parce que TOUTES les commandes
        // passent par `require_guild` : le placer dans chaque handler en
        // aurait laisse passer au moins un.
        //
        // L'API refuserait de toute facon, mais avec une erreur technique
        // illisible ; autant repondre clairement.
        if !guilde_autorisee(g) {
            self.reply_error(ctx, cmd, "Ce bot ne sert pas ce serveur.")
                .await;
            return None;
        }

        Some(g.to_string())
    }

    async fn handle_solde(&self, ctx: &Context, cmd: &CommandInteraction) {
        let Some(guild_id) = self.require_guild(ctx, cmd).await else {
            return;
        };

        let target_id = option_user(cmd, "membre").unwrap_or(cmd.user.id);
        let display_name = if target_id == cmd.user.id {
            cmd.user.display_name().to_string()
        } else {
            cmd.data
                .resolved
                .users
                .get(&target_id)
                .map(|u| u.display_name().to_string())
                .unwrap_or_else(|| format!("<@{target_id}>"))
        };

        if let Err(e) = cmd.defer(&ctx.http).await {
            tracing::error!("defer /solde impossible: {e}");
            return;
        }

        let response = self.api.get_wallet(&guild_id, &target_id.to_string()).await;
        let embed = match &response {
            Ok(w) => embeds::build_wallet_embed(w, &display_name),
            Err(msg) => embeds::build_error_embed(msg),
        };
        let builder = serenity::all::CreateInteractionResponseFollowup::new().embed(embed);
        if let Err(e) = cmd.create_followup(&ctx.http, builder).await {
            tracing::error!("followup /solde impossible: {e}");
        }
    }

    async fn handle_donner(&self, ctx: &Context, cmd: &CommandInteraction) {
        let Some(guild_id) = self.require_guild(ctx, cmd).await else {
            return;
        };

        let Some(target_id) = option_user(cmd, "membre") else {
            self.reply_error(ctx, cmd, "Indique le membre a qui donner.")
                .await;
            return;
        };
        let Some(amount) = option_integer(cmd, "montant") else {
            self.reply_error(ctx, cmd, "Indique le montant du don.")
                .await;
            return;
        };
        let reason = option_string(cmd, "raison");

        // Pre-checks UI rapides (la regle de verite reste cote core/API :
        // auto-transfert, montant > 0, solde suffisant sans clamp).
        if target_id == cmd.user.id {
            self.reply_error(ctx, cmd, "Tu ne peux pas te donner a toi-meme !")
                .await;
            return;
        }
        let target_user = cmd.data.resolved.users.get(&target_id);
        if target_user.is_some_and(|u| u.bot) {
            self.reply_error(ctx, cmd, "Tu ne peux pas donner a un bot !")
                .await;
            return;
        }
        let target_username = target_user
            .map(|u| u.display_name().to_string())
            .unwrap_or_default();

        if let Err(e) = cmd.defer(&ctx.http).await {
            tracing::error!("defer /donner impossible: {e}");
            return;
        }

        let response = self
            .api
            .transfer_coins(
                &guild_id,
                &api_client::TransferRequest {
                    from_user_id: cmd.user.id.to_string(),
                    from_username: cmd.user.display_name().to_string(),
                    to_user_id: target_id.to_string(),
                    to_username: target_username,
                    amount,
                    reason: reason.clone(),
                },
            )
            .await;

        let embed = match &response {
            Ok(res) => embeds::build_transfer_embed(
                cmd.user.id.get(),
                target_id.get(),
                res.amount,
                res.from_balance,
                reason.as_deref(),
            ),
            Err(msg) => embeds::build_error_embed(msg),
        };
        let builder = serenity::all::CreateInteractionResponseFollowup::new().embed(embed);
        if let Err(e) = cmd.create_followup(&ctx.http, builder).await {
            tracing::error!("followup /donner impossible: {e}");
        }
    }

    async fn handle_classement(&self, ctx: &Context, cmd: &CommandInteraction) {
        let Some(guild_id) = self.require_guild(ctx, cmd).await else {
            return;
        };

        if let Err(e) = cmd.defer(&ctx.http).await {
            tracing::error!("defer /classement impossible: {e}");
            return;
        }

        let response = self.api.wallet_leaderboard(&guild_id, 10).await;
        let embed = match &response {
            Ok(entries) => embeds::build_leaderboard_embed(entries),
            Err(msg) => embeds::build_error_embed(msg),
        };
        let builder = serenity::all::CreateInteractionResponseFollowup::new().embed(embed);
        if let Err(e) = cmd.create_followup(&ctx.http, builder).await {
            tracing::error!("followup /classement impossible: {e}");
        }
    }

    async fn handle_roue(&self, ctx: &Context, cmd: &CommandInteraction) {
        let Some(guild_id) = cmd.guild_id else {
            let msg = CreateInteractionResponseMessage::new()
                .embed(embeds::build_error_embed(
                    "La Roue se tire sur un serveur, pas en MP.",
                ))
                .ephemeral(true);
            let _ = cmd
                .create_response(&ctx.http, CreateInteractionResponse::Message(msg))
                .await;
            return;
        };

        // Defer : l'appel API peut prendre > 3s (cold start).
        if let Err(e) = cmd.defer(&ctx.http).await {
            tracing::error!("defer /roue impossible: {e}");
            return;
        }

        let username = cmd.user.display_name().to_string();
        let response = self
            .api
            .spin_wheel(&guild_id.to_string(), &cmd.user.id.to_string(), &username)
            .await;

        let embed = match &response {
            Ok(resp) => embeds::build_result_embed(resp, &username),
            Err(msg) => embeds::build_error_embed(msg),
        };
        let builder = serenity::all::CreateInteractionResponseFollowup::new().embed(embed);
        if let Err(e) = cmd.create_followup(&ctx.http, builder).await {
            tracing::error!("followup /roue impossible: {e}");
        }
    }

    async fn handle_coussin(&self, ctx: &Context, cmd: &CommandInteraction) {
        let Some(guild_id) = self.require_guild(ctx, cmd).await else {
            return;
        };
        let Some(defender_id) = option_user(cmd, "membre") else {
            self.reply_error(ctx, cmd, "Indique qui va s'asseoir dessus.")
                .await;
            return;
        };
        let Some(mise) = option_integer(cmd, "mise") else {
            self.reply_error(ctx, cmd, "Indique une mise valide.").await;
            return;
        };
        if defender_id == cmd.user.id {
            self.reply_error(ctx, cmd, "Te piéger toi-même, c'est juste t'asseoir.")
                .await;
            return;
        }
        let defender = cmd.data.resolved.users.get(&defender_id);
        if defender.is_some_and(|u| u.bot) {
            self.reply_error(ctx, cmd, "Un bot ne s'assoit jamais.")
                .await;
            return;
        }
        if let Err(e) = cmd.defer(&ctx.http).await {
            tracing::error!("defer /coussin impossible: {e}");
            return;
        }
        let response = self
            .api
            .challenge_coussin(
                &guild_id,
                &api_client::CoussinChallengeRequest {
                    channel_id: cmd.channel_id.to_string(),
                    attacker_id: cmd.user.id.to_string(),
                    attacker_name: cmd.user.display_name().to_string(),
                    defender_id: defender_id.to_string(),
                    defender_name: defender
                        .map(|u| u.display_name().to_string())
                        .unwrap_or_else(|| format!("<@{defender_id}>")),
                    mise,
                },
            )
            .await;
        match response {
            Ok(combat) => {
                let buttons = CreateActionRow::Buttons(vec![
                    CreateButton::new(format!("c:a:{}:{}:{}", combat.id, defender_id, cmd.user.id))
                        .label("Accepter")
                        .style(ButtonStyle::Success),
                    CreateButton::new(format!("c:r:{}:{}:{}", combat.id, defender_id, cmd.user.id))
                        .label("Refuser")
                        .style(ButtonStyle::Danger),
                ]);
                let message = serenity::all::CreateInteractionResponseFollowup::new()
                    .embed(embeds::build_coussin_challenge_embed(
                        cmd.user.id.get(),
                        defender_id.get(),
                        combat.mise,
                    ))
                    .components(vec![buttons]);
                if let Err(e) = cmd.create_followup(&ctx.http, message).await {
                    tracing::error!("envoi defi Coussin impossible: {e}");
                }
            }
            Err(message) => self.reply_error(ctx, cmd, &message).await,
        }
    }

    async fn handle_coussin_profile(&self, ctx: &Context, cmd: &CommandInteraction) {
        let Some(guild_id) = self.require_guild(ctx, cmd).await else {
            return;
        };
        let user_id = option_user(cmd, "membre").unwrap_or(cmd.user.id);
        let username = if user_id == cmd.user.id {
            cmd.user.display_name().to_string()
        } else {
            cmd.data
                .resolved
                .users
                .get(&user_id)
                .map(|u| u.display_name().to_string())
                .unwrap_or_default()
        };
        if let Err(e) = cmd.defer_ephemeral(&ctx.http).await {
            tracing::error!("defer /profil impossible: {e}");
            return;
        }
        let embed = match self
            .api
            .coussin_profile(&guild_id, &user_id.to_string(), &username)
            .await
        {
            Ok(profile) => embeds::build_coussin_profile_embed(&profile),
            Err(message) => embeds::build_error_embed(&message),
        };
        let _ = cmd
            .create_followup(
                &ctx.http,
                serenity::all::CreateInteractionResponseFollowup::new()
                    .embed(embed)
                    .ephemeral(true),
            )
            .await;
    }

    async fn handle_coussin_class(&self, ctx: &Context, cmd: &CommandInteraction) {
        let Some(guild_id) = self.require_guild(ctx, cmd).await else {
            return;
        };
        let Some(class) = option_string(cmd, "classe") else {
            self.reply_error(ctx, cmd, "Choisis ta maniere d'occuper le canape.")
                .await;
            return;
        };
        if let Err(e) = cmd.defer_ephemeral(&ctx.http).await {
            tracing::error!("defer /classe impossible: {e}");
            return;
        }
        let embed = match self
            .api
            .choose_coussin_class(
                &guild_id,
                &cmd.user.id.to_string(),
                cmd.user.display_name(),
                &class,
            )
            .await
        {
            Ok(profile) => embeds::build_coussin_profile_embed(&profile),
            Err(message) => embeds::build_error_embed(&message),
        };
        let _ = cmd
            .create_followup(
                &ctx.http,
                serenity::all::CreateInteractionResponseFollowup::new()
                    .embed(embed)
                    .ephemeral(true),
            )
            .await;
    }

    async fn handle_coussin_train(&self, ctx: &Context, cmd: &CommandInteraction) {
        let Some(guild_id) = self.require_guild(ctx, cmd).await else {
            return;
        };
        let Some(stat) = option_string(cmd, "stat") else {
            self.reply_error(ctx, cmd, "Choisis : impact ou moelleux.")
                .await;
            return;
        };
        if let Err(e) = cmd.defer_ephemeral(&ctx.http).await {
            tracing::error!("defer /train impossible: {e}");
            return;
        }
        let embed = match self
            .api
            .train_coussin(
                &guild_id,
                &cmd.user.id.to_string(),
                cmd.user.display_name(),
                &stat,
            )
            .await
        {
            Ok(profile) => embeds::build_coussin_profile_embed(&profile),
            Err(message) => embeds::build_error_embed(&message),
        };
        let _ = cmd
            .create_followup(
                &ctx.http,
                serenity::all::CreateInteractionResponseFollowup::new()
                    .embed(embed)
                    .ephemeral(true),
            )
            .await;
    }
    async fn handle_coussin_shop(&self, ctx: &Context, cmd: &CommandInteraction) {
        let Some(guild_id) = self.require_guild(ctx, cmd).await else {
            return;
        };
        let Some(item) = option_string(cmd, "objet") else {
            return;
        };
        if let Err(e) = cmd.defer_ephemeral(&ctx.http).await {
            tracing::error!("defer /shop impossible: {e}");
            return;
        }
        let embed = match self
            .api
            .buy_coussin_item(&guild_id, &cmd.user.id.to_string(), &item)
            .await
        {
            Ok(balance) => embeds::build_coussin_purchase_embed(&item, balance),
            Err(e) => embeds::build_error_embed(&e),
        };
        let _ = cmd
            .create_followup(
                &ctx.http,
                serenity::all::CreateInteractionResponseFollowup::new()
                    .embed(embed)
                    .ephemeral(true),
            )
            .await;
    }
    async fn handle_coussin_insurance(&self, ctx: &Context, cmd: &CommandInteraction) {
        let Some(guild_id) = self.require_guild(ctx, cmd).await else {
            return;
        };
        if let Err(e) = cmd.defer_ephemeral(&ctx.http).await {
            tracing::error!("defer /garantie impossible: {e}");
            return;
        }
        let embed = match self
            .api
            .buy_coussin_insurance(&guild_id, &cmd.user.id.to_string())
            .await
        {
            Ok((scam, expires)) => embeds::build_coussin_insurance_embed(scam, &expires),
            Err(e) => embeds::build_error_embed(&e),
        };
        let _ = cmd
            .create_followup(
                &ctx.http,
                serenity::all::CreateInteractionResponseFollowup::new()
                    .embed(embed)
                    .ephemeral(true),
            )
            .await;
    }
    async fn handle_steal(&self, ctx: &Context, cmd: &CommandInteraction) {
        let Some(guild) = self.require_guild(ctx, cmd).await else {
            return;
        };
        let Some(target) = option_user(cmd, "membre") else {
            return;
        };
        let name = cmd
            .data
            .resolved
            .users
            .get(&target)
            .map(|u| u.display_name().to_string())
            .unwrap_or_default();
        if cmd.defer(&ctx.http).await.is_err() {
            return;
        }
        let text = match self
            .api
            .steal_coussin(
                &guild,
                &cmd.user.id.to_string(),
                &api_client::CoussinStealRequest {
                    thief_name: cmd.user.display_name().into(),
                    victim_id: target.to_string(),
                    victim_name: name,
                },
            )
            .await
        {
            Ok((true, n)) => format!("🪙 Trouvé sous les coussins : {n} coins."),
            Ok((false, n)) => format!("🙈 Pris la main dans le canapé : {n} coins perdus."),
            Err(e) => e,
        };
        let _ = cmd
            .create_followup(
                &ctx.http,
                serenity::all::CreateInteractionResponseFollowup::new().content(text),
            )
            .await;
    }
    async fn handle_prime(&self, ctx: &Context, cmd: &CommandInteraction) {
        let Some(guild) = self.require_guild(ctx, cmd).await else {
            return;
        };
        let (Some(target), Some(amount)) =
            (option_user(cmd, "membre"), option_integer(cmd, "montant"))
        else {
            return;
        };
        let name = cmd
            .data
            .resolved
            .users
            .get(&target)
            .map(|u| u.display_name().to_string())
            .unwrap_or_default();
        if cmd.defer(&ctx.http).await.is_err() {
            return;
        }
        let text = match self
            .api
            .prime_coussin(
                &guild,
                &cmd.user.id.to_string(),
                &api_client::CoussinPrimeRequest {
                    target_id: target.to_string(),
                    target_name: name,
                    placer_name: cmd.user.display_name().into(),
                    amount,
                },
            )
            .await
        {
            Ok(()) => format!("📜 Contrat de {amount} coins sur <@{target}> : fais-le lever."),
            Err(e) => e,
        };
        let _ = cmd
            .create_followup(
                &ctx.http,
                serenity::all::CreateInteractionResponseFollowup::new().content(text),
            )
            .await;
    }

    async fn handle_inventory(&self, ctx: &Context, cmd: &CommandInteraction) {
        let Some(guild) = self.require_guild(ctx, cmd).await else {
            return;
        };
        if cmd.defer_ephemeral(&ctx.http).await.is_err() {
            return;
        }
        let embed = match self
            .api
            .inventory_coussin(&guild, &cmd.user.id.to_string())
            .await
        {
            Ok(items) => embeds::build_coussin_inventory_embed(&items),
            Err(e) => embeds::build_error_embed(&e),
        };
        let _ = cmd
            .create_followup(
                &ctx.http,
                serenity::all::CreateInteractionResponseFollowup::new()
                    .embed(embed)
                    .ephemeral(true),
            )
            .await;
    }

    async fn handle_bet(&self, ctx: &Context, cmd: &CommandInteraction) {
        let Some(guild) = self.require_guild(ctx, cmd).await else {
            return;
        };
        let (Some(id), Some(target), Some(amount)) = (
            option_string(cmd, "combat"),
            option_user(cmd, "membre"),
            option_integer(cmd, "montant"),
        ) else {
            return;
        };
        if cmd.defer(&ctx.http).await.is_err() {
            return;
        }
        let text = match self
            .api
            .bet_coussin(
                &guild,
                &cmd.user.id.to_string(),
                &api_client::CoussinBetRequest {
                    combat_id: id,
                    bettor_name: cmd.user.display_name().into(),
                    backed_id: target.to_string(),
                    amount,
                },
            )
            .await
        {
            Ok(()) => format!("🍿 {amount} coins sur <@{target}>. Popcorn servi."),
            Err(e) => e,
        };
        let _ = cmd
            .create_followup(
                &ctx.http,
                serenity::all::CreateInteractionResponseFollowup::new().content(text),
            )
            .await;
    }

    async fn handle_coussin_component(
        &self,
        ctx: &Context,
        component: &serenity::all::ComponentInteraction,
    ) {
        let parts: Vec<_> = component.data.custom_id.split(':').collect();
        if parts.len() != 5 || parts[0] != "c" {
            return;
        }
        let (action, combat_id, defender_id, attacker_id) =
            (parts[1], parts[2], parts[3], parts[4]);
        if component.user.id.to_string() != defender_id {
            let _ = component
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("Seul l'adversaire peut repondre a ce defi.")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }
        if action == "r" {
            match self.api.refuse_coussin(combat_id, defender_id).await {
                Ok(true) => {
                    let _ = component
                        .create_response(
                            &ctx.http,
                            CreateInteractionResponse::UpdateMessage(
                                CreateInteractionResponseMessage::new()
                                    .content("👊 Duel refuse.")
                                    .components(vec![]),
                            ),
                        )
                        .await;
                }
                Ok(false) => {
                    self.component_error(ctx, component, "Ce defi n'est plus disponible.")
                        .await
                }
                Err(e) => self.component_error(ctx, component, &e).await,
            }
            return;
        }
        if action != "a" {
            return;
        }
        match self.api.accept_coussin(combat_id, defender_id).await {
            Ok(true) => {
                let Ok(attacker_id) = attacker_id.parse::<u64>() else {
                    self.component_error(ctx, component, "Defi invalide.").await;
                    return;
                };
                match self.api.resolve_coussin(combat_id).await {
                    Ok(true) => {
                        let _ = component.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(CreateInteractionResponseMessage::new().content(format!("👊 Duel resolu entre <@{}> et <@{}>. Consultez /profil pour voir les consequences.", attacker_id, component.user.id)).components(vec![]))).await;
                    }
                    Ok(false) => {
                        self.component_error(ctx, component, "Le duel n'a pas pu etre resolu.")
                            .await
                    }
                    Err(e) => self.component_error(ctx, component, &e).await,
                }
            }
            Ok(false) => {
                self.component_error(ctx, component, "Ce defi n'est plus disponible.")
                    .await
            }
            Err(e) => self.component_error(ctx, component, &e).await,
        }
    }

    async fn component_error(
        &self,
        ctx: &Context,
        component: &serenity::all::ComponentInteraction,
        message: &str,
    ) {
        let _ = component
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(message)
                        .ephemeral(true),
                ),
            )
            .await;
    }
}

/// La guilde est-elle celle servie par cette installation ?
///
/// `PUBLIC_GUILD_ID` absente = aucun verrou. C'est le seul defaut sur :
/// refuser par defaut ferait quitter tous ses serveurs au bot au premier
/// demarrage mal configure, et un depart ne se rattrape pas d'un clic.
fn guilde_autorisee(guild_id: serenity::model::id::GuildId) -> bool {
    let attendu = std::env::var("PUBLIC_GUILD_ID")
        .or_else(|_| std::env::var("GUILD_ID"))
        .unwrap_or_default();
    let attendu = attendu.trim();

    attendu.is_empty() || attendu == guild_id.to_string()
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
            games::spawn_listener(ctx.clone(), self.api.clone());
        }
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
        let commands: Vec<CreateCommand> = commands
            .into_iter()
            .chain(games::register_commands())
            .chain(std::iter::once(wheel_panel::register()))
            .collect();
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
                _ => {}
            },
            Interaction::Component(component) => {
                let cid = component.data.custom_id.as_str();
                if cid.starts_with("c:") {
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

    let mut client = Client::builder(&token, GatewayIntents::non_privileged())
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
