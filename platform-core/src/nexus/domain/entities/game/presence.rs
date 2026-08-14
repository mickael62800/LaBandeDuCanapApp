//! Presence des joueurs sur un serveur de jeu, via RCON.
//!
//! Chaque jeu expose ses joueurs differemment : la commande ET le format de
//! reponse changent. Ce module isole cette difference pour que les jobs
//! n'aient pas a la connaitre.
//!
//! Enjeu au-dela de l'affichage : le nombre de joueurs alimente
//! `last_player_count` et donc l'extinction automatique (idle shutdown). Une
//! commande inadaptee renvoie « 0 joueur » sur un serveur peuple — et le
//! worker finit par eteindre un serveur ou des gens jouent.

/// Un joueur connecte, tel que rapporte par le serveur de jeu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerPresence {
    pub name: String,
    /// Identite verifiable dans le jeu quand le serveur l'expose. Palworld
    /// renvoie le SteamID64, ce qui permet de relier le joueur a un membre
    /// Discord. Minecraft ne renvoie qu'un pseudo : `None`, et aucun haut fait
    /// ne peut alors etre attribue sur cette base.
    pub game_player_id: Option<String>,
}

/// Commande RCON qui liste les joueurs connectes, selon le jeu.
///
/// Defaut `list` : c'est la commande Minecraft, historiquement utilisee pour
/// tous les jeux. On la conserve comme repli pour ne pas modifier le
/// comportement des serveurs qui fonctionnent deja avec.
pub fn players_command(template_slug: &str) -> &'static str {
    let slug = template_slug.to_ascii_lowercase();
    if slug.starts_with("palworld") {
        "ShowPlayers"
    } else {
        "list"
    }
}

/// Analyse la reponse RCON en liste de joueurs.
pub fn parse_players(template_slug: &str, raw: &str) -> Vec<PlayerPresence> {
    let slug = template_slug.to_ascii_lowercase();
    if slug.starts_with("palworld") {
        parse_palworld_show_players(raw)
    } else {
        parse_minecraft_list(raw)
    }
}

/// `ShowPlayers` (Palworld) renvoie un CSV avec en-tete :
///
/// ```text
/// name,playeruid,steamid
/// DarkPoney,1234567890,76561198000000000
/// ```
///
/// La derniere colonne est l'identifiant Steam. Les noms de joueurs peuvent
/// contenir des virgules : on decoupe donc par la DROITE (les deux dernieres
/// colonnes sont numeriques), et ce qui reste est le nom.
fn parse_palworld_show_players(raw: &str) -> Vec<PlayerPresence> {
    let mut players = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // En-tete du tableau : presente a chaque appel, ce n'est pas un joueur.
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("name,") {
            continue;
        }
        let Some((reste, dernier)) = line.rsplit_once(',') else {
            continue;
        };
        let Some((nom, _uid)) = reste.rsplit_once(',') else {
            continue;
        };
        let nom = nom.trim();
        let steam_id = dernier.trim();
        if nom.is_empty() {
            continue;
        }
        players.push(PlayerPresence {
            name: nom.to_owned(),
            // Un serveur peut renvoyer `00000000` pour un joueur dont l'identite
            // n'est pas encore resolue : ce n'est pas une identite exploitable.
            game_player_id: is_steam_id64(steam_id).then(|| steam_id.to_owned()),
        });
    }
    players
}

fn is_steam_id64(value: &str) -> bool {
    value.len() == 17 && value.chars().all(|c| c.is_ascii_digit()) && value.starts_with("7656119")
}

/// `list` (Minecraft) :
/// `There are 2 of a max of 20 players online: alice, bob`
fn parse_minecraft_list(raw: &str) -> Vec<PlayerPresence> {
    let Some(idx) = raw.find(':') else {
        return Vec::new();
    };
    raw[idx + 1..]
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|name| PlayerPresence {
            name: name.to_owned(),
            game_player_id: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palworld_utilise_show_players_et_pas_list() {
        assert_eq!(players_command("palworld"), "ShowPlayers");
        assert_eq!(players_command("minecraft-vanilla"), "list");
        // Repli sur le comportement historique pour les autres jeux.
        assert_eq!(players_command("valheim"), "list");
    }

    #[test]
    fn palworld_extrait_le_steam_id_et_ignore_l_entete() {
        let raw = "name,playeruid,steamid\n\
                   DarkPoney,1234567890,76561198000000000\n\
                   Voss,987654321,76561198111111111\n";
        let players = parse_players("palworld", raw);
        assert_eq!(players.len(), 2);
        assert_eq!(players[0].name, "DarkPoney");
        assert_eq!(
            players[0].game_player_id.as_deref(),
            Some("76561198000000000")
        );
        assert_eq!(players[1].name, "Voss");
    }

    #[test]
    fn palworld_gere_un_nom_contenant_une_virgule() {
        // Decoupage par la droite : les deux dernieres colonnes sont connues,
        // tout le reste appartient au nom.
        let raw = "name,playeruid,steamid\nNom, avec virgule,123,76561198000000000\n";
        let players = parse_players("palworld", raw);
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].name, "Nom, avec virgule");
        assert_eq!(
            players[0].game_player_id.as_deref(),
            Some("76561198000000000")
        );
    }

    #[test]
    fn palworld_refuse_un_identifiant_non_steam() {
        // Le joueur est bien connecte (il compte), mais son identite n'est pas
        // exploitable : aucun haut fait ne doit pouvoir s'y accrocher.
        let raw = "name,playeruid,steamid\nInconnu,123,00000000\n";
        let players = parse_players("palworld", raw);
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].game_player_id, None);
    }

    #[test]
    fn palworld_sans_joueur_renvoie_une_liste_vide() {
        assert!(parse_players("palworld", "name,playeruid,steamid\n").is_empty());
        assert!(parse_players("palworld", "").is_empty());
    }

    #[test]
    fn minecraft_conserve_son_analyse() {
        let players = parse_players(
            "minecraft-vanilla",
            "There are 2 of a max of 20 players online: alice, bob",
        );
        assert_eq!(players.len(), 2);
        assert_eq!(players[0].name, "alice");
        assert_eq!(players[1].name, "bob");
        assert!(players[0].game_player_id.is_none());
    }

    #[test]
    fn minecraft_sans_joueur_renvoie_une_liste_vide() {
        let players = parse_players(
            "minecraft-vanilla",
            "There are 0 of a max of 20 players online:",
        );
        assert!(players.is_empty());
    }
}
