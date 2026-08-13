//! Domaine pur de la Roue du Destin.
//!
//! 10 cases ponderees, chaque case a un effet coins (positif, negatif ou
//! neutre). RNG injectee via `spin_with_rng(rng)` -> testable/seedable.
//! Les cases par défaut et leurs probabilités constituent le comportement
//! standard de NEXUS. Une guilde peut les remplacer par une configuration
//! validée et persistée.

use chrono::DateTime;
use chrono::Utc;
use rand::distributions::WeightedIndex;
use rand::prelude::Distribution;
use rand::RngCore;
use uuid::Uuid;

/// Une case de la roue : identifiant stable, libelle affiche, payout fixe
/// en coins, et poids RNG (plus eleve = sort plus souvent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheelCase {
    pub key: &'static str,
    pub label: &'static str,
    /// Payout en coins. Negatif = perte. 0 = neutre (la case "blanche").
    pub payout: i64,
    pub weight: u32,
}

/// Les 10 cases historiques — coins-only. Somme des poids = 100 :
/// - 60% : petits gains / pertes (ambiance, presque neutres)
/// - 30% : gains moyens
/// - 9%  : gros gains ou grosses pertes
/// - 1%  : LICORNE jackpot rare
pub const WHEEL_CASES: &[WheelCase] = &[
    WheelCase {
        key: "blanche",
        label: "🌀 Blanche — Rien. Du tout.",
        payout: 0,
        weight: 25,
    },
    WheelCase {
        key: "pq",
        label: "🧻 PQ — +50c (collection)",
        payout: 50,
        weight: 20,
    },
    WheelCase {
        key: "sieste",
        label: "💤 Sieste — +200c",
        payout: 200,
        weight: 15,
    },
    WheelCase {
        key: "colis",
        label: "📦 Colis — +500c",
        payout: 500,
        weight: 12,
    },
    WheelCase {
        key: "trefle",
        label: "🍀 Trefle — +1000c",
        payout: 1000,
        weight: 10,
    },
    WheelCase {
        key: "couronne",
        label: "👑 Couronne — +1500c (Roi du jour)",
        payout: 1500,
        weight: 7,
    },
    WheelCase {
        key: "ruine",
        label: "💀 Ruine — -500c",
        payout: -500,
        weight: 5,
    },
    WheelCase {
        key: "jackpot",
        label: "🎰 Jackpot — +5000c",
        payout: 5000,
        weight: 3,
    },
    WheelCase {
        key: "bombe",
        label: "💣 Bombe — -2000c (apocalypse)",
        payout: -2000,
        weight: 2,
    },
    WheelCase {
        key: "licorne",
        label: "🦄 LICORNE — +10000c",
        payout: 10000,
        weight: 1,
    },
];

/// Une case telle qu'un serveur la definit.
///
/// Jumelle possedee de `WheelCase`, dont les champs sont `&'static str` parce
/// qu'ils viennent de constantes. Une case configurable vient de la base :
/// elle ne peut pas etre statique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheelCaseData {
    pub key: String,
    pub label: String,
    pub payout: i64,
    pub weight: u32,
}

impl From<&WheelCase> for WheelCaseData {
    fn from(c: &WheelCase) -> Self {
        Self {
            key: c.key.to_string(),
            label: c.label.to_string(),
            payout: c.payout,
            weight: c.weight,
        }
    }
}

/// Les 10 cases historiques, sous forme modifiable. C'est ce que voit un
/// serveur qui n'a jamais touche a sa roue — et le point de depart de
/// l'editeur, pour qu'on parte d'une roue jouable plutot que d'une page
/// blanche.
pub fn default_cases() -> Vec<WheelCaseData> {
    WHEEL_CASES.iter().map(WheelCaseData::from).collect()
}

/// Nombre de cases au-dela duquel la roue devient illisible : le site en
/// dessine les secteurs, et vingt-cinq etiquettes tiennent deja mal.
pub const MAX_CASES: usize = 25;

/// Une roue est-elle jouable ?
///
/// Verifie AVANT enregistrement : une roue invalide en base ferait echouer
/// tous les tirages du serveur, longtemps apres la saisie qui l'a cassee.
pub fn validate_cases(cases: &[WheelCaseData]) -> Result<(), String> {
    if cases.is_empty() {
        return Err("il faut au moins une case".into());
    }
    if cases.len() > MAX_CASES {
        return Err(format!("{MAX_CASES} cases au maximum"));
    }
    let mut vues = std::collections::HashSet::new();
    for case in cases {
        let key = case.key.trim();
        if key.is_empty() {
            return Err("chaque case doit avoir un identifiant".into());
        }
        if !vues.insert(key) {
            return Err(format!("identifiant en double : {key}"));
        }
        if case.label.trim().is_empty() {
            return Err(format!("la case {key} n'a pas de libelle"));
        }
        // Un poids nul n'est pas une case rare : c'est une case qui ne sort
        // JAMAIS. Autant la supprimer, sinon elle occupe l'ecran pour rien.
        if case.weight == 0 {
            return Err(format!(
                "la case {key} a un poids nul : elle ne sortirait jamais"
            ));
        }
    }
    Ok(())
}

/// Resultat d'un spin (pas encore persiste).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheelOutcome {
    pub case_index: usize,
    pub case: WheelCase,
}

/// Spin de la roue historique. RNG injectee -> seedable pour les tests.
/// Ne panique jamais : les poids constants sont non-nuls.
pub fn spin_with_rng(rng: &mut impl RngCore) -> WheelOutcome {
    let weights: Vec<u32> = WHEEL_CASES.iter().map(|c| c.weight).collect();
    let dist = WeightedIndex::new(&weights).expect("poids constants valides");
    let idx = dist.sample(rng);
    WheelOutcome {
        case_index: idx,
        case: WHEEL_CASES[idx].clone(),
    }
}

/// Resultat d'un tirage sur une roue quelconque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheelDraw {
    pub case_index: usize,
    pub case: WheelCaseData,
}

/// Spin sur les cases DU SERVEUR.
///
/// Retourne une erreur plutot que de paniquer : les poids viennent de la
/// base, donc d'une saisie humaine. `validate_cases` les a filtres a
/// l'enregistrement, mais une roue ecrite avant cette regle — ou a la main en
/// SQL — ne doit pas faire tomber le service.
pub fn spin_cases_with_rng(
    cases: &[WheelCaseData],
    rng: &mut impl RngCore,
) -> Result<WheelDraw, String> {
    validate_cases(cases)?;
    let weights: Vec<u32> = cases.iter().map(|c| c.weight).collect();
    let dist = WeightedIndex::new(&weights).map_err(|e| format!("poids invalides : {e}"))?;
    let idx = dist.sample(rng);
    Ok(WheelDraw {
        case_index: idx,
        case: cases[idx].clone(),
    })
}

/// True si la case est "memorable" (jackpot, licorne, bombe) -> mise en
/// avant dans l'embed de resultat.
pub fn is_memorable_case(key: &str) -> bool {
    matches!(key, "jackpot" | "licorne" | "bombe")
}

/// Entree persistee dans `nexus_wheel_spin_log`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheelSpin {
    pub id: Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub case_key: String,
    pub case_label: String,
    pub payout: i64,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
#[path = "tests/wheel.rs"]
mod tests;
