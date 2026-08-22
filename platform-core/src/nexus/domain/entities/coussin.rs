//! Regles pures de Coussin Piégé.
//!
//! Le jeu : on planque un coussin piégé sur le canapé et on s'arrange pour
//! que le voisin s'assoie dessus. Ce qu'on perd n'est pas de la vie, c'est du
//! CONFORT — a zero on ne meurt pas, on se leve du canape.
//!
//! Aucun acces DB/Discord : ces fonctions sont reutilisables par l'API et
//! testables sans infrastructure.

use serde::{Deserialize, Serialize};

/// Plafond historique. Reste le DEFAUT, mais n'est plus la loi : les
/// fonctions de progression prennent desormais le plafond en parametre, pour
/// qu'un serveur puisse allonger ou raccourcir la course.
pub const MAX_LEVEL: i32 = 25;

/// Les quatre manieres d'occuper un canape.
///
/// Chacune reprend un archetype reel de la bande : celui qui se laisse tomber
/// sans regarder, celui qui bondit, celui qui prepare son coup, et celui qui
/// ne bougera plus de la soiree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayerClass {
    /// Se laisse tomber de tout son poids. Fait mal, encaisse mal.
    Ecraseur,
    /// Rebondit d'un accoudoir a l'autre. Vif, mais leger.
    Ressort,
    /// Planque les coussins. Frappe correctement et chipe mieux que les autres.
    Piegeur,
    /// Roule dans la couette et ne bouge plus. Encaisse tout.
    Couette,
}

impl PlayerClass {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "ecraseur" => Some(Self::Ecraseur),
            "ressort" => Some(Self::Ressort),
            "piegeur" => Some(Self::Piegeur),
            "couette" => Some(Self::Couette),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ecraseur => "ecraseur",
            Self::Ressort => "ressort",
            Self::Piegeur => "piegeur",
            Self::Couette => "couette",
        }
    }

    /// (Impact, Moelleux) de depart, valeurs HISTORIQUES.
    ///
    /// Restent le defaut, mais ne sont plus la loi : un serveur peut les
    /// redefinir via `ClassRules`. Gardees ici parce qu'elles servent de
    /// point de depart a la configuration — et parce qu'un appelant sans
    /// configuration sous la main (tests, outillage) doit pouvoir jouer.
    pub fn base_stats(self) -> (i32, i32) {
        match self {
            Self::Ecraseur => (25, 8),
            Self::Ressort => (12, 18),
            Self::Piegeur => (18, 14),
            Self::Couette => (8, 25),
        }
    }

    pub fn growth(self) -> (i32, i32) {
        match self {
            Self::Ecraseur => (4, 1),
            Self::Ressort => (2, 3),
            Self::Piegeur => (3, 2),
            Self::Couette => (1, 4),
        }
    }

    /// Les quatre classes, pour parcourir sans rien oublier.
    pub const ALL: [PlayerClass; 4] = [Self::Ecraseur, Self::Ressort, Self::Piegeur, Self::Couette];

    pub fn label(self) -> &'static str {
        match self {
            Self::Ecraseur => "🪑 Écraseur",
            Self::Ressort => "🤸 Ressort",
            Self::Piegeur => "🪡 Piégeur",
            Self::Couette => "🛌 Couette",
        }
    }

    /// Une phrase pour choisir sans lire un tableau de stats.
    pub fn pitch(self) -> &'static str {
        match self {
            Self::Ecraseur => "Tu t'assois sans regarder. Ça fait mal aux deux.",
            Self::Ressort => "Tu rebondis d'un accoudoir a l'autre.",
            Self::Piegeur => "Tu places les coussins. Et tu fouilles sous les autres.",
            Self::Couette => "Tu es dans la couette. Tu n'en sortiras pas.",
        }
    }
}

/// XP cumulée requise pour atteindre un niveau, plafond donne.
///
/// Le plafond est un PARAMETRE et non plus une constante : c'est le reglage
/// qui decide de la longueur de la course. Il est borne a 1..=200 ici pour
/// qu'une valeur absurde en base ne produise pas une courbe infinie.
pub fn xp_for_level_capped(level: i32, max_level: i32) -> i64 {
    let max_level = max_level.clamp(1, 200);
    let level = level.clamp(1, max_level) as i64;
    50 * level * (level + 1)
}

pub fn level_for_xp_capped(xp: i64, max_level: i32) -> i32 {
    let max_level = max_level.clamp(1, 200);
    (1..=max_level)
        .rev()
        .find(|&level| xp >= xp_for_level_capped(level, max_level))
        .unwrap_or(1)
}

/// Variantes au plafond historique, gardees pour les appelants qui n'ont pas
/// de configuration sous la main (tests, outillage).
pub fn xp_for_level(level: i32) -> i64 {
    xp_for_level_capped(level, MAX_LEVEL)
}

pub fn level_for_xp(xp: i64) -> i32 {
    level_for_xp_capped(xp, MAX_LEVEL)
}

/// Les six titres, du bout d'accoudoir a la place du milieu.
pub const TITLES: [&str; 6] = [
    "Bout d'Accoudoir",
    "Squatteur",
    "Poseur de Coussins",
    "Gardien de la Telecommande",
    "Roi du Canape",
    "Le Canape, c'est Lui",
];

/// Le titre raconte la place gagnee sur le canape, pas un grade militaire :
/// on commence sur l'accoudoir et on finit au milieu, telecommande en main.
///
/// Les paliers sont PROPORTIONNELS au plafond : sur une echelle de 100
/// niveaux, des bandes fixes de cinq niveaux donneraient le dernier titre des
/// le niveau 25, puis soixante-quinze niveaux sans rien a gagner.
pub fn title_for_level_capped(level: i32, max_level: i32) -> &'static str {
    let max_level = max_level.clamp(1, 200);
    let level = level.clamp(1, max_level);
    // Le dernier titre est reserve au plafond : c'est ce qui en fait une fin
    // de course, et non un palier de plus.
    if level >= max_level {
        return TITLES[TITLES.len() - 1];
    }
    let bandes = (TITLES.len() - 1) as i32;
    let index = ((level - 1) * bandes / max_level.max(1)).clamp(0, bandes - 1);
    TITLES[index as usize]
}

pub fn title_for_level(level: i32) -> &'static str {
    title_for_level_capped(level, MAX_LEVEL)
}

/// Handicap applique au plus haut niveau, ou `None` si l'ecart interdit la
/// bagarre.
///
/// `gap_max` est l'ecart TOLERE, configurable : au-dela, on refuse. Les
/// paliers intermediaires restent proportionnels a ce plafond, sinon
/// l'allonger creerait un trou ou personne n'est penalise.
pub fn matchmaking_handicap_capped(
    first_level: i32,
    second_level: i32,
    gap_max: i32,
) -> Option<f32> {
    let gap_max = gap_max.max(0);
    let ecart = (first_level - second_level).unsigned_abs() as i32;
    if ecart > gap_max {
        return None;
    }
    // Trois tiers : intact, puis -20 %, puis -40 %. A gap_max = 9 on retrouve
    // exactement les paliers historiques 0-2 / 3-5 / 6-9.
    let tiers = (gap_max as f32 / 3.0).max(1.0);
    match ecart as f32 {
        e if e <= tiers => Some(1.0),
        e if e <= tiers * 2.0 => Some(0.8),
        _ => Some(0.6),
    }
}

pub fn matchmaking_handicap(first_level: i32, second_level: i32) -> Option<f32> {
    matchmaking_handicap_capped(first_level, second_level, 9)
}

/// Statistiques d'une classe : depart et croissance par niveau.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassStats {
    pub atk: i32,
    pub def: i32,
    pub atk_growth: i32,
    pub def_growth: i32,
}

/// Tout ce qui fait l'equilibre des classes et des coups.
///
/// Reunies dans UNE structure passee en donnees : c'est la condition pour que
/// le domaine reste pur et testable, et pour qu'un serveur puisse rejouer
/// l'equilibrage sans recompiler. Les defauts sont les chiffres historiques,
/// a l'identique.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassRules {
    pub ecraseur: ClassStats,
    pub ressort: ClassStats,
    pub piegeur: ClassStats,
    pub couette: ClassStats,
    /// Degats de l'Ecraseur, en %. 125 = +25 %.
    pub ecraseur_damage_pct: i32,
    /// Part de Confort en dessous de laquelle l'Ecraseur enrage, en %.
    pub ecraseur_rage_threshold_pct: i32,
    /// Degats de l'Ecraseur enrage, en % de ses degats normaux.
    pub ecraseur_rage_bonus_pct: i32,
    /// Confort maximum de la Couette, en %. 130 = +30 %.
    pub couette_hp_pct: i32,
    /// Degats SUBIS par la Couette, en %. 80 = -20 %.
    pub couette_damage_taken_pct: i32,
    /// Degats retires forfaitairement a chaque coup encaisse par la Couette.
    pub couette_flat_reduction: i32,
    /// Confort de base, avant moelleux.
    pub hp_base: i32,
    /// Confort gagne par point de moelleux.
    pub hp_per_def: i32,
    /// Degats de base, avant impact et moelleux.
    pub damage_base: i32,
    pub damage_per_atk: i32,
    pub damage_per_def: i32,
}

impl Default for ClassRules {
    fn default() -> Self {
        Self {
            ecraseur: ClassStats {
                atk: 25,
                def: 8,
                atk_growth: 4,
                def_growth: 1,
            },
            ressort: ClassStats {
                atk: 12,
                def: 18,
                atk_growth: 2,
                def_growth: 3,
            },
            piegeur: ClassStats {
                atk: 18,
                def: 14,
                atk_growth: 3,
                def_growth: 2,
            },
            couette: ClassStats {
                atk: 8,
                def: 25,
                atk_growth: 1,
                def_growth: 4,
            },
            ecraseur_damage_pct: 125,
            ecraseur_rage_threshold_pct: 30,
            ecraseur_rage_bonus_pct: 125,
            couette_hp_pct: 130,
            couette_damage_taken_pct: 80,
            couette_flat_reduction: 5,
            hp_base: 100,
            hp_per_def: 10,
            damage_base: 10,
            damage_per_atk: 4,
            damage_per_def: 2,
        }
    }
}

impl ClassRules {
    pub fn stats(&self, class: PlayerClass) -> ClassStats {
        match class {
            PlayerClass::Ecraseur => self.ecraseur,
            PlayerClass::Ressort => self.ressort,
            PlayerClass::Piegeur => self.piegeur,
            PlayerClass::Couette => self.couette,
        }
    }
}

/// Confort maximum. Jamais moins de 1 : a zero, le joueur serait deja leve du
/// canape avant le premier coup.
pub fn max_hp_with(defense: i32, class: PlayerClass, rules: &ClassRules) -> i32 {
    let base = rules.hp_base + defense.max(0) * rules.hp_per_def;
    let value = match class {
        PlayerClass::Couette => base * rules.couette_hp_pct / 100,
        _ => base,
    };
    value.max(1)
}

/// Confort maximum aux regles historiques.
pub fn max_hp(defense: i32, class: PlayerClass) -> i32 {
    max_hp_with(defense, class, &ClassRules::default())
}

/// Degats deterministes hors tirage : les effets aleatoires restent injectes
/// par le cas d'usage afin de garder le domaine reproductible en tests.
pub fn damage_with(
    attack: i32,
    defense: i32,
    attacker: PlayerClass,
    defender: PlayerClass,
    rules: &ClassRules,
) -> i32 {
    let mut value = (rules.damage_base + attack.max(0) * rules.damage_per_atk
        - defense.max(0) * rules.damage_per_def)
        .max(1);
    if attacker == PlayerClass::Ecraseur {
        value = value * rules.ecraseur_damage_pct / 100;
    }
    if defender == PlayerClass::Couette {
        value = value * rules.couette_damage_taken_pct / 100;
    }
    value.max(1)
}

pub fn damage(attack: i32, defense: i32, attacker: PlayerClass, defender: PlayerClass) -> i32 {
    damage_with(attack, defense, attacker, defender, &ClassRules::default())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuelResult {
    pub attacker_won: Option<bool>,
    pub attacker_damage: i32,
    pub defender_damage: i32,
}

/// Resultat d'un combat en plusieurs rounds. Les jets sont fournis par
/// l'appelant afin que le domaine demeure testable et reproductible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatResult {
    pub attacker_won: Option<bool>,
    pub attacker_hp: i32,
    pub defender_hp: i32,
    pub attacker_damage: i32,
    pub defender_damage: i32,
    pub rounds: i32,
}

/// Les regles de bagarre reglables par serveur.
///
/// Regroupees dans une structure plutot qu'ajoutees a la liste d'arguments :
/// `resolve_combat` en comptait deja neuf, deux entiers nus de plus se
/// seraient inverses tot ou tard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CombatRules {
    /// Ecart de niveau tolere. Au-dela, la bagarre est refusee.
    pub level_gap_max: i32,
    /// Nombre de manches impose. 0 = automatique, selon le Confort total.
    pub max_rounds: i32,
    /// Faces du de. Chaque manche multiplie les degats par le jet.
    pub dice_faces: i32,
    /// L'equilibre des classes, embarque ici plutot qu'en argument separe :
    /// `resolve_combat` comptait deja dix parametres.
    pub classes: ClassRules,
}

impl Default for CombatRules {
    /// Le comportement historique, a l'identique.
    fn default() -> Self {
        Self {
            level_gap_max: 9,
            max_rounds: 0,
            dice_faces: 6,
            classes: ClassRules::default(),
        }
    }
}

pub fn resolve_combat(
    attacker_atk: i32,
    attacker_def: i32,
    attacker_class: PlayerClass,
    attacker_level: i32,
    defender_atk: i32,
    defender_def: i32,
    defender_class: PlayerClass,
    defender_level: i32,
    rolls: &[(i32, i32)],
    rules: CombatRules,
) -> Result<CombatResult, &'static str> {
    let handicap = matchmaking_handicap_capped(attacker_level, defender_level, rules.level_gap_max)
        .ok_or("ecart de niveau trop important")?;
    let attacker_is_higher = attacker_level > defender_level;
    let dice = rules.dice_faces.clamp(2, 100);
    let mut attacker_hp = max_hp_with(attacker_def, attacker_class, &rules.classes);
    let mut defender_hp = max_hp_with(defender_def, defender_class, &rules.classes);
    let mut attacker_damage_total = 0;
    let mut defender_damage_total = 0;
    // 0 = automatique : le nombre de manches suit la resistance des deux
    // joueurs, comme avant. Une valeur imposee prime, bornee a 1..=10 pour
    // qu'un reglage absurde ne fasse pas durer une bagarre indefiniment.
    let max_rounds = if rules.max_rounds > 0 {
        rules.max_rounds.clamp(1, 10)
    } else if attacker_hp + defender_hp < 250 {
        3
    } else if attacker_hp + defender_hp <= 400 {
        5
    } else {
        7
    };
    for (index, (attacker_roll, defender_roll)) in
        rolls.iter().take(max_rounds as usize).enumerate()
    {
        let attacker_atk = if attacker_is_higher {
            (attacker_atk as f32 * handicap) as i32
        } else {
            attacker_atk
        };
        let defender_atk = if !attacker_is_higher {
            (defender_atk as f32 * handicap) as i32
        } else {
            defender_atk
        };
        let c = &rules.classes;
        let mut to_defender = damage_with(
            attacker_atk,
            defender_def,
            attacker_class,
            defender_class,
            c,
        ) * (*attacker_roll).clamp(1, dice);
        let mut to_attacker = damage_with(
            defender_atk,
            attacker_def,
            defender_class,
            attacker_class,
            c,
        ) * (*defender_roll).clamp(1, dice);
        // Le baroud d'honneur de l'Ecraseur : sous son seuil de Confort, il
        // frappe plus fort.
        if attacker_class == PlayerClass::Ecraseur
            && attacker_hp * 100
                < max_hp_with(attacker_def, attacker_class, c) * c.ecraseur_rage_threshold_pct
        {
            to_defender = to_defender * c.ecraseur_rage_bonus_pct / 100;
        }
        if defender_class == PlayerClass::Ecraseur
            && defender_hp * 100
                < max_hp_with(defender_def, defender_class, c) * c.ecraseur_rage_threshold_pct
        {
            to_attacker = to_attacker * c.ecraseur_rage_bonus_pct / 100;
        }
        if attacker_class == PlayerClass::Couette {
            to_attacker = (to_attacker - c.couette_flat_reduction).max(1);
        }
        if defender_class == PlayerClass::Couette {
            to_defender = (to_defender - c.couette_flat_reduction).max(1);
        }
        attacker_hp = (attacker_hp - to_attacker).max(0);
        defender_hp = (defender_hp - to_defender).max(0);
        attacker_damage_total += to_defender;
        defender_damage_total += to_attacker;
        if attacker_hp == 0 || defender_hp == 0 || index + 1 == max_rounds as usize {
            break;
        }
    }
    let attacker_won = (attacker_hp != defender_hp)
        .then_some(attacker_hp > defender_hp)
        .or_else(|| {
            (attacker_damage_total != defender_damage_total)
                .then_some(attacker_damage_total > defender_damage_total)
        });
    Ok(CombatResult {
        attacker_won,
        attacker_hp,
        defender_hp,
        attacker_damage: attacker_damage_total,
        defender_damage: defender_damage_total,
        rounds: rolls.len().min(max_rounds as usize) as i32,
    })
}

/// Resolution pure : le caller fournit les jets, ce qui garde le domaine
/// deterministe et permet au cas d'usage d'injecter son alea.
pub fn resolve_duel(
    attacker_atk: i32,
    attacker_def: i32,
    attacker_class: PlayerClass,
    defender_atk: i32,
    defender_def: i32,
    defender_class: PlayerClass,
    attacker_roll: i32,
    defender_roll: i32,
) -> DuelResult {
    let attacker_damage = damage(attacker_atk, defender_def, attacker_class, defender_class)
        * attacker_roll.clamp(1, 6);
    let defender_damage = damage(defender_atk, attacker_def, defender_class, attacker_class)
        * defender_roll.clamp(1, 6);
    DuelResult {
        attacker_won: (attacker_damage != defender_damage)
            .then_some(attacker_damage > defender_damage),
        attacker_damage,
        defender_damage,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn progression_is_bounded() {
        assert_eq!(level_for_xp(xp_for_level(25) + 1), 25);
    }
    #[test]
    fn tank_is_tougher() {
        assert!(max_hp(5, PlayerClass::Couette) > max_hp(5, PlayerClass::Ressort));
    }
    #[test]
    fn damage_never_zero() {
        assert_eq!(
            damage(0, 999, PlayerClass::Ressort, PlayerClass::Couette),
            1
        );
    }
    #[test]
    fn duel_declares_winner_or_draw() {
        assert_eq!(
            resolve_duel(
                10,
                1,
                PlayerClass::Ecraseur,
                1,
                1,
                PlayerClass::Ressort,
                6,
                1
            )
            .attacker_won,
            Some(true)
        );
    }
    #[test]
    fn matchmaking_blocks_gap_of_ten_levels() {
        assert_eq!(matchmaking_handicap(11, 1), None);
    }
    #[test]
    fn player_class_conversions_and_metadata() {
        assert_eq!(PlayerClass::parse("ecraseur"), Some(PlayerClass::Ecraseur));
        assert_eq!(PlayerClass::parse("invalid"), None);
        assert_eq!(PlayerClass::Ecraseur.as_str(), "ecraseur");
        assert_eq!(PlayerClass::Ecraseur.label(), "🪑 Écraseur");
        assert!(!PlayerClass::Ecraseur.pitch().is_empty());
        let (atk, def) = PlayerClass::Ecraseur.base_stats();
        assert_eq!(atk, 25);
        assert_eq!(def, 8);
        let (atk_g, def_g) = PlayerClass::Ecraseur.growth();
        assert_eq!(atk_g, 4);
        assert_eq!(def_g, 1);
        assert_eq!(PlayerClass::ALL.len(), 4);
    }
    #[test]
    fn level_and_xp_capped_logic() {
        assert_eq!(xp_for_level_capped(1, 25), 100);
        assert_eq!(level_for_xp_capped(100, 25), 1);
        assert_eq!(level_for_xp_capped(5000, 25), 9);
    }
    #[test]
    fn titles_capped_logic() {
        assert_eq!(title_for_level_capped(25, 25), "Le Canape, c'est Lui");
        assert_eq!(title_for_level_capped(1, 25), "Bout d'Accoudoir");
        assert_eq!(title_for_level_capped(10, 25), "Squatteur");
    }
    #[test]
    fn matchmaking_handicap_limits() {
        assert_eq!(matchmaking_handicap_capped(5, 5, 9), Some(1.0));
        assert_eq!(matchmaking_handicap_capped(5, 9, 9), Some(0.8));
        assert_eq!(matchmaking_handicap_capped(5, 12, 9), Some(0.6));
        assert_eq!(matchmaking_handicap_capped(5, 15, 9), None);
    }
    #[test]
    fn resolve_combat_edge_cases() {
        // Test combat avec ecart de niveau trop grand
        let res = resolve_combat(
            10,
            10,
            PlayerClass::Ecraseur,
            20,
            10,
            10,
            PlayerClass::Couette,
            1,
            &[(1, 1)],
            CombatRules::default(),
        );
        assert_eq!(res, Err("ecart de niveau trop important"));

        // Test combat avec rage de l'ecraseur
        let rules = CombatRules {
            max_rounds: 3,
            ..Default::default()
        };
        let res = resolve_combat(
            30,
            0,
            PlayerClass::Ecraseur,
            1,
            10,
            0,
            PlayerClass::Couette,
            1,
            &[(6, 6), (6, 6)],
            rules,
        )
        .unwrap();
        assert!(res.attacker_damage > 0);

        // Test combat avec reductions de la couette
        let res = resolve_combat(
            10,
            10,
            PlayerClass::Couette,
            1,
            10,
            10,
            PlayerClass::Couette,
            1,
            &[(1, 1)],
            CombatRules::default(),
        )
        .unwrap();
        assert!(res.attacker_damage > 0);
    }
    #[test]
    fn class_rules_default() {
        let rules = ClassRules::default();
        assert_eq!(rules.stats(PlayerClass::Ecraseur).atk, 25);
    }
    #[test]
    fn multi_round_combat_produces_a_result() {
        let result = resolve_combat(
            25,
            8,
            PlayerClass::Ecraseur,
            1,
            8,
            25,
            PlayerClass::Couette,
            1,
            &[(6, 1), (6, 1), (6, 1)],
            CombatRules::default(),
        )
        .unwrap();
        assert!(result.attacker_damage > 0);
        assert!(result.rounds >= 1);
    }
}
