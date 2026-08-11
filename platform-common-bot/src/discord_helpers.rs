use serenity::all::{
    CommandInteraction, Context, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseFollowup, CreateInteractionResponseMessage,
};
use tracing::warn;

/// Extrait le `guild_id` d'une slash command. Si la commande est utilisee
/// en DM (pas de guild), repond ephemerement et retourne `None`.
pub async fn require_guild_id(ctx: &Context, command: &CommandInteraction) -> Option<String> {
    match command.guild_id {
        Some(id) => Some(id.to_string()),
        None => {
            reply_ephemeral(ctx, command, "Commande serveur uniquement.").await;
            None
        }
    }
}

/// Defer une slash command en mode ephemere.
pub async fn defer_ephemeral(ctx: &Context, command: &CommandInteraction) {
    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, command = %command.data.name, "Echec defer ephemere");
    }
}

/// Followup ephemere embed apres un `defer_ephemeral`.
pub async fn followup_ephemeral_embed(
    ctx: &Context,
    command: &CommandInteraction,
    embed: CreateEmbed,
) {
    if let Err(e) = command
        .create_followup(
            &ctx.http,
            CreateInteractionResponseFollowup::new()
                .embed(embed)
                .ephemeral(true),
        )
        .await
    {
        warn!(error = %e, command = %command.data.name, "Echec followup ephemere embed");
    }
}

/// Edit la reponse texte apres un defer.
pub async fn edit_response_text(ctx: &Context, command: &CommandInteraction, content: &str) {
    if let Err(e) = command
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new().content(content),
        )
        .await
    {
        warn!(error = %e, command = %command.data.name, "Echec edit response texte");
    }
}

/// Edit la reponse avec un embed.
pub async fn edit_response_embed(ctx: &Context, command: &CommandInteraction, embed: CreateEmbed) {
    if let Err(e) = command
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new().embed(embed),
        )
        .await
    {
        warn!(error = %e, command = %command.data.name, "Echec edit response embed");
    }
}

/// Edit la reponse avec un feedback embed (depend de embeds::feedback_embed).
pub async fn edit_response_feedback(ctx: &Context, command: &CommandInteraction, content: &str) {
    if let Err(e) = command
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new()
                .embed(crate::embeds::feedback_embed(content)),
        )
        .await
    {
        warn!(error = %e, command = %command.data.name, "Echec edit response feedback");
    }
}

/// Reponse ephemere texte a une slash command.
pub async fn reply_ephemeral(ctx: &Context, command: &CommandInteraction, content: &str) {
    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(crate::embeds::feedback_embed(content))
                    .ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, command = %command.data.name, "Echec reponse ephemere texte");
    }
}

/// Reponse ephemere embed a une slash command.
pub async fn reply_ephemeral_embed(
    ctx: &Context,
    command: &CommandInteraction,
    embed: CreateEmbed,
) {
    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, command = %command.data.name, "Echec reponse ephemere embed");
    }
}

pub fn option_str<'a>(
    options: &'a [serenity::all::CommandDataOption],
    name: &str,
) -> Option<&'a str> {
    options
        .iter()
        .find(|o| o.name == name)
        .and_then(|o| match &o.value {
            serenity::all::CommandDataOptionValue::String(s) => Some(s.as_str()),
            _ => None,
        })
}

pub fn option_i64(options: &[serenity::all::CommandDataOption], name: &str) -> Option<i64> {
    options
        .iter()
        .find(|o| o.name == name)
        .and_then(|o| match &o.value {
            serenity::all::CommandDataOptionValue::Integer(n) => Some(*n),
            _ => None,
        })
}

pub fn option_bool(options: &[serenity::all::CommandDataOption], name: &str) -> Option<bool> {
    options
        .iter()
        .find(|o| o.name == name)
        .and_then(|o| match &o.value {
            serenity::all::CommandDataOptionValue::Boolean(b) => Some(*b),
            _ => None,
        })
}

pub fn option_user(
    options: &[serenity::all::CommandDataOption],
    name: &str,
) -> Option<serenity::all::UserId> {
    options
        .iter()
        .find(|o| o.name == name)
        .and_then(|o| match &o.value {
            serenity::all::CommandDataOptionValue::User(id) => Some(*id),
            _ => None,
        })
}
