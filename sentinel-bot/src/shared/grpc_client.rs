//! Client gRPC partage entre les bots Sentinel (Phase 7A).
//!
//! Coexiste avec `BaseApiClient` (reqwest/HTTP) pendant la migration
//! bot-par-bot. Chaque bot stocke un `SentinelGrpcClient` dans son TypeMap
//! Serenity (cf. `GrpcClientKey`) en plus de l'`ApiClientKey` HTTP existant.
//!
//! Caracteristiques :
//! - Connexion HTTP/2 unique persistante (multiplexage natif tonic).
//! - Reconnexion lazy + retry transport gere par hyper en interne.
//! - Auth par metadata `authorization: Bearer <api_key>` injectee a chaque
//!   appel via interceptor (meme schema que cote serveur).
//! - Circuit breaker simple (cf. `circuit_breaker.rs`) pour degrader
//!   gracieusement quand l'API est down.
//!
//! ## Comportement si l'API tombe
//!
//! - **Reads** (`get_user_level`, `get_leaderboard`) : retournent
//!   `Err(GrpcCallError::Unavailable)` apres N echecs consecutifs, le circuit
//!   breaker s'ouvre pendant `cooldown` puis tente une requete (half-open).
//!   Les commandes slash doivent traduire ca en message « API indisponible,
//!   reessayez dans quelques instants » au lieu de planter.
//! - **Writes critiques** (`add_xp`) : meme comportement, le bot peut soit
//!   ignorer (XP perdu, acceptable), soit pousser dans Redis Streams pour
//!   replay differe (cf. `event_bus`).
//! - **Fire-and-forget** (record_messages, record_voice via HTTP legacy
//!   pour l'instant) : restent sur HTTP tant que la migration n'est pas
//!   terminee, geres par `BaseApiClient::post_fire_and_forget`.

use std::sync::Arc;
use std::time::Duration;
use std::{io, path::Path};

use serenity::prelude::TypeMapKey;
use tonic::codec::CompressionEncoding;
use tonic::metadata::MetadataValue;
use tonic::service::interceptor::InterceptedService;
use tonic::service::Interceptor;
use tonic::transport::{Channel, Endpoint};
use tonic::{Request, Status};
use tracing::{error, info, warn};

use platform_proto::sentinel::age_gate::v1::age_gate_service_client::AgeGateServiceClient;
use platform_proto::sentinel::ai_dataset::v1::ai_dataset_service_client::AiDatasetServiceClient;
use platform_proto::sentinel::announcements::v1::announcements_service_client::AnnouncementsServiceClient;
use platform_proto::sentinel::audit::v1::audit_service_client::AuditServiceClient;
use platform_proto::sentinel::automod::v1::automod_service_client::AutomodServiceClient;
use platform_proto::sentinel::automod_review::v1::automod_review_service_client::AutomodReviewServiceClient;
use platform_proto::sentinel::community::v1::community_service_client::CommunityServiceClient;
use platform_proto::sentinel::confessions::v1::confessions_service_client::ConfessionsServiceClient;
use platform_proto::sentinel::discord_messages::v1::discord_action_messages_service_client::DiscordActionMessagesServiceClient;
use platform_proto::sentinel::embeds::v1::embeds_service_client::EmbedsServiceClient;
use platform_proto::sentinel::guild_backup::v1::guild_backup_service_client::GuildBackupServiceClient;
use platform_proto::sentinel::ideas::v1::ideas_service_client::IdeasServiceClient;
use platform_proto::sentinel::members::v1::members_service_client::MembersServiceClient;
use platform_proto::sentinel::moderation::v1::moderation_service_client::ModerationServiceClient;
use platform_proto::sentinel::progression::v1::progression_service_client::ProgressionServiceClient;
use platform_proto::sentinel::purge::v1::purge_service_client::PurgeServiceClient;
use platform_proto::sentinel::roles::v1::role_panels_service_client::RolePanelsServiceClient;
use platform_proto::sentinel::security::v1::security_service_client::SecurityServiceClient;
use platform_proto::sentinel::security_state::v1::security_state_service_client::SecurityStateServiceClient;
use platform_proto::sentinel::stats::v1::stats_service_client::StatsServiceClient;
use platform_proto::sentinel::sursis::v1::sursis_service_client::SursisServiceClient;
use platform_proto::sentinel::tickets::v1::tickets_service_client::TicketsServiceClient;
use platform_proto::sentinel::voice::v1::voice_channels_service_client::VoiceChannelsServiceClient;
use platform_proto::sentinel::welcome::v1::welcome_service_client::WelcomeServiceClient;

use super::circuit_breaker::CircuitBreaker;

/// Erreurs renvoyees par les appels gRPC du client partage.
#[derive(Debug, thiserror::Error)]
pub enum GrpcCallError {
    #[error("API indisponible (circuit breaker ouvert)")]
    Unavailable,
    #[error("appel gRPC echoue : {0}")]
    Status(#[from] Status),
    #[error("erreur transport : {0}")]
    Transport(#[from] tonic::transport::Error),
}

/// Erreur de construction du canal gRPC. Lorsque TLS est configure, toute
/// erreur de certificat est fatale afin de ne jamais retomber en clair.
#[derive(Debug, thiserror::Error)]
pub enum GrpcClientInitError {
    #[error("erreur transport gRPC : {0}")]
    Transport(#[from] tonic::transport::Error),
    #[error("impossible de lire la configuration mTLS dans {dir}: {source}")]
    TlsFiles {
        dir: String,
        #[source]
        source: io::Error,
    },
}

/// Convertit un `GrpcCallError` en message **destine a l'utilisateur Discord**,
/// categorise par cause et prefixe d'un emoji distinctif :
///   - ⚠️  probleme transitoire (service down, timeout) -> reessayer
///   - ⏳  rate limit -> patienter
///   - ❌  erreur definitive (saisie invalide, droits, introuvable, bug serveur)
///
/// Le but est qu'on distingue clairement sur Discord ce qui s'est passe et quoi
/// faire, plutot qu'un opaque "gRPC InvalidArgument: ...". Pour les codes qui
/// portent une regle metier (InvalidArgument, FailedPrecondition, AlreadyExists,
/// NotFound), on affiche le message serveur tel quel (il est ecrit pour l'user) ;
/// pour les erreurs techniques (Internal, Unknown...) on masque le detail.
///
/// La plupart des api_clients de modules s'en servent.
/// Factorise le boilerplate des appels gRPC via le circuit breaker.
///
/// Reproduit exactement le motif repete dans les `api_client` des modules :
/// ```ignore
/// let mut client = self.grpc.<service>();
/// self.grpc
///     .guarded(|| async move { client.<method>(req).await.map(|r| r.into_inner()) })
///     .await
///     .map_err(grpc_err_to_string)
/// ```
///
/// Le premier argument est le handle gRPC (`self.grpc`, `&self.grpc`, `g`...),
/// evalue deux fois (construction du client + `guarded`), comme dans le code
/// d'origine. `grpc_err_to_string` est resolu **au site d'appel** (non
/// qualifie) : un module qui possede sa propre version locale garde donc son
/// comportement.
///
/// Variantes :
/// - `grpc_call!(handle, service, method, req)` : reponse unaire ->
///   `Result<Inner, String>`.
/// - `grpc_call!(@unit handle, service, method, req)` : ignore le corps ->
///   `Result<(), String>`.
/// - `grpc_call!(@raw handle, service, method, req)` : resultat brut
///   `Result<Inner, GrpcCallError>` (pour post-traitement custom : match
///   NotFound, `.ok()?`, `.map(...).map_err(...)`...).
/// - `grpc_call!(@raw_unit handle, service, method, req)` : idem mais corps
///   ignore -> `Result<(), GrpcCallError>`.
#[macro_export]
macro_rules! grpc_call {
    // Coeur interne : construit l'appel garde avec une transformation du Ok.
    (@guarded $grpc:expr, $svc:ident, $method:ident, $req:expr, $map:expr) => {{
        let mut client = $grpc.$svc();
        $grpc
            .guarded(|| async move { client.$method($req).await.map($map) })
            .await
    }};
    ($grpc:expr, $svc:ident, $method:ident, $req:expr) => {
        $crate::grpc_call!(@guarded $grpc, $svc, $method, $req, |r| r.into_inner())
            .map_err(grpc_err_to_string)
    };
    (@unit $grpc:expr, $svc:ident, $method:ident, $req:expr) => {
        $crate::grpc_call!(@guarded $grpc, $svc, $method, $req, |_| ())
            .map_err(grpc_err_to_string)
    };
    (@raw $grpc:expr, $svc:ident, $method:ident, $req:expr) => {
        $crate::grpc_call!(@guarded $grpc, $svc, $method, $req, |r| r.into_inner())
    };
    (@raw_unit $grpc:expr, $svc:ident, $method:ident, $req:expr) => {
        $crate::grpc_call!(@guarded $grpc, $svc, $method, $req, |_| ())
    };
}

pub fn grpc_err_to_string(e: GrpcCallError) -> String {
    use tonic::Code;
    match e {
        GrpcCallError::Unavailable => {
            "⚠️ Service momentanement indisponible, reessaye dans quelques instants.".to_string()
        }
        GrpcCallError::Transport(_) => {
            "⚠️ Connexion au service impossible, reessaye dans quelques instants.".to_string()
        }
        GrpcCallError::Status(s) => {
            let msg = s.message().trim();
            match s.code() {
                // Regles metier : le message serveur est ecrit pour l'utilisateur.
                Code::InvalidArgument
                | Code::FailedPrecondition
                | Code::OutOfRange
                | Code::AlreadyExists => {
                    if msg.is_empty() {
                        "❌ Action impossible.".to_string()
                    } else {
                        format!("❌ {msg}")
                    }
                }
                Code::NotFound => {
                    if msg.is_empty() {
                        "❌ Introuvable.".to_string()
                    } else {
                        format!("❌ {msg}")
                    }
                }
                Code::PermissionDenied | Code::Unauthenticated => {
                    "❌ Action non autorisee.".to_string()
                }
                Code::ResourceExhausted => {
                    "⏳ Trop de requetes, patiente un instant avant de reessayer.".to_string()
                }
                Code::DeadlineExceeded => {
                    "⚠️ Le service a mis trop de temps a repondre, reessaye.".to_string()
                }
                Code::Cancelled => "⚠️ Operation interrompue, reessaye.".to_string(),
                // Internal / Unknown / DataLoss / ... : pas de detail technique a l'user.
                _ => "❌ Erreur interne du service, reessaye plus tard.".to_string(),
            }
        }
    }
}

/// Client gRPC partage. Cloneable a moindre cout (Channel = Arc en interne).
#[derive(Clone)]
pub struct SentinelGrpcClient {
    channel: Channel,
    interceptor: AuthInterceptor,
    breaker: Arc<CircuitBreaker>,
}

impl SentinelGrpcClient {
    /// Construit un client a partir des variables d'environnement :
    /// - `GRPC_API_URL` (defaut : `http://127.0.0.1:50051`)
    /// - `API_KEY` (optionnelle, injectee dans `authorization`)
    pub async fn from_env() -> Result<Self, GrpcClientInitError> {
        let url =
            std::env::var("GRPC_API_URL").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string());
        let api_key = std::env::var("SENTINEL_API_KEY").unwrap_or_default();
        Self::connect(&url, &api_key).await
    }

    /// Construit un client en pointant explicitement une URL gRPC.
    pub async fn connect(url: &str, api_key: &str) -> Result<Self, GrpcClientInitError> {
        let tls_dir = platform_proto::sentinel::tls::tls_dir();
        let endpoint = build_endpoint(url, tls_dir.as_deref())?;

        if let Some(dir) = tls_dir {
            info!(dir = %dir.display(), "gRPC client TLS active (mTLS)");
        }

        let channel = endpoint.connect_lazy();
        info!(url = %url, "SentinelGrpcClient initialise (lazy connect)");

        Ok(Self {
            channel,
            interceptor: AuthInterceptor::new(api_key),
            breaker: Arc::new(CircuitBreaker::new(5, Duration::from_secs(10))),
        })
    }
}

fn build_endpoint(url: &str, tls_dir: Option<&Path>) -> Result<Endpoint, GrpcClientInitError> {
    // Si mTLS active, force https:// dans l'URL. tonic exige https
    // pour declencher le handshake TLS lors du connect.
    let effective_url = if tls_dir.is_some() {
        if let Some(rest) = url.strip_prefix("http://") {
            format!("https://{rest}")
        } else if !url.starts_with("https://") {
            format!("https://{url}")
        } else {
            url.to_string()
        }
    } else {
        url.to_string()
    };

    let endpoint = Endpoint::from_shared(effective_url)?
        // Phase 7A — tunings raisonnables. Le multiplexage HTTP/2 evite
        // de multiplier les connexions ; un seul Channel suffit pour tous
        // les RPC concurrents d'un meme bot.
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(30))
        .tcp_keepalive(Some(Duration::from_secs(60)))
        .http2_keep_alive_interval(Duration::from_secs(30))
        .keep_alive_timeout(Duration::from_secs(10))
        .keep_alive_while_idle(true);

    // mTLS optionnel : active si GRPC_TLS_DIR defini en env.
    // Domaine SAN du cert serveur = "api" (cf. gen-grpc-certs.sh).
    match tls_dir {
        Some(dir) => {
            let domain = url
                .strip_prefix("http://")
                .or_else(|| url.strip_prefix("https://"))
                .unwrap_or(url)
                .split(':')
                .next()
                .unwrap_or("api");
            let tls = platform_proto::sentinel::tls::client_tls_config(dir, domain).map_err(
                |source| GrpcClientInitError::TlsFiles {
                    dir: dir.display().to_string(),
                    source,
                },
            )?;
            Ok(endpoint.tls_config(tls)?)
        }
        None => Ok(endpoint),
    }
}

impl SentinelGrpcClient {
    // ── Helpers de service ──
    //
    // Phase 7A optimisations : chaque client annonce l'envoi et l'acceptation
    // de la compression Gzip. Le serveur (cf. `sentinel-api/src/adapters/inbound/grpc/server.rs`)
    // accepte les deux, donc les deux bouts negocient gzip automatiquement.

    /// Retourne un client `ProgressionService` pret a l'emploi.
    pub fn progression(
        &self,
    ) -> ProgressionServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        ProgressionServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `StatsService` pret a l'emploi.
    pub fn stats(&self) -> StatsServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        StatsServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `TicketsService` pret a l'emploi.
    pub fn tickets(&self) -> TicketsServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        TicketsServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `ModerationService` pret a l'emploi.
    pub fn moderation(
        &self,
    ) -> ModerationServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        ModerationServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `RolePanelsService` pret a l'emploi.
    pub fn role_panels(
        &self,
    ) -> RolePanelsServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        RolePanelsServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `MembersService` pret a l'emploi.
    pub fn members(&self) -> MembersServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        MembersServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `SecurityService` pret a l'emploi.
    pub fn security(&self) -> SecurityServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        SecurityServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `SecurityStateService` (miroir quarantaine/slowmode/lockdown).
    pub fn security_state(
        &self,
    ) -> SecurityStateServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        SecurityStateServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `AutomodReviewService` (cartes de review/vote).
    pub fn automod_review(
        &self,
    ) -> AutomodReviewServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        AutomodReviewServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `DiscordActionMessagesService` (mapping de sync).
    pub fn discord_messages(
        &self,
    ) -> DiscordActionMessagesServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        DiscordActionMessagesServiceClient::with_interceptor(
            self.channel.clone(),
            self.interceptor.clone(),
        )
        .send_compressed(CompressionEncoding::Gzip)
        .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `EmbedsService` (callback embed posté).
    pub fn embeds(&self) -> EmbedsServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        EmbedsServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `AgeGateService` (verification d'age au reglement).
    pub fn age_gate(&self) -> AgeGateServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        AgeGateServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `AnnouncementsService` (callbacks d'annonces).
    pub fn announcements(
        &self,
    ) -> AnnouncementsServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        AnnouncementsServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `ConfessionsService` (confessions anonymes).
    pub fn confessions(
        &self,
    ) -> ConfessionsServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        ConfessionsServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `SursisService` (« ban en sursis »).
    pub fn sursis(&self) -> SursisServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        SursisServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `AutomodService` pret a l'emploi.
    pub fn automod(&self) -> AutomodServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        AutomodServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `AuditService` (journal, surveillance, anomalies).
    pub fn audit(&self) -> AuditServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        AuditServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `GuildBackupService` (captures + roles en attente).
    pub fn guild_backup(
        &self,
    ) -> GuildBackupServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        GuildBackupServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `IdeasService` pret a l'emploi (boite a idees).
    pub fn ideas(&self) -> IdeasServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        IdeasServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `PurgeService` pret a l'emploi (commandes `/cleanup`).
    pub fn purge(&self) -> PurgeServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        PurgeServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `VoiceChannelsService` pret a l'emploi.
    pub fn voice_channels(
        &self,
    ) -> VoiceChannelsServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        VoiceChannelsServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Phase 7A.opt F.4 — Retourne un client `WelcomeService`.
    pub fn welcome(&self) -> WelcomeServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        WelcomeServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Phase 7A.opt F.3 — Retourne un client `CommunityService`.
    pub fn community(
        &self,
    ) -> CommunityServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        CommunityServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Retourne un client `AiDatasetService` pret a l'emploi.
    pub fn ai_dataset(
        &self,
    ) -> AiDatasetServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        AiDatasetServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone())
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
    }

    /// Wrappe un appel gRPC dans le circuit breaker. A utiliser dans les
    /// wrappers metier des bots pour beneficier de la degradation gracieuse.
    pub async fn guarded<F, Fut, T>(&self, call: F) -> Result<T, GrpcCallError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, Status>>,
    {
        if !self.breaker.allow() {
            warn!("Circuit breaker ouvert : appel gRPC court-circuite");
            return Err(GrpcCallError::Unavailable);
        }
        match call().await {
            Ok(v) => {
                self.breaker.record_success();
                Ok(v)
            }
            Err(status) => {
                if matches!(
                    status.code(),
                    tonic::Code::Unavailable
                        | tonic::Code::DeadlineExceeded
                        | tonic::Code::Internal
                ) {
                    self.breaker.record_failure();
                    error!(code = ?status.code(), "Echec gRPC compte par le circuit breaker");
                }
                Err(GrpcCallError::Status(status))
            }
        }
    }
}

/// Cle TypeMap pour stocker le `SentinelGrpcClient` dans le data store de Serenity.
pub struct GrpcClientKey;
impl TypeMapKey for GrpcClientKey {
    type Value = Arc<SentinelGrpcClient>;
}

// ── Interceptor d'auth ──

#[derive(Clone)]
pub struct AuthInterceptor {
    header: Option<MetadataValue<tonic::metadata::Ascii>>,
}

impl AuthInterceptor {
    fn new(api_key: &str) -> Self {
        let header = if api_key.is_empty() {
            None
        } else {
            match format!("Bearer {api_key}").parse::<MetadataValue<_>>() {
                Ok(v) => Some(v),
                Err(_) => {
                    error!("API_KEY invalide pour un header gRPC, auth desactivee cote client");
                    None
                }
            }
        };
        Self { header }
    }
}

impl Interceptor for AuthInterceptor {
    fn call(&mut self, mut req: Request<()>) -> Result<Request<()>, Status> {
        if let Some(h) = &self.header {
            req.metadata_mut().insert("authorization", h.clone());
        }
        Ok(req)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_api_key_yields_no_header() {
        let mut interceptor = AuthInterceptor::new("");
        let req = interceptor.call(Request::new(())).unwrap();
        assert!(req.metadata().get("authorization").is_none());
    }

    #[test]
    fn ascii_api_key_injects_bearer_header() {
        let mut interceptor = AuthInterceptor::new("topsecret");
        let req = interceptor.call(Request::new(())).unwrap();
        let header = req.metadata().get("authorization").expect("header present");
        assert_eq!(header.to_str().unwrap(), "Bearer topsecret");
    }

    #[test]
    fn invalid_api_key_chars_disable_auth_silently() {
        let mut interceptor = AuthInterceptor::new("bad\nkey\0");
        let req = interceptor.call(Request::new(())).unwrap();
        assert!(req.metadata().get("authorization").is_none());
    }

    #[test]
    fn interceptor_clone_preserves_header() {
        let interceptor = AuthInterceptor::new("abc123");
        let mut clone = interceptor.clone();
        let req = clone.call(Request::new(())).unwrap();
        assert_eq!(
            req.metadata()
                .get("authorization")
                .unwrap()
                .to_str()
                .unwrap(),
            "Bearer abc123"
        );
    }

    #[test]
    fn grpc_call_error_status_variant() {
        let status = Status::unavailable("api down");
        let err = GrpcCallError::Status(status);
        match err {
            GrpcCallError::Status(s) => assert_eq!(s.code(), tonic::Code::Unavailable),
            _ => panic!("expected Status variant"),
        }
    }

    #[test]
    fn grpc_call_error_unavailable_display() {
        let err = GrpcCallError::Unavailable;
        let msg = format!("{err}");
        assert!(msg.contains("indisponible") || msg.contains("circuit breaker"));
    }

    #[test]
    fn missing_tls_configuration_allows_plain_http2() {
        assert!(build_endpoint("http://127.0.0.1:50051", None).is_ok());
    }

    #[test]
    fn configured_but_unreadable_tls_directory_is_rejected() {
        let missing =
            std::env::temp_dir().join(format!("sentinel-bot-missing-tls-{}", std::process::id()));

        let error = build_endpoint("http://api:50051", Some(&missing)).unwrap_err();

        assert!(matches!(error, GrpcClientInitError::TlsFiles { .. }));
        assert!(error.to_string().contains(&missing.display().to_string()));
    }

    #[test]
    fn configured_but_invalid_tls_certificates_are_rejected() {
        let dir = std::env::temp_dir().join(format!(
            "sentinel-bot-invalid-tls-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        for file in ["client.pem", "client.key", "ca.pem"] {
            std::fs::write(dir.join(file), b"not a PEM certificate").unwrap();
        }

        let error = build_endpoint("http://api:50051", Some(&dir)).unwrap_err();

        assert!(matches!(error, GrpcClientInitError::Transport(_)));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
