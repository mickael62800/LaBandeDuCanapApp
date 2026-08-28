use super::*;

/// Lignes reelles, copiees d'un premier demarrage de Project Zomboid. Les
/// reecrire de memoire aurait produit un analyseur qui ne reconnait que ce que
/// j'imaginais.
fn lignes(brut: &[&str]) -> Vec<String> {
    brut.iter().map(|l| (*l).to_string()).collect()
}

#[test]
fn sans_rien_de_reconnaissable_aucune_progression() {
    assert!(lire_progression(&[]).is_none());
    assert!(lire_progression(&lignes(&["LOG : General f:0 st:1> Saving"])).is_none());
}

// ── Mise a jour de SteamCMD ────────────────────────────────────────────────

#[test]
fn la_mise_a_jour_de_steamcmd_est_lue() {
    let p = lire_progression(&lignes(&[
        "[ 44%] Downloading update (21708 of 40321 KB)...",
    ]))
    .expect("progression attendue");

    assert_eq!(p.etape, EtapeDemarrage::MiseAJourSteamCmd);
    assert_eq!(p.pourcentage, Some(44.0));
    assert_eq!(p.octets, Some((21708 * 1024, 40321 * 1024)));
}

/// `[----]` precede le premier chiffre. Le lire comme zero ferait reculer la
/// barre au rafraichissement suivant.
#[test]
fn une_progression_inconnue_n_est_pas_zero() {
    assert!(
        lire_progression(&lignes(&["[----] Downloading update (0 of 40321 KB)...",])).is_none()
    );
}

// ── Telechargement du jeu ──────────────────────────────────────────────────

#[test]
fn chaque_etat_steam_a_son_etape() {
    let cas = [
        ("0x3", "reconfiguring", EtapeDemarrage::Preparation),
        ("0x11", "preallocating", EtapeDemarrage::Preparation),
        ("0x61", "downloading", EtapeDemarrage::Telechargement),
        ("0x81", "verifying update", EtapeDemarrage::Verification),
        ("0x101", "committing", EtapeDemarrage::Installation),
    ];
    for (code, mot, attendue) in cas {
        let ligne = format!(" Update state ({code}) {mot}, progress: 42.37 (30 / 70)");
        let p = lire_progression(&lignes(&[&ligne])).expect("progression attendue");
        assert_eq!(p.etape, attendue, "etat {code}");
        assert_eq!(p.pourcentage, Some(42.37));
    }
}

#[test]
fn les_octets_du_jeu_sont_lus() {
    let p = lire_progression(&lignes(&[
        " Update state (0x61) downloading, progress: 40.76 (2939486986 / 7212532083)",
    ]))
    .unwrap();

    assert_eq!(p.octets, Some((2939486986, 7212532083)));
}

/// `0x0 unknown, progress: 0.00 (0 / 0)` clot la sequence. Le prendre pour une
/// etape ferait retomber la barre a zero juste avant le lancement.
#[test]
fn l_etat_final_inconnu_est_ignore() {
    let p = lire_progression(&lignes(&[
        " Update state (0x101) committing, progress: 86.03 (6204920931 / 7212532083)",
        " Update state (0x0) unknown, progress: 0.00 (0 / 0)",
    ]))
    .unwrap();

    assert_eq!(p.etape, EtapeDemarrage::Installation);
    assert_eq!(p.pourcentage, Some(86.03));
}

/// `(0 / 0)` n'est pas une taille : l'afficher donnerait « 0 sur 0 ».
#[test]
fn une_taille_nulle_n_est_pas_rapportee() {
    let p = lire_progression(&lignes(&[
        " Update state (0x3) reconfiguring, progress: 0.00 (0 / 0)",
    ]))
    .unwrap();

    assert_eq!(p.etape, EtapeDemarrage::Preparation);
    assert_eq!(p.octets, None);
}

/// LA DERNIERE LIGNE FAIT FOI. Lire la premiere afficherait eternellement la
/// mise a jour de SteamCMD pendant que le jeu se telecharge.
#[test]
fn la_derniere_etape_remplace_les_precedentes() {
    let p = lire_progression(&lignes(&[
        "[ 99%] Downloading update (40321 of 40321 KB)...",
        " Update state (0x11) preallocating, progress: 81.73 (5894996141 / 7212532083)",
        " Update state (0x61) downloading, progress: 7.25 (522952899 / 7212532083)",
    ]))
    .unwrap();

    assert_eq!(p.etape, EtapeDemarrage::Telechargement);
    assert_eq!(p.pourcentage, Some(7.25));
}

// ── Mods du Workshop ───────────────────────────────────────────────────────

/// LE POINT LE PLUS IMPORTANT DE CE MODULE. Les mods ne publient AUCUN
/// pourcentage. En inventer un — au prorata des identifiants vus, par exemple —
/// afficherait une barre qui avance sans rapport avec la realite.
#[test]
fn les_mods_n_ont_pas_de_pourcentage() {
    let p = lire_progression(&lignes(&[
        "LOG  : General f:0 st:1> Workshop: DownloadPending GetItemState()=NeedsUpdate|Downloading|DownloadPending ID=2599752664",
    ]))
    .unwrap();

    assert_eq!(p.etape, EtapeDemarrage::Mods);
    assert_eq!(p.pourcentage, None);
    assert_eq!(p.octets, None);
}

/// Chaque identifiant apparait des dizaines de fois : seul leur ENSEMBLE dit
/// combien de mods ont ete abordes.
#[test]
fn les_mods_sont_comptes_une_seule_fois_chacun() {
    let mut brut = Vec::new();
    for id in ["2599752664", "2599752664", "2337452747", "2599752664"] {
        brut.push(format!(
            "LOG : General f:0 st:1> Workshop: DownloadPending GetItemState()=Downloading ID={id}"
        ));
    }
    let p = lire_progression(&brut).unwrap();

    assert_eq!(p.mods_vus, vec!["2599752664", "2337452747"]);
}

/// Les mods viennent APRES le jeu : voir un identifiant Workshop signifie que
/// le telechargement du jeu est termine.
#[test]
fn les_mods_succedent_au_telechargement_du_jeu() {
    let p = lire_progression(&lignes(&[
        " Update state (0x61) downloading, progress: 98.32 (7091416339 / 7212532083)",
        " Update state (0x101) committing, progress: 86.03 (6204920931 / 7212532083)",
        "LOG : General f:0 st:1> Workshop: DownloadPending GetItemState()=Downloading ID=2536865912",
    ]))
    .unwrap();

    assert_eq!(p.etape, EtapeDemarrage::Mods);
    assert_eq!(p.mods_vus, vec!["2536865912"]);
}

#[test]
fn chaque_etape_a_un_libelle() {
    for etape in [
        EtapeDemarrage::MiseAJourSteamCmd,
        EtapeDemarrage::Preparation,
        EtapeDemarrage::Telechargement,
        EtapeDemarrage::Verification,
        EtapeDemarrage::Installation,
        EtapeDemarrage::Mods,
    ] {
        assert!(!etape.libelle().is_empty());
    }
}
