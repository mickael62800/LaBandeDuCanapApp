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

// ── Les quatre tables ──────────────────────────────────────────────────────

#[test]
fn les_quatre_tables_sont_ecrites_dans_un_ordre_fige() {
    let lua = composer(&config(&[
        ("Zombies", "3"),
        ("AllowMiniMap", "false"),
        ("Speed", "2"),
        ("PopulationMultiplier", "1.5"),
    ]))
    .unwrap();

    let map = lua.find("Map = {").expect("Map attendue");
    let lore = lua.find("ZombieLore = {").expect("ZombieLore attendue");
    let cfg = lua.find("ZombieConfig = {").expect("ZombieConfig attendue");
    let racine = lua.find("Zombies = 3").expect("racine attendue");

    assert!(racine < map, "la racine s'ecrit avant les sous-tables");
    assert!(map < lore && lore < cfg, "ordre des sous-tables instable");
}

#[test]
fn un_booleen_s_ecrit_sans_guillemets() {
    let lua = composer(&config(&[("EnableSnowOnGround", "false")])).unwrap();
    assert!(lua.contains("EnableSnowOnGround = false,"), "{lua}");

    let lua = composer(&config(&[("Nutrition", "true")])).unwrap();
    assert!(lua.contains("Nutrition = true,"), "{lua}");
}

#[test]
fn un_booleen_illisible_est_ignore() {
    let lua = composer(&config(&[("Nutrition", "peut-etre"), ("Zombies", "3")])).unwrap();
    assert!(!lua.contains("Nutrition"));
}

/// Le seul reglage vanilla de type texte. Il finit entre guillemets dans du
/// Lua : le jeu de caracteres est restreint plutot qu'echappe, parce qu'un
/// echappement incomplet suffirait a rendre le fichier executable.
#[test]
fn la_liste_d_objets_est_ecrite_entre_guillemets() {
    let lua = composer(&config(&[(
        "WorldItemRemovalList",
        "Base.Hat,Base.Glasses,Base.Maggots",
    )]))
    .unwrap();

    assert!(
        lua.contains("WorldItemRemovalList = \"Base.Hat,Base.Glasses,Base.Maggots\","),
        "{lua}"
    );
}

#[test]
fn un_texte_hors_du_jeu_de_caracteres_est_refuse() {
    for poison in [
        "Base.Hat\", os.execute('x'), y = \"",
        "Base.Hat\\\"",
        "Base.Hat\nBase.Glasses",
        "Base.Hat[[",
        "café",
    ] {
        let lua = composer(&config(&[
            ("WorldItemRemovalList", poison),
            ("Zombies", "3"),
        ]))
        .unwrap();
        assert!(
            !lua.contains("WorldItemRemovalList"),
            "texte {poison:?} accepte :\n{lua}"
        );
    }
}

/// Les cent trente cles doivent etre celles du jeu, chacune dans sa table.
/// Une cle rangee dans la mauvaise table ne produit aucune erreur et n'a
/// simplement aucun effet — c'est le defaut le plus difficile a reperer.
#[test]
fn les_cles_connues_sont_dans_la_bonne_table() {
    let attendus = [
        ("Zombies", Section::Racine),
        ("CarSpawnRate", Section::Racine),
        ("WorldItemRemovalList", Section::Racine),
        ("Speed", Section::ZombieLore),
        ("DisableFakeDead", Section::ZombieLore),
        ("PopulationMultiplier", Section::ZombieConfig),
        ("RallyGroupRadius", Section::ZombieConfig),
        ("AllowMiniMap", Section::Map),
        ("MapAllKnown", Section::Map),
    ];
    for (cle, section) in attendus {
        let r = reglage(cle).unwrap_or_else(|| panic!("{cle} absente de la liste"));
        assert_eq!(r.section, section, "{cle} rangee dans la mauvaise table");
    }
    assert_eq!(REGLAGES.len(), 130, "le nombre de reglages a change");
}

/// Aucune cle de mod ne doit s'etre glissee dans la liste : elles n'ont aucun
/// effet chez qui n'a pas le mod, et brouillent un formulaire deja long.
#[test]
fn aucune_cle_de_mod_ne_figure_dans_la_liste() {
    for r in REGLAGES {
        assert!(
            !r.cle.starts_with("lgd_") && !r.cle.contains('_'),
            "cle suspecte, probablement issue d'un mod : {}",
            r.cle
        );
    }
}
