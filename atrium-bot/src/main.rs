use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use atrium_proto::welcome::v1::{
    bot_control_service_client::BotControlServiceClient,
    calming_service_client::CalmingServiceClient, welcome_service_client::WelcomeServiceClient,
    BotStateRequest, ConversationScope, GenerateCalmingRequest, GenerateReplyRequest,
    SetBotStateRequest,
};
use platform_common::EventBus;
use serde::Deserialize;
use serenity::{
    all::{
        CommandInteraction, CommandOptionType, CreateCommand, CreateCommandOption,
        CreateInteractionResponse, CreateInteractionResponseMessage, GuildId, Interaction,
        Permissions,
    },
    async_trait,
    model::{
        channel::Message,
        gateway::Ready,
        guild::Member,
        id::{ChannelId, MessageId},
    },
    prelude::*,
};
use tonic::transport::Channel;
use tonic::Request;

mod logic;

const DISCORD_DIRECTORY_MAX_CHARS: usize = 6_000;
const MEMBERS_PER_ROLE: usize = 30;
const SENTINEL_EVENTS: EventBus = EventBus::new("sentinel:events");
const CALMING_COOLDOWN_SECS: u64 = 15 * 60;
/// Repli du seuil de depart eclair quand la cle `welcome_ghost_minutes` du
/// serveur est absente (aligne sur le defaut declare dans le `config_schema`).
const DEFAULT_GHOST_MINUTES: u64 = 30;
/// Duree au-dela de laquelle une entree du tracker d'accueils ne peut plus
/// servir : la plus longue fenetre acceptee par le schema (1440 min) tient
/// largement dedans. Le balayage se fait a l'insertion, sans tache de fond.

#[derive(Deserialize)]
struct CalmingEvent {
    event: String,
    data: CalmingEventData,
}

#[derive(Deserialize)]
struct CalmingEventData {
    guild_id: String,
    #[serde(default)]
    user_id: String,
    #[serde(default)]
    channel_id: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    kind: String,
}

#[derive(Clone)]
struct Config {
    token: String,
    grpc_url: String,
    grpc_token: String,
    general_channel_id: ChannelId,
    server_context: String,
}

impl Config {
    fn from_env() -> Self {
        Self {
            token: std::env::var("ATRIUM_DISCORD_TOKEN").expect("ATRIUM_DISCORD_TOKEN manquant"),
            grpc_url: std::env::var("ATRIUM_GRPC_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8091".into()),
            grpc_token: std::env::var("ATRIUM_GRPC_TOKEN").expect("ATRIUM_GRPC_TOKEN manquant"),
            general_channel_id: ChannelId::new(
                std::env::var("ATRIUM_GENERAL_CHANNEL_ID")
                    .expect("ATRIUM_GENERAL_CHANNEL_ID manquant")
                    .parse()
                    .expect("ATRIUM_GENERAL_CHANNEL_ID invalide"),
            ),
            server_context: std::env::var("ATRIUM_SERVER_CONTEXT").unwrap_or_default(),
        }
    }
}

struct Handler {
    config: Arc<Config>,
    channel: Channel,
    primary_guild: Arc<tokio::sync::RwLock<Option<GuildId>>>,
    calming_consumer_started: Arc<AtomicBool>,
    directory_cache: std::sync::RwLock<Option<(std::time::Instant, String)>>,
    /// Mots d'accueil postes dans le general, en attente de la fin de la
    /// fenetre de depart eclair : (guilde, membre) -> (instant, message).
    /// En memoire volontairement — la donnee ne vit que quelques minutes et
    /// atrium-bot n'a pas d'acces base. Un redemarrage perd la trace et laisse
    /// le message, ce qui est le bon sens de l'echec.
    ///
    /// On enregistre sans consulter le seuil : le lire a l'arrivee ajouterait
    /// un aller-retour gRPC sur un chemin chaud pour une decision qui ne se
    /// prend qu'au depart, lequel n'arrive presque jamais.
    welcomes:
        std::sync::Mutex<std::collections::HashMap<(u64, u64), (std::time::Instant, MessageId)>>,
}

impl Handler {
    /// Fenetre de depart eclair du serveur, en minutes (0 = desactive).
    ///
    /// Le defaut vit ici et pas cote API : `GetGuildConfig` renvoie les cles
    /// brutes, et c'est l'appelant qui sait ce qu'une cle absente signifie pour
    /// lui — meme repartition que `config_or` chez sentinel-bot. Une config
    /// injoignable retombe donc sur 30 : au pire on supprime un message
    /// d'accueil de trop, jamais un message legitime hors fenetre (le tracker
    /// borne deja les entrees a `GHOST_RETENTION`).
    async fn ghost_minutes(&self, guild_id: GuildId) -> u64 {
        let mut client = BotControlServiceClient::new(self.channel.clone());
        let request = self.grpc_request(BotStateRequest {
            guild_id: guild_id.to_string(),
        });
        match client.get_guild_config(request).await {
            Ok(response) => response
                .into_inner()
                .values
                .get("welcome_ghost_minutes")
                .and_then(|value| value.trim().parse().ok())
                .unwrap_or(DEFAULT_GHOST_MINUTES),
            Err(error) => {
                tracing::warn!(%error, "config depart eclair illisible, repli sur le defaut");
                DEFAULT_GHOST_MINUTES
            }
        }
    }

    fn grpc_request<T>(&self, message: T) -> Request<T> {
        let mut request = Request::new(message);
        let value = format!("Bearer {}", self.config.grpc_token)
            .parse()
            .expect("ATRIUM_GRPC_TOKEN invalide pour les metadonnees gRPC");
        request.metadata_mut().insert("authorization", value);
        request
    }

    async fn handle_calming_event(
        ctx: Context,
        config: Arc<Config>,
        channel: Channel,
        primary_guild: Arc<tokio::sync::RwLock<Option<GuildId>>>,
        payload: String,
    ) {
        let Ok(event) = serde_json::from_str::<CalmingEvent>(&payload) else {
            return;
        };
        if primary_guild
            .read()
            .await
            .map(|id| id.to_string())
            .as_deref()
            != Some(event.data.guild_id.as_str())
        {
            return;
        }

        if event.event == "atrium_welcome_requested" {
            let Ok(guild_num) = event.data.guild_id.parse::<u64>() else {
                return;
            };
            let Ok(user_num) = event.data.user_id.parse::<u64>() else {
                return;
            };
            let guild_id = GuildId::new(guild_num);
            let Ok(member) = guild_id.member(&ctx.http, user_num).await else {
                return;
            };
            let mut client = WelcomeServiceClient::new(channel);
            let mut request = Request::new(GenerateReplyRequest {
                guild_id: event.data.guild_id,
                member_id: event.data.user_id,
                member_display_name: member.display_name().to_string(),
                channel_id: config.general_channel_id.to_string(),
                scope: ConversationScope::General as i32,
                member_message: String::new(),
                server_context: config.server_context.clone(),
            });
            let Ok(bearer) = format!("Bearer {}", config.grpc_token).parse() else {
                return;
            };
            request.metadata_mut().insert("authorization", bearer);
            if let Ok(response) = client.generate_reply(request).await {
                let reply = response.into_inner().reply;
                let atrium_id = ctx.cache.current_user().id;
                let message = format!(
                    "<@{}> {reply}\n\n**Pour discuter avec moi dans ce salon, mentionne-moi : <@{}>.**",
                    member.user.id, atrium_id
                );
                if let Err(error) = config.general_channel_id.say(&ctx.http, message).await {
                    tracing::warn!(%error, "message d'accueil Atrium non envoye");
                }
            }
            return;
        }

        if event.event != "atrium_calming_requested" || event.data.reason != "channel_tension" {
            return;
        }

        // Atomique et partage entre replicas : un rappel maximum par salon et
        // par 15 min. Une tension dans #general ne doit pas empecher Atrium
        // d'apaiser un autre salon textuel.
        let Ok(client) = redis::Client::open(std::env::var("REDIS_URL").unwrap_or_default()) else {
            tracing::warn!("Rappel Atrium ignore: REDIS_URL invalide");
            return;
        };
        let Ok(mut conn) = client.get_multiplexed_async_connection().await else {
            tracing::warn!("Rappel Atrium ignore: Redis indisponible");
            return;
        };
        let key = format!(
            "atrium:calming:cooldown:{}:{}",
            event.data.guild_id, event.data.channel_id
        );
        let accepted: redis::RedisResult<Option<String>> = redis::cmd("SET")
            .arg(&key)
            .arg("1")
            .arg("NX")
            .arg("EX")
            .arg(CALMING_COOLDOWN_SECS)
            .query_async(&mut conn)
            .await;
        if !matches!(accepted, Ok(Some(_))) {
            return;
        }

        // Le message n'est plus figé ici : atrium-api le rédige par IA à partir
        // du contexte d'apaisement configuré pour ce serveur, et retombe sur un
        // rappel statique si l'IA (ou Atrium) est indisponible. Le bot reste une
        // interface légère — il ne décide pas du texte.
        let mut client = CalmingServiceClient::new(channel);
        let mut request = Request::new(GenerateCalmingRequest {
            guild_id: event.data.guild_id.clone(),
            channel_id: event.data.channel_id.clone(),
            kind: event.data.kind.clone(),
        });
        let bearer = match format!("Bearer {}", config.grpc_token).parse() {
            Ok(value) => value,
            Err(_) => {
                tracing::warn!("ATRIUM_GRPC_TOKEN invalide pour l'apaisement");
                return;
            }
        };
        request.metadata_mut().insert("authorization", bearer);
        let message = match client.generate_calming(request).await {
            Ok(response) => response.into_inner().reply,
            Err(error) => {
                // Le cooldown est déjà consommé : on ne réessaie pas, un rappel
                // manqué sur panne réseau est un incident bénin.
                tracing::warn!(%error, "apaisement Atrium: appel gRPC impossible");
                return;
            }
        };
        if message.trim().is_empty() {
            return;
        }
        // Le rappel est publie directement dans le salon ou Sentinel a
        // constate la tension. Le general reste le repli pour les evenements
        // emis par une ancienne version de Sentinel sans channel_id.
        let target_channel = event
            .data
            .channel_id
            .parse::<u64>()
            .map(ChannelId::new)
            .unwrap_or(config.general_channel_id);
        if let Err(error) = target_channel.say(&ctx.http, message).await {
            tracing::warn!(%error, "rappel apaisant Atrium non envoye");
        } else {
            tracing::info!(guild_id = %event.data.guild_id, channel_id = %target_channel, kind = %event.data.kind, "rappel apaisant Atrium envoye");
        }
    }

    async fn reply(
        &self,
        guild_id: String,
        member_id: String,
        name: String,
        channel_id: String,
        scope: ConversationScope,
        message: String,
        server_context: String,
    ) -> Option<String> {
        let mut client = WelcomeServiceClient::new(self.channel.clone());
        client
            .generate_reply(self.grpc_request(GenerateReplyRequest {
                guild_id,
                member_id,
                member_display_name: name,
                channel_id,
                scope: scope as i32,
                member_message: message,
                server_context,
            }))
            .await
            .ok()
            .map(|response| response.into_inner().reply)
    }

    fn server_context(&self, ctx: &Context, guild_id: GuildId) -> String {
        let Some(guild) = ctx.cache.guild(guild_id) else {
            return self.config.server_context.clone();
        };

        if let Ok(guard) = self.directory_cache.read() {
            if let Some((ts, cached)) = guard.as_ref() {
                if ts.elapsed() < std::time::Duration::from_secs(60) {
                    return cached.clone();
                }
            }
        }

        let mut roles: Vec<_> = guild
            .roles
            .values()
            .filter(|role| role.id.get() != guild_id.get() && !role.managed)
            .collect();
        roles.sort_by_key(|role| std::cmp::Reverse(role.position));

        let mut directory = String::from("\n\nAnnuaire Discord actuel (roles et membres):\n");
        for role in roles {
            let mut members: Vec<_> = guild
                .members
                .values()
                .filter(|member| member.roles.contains(&role.id))
                .map(|member| {
                    format!(
                        "{} (<@{}>)",
                        member.display_name().replace(['\n', '\r'], " "),
                        member.user.id
                    )
                })
                .collect();
            if members.is_empty() {
                continue;
            }
            members.sort_unstable();
            let remaining = members.len().saturating_sub(MEMBERS_PER_ROLE);
            members.truncate(MEMBERS_PER_ROLE);
            let suffix = if remaining > 0 {
                format!(", et {remaining} autre(s)")
            } else {
                String::new()
            };
            let line = format!(
                "- {} (<@&{}>): {}{}\n",
                role.name.replace(['\n', '\r'], " "),
                role.id,
                members.join(", "),
                suffix
            );
            if directory.chars().count() + line.chars().count() > DISCORD_DIRECTORY_MAX_CHARS {
                directory.push_str("- Annuaire tronque pour limiter la taille du contexte.\n");
                break;
            }
            directory.push_str(&line);
        }
        let result = format!("{}{}", self.config.server_context, directory);
        if let Ok(mut guard) = self.directory_cache.write() {
            *guard = Some((std::time::Instant::now(), result.clone()));
        }
        result
    }

    async fn control_reply(&self, command: &CommandInteraction) -> String {
        let Some(guild_id) = command.guild_id else {
            return "Cette commande doit etre utilisee sur le serveur.".into();
        };
        let is_admin = command
            .member
            .as_ref()
            .and_then(|member| member.permissions)
            .is_some_and(|permissions| permissions.administrator());
        if !is_admin {
            return "Seul un administrateur peut modifier l'etat d'Atrium.".into();
        }

        let action = command
            .data
            .options
            .first()
            .map(|option| option.name.as_str());
        let mut client = BotControlServiceClient::new(self.channel.clone());
        let result = match action {
            Some("activer") => client
                .set_state(self.grpc_request(SetBotStateRequest {
                    guild_id: guild_id.to_string(),
                    enabled: true,
                    actor_id: command.user.id.to_string(),
                }))
                .await
                .map(|response| response.into_inner()),
            Some("desactiver") => client
                .set_state(self.grpc_request(SetBotStateRequest {
                    guild_id: guild_id.to_string(),
                    enabled: false,
                    actor_id: command.user.id.to_string(),
                }))
                .await
                .map(|response| response.into_inner()),
            Some("statut") => client
                .get_state(self.grpc_request(BotStateRequest {
                    guild_id: guild_id.to_string(),
                }))
                .await
                .map(|response| response.into_inner()),
            _ => return "Sous-commande Atrium inconnue.".into(),
        };

        match result {
            Ok(state) if state.enabled => "Atrium est maintenant active sur ce serveur.".into(),
            Ok(_) => "Atrium est maintenant desactive sur ce serveur.".into(),
            Err(error) => {
                tracing::warn!(%error, "commande de controle Atrium impossible");
                "Impossible de modifier Atrium pour le moment.".into()
            }
        }
    }
}

fn atrium_command() -> CreateCommand {
    CreateCommand::new("atrium")
        .description("Activer ou desactiver Atrium")
        .default_member_permissions(Permissions::ADMINISTRATOR)
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "activer",
            "Active les reponses d'Atrium",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "desactiver",
            "Desactive toutes les reponses d'Atrium",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "statut",
            "Affiche l'etat actuel d'Atrium",
        ))
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        if let Some(guild) = ready.guilds.first() {
            *self.primary_guild.write().await = Some(guild.id);
        }
        for guild in &ready.guilds {
            if let Err(error) = guild
                .id
                .set_commands(&ctx.http, vec![atrium_command()])
                .await
            {
                tracing::warn!(%error, guild_id = %guild.id, "commande /atrium non enregistree");
            }
        }
        if !self.calming_consumer_started.swap(true, Ordering::SeqCst) {
            let consumer = platform_common::default_consumer_name();
            let config = Arc::clone(&self.config);
            let primary_guild = Arc::clone(&self.primary_guild);
            // Canal gRPC vers atrium-api, cloné une fois : `Channel` est un
            // handle partagé (multiplexé), pas une nouvelle connexion par event.
            let channel = self.channel.clone();
            tokio::spawn(async move {
                SENTINEL_EVENTS
                    .listen_stream_group("atrium-bot".to_string(), consumer, move |payload| {
                        let ctx = ctx.clone();
                        let config = Arc::clone(&config);
                        let channel = channel.clone();
                        let primary_guild = Arc::clone(&primary_guild);
                        async move {
                            Handler::handle_calming_event(
                                ctx,
                                config,
                                channel,
                                primary_guild,
                                payload,
                            )
                            .await
                        }
                    })
                    .await;
            });
        }
        tracing::info!(user = %ready.user.name, "Atrium Bot pret");
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let Interaction::Command(command) = interaction else {
            return;
        };
        if command.data.name != "atrium" {
            return;
        }
        let content = self.control_reply(&command).await;
        if let Err(error) = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(content)
                        .ephemeral(true),
                ),
            )
            .await
        {
            tracing::warn!(%error, "reponse a la commande /atrium impossible");
        }
    }

    async fn guild_member_addition(&self, ctx: Context, member: Member) {
        // Sentinel connait la configuration optionnelle du reglement. Il emet
        // `atrium_welcome_requested` a l'arrivee si aucun bouton n'est requis,
        // ou seulement apres l'acceptation dans le cas contraire.
        let _ = (ctx, member);
    }

    /// Depart eclair : le membre accueilli quitte le serveur dans la foulee.
    /// Le mot d'accueil d'Atrium s'adressait a quelqu'un qui n'est plus la —
    /// on le retire, comme Sentinel retire sa card de bienvenue.
    async fn guild_member_removal(
        &self,
        ctx: Context,
        guild_id: GuildId,
        user: serenity::model::user::User,
        _member: Option<Member>,
    ) {
        let entry = self
            .welcomes
            .lock()
            .ok()
            .and_then(|mut map| map.remove(&(guild_id.get(), user.id.get())));
        // Rien de suivi : membre accueilli avant le dernier redemarrage, ou
        // accueil jamais poste. On ne consulte meme pas la config.
        let Some((at, message_id)) = entry else {
            return;
        };
        let minutes = self.ghost_minutes(guild_id).await;
        if minutes == 0 || at.elapsed() > std::time::Duration::from_secs(minutes * 60) {
            return;
        }
        if let Err(error) = ctx
            .http
            .delete_message(
                self.config.general_channel_id,
                message_id,
                Some("Depart du membre dans la fenetre d'accueil"),
            )
            .await
        {
            tracing::warn!(%error, "message d'accueil non supprime (depart eclair)");
        } else {
            tracing::info!(user = %user.name, "Depart eclair : mot d'accueil Atrium retire");
        }
    }

    async fn message(&self, ctx: Context, message: Message) {
        if message.author.bot {
            return;
        }
        // Les MP sont une partie volontaire du parcours d'accueil. Dans le
        // general, le bot ne repond qu'a une mention pour eviter le spam.
        let is_direct = message.guild_id.is_none();
        let is_general = message.channel_id == self.config.general_channel_id;
        let is_mentioned =
            !is_direct && is_general && message.mentions_me(&ctx.http).await.unwrap_or(false);
        let scope = match logic::message_handling(is_direct, is_general, is_mentioned) {
            logic::MessageHandling::Ignore => return,
            logic::MessageHandling::Reply(scope) => scope,
        };
        let guild_id = match message.guild_id.or(*self.primary_guild.read().await) {
            Some(id) => id,
            None => return,
        };
        let server_context = self.server_context(&ctx, guild_id);
        if let Some(reply) = self
            .reply(
                guild_id.to_string(),
                message.author.id.to_string(),
                message.author.display_name().to_string(),
                message.channel_id.to_string(),
                scope,
                message.content.clone(),
                server_context,
            )
            .await
        {
            let mentioned_reply = format!("<@{}> {reply}", message.author.id);
            if let Err(error) = message.channel_id.say(&ctx.http, mentioned_reply).await {
                tracing::warn!(%error, "reponse Atrium non envoyee");
            }
        }
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt().init();
    let config = Arc::new(Config::from_env());
    let channel = Channel::from_shared(config.grpc_url.clone())
        .expect("ATRIUM_GRPC_URL invalide")
        .connect_lazy();
    let primary_guild = Arc::new(tokio::sync::RwLock::new(None));
    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&config.token, intents)
        .event_handler(Handler {
            config,
            channel,
            primary_guild,
            calming_consumer_started: Arc::new(AtomicBool::new(false)),
            directory_cache: std::sync::RwLock::new(None),
            welcomes: std::sync::Mutex::new(std::collections::HashMap::new()),
        })
        .await
        .expect("creation client Discord");
    client.start().await.expect("arret Atrium Bot");
}
