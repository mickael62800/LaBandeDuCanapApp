use super::*;

pub const MSG_GUILD_REQUIRED: &str = "Cette commande s'utilise sur un serveur, pas en MP.";
pub const MSG_GUILD_UNAUTHORIZED: &str = "Ce bot ne sert pas ce serveur.";
pub const MSG_DONNER_NEED_TARGET: &str = "Indique le membre a qui donner.";
pub const MSG_DONNER_NEED_AMOUNT: &str = "Indique le montant du don.";
pub const MSG_ROUE_NO_DM: &str = "La Roue se tire sur un serveur, pas en MP.";

impl Handler {
    /// Reponse ephemere avec l'embed d'erreur standard.
    pub(super) async fn reply_error(&self, ctx: &Context, cmd: &CommandInteraction, message: &str) {
        let _ = cmd
            .create_response(&ctx.http, build_error_message(message))
            .await;
    }

    /// Exige un serveur : retourne le guild_id ou repond une erreur ephemere.
    pub(super) async fn require_guild(
        &self,
        ctx: &Context,
        cmd: &CommandInteraction,
    ) -> Option<String> {
        let Some(g) = cmd.guild_id else {
            self.reply_error(ctx, cmd, MSG_GUILD_REQUIRED).await;
            return None;
        };

        // Mono-serveur. Ce controle vit ici parce que TOUTES les commandes
        // passent par `require_guild` : le placer dans chaque handler en
        // aurait laisse passer au moins un.
        //
        // L'API refuserait de toute facon, mais avec une erreur technique
        // illisible ; autant repondre clairement.
        if !guilde_autorisee(g) {
            self.reply_error(ctx, cmd, MSG_GUILD_UNAUTHORIZED).await;
            return None;
        }

        Some(g.to_string())
    }

    pub(super) async fn handle_solde(&self, ctx: &Context, cmd: &CommandInteraction) {
        let Some(guild_id) = self.require_guild(ctx, cmd).await else {
            return;
        };

        let target_id = option_user(cmd, "membre").unwrap_or(cmd.user.id);
        let display_name = format_target_name(
            target_id,
            cmd.user.id,
            cmd.user.display_name(),
            cmd.data
                .resolved
                .users
                .get(&target_id)
                .map(|u| u.display_name()),
        );

        if let Err(e) = cmd.defer(&ctx.http).await {
            tracing::error!("defer /solde impossible: {e}");
            return;
        }

        let embed =
            execute_solde(&self.api, &guild_id, &target_id.to_string(), &display_name).await;
        let builder = build_embed_followup(embed);
        if let Err(e) = cmd.create_followup(&ctx.http, builder).await {
            tracing::error!("followup /solde impossible: {e}");
        }
    }

    pub(super) async fn handle_donner(&self, ctx: &Context, cmd: &CommandInteraction) {
        let Some(guild_id) = self.require_guild(ctx, cmd).await else {
            return;
        };

        let Some(target_id) = option_user(cmd, "membre") else {
            self.reply_error(ctx, cmd, MSG_DONNER_NEED_TARGET).await;
            return;
        };
        let Some(amount) = option_integer(cmd, "montant") else {
            self.reply_error(ctx, cmd, MSG_DONNER_NEED_AMOUNT).await;
            return;
        };
        let reason = option_string(cmd, "raison");

        // Pre-checks UI rapides (la regle de verite reste cote core/API :
        // auto-transfert, montant > 0, solde suffisant sans clamp).
        let target_user = cmd.data.resolved.users.get(&target_id);
        let is_bot = target_user.is_some_and(|u| u.bot);
        if let Err(err_msg) = validate_donner(cmd.user.id, target_id, is_bot) {
            self.reply_error(ctx, cmd, err_msg).await;
            return;
        }
        let target_username = target_user
            .map(|u| u.display_name().to_string())
            .unwrap_or_default();

        if let Err(e) = cmd.defer(&ctx.http).await {
            tracing::error!("defer /donner impossible: {e}");
            return;
        }

        let req = build_transfer_request_payload(
            cmd.user.id,
            cmd.user.display_name(),
            target_id,
            &target_username,
            amount,
            reason.clone(),
        );

        let embed = execute_donner(
            &self.api,
            &guild_id,
            req,
            cmd.user.id.get(),
            target_id.get(),
            reason.as_deref(),
        )
        .await;
        let builder = build_embed_followup(embed);
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

        let embed = execute_classement(&self.api, &guild_id, 10).await;
        let builder = build_embed_followup(embed);
        if let Err(e) = cmd.create_followup(&ctx.http, builder).await {
            tracing::error!("followup /classement impossible: {e}");
        }
    }

    pub(super) async fn handle_roue(&self, ctx: &Context, cmd: &CommandInteraction) {
        let Some(guild_id) = cmd.guild_id else {
            let _ = cmd
                .create_response(&ctx.http, build_error_message(MSG_ROUE_NO_DM))
                .await;
            return;
        };

        // Defer : l'appel API peut prendre > 3s (cold start).
        if let Err(e) = cmd.defer(&ctx.http).await {
            tracing::error!("defer /roue impossible: {e}");
            return;
        }

        let username = cmd.user.display_name().to_string();
        let embed = execute_roue(
            &self.api,
            &guild_id.to_string(),
            &cmd.user.id.to_string(),
            &username,
        )
        .await;
        let builder = build_embed_followup(embed);
        if let Err(e) = cmd.create_followup(&ctx.http, builder).await {
            tracing::error!("followup /roue impossible: {e}");
        }
    }
}

pub async fn execute_solde(
    api: &ApiClient,
    guild_id: &str,
    target_id: &str,
    display_name: &str,
) -> serenity::builder::CreateEmbed {
    let response = api.get_wallet(guild_id, target_id).await;
    build_wallet_response_embed(&response, display_name)
}

pub async fn execute_donner(
    api: &ApiClient,
    guild_id: &str,
    req: api_client::TransferRequest,
    from_user_id: u64,
    to_user_id: u64,
    reason: Option<&str>,
) -> serenity::builder::CreateEmbed {
    let response = api.transfer_coins(guild_id, &req).await;
    build_transfer_response_embed(&response, from_user_id, to_user_id, reason)
}

pub async fn execute_classement(
    api: &ApiClient,
    guild_id: &str,
    limit: i64,
) -> serenity::builder::CreateEmbed {
    let response = api.wallet_leaderboard(guild_id, limit).await;
    build_leaderboard_response_embed(&response)
}

pub async fn execute_roue(
    api: &ApiClient,
    guild_id: &str,
    user_id: &str,
    username: &str,
) -> serenity::builder::CreateEmbed {
    let response = api.spin_wheel(guild_id, user_id, username).await;
    build_spin_response_embed(&response, username)
}

pub fn build_error_message(message: &str) -> CreateInteractionResponse {
    let msg = CreateInteractionResponseMessage::new()
        .embed(embeds::build_error_embed(message))
        .ephemeral(true);
    CreateInteractionResponse::Message(msg)
}

pub fn build_embed_followup(
    embed: serenity::builder::CreateEmbed,
) -> serenity::all::CreateInteractionResponseFollowup {
    serenity::all::CreateInteractionResponseFollowup::new().embed(embed)
}

pub fn build_wallet_response_embed(
    response: &Result<api_client::WalletResponse, String>,
    display_name: &str,
) -> serenity::builder::CreateEmbed {
    match response {
        Ok(w) => embeds::build_wallet_embed(w, display_name),
        Err(msg) => embeds::build_error_embed(msg),
    }
}

pub fn build_transfer_response_embed(
    response: &Result<api_client::TransferResponse, String>,
    from_user_id: u64,
    to_user_id: u64,
    reason: Option<&str>,
) -> serenity::builder::CreateEmbed {
    match response {
        Ok(res) => embeds::build_transfer_embed(
            from_user_id,
            to_user_id,
            res.amount,
            res.from_balance,
            reason,
        ),
        Err(msg) => embeds::build_error_embed(msg),
    }
}

pub fn build_leaderboard_response_embed(
    response: &Result<Vec<api_client::WalletResponse>, String>,
) -> serenity::builder::CreateEmbed {
    match response {
        Ok(entries) => embeds::build_leaderboard_embed(entries),
        Err(msg) => embeds::build_error_embed(msg),
    }
}

pub fn build_spin_response_embed(
    response: &Result<api_client::WheelSpinResponse, String>,
    username: &str,
) -> serenity::builder::CreateEmbed {
    match response {
        Ok(resp) => embeds::build_result_embed(resp, username),
        Err(msg) => embeds::build_error_embed(msg),
    }
}

pub fn format_target_name(
    target_id: UserId,
    user_id: UserId,
    user_display_name: &str,
    resolved_name: Option<&str>,
) -> String {
    if target_id == user_id {
        user_display_name.to_string()
    } else {
        resolved_name
            .map(|u| u.to_string())
            .unwrap_or_else(|| format!("<@{target_id}>"))
    }
}

pub fn validate_donner(
    sender_id: UserId,
    target_id: UserId,
    is_target_bot: bool,
) -> Result<(), &'static str> {
    if target_id == sender_id {
        return Err("Tu ne peux pas te donner a toi-meme !");
    }
    if is_target_bot {
        return Err("Tu ne peux pas donner a un bot !");
    }
    Ok(())
}

pub fn build_transfer_request_payload(
    user_id: UserId,
    user_display_name: &str,
    target_id: UserId,
    target_username: &str,
    amount: i64,
    reason: Option<String>,
) -> api_client::TransferRequest {
    api_client::TransferRequest {
        from_user_id: user_id.to_string(),
        from_username: user_display_name.to_string(),
        to_user_id: target_id.to_string(),
        to_username: target_username.to_string(),
        amount,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_donner() {
        let user1 = UserId::new(100);
        let user2 = UserId::new(200);

        assert!(validate_donner(user1, user2, false).is_ok());
        assert_eq!(
            validate_donner(user1, user1, false),
            Err("Tu ne peux pas te donner a toi-meme !")
        );
        assert_eq!(
            validate_donner(user1, user2, true),
            Err("Tu ne peux pas donner a un bot !")
        );
    }

    #[test]
    fn test_format_target_name() {
        let self_id = UserId::new(1);
        let other_id = UserId::new(2);

        assert_eq!(format_target_name(self_id, self_id, "Alice", None), "Alice");
        assert_eq!(
            format_target_name(other_id, self_id, "Alice", Some("Bob")),
            "Bob"
        );
        assert_eq!(format_target_name(other_id, self_id, "Alice", None), "<@2>");

        let req = build_transfer_request_payload(
            self_id,
            "Alice",
            other_id,
            "Bob",
            50,
            Some("Cadeau".into()),
        );
        assert_eq!(req.from_user_id, "1");
        assert_eq!(req.from_username, "Alice");
        assert_eq!(req.to_user_id, "2");
        assert_eq!(req.to_username, "Bob");
        assert_eq!(req.amount, 50);
        assert_eq!(req.reason, Some("Cadeau".into()));
    }

    #[test]
    fn test_build_embed_helpers() {
        let w_ok = Ok(api_client::WalletResponse {
            user_id: "u1".into(),
            coins: 500,
            total_earned: 1000,
            total_spent: 500,
        });
        let emb_w = build_wallet_response_embed(&w_ok, "Alice");
        let j_w = serde_json::to_value(&emb_w).unwrap();
        assert!(j_w["title"].as_str().unwrap().contains("Alice"));

        let w_err: Result<api_client::WalletResponse, String> = Err("Wallet fail".into());
        let emb_w_err = build_wallet_response_embed(&w_err, "Alice");
        let j_w_err = serde_json::to_value(&emb_w_err).unwrap();
        assert_eq!(j_w_err["description"], "Wallet fail");

        let t_ok = Ok(api_client::TransferResponse {
            from_balance: 400,
            amount: 50,
        });
        let emb_t = build_transfer_response_embed(&t_ok, 1, 2, Some("Gift"));
        let j_t = serde_json::to_value(&emb_t).unwrap();
        assert!(j_t["title"].as_str().unwrap().contains("Don"));

        let t_err: Result<api_client::TransferResponse, String> = Err("Transfer fail".into());
        let emb_t_err = build_transfer_response_embed(&t_err, 1, 2, None);
        let j_t_err = serde_json::to_value(&emb_t_err).unwrap();
        assert_eq!(j_t_err["description"], "Transfer fail");

        let lb_ok = Ok(vec![api_client::WalletResponse {
            user_id: "u1".into(),
            coins: 1000,
            total_earned: 2000,
            total_spent: 1000,
        }]);
        let emb_lb = build_leaderboard_response_embed(&lb_ok);
        let j_lb = serde_json::to_value(&emb_lb).unwrap();
        assert!(j_lb["title"].as_str().unwrap().contains("Classement"));

        let lb_err: Result<Vec<api_client::WalletResponse>, String> = Err("LB fail".into());
        let emb_lb_err = build_leaderboard_response_embed(&lb_err);
        let j_lb_err = serde_json::to_value(&emb_lb_err).unwrap();
        assert_eq!(j_lb_err["description"], "LB fail");

        let s_ok = Ok(api_client::WheelSpinResponse {
            case_label: "Licorne".into(),
            payout: 500,
            balance_after: 1500,
            is_memorable: true,
        });
        let emb_s = build_spin_response_embed(&s_ok, "Alice");
        let j_s = serde_json::to_value(&emb_s).unwrap();
        assert!(j_s["title"].as_str().unwrap().contains("Alice"));

        let s_err: Result<api_client::WheelSpinResponse, String> = Err("Spin fail".into());
        let emb_s_err = build_spin_response_embed(&s_err, "Alice");
        let j_s_err = serde_json::to_value(&emb_s_err).unwrap();
        assert_eq!(j_s_err["description"], "Spin fail");

        let err_msg = build_error_message("Erreur critique");
        let j_err_msg = serde_json::to_value(&err_msg).unwrap();
        assert!(j_err_msg["data"]["embeds"].as_array().is_some());

        let followup = build_embed_followup(emb_w);
        let j_followup = serde_json::to_value(&followup).unwrap();
        assert!(j_followup["embeds"].as_array().is_some());

        assert!(MSG_GUILD_REQUIRED.contains("sur un serveur"));
        assert!(MSG_GUILD_UNAUTHORIZED.contains("ne sert pas"));
        assert!(MSG_DONNER_NEED_TARGET.contains("membre"));
        assert!(MSG_DONNER_NEED_AMOUNT.contains("montant"));
        assert!(MSG_ROUE_NO_DM.contains("sur un serveur"));
    }

    #[test]
    fn test_validate_donner_all_error_cases() {
        let user1 = UserId::new(100);
        let user2 = UserId::new(200);

        // Self transfer
        assert_eq!(
            validate_donner(user1, user1, false),
            Err("Tu ne peux pas te donner a toi-meme !")
        );

        // Transfer to bot
        assert_eq!(
            validate_donner(user1, user2, true),
            Err("Tu ne peux pas donner a un bot !")
        );

        // Valid transfer
        assert!(validate_donner(user1, user2, false).is_ok());
    }

    #[test]
    fn test_format_target_name_all_cases() {
        let self_id = UserId::new(1);
        let other_id = UserId::new(2);

        // Self with name
        assert_eq!(format_target_name(self_id, self_id, "Alice", None), "Alice");

        // Other with resolved name
        assert_eq!(
            format_target_name(other_id, self_id, "Alice", Some("Bob")),
            "Bob"
        );

        // Other without resolved name
        assert_eq!(format_target_name(other_id, self_id, "Alice", None), "<@2>");

        // Ensure target's own name doesn't override when target != self
        assert_eq!(
            format_target_name(other_id, self_id, "Alice", Some("Charlie")),
            "Charlie"
        );
    }

    #[test]
    fn test_build_transfer_request_payload_all_cases() {
        let user1 = UserId::new(100);
        let user2 = UserId::new(200);

        // With reason
        let req1 =
            build_transfer_request_payload(user1, "Alice", user2, "Bob", 50, Some("Gift".into()));
        assert_eq!(req1.from_user_id, "100");
        assert_eq!(req1.from_username, "Alice");
        assert_eq!(req1.to_user_id, "200");
        assert_eq!(req1.to_username, "Bob");
        assert_eq!(req1.amount, 50);
        assert_eq!(req1.reason, Some("Gift".into()));

        // Without reason
        let req2 = build_transfer_request_payload(user1, "Alice", user2, "Bob", 100, None);
        assert_eq!(req2.reason, None);
        assert_eq!(req2.amount, 100);

        // Large amount
        let req3 = build_transfer_request_payload(user1, "Alice", user2, "Bob", 1_000_000, None);
        assert_eq!(req3.amount, 1_000_000);
    }

    #[test]
    fn test_build_wallet_response_embed_success() {
        let ok = Ok(api_client::WalletResponse {
            user_id: "u1".into(),
            coins: 500,
            total_earned: 1000,
            total_spent: 500,
        });
        let emb = build_wallet_response_embed(&ok, "TestUser");
        let json = serde_json::to_value(&emb).unwrap();
        assert!(json["title"].as_str().unwrap().contains("TestUser"));
    }

    #[test]
    fn test_build_wallet_response_embed_error() {
        let err: Result<api_client::WalletResponse, String> = Err("API Error".into());
        let emb = build_wallet_response_embed(&err, "TestUser");
        let json = serde_json::to_value(&emb).unwrap();
        assert_eq!(json["description"], "API Error");
    }

    #[test]
    fn test_build_transfer_response_embed_success() {
        let ok = Ok(api_client::TransferResponse {
            from_balance: 400,
            amount: 100,
        });
        let emb = build_transfer_response_embed(&ok, 1, 2, Some("Reason"));
        let json = serde_json::to_value(&emb).unwrap();
        assert!(json["title"].as_str().unwrap().contains("Don"));
    }

    #[test]
    fn test_build_transfer_response_embed_error() {
        let err: Result<api_client::TransferResponse, String> = Err("Transfer failed".into());
        let emb = build_transfer_response_embed(&err, 1, 2, None);
        let json = serde_json::to_value(&emb).unwrap();
        assert_eq!(json["description"], "Transfer failed");
    }

    #[test]
    fn test_build_leaderboard_response_embed_empty() {
        let ok: Result<Vec<api_client::WalletResponse>, String> = Ok(vec![]);
        let emb = build_leaderboard_response_embed(&ok);
        let json = serde_json::to_value(&emb).unwrap();
        assert!(json["title"].as_str().unwrap().contains("Classement"));
    }

    #[test]
    fn test_build_leaderboard_response_embed_error() {
        let err: Result<Vec<api_client::WalletResponse>, String> = Err("LB Error".into());
        let emb = build_leaderboard_response_embed(&err);
        let json = serde_json::to_value(&emb).unwrap();
        assert_eq!(json["description"], "LB Error");
    }

    #[test]
    fn test_build_spin_response_embed_success() {
        let ok = Ok(api_client::WheelSpinResponse {
            case_label: "Jackpot".into(),
            payout: 500,
            balance_after: 1500,
            is_memorable: true,
        });
        let emb = build_spin_response_embed(&ok, "Player");
        let json = serde_json::to_value(&emb).unwrap();
        assert!(json["title"].as_str().unwrap().contains("Player"));
    }

    #[test]
    fn test_build_spin_response_embed_error() {
        let err: Result<api_client::WheelSpinResponse, String> = Err("Spin failed".into());
        let emb = build_spin_response_embed(&err, "Player");
        let json = serde_json::to_value(&emb).unwrap();
        assert_eq!(json["description"], "Spin failed");
    }

    #[test]
    fn test_error_message_ephemeral() {
        let msg = build_error_message("Critical error");
        let json = serde_json::to_value(&msg).unwrap();
        assert!(
            json["data"]["flags"].as_u64().is_some() || json["data"]["flags"].as_i64().is_some()
        );
    }

    #[test]
    fn test_embed_followup_structure() {
        let wallet = api_client::WalletResponse {
            user_id: "u1".into(),
            coins: 500,
            total_earned: 1000,
            total_spent: 500,
        };
        let emb = build_wallet_response_embed(&Ok(wallet), "User");
        let followup = build_embed_followup(emb);
        let json = serde_json::to_value(&followup).unwrap();
        assert!(json["embeds"].as_array().is_some());
    }

    #[test]
    fn test_format_target_name_edge_cases() {
        let id1 = UserId::new(1);

        // Very long name
        let long_name = "a".repeat(100);
        assert_eq!(format_target_name(id1, id1, &long_name, None), long_name);

        // Unicode name
        assert_eq!(format_target_name(id1, id1, "Café ☕", None), "Café ☕");

        // Large ID
        let large_id = UserId::new(999999999999);
        assert_eq!(
            format_target_name(large_id, id1, "Test", None),
            "<@999999999999>"
        );
    }

    #[test]
    fn test_build_transfer_request_payload_edge_cases() {
        let u1 = UserId::new(1);
        let u2 = UserId::new(2);

        // Zero amount
        let req = build_transfer_request_payload(u1, "A", u2, "B", 0, None);
        assert_eq!(req.amount, 0);

        // Negative amount
        let req = build_transfer_request_payload(u1, "A", u2, "B", -50, None);
        assert_eq!(req.amount, -50);

        // Very large amount
        let req = build_transfer_request_payload(u1, "A", u2, "B", i64::MAX - 1, None);
        assert_eq!(req.amount, i64::MAX - 1);

        // Unicode usernames
        let req =
            build_transfer_request_payload(u1, "Çøsé", u2, "Nördé", 100, Some("Merçi".into()));
        assert_eq!(req.from_username, "Çøsé");
        assert_eq!(req.to_username, "Nördé");
    }

    #[test]
    fn test_build_wallet_response_embed_edge_cases() {
        let ok = Ok(api_client::WalletResponse {
            user_id: "very_long_id_string".into(),
            coins: 0,
            total_earned: 0,
            total_spent: 0,
        });
        let emb = build_wallet_response_embed(&ok, "");
        let json = serde_json::to_value(&emb).unwrap();
        assert!(json["title"].as_str().is_some());

        let err: Result<api_client::WalletResponse, String> = Err("".into());
        let emb = build_wallet_response_embed(&err, "Test");
        let json = serde_json::to_value(&emb).unwrap();
        assert_eq!(json["description"], "");
    }

    #[test]
    fn test_build_transfer_response_embed_no_reason() {
        let ok = Ok(api_client::TransferResponse {
            from_balance: 0,
            amount: 1,
        });
        let emb = build_transfer_response_embed(&ok, 1, 2, None);
        let json = serde_json::to_value(&emb).unwrap();
        assert!(json["title"].as_str().unwrap().contains("Don"));
    }

    #[test]
    fn test_build_leaderboard_response_embed_multiple_entries() {
        let ok: Result<Vec<api_client::WalletResponse>, String> = Ok(vec![
            api_client::WalletResponse {
                user_id: "u1".into(),
                coins: 1000,
                total_earned: 2000,
                total_spent: 1000,
            },
            api_client::WalletResponse {
                user_id: "u2".into(),
                coins: 500,
                total_earned: 1000,
                total_spent: 500,
            },
        ]);
        let emb = build_leaderboard_response_embed(&ok);
        let json = serde_json::to_value(&emb).unwrap();
        assert!(json["title"].as_str().unwrap().contains("Classement"));
    }

    #[test]
    fn test_message_constants_completeness() {
        // All message constants should be non-empty and meaningful
        assert!(!MSG_GUILD_REQUIRED.is_empty());
        assert!(!MSG_GUILD_UNAUTHORIZED.is_empty());
        assert!(!MSG_DONNER_NEED_TARGET.is_empty());
        assert!(!MSG_DONNER_NEED_AMOUNT.is_empty());
        assert!(!MSG_ROUE_NO_DM.is_empty());
    }

    #[test]
    fn test_validate_donner_with_different_ids() {
        let ids = [
            (UserId::new(1), UserId::new(2)),
            (UserId::new(100), UserId::new(200)),
            (UserId::new(u64::MAX - 1), UserId::new(u64::MAX)),
        ];

        for (attacker, defender) in ids {
            if attacker != defender {
                assert!(validate_donner(attacker, defender, false).is_ok());
            }
        }
    }

    #[tokio::test]
    async fn test_execute_economy_actions() {
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

                let body = if req.contains("/api/wallet/") && req.contains("/transfer") {
                    r#"{"amount":50,"from_balance":450}"#
                } else if req.contains("/api/wallet/") && req.contains("/leaderboard") {
                    r#"[{"user_id":"u1","coins":500,"total_earned":1000,"total_spent":500}]"#
                } else if req.contains("/api/wallet/") {
                    r#"{"user_id":"u1","coins":500,"total_earned":1000,"total_spent":500}"#
                } else if req.contains("/api/wheel/") {
                    r#"{"case_label":"Jackpot","payout":1000,"balance_after":1500,"is_memorable":true}"#
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

        let w_emb = execute_solde(&client, "g1", "u1", "Alice").await;
        let j_w = serde_json::to_value(&w_emb).unwrap();
        assert!(j_w["title"].as_str().unwrap().contains("Alice"));

        let t_req = api_client::TransferRequest {
            from_user_id: "u1".into(),
            from_username: "Alice".into(),
            to_user_id: "u2".into(),
            to_username: "Bob".into(),
            amount: 50,
            reason: Some("Cadeau".into()),
        };
        let t_emb = execute_donner(&client, "g1", t_req, 1, 2, Some("Cadeau")).await;
        let j_t = serde_json::to_value(&t_emb).unwrap();
        assert!(j_t["title"].as_str().unwrap().contains("Don"));

        let lb_emb = execute_classement(&client, "g1", 10).await;
        let j_lb = serde_json::to_value(&lb_emb).unwrap();
        assert!(j_lb["title"].as_str().unwrap().contains("Classement"));

        let r_emb = execute_roue(&client, "g1", "u1", "Alice").await;
        let j_r = serde_json::to_value(&r_emb).unwrap();
        assert!(j_r["title"].as_str().unwrap().contains("Alice"));

        // Test error branches
        let err_res: Result<api_client::WalletResponse, String> = Err("wallet error".into());
        let err_w_emb = build_wallet_response_embed(&err_res, "Alice");
        let j_err_w = serde_json::to_value(&err_w_emb).unwrap();
        assert_eq!(j_err_w["description"], "wallet error");

        let err_t_res: Result<api_client::TransferResponse, String> = Err("transfer error".into());
        let err_t_emb = build_transfer_response_embed(&err_t_res, 1, 2, None);
        let j_err_t = serde_json::to_value(&err_t_emb).unwrap();
        assert_eq!(j_err_t["description"], "transfer error");

        let err_lb_res: Result<Vec<api_client::WalletResponse>, String> =
            Err("leaderboard error".into());
        let err_lb_emb = build_leaderboard_response_embed(&err_lb_res);
        let j_err_lb = serde_json::to_value(&err_lb_emb).unwrap();
        assert_eq!(j_err_lb["description"], "leaderboard error");

        let err_r_res: Result<api_client::WheelSpinResponse, String> = Err("spin error".into());
        let err_r_emb = build_spin_response_embed(&err_r_res, "Alice");
        let j_err_r = serde_json::to_value(&err_r_emb).unwrap();
        assert_eq!(j_err_r["description"], "spin error");
    }
}
