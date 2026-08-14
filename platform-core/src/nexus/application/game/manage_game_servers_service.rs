//! Use case principal Game Portal — orchestre Docker, Postgres, Redis, RCON.
//!
//! Securite & robustesse :
//!  - Whitelist templates : valide que le slug demande est dans
//!    `allowed_templates` (config bot guild).
//!  - Quota : verifie max_servers_per_guild et max_memory_total_mb avant create.
//!  - Validation memory_mb dans les bornes du template.
//!  - Allocation atomique des ports via Redis SETNX (deux ranges).
//!  - Audit log systematique de chaque action.
//!  - Etat DB persiste avant tout appel Docker (rollback en cas d'erreur).

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::nexus::application::game::config_loader::{load_game_portal_config, GamePortalConfig};
use crate::nexus::application::game::password_gen::generate_rcon_password;
use crate::nexus::domain::entities::game::audit::GameAuditAction;
use crate::nexus::domain::entities::game::quota::GuildQuotaState;
use crate::nexus::domain::entities::game::server::{
    validate_server_name, CreateGameServerCommand, GameServer, GameServerStatus,
};
use crate::nexus::domain::entities::game::template::GameTemplate;
use crate::nexus::domain::errors::DomainError;
use crate::nexus::ports::inbound::game::manage_game_servers::{
    GameServerDetail, ManageGameServersUseCase, RequestIpRevealOutcome,
};
use crate::nexus::ports::outbound::game::container_runtime::{
    ContainerRuntime, ContainerSpec, ContainerStats, PortMapping, PortProtocol, RestartPolicy,
    VolumeMount,
};
use crate::nexus::ports::outbound::game::game_audit_repository::GameAuditRepository;
use crate::nexus::ports::outbound::game::game_server_config_repository::GameServerConfigRepository;
use crate::nexus::ports::outbound::game::game_server_repository::{
    GameServerRepository, GameServerRuntimeUpdate, NewGameServer,
};
use crate::nexus::ports::outbound::game::game_template_repository::GameTemplateRepository;
use crate::nexus::ports::outbound::game::port_allocator::{PortAllocator, PortKind};
use crate::nexus::ports::outbound::game::rcon_client::{RconClient, RconConnectionParams};
use crate::nexus::ports::outbound::system::bot_config_repository::BotConfigRepository;

pub struct ManageGameServersService {
    pub server_repo: Arc<dyn GameServerRepository>,
    pub template_repo: Arc<dyn GameTemplateRepository>,
    pub config_repo: Arc<dyn GameServerConfigRepository>,
    pub audit_repo: Arc<dyn GameAuditRepository>,
    pub container_runtime: Arc<dyn ContainerRuntime>,
    pub rcon_client: Arc<dyn RconClient>,
    pub port_allocator: Arc<dyn PortAllocator>,
    pub bot_config: Arc<dyn BotConfigRepository>,
}

mod provisioning;

/// L'erreur de start correspond-elle a un reseau Docker introuvable ? Docker
/// repond « network <id> not found » quand le conteneur reference un reseau qui
/// n'existe plus (recreation du reseau, `docker network rm`). Detection par
/// motif : l'erreur traverse docker-agent puis http_runtime sous forme de texte
/// (le detail est inclus dans le message par `map_error`).
fn is_missing_network_error(e: &DomainError) -> bool {
    let msg = e.to_string().to_ascii_lowercase();
    msg.contains("network") && msg.contains("not found")
}

#[async_trait]
impl ManageGameServersUseCase for ManageGameServersService {
    async fn create(&self, cmd: CreateGameServerCommand) -> Result<GameServer, DomainError> {
        let cfg = load_game_portal_config(&self.bot_config, &cmd.guild_id).await?;
        let template = self.validate_create(&cmd, &cfg).await?;

        let memory = cmd
            .allocated_memory_mb
            .unwrap_or(template.default_memory_mb);

        // Validation des configs initiales (keys + values vs schema template)
        // AVANT toute ecriture DB.
        for (k, v) in &cmd.initial_config {
            crate::nexus::domain::entities::game::config::validate_config_key(k)
                .map_err(DomainError::ValidationError)?;
            template
                .validate_config_value(k, v)
                .map_err(DomainError::ValidationError)?;
        }

        // 1. Creation DB (serveur + configs) en statut `created` (pas encore de container).
        let new = NewGameServer {
            guild_id: cmd.guild_id.clone(),
            template_id: template.id,
            name: cmd.name.clone(),
            allocated_memory_mb: memory,
            // Borne le plafond CPU demande : au-dela, c'est l'host qu'on met
            // en danger. En dessous de 0.5 coeur, un serveur de jeu ne tourne
            // simplement pas. Plafond max strict : 6 coeurs.
            cpu_limit: cmd.cpu_limit.map(|c| c.clamp(0.5, 6.0)),
            owner_user_id: cmd.owner_user_id.clone(),
            idle_shutdown_days: None,
            initial_config: cmd.initial_config,
        };
        let server = self.server_repo.create(new).await?;
        let server_id = server.id;
        info!(server_id = %server_id, guild_id = %cmd.guild_id, "game_server cree (DB)");

        // 3. Audit
        self.audit(
            &cmd.guild_id,
            Some(server_id),
            Some(&cmd.owner_user_id),
            GameAuditAction::Create,
            serde_json::json!({
                "template": template.slug,
                "memory_mb": memory,
                "name": cmd.name,
            }),
        )
        .await;

        Ok(server)
    }

    async fn list_for_guild(&self, guild_id: &str) -> Result<Vec<GameServer>, DomainError> {
        self.server_repo.list_by_guild(guild_id).await
    }

    async fn get(&self, id: Uuid) -> Result<GameServerDetail, DomainError> {
        let server = self
            .server_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("game_server {id} introuvable")))?;
        let config = self.config_repo.get_all(id).await?;
        Ok(GameServerDetail { server, config })
    }

    async fn reveal_ip(&self, id: Uuid, actor_user_id: &str) -> Result<(), DomainError> {
        let server = self
            .server_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("game_server {id} introuvable")))?;

        if server.ip_revealed {
            return Err(DomainError::Conflict(
                "l'adresse du serveur est deja revelee".into(),
            ));
        }
        if server.status != GameServerStatus::Running {
            return Err(DomainError::Conflict(
                "le serveur doit etre en ligne pour reveler son adresse".into(),
            ));
        }
        let host_port = server.host_port.ok_or_else(|| {
            DomainError::Conflict("le port public n'est pas encore alloue".into())
        })?;
        let portal_config = self
            .bot_config
            .get_config(&server.guild_id, "game-portal")
            .await?;
        let public_host = crate::nexus::domain::entities::system::bot_config::cfg_str(
            &portal_config,
            "session_public_host",
        )
        .filter(|host| !host.trim().is_empty())
        .ok_or_else(|| {
            DomainError::Conflict(
                "l'hote public Nexus doit etre configure avant la revelation".into(),
            )
        })?;

        self.server_repo.mark_ip_revealed(id).await?;
        self.audit(
            &server.guild_id,
            Some(id),
            Some(actor_user_id),
            GameAuditAction::IpReveal,
            serde_json::json!({
                "scheduled_at": server.ip_reveal_at,
                "public_host": public_host,
                "host_port": host_port,
                "forced": true,
            }),
        )
        .await;
        Ok(())
    }

    async fn request_ip_reveal(
        &self,
        id: Uuid,
        actor_user_id: &str,
    ) -> Result<RequestIpRevealOutcome, DomainError> {
        use crate::nexus::domain::entities::system::bot_config;

        let server = self
            .server_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("game_server {id} introuvable")))?;

        if server.ip_revealed {
            return Err(DomainError::Conflict(
                "l'adresse du serveur est deja revelee".into(),
            ));
        }

        let portal_config = self
            .bot_config
            .get_config(&server.guild_id, "game-portal")
            .await?;

        // Hote public requis DES MAINTENANT : sans lui, le worker ne pourrait
        // rien reveler a l'echeance et le clic resterait sans effet visible.
        // On echoue en fermeture avec un message clair plutot qu'en silence.
        bot_config::cfg_str(&portal_config, "session_public_host")
            .filter(|host| !host.trim().is_empty())
            .ok_or_else(|| {
                DomainError::Conflict(
                    "l'hote public Nexus doit etre configure avant l'ouverture".into(),
                )
            })?;

        let delay = bot_config::cfg_i64(&portal_config, "reveal_delay_minutes", 10).clamp(1, 1440);

        // Faut-il demarrer le conteneur ? On DECIDE ici mais on ne demarre PAS :
        // le pull d'image + create + start prennent des minutes (image de 8 Go
        // pour certains jeux) et bloqueraient la requete au-dela du timeout HTTP
        // client. L'appelant (handler) lance `start` en tache de fond ; le
        // worker fera passer l'etat a `running`. Un serveur deja en ligne ou en
        // cours de demarrage n'est pas relance ; tout autre etat transitoire
        // (Stopping) ou terminal (Deleted) refuse en fermeture.
        let started = match server.status {
            GameServerStatus::Running | GameServerStatus::Starting => false,
            status if status.can_start() => true,
            status => {
                return Err(DomainError::Conflict(format!(
                    "impossible d'ouvrir le serveur depuis le statut {status:?}"
                )));
            }
        };

        let reveal_at = chrono::Utc::now() + chrono::Duration::minutes(delay);
        self.server_repo
            .set_ip_reveal_at(id, Some(reveal_at))
            .await?;
        self.audit(
            &server.guild_id,
            Some(id),
            Some(actor_user_id),
            GameAuditAction::Schedule,
            serde_json::json!({
                "reveal_at": reveal_at,
                "delay_minutes": delay,
                "started": started,
                "trigger": "reveal_request",
            }),
        )
        .await;
        info!(
            server_id = %id,
            %reveal_at,
            delay_minutes = delay,
            started,
            "ouverture demandee via le bouton (reveal_request)"
        );

        Ok(RequestIpRevealOutcome {
            delay_minutes: delay,
            reveal_at,
            started,
        })
    }

    async fn schedule(
        &self,
        id: Uuid,
        reveal_at: chrono::DateTime<chrono::Utc>,
        actor_user_id: &str,
    ) -> Result<(), DomainError> {
        let server = self
            .server_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("game_server {id} introuvable")))?;

        if reveal_at <= chrono::Utc::now() {
            return Err(DomainError::ValidationError(
                "l'heure d'ouverture doit etre dans le futur".into(),
            ));
        }

        // On ne programme que depuis un etat au repos (jamais un serveur en
        // pleine transition ou deja en ligne). Re-programmer un serveur deja
        // `scheduled` est permis (ajustement de l'heure). Claim atomique.
        let claimed = self
            .server_repo
            .try_transition_status(
                id,
                &[
                    GameServerStatus::Created,
                    GameServerStatus::Scheduled,
                    GameServerStatus::Stopped,
                    GameServerStatus::Error,
                ],
                GameServerStatus::Scheduled,
            )
            .await?;
        if !claimed {
            return Err(DomainError::Conflict(format!(
                "impossible de programmer depuis le statut {:?}",
                server.status
            )));
        }

        self.server_repo
            .set_ip_reveal_at(id, Some(reveal_at))
            .await?;
        self.audit(
            &server.guild_id,
            Some(id),
            Some(actor_user_id),
            GameAuditAction::Schedule,
            serde_json::json!({ "reveal_at": reveal_at }),
        )
        .await;
        info!(server_id = %id, %reveal_at, "game_server programme (scheduled)");
        Ok(())
    }

    async fn set_reveal_schedule(
        &self,
        id: Uuid,
        reveal_at: Option<chrono::DateTime<chrono::Utc>>,
        actor_user_id: &str,
    ) -> Result<(), DomainError> {
        let server = self
            .server_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("game_server {id} introuvable")))?;

        if let Some(at) = reveal_at {
            if at <= chrono::Utc::now() {
                return Err(DomainError::ValidationError(
                    "l'heure de revelation doit etre dans le futur".into(),
                ));
            }
        }

        self.server_repo.set_ip_reveal_at(id, reveal_at).await?;
        self.audit(
            &server.guild_id,
            Some(id),
            Some(actor_user_id),
            GameAuditAction::Schedule,
            serde_json::json!({ "reveal_at": reveal_at }),
        )
        .await;
        Ok(())
    }

    async fn delete(&self, id: Uuid, actor_user_id: &str) -> Result<(), DomainError> {
        let server = self
            .server_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("game_server {id} introuvable")))?;

        // Claim ATOMIQUE du delete (F4) : passe un etat STABLE -> Deleted en une
        // requete. Refuse si le serveur est en pleine transition (Starting/
        // Stopping) -> evite la course delete-pendant-start qui laissait un
        // conteneur + volume orphelins et des ports fuites.
        let claimed = self
            .server_repo
            .try_transition_status(
                id,
                &[
                    GameServerStatus::Created,
                    GameServerStatus::Scheduled,
                    GameServerStatus::Running,
                    GameServerStatus::Stopped,
                    GameServerStatus::Error,
                ],
                GameServerStatus::Deleted,
            )
            .await?;
        if !claimed {
            // Le claim stable a echoue : le serveur est en transition
            // (Starting/Stopping). On autorise QUAND MEME la suppression si cette
            // transition dure depuis plus que le seuil « bloque » — c'est alors un
            // etat fige (opeation morte), pas une operation en cours. En deca du
            // seuil, on refuse : une vraie operation pourrait etre en vol (ex. un
            // pull d'image long), et un delete la doublerait -> conteneur orphelin.
            // Un eventuel orphelin (delete pendant un pull tres long) est rattrape
            // par le nettoyage d'orphelins du reconciler.
            let cfg = load_game_portal_config(&self.bot_config, &server.guild_id).await?;
            let stuck_after = chrono::Duration::minutes(cfg.stuck_transition_threshold_minutes);
            let bloque = matches!(
                server.status,
                GameServerStatus::Starting | GameServerStatus::Stopping
            ) && (chrono::Utc::now() - server.updated_at) > stuck_after;

            let forced = bloque
                && self
                    .server_repo
                    .try_transition_status(
                        id,
                        &[GameServerStatus::Starting, GameServerStatus::Stopping],
                        GameServerStatus::Deleted,
                    )
                    .await?;
            if !forced {
                return Err(DomainError::Conflict(
                    "operation deja en cours sur ce serveur (delete)".into(),
                ));
            }
            warn!(
                server_id = %id,
                status = ?server.status,
                "delete force d'un serveur bloque en transition au-dela du seuil"
            );
        }

        // Stop si actif (best-effort).
        if let Some(cid) = &server.container_id {
            if server.status.is_active() {
                let cfg = load_game_portal_config(&self.bot_config, &server.guild_id).await?;
                if let Err(e) = self
                    .container_runtime
                    .stop_container(cid, cfg.stop_timeout_secs)
                    .await
                {
                    warn!(error = %e, "stop avant delete a echoue");
                }
            }
            if let Err(e) = self.container_runtime.remove_container(cid).await {
                warn!(error = %e, "remove container a echoue");
            }
        }

        // Volume
        if let Some(vol) = &server.volume_name {
            if let Err(e) = self.container_runtime.remove_volume(vol).await {
                warn!(error = %e, volume = %vol, "remove volume a echoue (peut-etre encore utilise)");
            }
        }

        // Liberation des ports. Valheim reserve un bloc de trois ports UDP
        // consecutifs (jeu + query Steam + port additionnel).
        if let Some(p) = server.host_port {
            let width = self
                .template_repo
                .find_by_id(server.template_id)
                .await
                .ok()
                .flatten()
                .filter(|template| template.slug == "valheim")
                .map(|_| 3)
                .unwrap_or(1);
            for offset in 0..width {
                if let Err(e) = self
                    .port_allocator
                    .release(PortKind::Game, p + offset)
                    .await
                {
                    warn!(error = %e, port = p + offset, "liberation port jeu apres delete echouee");
                }
            }
        }
        if let Some(p) = server.rcon_port {
            let _ = self.port_allocator.release(PortKind::Rcon, p).await;
        }

        self.server_repo.soft_delete(id).await?;
        self.audit(
            &server.guild_id,
            Some(id),
            Some(actor_user_id),
            GameAuditAction::Delete,
            serde_json::json!({ "name": server.name }),
        )
        .await;
        Ok(())
    }

    async fn start(&self, id: Uuid, actor_user_id: &str) -> Result<(), DomainError> {
        let server = self
            .server_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("game_server {id} introuvable")))?;
        if !server.status.can_start() {
            return Err(DomainError::Conflict(format!(
                "transition start invalide depuis status {:?}",
                server.status
            )));
        }
        let cfg = load_game_portal_config(&self.bot_config, &server.guild_id).await?;
        let template = self
            .template_repo
            .find_by_id(server.template_id)
            .await?
            .ok_or_else(|| DomainError::Internal("template du serveur introuvable".into()))?;

        // Claim ATOMIQUE de la transition : passe Created/Stopped/Error ->
        // Starting en une seule requete. Si false, un autre start/stop est
        // deja en cours (le can_start ci-dessus n'est qu'un garde-fou cheap,
        // ce claim est le vrai verrou anti-concurrence).
        let claimed = self
            .server_repo
            .try_transition_status(
                id,
                &[
                    GameServerStatus::Created,
                    GameServerStatus::Scheduled,
                    GameServerStatus::Stopped,
                    GameServerStatus::Error,
                ],
                GameServerStatus::Starting,
            )
            .await?;
        if !claimed {
            return Err(DomainError::Conflict(
                "operation deja en cours sur ce serveur (start)".into(),
            ));
        }

        // Si pas de container_id encore -> create complete (alloue ports +
        // volume + container).
        let mut server = server;
        // Conteneur cree DANS cet appel ? Si oui, un echec de start ne peut pas
        // venir d'un conteneur perime (il vient d'etre bati contre le reseau
        // courant) : on ne tente pas de le recreer.
        let freshly_created = server.container_id.is_none();
        if server.container_id.is_none() {
            // On REUTILISE les ports/volume deja persistes (retry d'un start
            // precedent en Error) au lieu d'en reallouer — sinon les anciennes
            // cles Redis fuient (TTL 7j) et le range s'epuise. On ne (re)alloue
            // que ce qui n'est pas encore attribue. `newly_allocated` trace les
            // ports alloues DANS cet appel pour les liberer si la suite echoue.
            let preexisting_volume = server.volume_name.is_some();
            let mut newly_allocated: Vec<(PortKind, u16)> = Vec::new();

            let game_port = match server.host_port {
                Some(p) => p,
                None => {
                    let width = if template.slug == "valheim" { 3 } else { 1 };
                    let p = self
                        .port_allocator
                        .allocate_block(
                            PortKind::Game,
                            cfg.port_range_start,
                            cfg.port_range_end,
                            width,
                            &server.id.to_string(),
                        )
                        .await?;
                    for offset in 0..width {
                        newly_allocated.push((PortKind::Game, p + offset));
                    }
                    p
                }
            };

            let rcon_port = if template.supports_rcon && cfg.rcon_enabled {
                match server.rcon_port {
                    Some(p) => Some(p),
                    None => match self
                        .port_allocator
                        .allocate(
                            PortKind::Rcon,
                            cfg.rcon_port_range_start,
                            cfg.rcon_port_range_end,
                            &server.id.to_string(),
                        )
                        .await
                    {
                        Ok(p) => {
                            newly_allocated.push((PortKind::Rcon, p));
                            Some(p)
                        }
                        Err(e) => {
                            // Libere le game_port fraichement alloue avant de sortir.
                            self.release_ports(&newly_allocated).await;
                            return Err(e);
                        }
                    },
                }
            } else {
                server.rcon_port
            };

            // Reutilise le password existant si on reutilise un rcon_port,
            // sinon en genere un nouveau.
            //
            // EXCEPTION : sur les images ou RCON n'a pas de mot de passe propre
            // (Palworld : c'est l'`ADMIN_PASSWORD` qui fait foi), generer un
            // secret separe garantirait l'echec — la plateforme s'authentifierait
            // avec une valeur que le serveur ignore. On adopte donc le mot de
            // passe admin EFFECTIF, pour ne pas ecraser celui choisi dans
            // l'interface. S'il est vide, on retombe sur un secret genere, qui
            // devient alors l'`ADMIN_PASSWORD` du serveur.
            let rcon_password = match (&server.rcon_password, rcon_port) {
                (Some(p), Some(_)) => Some(p.clone()),
                (None, Some(_)) => {
                    let contrat =
                        crate::nexus::domain::entities::game::presence::rcon_env(&template.slug);
                    let mot_de_passe_partage = if contrat.password_key == "ADMIN_PASSWORD" {
                        let overrides = self.config_repo.get_all(id).await.unwrap_or_default();
                        Self::render_env(&template, &overrides)
                            .get(contrat.password_key)
                            .map(|v| v.trim().to_owned())
                            .filter(|v| !v.is_empty())
                    } else {
                        None
                    };
                    Some(mot_de_passe_partage.unwrap_or_else(generate_rcon_password))
                }
                _ => None,
            };

            let volume_name = server.volume_name.clone().or_else(|| {
                if cfg.auto_create_world_volume {
                    Some(GameServer::docker_volume_name(server.id))
                } else {
                    None
                }
            });
            let container_name = server
                .container_name
                .clone()
                .unwrap_or_else(|| GameServer::docker_container_name(server.id));

            server.host_port = Some(game_port);
            server.rcon_port = rcon_port;
            server.rcon_password = rcon_password.clone();
            server.volume_name = volume_name.clone();
            server.container_name = Some(container_name.clone());

            // Pre-requis Docker. En cas d'echec : on libere les ports alloues
            // DANS cet appel et on retire le volume si on vient de le creer,
            // puis status Error. Rien n'est encore persiste en DB -> pas de
            // ressource orpheline cote DB.
            if let Err(e) = self
                .container_runtime
                .ensure_network(&cfg.docker_network_name)
                .await
            {
                self.fail_start_cleanup(id, &newly_allocated, None, "ensure_network", &e)
                    .await?;
                return Err(e);
            }
            let mut volume_created = false;
            if let Some(vol) = &server.volume_name {
                if let Err(e) = self.container_runtime.ensure_volume(vol).await {
                    self.fail_start_cleanup(id, &newly_allocated, None, "ensure_volume", &e)
                        .await?;
                    return Err(e);
                }
                volume_created = !preexisting_volume;
            }
            // Volume retirable au cleanup uniquement si cree dans cet appel
            // (jamais un volume preexistant : il contient le monde du joueur).
            let removable_volume = if volume_created {
                server.volume_name.as_deref()
            } else {
                None
            };
            if let Err(e) = self
                .container_runtime
                .pull_image_if_missing(&template.image)
                .await
            {
                self.fail_start_cleanup(id, &newly_allocated, removable_volume, "pull_image", &e)
                    .await?;
                return Err(e);
            }

            // Build spec + create
            let overrides = self.config_repo.get_all(id).await?;
            let spec = self.build_spec(&server, &template, &overrides, &cfg);
            let cid = match self.container_runtime.create_container(&spec).await {
                Ok(cid) => cid,
                Err(e) => {
                    self.fail_start_cleanup(
                        id,
                        &newly_allocated,
                        removable_volume,
                        "create_container",
                        &e,
                    )
                    .await?;
                    return Err(e);
                }
            };
            // Succes create : on persiste TOUTES les ressources ensemble
            // (ports, volume, rcon, container). Avant ce point rien n'est
            // ecrit, donc un echec laisse la DB propre.
            self.server_repo
                .update_runtime(
                    id,
                    GameServerRuntimeUpdate {
                        container_id: Some(cid.clone()),
                        host_port: Some(game_port),
                        rcon_port,
                        rcon_password: rcon_password.clone(),
                        volume_name: volume_name.clone(),
                        container_name: Some(container_name.clone()),
                        ..Default::default()
                    },
                )
                .await?;
            server.container_id = Some(cid);
        }

        // Start
        let mut cid = server
            .container_id
            .clone()
            .ok_or_else(|| DomainError::Internal("container_id absent apres create".into()))?;

        // Upload des init files puis start, avec UNE tentative de recreation si
        // le conteneur reutilise pointe sur un reseau disparu. Ce cas survient
        // apres une recreation du reseau Docker (migration de labels, ou un
        // `docker network rm`) : le conteneur garde l'ancien ID de reseau et
        // Docker refuse le start avec « network ... not found ». Le monde est
        // sur le volume, pas dans le conteneur : le recreer ne perd rien.
        let mut recreated = false;
        loop {
            self.upload_init_files(id, &cid, &template).await?;
            match self.container_runtime.start_container(&cid).await {
                Ok(()) => break,
                Err(e) if !freshly_created && !recreated && is_missing_network_error(&e) => {
                    warn!(
                        error = %e,
                        server_id = %id,
                        "start: conteneur lie a un reseau disparu -> recreation du conteneur"
                    );
                    cid = match self.recreate_container(id, &server, &template, &cfg).await {
                        Ok(new_cid) => new_cid,
                        Err(recreate_err) => {
                            self.server_repo
                                .update_status(
                                    id,
                                    GameServerStatus::Error,
                                    Some(&format!("recreation conteneur: {recreate_err}")),
                                )
                                .await?;
                            return Err(recreate_err);
                        }
                    };
                    recreated = true;
                    // On repart en haut de la boucle : reupload des init files
                    // dans le NOUVEAU conteneur, puis nouvelle tentative de start.
                }
                Err(e) => {
                    self.server_repo
                        .update_status(id, GameServerStatus::Error, Some(&format!("start: {e}")))
                        .await?;
                    return Err(e);
                }
            }
        }

        self.server_repo
            .update_runtime(
                id,
                GameServerRuntimeUpdate {
                    status: Some(GameServerStatus::Running),
                    started_at_now: true,
                    clear_last_error: true,
                    ..Default::default()
                },
            )
            .await?;

        self.audit(
            &server.guild_id,
            Some(id),
            Some(actor_user_id),
            GameAuditAction::Start,
            serde_json::json!({ "host_port": server.host_port }),
        )
        .await;
        Ok(())
    }

    async fn stop(&self, id: Uuid, actor_user_id: &str) -> Result<(), DomainError> {
        let server = self
            .server_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("game_server {id} introuvable")))?;
        if !server.status.can_stop() {
            return Err(DomainError::Conflict(format!(
                "transition stop invalide depuis status {:?}",
                server.status
            )));
        }
        let cfg = load_game_portal_config(&self.bot_config, &server.guild_id).await?;
        // Claim atomique Running/Starting -> Stopping (verrou anti-concurrence).
        let claimed = self
            .server_repo
            .try_transition_status(
                id,
                &[GameServerStatus::Running, GameServerStatus::Starting],
                GameServerStatus::Stopping,
            )
            .await?;
        if !claimed {
            return Err(DomainError::Conflict(
                "operation deja en cours sur ce serveur (stop)".into(),
            ));
        }

        if let Some(cid) = &server.container_id {
            if let Err(e) = self
                .container_runtime
                .stop_container(cid, cfg.stop_timeout_secs)
                .await
            {
                error!(error = %e, "stop_container failed");
                self.server_repo
                    .update_status(id, GameServerStatus::Error, Some(&format!("stop: {e}")))
                    .await?;
                return Err(e);
            }
        }

        self.server_repo
            .update_runtime(
                id,
                GameServerRuntimeUpdate {
                    status: Some(GameServerStatus::Stopped),
                    stopped_at_now: true,
                    clear_last_error: true,
                    ..Default::default()
                },
            )
            .await?;

        self.audit(
            &server.guild_id,
            Some(id),
            Some(actor_user_id),
            GameAuditAction::Stop,
            serde_json::json!({}),
        )
        .await;
        Ok(())
    }

    async fn restart(&self, id: Uuid, actor_user_id: &str) -> Result<(), DomainError> {
        // restart = stop + start. On peut deleguer a Docker direct si container existe,
        // mais on prefere passer par notre logique (audit + transitions).
        self.stop(id, actor_user_id).await?;
        self.start(id, actor_user_id).await?;
        // Resout le guild_id reel depuis la ligne serveur (comme les autres
        // operations) plutot qu'un placeholder.
        let guild_id = self
            .server_repo
            .find_by_id(id)
            .await?
            .map(|s| s.guild_id)
            .unwrap_or_default();
        self.audit(
            &guild_id,
            Some(id),
            Some(actor_user_id),
            GameAuditAction::Restart,
            serde_json::json!({}),
        )
        .await;
        Ok(())
    }

    async fn get_logs(&self, id: Uuid, lines: u32) -> Result<Vec<String>, DomainError> {
        let server = self
            .server_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("game_server {id} introuvable")))?;
        let cid = server.container_id.ok_or_else(|| {
            DomainError::Conflict("container_id non defini (jamais demarre)".into())
        })?;
        let cfg = load_game_portal_config(&self.bot_config, &server.guild_id).await?;
        self.container_runtime
            .logs(&cid, lines.min(cfg.max_log_lines))
            .await
    }

    async fn get_stats(&self, id: Uuid) -> Result<ContainerStats, DomainError> {
        let server = self
            .server_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("game_server {id} introuvable")))?;
        let cid = server
            .container_id
            .ok_or_else(|| DomainError::Conflict("container_id non defini".into()))?;
        self.container_runtime.stats(&cid).await
    }

    async fn update_config(
        &self,
        id: Uuid,
        entries: HashMap<String, String>,
        actor_user_id: &str,
    ) -> Result<(), DomainError> {
        let server = self
            .server_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("game_server {id} introuvable")))?;
        // Validation des keys (SCREAMING_SNAKE_CASE) + values dans les bornes
        // declarees par le template (min/max, options, max_length).
        let template = self
            .template_repo
            .find_by_id(server.template_id)
            .await?
            .ok_or_else(|| DomainError::Internal("template du serveur introuvable".into()))?;
        for (k, v) in &entries {
            crate::nexus::domain::entities::game::config::validate_config_key(k)
                .map_err(DomainError::ValidationError)?;
            template
                .validate_config_value(k, v)
                .map_err(DomainError::ValidationError)?;
        }
        self.config_repo
            .replace_all(id, entries.clone(), Some(actor_user_id))
            .await?;
        self.audit(
            &server.guild_id,
            Some(id),
            Some(actor_user_id),
            GameAuditAction::ConfigUpdate,
            serde_json::json!({ "keys_updated": entries.keys().collect::<Vec<_>>() }),
        )
        .await;
        Ok(())
    }

    async fn execute_rcon(
        &self,
        id: Uuid,
        command: &str,
        actor_user_id: &str,
    ) -> Result<String, DomainError> {
        // Avant tout appel reseau : une commande malformee ne doit pas
        // atteindre le serveur de jeu, ni consommer une connexion RCON.
        valider_commande_rcon(command)?;

        let server = self
            .server_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("game_server {id} introuvable")))?;
        if server.status != GameServerStatus::Running {
            return Err(DomainError::Conflict(
                "le serveur doit etre running pour RCON".into(),
            ));
        }
        let cfg = load_game_portal_config(&self.bot_config, &server.guild_id).await?;
        if !cfg.rcon_enabled {
            return Err(DomainError::Forbidden("RCON desactive".into()));
        }
        let port = server
            .rcon_port
            .ok_or_else(|| DomainError::Conflict("rcon_port non alloue".into()))?;
        let pwd = server
            .rcon_password
            .ok_or_else(|| DomainError::Conflict("rcon_password non defini".into()))?;
        let params = RconConnectionParams {
            host: "127.0.0.1".to_string(),
            port,
            password: pwd,
            timeout_secs: 5,
        };
        let resultat = self.rcon_client.execute(&params, command).await;

        // Journalise la TENTATIVE, aboutie ou non.
        //
        // L'audit etait pose apres le `?` : une commande refusee par le serveur
        // de jeu, ou partie en timeout, ne laissait donc AUCUNE trace. C'est le
        // meme defaut que celui corrige cote OPS — « une operation refusee ou en
        // erreur n'apparait plus comme executee » — mais pris a l'envers : ici
        // elle n'apparaissait pas du tout. Or ce qu'on veut savoir apres coup,
        // c'est ce qui a ete TENTE, pas seulement ce qui a reussi.
        self.audit(
            &server.guild_id,
            Some(id),
            Some(actor_user_id),
            GameAuditAction::CommandRcon,
            serde_json::json!({ "cmd": command, "succes": resultat.is_ok() }),
        )
        .await;

        Ok(resultat?.raw)
    }
}

/// Longueur maximale d'une commande RCON.
///
/// Genereux a dessein : une commande Minecraft avec NBT depasse largement la
/// centaine de caracteres. Ce n'est pas une liste blanche — c'est une borne.
const RCON_COMMANDE_MAX: usize = 2_000;

/// Controle de FORME d'une commande RCON — pas de son contenu.
///
/// N4 reste un choix de produit : un panneau d'administration de serveur de jeu
/// sert precisement a executer des commandes, et en restreindre la liste
/// reviendrait a reimplementer la console. Ce qui suit ne restreint donc PAS ce
/// qu'un administrateur peut faire ; ca borne ce qui peut etre envoye :
///
///   - une commande vide n'a pas de sens et ferait un aller-retour pour rien ;
///   - les caracteres de controle (`\n`, `\r`, `\0`) n'appartiennent pas a une
///     commande. Le protocole RCON transporte UNE commande par paquet, mais
///     l'interpretation du corps appartient au serveur de jeu : selon les
///     implementations, un saut de ligne peut y etre lu comme un separateur.
///     Refuser ces caracteres coute une comparaison et retire la question ;
///   - une longueur bornee evite d'envoyer un corps qui depasserait le paquet.
///
/// Place dans le DOMAINE et non dans le handler : le bot Discord appelle le
/// meme use case, et un controle pose cote HTTP l'aurait laisse passer — c'est
/// exactement la remarque de l'audit sur l'emplacement d'une future restriction.
fn valider_commande_rcon(command: &str) -> Result<(), DomainError> {
    let commande = command.trim();
    if commande.is_empty() {
        return Err(DomainError::ValidationError(
            "commande RCON vide".to_string(),
        ));
    }
    if commande.chars().count() > RCON_COMMANDE_MAX {
        return Err(DomainError::ValidationError(format!(
            "commande RCON trop longue (max {RCON_COMMANDE_MAX} caracteres)"
        )));
    }
    if commande.chars().any(|c| c.is_control()) {
        return Err(DomainError::ValidationError(
            "commande RCON invalide : caractere de controle".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/manage_game_servers_service.rs"]
mod tests;
