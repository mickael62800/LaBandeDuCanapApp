use super::*;

impl Handler {
    pub(super) async fn handle_coussin(&self, ctx: &Context, cmd: &CommandInteraction) {
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

    pub(super) async fn handle_coussin_profile(&self, ctx: &Context, cmd: &CommandInteraction) {
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

    pub(super) async fn handle_coussin_class(&self, ctx: &Context, cmd: &CommandInteraction) {
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

    pub(super) async fn handle_coussin_train(&self, ctx: &Context, cmd: &CommandInteraction) {
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
    pub(super) async fn handle_coussin_shop(&self, ctx: &Context, cmd: &CommandInteraction) {
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
    pub(super) async fn handle_coussin_insurance(&self, ctx: &Context, cmd: &CommandInteraction) {
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
    /// `/chiper` — ouvre une fouille et laisse a la cible le temps de reagir.
    ///
    /// Rien n'est joue a cet instant : la victime peut serrer les coussins
    /// pendant la fenetre et garder toute sa defense. Si elle ne dit rien, le
    /// job tranche avec le malus d'absence et le voleur passe beaucoup plus
    /// facilement. Le vol se decidait avant sur un simple tirage, sans que la
    /// cible puisse quoi que ce soit.
    pub(super) async fn handle_steal(&self, ctx: &Context, cmd: &CommandInteraction) {
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

        let opened = match self
            .api
            .steal_coussin(
                &guild,
                &cmd.user.id.to_string(),
                &api_client::CoussinStealRequest {
                    thief_name: cmd.user.display_name().into(),
                    victim_id: target.to_string(),
                    victim_name: name,
                    channel_id: cmd.channel_id.to_string(),
                },
            )
            .await
        {
            Ok(opened) => opened,
            Err(message) => {
                let _ = cmd
                    .create_followup(
                        &ctx.http,
                        serenity::all::CreateInteractionResponseFollowup::new().content(message),
                    )
                    .await;
                return;
            }
        };

        // Le bouton ne s'adresse qu'a la victime : son identifiant est dans le
        // custom_id, et l'API revalide qui clique. Un bouton visible de tous
        // n'est pas un bouton ouvert a tous.
        let bouton = CreateActionRow::Buttons(vec![CreateButton::new(format!(
            "cs:d:{}:{}",
            opened.attempt_id, target
        ))
        .label("Serrer les coussins")
        .style(ButtonStyle::Primary)]);

        let message = serenity::all::CreateInteractionResponseFollowup::new()
            .content(format!(
                "🛋️ <@{}> fouille les coussins de <@{}> !
                 <@{}>, tu as **{} secondes** pour réagir — sans quoi tu te feras chiper bien plus facilement.",
                cmd.user.id, target, target, opened.defense_window_seconds
            ))
            .components(vec![bouton]);

        match cmd.create_followup(&ctx.http, message).await {
            Ok(posted) => {
                // Rattache le message : le denouement doit pouvoir etre publie
                // au bon endroit meme si le bot redemarre entre-temps.
                if let Err(error) = self
                    .api
                    .attach_steal_message(&opened.attempt_id, &posted.id.to_string())
                    .await
                {
                    tracing::warn!(%error, "fouille : message non rattache");
                }
            }
            Err(error) => tracing::error!(%error, "envoi de la fouille impossible"),
        }
    }

    /// Clic sur « Serrer les coussins » : la victime resout tout de suite, avec
    /// sa defense pleine.
    pub(super) async fn handle_steal_component(
        &self,
        ctx: &Context,
        component: &serenity::all::ComponentInteraction,
    ) {
        let parts: Vec<_> = component.data.custom_id.split(':').collect();
        if parts.len() != 4 || parts[0] != "cs" || parts[1] != "d" {
            return;
        }
        let (attempt_id, victim_id) = (parts[2], parts[3]);

        // ACCUSE IMMEDIAT : la resolution passe par l'API, et Discord ferme
        // l'interaction au bout de 3 secondes.
        if let Err(error) = component
            .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
            .await
        {
            tracing::warn!(%error, "fouille: accuse de reception impossible");
            return;
        }

        if component.user.id.to_string() != victim_id {
            self.component_error(ctx, component, "Ce sont les coussins de quelqu'un d'autre.")
                .await;
            return;
        }

        match self.api.defend_steal(attempt_id, victim_id).await {
            Ok(outcome) => {
                let recit = if outcome.success {
                    format!(
                        "🪙 Trop tard : <@{}> repart avec **{}** coins.",
                        outcome.thief_id, outcome.amount
                    )
                } else {
                    format!(
                        "🛡️ Coussins bien serrés ! <@{}> repart bredouille et perd **{}** coins.",
                        outcome.thief_id, outcome.amount
                    )
                };
                let detail = format!(
                    "
🎲 Voleur : **{}** — Défense : **{}**",
                    outcome.thief_total, outcome.victim_total
                );
                let _ = component
                    .edit_response(
                        &ctx.http,
                        serenity::all::EditInteractionResponse::new()
                            .content(format!("{recit}{detail}"))
                            .components(vec![]),
                    )
                    .await;
            }
            Err(message) => self.component_error(ctx, component, &message).await,
        }
    }

    pub(super) async fn handle_prime(&self, ctx: &Context, cmd: &CommandInteraction) {
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

    pub(super) async fn handle_inventory(&self, ctx: &Context, cmd: &CommandInteraction) {
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

    pub(super) async fn handle_bet(&self, ctx: &Context, cmd: &CommandInteraction) {
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

    pub(super) async fn handle_coussin_component(
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

        // ACCUSE IMMEDIAT : accepter un defi enchaine l'acceptation puis la
        // resolution, soit deux allers-retours API la ou Discord ferme
        // l'interaction au bout de 3 secondes.
        if let Err(error) = component
            .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
            .await
        {
            tracing::warn!(%error, "coussin: accuse de reception impossible");
            return;
        }

        if component.user.id.to_string() != defender_id {
            self.component_error(ctx, component, "Seul l'adversaire peut repondre a ce defi.")
                .await;
            return;
        }
        if action == "r" {
            match self.api.refuse_coussin(combat_id, defender_id).await {
                Ok(true) => {
                    let _ = component
                        .edit_response(
                            &ctx.http,
                            serenity::all::EditInteractionResponse::new()
                                .content("👊 Duel refuse.")
                                .components(vec![]),
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
                        let _ = component
                            .edit_response(
                                &ctx.http,
                                serenity::all::EditInteractionResponse::new()
                                    .content(format!(
                                        "👊 Duel resolu entre <@{}> et <@{}>. Consultez /profil pour voir les consequences.",
                                        attacker_id,
                                        component.user.id
                                    ))
                                    .components(vec![]),
                            )
                            .await;
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

    /// Message d'erreur prive apres un clic.
    ///
    /// Un followup, et non une reponse initiale : l'interaction est acquittee
    /// des l'entree du handler, sans quoi les appels API qui suivent depassent
    /// les 3 s accordees par Discord.
    async fn component_error(
        &self,
        ctx: &Context,
        component: &serenity::all::ComponentInteraction,
        message: &str,
    ) {
        let _ = component
            .create_followup(
                &ctx.http,
                serenity::all::CreateInteractionResponseFollowup::new()
                    .content(message)
                    .ephemeral(true),
            )
            .await;
    }
}
