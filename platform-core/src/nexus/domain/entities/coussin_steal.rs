//! Fouille sous les coussins : la victime peut se defendre.
//!
//! Le vol se jouait a pile ou face — un pourcentage fixe, sans que la cible
//! puisse quoi que ce soit. Perdre sept fois sur dix sans avoir eu son mot a
//! dire n'est pas un jeu, c'est une taxe.
//!
//! Le modele repris ici est celui de l'ancien Coup de Coude : la tentative
//! ouvre une fenetre pendant laquelle la victime peut reagir. Si elle serre
//! les coussins a temps, elle garde toute sa defense ; si elle ne dit rien,
//! elle encaisse un malus et le voleur passe beaucoup plus facilement.
//!
//! Deux jets opposes plutot qu'un pourcentage : le hasard reste, mais la
//! classe, la defense et surtout la reaction de la cible pesent dessus.

use serde::{Deserialize, Serialize};

/// Faces du de utilise par les deux camps.
pub const STEAL_DICE_FACES: i32 = 20;

/// Bonus de jet du Piegeur : il sait ou les autres planquent.
pub const PIEGEUR_STEAL_BONUS: i32 = 4;

/// Diviseur du bonus defensif tire de la statistique DEF.
pub const DEF_BONUS_DIVISOR: i32 = 10;

/// Etat d'une tentative de fouille.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StealAttemptStatus {
    /// La fenetre de defense court encore.
    Pending,
    /// Resolue : la victime a reagi, ou la fenetre s'est fermee.
    Resolved,
}

/// Ce que la victime a fait de sa fenetre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Defense {
    /// Elle a serre les coussins a temps : defense pleine.
    Reacted,
    /// Elle n'a rien fait : malus de vigilance.
    Absent,
}

/// Detail d'un jet, pour que le message explique le resultat au lieu de
/// l'annoncer. Un joueur qui perd doit pouvoir voir POURQUOI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StealRoll {
    pub thief_die: i32,
    pub thief_bonus: i32,
    pub victim_die: i32,
    /// Bonus defensif effectivement applique, malus d'absence deja deduit.
    pub victim_bonus: i32,
    /// Malus retire faute de reaction. Zero si la victime a reagi.
    pub absence_malus: i32,
    pub thief_total: i32,
    pub victim_total: i32,
    pub success: bool,
}

/// Resout une tentative a partir des deux des et de l'etat de la cible.
///
/// Egalite : la victime l'emporte. Un vol doit se meriter strictement, et le
/// doute profite a celui qui se fait fouiller.
pub fn resolve_steal(
    thief_die: i32,
    victim_die: i32,
    is_piegeur: bool,
    victim_def: i32,
    defense: Defense,
    absence_malus: i32,
) -> StealRoll {
    let thief_bonus = if is_piegeur { PIEGEUR_STEAL_BONUS } else { 0 };

    // Le bonus defensif ne descend jamais sous zero : une cible sans defense
    // subit le malus dans sa pleine mesure, mais ne se met pas a AIDER le
    // voleur, ce qui n'aurait aucun sens.
    let base_defense = (victim_def / DEF_BONUS_DIVISOR).max(0);
    let malus = match defense {
        Defense::Reacted => 0,
        Defense::Absent => absence_malus.max(0),
    };
    let victim_bonus = (base_defense - malus).max(0);

    let thief_total = thief_die + thief_bonus;
    let victim_total = victim_die + victim_bonus;

    StealRoll {
        thief_die,
        thief_bonus,
        victim_die,
        victim_bonus,
        absence_malus: malus,
        thief_total,
        victim_total,
        success: thief_total > victim_total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reagir_a_temps_change_le_resultat() {
        // Memes des, meme joueur : seule la reaction de la cible differe.
        let attentive = resolve_steal(12, 11, false, 30, Defense::Reacted, 8);
        let absente = resolve_steal(12, 11, false, 30, Defense::Absent, 8);

        // DEF 30 -> +3. 11+3 = 14 > 12 : le vol echoue.
        assert!(!attentive.success);
        // Sans reaction, le bonus tombe a 0 : 11 < 12, le vol passe.
        assert!(absente.success);
    }

    #[test]
    fn le_piegeur_garde_son_avantage() {
        let ordinaire = resolve_steal(10, 12, false, 0, Defense::Reacted, 8);
        let piegeur = resolve_steal(10, 12, true, 0, Defense::Reacted, 8);
        assert!(!ordinaire.success);
        // 10 + 4 = 14 > 12.
        assert!(piegeur.success);
        assert_eq!(piegeur.thief_bonus, PIEGEUR_STEAL_BONUS);
    }

    #[test]
    fn egalite_profite_a_la_victime() {
        let roll = resolve_steal(10, 10, false, 0, Defense::Reacted, 8);
        assert_eq!(roll.thief_total, roll.victim_total);
        assert!(!roll.success, "un vol doit se meriter strictement");
    }

    #[test]
    fn une_cible_sans_defense_naide_jamais_le_voleur() {
        // DEF 0 et malus 8 : le bonus s'arrete a zero, il ne devient pas
        // negatif — sinon ne pas reagir reviendrait a tendre son porte-monnaie.
        let roll = resolve_steal(5, 5, false, 0, Defense::Absent, 8);
        assert_eq!(roll.victim_bonus, 0);
        assert_eq!(roll.victim_total, 5);
        assert!(!roll.success);
    }

    #[test]
    fn le_detail_du_jet_est_reconstituable() {
        // Le message doit pouvoir expliquer le resultat, pas seulement
        // l'annoncer : chaque terme de l'addition reste lisible.
        let roll = resolve_steal(14, 9, true, 50, Defense::Absent, 8);
        assert_eq!(roll.thief_die, 14);
        assert_eq!(roll.thief_bonus, 4);
        assert_eq!(roll.victim_die, 9);
        assert_eq!(roll.absence_malus, 8);
        // DEF 50 -> +5, moins 8 -> plancher a 0.
        assert_eq!(roll.victim_bonus, 0);
        assert_eq!(roll.thief_total, 18);
        assert_eq!(roll.victim_total, 9);
        assert!(roll.success);
    }

    #[test]
    fn une_grosse_defense_resiste_meme_absente() {
        // DEF 200 -> +20, moins 8 -> +12. Se blinder garde son interet meme
        // quand on ne peut pas reagir a temps.
        let roll = resolve_steal(15, 10, false, 200, Defense::Absent, 8);
        assert_eq!(roll.victim_bonus, 12);
        assert!(!roll.success);
    }
}
