use super::*;
use chrono::TimeZone;

fn d(y: i32, m: u32, j: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, j, 12, 0, 0).unwrap()
}

#[test]
fn annees_comptent_depuis_l_arrivee() {
    assert_eq!(years_at(d(2023, 2, 4), d(2026, 2, 4)), 3);
}

/// Le cas qui motive le calcul sur l'annee de l'anniversaire : la fenetre
/// glissante deborde sur l'annee suivante. Un 2 janvier consulte le
/// 28 decembre doit annoncer 3 ans, pas 2.
#[test]
fn anniversaire_de_l_annee_suivante_compte_l_annee_a_venir() {
    assert_eq!(years_at(d(2023, 1, 2), d(2026, 1, 2)), 3);
}

#[test]
fn arrivee_dans_le_futur_ne_donne_pas_d_annees_negatives() {
    assert_eq!(years_at(d(2030, 1, 1), d(2026, 1, 1)), 0);
}

#[test]
fn dates_ordinaires_sont_inchangees() {
    assert_eq!(celebrated_day(6, 15, 2026), (6, 15));
    assert_eq!(celebrated_day(2, 28, 2026), (2, 28));
}

/// Sans ce repli, les membres arrives un 29 fevrier n'auraient d'anniversaire
/// qu'une annee sur quatre.
#[test]
fn vingt_neuf_fevrier_est_fete_le_vingt_huit_en_annee_commune() {
    assert_eq!(celebrated_day(2, 29, 2026), (2, 28));
}

#[test]
fn vingt_neuf_fevrier_reste_le_vingt_neuf_en_annee_bissextile() {
    assert_eq!(celebrated_day(2, 29, 2028), (2, 29));
}

/// 1900 n'est pas bissextile, 2000 l'est : la regle des siecles compte.
#[test]
fn regle_des_siecles_est_respectee() {
    assert_eq!(celebrated_day(2, 29, 1900), (2, 28));
    assert_eq!(celebrated_day(2, 29, 2000), (2, 29));
}
