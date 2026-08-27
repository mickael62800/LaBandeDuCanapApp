use std::collections::HashMap;

use super::*;

fn config(paires: &[(&str, &str)]) -> HashMap<String, String> {
    paires
        .iter()
        .map(|(c, v)| (format!("{PREFIXE_SANDBOX}{c}"), (*v).to_string()))
        .collect()
}

#[test]
fn sans_aucun_reglage_aucun_fichier_n_est_ecrit() {
    assert!(composer(&HashMap::new()).is_none());
    // Une valeur vide n'est pas un choix : le champ a ete laisse tel quel.
    assert!(composer(&config(&[("Zombies", "")])).is_none());
    assert!(composer(&config(&[("Zombies", "   ")])).is_none());
}

#[test]
fn un_reglage_de_racine_est_ecrit_avec_la_version() {
    let lua = composer(&config(&[("Zombies", "3")])).expect("un fichier est attendu");

    assert!(lua.starts_with("SandboxVars = {\n"));
    assert!(lua.contains("VERSION = 5,"));
    assert!(lua.contains("    Zombies = 3,"));
    assert!(lua.trim_end().ends_with('}'));
}

/// LA SOUS-TABLE COMPTE. Ecrire `Speed` a la racine ne produit aucune erreur
/// et n'a simplement aucun effet : le jeu ne le lit que sous `ZombieLore`.
#[test]
fn les_reglages_des_morts_vont_dans_leur_sous_table() {
    let lua = composer(&config(&[("Zombies", "3"), ("Speed", "2")])).unwrap();

    let debut_lore = lua.find("ZombieLore = {").expect("sous-table attendue");
    let position_speed = lua.find("Speed = 2").expect("reglage attendu");
    assert!(
        position_speed > debut_lore,
        "Speed ecrit hors de ZombieLore :\n{lua}"
    );
    // Zombies reste a la racine, avant la sous-table.
    assert!(lua.find("Zombies = 3").unwrap() < debut_lore);
}

#[test]
fn sans_reglage_de_morts_la_sous_table_n_apparait_pas() {
    let lua = composer(&config(&[("Zombies", "3")])).unwrap();
    assert!(!lua.contains("ZombieLore"));
}

/// Le fichier doit etre identique d'une ecriture a l'autre pour la meme
/// configuration : l'ordre vient de la liste des reglages, jamais de celui
/// d'un `HashMap`, qui change a chaque execution.
#[test]
fn le_fichier_est_stable_d_une_ecriture_a_l_autre() {
    let c = config(&[
        ("Speed", "2"),
        ("Zombies", "3"),
        ("FoodLoot", "1"),
        ("Sight", "2"),
    ]);
    let premier = composer(&c).unwrap();
    for _ in 0..20 {
        assert_eq!(composer(&c).unwrap(), premier);
    }
}

/// UNE VALEUR ILLISIBLE NE DOIT RIEN ECRIRE. Un fichier Lua invalide empeche le
/// jeu de charger la partie ENTIERE, pas seulement ce reglage.
#[test]
fn une_valeur_illisible_est_ignoree_et_non_devinee() {
    let lua = composer(&config(&[("Zombies", "beaucoup"), ("FoodLoot", "2")])).unwrap();

    assert!(!lua.contains("Zombies"));
    assert!(lua.contains("FoodLoot = 2"));
}

/// Ce qui empeche une valeur saisie de devenir du code execute au chargement.
#[test]
fn aucune_injection_lua_ne_passe() {
    for poison in [
        "1, os.execute('rm -rf /')",
        "} print('x') SandboxVars = {",
        "\"texte\"",
        "0x10",
        "nil",
    ] {
        let lua = composer(&config(&[("Zombies", poison), ("FoodLoot", "2")])).unwrap();
        assert!(
            !lua.contains("Zombies"),
            "valeur {poison:?} acceptee :\n{lua}"
        );
    }
}

#[test]
fn un_entier_s_ecrit_sans_decimale() {
    let lua = composer(&config(&[("XpMultiplier", "2.0")])).unwrap();
    assert!(lua.contains("XpMultiplier = 2,"), "{lua}");

    let lua = composer(&config(&[("XpMultiplier", "1.5")])).unwrap();
    assert!(lua.contains("XpMultiplier = 1.5,"), "{lua}");
}

/// Une cle inconnue du jeu ne doit pas se retrouver dans le fichier, meme si
/// quelqu'un l'a rangee en base : elle ne servirait a rien, et brouillerait la
/// relecture du fichier.
#[test]
fn une_cle_hors_liste_n_est_pas_ecrite() {
    let mut c = config(&[("Zombies", "3")]);
    c.insert(format!("{PREFIXE_SANDBOX}InventeeParQuelquUn"), "9".into());

    let lua = composer(&c).unwrap();
    assert!(!lua.contains("Inventee"));
}

#[test]
fn le_prefixe_distingue_les_cles_de_bac_a_sable() {
    assert!(est_cle_sandbox("SANDBOX_Zombies"));
    assert!(!est_cle_sandbox("SERVER_NAME"));
    assert!(!est_cle_sandbox("MAX_PLAYERS"));
}

/// Le nom du serveur fait partie du chemin : chaque partie a son bac a sable,
/// et c'est aussi pourquoi renommer un serveur demarre une partie vierge.
#[test]
fn le_chemin_porte_le_nom_du_serveur() {
    assert_eq!(
        chemin_du_fichier("La Bande Du Canap"),
        "/home/steam/Zomboid/Server/La Bande Du Canap_SandboxVars.lua"
    );
}

#[test]
fn les_reglages_exposes_sont_uniques() {
    let mut vues = std::collections::HashSet::new();
    for r in REGLAGES {
        assert!(
            vues.insert((r.section, r.cle)),
            "reglage en double : {}",
            r.cle
        );
    }
    assert!(reglage("Zombies").is_some());
    assert!(reglage("PasUnReglage").is_none());
}
