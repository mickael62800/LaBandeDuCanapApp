//! Noms des salons Discord d'une session de jeu.
//!
//! Trois salons naissent avec une session : l'inscription, le salon prive des
//! inscrits, et le vocal. Leurs noms etaient figes dans le code du bot
//! (`inscription-{slug}`, `salon-{slug}`, `Vocal {nom du serveur}`). Ce module
//! les rend configurables, a deux niveaux :
//!
//!   - un MODELE par guilde, avec des reperes `{jeu}` et `{serveur}`, qui vaut
//!     pour tous les jeux presents et futurs ;
//!   - un NOM LIBRE par serveur, qui remplace le modele pour ce serveur-la.
//!
//! Le calcul est ici, et non dans le bot, parce qu'il doit donner exactement
//! le meme resultat a la creation d'une session et lors d'un renommage
//! ulterieur. Deux implementations auraient fini par diverger, et un salon
//! aurait porte un nom que plus aucun nettoyage ne reconnaitrait.

/// Modeles par defaut, ceux qui reproduisent le comportement historique.
pub const MODELE_INSCRIPTION_PAR_DEFAUT: &str = "inscription-{jeu}";
pub const MODELE_PRIVE_PAR_DEFAUT: &str = "salon-{jeu}";
pub const MODELE_VOCAL_PAR_DEFAUT: &str = "Vocal {serveur}";

/// Discord refuse au-dela de cent caracteres.
pub const LONGUEUR_MAX: usize = 100;

/// Nature du salon : elle decide des caracteres tolerés.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeDeSalon {
    /// Salon ecrit : Discord impose des minuscules et remplace les espaces par
    /// des tirets. On applique la meme transformation nous-memes, sinon le nom
    /// enregistre en base differerait de celui affiche a l'ecran — et les
    /// nettoyages qui comparent des noms ne retrouveraient plus rien.
    Ecrit,
    /// Salon vocal : Discord accepte majuscules, espaces et emoji.
    Vocal,
}

/// Remplace les reperes d'un modele.
///
/// Les deux seuls reperes sont `{jeu}` et `{serveur}`. Un modele qui n'en
/// contient aucun est parfaitement valable : c'est un nom fixe, et il n'y a
/// aucune raison de l'interdire.
fn substituer(modele: &str, jeu: &str, serveur: &str) -> String {
    modele.replace("{jeu}", jeu).replace("{serveur}", serveur)
}

/// Met un nom en forme pour Discord.
///
/// TRONQUER SUR UNE FRONTIERE DE CARACTERE. Discord compte des caracteres, pas
/// des octets, et un nom d'emoji ou d'accents coupe au milieu d'un point de
/// code produirait une chaine invalide. On compte donc des `char`.
fn mettre_en_forme(brut: &str, genre: TypeDeSalon) -> String {
    let nettoye: String = match genre {
        TypeDeSalon::Ecrit => brut
            .trim()
            .to_lowercase()
            .chars()
            .map(|c| if c.is_whitespace() { '-' } else { c })
            .filter(|c| !matches!(c, ',' | '.' | '?' | '!' | '"' | '/' | '#'))
            .filter(|c| *c != '\'' && *c != char::from(92))
            .collect(),
        TypeDeSalon::Vocal => brut.trim().chars().filter(|c| *c != '\n').collect(),
    };

    let nettoye = match genre {
        // Des tirets accoles viennent d'espaces multiples ou d'un repere vide.
        // Discord les accepte, mais « inscription--- » se lit mal.
        TypeDeSalon::Ecrit => {
            let mut sortie = String::with_capacity(nettoye.len());
            let mut tiret = false;
            for c in nettoye.chars() {
                if c == '-' {
                    if !tiret {
                        sortie.push(c);
                    }
                    tiret = true;
                } else {
                    sortie.push(c);
                    tiret = false;
                }
            }
            sortie.trim_matches('-').to_string()
        }
        TypeDeSalon::Vocal => nettoye,
    };

    nettoye.chars().take(LONGUEUR_MAX).collect()
}

/// Nom final d'un salon de session.
///
/// L'ordre des sources est : nom libre du serveur, puis modele de la guilde,
/// puis modele par defaut. Chaque source qui donne un nom VIDE apres mise en
/// forme est ignoree au profit de la suivante — un salon sans nom serait
/// refuse par Discord, et la creation de la session echouerait a moitie, en
/// laissant derriere elle un role et parfois un ou deux salons.
pub fn nom_de_salon(
    nom_libre: Option<&str>,
    modele_guilde: Option<&str>,
    modele_defaut: &str,
    jeu: &str,
    serveur: &str,
    genre: TypeDeSalon,
) -> String {
    for source in [nom_libre, modele_guilde, Some(modele_defaut)]
        .into_iter()
        .flatten()
    {
        let candidat = mettre_en_forme(&substituer(source, jeu, serveur), genre);
        if !candidat.is_empty() {
            return candidat;
        }
    }

    // Les trois sources ont echoue : le modele par defaut lui-meme ne donne
    // rien, ce qui suppose un nom de jeu vide. On ne renvoie jamais une chaine
    // vide, sous peine de faire echouer la creation du salon.
    match genre {
        TypeDeSalon::Ecrit => "salon-de-jeu".to_string(),
        TypeDeSalon::Vocal => "Vocal".to_string(),
    }
}

#[cfg(test)]
#[path = "tests/channel_names.rs"]
mod tests;
