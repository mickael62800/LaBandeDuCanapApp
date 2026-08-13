use super::*;
use chrono::TimeZone;

fn at(y: i32, m: u32, d: u32, h: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, h, 0, 0).unwrap()
}

fn event(start: DateTime<Utc>, end: DateTime<Utc>) -> CommunityEvent {
    CommunityEvent {
        id: Uuid::nil(),
        guild_id: "g".into(),
        title: "Campagne".into(),
        description: None,
        game: None,
        color: None,
        starts_at: start,
        ends_at: end,
        all_day: false,
        is_public: true,
        status: EventStatus::Published,
        created_by: "u".into(),
        created_at: start,
        updated_at: start,
    }
}

#[test]
fn campagne_multi_semaines_visible_dans_chaque_semaine() {
    // Une saison du 1er au 28 fevrier doit apparaitre dans la semaine du 10,
    // alors qu'elle ne commence ni ne finit pendant cette semaine. C'est tout
    // l'interet de raisonner en chevauchement plutot qu'en date de debut.
    let saison = event(at(2026, 2, 1, 18), at(2026, 2, 28, 23));
    assert!(saison.overlaps(at(2026, 2, 9, 0), at(2026, 2, 16, 0)));
    assert_eq!(saison.span_days(), 28);
    assert!(saison.is_multi_day());
}

#[test]
fn hors_fenetre_non_visible() {
    let soiree = event(at(2026, 2, 3, 21), at(2026, 2, 3, 23));
    assert!(!soiree.overlaps(at(2026, 2, 9, 0), at(2026, 2, 16, 0)));
}

#[test]
fn bornes_de_fenetre() {
    // Se termine pile au debut de la fenetre : reste visible ce jour-la.
    let a = event(at(2026, 2, 1, 10), at(2026, 2, 9, 0));
    assert!(a.overlaps(at(2026, 2, 9, 0), at(2026, 2, 16, 0)));

    // Commence pile a la fin de la fenetre : appartient a la suivante.
    let b = event(at(2026, 2, 16, 0), at(2026, 2, 17, 0));
    assert!(!b.overlaps(at(2026, 2, 9, 0), at(2026, 2, 16, 0)));
}

#[test]
fn soiree_compte_pour_un_jour() {
    let soiree = event(at(2026, 2, 3, 21), at(2026, 2, 3, 23));
    assert_eq!(soiree.span_days(), 1);
    assert!(!soiree.is_multi_day());
}

#[test]
fn statut_inconnu_reste_brouillon() {
    // Fail-safe : jamais publie par accident.
    assert_eq!(EventStatus::parse("nawak"), EventStatus::Draft);
    assert_eq!(EventStatus::parse("published"), EventStatus::Published);
}
