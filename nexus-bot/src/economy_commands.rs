use super::*;

impl Handler {
    /// Reponse ephemere avec l'embed d'erreur standard.
    pub(super) async fn reply_error(&self, ctx: &Context, cmd: &CommandInteraction, message: &str) {
        let msg = CreateInteractionResponseMessage::new()
            .embed(embeds::build_error_embed(message))
            .ephemeral(true);
        let _ = cmd
            .create_response(&ctx.http, CreateInteractionResponse::Message(msg))
            .await;
    }

    /// Exige un serveur : retourne le guild_id ou repond une erreur ephemere.
    pub(super) async fn require_guild(
        &self,
        ctx: &Context,
        cmd: &CommandInteraction,
    ) -> Option<String> {
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

    pub(super) async fn handle_solde(&self, ctx: &Context, cmd: &CommandInteraction) {
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

    pub(super) async fn handle_donner(&self, ctx: &Context, cmd: &CommandInteraction) {
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

    pub(super) async fn handle_classement(&self, ctx: &Context, cmd: &CommandInteraction) {
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

    pub(super) async fn handle_roue(&self, ctx: &Context, cmd: &CommandInteraction) {
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
}
