use super::*;

#[test]
fn identifiant_discord_valide_est_accepte() {
    assert!(ensure_guild_id("123456789012345678").is_ok());
}

#[test]
fn identifiant_vide_est_refuse() {
    assert!(ensure_guild_id("").is_err());
}

#[test]
fn identifiant_trop_long_est_refuse() {
    assert!(ensure_guild_id(&"1".repeat(21)).is_err());
}

/// L'endpoint est expose au balayage : tout ce qui n'est pas un entier
/// decimal doit etre arrete avant d'atteindre la persistance.
#[test]
fn caracteres_non_numeriques_sont_refuses() {
    for mauvais in [
        "12345678901234567a",
        "1234'; DROP TABLE--",
        "../../etc/passwd",
        "12345678 90123456",
        "-123456789012345",
    ] {
        assert!(
            ensure_guild_id(mauvais).is_err(),
            "{mauvais} aurait du etre refuse"
        );
    }
}

#[test]
fn limite_absente_prend_la_valeur_par_defaut() {
    assert_eq!(clamp_limit(None, 10, 50), 10);
}

#[test]
fn limite_demandee_est_respectee_dans_les_bornes() {
    assert_eq!(clamp_limit(Some(25), 10, 50), 25);
}

/// Sans ce plafond, une page publique deviendrait un moyen de saturer la
/// base depuis l'exterieur.
#[test]
fn limite_excessive_est_plafonnee() {
    assert_eq!(clamp_limit(Some(1_000_000), 10, 50), 50);
}

#[test]
fn limite_nulle_ou_negative_remonte_a_un() {
    assert_eq!(clamp_limit(Some(0), 10, 50), 1);
    assert_eq!(clamp_limit(Some(-5), 10, 50), 1);
}
