//! Plan de création de salons / catégories piloté depuis le panel web.
//!
//! Le web décrit ce qu'il VEUT (« une catégorie Support, avec dedans un salon
//! texte privé et un vocal »), pas la suite d'appels Discord à faire. Ce module
//! porte cette description, la valide, et surtout l'ORDONNE : Discord exige que
//! la catégorie parente existe avant ses enfants, et le plan est écrit par un
//! humain qui n'a aucune raison de s'en soucier.
//!
//! Le lien parent→enfant se fait par `parent_key` (une clé LOCALE au plan,
//! inventée par le front) et non par un ID Discord : au moment où l'utilisateur
//! compose son plan, la catégorie n'existe pas encore. `parent_id` couvre le
//! cas inverse — accrocher un nouveau salon sous une catégorie déjà en place.

use crate::sentinel::domain::entities::system::channel_access::{validate_access, ChannelAccess};
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::domain::services::system::discord_naming::slugify_channel_name;

/// Taille maximale d'un plan. Discord plafonne à 500 salons par serveur ; on
/// reste très en deçà car un plan est une intention humaine, pas une migration
/// de masse (celle-ci passe par guild-backup). Borne aussi le coût : chaque
/// item est un appel HTTP Discord rate-limité.
pub const MAX_PLAN_ITEMS: usize = 100;

/// Limites Discord (documentées côté API v10).
const MAX_NAME_LEN: usize = 100;
const MAX_TOPIC_LEN: usize = 1024;
const MAX_SLOWMODE_SECS: u32 = 21_600; // 6 h
const MAX_USER_LIMIT: u32 = 99;
/// Enfants directs d'une catégorie (limite Discord).
const MAX_CHILDREN_PER_CATEGORY: usize = 50;

/// Type de salon créable depuis le panel. Volontairement plus restreint que
/// l'énumération Discord : pas de thread ni de salon de forum média, qui se
/// créent depuis Discord dans un contexte qu'un panel d'admin ne reproduit pas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlannedChannelKind {
    Category,
    Text,
    Voice,
    Announcement,
    Stage,
    Forum,
}

impl PlannedChannelKind {
    /// Valeur du champ `type` de l'API Discord.
    pub fn discord_type(self) -> u8 {
        match self {
            Self::Text => 0,
            Self::Voice => 2,
            Self::Category => 4,
            Self::Announcement => 5,
            Self::Stage => 13,
            Self::Forum => 15,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Category => "category",
            Self::Text => "text",
            Self::Voice => "voice",
            Self::Announcement => "announcement",
            Self::Stage => "stage",
            Self::Forum => "forum",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_lowercase().as_str() {
            "category" => Some(Self::Category),
            "text" => Some(Self::Text),
            "voice" => Some(Self::Voice),
            "announcement" | "news" => Some(Self::Announcement),
            "stage" => Some(Self::Stage),
            "forum" => Some(Self::Forum),
            _ => None,
        }
    }

    /// Un nom de salon textuel est normalisé par Discord (minuscules, tirets) ;
    /// catégories et vocaux gardent leur casse et leurs espaces. On applique la
    /// même règle nous-mêmes pour que l'aperçu du web dise la vérité.
    pub fn slugifies_name(self) -> bool {
        matches!(self, Self::Text | Self::Announcement | Self::Forum)
    }

    /// Accepte un sujet (`topic`).
    pub fn accepts_topic(self) -> bool {
        matches!(self, Self::Text | Self::Announcement | Self::Forum)
    }

    /// Accepte le mode lent (`rate_limit_per_user`).
    pub fn accepts_slowmode(self) -> bool {
        matches!(self, Self::Text | Self::Forum)
    }

    /// Accepte une limite de participants (`user_limit`).
    pub fn accepts_user_limit(self) -> bool {
        matches!(self, Self::Voice | Self::Stage)
    }
}

/// Un élément du plan : une catégorie ou un salon à créer.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlannedChannel {
    /// Clé locale au plan, choisie par le front. Sert uniquement à relier un
    /// enfant à sa catégorie avant que les IDs Discord n'existent.
    pub key: String,
    pub name: String,
    pub kind: PlannedChannelKind,
    /// Catégorie parente À CRÉER dans ce même plan.
    #[serde(default)]
    pub parent_key: Option<String>,
    /// Catégorie parente DÉJÀ existante sur le serveur (ID Discord).
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub slowmode: u32,
    #[serde(default)]
    pub user_limit: Option<u32>,
    #[serde(default)]
    pub nsfw: bool,
    /// Salon privé : raccourci pour « @everyone : refusé ». Équivaut à une
    /// règle d'accès explicite, et s'exclut donc d'elle (cf. `validate_access`).
    #[serde(default)]
    pub private: bool,
    /// Accès par rôle, en intentions. Traduit en overwrites Discord par
    /// [`crate::sentinel::domain::entities::system::channel_access`].
    #[serde(default)]
    pub access: Vec<ChannelAccess>,
}

impl PlannedChannel {
    /// Nom tel que Discord le retiendra réellement.
    pub fn normalized_name(&self) -> String {
        let trimmed = self.name.trim();
        if self.kind.slugifies_name() {
            slugify_channel_name(trimmed)
        } else {
            trimmed.chars().take(MAX_NAME_LEN).collect()
        }
    }
}

/// Le plan complet soumis par le web.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ChannelPlan {
    pub items: Vec<PlannedChannel>,
}

impl ChannelPlan {
    /// Valide le plan ET le renvoie ordonné, prêt à être exécuté tel quel :
    /// chaque catégorie précède ses enfants.
    ///
    /// La validation est TOTALE (rien n'est créé si quoi que ce soit cloche) —
    /// un plan à moitié appliqué laisserait le serveur dans un état que
    /// l'utilisateur n'a ni demandé ni prévu, et qu'il devrait démêler à la main.
    ///
    /// `guild_id` sert à reconnaître le rôle @everyone, qui porte par
    /// convention Discord l'ID du serveur.
    pub fn validate_and_order(&self, guild_id: &str) -> Result<Vec<PlannedChannel>, DomainError> {
        if self.items.is_empty() {
            return Err(DomainError::ValidationError(
                "Le plan ne contient aucun salon à créer.".into(),
            ));
        }
        if self.items.len() > MAX_PLAN_ITEMS {
            return Err(DomainError::ValidationError(format!(
                "Le plan contient {} éléments (maximum {MAX_PLAN_ITEMS}).",
                self.items.len()
            )));
        }

        let mut seen_keys: Vec<&str> = Vec::with_capacity(self.items.len());
        for item in &self.items {
            let key = item.key.trim();
            if key.is_empty() {
                return Err(DomainError::ValidationError(
                    "Chaque élément du plan doit porter une clé.".into(),
                ));
            }
            if seen_keys.contains(&key) {
                return Err(DomainError::ValidationError(format!(
                    "Clé « {key} » présente deux fois dans le plan."
                )));
            }
            seen_keys.push(key);
            validate_item(item, guild_id)?;
        }

        validate_parents(&self.items, &seen_keys)?;
        validate_no_duplicate_names(&self.items)?;
        validate_category_capacity(&self.items)?;

        Ok(order_categories_first(&self.items))
    }
}

/// Règles portant sur un élément isolé.
fn validate_item(item: &PlannedChannel, guild_id: &str) -> Result<(), DomainError> {
    let name = item.name.trim();
    if name.is_empty() {
        return Err(DomainError::ValidationError(format!(
            "L'élément « {} » n'a pas de nom.",
            item.key
        )));
    }
    if name.chars().count() > MAX_NAME_LEN {
        return Err(DomainError::ValidationError(format!(
            "Le nom « {name} » dépasse {MAX_NAME_LEN} caractères."
        )));
    }
    // Un nom entièrement composé de caractères que Discord retire (« ### »)
    // donnerait un salon au nom vide, que Discord refuse. On le dit ici plutôt
    // que de laisser l'utilisateur découvrir une erreur Discord opaque.
    if item.normalized_name().is_empty() {
        return Err(DomainError::ValidationError(format!(
            "Le nom « {name} » ne contient aucun caractère utilisable pour un salon."
        )));
    }

    if item.kind == PlannedChannelKind::Category
        && (item.parent_key.is_some() || item.parent_id.is_some())
    {
        return Err(DomainError::ValidationError(format!(
            "La catégorie « {name} » ne peut pas être rangée dans une autre catégorie."
        )));
    }
    if item.parent_key.is_some() && item.parent_id.is_some() {
        return Err(DomainError::ValidationError(format!(
            "Le salon « {name} » désigne à la fois une catégorie du plan et une catégorie existante."
        )));
    }

    if let Some(topic) = &item.topic {
        if !topic.is_empty() && !item.kind.accepts_topic() {
            return Err(DomainError::ValidationError(format!(
                "Un salon {} ne peut pas avoir de sujet ({name}).",
                item.kind.as_str()
            )));
        }
        if topic.chars().count() > MAX_TOPIC_LEN {
            return Err(DomainError::ValidationError(format!(
                "Le sujet de « {name} » dépasse {MAX_TOPIC_LEN} caractères."
            )));
        }
    }
    if item.slowmode > 0 {
        if !item.kind.accepts_slowmode() {
            return Err(DomainError::ValidationError(format!(
                "Le mode lent ne s'applique pas à un salon {} ({name}).",
                item.kind.as_str()
            )));
        }
        if item.slowmode > MAX_SLOWMODE_SECS {
            return Err(DomainError::ValidationError(format!(
                "Le mode lent de « {name} » dépasse {MAX_SLOWMODE_SECS} secondes (6 h)."
            )));
        }
    }
    if let Some(limit) = item.user_limit {
        if !item.kind.accepts_user_limit() {
            return Err(DomainError::ValidationError(format!(
                "La limite de participants ne s'applique pas à un salon {} ({name}).",
                item.kind.as_str()
            )));
        }
        if limit > MAX_USER_LIMIT {
            return Err(DomainError::ValidationError(format!(
                "La limite de participants de « {name} » dépasse {MAX_USER_LIMIT}."
            )));
        }
    }

    validate_access(&item.access, item.private, guild_id, name)?;
    Ok(())
}

/// Chaque `parent_key` doit désigner une catégorie du plan.
fn validate_parents(items: &[PlannedChannel], keys: &[&str]) -> Result<(), DomainError> {
    for item in items {
        let Some(parent) = item.parent_key.as_deref().map(str::trim) else {
            continue;
        };
        if !keys.contains(&parent) {
            return Err(DomainError::ValidationError(format!(
                "Le salon « {} » référence une catégorie inconnue ({parent}).",
                item.name.trim()
            )));
        }
        let parent_kind = items
            .iter()
            .find(|c| c.key.trim() == parent)
            .map(|c| c.kind);
        if parent_kind != Some(PlannedChannelKind::Category) {
            return Err(DomainError::ValidationError(format!(
                "Le salon « {} » est rangé sous un élément qui n'est pas une catégorie.",
                item.name.trim()
            )));
        }
    }
    Ok(())
}

/// Deux salons de même type et même nom sous le même parent seraient
/// indiscernables une fois créés. Discord l'autorise ; l'utilisateur, lui, ne
/// le voulait presque certainement pas.
fn validate_no_duplicate_names(items: &[PlannedChannel]) -> Result<(), DomainError> {
    let mut seen: Vec<(String, PlannedChannelKind, String)> = Vec::with_capacity(items.len());
    for item in items {
        let parent = item
            .parent_key
            .as_deref()
            .or(item.parent_id.as_deref())
            .unwrap_or("")
            .trim()
            .to_string();
        let entry = (item.normalized_name(), item.kind, parent);
        if seen.contains(&entry) {
            return Err(DomainError::ValidationError(format!(
                "Deux salons « {} » du même type au même endroit.",
                entry.0
            )));
        }
        seen.push(entry);
    }
    Ok(())
}

/// Discord refuse au-delà de 50 salons dans une catégorie. On compte seulement
/// ce que le plan ajoute : l'existant est vérifié par Discord lui-même, qui
/// répondra une erreur explicite que l'API remonte telle quelle.
fn validate_category_capacity(items: &[PlannedChannel]) -> Result<(), DomainError> {
    for cat in items
        .iter()
        .filter(|i| i.kind == PlannedChannelKind::Category)
    {
        let children = items
            .iter()
            .filter(|i| i.parent_key.as_deref().map(str::trim) == Some(cat.key.trim()))
            .count();
        if children > MAX_CHILDREN_PER_CATEGORY {
            return Err(DomainError::ValidationError(format!(
                "La catégorie « {} » contient {children} salons (maximum {MAX_CHILDREN_PER_CATEGORY}).",
                cat.name.trim()
            )));
        }
    }
    Ok(())
}

/// Ordonne : catégories d'abord (dans l'ordre du plan), puis le reste (dans
/// l'ordre du plan). Une seule passe suffit — les catégories ne peuvent pas
/// s'imbriquer, donc il n'y a jamais de chaîne de dépendances à démêler.
fn order_categories_first(items: &[PlannedChannel]) -> Vec<PlannedChannel> {
    let mut ordered: Vec<PlannedChannel> = items
        .iter()
        .filter(|i| i.kind == PlannedChannelKind::Category)
        .cloned()
        .collect();
    ordered.extend(
        items
            .iter()
            .filter(|i| i.kind != PlannedChannelKind::Category)
            .cloned(),
    );
    ordered
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sentinel::domain::entities::system::channel_access::AccessMode;

    /// ID du serveur de test — donc aussi celui du rôle @everyone.
    const GUILD: &str = "42";

    fn item(key: &str, name: &str, kind: PlannedChannelKind) -> PlannedChannel {
        PlannedChannel {
            key: key.into(),
            name: name.into(),
            kind,
            parent_key: None,
            parent_id: None,
            topic: None,
            slowmode: 0,
            user_limit: None,
            nsfw: false,
            private: false,
            access: Vec::new(),
        }
    }

    fn child(key: &str, name: &str, kind: PlannedChannelKind, parent: &str) -> PlannedChannel {
        PlannedChannel {
            parent_key: Some(parent.into()),
            ..item(key, name, kind)
        }
    }

    #[test]
    fn categories_are_created_before_their_children() {
        let plan = ChannelPlan {
            items: vec![
                child("c1", "Général", PlannedChannelKind::Text, "cat"),
                item("cat", "Communauté", PlannedChannelKind::Category),
                child("c2", "Vocal", PlannedChannelKind::Voice, "cat"),
            ],
        };
        let ordered = plan.validate_and_order(GUILD).expect("plan valide");
        assert_eq!(ordered[0].key, "cat");
        assert_eq!(ordered[1].key, "c1");
        assert_eq!(ordered[2].key, "c2");
    }

    #[test]
    fn text_channel_names_are_slugified_but_voice_names_are_not() {
        // Le slugifieur est Unicode-aware : les accents sont conserves, seuls
        // les caracteres non alphanumeriques deviennent des tirets.
        let text = item("a", "Salon Général !", PlannedChannelKind::Text);
        assert_eq!(text.normalized_name(), "salon-général");
        let voice = item("b", "Salon Général !", PlannedChannelKind::Voice);
        assert_eq!(voice.normalized_name(), "Salon Général !");
    }

    #[test]
    fn empty_plan_is_rejected() {
        assert!(ChannelPlan::default().validate_and_order(GUILD).is_err());
    }

    #[test]
    fn duplicate_keys_are_rejected() {
        let plan = ChannelPlan {
            items: vec![
                item("k", "un", PlannedChannelKind::Text),
                item("k", "deux", PlannedChannelKind::Text),
            ],
        };
        assert!(plan.validate_and_order(GUILD).is_err());
    }

    #[test]
    fn unknown_parent_is_rejected() {
        let plan = ChannelPlan {
            items: vec![child("c", "salon", PlannedChannelKind::Text, "fantome")],
        };
        assert!(plan.validate_and_order(GUILD).is_err());
    }

    #[test]
    fn parent_must_be_a_category() {
        let plan = ChannelPlan {
            items: vec![
                item("t", "accueil", PlannedChannelKind::Text),
                child("c", "salon", PlannedChannelKind::Text, "t"),
            ],
        };
        assert!(plan.validate_and_order(GUILD).is_err());
    }

    #[test]
    fn nested_categories_are_rejected() {
        let plan = ChannelPlan {
            items: vec![
                item("a", "Parent", PlannedChannelKind::Category),
                child("b", "Enfant", PlannedChannelKind::Category, "a"),
            ],
        };
        assert!(plan.validate_and_order(GUILD).is_err());
    }

    #[test]
    fn same_name_same_kind_same_parent_is_rejected() {
        let plan = ChannelPlan {
            items: vec![
                item("a", "General", PlannedChannelKind::Text),
                item("b", "general", PlannedChannelKind::Text),
            ],
        };
        assert!(plan.validate_and_order(GUILD).is_err());
    }

    #[test]
    fn same_name_different_kind_is_allowed() {
        let plan = ChannelPlan {
            items: vec![
                item("a", "general", PlannedChannelKind::Text),
                item("b", "general", PlannedChannelKind::Voice),
            ],
        };
        assert!(plan.validate_and_order(GUILD).is_ok());
    }

    #[test]
    fn name_made_only_of_separators_is_rejected() {
        let plan = ChannelPlan {
            items: vec![item("a", "###", PlannedChannelKind::Text)],
        };
        assert!(plan.validate_and_order(GUILD).is_err());
    }

    #[test]
    fn slowmode_on_voice_channel_is_rejected() {
        let plan = ChannelPlan {
            items: vec![PlannedChannel {
                slowmode: 30,
                ..item("a", "Vocal", PlannedChannelKind::Voice)
            }],
        };
        assert!(plan.validate_and_order(GUILD).is_err());
    }

    #[test]
    fn user_limit_on_text_channel_is_rejected() {
        let plan = ChannelPlan {
            items: vec![PlannedChannel {
                user_limit: Some(5),
                ..item("a", "salon", PlannedChannelKind::Text)
            }],
        };
        assert!(plan.validate_and_order(GUILD).is_err());
    }

    #[test]
    fn out_of_range_values_are_rejected() {
        let slowmode = ChannelPlan {
            items: vec![PlannedChannel {
                slowmode: 21_601,
                ..item("a", "salon", PlannedChannelKind::Text)
            }],
        };
        assert!(slowmode.validate_and_order(GUILD).is_err());
        let limit = ChannelPlan {
            items: vec![PlannedChannel {
                user_limit: Some(100),
                ..item("a", "Vocal", PlannedChannelKind::Voice)
            }],
        };
        assert!(limit.validate_and_order(GUILD).is_err());
    }

    #[test]
    fn both_parent_key_and_parent_id_is_rejected() {
        let plan = ChannelPlan {
            items: vec![
                item("cat", "Cat", PlannedChannelKind::Category),
                PlannedChannel {
                    parent_id: Some("123".into()),
                    ..child("c", "salon", PlannedChannelKind::Text, "cat")
                },
            ],
        };
        assert!(plan.validate_and_order(GUILD).is_err());
    }

    #[test]
    fn oversized_plan_is_rejected() {
        let items = (0..=MAX_PLAN_ITEMS)
            .map(|i| {
                item(
                    &format!("k{i}"),
                    &format!("salon-{i}"),
                    PlannedChannelKind::Text,
                )
            })
            .collect();
        assert!(ChannelPlan { items }.validate_and_order(GUILD).is_err());
    }

    #[test]
    fn category_over_fifty_children_is_rejected() {
        let mut items = vec![item("cat", "Cat", PlannedChannelKind::Category)];
        for i in 0..=MAX_CHILDREN_PER_CATEGORY {
            items.push(child(
                &format!("k{i}"),
                &format!("salon-{i}"),
                PlannedChannelKind::Text,
                "cat",
            ));
        }
        assert!(ChannelPlan { items }.validate_and_order(GUILD).is_err());
    }

    #[test]
    fn access_rules_are_validated_with_the_rest_of_the_plan() {
        let plan = ChannelPlan {
            items: vec![PlannedChannel {
                access: vec![super::ChannelAccess {
                    role_id: "pas-un-id".into(),
                    mode: AccessMode::Read,
                }],
                ..item("a", "salon", PlannedChannelKind::Text)
            }],
        };
        assert!(plan.validate_and_order(GUILD).is_err());
    }

    #[test]
    fn private_together_with_an_explicit_everyone_rule_is_rejected() {
        let plan = ChannelPlan {
            items: vec![PlannedChannel {
                private: true,
                access: vec![super::ChannelAccess {
                    role_id: GUILD.into(),
                    mode: AccessMode::Read,
                }],
                ..item("a", "salon", PlannedChannelKind::Text)
            }],
        };
        assert!(plan.validate_and_order(GUILD).is_err());
    }

    #[test]
    fn discord_types_match_the_api() {
        assert_eq!(PlannedChannelKind::Text.discord_type(), 0);
        assert_eq!(PlannedChannelKind::Voice.discord_type(), 2);
        assert_eq!(PlannedChannelKind::Category.discord_type(), 4);
        assert_eq!(PlannedChannelKind::Announcement.discord_type(), 5);
        assert_eq!(PlannedChannelKind::Stage.discord_type(), 13);
        assert_eq!(PlannedChannelKind::Forum.discord_type(), 15);
    }

    #[test]
    fn kind_parsing_accepts_the_discord_news_alias() {
        assert_eq!(
            PlannedChannelKind::parse("news"),
            Some(PlannedChannelKind::Announcement)
        );
        assert_eq!(PlannedChannelKind::parse("inconnu"), None);
    }
}
