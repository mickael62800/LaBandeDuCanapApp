//! Ce que raconte un serveur de jeu pendant qu'il demarre.
//!
//! Un premier lancement telecharge plusieurs gigaoctets : le jeu par SteamCMD,
//! puis les mods du Workshop. Pendant ce temps l'interface n'affichait que
//! « Demarrage… », sans dire s'il restait dix secondes ou dix minutes — ni si
//! quelque chose etait bloque.
//!
//! Les journaux, eux, contiennent tout. Ce module les lit.
//!
//! # Trois formats, et un seul donne un pourcentage
//!
//! **SteamCMD se met a jour lui-meme**, avec son propre compteur :
//!
//! ```text
//! [ 42%] Downloading update (17983 of 40321 KB)...
//! ```
//!
//! **Le jeu se telecharge**, avec un etat et un pourcentage :
//!
//! ```text
//!  Update state (0x61) downloading, progress: 42.37 (3054321 / 7212532083)
//! ```
//!
//! **Les mods du Workshop**, eux, n'ont AUCUN pourcentage :
//!
//! ```text
//! Workshop: DownloadPending GetItemState()=NeedsUpdate|Downloading|... ID=2536865912
//! ```
//!
//! On ne peut donc pas afficher de barre pour cette derniere etape. Plutot
//! qu'inventer une progression, on dit quel mod est en cours et combien ont
//! deja ete vus — c'est la seule chose que les journaux permettent d'affirmer.

/// Etape du demarrage, dans l'ordre ou elles se succedent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtapeDemarrage {
    /// SteamCMD se met a jour avant de pouvoir travailler.
    MiseAJourSteamCmd,
    /// Reservation de l'espace disque, avant le telechargement lui-meme.
    Preparation,
    /// Telechargement du jeu.
    Telechargement,
    /// Verification des fichiers recus.
    Verification,
    /// Ecriture definitive sur le disque.
    Installation,
    /// Recuperation des mods du Workshop. Sans pourcentage.
    Mods,
}

impl EtapeDemarrage {
    /// Libelle affichable, en francais.
    pub fn libelle(self) -> &'static str {
        match self {
            Self::MiseAJourSteamCmd => "Mise a jour de SteamCMD",
            Self::Preparation => "Preparation de l'espace disque",
            Self::Telechargement => "Telechargement du jeu",
            Self::Verification => "Verification des fichiers",
            Self::Installation => "Installation",
            Self::Mods => "Telechargement des mods",
        }
    }
}

/// Ou en est le demarrage.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressionDemarrage {
    pub etape: EtapeDemarrage,
    /// 0 a 100. `None` pour les mods, qui n'en publient pas.
    pub pourcentage: Option<f32>,
    /// Octets recus et attendus, quand le journal les donne.
    pub octets: Option<(u64, u64)>,
    /// Identifiants Workshop deja apparus, dans l'ordre.
    ///
    /// Le dernier est celui en cours. Leur nombre dit combien de mods ont ete
    /// abordes — pas combien sont termines : le journal ne le dit pas.
    pub mods_vus: Vec<String>,
}

/// Lit la progression dans les dernieres lignes de journal.
///
/// `None` quand rien n'est reconnu : le serveur n'a pas encore parle, ou il a
/// depasse la phase de telechargement. Dans les deux cas, mieux vaut ne rien
/// afficher qu'un chiffre invente.
///
/// LA DERNIERE LIGNE RECONNUE FAIT FOI. Les journaux sont chronologiques, et
/// une phase remplace la precedente : lire la premiere afficherait
/// eternellement « mise a jour de SteamCMD » pendant que le jeu se telecharge.
pub fn lire_progression(lignes: &[String]) -> Option<ProgressionDemarrage> {
    let mut trouvee: Option<ProgressionDemarrage> = None;
    let mut mods_vus: Vec<String> = Vec::new();

    for ligne in lignes {
        // Les mods se comptent sur TOUTE la fenetre, pas seulement sur la
        // derniere ligne : chaque identifiant apparait des dizaines de fois, et
        // seul leur ensemble dit combien de mods ont ete abordes.
        if let Some(id) = identifiant_workshop(ligne) {
            if !mods_vus.iter().any(|vu| vu == id) {
                mods_vus.push(id.to_string());
            }
            trouvee = Some(ProgressionDemarrage {
                etape: EtapeDemarrage::Mods,
                pourcentage: None,
                octets: None,
                mods_vus: Vec::new(),
            });
            continue;
        }
        if let Some(p) = lire_etat_steam(ligne) {
            trouvee = Some(p);
            continue;
        }
        if let Some(p) = lire_maj_steamcmd(ligne) {
            trouvee = Some(p);
        }
    }

    trouvee.map(|mut p| {
        p.mods_vus = mods_vus;
        p
    })
}

/// `Workshop: ... ID=2536865912`
fn identifiant_workshop(ligne: &str) -> Option<&str> {
    if !ligne.contains("Workshop:") {
        return None;
    }
    let apres = ligne.rsplit_once("ID=")?.1.trim();
    let id: &str = apres
        .split(|c: char| !c.is_ascii_digit())
        .next()
        .filter(|s| !s.is_empty())?;
    Some(id)
}

/// ` Update state (0x61) downloading, progress: 42.37 (3054321 / 7212532083)`
///
/// L'etat hexadecimal est plus sur que le mot qui le suit : celui-ci change
/// avec la langue et les versions de SteamCMD, le code non.
fn lire_etat_steam(ligne: &str) -> Option<ProgressionDemarrage> {
    let apres_etat = ligne.split_once("Update state (")?.1;
    let (code, reste) = apres_etat.split_once(')')?;

    let etape = match code.trim() {
        "0x3" | "0x11" => EtapeDemarrage::Preparation,
        "0x61" => EtapeDemarrage::Telechargement,
        "0x81" => EtapeDemarrage::Verification,
        "0x101" => EtapeDemarrage::Installation,
        // `0x0 unknown, progress: 0.00 (0 / 0)` clot la sequence : ce n'est pas
        // une etape, et l'afficher a zero pour cent ferait reculer la barre.
        _ => return None,
    };

    let apres_progress = reste.split_once("progress:")?.1;
    let pourcentage: f32 = apres_progress
        .split_whitespace()
        .next()?
        .trim_end_matches(',')
        .parse()
        .ok()?;

    let octets = apres_progress
        .split_once('(')
        .and_then(|(_, dedans)| dedans.split_once(')'))
        .and_then(|(dedans, _)| dedans.split_once('/'))
        .and_then(|(recus, total)| {
            Some((
                recus.trim().parse::<u64>().ok()?,
                total.trim().parse::<u64>().ok()?,
            ))
        })
        // `(0 / 0)` n'est pas une taille : l'afficher donnerait « 0 sur 0 ».
        .filter(|(_, total)| *total > 0);

    Some(ProgressionDemarrage {
        etape,
        pourcentage: Some(pourcentage.clamp(0.0, 100.0)),
        octets,
        mods_vus: Vec::new(),
    })
}

/// `[ 42%] Downloading update (17983 of 40321 KB)...`
fn lire_maj_steamcmd(ligne: &str) -> Option<ProgressionDemarrage> {
    if !ligne.contains("Downloading update") {
        return None;
    }
    let dedans = ligne.split_once('[')?.1.split_once(']')?.0.trim();
    // `[----]` precede le premier chiffre : c'est une progression inconnue, pas
    // zero. La donner comme zero ferait sauter la barre en arriere au premier
    // rafraichissement.
    let pourcentage: f32 = dedans.trim_end_matches('%').trim().parse().ok()?;

    let octets = ligne
        .split_once('(')
        .and_then(|(_, d)| d.split_once(" KB)"))
        .and_then(|(d, _)| d.split_once(" of "))
        .and_then(|(recus, total)| {
            Some((
                recus.trim().parse::<u64>().ok()? * 1024,
                total.trim().parse::<u64>().ok()? * 1024,
            ))
        })
        .filter(|(_, total)| *total > 0);

    Some(ProgressionDemarrage {
        etape: EtapeDemarrage::MiseAJourSteamCmd,
        pourcentage: Some(pourcentage.clamp(0.0, 100.0)),
        octets,
        mods_vus: Vec::new(),
    })
}

#[cfg(test)]
#[path = "tests/progression_demarrage.rs"]
mod tests;
