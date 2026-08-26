use super::*;

fn demande() -> GameAnnouncementRequest {
    GameAnnouncementRequest {
        guild_id: "123456789012345678".into(),
        game_name: "Project Zomboid".into(),
        server_name: "Le Canap sur Zomboid".into(),
        max_players: Some(10),
        opening_label: Some("vendredi 29 aout a 19h".into()),
        schedule_label: Some("vendredi et samedi, 19h-23h".into()),
        admin_context: String::new(),
    }
}

#[test]
fn une_demande_complete_est_valide() {
    assert!(demande().validate().is_ok());
}

#[test]
fn les_champs_indispensables_sont_exiges() {
    let cas: Vec<(Box<dyn Fn(&mut GameAnnouncementRequest)>, &str)> = vec![
        (Box::new(|d| d.guild_id = "  ".into()), "guild_id"),
        (Box::new(|d| d.game_name = String::new()), "game_name"),
        (Box::new(|d| d.server_name = String::new()), "server_name"),
    ];
    for (mutation, attendu) in cas {
        let mut d = demande();
        mutation(&mut d);
        assert_eq!(d.validate(), Err(GameAnnouncementError::Missing(attendu)));
    }
}

#[test]
fn un_contexte_trop_long_est_refuse() {
    let mut d = demande();
    d.admin_context = "a".repeat(MAX_ADMIN_CONTEXT_CHARS + 1);
    assert_eq!(
        d.validate(),
        Err(GameAnnouncementError::TooLong {
            field: "admin_context",
            limit: MAX_ADMIN_CONTEXT_CHARS,
        })
    );
}

#[test]
fn les_faits_reprennent_ce_que_nexus_a_fourni() {
    let faits = demande().faits();
    assert!(faits.contains("Jeu : Project Zomboid"));
    assert!(faits.contains("Nom du serveur : Le Canap sur Zomboid"));
    assert!(faits.contains("Joueurs maximum : 10"));
    assert!(faits.contains("Ouverture : vendredi 29 aout a 19h"));
    assert!(faits.contains("Horaires : vendredi et samedi, 19h-23h"));
}

/// UN FAIT ABSENT NE DOIT LAISSER AUCUNE TRACE. Ecrire « joueurs max :
/// inconnu » inviterait le modele a broder autour du trou — et une annonce qui
/// invente une jauge est pire qu'une annonce qui n'en parle pas.
#[test]
fn un_fait_absent_ne_produit_aucune_ligne() {
    let mut d = demande();
    d.max_players = None;
    d.opening_label = None;
    d.schedule_label = None;

    let faits = d.faits();
    assert!(!faits.to_lowercase().contains("joueurs"));
    assert!(!faits.to_lowercase().contains("ouverture"));
    assert!(!faits.to_lowercase().contains("horaires"));
    assert!(faits.contains("Jeu : Project Zomboid"));
}

/// Une chaine vide venue d'un champ non renseigne ne vaut pas mieux qu'un
/// `None` : elle produirait « Ouverture : » suivi de rien.
#[test]
fn une_etiquette_vide_vaut_une_absence() {
    let mut d = demande();
    d.opening_label = Some("   ".into());
    assert!(!d.faits().contains("Ouverture"));
}
