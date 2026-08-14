use super::*;

/// Substitue `{{KEY}}` (avec spaces tolerees) par `env[KEY]`. Si la cle
/// n'existe pas, le placeholder est remplace par une chaine vide (comme
/// Docker compose pour les env unset). Volontairement minimaliste : pas
/// de logique conditionnelle, pas d'echappement. Suffit pour seed des
/// fichiers de config jeu.
pub(super) fn render_template(input: &str, env: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        if let Some(end) = after.find("}}") {
            let key = after[..end].trim();
            if let Some(v) = env.get(key) {
                out.push_str(v);
            }
            rest = &after[end + 2..];
        } else {
            // Placeholder non ferme : on emet le tail tel quel et on sort.
            out.push_str("{{");
            out.push_str(after);
            return out;
        }
    }
    out.push_str(rest);
    out
}

impl ManageGameServersService {
    /// Resout le template, valide whitelist, quotas et memoire demandee.
    pub(super) async fn validate_create(
        &self,
        cmd: &CreateGameServerCommand,
        cfg: &GamePortalConfig,
    ) -> Result<GameTemplate, DomainError> {
        if !cfg.enabled {
            return Err(DomainError::Forbidden(
                "Game Portal desactive pour cette guild (cf. config bot)".into(),
            ));
        }
        validate_server_name(&cmd.name).map_err(DomainError::ValidationError)?;

        // Whitelist template
        if !cfg
            .allowed_templates
            .iter()
            .any(|s| s == &cmd.template_slug)
        {
            return Err(DomainError::Forbidden(format!(
                "template '{}' non autorise pour cette guild",
                cmd.template_slug
            )));
        }
        let template = self
            .template_repo
            .find_by_slug(&cmd.template_slug)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("template slug={} introuvable", cmd.template_slug))
            })?;

        // Memoire
        let memory = cmd
            .allocated_memory_mb
            .unwrap_or(template.default_memory_mb);
        // Refus D'EMBLEE si la plateforme ne peut pas piloter de conteneurs.
        //
        // Sans ce controle, la creation reussissait et le serveur n'existait
        // qu'en base : il apparaissait dans la liste, en erreur, et chaque
        // tentative de demarrage renvoyait une erreur 500. Mieux vaut refuser
        // clairement que fabriquer quelque chose d'inutilisable.
        if !self.container_runtime.is_operational() {
            return Err(DomainError::NotImplemented(
                "La plateforme de jeux n'est pas activee : aucun serveur ne peut                  etre cree. Definis NEXUS_GAME_RUNTIME=docker dans .env, puis                  redemarre nexus-api."
                    .into(),
            ));
        }

        template
            .validate_memory(memory)
            .map_err(DomainError::ValidationError)?;

        // Quota guild
        let (active, mem_alloc) = self
            .server_repo
            .count_active_for_guild(&cmd.guild_id)
            .await?;
        let quota = GuildQuotaState {
            active_servers: active,
            max_servers: cfg.max_servers_per_guild,
            allocated_memory_mb: mem_alloc,
            max_memory_mb: cfg.max_memory_total_mb,
        };
        quota
            .can_create_server(memory)
            .map_err(|e| DomainError::ValidationError(e.to_string()))?;

        Ok(template)
    }

    /// Construit la spec Docker complete a partir du template + overrides + ports.
    pub(super) fn build_spec(
        &self,
        server: &GameServer,
        template: &GameTemplate,
        overrides: &HashMap<String, String>,
        cfg: &GamePortalConfig,
    ) -> ContainerSpec {
        // Env = default_env + overrides + RCON injecte si applicable.
        let mut env: HashMap<String, String> = template
            .default_env
            .as_object()
            .map(|m| {
                m.iter()
                    .map(|(k, v)| {
                        let val_str = match v {
                            serde_json::Value::String(s) => s.clone(),
                            serde_json::Value::Number(n) => n.to_string(),
                            serde_json::Value::Bool(b) => b.to_string(),
                            _ => v.as_str().map(String::from).unwrap_or_default(),
                        };
                        (k.clone(), val_str)
                    })
                    .collect()
            })
            .unwrap_or_default();
        for (k, v) in overrides {
            env.insert(k.clone(), v.clone());
        }
        if template.slug == "palworld" {
            if let Some(host_port) = server.host_port {
                env.insert("PUBLIC_PORT".to_string(), host_port.to_string());
                env.insert("PORT".to_string(), template.container_port.to_string());
            }
        }
        if template.supports_rcon && cfg.rcon_enabled {
            if let Some(pwd) = &server.rcon_password {
                env.insert("ENABLE_RCON".to_string(), "true".to_string());
                env.insert("RCON_PASSWORD".to_string(), pwd.clone());
                if let Some(p) = server.rcon_port {
                    // RCON_PORT du container : on garde 25575 par defaut Minecraft,
                    // le mapping host expose le port_alloue.
                    let _ = p;
                    env.insert("RCON_PORT".to_string(), "25575".to_string());
                }
            }
        }
        // Le CONTENEUR recoit plus que le jeu : une JVM configuree avec 2 Go
        // de tas en consomme davantage, et se fait tuer par le noyau si la
        // limite du conteneur vaut exactement son tas.
        let memory_bytes = (crate::nexus::domain::entities::game::server::container_memory_mb(
            server.allocated_memory_mb,
        ) as u64)
            * 1024
            * 1024;
        env.insert(
            "MEMORY".to_string(),
            format!("{}M", server.allocated_memory_mb),
        );

        let mut port_mappings = vec![];
        if let Some(host_port) = server.host_port {
            // Protocole defini par le template (TCP : Minecraft, Terraria ;
            // UDP : Valheim, Factorio, Palworld).
            let proto = match template.port_protocol {
                crate::nexus::domain::entities::game::template::PortProtocol::Tcp => {
                    PortProtocol::Tcp
                }
                crate::nexus::domain::entities::game::template::PortProtocol::Udp => {
                    PortProtocol::Udp
                }
            };
            port_mappings.push(PortMapping {
                host_port,
                container_port: template.container_port,
                protocol: proto,
                // Port jeu : exposé sur toutes les interfaces.
                host_ip: "0.0.0.0".to_string(),
            });
            // Pour Valheim (lloesche/valheim-server), Steam Query Port (container_port + 1) et
            // communication port (container_port + 2) doivent être exposés en UDP.
            if template.slug == "valheim" {
                port_mappings.push(PortMapping {
                    host_port: host_port + 1,
                    container_port: template.container_port + 1,
                    protocol: PortProtocol::Udp,
                    host_ip: "0.0.0.0".to_string(),
                });
                port_mappings.push(PortMapping {
                    host_port: host_port + 2,
                    container_port: template.container_port + 2,
                    protocol: PortProtocol::Udp,
                    host_ip: "0.0.0.0".to_string(),
                });
            }
        }
        if template.supports_rcon && cfg.rcon_enabled {
            if let Some(rcon_host_port) = server.rcon_port {
                // RCON est toujours TCP.
                port_mappings.push(PortMapping {
                    host_port: rcon_host_port,
                    container_port: 25575,
                    protocol: PortProtocol::Tcp,
                    // RCON = console admin : bind uniquement sur loopback,
                    // l'app s'y connecte via 127.0.0.1. JAMAIS exposé.
                    host_ip: "127.0.0.1".to_string(),
                });
            }
        }

        let volumes = if cfg.auto_create_world_volume {
            // Path interne specifique au template (Minecraft: /data,
            // Terraria: /root/.local/share/Terraria/Worlds, etc.)
            vec![VolumeMount {
                volume_name: server
                    .volume_name
                    .clone()
                    .unwrap_or_else(|| GameServer::docker_volume_name(server.id)),
                container_path: template.volume_path.clone(),
                read_only: false,
            }]
        } else {
            vec![]
        };

        // Labels de tracabilite, dans les DEUX generations : `nexus.*`
        // (canonique — les jeux ont quitte Sentinel au portage) et `sentinel.*`
        // (la flotte deja en service ne porte que celui-la, et le reconciler
        // s'en sert pour retrouver ses conteneurs). Voir `LEGACY_*` dans
        // `docker-agent/src/bollard_game.rs` pour la sortie de transition.
        let mut labels = HashMap::new();
        for prefix in ["nexus", "sentinel"] {
            labels.insert(format!("{prefix}.server_id"), server.id.to_string());
            labels.insert(format!("{prefix}.guild_id"), server.guild_id.clone());
            labels.insert(format!("{prefix}.template_slug"), template.slug.clone());
            labels.insert(format!("{prefix}.owner"), server.owner_user_id.clone());
        }

        // Command (templated) : si le template definit un override CMD, on
        // substitue les {{KEY}} par les env effectives (defaults + overrides
        // utilisateur), puis on passe au runtime. Sinon None.
        let command = template.command.as_ref().map(|tmpl| {
            tmpl.iter()
                .map(|arg| render_template(arg, &env))
                .collect::<Vec<_>>()
        });

        ContainerSpec {
            image: template.image.clone(),
            name: server
                .container_name
                .clone()
                .unwrap_or_else(|| GameServer::docker_container_name(server.id)),
            env,
            port_mappings,
            volumes,
            memory_bytes,
            cpu_limit: server.cpu_limit,
            network: cfg.docker_network_name.clone(),
            // Si run_as_root=true (Terraria, Valheim, Factorio) ou template palworld,
            // on ne passe pas --user et l'image utilise son user par defaut (root ou steam).
            user: if template.run_as_root || template.slug == "palworld" {
                None
            } else {
                Some(cfg.container_user.clone())
            },
            // L'auto-restart est gere par notre worker (pas Docker), pour
            // tracer chaque crash dans audit_log et appliquer du backoff.
            restart_policy: RestartPolicy::None,
            labels,
            command,
        }
    }

    /// Construit la map env effective (defaults + overrides) pour rendre
    /// les templates init_files / command. Reproduit la meme logique que
    /// `build_spec` mais sans injecter MEMORY/RCON (qui ne servent pas pour
    /// les fichiers de config jeu).
    pub(super) fn render_env(
        template: &GameTemplate,
        overrides: &HashMap<String, String>,
    ) -> HashMap<String, String> {
        let mut env: HashMap<String, String> = template
            .default_env
            .as_object()
            .map(|m| {
                m.iter()
                    .map(|(k, v)| {
                        let val_str = match v {
                            serde_json::Value::String(s) => s.clone(),
                            serde_json::Value::Number(n) => n.to_string(),
                            serde_json::Value::Bool(b) => b.to_string(),
                            _ => v.as_str().map(String::from).unwrap_or_default(),
                        };
                        (k.clone(), val_str)
                    })
                    .collect()
            })
            .unwrap_or_default();
        for (k, v) in overrides {
            env.insert(k.clone(), v.clone());
        }
        env
    }

    /// Libere best-effort une liste de ports (kind, port) dans le pool.
    /// Utilise sur les chemins d'echec de `start`.
    pub(super) async fn release_ports(&self, ports: &[(PortKind, u16)]) {
        for (kind, port) in ports {
            if let Err(e) = self.port_allocator.release(*kind, *port).await {
                warn!(error = %e, port = *port, "release port apres echec start a echoue");
            }
        }
    }

    /// Nettoyage commun d'un echec de `start` AVANT que le container soit
    /// demarre : libere les ports alloues dans cet appel, retire le volume
    /// fraichement cree (best-effort), puis bascule le serveur en Error.
    pub(super) async fn fail_start_cleanup(
        &self,
        id: Uuid,
        newly_allocated: &[(PortKind, u16)],
        removable_volume: Option<&str>,
        stage: &str,
        err: &DomainError,
    ) -> Result<(), DomainError> {
        self.release_ports(newly_allocated).await;
        if let Some(vol) = removable_volume {
            if let Err(e) = self.container_runtime.remove_volume(vol).await {
                warn!(error = %e, volume = %vol, "cleanup volume apres echec start a echoue");
            }
        }
        self.server_repo
            .update_status(
                id,
                GameServerStatus::Error,
                Some(&format!("{stage}: {err}")),
            )
            .await
    }

    pub(super) async fn audit(
        &self,
        guild_id: &str,
        server_id: Option<Uuid>,
        actor: Option<&str>,
        action: GameAuditAction,
        details: serde_json::Value,
    ) {
        if let Err(e) = self
            .audit_repo
            .log(guild_id, server_id, actor, action, details)
            .await
        {
            warn!(error = %e, "game_audit log failed");
        }
    }

    /// Upload les init files du template dans le conteneur avant start. No-op
    /// si le template n'en definit pas. Sur echec, passe le serveur en `Error`
    /// et propage. Extrait de `start` pour etre rejoue apres une recreation de
    /// conteneur (les fichiers doivent etre reinjectes dans le nouveau).
    pub(super) async fn upload_init_files(
        &self,
        id: Uuid,
        cid: &str,
        template: &GameTemplate,
    ) -> Result<(), DomainError> {
        if template.init_files.is_empty() {
            return Ok(());
        }
        let overrides = self.config_repo.get_all(id).await.unwrap_or_default();
        let render_env = Self::render_env(template, &overrides);
        for f in &template.init_files {
            let path = render_template(&f.path, &render_env);
            let content = render_template(&f.content, &render_env);
            if let Err(e) = self
                .container_runtime
                .upload_file_to_container(cid, &path, &content)
                .await
            {
                error!(error = %e, path = %path, "init_file upload echoue");
                self.server_repo
                    .update_status(
                        id,
                        GameServerStatus::Error,
                        Some(&format!("init_file {path}: {e}")),
                    )
                    .await?;
                return Err(e);
            }
        }
        Ok(())
    }

    /// Recree le conteneur d'un serveur dont le conteneur precedent est perime
    /// (typiquement lie a un reseau Docker disparu). Retire l'ancien conteneur
    /// (best-effort), garantit le reseau COURANT et le volume existant, puis
    /// recree le conteneur et persiste son nouvel ID.
    ///
    /// Les ports et le volume sont DEJA alloues et persistes : on les reutilise
    /// tels quels, on ne realloue rien. Le monde du joueur vit dans le volume,
    /// pas dans le conteneur : le recreer ne perd aucune donnee.
    pub(super) async fn recreate_container(
        &self,
        id: Uuid,
        server: &GameServer,
        template: &GameTemplate,
        cfg: &GamePortalConfig,
    ) -> Result<String, DomainError> {
        if let Some(old) = &server.container_id {
            if let Err(e) = self.container_runtime.remove_container(old).await {
                warn!(error = %e, "recreation: suppression de l'ancien conteneur a echoue (peut-etre deja absent)");
            }
        }
        self.container_runtime
            .ensure_network(&cfg.docker_network_name)
            .await?;
        if let Some(vol) = &server.volume_name {
            self.container_runtime.ensure_volume(vol).await?;
        }
        self.container_runtime
            .pull_image_if_missing(&template.image)
            .await?;
        let overrides = self.config_repo.get_all(id).await?;
        let spec = self.build_spec(server, template, &overrides, cfg);
        let cid = self.container_runtime.create_container(&spec).await?;
        self.server_repo
            .update_runtime(
                id,
                GameServerRuntimeUpdate {
                    container_id: Some(cid.clone()),
                    ..Default::default()
                },
            )
            .await?;
        Ok(cid)
    }
}
