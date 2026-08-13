use super::*;

fn t(h: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(1_700_000_000 + h * 3600, 0).unwrap()
}

fn interest(nom: &str) -> LfgInterest {
    LfgInterest {
        user_id: nom.to_string(),
        username: nom.to_string(),
        joined_at: t(0),
    }
}

fn post(slots: i32, interesses: usize, ouvert: bool, expire_a: i64) -> LfgPost {
    LfgPost {
        id: Uuid::nil(),
        guild_id: "g".into(),
        author_id: "a".into(),
        author_name: "Kalyx".into(),
        game: "Valheim".into(),
        game_server_id: None,
        slots,
        when_text: "ce soir 21h".into(),
        description: None,
        is_open: ouvert,
        expires_at: t(expire_a),
        created_at: t(0),
        interested: (0..interesses)
            .map(|i| interest(&format!("u{i}")))
            .collect(),
    }
}

#[test]
fn annonce_ouverte_et_non_expiree_est_visible() {
    assert!(post(2, 0, true, 10).is_live(t(5)));
}

#[test]
fn annonce_fermee_n_est_plus_visible() {
    assert!(!post(2, 0, false, 10).is_live(t(5)));
}

/// Le point du champ `expires_at` : une annonce que personne n'a fermee
/// disparait quand meme.
#[test]
fn annonce_expiree_n_est_plus_visible_meme_si_ouverte() {
    let p = post(2, 0, true, 10);
    assert!(p.is_open);
    assert!(!p.is_live(t(11)));
}

#[test]
fn expiration_pile_a_l_heure_compte_comme_expiree() {
    assert!(post(2, 0, true, 10).is_expired(t(10)));
}

#[test]
fn places_restantes_decroissent_avec_les_interesses() {
    assert_eq!(post(4, 1, true, 10).remaining_slots(), 3);
}

/// Trois reponses a une annonce qui en cherchait deux : on affiche « complet »,
/// jamais « -1 place ».
#[test]
fn places_restantes_ne_passent_pas_sous_zero() {
    let p = post(2, 3, true, 10);
    assert_eq!(p.remaining_slots(), 0);
    assert!(p.is_full());
}

#[test]
fn presence_d_un_interesse_est_detectee() {
    let p = post(2, 2, true, 10);
    assert!(p.has_interest_from("u0"));
    assert!(!p.has_interest_from("inconnu"));
}

fn cmd(expires_at: Option<DateTime<Utc>>) -> UpsertLfgCommand {
    UpsertLfgCommand {
        guild_id: "g".into(),
        author_id: "a".into(),
        author_name: "Kalyx".into(),
        game: "Valheim".into(),
        game_server_id: None,
        slots: 2,
        when_text: "ce soir".into(),
        description: None,
        expires_at,
    }
}

#[test]
fn expiration_absente_retombe_sur_la_duree_par_defaut() {
    let now = t(0);
    assert_eq!(
        cmd(None).resolved_expiry(now),
        now + Duration::hours(DEFAULT_LIFETIME_HOURS)
    );
}

#[test]
fn expiration_explicite_est_conservee() {
    let now = t(0);
    let voulu = now + Duration::hours(6);
    assert_eq!(cmd(Some(voulu)).resolved_expiry(now), voulu);
}

/// Sinon une annonce pourrait squatter la page un an.
#[test]
fn expiration_trop_lointaine_est_plafonnee() {
    let now = t(0);
    let voulu = now + Duration::days(365);
    assert_eq!(
        cmd(Some(voulu)).resolved_expiry(now),
        now + Duration::hours(MAX_LIFETIME_HOURS)
    );
}

/// Une annonce morte-nee ne sert personne : on la ramene a la duree par
/// defaut plutot que de refuser la creation.
#[test]
fn expiration_deja_passee_retombe_sur_la_duree_par_defaut() {
    let now = t(10);
    let passe = t(1);
    assert_eq!(
        cmd(Some(passe)).resolved_expiry(now),
        now + Duration::hours(DEFAULT_LIFETIME_HOURS)
    );
}
