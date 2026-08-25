use super::*;

const JEU: &str = "Project Zomboid";
const SERVEUR: &str = "Le Canap sur Zomboid";

fn ecrit(libre: Option<&str>, guilde: Option<&str>) -> String {
    nom_de_salon(
        libre,
        guilde,
        MODELE_INSCRIPTION_PAR_DEFAUT,
        JEU,
        SERVEUR,
        TypeDeSalon::Ecrit,
    )
}

fn vocal(libre: Option<&str>, guilde: Option<&str>) -> String {
    nom_de_salon(
        libre,
        guilde,
        MODELE_VOCAL_PAR_DEFAUT,
        JEU,
        SERVEUR,
        TypeDeSalon::Vocal,
    )
}

#[test]
fn sans_rien_de_configure_le_comportement_historique_est_conserve() {
    assert_eq!(ecrit(None, None), "inscription-project-zomboid");
    assert_eq!(vocal(None, None), "Vocal Le Canap sur Zomboid");
}

#[test]
fn le_modele_de_guilde_remplace_le_defaut() {
    assert_eq!(ecrit(None, Some("jeu-{jeu}")), "jeu-project-zomboid");
    assert_eq!(vocal(None, Some("🎙 {serveur}")), "🎙 Le Canap sur Zomboid");
}

#[test]
fn le_nom_libre_du_serveur_prime_sur_le_modele_de_guilde() {
    assert_eq!(
        ecrit(Some("le-camp-de-base"), Some("jeu-{jeu}")),
        "le-camp-de-base"
    );
    assert_eq!(vocal(Some("La Taverne"), Some("🎙 {serveur}")), "La Taverne");
}

/// Un champ laisse vide dans le formulaire arrive comme une chaine vide, pas
/// comme `None`. Le traiter comme un nom voulu creerait un salon sans nom, que
/// Discord refuse — et la session naitrait a moitie.
#[test]
fn une_source_vide_est_ignoree_au_profit_de_la_suivante() {
    assert_eq!(ecrit(Some(""), Some("jeu-{jeu}")), "jeu-project-zomboid");
    assert_eq!(ecrit(Some("   "), None), "inscription-project-zomboid");
    assert_eq!(
        ecrit(Some("-- --"), Some("jeu-{jeu}")),
        "jeu-project-zomboid"
    );
}

/// Meme sans aucune source exploitable, on ne renvoie jamais rien : c'est la
/// derniere barriere avant un appel Discord voue a l'echec.
#[test]
fn un_nom_vide_n_est_jamais_renvoye() {
    let sans_jeu = nom_de_salon(Some(""), Some(""), "{jeu}", "", "", TypeDeSalon::Ecrit);
    assert_eq!(sans_jeu, "salon-de-jeu");

    let sans_jeu_vocal = nom_de_salon(Some(""), Some(""), "{jeu}", "", "", TypeDeSalon::Vocal);
    assert_eq!(sans_jeu_vocal, "Vocal");
}

/// Discord met de force un salon ecrit en minuscules et remplace les espaces.
/// Si on ne le faisait pas nous-memes, le nom enregistre differerait du nom
/// affiche, et tout nettoyage comparant des noms echouerait en silence.
#[test]
fn un_salon_ecrit_est_ramene_a_la_forme_que_discord_imposera() {
    assert_eq!(ecrit(Some("Le Camp De Base"), None), "le-camp-de-base");
    assert_eq!(
        ecrit(Some("Salon   des   Joueurs"), None),
        "salon-des-joueurs"
    );
    assert_eq!(ecrit(Some("C'est parti !"), None), "cest-parti");
}

#[test]
fn le_vocal_garde_majuscules_espaces_et_emoji() {
    assert_eq!(vocal(Some("🎙 La Taverne"), None), "🎙 La Taverne");
}

/// Discord compte des caracteres, pas des octets. Couper au milieu d'un point
/// de code produirait une chaine invalide.
#[test]
fn un_nom_trop_long_est_tronque_sur_une_frontiere_de_caractere() {
    let long = "é".repeat(150);
    let obtenu = vocal(Some(&long), None);

    assert_eq!(obtenu.chars().count(), LONGUEUR_MAX);
    assert!(obtenu.chars().all(|c| c == 'é'));
}

#[test]
fn un_modele_sans_repere_est_un_nom_fixe() {
    assert_eq!(ecrit(None, Some("les-jeux")), "les-jeux");
}

#[test]
fn les_deux_reperes_cohabitent() {
    assert_eq!(
        vocal(None, Some("{jeu} — {serveur}")),
        "Project Zomboid — Le Canap sur Zomboid"
    );
}
