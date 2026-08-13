//! Tests des paliers de roles par niveau.

use super::*;

// ── Analyse de la configuration ──

#[test]
fn analyse_une_liste_simple() {
    let p = analyser_paliers("1:100,5:200,10:300");
    assert_eq!(
        p,
        vec![
            Palier {
                niveau: 1,
                role_id: 100
            },
            Palier {
                niveau: 5,
                role_id: 200
            },
            Palier {
                niveau: 10,
                role_id: 300
            },
        ]
    );
}

#[test]
fn accepte_les_retours_a_la_ligne_et_les_espaces() {
    // Le champ du back-office est multiligne : coller une liste depuis un
    // tableur ne doit pas obliger a tout remettre sur une ligne.
    let p = analyser_paliers(" 1 : 100 \n 5:200 ,\n10:300");
    assert_eq!(p.len(), 3);
    assert_eq!(p[0].role_id, 100);
}

#[test]
fn trie_par_niveau_croissant() {
    let p = analyser_paliers("10:300,1:100,5:200");
    assert_eq!(
        p.iter().map(|x| x.niveau).collect::<Vec<_>>(),
        vec![1, 5, 10]
    );
}

#[test]
fn ignore_les_entrees_illisibles_sans_perdre_les_autres() {
    // Une virgule en trop ou une faute de frappe ne doit pas priver de roles
    // tous les paliers valides.
    let p = analyser_paliers("1:100,,abc,5:,:200,7:xyz,10:300");
    assert_eq!(p.len(), 2);
    assert_eq!(p[0].role_id, 100);
    assert_eq!(p[1].role_id, 300);
}

#[test]
fn rejette_le_niveau_zero_et_negatif() {
    // Niveau 0 = etat initial : un palier a 0 donnerait le role a tout le
    // monde en permanence, ce qui est le travail de default_role_ids.
    assert!(analyser_paliers("0:100,-3:200").is_empty());
}

#[test]
fn rejette_un_role_nul() {
    assert!(analyser_paliers("5:0").is_empty());
}

#[test]
fn garde_le_premier_role_quand_un_niveau_est_dedouble() {
    let p = analyser_paliers("5:100,5:999");
    assert_eq!(
        p,
        vec![Palier {
            niveau: 5,
            role_id: 100
        }]
    );
}

#[test]
fn chaine_vide_ne_donne_aucun_palier() {
    assert!(analyser_paliers("").is_empty());
    assert!(analyser_paliers("   ").is_empty());
}

// ── Mode ──

#[test]
fn mode_inconnu_retombe_sur_cumulatif() {
    // Le mode qui n'enleve rien : une faute de saisie ne peut pas depouiller
    // un membre de ses roles.
    assert_eq!(
        ModePalier::depuis_config("nimportequoi"),
        ModePalier::Cumulatif
    );
    assert_eq!(ModePalier::depuis_config(""), ModePalier::Cumulatif);
}

#[test]
fn mode_remplacement_est_reconnu() {
    assert_eq!(
        ModePalier::depuis_config("remplacement"),
        ModePalier::Remplacement
    );
    assert_eq!(
        ModePalier::depuis_config(" replace "),
        ModePalier::Remplacement
    );
}

// ── Selection des roles ──

#[test]
fn cumulatif_donne_tous_les_paliers_atteints() {
    let p = analyser_paliers("1:100,5:200,10:300");
    let (ajout, retrait) = roles_pour_niveau(&p, 7, ModePalier::Cumulatif);
    assert_eq!(ajout, vec![100, 200]);
    // Le palier 10 n'est pas atteint : il part au retrait, ce qui corrige un
    // membre retrograde.
    assert_eq!(retrait, vec![300]);
}

#[test]
fn remplacement_ne_garde_que_le_palier_courant() {
    let p = analyser_paliers("1:100,5:200,10:300");
    let (ajout, retrait) = roles_pour_niveau(&p, 7, ModePalier::Remplacement);
    assert_eq!(ajout, vec![200]);
    assert_eq!(retrait, vec![100, 300]);
}

#[test]
fn niveau_sous_le_premier_palier_ne_donne_rien() {
    let p = analyser_paliers("5:200,10:300");
    let (ajout, _) = roles_pour_niveau(&p, 1, ModePalier::Cumulatif);
    assert!(ajout.is_empty());
}

#[test]
fn niveau_zero_est_traite_comme_niveau_un() {
    // Defense residuelle : une ligne d'XP heritee (avant la bascule en base 1)
    // peut encore porter un `level = 0`. Un tel membre reste un membre accepte,
    // niveau 1 au minimum : son role de depart (palier niveau 1) ne doit JAMAIS
    // partir au retrait, sinon la boucle periodique le reprend a chaque tick et
    // le membre finit sans aucun role.
    let p = analyser_paliers("1:100,5:200");
    let (ajout, retrait) = roles_pour_niveau(&p, 0, ModePalier::Cumulatif);
    assert_eq!(ajout, vec![100]);
    assert!(!retrait.contains(&100));

    // Idem en mode remplacement : le palier de depart est le palier courant.
    let (ajout, retrait) = roles_pour_niveau(&p, 0, ModePalier::Remplacement);
    assert_eq!(ajout, vec![100]);
    assert!(!retrait.contains(&100));
}

#[test]
fn niveau_exact_declenche_le_palier() {
    // Le seuil est inclusif : atteindre 5 donne le role du palier 5.
    let p = analyser_paliers("5:200");
    let (ajout, _) = roles_pour_niveau(&p, 5, ModePalier::Cumulatif);
    assert_eq!(ajout, vec![200]);
}

#[test]
fn niveau_tres_haut_donne_le_dernier_palier() {
    let p = analyser_paliers("1:100,5:200,10:300");
    let (ajout, retrait) = roles_pour_niveau(&p, 999, ModePalier::Remplacement);
    assert_eq!(ajout, vec![300]);
    assert_eq!(retrait, vec![100, 200]);
}

#[test]
fn retrogradation_retire_le_rang_perdu() {
    // Correction d'XP ou remise a zero : le membre ne doit pas garder un rang
    // qu'il ne merite plus. C'est pourquoi les paliers SUPERIEURS partent
    // aussi au retrait, pas seulement les inferieurs.
    let p = analyser_paliers("1:100,5:200,10:300");
    let (ajout, retrait) = roles_pour_niveau(&p, 2, ModePalier::Cumulatif);
    assert_eq!(ajout, vec![100]);
    assert_eq!(retrait, vec![200, 300]);
}

#[test]
fn aucun_palier_configure_ne_touche_a_rien() {
    let (ajout, retrait) = roles_pour_niveau(&[], 50, ModePalier::Remplacement);
    assert!(ajout.is_empty());
    assert!(retrait.is_empty());
}

#[test]
fn un_meme_role_sur_deux_paliers_n_est_pas_retire_a_tort() {
    // Saisie plausible : le meme role sert de rang a deux seuils. Il ne doit
    // pas figurer a la fois dans l'ajout et le retrait, sinon le bot le pose
    // puis l'enleve.
    let p = analyser_paliers("1:100,5:100,10:300");
    let (ajout, retrait) = roles_pour_niveau(&p, 7, ModePalier::Cumulatif);
    assert!(ajout.contains(&100));
    assert!(!retrait.contains(&100));
}
