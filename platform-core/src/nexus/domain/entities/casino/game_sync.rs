//! Reconciliation des jeux mentionnables entre la base et Discord.
//!
//! Deux mondes decrivent le meme etat : la base (ce que le dashboard a
//! enregistre) et Discord (les roles et les messages qui existent vraiment).
//! Rien ne garantit qu'ils restent d'accord — un role supprime a la main dans
//! Discord ne remonte nulle part, et un jeu supprime pendant que le bot est
//! hors ligne laisse son role derriere lui.
//!
//! Ce module ne repare RIEN. Il constate, nomme chaque ecart et laisse la
//! decision a un humain : c'est la seule facon de ne pas choisir a sa place
//! entre « Discord fait foi » et « le dashboard fait foi ». Les deux reponses
//! sont legitimes selon ce qui s'est passe, et une reparation automatique
//! detruirait la moitie du temps le cote qu'il fallait garder.
//!
//! Le calcul est pur : l'inventaire Discord est fourni par le bot (seul a voir
//! Discord), l'etat attendu vient de la base. Aucune I/O ici.

use serde::{Deserialize, Serialize};

/// Un role tel qu'il existe REELLEMENT dans la guilde, vu par le bot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiscordRole {
    pub id: String,
    pub name: String,
    /// Couleur du role. Sert a reconnaitre les roles crees par le bot pour un
    /// jeu : sans cet indice, tout role sans jeu associe passerait pour un
    /// orphelin, y compris les roles de moderation de la guilde.
    pub color: u32,
    pub mentionable: bool,
}

/// Photographie de la guilde prise par le bot a un instant donne.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiscordInventory {
    pub roles: Vec<DiscordRole>,
    /// Identifiants des messages de panneau encore presents dans leur salon.
    pub live_panel_messages: Vec<String>,
    /// Salons devenus illisibles (supprimes, ou permissions retirees). Un
    /// panneau qui s'y trouve n'est PAS declare disparu : on ne sait pas.
    pub unreadable_channels: Vec<String>,
}

/// Nature d'un ecart. Chaque variante porte de quoi l'afficher et la resoudre.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Divergence {
    /// La base reference un role que Discord ne connait plus : supprime a la
    /// main, ou perdu. Les boutons du panneau ne donnent plus rien.
    RoleMissing {
        game_id: String,
        game_name: String,
        role_id: String,
    },
    /// Un jeu existe sans role : ses membres ne peuvent pas etre mentionnes.
    RoleUnbound { game_id: String, game_name: String },
    /// Un role cree par le bot ne correspond a aucun jeu.
    ///
    /// La comparaison porte sur l'IDENTIFIANT, jamais sur le nom : deux roles
    /// peuvent s'appeler « Factorio » sans etre le meme. `duplicate_of` dit
    /// justement qu'un jeu de ce nom existe et pointe AILLEURS — c'est alors
    /// un doublon, pas le reste d'un jeu supprime, et les deux ne se resolvent
    /// pas de la meme facon.
    RoleOrphan {
        role_id: String,
        role_name: String,
        duplicate_of: Option<String>,
    },
    /// Le message de panneau enregistre n'existe plus dans Discord.
    PanelMessageMissing {
        panel_id: String,
        channel_id: String,
        message_id: String,
    },
}

impl Divergence {
    /// Cle stable d'un ecart, pour que le web puisse designer exactement la
    /// ligne a resoudre sans reposter tout le rapport.
    pub fn key(&self) -> String {
        match self {
            Self::RoleMissing { game_id, .. } => format!("role_missing:{game_id}"),
            Self::RoleUnbound { game_id, .. } => format!("role_unbound:{game_id}"),
            Self::RoleOrphan { role_id, .. } => format!("role_orphan:{role_id}"),
            Self::PanelMessageMissing { panel_id, .. } => format!("panel_missing:{panel_id}"),
        }
    }
}

/// Ce qu'on garde comme verite pour resoudre un ecart.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyncDirection {
    /// Discord fait foi : la base s'aligne sur ce qui existe la-bas. Efface une
    /// liaison morte, oublie un panneau disparu.
    Discord,
    /// Le dashboard fait foi : Discord est remis en conformite. Recree un role,
    /// redeploie un panneau, supprime un role orphelin.
    Dashboard,
}

/// Etat de la guilde vu par la base, a confronter a l'inventaire.
#[derive(Debug, Clone)]
pub struct StoredGame {
    pub id: String,
    pub name: String,
    pub role_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StoredPanel {
    pub id: String,
    pub channel_id: String,
    pub message_id: String,
}

/// Resultat de la comparaison, tel que le web l'affiche.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncReport {
    /// Date de l'inventaire ayant servi au calcul (RFC3339). `None` quand le
    /// bot n'a jamais rendu compte : on ne peut alors RIEN affirmer.
    pub inventory_taken_at: Option<String>,
    pub divergences: Vec<Divergence>,
}

impl SyncReport {
    pub fn is_clean(&self) -> bool {
        self.divergences.is_empty()
    }
}

/// Compare l'etat enregistre et l'inventaire Discord.
///
/// `role_color` est la couleur des roles crees par le bot : elle seule permet
/// de distinguer un orphelin d'un role de la guilde qui ne nous regarde pas.
///
/// Sans inventaire (le bot n'a jamais repondu), le rapport est VIDE et non
/// « tout va bien » : ne rien savoir n'est pas la meme chose que tout aller
/// bien, et afficher des ecarts inventes ferait supprimer des roles vivants.
pub fn build_sync_report(
    games: &[StoredGame],
    panels: &[StoredPanel],
    inventory: Option<&DiscordInventory>,
    inventory_taken_at: Option<String>,
    role_color: u32,
) -> SyncReport {
    let Some(inventory) = inventory else {
        return SyncReport {
            inventory_taken_at: None,
            divergences: Vec::new(),
        };
    };

    let mut divergences = Vec::new();

    for game in games {
        match game
            .role_id
            .as_deref()
            .map(str::trim)
            .filter(|r| !r.is_empty())
        {
            None => divergences.push(Divergence::RoleUnbound {
                game_id: game.id.clone(),
                game_name: game.name.clone(),
            }),
            Some(role_id) => {
                if !inventory.roles.iter().any(|role| role.id == role_id) {
                    divergences.push(Divergence::RoleMissing {
                        game_id: game.id.clone(),
                        game_name: game.name.clone(),
                        role_id: role_id.to_string(),
                    });
                }
            }
        }
    }

    // Un orphelin doit cumuler TOUS les indices d'un role cree par le bot :
    // sa couleur, son caractere mentionnable, et l'absence de jeu qui le
    // reclame. Le doute profite au role : mieux vaut manquer un orphelin que
    // proposer de supprimer un role de la guilde.
    for role in &inventory.roles {
        let claimed = games
            .iter()
            .any(|game| game.role_id.as_deref() == Some(role.id.as_str()));
        if !claimed && role.color == role_color && role.mentionable {
            // Un jeu du meme nom existe-t-il, rattache a un AUTRE role ? Si
            // oui, ce role-ci est un doublon : le dire evite de faire croire
            // que le jeu a disparu alors qu'il est parfaitement configure.
            let duplicate_of = games
                .iter()
                .find(|game| game.name.eq_ignore_ascii_case(&role.name))
                .map(|game| game.name.clone());
            divergences.push(Divergence::RoleOrphan {
                role_id: role.id.clone(),
                role_name: role.name.clone(),
                duplicate_of,
            });
        }
    }

    for panel in panels {
        // Un salon illisible ne prouve pas la disparition du message. On se
        // tait plutot que de faire redeployer un panneau qui existe encore.
        if inventory.unreadable_channels.contains(&panel.channel_id) {
            continue;
        }
        if !inventory.live_panel_messages.contains(&panel.message_id) {
            divergences.push(Divergence::PanelMessageMissing {
                panel_id: panel.id.clone(),
                channel_id: panel.channel_id.clone(),
                message_id: panel.message_id.clone(),
            });
        }
    }

    SyncReport {
        inventory_taken_at,
        divergences,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COULEUR: u32 = 0x3498DB;

    fn jeu(id: &str, role: Option<&str>) -> StoredGame {
        StoredGame {
            id: id.into(),
            name: format!("Jeu {id}"),
            role_id: role.map(str::to_string),
        }
    }

    fn role(id: &str, color: u32, mentionable: bool) -> DiscordRole {
        DiscordRole {
            id: id.into(),
            name: format!("Role {id}"),
            color,
            mentionable,
        }
    }

    #[test]
    fn sans_inventaire_aucun_ecart_nest_affirme() {
        // Ne rien savoir n'est pas « tout va bien » : si le bot n'a jamais
        // repondu, inventer des ecarts ferait supprimer des roles vivants.
        let report = build_sync_report(&[jeu("1", Some("100"))], &[], None, None, COULEUR);
        assert!(report.is_clean());
        assert!(report.inventory_taken_at.is_none());
    }

    #[test]
    fn role_supprime_a_la_main_est_detecte() {
        let inventaire = DiscordInventory::default();
        let report = build_sync_report(
            &[jeu("1", Some("100"))],
            &[],
            Some(&inventaire),
            Some("2026-08-17T10:00:00Z".into()),
            COULEUR,
        );
        assert_eq!(
            report.divergences,
            vec![Divergence::RoleMissing {
                game_id: "1".into(),
                game_name: "Jeu 1".into(),
                role_id: "100".into(),
            }]
        );
    }

    #[test]
    fn jeu_sans_role_est_signale() {
        let inventaire = DiscordInventory::default();
        let report = build_sync_report(&[jeu("1", None)], &[], Some(&inventaire), None, COULEUR);
        assert_eq!(
            report.divergences,
            vec![Divergence::RoleUnbound {
                game_id: "1".into(),
                game_name: "Jeu 1".into(),
            }]
        );
        // Une liaison vide ou blanche vaut une absence de liaison.
        let report = build_sync_report(
            &[jeu("1", Some("  "))],
            &[],
            Some(&inventaire),
            None,
            COULEUR,
        );
        assert!(matches!(
            report.divergences.as_slice(),
            [Divergence::RoleUnbound { .. }]
        ));
    }

    #[test]
    fn seul_un_role_aux_marques_du_bot_passe_pour_orphelin() {
        let inventaire = DiscordInventory {
            roles: vec![
                // Le role d'un jeu vivant : jamais orphelin.
                role("100", COULEUR, true),
                // Un vrai orphelin : couleur du bot, mentionnable, sans jeu.
                role("200", COULEUR, true),
                // SECURITE : roles de la guilde. Les proposer a la suppression
                // ferait perdre des roles de moderation sur un clic.
                role("300", 0xFF0000, true),
                role("400", COULEUR, false),
            ],
            ..Default::default()
        };
        let report = build_sync_report(
            &[jeu("1", Some("100"))],
            &[],
            Some(&inventaire),
            None,
            COULEUR,
        );
        assert_eq!(
            report.divergences,
            vec![Divergence::RoleOrphan {
                role_id: "200".into(),
                role_name: "Role 200".into(),
                duplicate_of: None,
            }]
        );
    }

    #[test]
    fn un_role_homonyme_dun_jeu_configure_est_signale_comme_doublon() {
        // Cas reel : deux roles « Factorio » existent dans Discord, le jeu ne
        // pointe que vers l'un. Dire « role sans jeu » laisse croire que le jeu
        // a disparu, alors qu'il est parfaitement configure.
        let inventaire = DiscordInventory {
            roles: vec![role("100", COULEUR, true), role("999", COULEUR, true)],
            ..Default::default()
        };
        let mut jeu_factorio = jeu("1", Some("100"));
        jeu_factorio.name = "Role 999".into(); // homonyme du role orphelin

        let report = build_sync_report(&[jeu_factorio], &[], Some(&inventaire), None, COULEUR);
        assert_eq!(
            report.divergences,
            vec![Divergence::RoleOrphan {
                role_id: "999".into(),
                role_name: "Role 999".into(),
                duplicate_of: Some("Role 999".into()),
            }]
        );
    }

    #[test]
    fn panneau_disparu_detecte_sauf_si_le_salon_est_illisible() {
        let panneau = StoredPanel {
            id: "p1".into(),
            channel_id: "c1".into(),
            message_id: "m1".into(),
        };

        let inventaire = DiscordInventory::default();
        let report = build_sync_report(
            &[],
            std::slice::from_ref(&panneau),
            Some(&inventaire),
            None,
            COULEUR,
        );
        assert!(matches!(
            report.divergences.as_slice(),
            [Divergence::PanelMessageMissing { .. }]
        ));

        // Salon illisible : on ne sait pas, donc on ne dit rien.
        let aveugle = DiscordInventory {
            unreadable_channels: vec!["c1".into()],
            ..Default::default()
        };
        let report = build_sync_report(&[], &[panneau], Some(&aveugle), None, COULEUR);
        assert!(report.is_clean());
    }

    #[test]
    fn guilde_saine_ne_produit_aucun_ecart() {
        let inventaire = DiscordInventory {
            roles: vec![role("100", COULEUR, true)],
            live_panel_messages: vec!["m1".into()],
            unreadable_channels: vec![],
        };
        let panneau = StoredPanel {
            id: "p1".into(),
            channel_id: "c1".into(),
            message_id: "m1".into(),
        };
        let report = build_sync_report(
            &[jeu("1", Some("100"))],
            &[panneau],
            Some(&inventaire),
            Some("2026-08-17T10:00:00Z".into()),
            COULEUR,
        );
        assert!(report.is_clean());
    }

    #[test]
    fn les_cles_identifient_chaque_ligne_sans_ambiguite() {
        let a = Divergence::RoleMissing {
            game_id: "1".into(),
            game_name: "Jeu".into(),
            role_id: "100".into(),
        };
        let b = Divergence::RoleUnbound {
            game_id: "1".into(),
            game_name: "Jeu".into(),
        };
        assert_ne!(a.key(), b.key());
    }
}
