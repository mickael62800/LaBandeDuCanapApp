//! Reglages de l'economie et de Coussin Piégé, par serveur.
//!
//! Meme patron que `game::config_loader` : on lit les valeurs stockees pour
//! la guilde et on applique les defauts. Ces defauts reproduisent EXACTEMENT
//! le comportement d'avant l'introduction de la configuration — une
//! installation qui ne touche a rien ne voit aucun changement.
//!
//! Le domaine reste PUR : ces structures lui sont passees en DONNEES. Aucune
//! fonction de jeu ne va chercher sa configuration elle-meme, sinon il
//! deviendrait impossible de la tester sans base.

use std::sync::Arc;

use crate::nexus::domain::entities::system::bot_config::BotGuildConfig;
use crate::nexus::domain::errors::DomainError;
use crate::nexus::ports::outbound::system::bot_config_repository::BotConfigRepository;

pub const ECONOMY_BOT: &str = "nexus-economy";
pub const COUSSIN_BOT: &str = "nexus-coussin";

// ── Economie ──

#[derive(Debug, Clone)]
pub struct EconomyConfig {
    pub enabled: bool,
    pub starting_coins: i64,
    pub transfer_enabled: bool,
    pub transfer_min: i64,
    /// 0 = pas de plafond.
    pub transfer_max: i64,
    pub wheel_enabled: bool,
    pub wheel_cooldown_hours: i64,
    /// En pourcentage : 100 = gains inchanges.
    pub wheel_payout_multiplier: i64,
}

impl Default for EconomyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            starting_coins: 100,
            transfer_enabled: true,
            transfer_min: 1,
            transfer_max: 0,
            wheel_enabled: true,
            wheel_cooldown_hours: 24,
            wheel_payout_multiplier: 100,
        }
    }
}

impl EconomyConfig {
    /// Applique le multiplicateur a un gain ou une perte.
    ///
    /// Multiplie AVANT de diviser : l'inverse ecraserait les petits montants
    /// a zero (50 * 120 / 100 = 60, alors que 50 / 100 * 120 = 0 en entier).
    ///
    /// Le signe est conserve : une perte multipliee reste une perte.
    pub fn apply_payout(&self, base: i64) -> i64 {
        if self.wheel_payout_multiplier == 100 {
            return base;
        }
        base.saturating_mul(self.wheel_payout_multiplier) / 100
    }

    /// Frais preleves sur un transfert, arrondis a l'entier inferieur.
    ///

    /// Le montant est-il acceptable pour un transfert ?
    pub fn validate_transfer(&self, amount: i64) -> Result<(), String> {
        if !self.transfer_enabled {
            return Err("les transferts sont desactives sur ce serveur".into());
        }
        if amount < self.transfer_min {
            return Err(format!("le minimum est de {} coins", self.transfer_min));
        }
        if self.transfer_max > 0 && amount > self.transfer_max {
            return Err(format!("le maximum est de {} coins", self.transfer_max));
        }
        Ok(())
    }
}

// ── Coussin Piégé ──

#[derive(Debug, Clone)]
pub struct CoussinConfig {
    pub enabled: bool,
    pub steal_enabled: bool,
    pub steal_success_pct: u32,
    pub steal_success_pct_piegeur: u32,
    pub steal_gain_pct: i64,
    pub steal_penalty_pct: i64,
    pub steal_cooldown_minutes: i64,
    pub steal_min_victim_coins: i64,
    pub prime_enabled: bool,
    pub prime_min: i64,
    pub prime_max: i64,
    pub bet_enabled: bool,
    pub bet_min: i64,
    pub insurance_enabled: bool,
    pub insurance_cost: i64,
    pub insurance_duration_minutes: i64,
    /// Pourcentage de chance que l'assurance soit une arnaque (0-100, defaut 5).
    pub insurance_scam_pct: u32,
    /// Gain d'un pari gagnant, en % de la mise. 200 = mise doublee.
    pub bet_payout_pct: i64,
    // ── Bagarres ──
    pub combat_mise_min: i64,
    /// 0 = pas de plafond.
    pub combat_mise_max: i64,
    /// Ecart de niveau au-dela duquel une bagarre est refusee.
    pub level_gap_max: i32,
    /// 0 = automatique (3, 5 ou 7 manches selon le Confort total).
    pub combat_max_rounds: i32,
    // ── Progression ──
    pub max_level: i32,
    pub xp_winner: i64,
    pub xp_loser: i64,
    pub stat_points_per_level: i32,
    /// Prix des objets du coffre, en % du tarif catalogue. 100 = inchange.
    /// S'applique PAR-DESSUS un prix fixe eventuel.
    pub shop_price_pct: i64,
    /// Prix imposes par objet, `shop_price_<cle>`. 0 = tarif catalogue.
    ///
    /// Une liste plutot qu'une struct a sept champs : le catalogue peut
    /// gagner un objet, et il ne faudrait pas que le jour venu on oublie de
    /// completer un endroit sur deux.
    pub shop_prices: Vec<(String, i64)>,
    // ── Delais entre actions ──
    pub combat_cooldown_minutes: i64,
    pub bet_cooldown_minutes: i64,
    pub prime_cooldown_minutes: i64,
    pub class_change_cooldown_minutes: i64,
    // ── Equilibre des classes ──
    pub classes: crate::nexus::domain::entities::coussin::ClassRules,
    /// Faces du de utilise a chaque manche.
    pub dice_faces: i32,
}

impl Default for CoussinConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            steal_enabled: true,
            // Les quatre valeurs historiques du service de vol.
            steal_success_pct: 30,
            steal_success_pct_piegeur: 50,
            steal_gain_pct: 20,
            steal_penalty_pct: 15,
            steal_cooldown_minutes: 30,
            steal_min_victim_coins: 10,
            prime_enabled: true,
            prime_min: 50,
            prime_max: 0,
            bet_enabled: true,
            bet_min: 10,
            insurance_enabled: true,
            // 50 et non 100 : c'est ce que le SQL prelevait reellement, et
            // c'est ce prix-la que les joueurs connaissent.
            insurance_cost: 50,
            insurance_duration_minutes: 60,
            insurance_scam_pct: 5,
            bet_payout_pct: 200,
            combat_mise_min: 1,
            combat_mise_max: 0,
            level_gap_max: 9,
            combat_max_rounds: 0,
            max_level: 25,
            xp_winner: 15,
            xp_loser: 5,
            stat_points_per_level: 3,
            shop_price_pct: 100,
            shop_prices: Vec::new(),
            // Aucun delai a l'origine : les ajouter par defaut changerait le
            // jeu sous les pieds des serveurs existants.
            combat_cooldown_minutes: 0,
            bet_cooldown_minutes: 0,
            prime_cooldown_minutes: 0,
            class_change_cooldown_minutes: 0,
            classes: crate::nexus::domain::entities::coussin::ClassRules::default(),
            dice_faces: 6,
        }
    }
}

impl CoussinConfig {
    /// Interrupteur maitre du jeu.
    ///
    /// Il PRIME sur les interrupteurs de detail : jeu ferme, on ne fouille
    /// pas, on ne parie pas, on n'achete pas — meme si ces reglages-la sont
    /// restes a « autorise ». Sans cette regle, fermer le jeu demanderait de
    /// penser a decocher cinq cases, et il en resterait toujours une.
    ///
    /// A appeler dans chaque cas d'usage qui MODIFIE l'etat. Les lectures
    /// (profil, classement, historique, inventaire) restent ouvertes : un jeu
    /// ferme ne doit pas rendre illisible ce que les joueurs ont deja
    /// accompli.
    pub fn ensure_enabled(&self) -> Result<(), DomainError> {
        if self.enabled {
            return Ok(());
        }
        Err(DomainError::Validation(
            "Coussin Piege est desactive sur ce serveur.".into(),
        ))
    }

    /// La mise proposee est-elle acceptable ?
    pub fn validate_mise(&self, mise: i64) -> Result<(), DomainError> {
        if mise < self.combat_mise_min {
            return Err(DomainError::Validation(format!(
                "la mise minimum est de {} coins",
                self.combat_mise_min
            )));
        }
        // 0 = pas de plafond.
        if self.combat_mise_max > 0 && mise > self.combat_mise_max {
            return Err(DomainError::Validation(format!(
                "la mise maximum est de {} coins",
                self.combat_mise_max
            )));
        }
        Ok(())
    }

    /// Prix reel d'un objet.
    ///
    /// Deux etages, dans cet ordre : un prix FIXE par objet s'il est reglé
    /// (0 = on garde le tarif catalogue), puis le multiplicateur global
    /// par-dessus. Ainsi « tout coute 20 % plus cher » reste un seul geste,
    /// meme quand deux objets ont un prix impose.
    ///
    /// Jamais moins de 1 : un objet gratuit se prend en boucle.
    pub fn shop_price(&self, key: &str, catalogue: i64) -> i64 {
        let base = self
            .shop_prices
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, prix)| *prix)
            .filter(|prix| *prix > 0)
            .unwrap_or(catalogue);
        if self.shop_price_pct == 100 {
            return base.max(1);
        }
        (base.saturating_mul(self.shop_price_pct) / 100).max(1)
    }

    /// Les regles de bagarre completes, prêtes pour le domaine.
    pub fn combat_rules(&self) -> crate::nexus::domain::entities::coussin::CombatRules {
        crate::nexus::domain::entities::coussin::CombatRules {
            level_gap_max: self.level_gap_max,
            max_rounds: self.combat_max_rounds,
            dice_faces: self.dice_faces,
            classes: self.classes,
        }
    }

    /// Chance de reussite d'un vol, selon la classe du voleur.
    pub fn steal_chance(&self, is_piegeur: bool) -> u32 {
        if is_piegeur {
            self.steal_success_pct_piegeur
        } else {
            self.steal_success_pct
        }
        // Un taux de 0 ou 100 rendrait le vol inutile ou infaillible : on
        // garde toujours une part de risque des deux cotes.
        .clamp(1, 99)
    }

    /// Montant vole a une victime. Au moins 1 : un vol reussi qui ne rapporte
    /// rien serait indistinguable d'un echec.
    pub fn steal_gain(&self, victim_coins: i64) -> i64 {
        (victim_coins.saturating_mul(self.steal_gain_pct) / 100).max(1)
    }

    /// Penalite subie par un voleur qui echoue.
    pub fn steal_penalty(&self, thief_coins: i64) -> i64 {
        (thief_coins.saturating_mul(self.steal_penalty_pct) / 100).max(1)
    }
}

/// Refuse une action si son delai n'est pas ecoule.
///
/// Ecrit une fois et partage : bagarre, pari, contrat et changement de classe
/// posent la meme question, et quatre copies auraient donne quatre messages
/// differents pour la meme situation.
pub async fn ensure_cooldown_over(
    repo: &Arc<
        dyn crate::nexus::ports::outbound::coussin_cooldown_repository::CoussinCooldownRepository,
    >,
    guild_id: &str,
    user_id: &str,
    action: &str,
    quoi: &str,
) -> Result<(), DomainError> {
    let Some(secondes) = repo.remaining_seconds(guild_id, user_id, action).await? else {
        return Ok(());
    };
    // Sous la minute, on compte en secondes : « reessaie dans 1 min » alors
    // qu'il reste trois secondes donne envie de partir.
    let attente = if secondes < 60 {
        format!("{secondes} s")
    } else {
        format!("{} min", secondes.div_euclid(60).max(1))
    };
    Err(DomainError::RateLimited(format!(
        "{quoi} : reessaie dans {attente}"
    )))
}

// ── Lecture ──

fn find<'a>(items: &'a [BotGuildConfig], key: &str) -> Option<&'a str> {
    items
        .iter()
        .find(|c| c.config_key == key)
        .map(|c| c.config_value.as_str())
}

fn b(items: &[BotGuildConfig], key: &str, default: bool) -> bool {
    match find(items, key) {
        Some("true") | Some("1") => true,
        Some("false") | Some("0") => false,
        _ => default,
    }
}

/// Une valeur illisible retombe sur le defaut plutot que d'echouer : une
/// saisie erronee ne doit pas rendre le jeu injouable.
fn n<T: std::str::FromStr>(items: &[BotGuildConfig], key: &str, default: T) -> T {
    find(items, key)
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(default)
}

/// Les regles de classe du serveur, defaut historique pour tout ce qui n'est
/// pas renseigne.
///
/// Chaque classe se regle avec quatre cles : `<classe>_atk`, `<classe>_def`,
/// `<classe>_atk_growth`, `<classe>_def_growth`. Les nommer d'apres la classe
/// plutot que d'aligner seize cles a plat evite de se tromper de colonne.
fn load_classes(items: &[BotGuildConfig]) -> crate::nexus::domain::entities::coussin::ClassRules {
    use crate::nexus::domain::entities::coussin::{ClassRules, ClassStats, PlayerClass};

    let d = ClassRules::default();
    let stats = |class: PlayerClass, defaut: ClassStats| {
        let nom = class.as_str();
        ClassStats {
            atk: n(items, &format!("{nom}_atk"), defaut.atk),
            def: n(items, &format!("{nom}_def"), defaut.def),
            atk_growth: n(items, &format!("{nom}_atk_growth"), defaut.atk_growth),
            def_growth: n(items, &format!("{nom}_def_growth"), defaut.def_growth),
        }
    };

    ClassRules {
        ecraseur: stats(PlayerClass::Ecraseur, d.ecraseur),
        ressort: stats(PlayerClass::Ressort, d.ressort),
        piegeur: stats(PlayerClass::Piegeur, d.piegeur),
        couette: stats(PlayerClass::Couette, d.couette),
        ecraseur_damage_pct: n(items, "ecraseur_damage_pct", d.ecraseur_damage_pct),
        ecraseur_rage_threshold_pct: n(
            items,
            "ecraseur_rage_threshold_pct",
            d.ecraseur_rage_threshold_pct,
        ),
        ecraseur_rage_bonus_pct: n(items, "ecraseur_rage_bonus_pct", d.ecraseur_rage_bonus_pct),
        couette_hp_pct: n(items, "couette_hp_pct", d.couette_hp_pct),
        couette_damage_taken_pct: n(
            items,
            "couette_damage_taken_pct",
            d.couette_damage_taken_pct,
        ),
        couette_flat_reduction: n(items, "couette_flat_reduction", d.couette_flat_reduction),
        hp_base: n(items, "hp_base", d.hp_base),
        hp_per_def: n(items, "hp_per_def", d.hp_per_def),
        damage_base: n(items, "damage_base", d.damage_base),
        damage_per_atk: n(items, "damage_per_atk", d.damage_per_atk),
        damage_per_def: n(items, "damage_per_def", d.damage_per_def),
    }
}

pub async fn load_economy(
    repo: &Arc<dyn BotConfigRepository>,
    guild_id: &str,
) -> Result<EconomyConfig, DomainError> {
    let items = repo.get_config(guild_id, ECONOMY_BOT).await?;
    let d = EconomyConfig::default();
    Ok(EconomyConfig {
        enabled: b(&items, "enabled", d.enabled),
        starting_coins: n(&items, "starting_coins", d.starting_coins),
        transfer_enabled: b(&items, "transfer_enabled", d.transfer_enabled),
        transfer_min: n(&items, "transfer_min", d.transfer_min),
        transfer_max: n(&items, "transfer_max", d.transfer_max),
        wheel_enabled: b(&items, "wheel_enabled", d.wheel_enabled),
        wheel_cooldown_hours: n(&items, "wheel_cooldown_hours", d.wheel_cooldown_hours),
        wheel_payout_multiplier: n(&items, "wheel_payout_multiplier", d.wheel_payout_multiplier),
    })
}

pub async fn load_coussin(
    repo: &Arc<dyn BotConfigRepository>,
    guild_id: &str,
) -> Result<CoussinConfig, DomainError> {
    let items = repo.get_config(guild_id, COUSSIN_BOT).await?;
    let d = CoussinConfig::default();
    Ok(CoussinConfig {
        enabled: b(&items, "enabled", d.enabled),
        steal_enabled: b(&items, "steal_enabled", d.steal_enabled),
        steal_success_pct: n(&items, "steal_success_pct", d.steal_success_pct),
        steal_success_pct_piegeur: n(
            &items,
            "steal_success_pct_piegeur",
            d.steal_success_pct_piegeur,
        ),
        steal_gain_pct: n(&items, "steal_gain_pct", d.steal_gain_pct),
        steal_penalty_pct: n(&items, "steal_penalty_pct", d.steal_penalty_pct),
        steal_cooldown_minutes: n(&items, "steal_cooldown_minutes", d.steal_cooldown_minutes),
        steal_min_victim_coins: n(&items, "steal_min_victim_coins", d.steal_min_victim_coins),
        prime_enabled: b(&items, "prime_enabled", d.prime_enabled),
        prime_min: n(&items, "prime_min", d.prime_min),
        prime_max: n(&items, "prime_max", d.prime_max),
        bet_enabled: b(&items, "bet_enabled", d.bet_enabled),
        bet_min: n(&items, "bet_min", d.bet_min),
        insurance_enabled: b(&items, "insurance_enabled", d.insurance_enabled),
        insurance_cost: n(&items, "insurance_cost", d.insurance_cost),
        insurance_duration_minutes: n(
            &items,
            "insurance_duration_minutes",
            d.insurance_duration_minutes,
        ),
        insurance_scam_pct: n(&items, "insurance_scam_pct", d.insurance_scam_pct),
        bet_payout_pct: n(&items, "bet_payout_pct", d.bet_payout_pct),
        combat_mise_min: n(&items, "combat_mise_min", d.combat_mise_min),
        combat_mise_max: n(&items, "combat_mise_max", d.combat_mise_max),
        level_gap_max: n(&items, "level_gap_max", d.level_gap_max),
        combat_max_rounds: n(&items, "combat_max_rounds", d.combat_max_rounds),
        max_level: n(&items, "max_level", d.max_level),
        xp_winner: n(&items, "xp_winner", d.xp_winner),
        xp_loser: n(&items, "xp_loser", d.xp_loser),
        stat_points_per_level: n(&items, "stat_points_per_level", d.stat_points_per_level),
        shop_price_pct: n(&items, "shop_price_pct", d.shop_price_pct),
        shop_prices: crate::nexus::domain::entities::coussin_shop::ITEMS
            .iter()
            .map(|item| {
                (
                    item.key.to_string(),
                    n(&items, &format!("shop_price_{}", item.key), 0),
                )
            })
            .collect(),
        combat_cooldown_minutes: n(&items, "combat_cooldown_minutes", d.combat_cooldown_minutes),
        bet_cooldown_minutes: n(&items, "bet_cooldown_minutes", d.bet_cooldown_minutes),
        prime_cooldown_minutes: n(&items, "prime_cooldown_minutes", d.prime_cooldown_minutes),
        class_change_cooldown_minutes: n(
            &items,
            "class_change_cooldown_minutes",
            d.class_change_cooldown_minutes,
        ),
        classes: load_classes(&items),
        dice_faces: n(&items, "dice_faces", d.dice_faces),
    })
}

/// Depot de configuration VIDE, pour les tests.
///
/// Les services retombent alors sur leurs defauts, qui reproduisent le
/// comportement historique. Declare ici et non recopie dans chaque fichier
/// de test : cinq copies auraient diverge des la premiere evolution du
/// contrat.
#[cfg(test)]
pub(crate) struct EmptyBotConfigRepository;

#[cfg(test)]
#[async_trait::async_trait]
impl BotConfigRepository for EmptyBotConfigRepository {
    async fn get_definitions(
        &self,
    ) -> Result<Vec<crate::nexus::domain::entities::system::bot_config::BotDefinition>, DomainError>
    {
        Ok(vec![])
    }
    async fn get_config(&self, _: &str, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        Ok(vec![])
    }
    async fn get_all_config(&self, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        Ok(vec![])
    }
    async fn set_config(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/economy_config.rs"]
mod tests;
