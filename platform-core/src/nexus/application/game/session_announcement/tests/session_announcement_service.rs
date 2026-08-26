use std::collections::HashMap;

use super::*;
use crate::nexus::domain::entities::game::schedule::{ScheduleMode, TimeRange, TOUS_LES_JOURS};

// ── Jauge de joueurs ───────────────────────────────────────────────────────

fn config(paires: &[(&str, &str)]) -> HashMap<String, String> {
    paires
        .iter()
        .map(|(c, v)| ((*c).to_string(), (*v).to_string()))
        .collect()
}

#[test]
fn la_jauge_du_serveur_prime_sur_celle_du_modele() {
    let defauts = serde_json::json!({ "MAX_PLAYERS": "16" });
    assert_eq!(
        jauge_de_joueurs(&config(&[("MAX_PLAYERS", "10")]), &defauts),
        Some(10)
    );
}

#[test]
fn a_defaut_la_jauge_vient_du_modele() {
    let defauts = serde_json::json!({ "MAX_PLAYERS": "16" });
    assert_eq!(jauge_de_joueurs(&config(&[]), &defauts), Some(16));
}

/// La fiche du catalogue ecrit tantot `"16"`, tantot `16` : les deux formes
/// existent dans les migrations, et lire une seule des deux ferait disparaitre
/// la jauge de la moitie des jeux.
#[test]
fn la_jauge_se_lit_en_texte_comme_en_nombre() {
    assert_eq!(
        jauge_de_joueurs(&config(&[]), &serde_json::json!({ "MAX_PLAYERS": 24 })),
        Some(24)
    );
}

/// UNE VALEUR ILLISIBLE VAUT UNE ABSENCE. Annoncer une jauge fausse est pire
/// que de n'en annoncer aucune : les joueurs la constateront.
#[test]
fn une_jauge_illisible_disparait_plutot_que_de_mentir() {
    let defauts = serde_json::json!({ "MAX_PLAYERS": "beaucoup" });
    assert_eq!(jauge_de_joueurs(&config(&[]), &defauts), None);
    assert_eq!(
        jauge_de_joueurs(&config(&[("MAX_PLAYERS", "")]), &serde_json::json!({})),
        None
    );
    assert_eq!(jauge_de_joueurs(&config(&[]), &serde_json::json!({})), None);
}

// ── Ouverture prevue ───────────────────────────────────────────────────────

fn horaire(ranges: Vec<TimeRange>, fuseau: &str) -> AutoSchedule {
    AutoSchedule {
        enabled: true,
        mode: ScheduleMode::Ranges,
        timezone: fuseau.into(),
        ranges,
        warn_minutes: 10,
        opens_at: None,
        closes_at: None,
        restart_interval_hours: None,
        restart_anchor_minute: 0,
        last_restart_at: None,
        last_warned_at: None,
        last_final_warned_at: None,
    }
}

fn plage(h1: u16, h2: u16) -> TimeRange {
    TimeRange {
        start_minute: h1 * 60,
        end_minute: h2 * 60,
        days: TOUS_LES_JOURS,
    }
}

#[test]
fn l_ouverture_est_annoncee_en_francais() {
    let etiquette = etiquette_d_ouverture(&horaire(vec![plage(19, 23)], "Europe/Paris"))
        .expect("une ouverture est prevue chaque jour");

    let jours = [
        "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche",
    ];
    assert!(
        jours.iter().any(|j| etiquette.starts_with(j)),
        "etiquette inattendue : {etiquette}"
    );
    assert!(etiquette.contains(" a 19h"), "etiquette : {etiquette}");
}

/// UN FUSEAU ILLISIBLE NE DOIT PAS PRODUIRE UNE HEURE. Retomber sur UTC
/// annoncerait 19h pour une ouverture a 21h la moitie de l'annee.
#[test]
fn un_fuseau_illisible_n_annonce_aucune_heure() {
    assert!(etiquette_d_ouverture(&horaire(vec![plage(19, 23)], "Pas/UnFuseau")).is_none());
}

#[test]
fn sans_plage_aucune_ouverture_n_est_annoncee() {
    assert!(etiquette_d_ouverture(&horaire(vec![], "Europe/Paris")).is_none());
}

/// Une permanence tourne deja : elle n'a pas d'ouverture a annoncer.
#[test]
fn une_permanence_n_a_pas_d_ouverture() {
    let mut permanence = horaire(vec![plage(19, 23)], "Europe/Paris");
    permanence.mode = ScheduleMode::Restart;
    permanence.restart_interval_hours = Some(6);

    assert!(etiquette_des_plages(&permanence).is_none());
}

#[test]
fn un_horaire_desactive_n_annonce_rien() {
    let mut eteint = horaire(vec![plage(19, 23)], "Europe/Paris");
    eteint.enabled = false;

    assert!(etiquette_d_ouverture(&eteint).is_none());
}
