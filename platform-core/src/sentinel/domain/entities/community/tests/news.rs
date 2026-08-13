use super::*;

fn t(h: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(1_700_000_000 + h * 3600, 0).unwrap()
}

fn news(body: &str, publie_a: i64) -> NewsPost {
    NewsPost {
        id: Uuid::nil(),
        guild_id: "g".into(),
        title: "Titre".into(),
        body: body.to_string(),
        image_url: None,
        is_pinned: false,
        is_public: true,
        published_at: t(publie_a),
        created_by: "staff".into(),
        created_at: t(0),
    }
}

#[test]
fn nouvelle_datee_du_passe_est_publiee() {
    assert!(news("x", 5).is_published(t(10)));
}

/// Permet de preparer une nouvelle a l'avance.
#[test]
fn nouvelle_datee_du_futur_n_est_pas_encore_publiee() {
    assert!(!news("x", 20).is_published(t(10)));
}

#[test]
fn corps_court_est_rendu_tel_quel_sans_ellipse() {
    let n = news("Le serveur passe en 1.21.", 0);
    assert_eq!(n.excerpt(), "Le serveur passe en 1.21.");
}

#[test]
fn corps_long_est_tronque_avec_une_ellipse() {
    let n = news(&"mot ".repeat(100), 0);
    let e = n.excerpt();
    assert!(e.ends_with('…'));
    assert!(e.chars().count() <= EXCERPT_CHARS + 1);
}

/// Couper sur une frontiere de mot, pas au milieu.
#[test]
fn troncature_ne_coupe_pas_un_mot_en_deux() {
    let n = news(&"abcde ".repeat(60), 0);
    let e = n.excerpt();
    assert!(!e.contains("ab…"), "coupe au milieu d'un mot : {e}");
}

/// Le cas qui ferait paniquer un `&body[..180]` : en francais, les accents
/// occupent deux octets, donc l'indice 180 en octets tombe volontiers au
/// milieu d'un caractere.
#[test]
fn troncature_gere_les_accents_sans_paniquer() {
    let n = news(&"éàçùè ".repeat(80), 0);
    let e = n.excerpt();
    assert!(e.ends_with('…'));
}

/// Un texte sans aucune espace n'offre aucune frontiere de mot : on garde la
/// coupe brute plutot que de renvoyer une chaine vide.
#[test]
fn corps_sans_espace_est_coupe_brut() {
    let n = news(&"a".repeat(400), 0);
    let e = n.excerpt();
    assert_eq!(e.chars().count(), EXCERPT_CHARS + 1);
}

#[test]
fn espaces_de_bord_sont_ignores() {
    assert_eq!(news("   court   ", 0).excerpt(), "court");
}
