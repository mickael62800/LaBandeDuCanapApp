use super::*;

fn t(h: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(1_700_000_000 + h * 3600, 0).unwrap()
}

fn option(label: &str, votes: i64, color: Option<&str>) -> PollOption {
    PollOption {
        id: Uuid::nil(),
        label: label.to_string(),
        color: color.map(str::to_string),
        position: 0,
        votes,
    }
}

fn poll(options: Vec<PollOption>, ferme: bool, clot_a: i64) -> Poll {
    Poll {
        id: Uuid::nil(),
        guild_id: "g".into(),
        question: "Quel jeu ?".into(),
        description: None,
        closes_at: t(clot_a),
        is_closed: ferme,
        is_public: true,
        created_by: "staff".into(),
        created_at: t(0),
        options,
    }
}

#[test]
fn sondage_non_ferme_avant_sa_date_est_ouvert() {
    assert!(poll(vec![], false, 10).is_open(t(5)));
}

#[test]
fn cloture_manuelle_ferme_le_sondage_avant_sa_date() {
    assert!(!poll(vec![], true, 10).is_open(t(5)));
}

/// Sans la date, un sondage oublie resterait ouvert des mois.
#[test]
fn date_depassee_ferme_le_sondage_sans_action_manuelle() {
    assert!(!poll(vec![], false, 10).is_open(t(11)));
}

#[test]
fn total_additionne_les_voix() {
    let p = poll(
        vec![
            option("A", 18, None),
            option("B", 13, None),
            option("C", 9, None),
        ],
        false,
        10,
    );
    assert_eq!(p.total_votes(), 40);
}

#[test]
fn parts_sont_en_pourcentage_entier() {
    let p = poll(vec![option("A", 3, None), option("B", 1, None)], false, 10);
    assert_eq!(p.shares(), vec![75, 25]);
}

/// Un sondage qui vient d'ouvrir est le cas normal, pas une erreur : pas de
/// division par zero.
#[test]
fn sondage_sans_voix_donne_zero_partout() {
    let p = poll(vec![option("A", 0, None), option("B", 0, None)], false, 10);
    assert_eq!(p.shares(), vec![0, 0]);
}

#[test]
fn couleur_choisie_prime_sur_la_palette() {
    let p = poll(vec![option("A", 0, Some("ff0000"))], false, 10);
    assert_eq!(p.color_at(0), "ff0000");
}

#[test]
fn option_sans_couleur_prend_la_palette_selon_sa_position() {
    let p = poll(vec![option("A", 0, None), option("B", 0, None)], false, 10);
    assert_eq!(p.color_at(0), DEFAULT_COLORS[0]);
    assert_eq!(p.color_at(1), DEFAULT_COLORS[1]);
}

/// Plus d'options que de couleurs : la palette boucle au lieu de sortir des
/// bornes.
#[test]
fn palette_boucle_au_dela_de_sa_taille() {
    let options: Vec<_> = (0..8).map(|i| option(&format!("O{i}"), 0, None)).collect();
    let p = poll(options, false, 10);
    assert_eq!(p.color_at(6), DEFAULT_COLORS[0]);
}
