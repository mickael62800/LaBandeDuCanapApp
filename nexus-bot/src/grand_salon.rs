use crate::api_client::ApiClient;
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};
use std::sync::Arc;

pub fn register() -> CreateCommand {
    CreateCommand::new("salon")
        .description("Le Grand Salon de La Bande du Canapé")
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "rejoindre",
            "Prends place dans le Grand Salon",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "profil",
            "Affiche tes ressources du Salon",
        ))
}

pub async fn handle_command(api: &Arc<ApiClient>, ctx: &Context, cmd: &CommandInteraction) {
    let Some(guild) = cmd.guild_id else { return };
    let action = cmd
        .data
        .options
        .first()
        .map(|o| o.name.as_str())
        .unwrap_or("profil");
    let result = if action == "rejoindre" {
        api.grand_salon_join(
            &guild.to_string(),
            &cmd.user.id.to_string(),
            cmd.user.global_name.as_deref().unwrap_or(&cmd.user.name),
        )
        .await
    } else {
        api.grand_salon_profile(&guild.to_string(), &cmd.user.id.to_string())
            .await
    };
    let content=match result{Ok(p)=>format!("🛋️ **{} — Le Grand Salon**\nRayonnement **{}** · Jetons **{}** · Réputation **{}** · Bons plans **{}** · Réseau **{}**",p.display_name,p.rayonnement,p.jetons,p.reputation,p.bons_plans,p.reseau),Err(e)=>format!("Le Grand Salon est indisponible : {e}")};
    let _ = cmd
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content(content),
            ),
        )
        .await;
}
