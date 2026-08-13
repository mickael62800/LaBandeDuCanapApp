//! Adaptateur gRPC consommé par `atrium-bot`.
//!
//! Cette surface vérifie le token, recharge la configuration par guilde,
//! applique les quotas, enrichit le contexte avec RAG et mémoire, puis
//! délègue la rédaction à `platform_core::atrium`.

use std::sync::Arc;

use platform_core::atrium::{
    domain::{CalmingRequest, ConflictKind, ConversationScope, WelcomeRequest},
    ports::inbound::{GenerateCalmingReplyUseCase, GenerateWelcomeReplyUseCase},
};
use platform_proto::atrium::welcome::v1::{
    self as proto,
    bot_control_service_server::{BotControlService, BotControlServiceServer},
    calming_service_server::{CalmingService, CalmingServiceServer},
    welcome_service_server::{WelcomeService, WelcomeServiceServer},
};
use sqlx::PgPool;
use tonic::{Request, Response, Status};

use std::collections::HashMap;

use crate::atrium::{
    budget::BudgetGuard, calming_use_case, control::BotControlStore, guild_config,
    memory::ConversationMemory, merge_context, rag::RagService, welcome_use_case, AppConfig,
};

pub async fn serve(
    config: AppConfig,
    pool: PgPool,
    rag: Arc<RagService>,
    budget: Arc<BudgetGuard>,
    control: Arc<BotControlStore>,
    memory: Arc<ConversationMemory>,
) {
    let addr = config.grpc_addr;
    // Même pool partagé que la surface HTTP : sert à lire les consignes de ton
    // par serveur (`welcome_context` / `conflict_context`) au fil des appels.
    let config_pool = Some(pool);
    let welcome_service = WelcomeGrpc {
        welcome: welcome_use_case(&config),
        rag: Some(rag.clone()),
        budget: Some(budget.clone()),
        control: Some(control.clone()),
        memory: Some(memory),
        config_pool: config_pool.clone(),
    };
    let calming_service = CalmingGrpc {
        calming: calming_use_case(&config),
        control: Some(control.clone()),
        budget: Some(budget.clone()),
        config_pool: config_pool.clone(),
    };
    let rag_service = RagGrpc { rag: Some(rag) };
    let control_service = BotControlGrpc { control };
    let expected = format!("Bearer {}", config.grpc_token);
    let auth = move |request: Request<()>| {
        // Comparaison constant-time, comme `bearer_auth::matches` cote HTTP et
        // l'interceptor gRPC de sentinel-api : un `==` sur un secret partage
        // court-circuite au premier octet different et laisse fuir sa longueur
        // et son prefixe par la latence.
        let fourni = request
            .metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        if bool::from(subtle::ConstantTimeEq::ct_eq(
            fourni.as_bytes(),
            expected.as_bytes(),
        )) {
            Ok(request)
        } else {
            Err(Status::unauthenticated(
                "jeton gRPC Atrium invalide ou absent",
            ))
        }
    };

    tracing::info!(%addr, "Atrium gRPC démarré (Welcome & RAG)");
    tonic::transport::Server::builder()
        .add_service(WelcomeServiceServer::with_interceptor(
            welcome_service,
            auth.clone(),
        ))
        .add_service(
            proto::rag_service_server::RagServiceServer::with_interceptor(
                rag_service,
                auth.clone(),
            ),
        )
        .add_service(BotControlServiceServer::with_interceptor(
            control_service,
            auth.clone(),
        ))
        .add_service(CalmingServiceServer::with_interceptor(
            calming_service,
            auth,
        ))
        .serve(addr)
        .await
        .expect("serveur gRPC Atrium");
}

pub struct WelcomeGrpc {
    pub welcome: Arc<dyn GenerateWelcomeReplyUseCase>,
    pub rag: Option<Arc<RagService>>,
    pub budget: Option<Arc<BudgetGuard>>,
    pub control: Option<Arc<BotControlStore>>,
    pub memory: Option<Arc<ConversationMemory>>,
    /// Lecture de `welcome_context` (ton d'accueil configuré par serveur).
    pub config_pool: Option<PgPool>,
}

impl WelcomeGrpc {
    pub fn new(welcome: Arc<dyn GenerateWelcomeReplyUseCase>) -> Self {
        Self {
            welcome,
            rag: None,
            budget: None,
            control: None,
            memory: None,
            config_pool: None,
        }
    }

    /// Instantane de la config du serveur, lu UNE fois par requete (P2).
    ///
    /// Auparavant, une reponse d'accueil relisait `bot_guild_config` trois fois
    /// (activation, quotas, `welcome_context`). On charge desormais une seule
    /// photographie et on en deduit tout. Vide si aucune base n'est branchee
    /// (tests) ; une erreur de lecture est remontee en 503 comme avant.
    async fn config_snapshot(&self, guild_id: &str) -> Result<HashMap<String, String>, Status> {
        match &self.config_pool {
            Some(pool) => guild_config::load(pool, guild_id).await.map_err(|error| {
                tracing::error!(%error, "Lecture de la config Atrium impossible");
                Status::unavailable("verification de l'etat indisponible")
            }),
            None => Ok(HashMap::new()),
        }
    }

    fn ensure_enabled(&self, config: &HashMap<String, String>) -> Result<(), Status> {
        if self.control.is_some() && !guild_config::enabled(config) {
            return Err(Status::failed_precondition("Atrium est desactive"));
        }
        Ok(())
    }

    async fn budget_message(
        &self,
        input: &proto::GenerateReplyRequest,
        config: &HashMap<String, String>,
    ) -> Result<Option<String>, Status> {
        let Some(budget) = &self.budget else {
            return Ok(None);
        };
        let limits = budget.settings_from_map(config);
        budget
            .check_and_record(
                &input.guild_id,
                &input.member_id,
                !input.member_message.trim().is_empty(),
                &limits,
            )
            .await
            .map_err(|error| {
                tracing::error!(%error, "Verification du budget DeepSeek gRPC impossible");
                Status::unavailable("verification du quota indisponible")
            })
    }

    async fn history(&self, guild_id: &str, member_id: &str) -> Result<String, Status> {
        match &self.memory {
            Some(memory) => memory.history(guild_id, member_id).await.map_err(|error| {
                tracing::error!(%error, "Lecture de la memoire Atrium impossible");
                Status::unavailable("lecture de la memoire indisponible")
            }),
            None => Ok(String::new()),
        }
    }

    async fn remember(&self, guild_id: &str, member_id: &str, message: &str, reply: &str) {
        if let Some(memory) = &self.memory {
            if let Err(error) = memory
                .remember_exchange(guild_id, member_id, message, reply)
                .await
            {
                tracing::warn!(%error, "Sauvegarde de la memoire Atrium impossible");
            }
        }
    }
}

pub struct RagGrpc {
    pub rag: Option<Arc<RagService>>,
}

#[tonic::async_trait]
impl WelcomeService for WelcomeGrpc {
    async fn generate_reply(
        &self,
        request: Request<proto::GenerateReplyRequest>,
    ) -> Result<Response<proto::GenerateReplyResponse>, Status> {
        let input = request.into_inner();
        // Une seule lecture de la config du serveur pour toute la requete (P2).
        let config = self.config_snapshot(&input.guild_id).await?;
        self.ensure_enabled(&config)?;
        if let Some(message) = self.budget_message(&input, &config).await? {
            return Ok(Response::new(proto::GenerateReplyResponse {
                reply: message,
                generated_by_ai: false,
            }));
        }
        let history = self.history(&input.guild_id, &input.member_id).await?;
        let guild_id = input.guild_id.clone();
        let member_id = input.member_id.clone();
        let member_message = input.member_message.clone();
        let scope = match proto::ConversationScope::try_from(input.scope)
            .unwrap_or(proto::ConversationScope::General)
        {
            proto::ConversationScope::General => ConversationScope::General,
            proto::ConversationScope::Direct => ConversationScope::Direct,
        };
        let retrieved = match &self.rag {
            Some(rag) => rag
                .context_for(&input.guild_id, &input.member_message)
                .await
                .map_err(|error| {
                    tracing::warn!(%error, "Recherche RAG gRPC indisponible");
                    Status::unavailable("recherche de connaissances indisponible")
                })?,
            None => String::new(),
        };
        let mut final_context = input.server_context.clone();
        if let Some(memory) = &self.memory {
            if let Ok(Some(summary)) = memory.get_latest_summary(&input.guild_id).await {
                final_context.push_str("\n\nRésumé de l'activité du serveur (récent) :\n");
                final_context.push_str(&summary);
            }
        }
        let admin_context = config.get("welcome_context").cloned().unwrap_or_default();
        let reply = self
            .welcome
            .reply(WelcomeRequest {
                guild_id: input.guild_id,
                member_id: input.member_id,
                member_display_name: input.member_display_name,
                channel_id: input.channel_id,
                scope,
                member_message: input.member_message,
                conversation_history: history,
                server_context: merge_context(&final_context, &retrieved),
                admin_context,
            })
            .await
            .map_err(|error| Status::invalid_argument(error.to_string()))?;
        self.remember(&guild_id, &member_id, &member_message, &reply.content)
            .await;
        Ok(Response::new(proto::GenerateReplyResponse {
            reply: reply.content,
            generated_by_ai: reply.generated_by_ai,
        }))
    }
}

/// Apaisement (« conflit ») : génère le rappel à publier dans un salon en tension.
pub struct CalmingGrpc {
    pub calming: Arc<dyn GenerateCalmingReplyUseCase>,
    pub control: Option<Arc<BotControlStore>>,
    /// Le meme plafond que l'accueil. L'apaisement appelait DeepSeek sans passer
    /// par le budget : il etait declenche par un evenement du bus Redis COMMUN
    /// (`atrium_calming_requested`), non signe, dont le `channel_id` sert de cle
    /// de cooldown — en faisant varier ce champ on obtenait autant d'appels
    /// payants qu'on voulait, sans qu'aucun compteur ne bouge.
    pub budget: Option<Arc<BudgetGuard>>,
    pub config_pool: Option<PgPool>,
}

/// Identite portee au compteur de quota pour les appels declenches par un
/// evenement plutot que par un membre. Un identifiant fixe suffit : ce qui
/// compte est que la depense s'impute au plafond quotidien de la guilde.
const CALMING_QUOTA_ACTOR: &str = "system:calming";

#[tonic::async_trait]
impl CalmingService for CalmingGrpc {
    async fn generate_calming(
        &self,
        request: Request<proto::GenerateCalmingRequest>,
    ) -> Result<Response<proto::GenerateCalmingResponse>, Status> {
        let input = request.into_inner();
        if input.guild_id.is_empty() || input.channel_id.is_empty() {
            return Err(Status::invalid_argument("guild_id ou channel_id manquant"));
        }
        let kind = ConflictKind::parse(&input.kind);

        // Une seule lecture de la config du serveur pour toute la requete (P2) :
        // l'activation et la consigne `conflict_context` en sont deduites.
        let config = match &self.config_pool {
            Some(pool) => guild_config::load(pool, &input.guild_id)
                .await
                .map_err(|error| {
                    tracing::error!(%error, "Verification de l'etat Atrium impossible");
                    Status::unavailable("verification de l'etat indisponible")
                })?,
            None => HashMap::new(),
        };

        // Atrium désactivé sur ce serveur : on ne consomme pas le modèle, mais
        // on publie quand même le rappel STATIQUE — l'apaisement est une consigne
        // de modération, pas une réponse à un membre. C'est le comportement
        // historique (messages figés), simplement conservé quand l'IA est coupée.
        if self.control.is_some() && !guild_config::enabled(&config) {
            return Ok(Response::new(proto::GenerateCalmingResponse {
                reply: kind.fallback_message().to_string(),
                generated_by_ai: false,
            }));
        }

        // Plafond quotidien de la guilde, applique AVANT l'appel payant.
        // `interactive = false` : le cooldown par membre n'a pas de sens pour un
        // declenchement automatique, seul le compteur journalier compte. Quota
        // atteint -> rappel statique, qui reste la bonne reponse de moderation.
        if let Some(budget) = &self.budget {
            let limits = budget.settings_from_map(&config);
            let bloque = budget
                .check_and_record(&input.guild_id, CALMING_QUOTA_ACTOR, false, &limits)
                .await
                .map_err(|error| {
                    tracing::error!(%error, "Verification du budget d'apaisement impossible");
                    Status::unavailable("verification du quota indisponible")
                })?;
            if bloque.is_some() {
                return Ok(Response::new(proto::GenerateCalmingResponse {
                    reply: kind.fallback_message().to_string(),
                    generated_by_ai: false,
                }));
            }
        }

        let admin_context = config.get("conflict_context").cloned().unwrap_or_default();
        let reply = self
            .calming
            .reply(CalmingRequest {
                guild_id: input.guild_id,
                channel_id: input.channel_id,
                kind,
                admin_context,
            })
            .await
            .map_err(|error| Status::invalid_argument(error.to_string()))?;
        Ok(Response::new(proto::GenerateCalmingResponse {
            reply: reply.content,
            generated_by_ai: reply.generated_by_ai,
        }))
    }
}

pub struct BotControlGrpc {
    control: Arc<BotControlStore>,
}

#[tonic::async_trait]
impl BotControlService for BotControlGrpc {
    async fn get_state(
        &self,
        request: Request<proto::BotStateRequest>,
    ) -> Result<Response<proto::BotStateResponse>, Status> {
        let guild_id = request.into_inner().guild_id;
        if guild_id.is_empty() {
            return Err(Status::invalid_argument("guild_id manquant"));
        }
        let enabled = self.control.is_enabled(&guild_id).await.map_err(|error| {
            tracing::error!(%error, "Lecture de l'etat Atrium impossible");
            Status::internal("lecture de l'etat impossible")
        })?;
        Ok(Response::new(proto::BotStateResponse { enabled }))
    }

    async fn get_guild_config(
        &self,
        request: Request<proto::BotStateRequest>,
    ) -> Result<Response<proto::GuildConfigResponse>, Status> {
        let guild_id = request.into_inner().guild_id;
        if guild_id.is_empty() {
            return Err(Status::invalid_argument("guild_id manquant"));
        }
        let values = self.control.raw_config(&guild_id).await.map_err(|error| {
            tracing::error!(%error, "Lecture de la config Atrium impossible");
            Status::internal("lecture de la config impossible")
        })?;
        Ok(Response::new(proto::GuildConfigResponse { values }))
    }

    async fn set_state(
        &self,
        request: Request<proto::SetBotStateRequest>,
    ) -> Result<Response<proto::BotStateResponse>, Status> {
        let input = request.into_inner();
        if input.guild_id.is_empty() || input.actor_id.is_empty() {
            return Err(Status::invalid_argument("guild_id ou actor_id manquant"));
        }
        self.control
            .set_enabled(&input.guild_id, input.enabled, &input.actor_id)
            .await
            .map_err(|error| {
                tracing::error!(%error, "Modification de l'etat Atrium impossible");
                Status::internal("modification de l'etat impossible")
            })?;
        tracing::info!(guild_id = %input.guild_id, actor_id = %input.actor_id, enabled = input.enabled, "Etat Atrium modifie");
        Ok(Response::new(proto::BotStateResponse {
            enabled: input.enabled,
        }))
    }
}

#[tonic::async_trait]
impl proto::rag_service_server::RagService for RagGrpc {
    async fn search_knowledge(
        &self,
        request: Request<proto::SearchKnowledgeRequest>,
    ) -> Result<Response<proto::SearchKnowledgeResponse>, Status> {
        let req = request.into_inner();
        let rag = self
            .rag
            .as_ref()
            .ok_or_else(|| Status::unavailable("RAG non configuré"))?;
        let chunks = rag
            .search_chunks(&req.guild_id, &req.query, req.limit)
            .await
            .map_err(Status::internal)?;

        let proto_chunks = chunks
            .into_iter()
            .map(
                |(source, content, similarity)| proto::SearchKnowledgeChunk {
                    source,
                    content,
                    similarity,
                },
            )
            .collect();

        Ok(Response::new(proto::SearchKnowledgeResponse {
            chunks: proto_chunks,
        }))
    }
}
