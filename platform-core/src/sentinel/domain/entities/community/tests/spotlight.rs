use super::*;

#[test]
fn periode_est_le_mois_de_l_instant_donne() {
    let t = DateTime::from_timestamp(1_700_000_000, 0).unwrap(); // 14 nov. 2023
    assert_eq!(period_of(t), "2023-11");
}

/// Le zero de tete compte : « 2024-3 » ne trierait pas correctement et
/// violerait la contrainte en base.
#[test]
fn mois_a_un_chiffre_est_complete_par_un_zero() {
    let t = DateTime::from_timestamp(1_709_000_000, 0).unwrap(); // fevrier 2024
    assert_eq!(period_of(t), "2024-02");
}

#[test]
fn periodes_bien_formees_sont_acceptees() {
    assert!(is_valid_period("2026-01"));
    assert!(is_valid_period("2026-12"));
}

#[test]
fn periodes_mal_formees_sont_refusees() {
    assert!(!is_valid_period("2026-1"), "mois sur un chiffre");
    assert!(!is_valid_period("2026/01"), "mauvais separateur");
    assert!(!is_valid_period("26-01"), "annee sur deux chiffres");
    assert!(!is_valid_period(""), "vide");
    assert!(!is_valid_period("aaaa-bb"), "non numerique");
}

/// Le format serait valide mais le mois n'existe pas : la verification doit
/// aller au-dela de la forme.
#[test]
fn mois_hors_bornes_est_refuse() {
    assert!(!is_valid_period("2026-00"));
    assert!(!is_valid_period("2026-13"));
}

/// La periode arrive parfois d'une requete HTTP : elle ne doit jamais faire
/// paniquer le decoupage sur des octets multi-octets.
#[test]
fn periode_non_ascii_est_refusee_sans_paniquer() {
    assert!(!is_valid_period("20é6-01"));
}
