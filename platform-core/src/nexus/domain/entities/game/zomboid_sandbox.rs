//! Bac a sable de Project Zomboid : le fichier que l'image ne sait pas ecrire.
//!
//! Population de zombies, rarete du butin, duree du jour, vitesse des morts :
//! rien de tout cela n'est une variable d'environnement. Ces reglages vivent
//! dans `SandboxVars.lua`, un fichier Lua de la sauvegarde, et l'image
//! `renegademaster/zomboid-dedicated-server` n'offre aucun moyen d'y toucher.
//!
//! Ce module produit ce fichier a partir des reglages saisis dans l'interface.
//!
//! # Trois choses a savoir avant d'y toucher
//!
//! **Le fichier ne prend effet qu'au demarrage.** Project Zomboid le lit une
//! fois, au lancement du serveur, et n'y revient jamais. Un reglage change en
//! cours de partie ne se verra qu'apres un redemarrage complet — l'interface
//! doit le dire, sinon on croit le changement perdu.
//!
//! **Les cles absentes prennent leur valeur par defaut.** Le jeu lit ce fichier
//! par-dessus ses propres defauts : on n'ecrit donc que ce que l'exploitant a
//! choisi. C'est ce qui permet d'exposer vingt reglages sur quatre-vingts sans
//! figer les soixante autres.
//!
//! **`VERSION` est obligatoire.** Sans elle, le jeu considere le fichier comme
//! issu d'une version anterieure et applique des conversions qui ne
//! correspondent a rien.

/// Version du format de bac a sable attendue par le jeu.
///
/// Elle accompagne TOUT fichier valide. Le jeu s'en sert pour convertir les
/// fichiers plus anciens ; l'omettre revient a se declarer d'une version
/// inconnue.
pub const SANDBOX_VERSION: u32 = 5;

/// Prefixe des cles de bac a sable dans la configuration d'un serveur.
///
/// IL LES DISTINGUE DES VARIABLES D'ENVIRONNEMENT. Toute cle de configuration
/// est normalement injectee dans le conteneur ; celles-ci n'ont rien a y faire
/// — l'image les ignorerait, et elles encombreraient l'inspection du conteneur
/// de reglages que rien ne lit. Le prefixe permet de les mettre de cote au
/// moment de composer l'environnement.
pub const PREFIXE_SANDBOX: &str = "SANDBOX_";

/// Ou vit le fichier, dans le volume de la sauvegarde.
///
/// Le nom du serveur en fait partie : chaque partie a le sien. C'est aussi
/// pourquoi renommer un serveur demarre une partie vierge — le jeu ne retrouve
/// plus ni sa sauvegarde ni son bac a sable.
pub fn chemin_du_fichier(nom_du_serveur: &str) -> String {
    format!("/home/steam/Zomboid/Server/{nom_du_serveur}_SandboxVars.lua")
}

/// Ou ranger un reglage dans le fichier.
///
/// Le format n'est pas plat : certains reglages vivent dans des sous-tables.
/// `ZombieLore` porte tout ce qui decrit les morts eux-memes — vitesse, force,
/// vue, ouie. Les ecrire a la racine ne produirait aucune erreur, et n'aurait
/// simplement aucun effet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Section {
    Racine,
    ZombieLore,
}

/// Comment ecrire la valeur en Lua.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeValeur {
    /// Ecrit tel quel : `Zombies = 3`.
    Nombre,
    /// Ecrit `true` ou `false`, jamais entre guillemets.
    Booleen,
}

/// Un reglage de bac a sable expose par l'interface.
#[derive(Debug, Clone, Copy)]
pub struct ReglageSandbox {
    /// Nom de la cle dans le fichier Lua, sans prefixe ni section.
    pub cle: &'static str,
    pub section: Section,
    pub type_valeur: TypeValeur,
}

/// Les reglages exposes.
///
/// QUATRE-VINGTS EXISTENT ; ON EN EXPOSE VINGT ET UN. Le choix porte sur ceux
/// qui changent reellement une partie et qu'une communaute discute avant de
/// lancer une soiree. Exposer les quatre-vingts donnerait un formulaire que
/// personne ne lirait, et chaque reglage de plus est une occasion de casser une
/// partie en cours.
///
/// Les autres restent accessibles en jeu, par le menu d'administration.
pub const REGLAGES: &[ReglageSandbox] = &[
    // ── Le monde ──
    r("Zombies", Section::Racine, TypeValeur::Nombre),
    r("Distribution", Section::Racine, TypeValeur::Nombre),
    r("DayLength", Section::Racine, TypeValeur::Nombre),
    r("StartYear", Section::Racine, TypeValeur::Nombre),
    r("StartMonth", Section::Racine, TypeValeur::Nombre),
    r("StartDay", Section::Racine, TypeValeur::Nombre),
    r("StartTime", Section::Racine, TypeValeur::Nombre),
    r("WaterShut", Section::Racine, TypeValeur::Nombre),
    r("ElecShut", Section::Racine, TypeValeur::Nombre),
    // ── Le butin ──
    r("FoodLoot", Section::Racine, TypeValeur::Nombre),
    r("WeaponLoot", Section::Racine, TypeValeur::Nombre),
    r("MedicalLoot", Section::Racine, TypeValeur::Nombre),
    r("OtherLoot", Section::Racine, TypeValeur::Nombre),
    // ── Le personnage ──
    r("XpMultiplier", Section::Racine, TypeValeur::Nombre),
    r(
        "ZombieAttractionMultiplier",
        Section::Racine,
        TypeValeur::Nombre,
    ),
    r("CharacterFreePoints", Section::Racine, TypeValeur::Nombre),
    r("NightDarkness", Section::Racine, TypeValeur::Nombre),
    // ── Les morts ──
    r("Speed", Section::ZombieLore, TypeValeur::Nombre),
    r("Strength", Section::ZombieLore, TypeValeur::Nombre),
    r("Toughness", Section::ZombieLore, TypeValeur::Nombre),
    r("Cognition", Section::ZombieLore, TypeValeur::Nombre),
    r("Memory", Section::ZombieLore, TypeValeur::Nombre),
    r("Sight", Section::ZombieLore, TypeValeur::Nombre),
    r("Hearing", Section::ZombieLore, TypeValeur::Nombre),
    r("Smell", Section::ZombieLore, TypeValeur::Nombre),
    r("ActiveOnly", Section::ZombieLore, TypeValeur::Nombre),
];

const fn r(cle: &'static str, section: Section, type_valeur: TypeValeur) -> ReglageSandbox {
    ReglageSandbox {
        cle,
        section,
        type_valeur,
    }
}

/// Le reglage portant cette cle, s'il est expose.
pub fn reglage(cle: &str) -> Option<&'static ReglageSandbox> {
    REGLAGES.iter().find(|r| r.cle == cle)
}

/// Cette cle de configuration est-elle un reglage de bac a sable ?
pub fn est_cle_sandbox(cle: &str) -> bool {
    cle.starts_with(PREFIXE_SANDBOX)
}

/// Compose le fichier `SandboxVars.lua` a partir de la configuration.
///
/// `None` quand aucun reglage n'a ete choisi : ecrire un fichier ne contenant
/// que `VERSION` remplacerait des defauts par eux-memes, mais surtout
/// deposerait un fichier la ou le jeu n'en attendait pas — autant ne rien
/// faire.
///
/// LES VALEURS ILLISIBLES SONT IGNOREES, PAS DEVINEES. Une valeur qu'on ne sait
/// pas lire ecrirait n'importe quoi dans le fichier, et le jeu refuserait de
/// charger la partie entiere. La sauter laisse le defaut du jeu s'appliquer :
/// un reglage sans effet vaut mieux qu'une partie qui ne demarre pas.
pub fn composer(config: &std::collections::HashMap<String, String>) -> Option<String> {
    let mut racine: Vec<(&str, String)> = Vec::new();
    let mut lore: Vec<(&str, String)> = Vec::new();

    // Ordre stable : l'ordre de `REGLAGES`, pas celui d'un `HashMap`. Deux
    // ecritures successives de la meme configuration doivent produire le meme
    // fichier, sans quoi toute comparaison devient impossible.
    for reglage in REGLAGES {
        let cle_config = format!("{PREFIXE_SANDBOX}{}", reglage.cle);
        let Some(brut) = config.get(&cle_config).map(|v| v.trim()) else {
            continue;
        };
        if brut.is_empty() {
            continue;
        }
        let Some(valeur) = valeur_lua(brut, reglage.type_valeur) else {
            continue;
        };
        match reglage.section {
            Section::Racine => racine.push((reglage.cle, valeur)),
            Section::ZombieLore => lore.push((reglage.cle, valeur)),
        }
    }

    if racine.is_empty() && lore.is_empty() {
        return None;
    }

    let mut sortie = String::from("SandboxVars = {\n");
    sortie.push_str(&format!("    VERSION = {SANDBOX_VERSION},\n"));
    for (cle, valeur) in &racine {
        sortie.push_str(&format!("    {cle} = {valeur},\n"));
    }
    if !lore.is_empty() {
        sortie.push_str("    ZombieLore = {\n");
        for (cle, valeur) in &lore {
            sortie.push_str(&format!("        {cle} = {valeur},\n"));
        }
        sortie.push_str("    },\n");
    }
    sortie.push_str("}\n");
    Some(sortie)
}

/// Traduit une valeur du formulaire en litteral Lua.
///
/// Rien n'est echappe ni mis entre guillemets : seuls des nombres et des
/// booleens sont acceptes, et tout le reste est refuse. C'est ce qui empeche
/// qu'une valeur saisie devienne du code Lua execute au chargement de la
/// partie.
fn valeur_lua(brut: &str, genre: TypeValeur) -> Option<String> {
    match genre {
        TypeValeur::Nombre => {
            let n: f64 = brut.parse().ok()?;
            if !n.is_finite() {
                return None;
            }
            // Un entier s'ecrit sans decimale : `Zombies = 3`, pas `3.0`. Le
            // jeu accepte les deux, mais le fichier se relit a l'oeil.
            if n.fract() == 0.0 && n.abs() < 1e15 {
                Some(format!("{}", n as i64))
            } else {
                Some(format!("{n}"))
            }
        }
        TypeValeur::Booleen => match brut.to_ascii_lowercase().as_str() {
            "true" | "1" | "oui" => Some("true".into()),
            "false" | "0" | "non" => Some("false".into()),
            _ => None,
        },
    }
}

#[cfg(test)]
#[path = "tests/zomboid_sandbox.rs"]
mod tests;
