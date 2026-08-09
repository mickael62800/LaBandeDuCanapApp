use std::time::Duration;
use tracing::{info, error};
use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;
use reqwest::Client;
use serde_json::json;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt().init();

    info!("Démarrage du worker Atrium...");

    let database_url = platform_common_worker::load_database_url();
    let deepseek_api_key = std::env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY manquant");
    let pool = platform_common_worker::create_pg_pool(&database_url).await;
    
    let client = Client::new();

    // Boucle principale du worker
    loop {
        info!("Atrium Worker: vérification des résumés météo en attente...");
        
        // Logique simplifiée pour la démo: On génère le résumé toutes les 24h
        // (En prod, on utiliserait cron ou tokio-cron-scheduler)
        
        let guild_id = std::env::var("ATRIUM_PRIMARY_GUILD_ID").unwrap_or_else(|_| "123456789".to_string());
        
        match generate_summary(&pool, &client, &deepseek_api_key, &guild_id).await {
            Ok(summary) => {
                info!("Résumé généré avec succès: {} caractères", summary.len());
                let id = Uuid::new_v4();
                let now = Utc::now();
                let start_date = now - chrono::Duration::days(7);
                
                let res = sqlx::query(
                    "INSERT INTO atrium_server_summaries (id, guild_id, start_date, end_date, content, created_at) \
                     VALUES ($1, $2, $3, $4, $5, $6)"
                )
                .bind(id)
                .bind(&guild_id)
                .bind(start_date)
                .bind(now)
                .bind(&summary)
                .bind(now)
                .execute(&pool)
                .await;
                
                if let Err(e) = res {
                    error!("Erreur lors de l'insertion du résumé en DB: {}", e);
                }
            },
            Err(e) => {
                error!("Erreur lors de la génération du résumé: {}", e);
            }
        }
        
        // Attendre 24 heures (ou une durée de test)
        tokio::time::sleep(Duration::from_secs(86400)).await;
    }
}

async fn generate_summary(pool: &PgPool, client: &Client, api_key: &str, guild_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    // 1. Récupérer un échantillon d'activité récente (ex: les 100 derniers messages parlant à l'IA)
    let rows = sqlx::query(
        "SELECT role, content FROM atrium_conversation_messages WHERE guild_id = $1 ORDER BY id DESC LIMIT 50"
    )
    .bind(guild_id)
    .fetch_all(pool)
    .await?;

    let mut activity_log = String::new();
    for row in rows.iter().rev() {
        use sqlx::Row;
        let role: String = row.try_get("role").unwrap_or_default();
        let content: String = row.try_get("content").unwrap_or_default();
        activity_log.push_str(&format!("{}: {}\n", role, content));
    }
    
    if activity_log.is_empty() {
        activity_log.push_str("Aucune activité récente.");
    }

    // 2. Interroger DeepSeek
    let prompt = format!(
        "Voici un échantillon de l'activité récente de notre serveur Discord.\n\
        Fais un bref résumé 'Météo' (3-4 phrases) très fun, positif et décontracté de l'ambiance globale.\n\
        Ne mentionne pas que tu as lu un 'échantillon', fais comme si tu avais tout suivi.\n\
        Activité:\n{}",
        activity_log
    );

    let payload = json!({
        "model": "deepseek-chat",
        "messages": [
            {
                "role": "system",
                "content": "Tu es Atrium, un assistant IA cool."
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
        "temperature": 0.7,
        "max_tokens": 300
    });

    let res = client.post("https://api.deepseek.com/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&payload)
        .send()
        .await?;

    if !res.status().is_success() {
        let err = res.text().await?;
        return Err(format!("Erreur API DeepSeek: {}", err).into());
    }

    let json: serde_json::Value = res.json().await?;
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("Météo ensoleillée sur le serveur ! (Erreur de parsing AI)")
        .to_string();

    Ok(content)
}

