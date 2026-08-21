use super::*;

// ── Economie ──

/// Le defaut ne doit RIEN changer : c'est la condition pour introduire une
/// configuration sur un jeu deja en cours.
#[test]
fn multiplicateur_neutre_ne_touche_pas_au_gain() {
    let c = EconomyConfig::default();
    assert_eq!(c.wheel_payout_multiplier, 100);
    assert_eq!(c.apply_payout(500), 500);
    assert_eq!(c.apply_payout(-2000), -2000);
}

#[test]
fn multiplicateur_double_les_gains_et_les_pertes() {
    let c = EconomyConfig {
        wheel_payout_multiplier: 200,
        ..Default::default()
    };
    assert_eq!(c.apply_payout(500), 1000);
    // Une perte multipliee reste une perte.
    assert_eq!(c.apply_payout(-500), -1000);
}

/// L'ordre des operations compte : diviser d'abord ecraserait les petits
/// montants a zero.
#[test]
fn multiplicateur_n_ecrase_pas_les_petits_montants() {
    let c = EconomyConfig {
        wheel_payout_multiplier: 120,
        ..Default::default()
    };
    assert_eq!(c.apply_payout(50), 60);
}

#[test]
fn transfert_dans_les_bornes_est_accepte() {
    let c = EconomyConfig {
        transfer_min: 10,
        transfer_max: 100,
        ..Default::default()
    };
    assert!(c.validate_transfer(50).is_ok());
    assert!(c.validate_transfer(10).is_ok());
    assert!(c.validate_transfer(100).is_ok());
}

#[test]
fn transfert_hors_bornes_est_refuse() {
    let c = EconomyConfig {
        transfer_min: 10,
        transfer_max: 100,
        ..Default::default()
    };
    assert!(c.validate_transfer(9).is_err());
    assert!(c.validate_transfer(101).is_err());
}

/// 0 signifie « pas de plafond », pas « plafond a zero » — sans quoi tout
/// transfert serait refuse.
#[test]
fn maximum_a_zero_signifie_aucun_plafond() {
    let c = EconomyConfig {
        transfer_max: 0,
        ..Default::default()
    };
    assert!(c.validate_transfer(1_000_000).is_ok());
}

#[test]
fn transferts_desactives_refusent_tout() {
    let c = EconomyConfig {
        transfer_enabled: false,
        ..Default::default()
    };
    assert!(c.validate_transfer(50).is_err());
}

// ── Coussin Piégé ──

/// Les defauts reproduisent les constantes historiques du service de vol.
#[test]
fn defauts_du_vol_reproduisent_le_comportement_historique() {
    let c = CoussinConfig::default();
    // victim/5 == 20 %
    assert_eq!(c.steal_gain(100), 20);
    // thief * 15 / 100
    assert_eq!(c.steal_penalty(100), 15);
}

/// La fenetre de defense doit laisser le temps de voir une notification, et le
/// malus doit peser assez pour que reagir en vaille la peine.
#[test]
fn defauts_de_la_fenetre_de_defense() {
    let c = CoussinConfig::default();
    assert_eq!(c.steal_defense_window_seconds, 60);
    assert_eq!(c.steal_absence_malus, 8);
}

/// Un vol reussi qui ne rapporte rien serait indistinguable d'un echec.
#[test]
fn un_vol_reussi_rapporte_toujours_au_moins_un_coin() {
    let c = CoussinConfig::default();
    assert_eq!(c.steal_gain(1), 1);
    assert_eq!(c.steal_gain(0), 1);
}

#[test]
fn penalite_d_echec_est_toujours_d_au_moins_un_coin() {
    assert_eq!(CoussinConfig::default().steal_penalty(0), 1);
}

// ── Lecture des valeurs stockees ──

fn kv(paires: &[(&str, &str)]) -> Vec<BotGuildConfig> {
    paires
        .iter()
        .map(|(k, v)| BotGuildConfig {
            id: uuid::Uuid::nil(),
            guild_id: "g".into(),
            bot_name: "nexus-coussin".into(),
            config_key: (*k).into(),
            config_value: (*v).into(),
            updated_at: chrono::Utc::now(),
        })
        .collect()
}

#[test]
fn valeur_stockee_remplace_le_defaut() {
    let items = kv(&[("steal_success_pct", "75")]);
    assert_eq!(n(&items, "steal_success_pct", 30_u32), 75);
}

/// Une saisie erronee ne doit pas rendre le jeu injouable.
#[test]
fn valeur_illisible_retombe_sur_le_defaut() {
    let items = kv(&[("steal_success_pct", "beaucoup")]);
    assert_eq!(n(&items, "steal_success_pct", 30_u32), 30);
}

#[test]
fn cle_absente_retombe_sur_le_defaut() {
    assert_eq!(n(&kv(&[]), "steal_success_pct", 30_u32), 30);
}

#[test]
fn booleen_accepte_les_deux_ecritures() {
    assert!(b(&kv(&[("steal_enabled", "true")]), "steal_enabled", false));
    assert!(b(&kv(&[("steal_enabled", "1")]), "steal_enabled", false));
    assert!(!b(
        &kv(&[("steal_enabled", "false")]),
        "steal_enabled",
        true
    ));
    assert!(!b(&kv(&[("steal_enabled", "0")]), "steal_enabled", true));
}

#[test]
fn booleen_illisible_retombe_sur_le_defaut() {
    assert!(b(&kv(&[("steal_enabled", "oui")]), "steal_enabled", true));
}

#[test]
fn un_module_sans_cle_enabled_est_eteint() {
    // FAIL CLOSED. Le defaut valait `true` : le dashboard affichait « inactif »
    // (il applique bien la regle) pendant que Discord repondait normalement.
    // Les deux se contredisaient sans que rien ne permette de trancher.
    assert!(!CoussinConfig::default().enabled);
    assert!(!EconomyConfig::default().enabled);

    assert!(CoussinConfig::default().ensure_enabled().is_err());
}

// ── CoussinConfig ──

#[test]
fn validate_mise_accepte_les_valeurs_valides() {
    let c = CoussinConfig {
        combat_mise_min: 10,
        combat_mise_max: 100,
        ..Default::default()
    };
    assert!(c.validate_mise(50).is_ok());
    assert!(c.validate_mise(10).is_ok());
    assert!(c.validate_mise(100).is_ok());
}

#[test]
fn validate_mise_refuse_hors_bornes() {
    let c = CoussinConfig {
        combat_mise_min: 10,
        combat_mise_max: 100,
        ..Default::default()
    };
    assert!(c.validate_mise(9).is_err());
    assert!(c.validate_mise(101).is_err());
}

#[test]
fn validate_mise_sans_plafond() {
    let c = CoussinConfig {
        combat_mise_max: 0,
        ..Default::default()
    };
    assert!(c.validate_mise(1_000_000).is_ok());
}

#[test]
fn shop_price_applique_le_multiplicateur_global() {
    let c = CoussinConfig {
        shop_price_pct: 120,
        shop_prices: vec![],
        ..Default::default()
    };
    assert_eq!(c.shop_price("unknown", 100), 120);
}

#[test]
fn shop_price_respecte_les_prix_personnalises() {
    let c = CoussinConfig {
        shop_prices: vec![("item1".to_string(), 50)],
        shop_price_pct: 100,
        ..Default::default()
    };
    assert_eq!(c.shop_price("item1", 100), 50);
}

#[test]
fn shop_price_jamais_moins_d_un() {
    let c = CoussinConfig {
        shop_price_pct: 1,
        shop_prices: vec![],
        ..Default::default()
    };
    assert_eq!(c.shop_price("any", 100), 1);
}

#[test]
fn combat_rules_construit_la_structure() {
    let c = CoussinConfig::default();
    let rules = c.combat_rules();
    assert_eq!(rules.level_gap_max, c.level_gap_max);
    assert_eq!(rules.dice_faces, c.dice_faces);
}

#[test]
fn ensure_enabled_avec_module_actif() {
    let c = CoussinConfig {
        enabled: true,
        ..Default::default()
    };
    assert!(c.ensure_enabled().is_ok());
}
