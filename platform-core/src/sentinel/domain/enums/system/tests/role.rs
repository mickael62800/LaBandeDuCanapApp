//! Tests de la hierarchie RBAC (roundtrip + relation d'ordre `satisfies`).

use super::*;

/// Tous les roles, du plus faible au plus fort. Une seule liste : un test qui
/// oublierait `Member` passerait au vert sans rien verifier a son sujet.
const TOUS: [Role; 5] = [
    Role::Member,
    Role::Viewer,
    Role::Moderator,
    Role::Admin,
    Role::Owner,
];

#[test]
fn from_str_roundtrip_all() {
    for r in TOUS {
        assert_eq!(Role::from_str(r.as_str()), Some(r));
    }
}

#[test]
fn from_str_unknown_is_none() {
    assert_eq!(Role::from_str("root"), None);
    assert_eq!(Role::from_str(""), None);
    assert_eq!(Role::from_str("Admin"), None); // sensible a la casse
}

#[test]
fn ordering_matches_hierarchy() {
    // Verifie la chaine complete plutot que des paires choisies : ainsi
    // l'ajout d'un role au milieu casse le test au lieu de passer inapercu.
    for paire in TOUS.windows(2) {
        assert!(
            paire[0] < paire[1],
            "{:?} devrait preceder {:?}",
            paire[0],
            paire[1]
        );
    }
}

#[test]
fn satisfies_at_or_above_required() {
    // Owner satisfait tout.
    for req in TOUS {
        assert!(Role::Owner.satisfies(req));
    }
    // Egalite satisfait.
    assert!(Role::Moderator.satisfies(Role::Moderator));
    // En dessous ne satisfait pas.
    assert!(!Role::Viewer.satisfies(Role::Moderator));
    assert!(!Role::Moderator.satisfies(Role::Admin));
    assert!(!Role::Admin.satisfies(Role::Owner));
}

/// Le point du palier : un membre ordinaire ne satisfait AUCUN gate du
/// back-office, alors qu'il retombait sur `Viewer` auparavant.
#[test]
fn member_ne_satisfait_aucun_gate_du_backoffice() {
    for req in [Role::Viewer, Role::Moderator, Role::Admin, Role::Owner] {
        assert!(
            !Role::Member.satisfies(req),
            "Member ne doit pas satisfaire {req:?}"
        );
    }
}

#[test]
fn member_est_le_role_le_plus_faible() {
    for r in TOUS {
        assert!(Role::Member <= r);
    }
}

#[test]
fn acces_backoffice_commence_a_viewer() {
    assert!(!Role::Member.has_backoffice_access());
    for r in [Role::Viewer, Role::Moderator, Role::Admin, Role::Owner] {
        assert!(
            r.has_backoffice_access(),
            "{r:?} doit acceder au back-office"
        );
    }
}
