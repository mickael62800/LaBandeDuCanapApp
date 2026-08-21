use super::*;
use serenity::all::{CreateEmbed, ChannelId, UserId};

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
        let defender = cmd.data.resolved.users.get(&defender_id);
        let is_bot = defender.is_some_and(|u| u.bot);
        if let Err(err_msg) = validate_coussin_challenge(cmd.user.id, defender_id, is_bot) {
            self.reply_error(ctx, cmd, err_msg).await;
            return;
        }
        if let Err(e) = cmd.defer(&ctx.http).await {
            tracing::error!("defer /coussin impossible: {e}");
            return;
        }
        let req = api_client::CoussinChallengeRequest {
            channel_id: cmd.channel_id.to_string(),
            attacker_id: cmd.user.id.to_string(),
            attacker_name: cmd.user.display_name().to_string(),
            defender_id: defender_id.to_string(),
            defender_name: defender
                .map(|u| u.display_name().to_string())
                .unwrap_or_else(|| format!("<@{defender_id}>")),
            mise,
        };
        let response = execute_coussin_challenge(&self.api, &guild_id, &req).await;
        match response {
            Ok(combat) => {
                let message = build_coussin_challenge_followup(
                    cmd.user.id,
                    defender_id,
                    &combat.id,
                    combat.mise,
                );
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
        let embed = execute_coussin_profile(&self.api, &guild_id, &user_id.to_string(), &username).await;
        let _ = cmd
            .create_followup(&ctx.http, build_ephemeral_embed_followup(embed))
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
        let embed = execute_coussin_class(
            &self.api,
            &guild_id,
            &cmd.user.id.to_string(),
            cmd.user.display_name(),
            &class,
        )
        .await;
        let _ = cmd
            .create_followup(&ctx.http, build_ephemeral_embed_followup(embed))
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
        let embed = execute_coussin_train(
            &self.api,
            &guild_id,
            &cmd.user.id.to_string(),
            cmd.user.display_name(),
            &stat,
        )
        .await;
        let _ = cmd
            .create_followup(&ctx.http, build_ephemeral_embed_followup(embed))
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
        let embed = execute_coussin_shop(&self.api, &guild_id, &cmd.user.id.to_string(), &item).await;
        let _ = cmd
            .create_followup(&ctx.http, build_ephemeral_embed_followup(embed))
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
        let embed = execute_coussin_insurance(&self.api, &guild_id, &cmd.user.id.to_string()).await;
        let _ = cmd
            .create_followup(&ctx.http, build_ephemeral_embed_followup(embed))
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
        let message = build_coussin_steal_followup(
            cmd.user.id,
            target,
            &opened.attempt_id,
            opened.defense_window_seconds,
        );

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
        let Some((attempt_id, victim_id)) = parse_coussin_steal_button(&component.data.custom_id) else {
            return;
        };

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
                let content = format_steal_defense_result(&outcome);
                let _ = component
                    .edit_response(
                        &ctx.http,
                        serenity::all::EditInteractionResponse::new()
                            .content(content)
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
        let req = api_client::CoussinPrimeRequest {
            target_id: target.to_string(),
            target_name: name,
            placer_name: cmd.user.display_name().into(),
            amount,
        };
        let text = execute_coussin_prime(&self.api, &guild, &cmd.user.id.to_string(), target, amount, &req).await;
        let _ = cmd
            .create_followup(&ctx.http, build_content_followup(text))
            .await;
    }

    pub(super) async fn handle_inventory(&self, ctx: &Context, cmd: &CommandInteraction) {
        let Some(guild) = self.require_guild(ctx, cmd).await else {
            return;
        };
        if cmd.defer_ephemeral(&ctx.http).await.is_err() {
            return;
        }
        let embed = execute_coussin_inventory(&self.api, &guild, &cmd.user.id.to_string()).await;
        let _ = cmd
            .create_followup(&ctx.http, build_ephemeral_embed_followup(embed))
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
        let req = api_client::CoussinBetRequest {
            combat_id: id,
            bettor_name: cmd.user.display_name().into(),
            backed_id: target.to_string(),
            amount,
        };
        let text = execute_coussin_bet(&self.api, &guild, &cmd.user.id.to_string(), target, amount, &req).await;
        let _ = cmd
            .create_followup(&ctx.http, build_content_followup(text))
            .await;
    }

    pub(super) async fn handle_coussin_component(
        &self,
        ctx: &Context,
        component: &serenity::all::ComponentInteraction,
    ) {
        let Some((action, combat_id, defender_id, attacker_id)) =
            parse_coussin_challenge_button(&component.data.custom_id)
        else {
            return;
        };

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
                                    .content(format_coussin_resolve_result(
                                        attacker_id,
                                        component.user.id.get(),
                                    ))
                                    .components(vec![]),
                            )
                            .await;
                    }
                    Ok(false) => {
                        self.component_error(ctx, component, MSG_DUEL_RESOLVE_FAILED)
                            .await
                    }
                    Err(e) => self.component_error(ctx, component, &e).await,
                }
            }
            Ok(false) => {
                self.component_error(ctx, component, MSG_CHALLENGE_UNAVAILABLE)
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

pub const MSG_DUEL_RESOLVE_FAILED: &str = "Le duel n'a pas pu etre resolu.";
pub const MSG_CHALLENGE_UNAVAILABLE: &str = "Ce defi n'est plus disponible.";

pub fn parse_coussin_challenge_button(custom_id: &str) -> Option<(&str, &str, &str, &str)> {
    let parts: Vec<_> = custom_id.split(':').collect();
    if parts.len() == 5 && parts[0] == "c" {
        Some((parts[1], parts[2], parts[3], parts[4]))
    } else {
        None
    }
}

pub fn parse_coussin_steal_button(custom_id: &str) -> Option<(&str, &str)> {
    let parts: Vec<_> = custom_id.split(':').collect();
    if parts.len() == 4 && parts[0] == "cs" && parts[1] == "d" {
        Some((parts[2], parts[3]))
    } else {
        None
    }
}

pub fn validate_coussin_challenge(
    attacker_id: UserId,
    defender_id: UserId,
    is_defender_bot: bool,
) -> Result<(), &'static str> {
    if defender_id == attacker_id {
        return Err("Te piéger toi-même, c'est juste t'asseoir.");
    }
    if is_defender_bot {
        return Err("Un bot ne s'assoit jamais.");
    }
    Ok(())
}

pub fn build_challenge_request_payload(
    channel_id: ChannelId,
    attacker_id: UserId,
    attacker_name: &str,
    defender_id: UserId,
    defender_name: &str,
    mise: i64,
) -> api_client::CoussinChallengeRequest {
    api_client::CoussinChallengeRequest {
        channel_id: channel_id.to_string(),
        attacker_id: attacker_id.to_string(),
        attacker_name: attacker_name.to_string(),
        defender_id: defender_id.to_string(),
        defender_name: defender_name.to_string(),
        mise,
    }
}

pub fn format_steal_prompt(attacker_id: UserId, target_id: UserId, window_seconds: i64) -> String {
    format!(
        "🛋️ <@{}> fouille les coussins de <@{}> !\n<@{}>, tu as **{} secondes** pour réagir — sans quoi tu te feras chiper bien plus facilement.",
        attacker_id, target_id, target_id, window_seconds
    )
}

pub fn format_steal_defense_result(outcome: &api_client::CoussinStealOutcome) -> String {
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
        "\n🎲 Voleur : **{}** — Défense : **{}**",
        outcome.thief_total, outcome.victim_total
    );
    format!("{recit}{detail}")
}

pub fn format_prime_announcement(target_id: UserId, amount: i64) -> String {
    format!("📜 Contrat de {amount} coins sur <@{target_id}> : fais-le lever.")
}

pub fn format_bet_announcement(target_id: UserId, amount: i64) -> String {
    format!("🍿 {amount} coins sur <@{target_id}>. Popcorn servi.")
}

pub async fn execute_coussin_challenge(
    api: &ApiClient,
    guild_id: &str,
    req: &api_client::CoussinChallengeRequest,
) -> Result<api_client::CoussinChallengeResponse, String> {
    api.challenge_coussin(guild_id, req).await
}

pub async fn execute_coussin_profile(
    api: &ApiClient,
    guild_id: &str,
    user_id: &str,
    username: &str,
) -> CreateEmbed {
    let res = api.coussin_profile(guild_id, user_id, username).await;
    build_coussin_profile_response(&res)
}

pub async fn execute_coussin_class(
    api: &ApiClient,
    guild_id: &str,
    user_id: &str,
    username: &str,
    class: &str,
) -> CreateEmbed {
    let res = api.choose_coussin_class(guild_id, user_id, username, class).await;
    build_coussin_profile_response(&res)
}

pub async fn execute_coussin_train(
    api: &ApiClient,
    guild_id: &str,
    user_id: &str,
    username: &str,
    stat: &str,
) -> CreateEmbed {
    let res = api.train_coussin(guild_id, user_id, username, stat).await;
    build_coussin_profile_response(&res)
}

pub async fn execute_coussin_shop(
    api: &ApiClient,
    guild_id: &str,
    user_id: &str,
    item: &str,
) -> CreateEmbed {
    let res = api.buy_coussin_item(guild_id, user_id, item).await;
    build_coussin_shop_response(item, &res)
}

pub async fn execute_coussin_insurance(
    api: &ApiClient,
    guild_id: &str,
    user_id: &str,
) -> CreateEmbed {
    let res = api.buy_coussin_insurance(guild_id, user_id).await;
    build_coussin_insurance_response(&res)
}

pub async fn execute_coussin_prime(
    api: &ApiClient,
    guild_id: &str,
    user_id: &str,
    target_id: UserId,
    amount: i64,
    req: &api_client::CoussinPrimeRequest,
) -> String {
    let res = api.prime_coussin(guild_id, user_id, req).await;
    build_coussin_prime_response(target_id, amount, &res)
}

pub async fn execute_coussin_inventory(
    api: &ApiClient,
    guild_id: &str,
    user_id: &str,
) -> CreateEmbed {
    let res = api.inventory_coussin(guild_id, user_id).await;
    build_coussin_inventory_response(&res)
}

pub async fn execute_coussin_bet(
    api: &ApiClient,
    guild_id: &str,
    user_id: &str,
    target_id: UserId,
    amount: i64,
    req: &api_client::CoussinBetRequest,
) -> String {
    let res = api.bet_coussin(guild_id, user_id, req).await;
    build_coussin_bet_response(target_id, amount, &res)
}

pub fn build_coussin_challenge_buttons(combat_id: &str, defender_id: UserId, attacker_id: UserId) -> CreateActionRow {
    CreateActionRow::Buttons(vec![
        CreateButton::new(format!("c:a:{combat_id}:{defender_id}:{attacker_id}"))
            .label("Accepter")
            .style(ButtonStyle::Success),
        CreateButton::new(format!("c:r:{combat_id}:{defender_id}:{attacker_id}"))
            .label("Refuser")
            .style(ButtonStyle::Danger),
    ])
}

pub fn build_coussin_challenge_followup(
    attacker_id: UserId,
    defender_id: UserId,
    combat_id: &str,
    mise: i64,
) -> serenity::all::CreateInteractionResponseFollowup {
    let buttons = build_coussin_challenge_buttons(combat_id, defender_id, attacker_id);
    let embed = embeds::build_coussin_challenge_embed(
        attacker_id.get(),
        defender_id.get(),
        mise,
    );
    serenity::all::CreateInteractionResponseFollowup::new()
        .embed(embed)
        .components(vec![buttons])
}

pub fn build_coussin_steal_buttons(attempt_id: &str, target_id: UserId) -> CreateActionRow {
    CreateActionRow::Buttons(vec![
        CreateButton::new(format!("cs:d:{attempt_id}:{target_id}"))
            .label("Serrer les coussins")
            .style(ButtonStyle::Primary),
    ])
}

pub fn build_coussin_steal_followup(
    attacker_id: UserId,
    target_id: UserId,
    attempt_id: &str,
    window_seconds: i64,
) -> serenity::all::CreateInteractionResponseFollowup {
    let bouton = build_coussin_steal_buttons(attempt_id, target_id);
    let prompt = format_steal_prompt(attacker_id, target_id, window_seconds);
    serenity::all::CreateInteractionResponseFollowup::new()
        .content(prompt)
        .components(vec![bouton])
}

pub fn build_coussin_profile_response(res: &Result<api_client::CoussinProfileResponse, String>) -> CreateEmbed {
    match res {
        Ok(profile) => embeds::build_coussin_profile_embed(profile),
        Err(message) => embeds::build_error_embed(message),
    }
}

pub fn build_coussin_shop_response(item: &str, res: &Result<i64, String>) -> CreateEmbed {
    match res {
        Ok(balance) => embeds::build_coussin_purchase_embed(item, *balance),
        Err(e) => embeds::build_error_embed(e),
    }
}

pub fn build_coussin_insurance_response(res: &Result<(bool, String), String>) -> CreateEmbed {
    match res {
        Ok((scam, expires)) => embeds::build_coussin_insurance_embed(*scam, expires),
        Err(e) => embeds::build_error_embed(e),
    }
}

pub fn build_coussin_inventory_response(res: &Result<Vec<api_client::CoussinInventoryItem>, String>) -> CreateEmbed {
    match res {
        Ok(items) => embeds::build_coussin_inventory_embed(items),
        Err(e) => embeds::build_error_embed(e),
    }
}

pub fn build_coussin_prime_response(target_id: UserId, amount: i64, res: &Result<(), String>) -> String {
    match res {
        Ok(()) => format_prime_announcement(target_id, amount),
        Err(e) => e.clone(),
    }
}

pub fn build_coussin_bet_response(target_id: UserId, amount: i64, res: &Result<(), String>) -> String {
    match res {
        Ok(()) => format_bet_announcement(target_id, amount),
        Err(e) => e.clone(),
    }
}

pub fn build_ephemeral_embed_followup(embed: CreateEmbed) -> serenity::all::CreateInteractionResponseFollowup {
    serenity::all::CreateInteractionResponseFollowup::new()
        .embed(embed)
        .ephemeral(true)
}

pub fn build_content_followup(content: String) -> serenity::all::CreateInteractionResponseFollowup {
    serenity::all::CreateInteractionResponseFollowup::new().content(content)
}

pub fn format_coussin_resolve_result(attacker_id: u64, defender_id: u64) -> String {
    format!(
        "👊 Duel resolu entre <@{attacker_id}> et <@{defender_id}>. Consultez /profil pour voir les consequences."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_coussin_challenge_button() {
        assert_eq!(
            parse_coussin_challenge_button("c:a:combat_1:def_1:att_1"),
            Some(("a", "combat_1", "def_1", "att_1"))
        );
        assert_eq!(
            parse_coussin_challenge_button("c:r:combat_2:def_2:att_2"),
            Some(("r", "combat_2", "def_2", "att_2"))
        );
        assert_eq!(parse_coussin_challenge_button("invalid"), None);
        assert_eq!(parse_coussin_challenge_button("c:a:1"), None);
        assert_eq!(parse_coussin_challenge_button(""), None);
        assert_eq!(parse_coussin_challenge_button("c:::"), None);
        assert_eq!(parse_coussin_challenge_button("x:a:1:2:3"), None);
    }

    #[test]
    fn test_parse_coussin_steal_button() {
        assert_eq!(
            parse_coussin_steal_button("cs:d:att_1:vic_1"),
            Some(("att_1", "vic_1"))
        );
        assert_eq!(parse_coussin_steal_button("other"), None);
        assert_eq!(parse_coussin_steal_button("cs:x:1:2"), None);
    }

    #[test]
    fn test_validate_coussin_challenge() {
        let user1 = UserId::new(10);
        let user2 = UserId::new(20);

        assert!(validate_coussin_challenge(user1, user2, false).is_ok());
        assert_eq!(
            validate_coussin_challenge(user1, user1, false),
            Err("Te piéger toi-même, c'est juste t'asseoir.")
        );
        assert_eq!(
            validate_coussin_challenge(user1, user2, true),
            Err("Un bot ne s'assoit jamais.")
        );
    }

    #[test]
    fn test_formatting_helpers() {
        let prompt = format_steal_prompt(UserId::new(1), UserId::new(2), 30);
        assert!(prompt.contains("30 secondes"));
        assert!(prompt.contains("<@1>"));

        let out_success = api_client::CoussinStealOutcome {
            thief_id: "t1".into(),
            success: true,
            amount: 50,
            thief_total: 10,
            victim_total: 5,
        };
        let s_text = format_steal_defense_result(&out_success);
        assert!(s_text.contains("repart avec **50** coins"));

        let out_fail = api_client::CoussinStealOutcome {
            thief_id: "t1".into(),
            success: false,
            amount: 25,
            thief_total: 5,
            victim_total: 12,
        };
        let f_text = format_steal_defense_result(&out_fail);
        assert!(f_text.contains("repart bredouille et perd **25** coins"));

        let prime_text = format_prime_announcement(UserId::new(5), 100);
        assert!(prime_text.contains("100 coins"));

        let bet_text = format_bet_announcement(UserId::new(6), 20);
        assert!(bet_text.contains("Popcorn servi"));

        let res_text = format_coussin_resolve_result(10, 20);
        assert!(res_text.contains("Duel resolu"));
    }

    #[test]
    fn test_coussin_response_helpers() {
        let btn_challenge = build_coussin_challenge_buttons("combat1", UserId::new(10), UserId::new(20));
        let j_btn = serde_json::to_value(&btn_challenge).unwrap();
        assert_eq!(j_btn["components"].as_array().unwrap().len(), 2);

        let btn_steal = build_coussin_steal_buttons("att1", UserId::new(10));
        let j_steal = serde_json::to_value(&btn_steal).unwrap();
        assert_eq!(j_steal["components"].as_array().unwrap().len(), 1);

        let profile = api_client::CoussinProfileResponse {
            username: "Alice".into(),
            class: "Guerrier".into(),
            level: 2,
            xp: 50,
            atk: 10,
            def: 5,
            hp_current: 100,
            hp_max: 100,
            coins: 500,
            stat_points: 0,
            title: "Recrue".into(),
            total_wins: 3,
            total_losses: 1,
            total_draws: 0,
            total_stolen: 0,
            cowardice_count: 0,
            chaos_events: 0,
        };
        let emb_p = build_coussin_profile_response(&Ok(profile));
        let j_p = serde_json::to_value(&emb_p).unwrap();
        assert!(j_p["title"].as_str().unwrap().contains("Alice"));

        let emb_p_err = build_coussin_profile_response(&Err("Profile err".into()));
        let j_p_err = serde_json::to_value(&emb_p_err).unwrap();
        assert_eq!(j_p_err["description"], "Profile err");

        let emb_shop_ok = build_coussin_shop_response("shield", &Ok(450));
        let j_shop = serde_json::to_value(&emb_shop_ok).unwrap();
        assert!(j_shop["title"].as_str().unwrap().contains("Planque"));

        let emb_shop_err = build_coussin_shop_response("shield", &Err("Shop err".into()));
        let j_shop_err = serde_json::to_value(&emb_shop_err).unwrap();
        assert_eq!(j_shop_err["description"], "Shop err");

        let emb_ins_ok = build_coussin_insurance_response(&Ok((false, "demain".into())));
        let j_ins = serde_json::to_value(&emb_ins_ok).unwrap();
        assert!(j_ins["title"].as_str().unwrap().contains("Garantie"));

        let emb_ins_err = build_coussin_insurance_response(&Err("Ins err".into()));
        let j_ins_err = serde_json::to_value(&emb_ins_err).unwrap();
        assert_eq!(j_ins_err["description"], "Ins err");

        let items = vec![
            api_client::CoussinInventoryItem {
                item_key: "shield".into(),
                quantity: 1,
            }
        ];
        let emb_inv_ok = build_coussin_inventory_response(&Ok(items));
        let j_inv = serde_json::to_value(&emb_inv_ok).unwrap();
        assert!(j_inv["title"].as_str().unwrap().contains("Sous ton coussin"));

        let emb_inv_err = build_coussin_inventory_response(&Err("Inv err".into()));
        let j_inv_err = serde_json::to_value(&emb_inv_err).unwrap();
        assert_eq!(j_inv_err["description"], "Inv err");

        let prime_ok = build_coussin_prime_response(UserId::new(5), 100, &Ok(()));
        assert!(prime_ok.contains("100 coins"));
        let prime_err = build_coussin_prime_response(UserId::new(5), 100, &Err("Prime err".into()));
        assert_eq!(prime_err, "Prime err");

        let bet_ok = build_coussin_bet_response(UserId::new(6), 50, &Ok(()));
        assert!(bet_ok.contains("Popcorn"));
        let bet_err = build_coussin_bet_response(UserId::new(6), 50, &Err("Bet err".into()));
        assert_eq!(bet_err, "Bet err");

        let challenge_fup = build_coussin_challenge_followup(UserId::new(1), UserId::new(2), "combat_123", 100);
        let j_cfup = serde_json::to_value(&challenge_fup).unwrap();
        assert!(j_cfup["embeds"].as_array().is_some());
        assert!(j_cfup["components"].as_array().is_some());

        let steal_fup = build_coussin_steal_followup(UserId::new(1), UserId::new(2), "attempt_456", 30);
        let j_sfup = serde_json::to_value(&steal_fup).unwrap();
        assert!(j_sfup["content"].as_str().unwrap().contains("30 secondes"));
        assert!(j_sfup["components"].as_array().is_some());

        let eph_fup = build_ephemeral_embed_followup(emb_inv_ok);
        let j_efup = serde_json::to_value(&eph_fup).unwrap();
        assert!(j_efup["embeds"].as_array().is_some());
        assert_eq!(j_efup["flags"], 64);

        let cont_fup = build_content_followup("Test content".into());
        let j_cont = serde_json::to_value(&cont_fup).unwrap();
        assert_eq!(j_cont["content"], "Test content");

        let resolve_txt = format_coussin_resolve_result(10, 20);
        assert!(resolve_txt.contains("<@10>"));
        assert!(resolve_txt.contains("<@20>"));

        assert!(MSG_DUEL_RESOLVE_FAILED.contains("Le duel"));
        assert!(MSG_CHALLENGE_UNAVAILABLE.contains("plus disponible"));

        let def_succ = api_client::CoussinStealOutcome {
            success: true,
            thief_id: "t1".into(),
            amount: 50,
            thief_total: 20,
            victim_total: 10,
        };
        let def_succ_txt = format_steal_defense_result(&def_succ);
        assert!(def_succ_txt.contains("repart avec **50** coins"));

        let def_fail = api_client::CoussinStealOutcome {
            success: false,
            thief_id: "t1".into(),
            amount: 50,
            thief_total: 10,
            victim_total: 20,
        };
        let def_fail_txt = format_steal_defense_result(&def_fail);
        assert!(def_fail_txt.contains("repart bredouille"));
    }

    #[tokio::test]
    async fn test_execute_coussin_actions() {
        use tokio::net::TcpListener;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{}", addr);

        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]);

                let profile_json = r#"{"username":"Alice","class":"ecraseur","level":1,"xp":10,"atk":5,"def":5,"hp_current":100,"hp_max":100,"coins":50,"stat_points":2,"title":"Squatteur","total_wins":1,"total_losses":0,"total_draws":0,"total_stolen":0,"cowardice_count":0,"chaos_events":0}"#;

                let body = if req.contains("/api/coussin/") && req.contains("/combats") {
                    r#"{"id":"comb1","mise":50,"attacker_id":"u1","defender_id":"u2","status":"pending"}"#
                } else if req.contains("/api/coussin/") && req.contains("/class") {
                    profile_json
                } else if req.contains("/api/coussin/") && req.contains("/train") {
                    profile_json
                } else if req.contains("/api/coussin/") && req.contains("/shop") {
                    r#"{"balance_after":90}"#
                } else if req.contains("/api/coussin/") && req.contains("/insurance") {
                    r#"{"is_scam":false,"expires_at":"demain"}"#
                } else if req.contains("/api/coussin/") && req.contains("/prime") {
                    r#"{"ok":true}"#
                } else if req.contains("/api/coussin/") && req.contains("/inventory") {
                    r#"[{"item_key":"plume","quantity":2}]"#
                } else if req.contains("/api/coussin/") && req.contains("/bet") {
                    r#"{"ok":true}"#
                } else if req.contains("/api/coussin/") && req.contains("/profile") {
                    profile_json
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

        let chall_req = api_client::CoussinChallengeRequest {
            channel_id: "c1".into(),
            attacker_id: "u1".into(),
            attacker_name: "Alice".into(),
            defender_id: "u2".into(),
            defender_name: "Bob".into(),
            mise: 50,
        };
        let chall_res = execute_coussin_challenge(&client, "g1", &chall_req).await;
        assert!(chall_res.is_ok());

        let prof = execute_coussin_profile(&client, "g1", "u1", "Alice").await;
        let j_prof = serde_json::to_value(&prof).unwrap();
        assert!(j_prof["title"].as_str().unwrap().contains("Alice"));

        let cls = execute_coussin_class(&client, "g1", "u1", "Alice", "ecraseur").await;
        let j_cls = serde_json::to_value(&cls).unwrap();
        assert!(j_cls["title"].as_str().unwrap().contains("Alice"));

        let trn = execute_coussin_train(&client, "g1", "u1", "Alice", "impact").await;
        let j_trn = serde_json::to_value(&trn).unwrap();
        assert!(j_trn["title"].as_str().unwrap().contains("Alice"));

        let shp = execute_coussin_shop(&client, "g1", "u1", "chapeau").await;
        let j_shp = serde_json::to_value(&shp).unwrap();
        assert!(j_shp["title"].as_str().unwrap().contains("Planque"));

        let ins = execute_coussin_insurance(&client, "g1", "u1").await;
        let j_ins = serde_json::to_value(&ins).unwrap();
        assert!(j_ins["title"].as_str().unwrap().contains("Garantie"));

        let p_req = api_client::CoussinPrimeRequest {
            target_id: "u2".into(),
            target_name: "Bob".into(),
            placer_name: "Alice".into(),
            amount: 100,
        };
        let prime_txt = execute_coussin_prime(&client, "g1", "u1", UserId::new(2), 100, &p_req).await;
        assert!(prime_txt.contains("100 coins"));

        let inv = execute_coussin_inventory(&client, "g1", "u1").await;
        let j_inv = serde_json::to_value(&inv).unwrap();
        assert!(j_inv["title"].as_str().unwrap().contains("Sous ton coussin"));

        let b_req = api_client::CoussinBetRequest {
            combat_id: "comb1".into(),
            bettor_name: "Alice".into(),
            backed_id: "u2".into(),
            amount: 50,
        };
        let bet_txt = execute_coussin_bet(&client, "g1", "u1", UserId::new(2), 50, &b_req).await;
        assert!(bet_txt.contains("Popcorn"));

        // Error branches
        let err_prof: Result<api_client::CoussinProfileResponse, String> = Err("prof err".into());
        let j_err_p = serde_json::to_value(&build_coussin_profile_response(&err_prof)).unwrap();
        assert_eq!(j_err_p["description"], "prof err");

        let err_shop: Result<i64, String> = Err("shop err".into());
        let j_err_s = serde_json::to_value(&build_coussin_shop_response("chapeau", &err_shop)).unwrap();
        assert_eq!(j_err_s["description"], "shop err");

        let err_ins: Result<(bool, String), String> = Err("ins err".into());
        let j_err_i = serde_json::to_value(&build_coussin_insurance_response(&err_ins)).unwrap();
        assert_eq!(j_err_i["description"], "ins err");

        let err_inv: Result<Vec<api_client::CoussinInventoryItem>, String> = Err("inv err".into());
        let j_err_inv = serde_json::to_value(&build_coussin_inventory_response(&err_inv)).unwrap();
        assert_eq!(j_err_inv["description"], "inv err");

        let err_prime: Result<(), String> = Err("prime err".into());
        assert_eq!(build_coussin_prime_response(UserId::new(2), 100, &err_prime), "prime err");

        let err_bet: Result<(), String> = Err("bet err".into());
        assert_eq!(build_coussin_bet_response(UserId::new(2), 50, &err_bet), "bet err");
    }
}
