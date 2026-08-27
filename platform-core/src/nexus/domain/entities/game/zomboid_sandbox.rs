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
//! choisi. Un formulaire laisse vide ne produit aucun fichier, et la partie
//! demarre exactement comme sans ce module.
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
    /// Ce qui decrit les morts eux-memes : vitesse, force, sens, transmission.
    ZombieLore,
    /// La densite et la reapparition : ce qui decide combien il y en a.
    ZombieConfig,
    /// Ce que le joueur voit de la carte.
    Map,
}

impl Section {
    /// Nom de la sous-table dans le fichier, `None` pour la racine.
    fn table(self) -> Option<&'static str> {
        match self {
            Self::Racine => None,
            Self::ZombieLore => Some("ZombieLore"),
            Self::ZombieConfig => Some("ZombieConfig"),
            Self::Map => Some("Map"),
        }
    }
}

/// Ordre d'ecriture des sous-tables.
///
/// Fige : deux ecritures de la meme configuration doivent produire le meme
/// fichier, sinon toute comparaison devient impossible.
const SOUS_TABLES: [Section; 3] = [Section::Map, Section::ZombieLore, Section::ZombieConfig];

/// Comment ecrire la valeur en Lua.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeValeur {
    /// Ecrit tel quel : `Zombies = 3`.
    Nombre,
    /// Ecrit `true` ou `false`, jamais entre guillemets.
    Booleen,
    /// Chaine entre guillemets.
    ///
    /// UN SEUL REGLAGE VANILLA EN EST UN : `WorldItemRemovalList`, la liste des
    /// objets nettoyes au sol. Comme le texte finit entre guillemets dans du
    /// Lua, le jeu de caracteres est reduit a ce que cette liste peut contenir
    /// — lettres, chiffres, point, virgule, tiret, souligne. Ni guillemet, ni
    /// contre-oblique, ni retour a la ligne ne peuvent donc en sortir.
    Texte,
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
/// TOUS LES REGLAGES DU JEU DE BASE : cent trente, en quatre tables Lua
///
/// CEUX DES MODS EN SONT EXCLUS. Un `SandboxVars.lua` reel en contient souvent
/// plus que le jeu lui-meme — un seul mod d'anticorps y ajoute soixante-dix
/// lignes. Les exposer donnerait un formulaire ou la plupart des champs
/// n'auraient aucun effet chez qui n'a pas le mod correspondant.
///
/// La liste est GENEREE, pas saisie a la main : cent trente noms recopies deux
/// fois — ici et dans le formulaire — auraient produit des ecarts qu'aucun
/// message d'erreur n'aurait signales, puisqu'une cle inconnue du jeu est
/// simplement ignoree.
pub const REGLAGES: &[ReglageSandbox] = &[
    r("Zombies", Section::Racine, TypeValeur::Nombre),
    r("Distribution", Section::Racine, TypeValeur::Nombre),
    r("DayLength", Section::Racine, TypeValeur::Nombre),
    r("StartYear", Section::Racine, TypeValeur::Nombre),
    r("StartMonth", Section::Racine, TypeValeur::Nombre),
    r("StartDay", Section::Racine, TypeValeur::Nombre),
    r("StartTime", Section::Racine, TypeValeur::Nombre),
    r("WaterShut", Section::Racine, TypeValeur::Nombre),
    r("ElecShut", Section::Racine, TypeValeur::Nombre),
    r("WaterShutModifier", Section::Racine, TypeValeur::Nombre),
    r("ElecShutModifier", Section::Racine, TypeValeur::Nombre),
    r("Temperature", Section::Racine, TypeValeur::Nombre),
    r("Rain", Section::Racine, TypeValeur::Nombre),
    r("ErosionSpeed", Section::Racine, TypeValeur::Nombre),
    r("ErosionDays", Section::Racine, TypeValeur::Nombre),
    r("TimeSinceApo", Section::Racine, TypeValeur::Nombre),
    r("NatureAbundance", Section::Racine, TypeValeur::Nombre),
    r("PlantResilience", Section::Racine, TypeValeur::Nombre),
    r("PlantAbundance", Section::Racine, TypeValeur::Nombre),
    r("Farming", Section::Racine, TypeValeur::Nombre),
    r("CompostTime", Section::Racine, TypeValeur::Nombre),
    r("NightDarkness", Section::Racine, TypeValeur::Nombre),
    r("NightLength", Section::Racine, TypeValeur::Nombre),
    r("MaxFogIntensity", Section::Racine, TypeValeur::Nombre),
    r("MaxRainFxIntensity", Section::Racine, TypeValeur::Nombre),
    r("EnableSnowOnGround", Section::Racine, TypeValeur::Booleen),
    r("Alarm", Section::Racine, TypeValeur::Nombre),
    r("LockedHouses", Section::Racine, TypeValeur::Nombre),
    r("Helicopter", Section::Racine, TypeValeur::Nombre),
    r("MetaEvent", Section::Racine, TypeValeur::Nombre),
    r("SleepingEvent", Section::Racine, TypeValeur::Nombre),
    r("SurvivorHouseChance", Section::Racine, TypeValeur::Nombre),
    r("VehicleStoryChance", Section::Racine, TypeValeur::Nombre),
    r("ZoneStoryChance", Section::Racine, TypeValeur::Nombre),
    r("AnnotatedMapChance", Section::Racine, TypeValeur::Nombre),
    r("FoodLoot", Section::Racine, TypeValeur::Nombre),
    r("CannedFoodLoot", Section::Racine, TypeValeur::Nombre),
    r("LiteratureLoot", Section::Racine, TypeValeur::Nombre),
    r("SurvivalGearsLoot", Section::Racine, TypeValeur::Nombre),
    r("MedicalLoot", Section::Racine, TypeValeur::Nombre),
    r("WeaponLoot", Section::Racine, TypeValeur::Nombre),
    r("RangedWeaponLoot", Section::Racine, TypeValeur::Nombre),
    r("AmmoLoot", Section::Racine, TypeValeur::Nombre),
    r("MechanicsLoot", Section::Racine, TypeValeur::Nombre),
    r("OtherLoot", Section::Racine, TypeValeur::Nombre),
    r("LootRespawn", Section::Racine, TypeValeur::Nombre),
    r(
        "SeenHoursPreventLootRespawn",
        Section::Racine,
        TypeValeur::Nombre,
    ),
    r(
        "HoursForWorldItemRemoval",
        Section::Racine,
        TypeValeur::Nombre,
    ),
    r("WorldItemRemovalList", Section::Racine, TypeValeur::Texte),
    r(
        "ItemRemovalListBlacklistToggle",
        Section::Racine,
        TypeValeur::Booleen,
    ),
    r(
        "DaysForRottenFoodRemoval",
        Section::Racine,
        TypeValeur::Nombre,
    ),
    r("FoodRotSpeed", Section::Racine, TypeValeur::Nombre),
    r("FridgeFactor", Section::Racine, TypeValeur::Nombre),
    r("Nutrition", Section::Racine, TypeValeur::Booleen),
    r("StatsDecrease", Section::Racine, TypeValeur::Nombre),
    r("XpMultiplier", Section::Racine, TypeValeur::Nombre),
    r(
        "XpMultiplierAffectsPassive",
        Section::Racine,
        TypeValeur::Booleen,
    ),
    r("CharacterFreePoints", Section::Racine, TypeValeur::Nombre),
    r(
        "ConstructionBonusPoints",
        Section::Racine,
        TypeValeur::Nombre,
    ),
    r(
        "ZombieAttractionMultiplier",
        Section::Racine,
        TypeValeur::Nombre,
    ),
    r("StarterKit", Section::Racine, TypeValeur::Booleen),
    r("AllClothesUnlocked", Section::Racine, TypeValeur::Booleen),
    r("InjurySeverity", Section::Racine, TypeValeur::Nombre),
    r("BoneFracture", Section::Racine, TypeValeur::Booleen),
    r("EndRegen", Section::Racine, TypeValeur::Nombre),
    r("ClothingDegradation", Section::Racine, TypeValeur::Nombre),
    r("RearVulnerability", Section::Racine, TypeValeur::Nombre),
    r("MultiHitZombies", Section::Racine, TypeValeur::Booleen),
    r("AttackBlockMovements", Section::Racine, TypeValeur::Booleen),
    r("EnablePoisoning", Section::Racine, TypeValeur::Nombre),
    r(
        "EnableTaintedWaterText",
        Section::Racine,
        TypeValeur::Booleen,
    ),
    r("BloodLevel", Section::Racine, TypeValeur::Nombre),
    r("HoursForCorpseRemoval", Section::Racine, TypeValeur::Nombre),
    r(
        "DecayingCorpseHealthImpact",
        Section::Racine,
        TypeValeur::Nombre,
    ),
    r("MaggotSpawn", Section::Racine, TypeValeur::Nombre),
    r("FireSpread", Section::Racine, TypeValeur::Booleen),
    r("LightBulbLifespan", Section::Racine, TypeValeur::Nombre),
    r("GeneratorSpawning", Section::Racine, TypeValeur::Nombre),
    r(
        "GeneratorFuelConsumption",
        Section::Racine,
        TypeValeur::Nombre,
    ),
    r(
        "AllowExteriorGenerator",
        Section::Racine,
        TypeValeur::Booleen,
    ),
    r("EnableVehicles", Section::Racine, TypeValeur::Booleen),
    r("VehicleEasyUse", Section::Racine, TypeValeur::Booleen),
    r("CarSpawnRate", Section::Racine, TypeValeur::Nombre),
    r("ChanceHasGas", Section::Racine, TypeValeur::Nombre),
    r("InitialGas", Section::Racine, TypeValeur::Nombre),
    r("FuelStationGas", Section::Racine, TypeValeur::Nombre),
    r("CarGasConsumption", Section::Racine, TypeValeur::Nombre),
    r("LockedCar", Section::Racine, TypeValeur::Nombre),
    r("CarGeneralCondition", Section::Racine, TypeValeur::Nombre),
    r("CarDamageOnImpact", Section::Racine, TypeValeur::Nombre),
    r(
        "DamageToPlayerFromHitByACar",
        Section::Racine,
        TypeValeur::Nombre,
    ),
    r(
        "PlayerDamageFromCrash",
        Section::Racine,
        TypeValeur::Booleen,
    ),
    r("TrafficJam", Section::Racine, TypeValeur::Booleen),
    r("CarAlarm", Section::Racine, TypeValeur::Nombre),
    r("SirenShutoffHours", Section::Racine, TypeValeur::Nombre),
    r(
        "RecentlySurvivorVehicles",
        Section::Racine,
        TypeValeur::Nombre,
    ),
    r("Speed", Section::ZombieLore, TypeValeur::Nombre),
    r("Strength", Section::ZombieLore, TypeValeur::Nombre),
    r("Toughness", Section::ZombieLore, TypeValeur::Nombre),
    r("Transmission", Section::ZombieLore, TypeValeur::Nombre),
    r("Mortality", Section::ZombieLore, TypeValeur::Nombre),
    r("Reanimate", Section::ZombieLore, TypeValeur::Nombre),
    r("Cognition", Section::ZombieLore, TypeValeur::Nombre),
    r("CrawlUnderVehicle", Section::ZombieLore, TypeValeur::Nombre),
    r("Memory", Section::ZombieLore, TypeValeur::Nombre),
    r("Sight", Section::ZombieLore, TypeValeur::Nombre),
    r("Hearing", Section::ZombieLore, TypeValeur::Nombre),
    r("ThumpNoChasing", Section::ZombieLore, TypeValeur::Booleen),
    r(
        "ThumpOnConstruction",
        Section::ZombieLore,
        TypeValeur::Booleen,
    ),
    r("ActiveOnly", Section::ZombieLore, TypeValeur::Nombre),
    r(
        "TriggerHouseAlarm",
        Section::ZombieLore,
        TypeValeur::Booleen,
    ),
    r("ZombiesDragDown", Section::ZombieLore, TypeValeur::Booleen),
    r(
        "ZombiesFenceLunge",
        Section::ZombieLore,
        TypeValeur::Booleen,
    ),
    r("DisableFakeDead", Section::ZombieLore, TypeValeur::Nombre),
    r(
        "PopulationMultiplier",
        Section::ZombieConfig,
        TypeValeur::Nombre,
    ),
    r(
        "PopulationStartMultiplier",
        Section::ZombieConfig,
        TypeValeur::Nombre,
    ),
    r(
        "PopulationPeakMultiplier",
        Section::ZombieConfig,
        TypeValeur::Nombre,
    ),
    r(
        "PopulationPeakDay",
        Section::ZombieConfig,
        TypeValeur::Nombre,
    ),
    r("RespawnHours", Section::ZombieConfig, TypeValeur::Nombre),
    r(
        "RespawnUnseenHours",
        Section::ZombieConfig,
        TypeValeur::Nombre,
    ),
    r(
        "RespawnMultiplier",
        Section::ZombieConfig,
        TypeValeur::Nombre,
    ),
    r(
        "RedistributeHours",
        Section::ZombieConfig,
        TypeValeur::Nombre,
    ),
    r(
        "FollowSoundDistance",
        Section::ZombieConfig,
        TypeValeur::Nombre,
    ),
    r("RallyGroupSize", Section::ZombieConfig, TypeValeur::Nombre),
    r(
        "RallyTravelDistance",
        Section::ZombieConfig,
        TypeValeur::Nombre,
    ),
    r(
        "RallyGroupSeparation",
        Section::ZombieConfig,
        TypeValeur::Nombre,
    ),
    r(
        "RallyGroupRadius",
        Section::ZombieConfig,
        TypeValeur::Nombre,
    ),
    r("AllowMiniMap", Section::Map, TypeValeur::Booleen),
    r("AllowWorldMap", Section::Map, TypeValeur::Booleen),
    r("MapAllKnown", Section::Map, TypeValeur::Booleen),
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
    // Ordre stable : celui de `REGLAGES`, jamais celui d'un `HashMap`. Deux
    // ecritures successives de la meme configuration doivent produire le meme
    // fichier, sans quoi toute comparaison devient impossible.
    let retenus: Vec<(&ReglageSandbox, String)> = REGLAGES
        .iter()
        .filter_map(|reglage| {
            let brut = config
                .get(&format!("{PREFIXE_SANDBOX}{}", reglage.cle))
                .map(|v| v.trim())
                .filter(|v| !v.is_empty())?;
            valeur_lua(brut, reglage.type_valeur).map(|valeur| (reglage, valeur))
        })
        .collect();

    if retenus.is_empty() {
        return None;
    }

    let mut sortie = String::from("SandboxVars = {\n");
    sortie.push_str(&format!("    VERSION = {SANDBOX_VERSION},\n"));

    for (reglage, valeur) in retenus.iter().filter(|(r, _)| r.section == Section::Racine) {
        sortie.push_str(&format!("    {} = {valeur},\n", reglage.cle));
    }

    for section in SOUS_TABLES {
        let dedans: Vec<_> = retenus
            .iter()
            .filter(|(r, _)| r.section == section)
            .collect();
        if dedans.is_empty() {
            continue;
        }
        // `table()` ne rend `None` que pour la racine, deja traitee.
        let Some(nom) = section.table() else { continue };
        sortie.push_str(&format!("    {nom} = {{\n"));
        for (reglage, valeur) in dedans {
            sortie.push_str(&format!("        {} = {valeur},\n", reglage.cle));
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
        TypeValeur::Texte => {
            // RESTREINT PLUTOT QU'ECHAPPE. Le texte finit entre guillemets dans
            // du Lua ; echapper correctement demanderait de traiter guillemets,
            // contre-obliques, retours a la ligne et sequences longues, et une
            // seule omission suffirait a rendre le fichier executable.
            //
            // Le seul reglage vanilla de ce type est une liste d'identifiants
            // d'objets (`Base.Hat,Base.Glasses`) : ce jeu de caracteres lui
            // suffit largement, et rien de dangereux n'y entre.
            let propre = brut.chars().all(|c| {
                c.is_ascii_alphanumeric() || matches!(c, '.' | ',' | '-' | '_' | ' ' | ';')
            });
            if !propre || brut.len() > 500 {
                return None;
            }
            Some(format!("\"{brut}\""))
        }
    }
}

#[cfg(test)]
#[path = "tests/zomboid_sandbox.rs"]
mod tests;
