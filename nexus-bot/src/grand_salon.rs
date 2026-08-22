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

pub fn format_profile(p: &crate::api_client::GrandSalonProfileResponse) -> String {
    format!(
        "🛋️ **{} — Le Grand Salon**\nRayonnement **{}** · Jetons **{}** · Réputation **{}** · Bons plans **{}** · Réseau **{}**",
        p.display_name, p.rayonnement, p.jetons, p.reputation, p.bons_plans, p.reseau
    )
}

pub async fn handle_command(api: &Arc<ApiClient>, ctx: &Context, cmd: &CommandInteraction) {
    let Some(guild) = cmd.guild_id else { return };
    let action = extract_salon_action(cmd.data.options.first().map(|o| o.name.as_str()));
    let display_name = cmd.user.global_name.as_deref().unwrap_or(&cmd.user.name);
    let content = execute_salon_action(
        api,
        &guild.to_string(),
        &cmd.user.id.to_string(),
        display_name,
        action,
    )
    .await;
    let _ = cmd
        .create_response(&ctx.http, build_salon_message(&content))
        .await;
}

pub async fn execute_salon_action(
    api: &ApiClient,
    guild_id: &str,
    user_id: &str,
    display_name: &str,
    action: &str,
) -> String {
    let result = if action == "rejoindre" {
        api.grand_salon_join(guild_id, user_id, display_name).await
    } else {
        api.grand_salon_profile(guild_id, user_id).await
    };
    build_salon_response(result)
}

pub fn build_salon_message(content: &str) -> CreateInteractionResponse {
    CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content(content))
}

pub fn extract_salon_action(first_option_name: Option<&str>) -> &str {
    first_option_name.unwrap_or("profil")
}

pub fn build_salon_response(
    result: Result<crate::api_client::GrandSalonProfileResponse, String>,
) -> String {
    match result {
        Ok(p) => format_profile(&p),
        Err(e) => format_salon_error(&e),
    }
}

pub fn format_salon_error(e: &str) -> String {
    format!("Le Grand Salon est indisponible : {e}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register() {
        let cmd = register();
        let json = serde_json::to_value(&cmd).unwrap();
        assert_eq!(json["name"], "salon");
    }

    #[test]
    fn test_format_profile() {
        let p = crate::api_client::GrandSalonProfileResponse {
            display_name: "Alice".into(),
            rayonnement: 10,
            jetons: 5,
            reputation: 20,
            bons_plans: 2,
            reseau: 3,
        };
        let formatted = format_profile(&p);
        assert!(formatted.contains("Alice"));
        assert!(formatted.contains("Rayonnement **10**"));

        let err = format_salon_error("timeout");
        assert!(err.contains("timeout"));

        let ok_resp = build_salon_response(Ok(p));
        assert!(ok_resp.contains("Alice"));

        let err_resp = build_salon_response(Err("down".into()));
        assert!(err_resp.contains("down"));

        assert_eq!(extract_salon_action(Some("rejoindre")), "rejoindre");
        assert_eq!(extract_salon_action(None), "profil");

        let msg = build_salon_message("Bonjour Salon");
        let j_msg = serde_json::to_value(&msg).unwrap();
        assert!(j_msg["data"]["content"]
            .as_str()
            .unwrap()
            .contains("Bonjour Salon"));
    }

    #[tokio::test]
    async fn test_execute_salon_action() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{}", addr);

        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf).await;
                let body = r#"{"display_name":"Alice","rayonnement":1,"jetons":2,"reputation":3,"bons_plans":4,"reseau":5}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes()).await;
            }
        });

        let client = ApiClient::new(base_url, Some("token".into()));

        let res_join = execute_salon_action(&client, "g1", "u1", "Alice", "rejoindre").await;
        assert!(res_join.contains("Alice"));

        let res_prof = execute_salon_action(&client, "g1", "u1", "Alice", "profil").await;
        assert!(res_prof.contains("Alice"));
    }

    #[test]
    fn test_format_profile_all_fields() {
        let p = crate::api_client::GrandSalonProfileResponse {
            display_name: "Bob".into(),
            rayonnement: 100,
            jetons: 50,
            reputation: 200,
            bons_plans: 10,
            reseau: 5,
        };
        let formatted = format_profile(&p);
        assert!(formatted.contains("Bob"));
        assert!(formatted.contains("100"));
        assert!(formatted.contains("50"));
        assert!(formatted.contains("200"));
        assert!(formatted.contains("10"));
        assert!(formatted.contains("5"));
    }

    #[test]
    fn test_format_profile_zero_values() {
        let p = crate::api_client::GrandSalonProfileResponse {
            display_name: "Charlie".into(),
            rayonnement: 0,
            jetons: 0,
            reputation: 0,
            bons_plans: 0,
            reseau: 0,
        };
        let formatted = format_profile(&p);
        assert!(formatted.contains("Charlie"));
        assert!(formatted.contains("0"));
    }

    #[test]
    fn test_extract_salon_action_all_actions() {
        assert_eq!(extract_salon_action(Some("rejoindre")), "rejoindre");
        assert_eq!(extract_salon_action(Some("profil")), "profil");
        assert_eq!(extract_salon_action(None), "profil");
        assert_eq!(extract_salon_action(Some("")), "");
    }

    #[test]
    fn test_build_salon_response_success() {
        let p = crate::api_client::GrandSalonProfileResponse {
            display_name: "Test".into(),
            rayonnement: 1,
            jetons: 2,
            reputation: 3,
            bons_plans: 4,
            reseau: 5,
        };
        let res = build_salon_response(Ok(p));
        assert!(res.contains("Test"));
        assert!(res.contains("Rayonnement"));
    }

    #[test]
    fn test_build_salon_response_error() {
        let res = build_salon_response(Err("API Error".into()));
        assert!(res.contains("API Error"));
        assert!(res.contains("indisponible"));
    }

    #[test]
    fn test_format_salon_error_various() {
        let err1 = format_salon_error("timeout");
        assert!(err1.contains("timeout"));

        let err2 = format_salon_error("user not found");
        assert!(err2.contains("user not found"));

        let err3 = format_salon_error("");
        assert!(err3.contains("indisponible"));
    }

    #[test]
    fn test_build_salon_message() {
        let msg = build_salon_message("Test message");
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["data"]["content"], "Test message");
    }

    #[test]
    fn test_build_salon_message_empty() {
        let msg = build_salon_message("");
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["data"]["content"], "");
    }

    #[test]
    fn test_register_structure() {
        let cmd = register();
        let json = serde_json::to_value(&cmd).unwrap();
        assert_eq!(json["name"], "salon");
        assert!(json["options"].as_array().unwrap().len() >= 2);
    }
}
