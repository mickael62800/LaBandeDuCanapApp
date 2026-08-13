use super::*;

const GUILDE: &str = "1509157282636890265";

#[test]
fn extrait_un_identifiant_de_chemin() {
    assert_eq!(
        guild_id_from_path(&format!("/api/public/news/{GUILDE}")),
        Some(GUILDE.to_string())
    );
}

#[test]
fn extrait_meme_au_milieu_du_chemin() {
    assert_eq!(
        guild_id_from_path(&format!("/api/levels/{GUILDE}/leaderboard")),
        Some(GUILDE.to_string())
    );
}

#[test]
fn chemin_sans_identifiant_ne_donne_rien() {
    assert_eq!(guild_id_from_path("/api/guilds"), None);
    assert_eq!(guild_id_from_path("/health"), None);
    assert_eq!(guild_id_from_path("/"), None);
}

/// Un uuid ne doit pas etre pris pour un identifiant de serveur : le
/// confondre ferait refuser une route de detail parfaitement legitime.
#[test]
fn un_uuid_n_est_pas_pris_pour_une_guilde() {
    assert_eq!(
        guild_id_from_path("/api/news/detail/3f2504e0-4f89-11d3-9a0c-0305e82c3301"),
        None
    );
}

/// Ici un faux positif provoque un REFUS : on prefere ignorer un nombre
/// trop court plutot que bloquer une route valide.
#[test]
fn un_nombre_trop_court_est_ignore() {
    assert_eq!(guild_id_from_path("/api/events/detail/42"), None);
    assert_eq!(guild_id_from_path("/api/levels/12345/leaderboard"), None);
}

#[test]
fn un_nombre_trop_long_est_ignore() {
    let trop_long = "1".repeat(21);
    assert_eq!(guild_id_from_path(&format!("/api/x/{trop_long}")), None);
}

#[test]
fn un_segment_alphanumerique_est_ignore() {
    assert_eq!(
        guild_id_from_path("/api/x/1509157282636890a65"),
        None,
        "un segment contenant une lettre n'est pas un snowflake"
    );
}

/// Un channel_id derriere `by-channel` NE doit PAS etre pris pour un guild :
/// sinon toutes les mutations vocales (`/by-channel/{id}/purge`, `.../bans`...)
/// etaient refusees a tort (403).
#[test]
fn un_id_derriere_un_marqueur_d_entite_est_ignore() {
    let chan = "1534683694302756935";
    let user = "1534683450097799390";
    assert_eq!(
        guild_id_from_path(&format!("/api/voice-channels/by-channel/{chan}/purge")),
        None
    );
    assert_eq!(
        guild_id_from_path(&format!(
            "/api/voice-channels/by-channel/{chan}/bans/{user}"
        )),
        None
    );
    assert_eq!(
        guild_id_from_path(&format!("/api/confessions/by-message-id/{chan}")),
        None
    );
}

/// Le premier segment plausible gagne : les routes reelles portent le
/// `guild_id` avant tout autre identifiant numerique.
#[test]
fn le_premier_segment_plausible_est_retenu() {
    let autre = "9999999999999999999";
    assert_eq!(
        guild_id_from_path(&format!("/api/x/{GUILDE}/y/{autre}")),
        Some(GUILDE.to_string())
    );
}

#[test]
fn guild_id_json_identique_est_accepte() {
    let body = serde_json::json!({ "guild_id": GUILDE, "value": 1 });
    assert_eq!(foreign_guild_id(&body, GUILDE), None);
}

#[test]
fn guild_id_json_different_est_refuse() {
    let autre = "9999999999999999999";
    let body = serde_json::json!({ "guild_id": autre });
    assert_eq!(foreign_guild_id(&body, GUILDE), Some(autre.to_string()));
}

#[test]
fn guild_id_json_imbrique_est_refuse() {
    let autre = "9999999999999999999";
    let body = serde_json::json!({ "items": [{ "config": { "guild_id": autre } }] });
    assert_eq!(foreign_guild_id(&body, GUILDE), Some(autre.to_string()));
}

#[test]
fn un_autre_identifiant_json_n_est_pas_confondu() {
    let body = serde_json::json!({ "user_id": "9999999999999999999" });
    assert_eq!(foreign_guild_id(&body, GUILDE), None);
}
