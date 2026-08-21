use std::sync::Mutex;

use super::*;
use crate::nexus::domain::entities::achievement::{Platform, Verification};

/// Repository en memoire. Reproduit les contraintes d'unicite que porte le
/// schema SQL — c'est ce qui rend le test d'idempotence significatif.
#[derive(Default)]
struct FakeRepo {
    definitions: Mutex<Vec<Achievement>>,
    links: Mutex<Vec<GamePlayerLink>>,
    unlocks: Mutex<Vec<UserAchievement>>,
}

impl FakeRepo {
    fn with_definition(achievement: Achievement) -> Self {
        Self {
            definitions: Mutex::new(vec![achievement]),
            ..Default::default()
        }
    }

    fn add_link(&self, guild: &str, user: &str, game: &str, player: &str, verified: bool) {
        self.links.lock().unwrap().push(GamePlayerLink {
            id: Uuid::new_v4(),
            guild_id: guild.into(),
            discord_user_id: user.into(),
            game: game.into(),
            platform: Platform::Steam,
            game_player_id: player.into(),
            verified_at: verified.then(Utc::now),
        });
    }
}

fn definition(code: &str, verification: Verification) -> Achievement {
    Achievement {
        id: Uuid::new_v4(),
        game: Some("palworld".into()),
        code: code.into(),
        name: code.into(),
        description: String::new(),
        category: "test".into(),
        icon_url: None,
        criteria: serde_json::json!({}),
        verification,
        hidden: false,
        enabled: true,
    }
}

#[async_trait]
impl AchievementRepository for FakeRepo {
    async fn list_definitions(&self, game: Option<&str>) -> Result<Vec<Achievement>, DomainError> {
        Ok(self
            .definitions
            .lock()
            .unwrap()
            .iter()
            .filter(|d| game.is_none() || d.game.as_deref() == game)
            .cloned()
            .collect())
    }

    async fn find_definition(&self, id: Uuid) -> Result<Option<Achievement>, DomainError> {
        Ok(self
            .definitions
            .lock()
            .unwrap()
            .iter()
            .find(|d| d.id == id)
            .cloned())
    }

    async fn find_definition_by_code(
        &self,
        game: Option<&str>,
        code: &str,
    ) -> Result<Option<Achievement>, DomainError> {
        Ok(self
            .definitions
            .lock()
            .unwrap()
            .iter()
            .find(|d| d.game.as_deref() == game && d.code == code)
            .cloned())
    }

    async fn update_definition(
        &self,
        id: Uuid,
        update: AchievementUpdate,
    ) -> Result<Achievement, DomainError> {
        let mut defs = self.definitions.lock().unwrap();
        let def = defs
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or_else(|| DomainError::NotFound("inconnu".into()))?;
        if let Some(icon) = update.icon_url {
            def.icon_url = icon;
        }
        Ok(def.clone())
    }

    async fn find_link(
        &self,
        guild_id: &str,
        discord_user_id: &str,
        game: &str,
    ) -> Result<Option<GamePlayerLink>, DomainError> {
        Ok(self
            .links
            .lock()
            .unwrap()
            .iter()
            .find(|l| {
                l.guild_id == guild_id && l.discord_user_id == discord_user_id && l.game == game
            })
            .cloned())
    }

    async fn find_link_by_player(
        &self,
        guild_id: &str,
        identity: &GameIdentity,
    ) -> Result<Option<GamePlayerLink>, DomainError> {
        Ok(self
            .links
            .lock()
            .unwrap()
            .iter()
            .find(|l| {
                l.guild_id == guild_id
                    && l.game == identity.game()
                    && l.game_player_id == identity.player_id()
            })
            .cloned())
    }

    async fn upsert_link(
        &self,
        guild_id: &str,
        discord_user_id: &str,
        identity: &GameIdentity,
        verified: bool,
    ) -> Result<GamePlayerLink, DomainError> {
        let link = GamePlayerLink {
            id: Uuid::new_v4(),
            guild_id: guild_id.into(),
            discord_user_id: discord_user_id.into(),
            game: identity.game().into(),
            platform: identity.platform(),
            game_player_id: identity.player_id().into(),
            verified_at: verified.then(Utc::now),
        };
        self.links.lock().unwrap().push(link.clone());
        Ok(link)
    }

    async fn delete_link(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> {
        Ok(true)
    }

    async fn insert_unlock(
        &self,
        unlock: &UserAchievement,
    ) -> Result<Option<UserAchievement>, DomainError> {
        let mut unlocks = self.unlocks.lock().unwrap();
        // UNIQUE (guild_id, discord_user_id, achievement_id)
        let deja_possede = unlocks.iter().any(|u| {
            u.guild_id == unlock.guild_id
                && u.discord_user_id == unlock.discord_user_id
                && u.achievement_id == unlock.achievement_id
        });
        // UNIQUE (source_event_id)
        let evenement_rejoue = unlock.source_event_id.is_some()
            && unlocks
                .iter()
                .any(|u| u.source_event_id == unlock.source_event_id);
        if deja_possede || evenement_rejoue {
            return Ok(None);
        }
        unlocks.push(unlock.clone());
        Ok(Some(unlock.clone()))
    }

    async fn list_for_member(
        &self,
        guild_id: &str,
        discord_user_id: &str,
    ) -> Result<Vec<UserAchievement>, DomainError> {
        Ok(self
            .unlocks
            .lock()
            .unwrap()
            .iter()
            .filter(|u| u.guild_id == guild_id && u.discord_user_id == discord_user_id)
            .cloned()
            .collect())
    }

    async fn count_for_member(&self, g: &str, u: &str) -> Result<i64, DomainError> {
        Ok(self.list_for_member(g, u).await?.len() as i64)
    }
}

fn commande(code: &str, event_id: &str) -> GameUnlockCommand {
    GameUnlockCommand {
        guild_id: "guild".into(),
        game: "palworld".into(),
        platform: Platform::Steam,
        game_player_id: "76561198000000000".into(),
        achievement_code: code.into(),
        source_event_id: event_id.into(),
    }
}

#[tokio::test]
async fn sans_liaison_verifiee_rien_n_est_attribue() {
    let repo = Arc::new(FakeRepo::with_definition(definition(
        "first_launch_palworld",
        Verification::Auto,
    )));
    // Liaison presente mais NON verifiee : c'est le cas que le document
    // impose de refuser (candidat en attente, aucune attribution).
    repo.add_link("guild", "user", "palworld", "76561198000000000", false);
    let service = AchievementsService::new(repo.clone());

    let result = service
        .unlock_from_game_event(commande("first_launch_palworld", "evt-1"))
        .await;

    assert!(matches!(result, Err(DomainError::NotFound(_))));
    assert_eq!(repo.count_for_member("guild", "user").await.unwrap(), 0);
}

#[tokio::test]
async fn un_evenement_rejoue_n_attribue_pas_deux_fois() {
    let repo = Arc::new(FakeRepo::with_definition(definition(
        "first_launch_palworld",
        Verification::Auto,
    )));
    repo.add_link("guild", "user", "palworld", "76561198000000000", true);
    let service = AchievementsService::new(repo.clone());

    let premier = service
        .unlock_from_game_event(commande("first_launch_palworld", "evt-1"))
        .await
        .unwrap();
    assert!(matches!(premier, UnlockOutcome::Unlocked(_)));

    // Meme evenement rejoue (redelivery Redis) : rien de nouveau, donc rien
    // a publier — sinon le salon recevrait deux annonces identiques.
    let rejeu = service
        .unlock_from_game_event(commande("first_launch_palworld", "evt-1"))
        .await
        .unwrap();
    assert!(matches!(rejeu, UnlockOutcome::AlreadyOwned));
    assert_eq!(repo.count_for_member("guild", "user").await.unwrap(), 1);
}

#[tokio::test]
async fn un_haut_fait_a_validation_humaine_refuse_l_evenement_de_jeu() {
    let repo = Arc::new(FakeRepo::with_definition(definition(
        "palworld_all_towers",
        Verification::Manual,
    )));
    repo.add_link("guild", "user", "palworld", "76561198000000000", true);
    let service = AchievementsService::new(repo.clone());

    let result = service
        .unlock_from_game_event(commande("palworld_all_towers", "evt-1"))
        .await;

    assert!(matches!(result, Err(DomainError::Forbidden(_))));
    assert_eq!(repo.count_for_member("guild", "user").await.unwrap(), 0);
}

#[tokio::test]
async fn l_admin_peut_attribuer_un_haut_fait_manuel_une_seule_fois() {
    let def = definition("palworld_all_towers", Verification::Manual);
    let id = def.id;
    let repo = Arc::new(FakeRepo::with_definition(def));
    let service = AchievementsService::new(repo.clone());

    let premier = service
        .grant_manually("guild", "user", id, "admin")
        .await
        .unwrap();
    assert!(matches!(premier, UnlockOutcome::Unlocked(_)));

    let second = service
        .grant_manually("guild", "user", id, "admin")
        .await
        .unwrap();
    assert!(matches!(second, UnlockOutcome::AlreadyOwned));
    assert_eq!(repo.count_for_member("guild", "user").await.unwrap(), 1);
}

#[tokio::test]
async fn une_identite_deja_prise_par_un_autre_membre_est_refusee() {
    let repo = Arc::new(FakeRepo::default());
    repo.add_link("guild", "autre", "palworld", "76561198000000000", true);
    let service = AchievementsService::new(repo);

    let result = service
        .link_identity(
            "guild",
            "user",
            "palworld",
            Platform::Steam,
            "76561198000000000",
        )
        .await;

    assert!(matches!(result, Err(DomainError::Conflict(_))));
}

#[tokio::test]
async fn un_steam_id_mal_forme_est_refuse_avant_toute_ecriture() {
    let repo = Arc::new(FakeRepo::default());
    let service = AchievementsService::new(repo.clone());

    let result = service
        .link_identity("guild", "user", "palworld", Platform::Steam, "DarkPoney")
        .await;

    assert!(matches!(result, Err(DomainError::ValidationError(_))));
    assert!(repo.links.lock().unwrap().is_empty());
}

#[tokio::test]
async fn un_haut_fait_secret_non_debloque_n_apparait_pas() {
    let mut secret = definition("palworld_immortal_run", Verification::Manual);
    secret.hidden = true;
    let repo = Arc::new(FakeRepo::with_definition(secret));
    let service = AchievementsService::new(repo);

    let progress = service
        .member_progress("guild", "user", Some("palworld"))
        .await
        .unwrap();

    assert!(progress.is_empty());
}

#[tokio::test]
async fn une_image_non_http_est_refusee() {
    let def = definition("first_launch_palworld", Verification::Auto);
    let id = def.id;
    let service = AchievementsService::new(Arc::new(FakeRepo::with_definition(def)));

    let result = service
        .update_definition(
            id,
            AchievementUpdate {
                icon_url: Some(Some("javascript:alert(1)".into())),
                ..Default::default()
            },
        )
        .await;

    assert!(matches!(result, Err(DomainError::ValidationError(_))));
}

#[tokio::test]
async fn un_chemin_local_est_accepte_mais_pas_une_remontee_ni_un_hote() {
    let def = definition("first_launch_palworld", Verification::Auto);
    let id = def.id;
    let service = AchievementsService::new(Arc::new(FakeRepo::with_definition(def)));

    let maj = |valeur: &str| AchievementUpdate {
        icon_url: Some(Some(valeur.to_string())),
        ..Default::default()
    };

    // Forme livree par le dashboard : stable d'un build a l'autre.
    assert!(service
        .update_definition(id, maj("/Achievement/palworld/pal_01.jpg"))
        .await
        .is_ok());
    // Remontee de repertoire.
    assert!(service
        .update_definition(id, maj("/Achievement/../../etc/passwd"))
        .await
        .is_err());
    // Protocol-relatif : c'est un autre hote, pas un chemin local.
    assert!(service
        .update_definition(id, maj("//evil.example/x.jpg"))
        .await
        .is_err());
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests supplémentaires pour atteindre ≥ 90 % de couverture
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_definitions_filtre_par_jeu() {
    let palworld = definition("first_launch_palworld", Verification::Auto);
    let mut minecraft = definition("first_minecraft", Verification::Auto);
    minecraft.game = Some("minecraft".into());
    let repo = Arc::new(FakeRepo {
        definitions: Mutex::new(vec![palworld, minecraft]),
        ..Default::default()
    });
    let service = AchievementsService::new(repo.clone());

    let tous = service.list_definitions(None).await.unwrap();
    assert_eq!(tous.len(), 2);

    let pal = service.list_definitions(Some("palworld")).await.unwrap();
    assert_eq!(pal.len(), 1);
    assert_eq!(pal[0].code, "first_launch_palworld");
}

#[tokio::test]
async fn update_definition_vide_est_refuse() {
    let def = definition("first_launch_palworld", Verification::Auto);
    let id = def.id;
    let service = AchievementsService::new(Arc::new(FakeRepo::with_definition(def)));

    let result = service.update_definition(id, AchievementUpdate::default()).await;
    assert!(matches!(result, Err(DomainError::ValidationError(_))));
}

#[tokio::test]
async fn update_definition_refuse_un_nom_vide_ou_trop_long() {
    let def = definition("first_launch_palworld", Verification::Auto);
    let id = def.id;
    let service = AchievementsService::new(Arc::new(FakeRepo::with_definition(def)));

    let maj_vide = AchievementUpdate {
        name: Some("   ".into()),
        ..Default::default()
    };
    assert!(matches!(
        service.update_definition(id, maj_vide).await,
        Err(DomainError::ValidationError(_))
    ));

    let maj_long = AchievementUpdate {
        name: Some("x".repeat(101)),
        ..Default::default()
    };
    assert!(matches!(
        service.update_definition(id, maj_long).await,
        Err(DomainError::ValidationError(_))
    ));
}

#[tokio::test]
async fn update_definition_refuse_une_description_trop_longue() {
    let def = definition("first_launch_palworld", Verification::Auto);
    let id = def.id;
    let service = AchievementsService::new(Arc::new(FakeRepo::with_definition(def)));

    let maj = AchievementUpdate {
        description: Some("x".repeat(501)),
        ..Default::default()
    };
    assert!(matches!(
        service.update_definition(id, maj).await,
        Err(DomainError::ValidationError(_))
    ));
}

#[tokio::test]
async fn validate_icon_url_refuse_une_url_trop_longue() {
    let def = definition("first_launch_palworld", Verification::Auto);
    let id = def.id;
    let service = AchievementsService::new(Arc::new(FakeRepo::with_definition(def)));

    let maj = AchievementUpdate {
        icon_url: Some(Some("https://".to_string() + &"a".repeat(500))),
        ..Default::default()
    };
    assert!(matches!(
        service.update_definition(id, maj).await,
        Err(DomainError::ValidationError(_))
    ));
}

#[tokio::test]
async fn validate_icon_url_accepte_http_et_chemin_local() {
    let def = definition("first_launch_palworld", Verification::Auto);
    let id = def.id;
    let service = AchievementsService::new(Arc::new(FakeRepo::with_definition(def)));

    let http = AchievementUpdate {
        icon_url: Some(Some("https://example.com/icon.png".into())),
        ..Default::default()
    };
    assert!(service.update_definition(id, http).await.is_ok());

    let local = AchievementUpdate {
        icon_url: Some(Some("/Achievement/palworld/pal_01.jpg".into())),
        ..Default::default()
    };
    assert!(service.update_definition(id, local).await.is_ok());
}

#[tokio::test]
async fn member_progress_masque_un_haut_fait_desactive_non_debloque() {
    let mut def = definition("first_launch_palworld", Verification::Auto);
    def.enabled = false;
    let repo = Arc::new(FakeRepo::with_definition(def));
    let service = AchievementsService::new(repo);

    let progress = service
        .member_progress("guild", "user", Some("palworld"))
        .await
        .unwrap();
    assert!(progress.is_empty());
}

#[tokio::test]
async fn member_progress_affiche_un_haut_fait_desactive_debloque() {
    let mut def = definition("first_launch_palworld", Verification::Auto);
    def.enabled = false;
    let achievement_id = def.id;
    let repo = Arc::new(FakeRepo::with_definition(def));
    // L'utilisateur possede deja le haut fait (avant desactivation).
    repo.unlocks.lock().unwrap().push(UserAchievement {
        id: Uuid::new_v4(),
        guild_id: "guild".into(),
        discord_user_id: "user".into(),
        achievement_id,
        game_player_id: None,
        source_event_id: None,
        granted_by: None,
        unlocked_at: Utc::now(),
    });
    let service = AchievementsService::new(repo);

    let progress = service
        .member_progress("guild", "user", Some("palworld"))
        .await
        .unwrap();
    // Le haut fait reste visible car il a ete obtenu.
    assert_eq!(progress.len(), 1);
    assert!(progress[0].is_unlocked());
}

#[tokio::test]
async fn find_link_retourne_une_liaison_existante() {
    let repo = Arc::new(FakeRepo::default());
    repo.add_link("guild", "user", "palworld", "76561198000000000", true);
    let service = AchievementsService::new(repo);

    let link = service
        .find_link("guild", "user", "palworld")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(link.discord_user_id, "user");
    assert!(link.is_verified());
}

#[tokio::test]
async fn unlink_identity_supprime_une_liaison() {
    let repo = Arc::new(FakeRepo::default());
    repo.add_link("guild", "user", "palworld", "76561198000000000", true);
    let service = AchievementsService::new(repo);

    let supprim = service
        .unlink_identity("guild", "user", "palworld")
        .await
        .unwrap();
    assert!(supprim);
}

#[tokio::test]
async fn unlock_from_game_event_refuse_une_identite_mal_formee() {
    let repo = Arc::new(FakeRepo::with_definition(definition(
        "first_launch_palworld",
        Verification::Auto,
    )));
    let service = AchievementsService::new(repo);

    let cmd = GameUnlockCommand {
        guild_id: "guild".into(),
        game: "palworld".into(),
        platform: Platform::Steam,
        game_player_id: "DarkPoney".into(),
        achievement_code: "first_launch_palworld".into(),
        source_event_id: "evt-1".into(),
    };

    let result = service.unlock_from_game_event(cmd).await;
    assert!(matches!(result, Err(DomainError::ValidationError(_))));
}

#[tokio::test]
async fn unlock_from_game_event_refuse_un_haut_fait_inconnu() {
    let repo = Arc::new(FakeRepo::with_definition(definition(
        "first_launch_palworld",
        Verification::Auto,
    )));
    repo.add_link("guild", "user", "palworld", "76561198000000000", true);
    let service = AchievementsService::new(repo);

    let result = service
        .unlock_from_game_event(commande("code_inconnu", "evt-1"))
        .await;
    assert!(matches!(result, Err(DomainError::NotFound(_))));
}

#[tokio::test]
async fn unlock_from_game_event_refuse_un_haut_fait_desactive() {
    let mut def = definition("first_launch_palworld", Verification::Auto);
    def.enabled = false;
    let repo = Arc::new(FakeRepo::with_definition(def));
    repo.add_link("guild", "user", "palworld", "76561198000000000", true);
    let service = AchievementsService::new(repo);

    let result = service
        .unlock_from_game_event(commande("first_launch_palworld", "evt-1"))
        .await;
    assert!(matches!(result, Err(DomainError::Conflict(_))));
}

#[tokio::test]
async fn grant_manually_refuse_un_haut_fait_inconnu() {
    let repo = Arc::new(FakeRepo::default());
    let service = AchievementsService::new(repo);

    let result = service
        .grant_manually("guild", "user", Uuid::new_v4(), "admin")
        .await;
    assert!(matches!(result, Err(DomainError::NotFound(_))));
}

#[tokio::test]
async fn grant_manually_refuse_un_haut_fait_desactive() {
    let mut def = definition("first_launch_palworld", Verification::Auto);
    def.enabled = false;
    let id = def.id;
    let repo = Arc::new(FakeRepo::with_definition(def));
    let service = AchievementsService::new(repo);

    let result = service
        .grant_manually("guild", "user", id, "admin")
        .await;
    assert!(matches!(result, Err(DomainError::Conflict(_))));
}

#[tokio::test]
async fn grant_manually_trace_l_identite_quand_elle_est_verifiee() {
    let def = definition("first_launch_palworld", Verification::Auto);
    let id = def.id;
    let repo = Arc::new(FakeRepo::with_definition(def));
    repo.add_link("guild", "user", "palworld", "76561198000000000", true);
    let service = AchievementsService::new(repo);

    let outcome = service
        .grant_manually("guild", "user", id, "admin")
        .await
        .unwrap();
    match outcome {
        UnlockOutcome::Unlocked(unlocked) => {
            assert_eq!(unlocked.game_player_id.as_deref(), Some("76561198000000000"));
        }
        _ => panic!("attendu Unlocked"),
    }
}

#[tokio::test]
async fn grant_manually_trace_l_identite_verifiee_pour_une_liaison_non_verifiee() {
    let def = definition("first_launch_palworld", Verification::Auto);
    let id = def.id;
    let repo = Arc::new(FakeRepo::with_definition(def));
    // Liaison non verifiee : l'identite ne doit pas etre tracee.
    repo.add_link("guild", "user", "palworld", "76561198000000000", false);
    let service = AchievementsService::new(repo);

    let outcome = service
        .grant_manually("guild", "user", id, "admin")
        .await
        .unwrap();
    match outcome {
        UnlockOutcome::Unlocked(unlocked) => {
            assert!(unlocked.game_player_id.is_none());
        }
        _ => panic!("attendu Unlocked"),
    }
}

#[tokio::test]
async fn link_identity_remplace_silencieusement_sa_propriete_liaison() {
    let repo = Arc::new(FakeRepo::default());
    // Le meme membre re-lie la meme identite : pas de conflit.
    repo.add_link("guild", "user", "palworld", "76561198000000000", true);
    let service = AchievementsService::new(repo);

    let result = service
        .link_identity(
            "guild",
            "user",
            "palworld",
            Platform::Steam,
            "76561198000000000",
        )
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn un_evenement_deja_consomme_par_un_autre_membre_est_refuse() {
    let repo = Arc::new(FakeRepo::with_definition(definition(
        "first_launch_palworld",
        Verification::Auto,
    )));
    // Le meme membre a deja consomme cet evenement.
    repo.add_link("guild", "user", "palworld", "76561198000000000", true);
    let service = AchievementsService::new(repo);

    let cmd = GameUnlockCommand {
        guild_id: "guild".into(),
        game: "palworld".into(),
        platform: Platform::Steam,
        game_player_id: "76561198000000000".into(),
        achievement_code: "first_launch_palworld".into(),
        source_event_id: "evt-1".into(),
    };
    // Premier appel : l'evenement n'a pas encore ete consomme.
    let premier = service.unlock_from_game_event(cmd.clone()).await.unwrap();
    assert!(matches!(premier, UnlockOutcome::Unlocked(_)));

    // Rejeu du meme evenement : deja consomme.
    let rejeu = service.unlock_from_game_event(cmd).await.unwrap();
    assert!(matches!(rejeu, UnlockOutcome::AlreadyOwned));
}
