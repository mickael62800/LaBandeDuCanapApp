//! Accès à un salon, exprimé en intentions plutôt qu'en bits.
//!
//! Discord fait choisir, rôle par rôle, parmi une quarantaine d'interrupteurs
//! dont la moitié n'a de sens que sur certains types de salon — et se trompe
//! silencieusement quand on en oublie un (autoriser « écrire » sans « voir » ne
//! produit rien). Ici on choisit une INTENTION — refusé, lecture, écriture,
//! modération — et c'est ce module qui la traduit en couples allow/deny
//! cohérents, adaptés au type de salon.
//!
//! Aucun mode ne peut accorder de permission d'administration : les jeux de
//! bits sont figés ici, donc un accès posé depuis le panel ne peut pas servir à
//! se fabriquer un rôle privilégié.

use crate::sentinel::domain::errors::DomainError;

use super::channel_plan::PlannedChannelKind;

// ── Bits de permission Discord ──
const VIEW_CHANNEL: u64 = 1 << 10;
const READ_MESSAGE_HISTORY: u64 = 1 << 16;
const SEND_MESSAGES: u64 = 1 << 11;
const SEND_MESSAGES_IN_THREADS: u64 = 1 << 38;
const CREATE_PUBLIC_THREADS: u64 = 1 << 35;
const ADD_REACTIONS: u64 = 1 << 6;
const EMBED_LINKS: u64 = 1 << 14;
const ATTACH_FILES: u64 = 1 << 15;
const MANAGE_MESSAGES: u64 = 1 << 13;
const CONNECT: u64 = 1 << 20;
const SPEAK: u64 = 1 << 21;
const STREAM: u64 = 1 << 9;
const MUTE_MEMBERS: u64 = 1 << 22;
const DEAFEN_MEMBERS: u64 = 1 << 23;
const MOVE_MEMBERS: u64 = 1 << 24;

/// Ce qu'un rôle a le droit de faire dans le salon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccessMode {
    /// Le salon n'existe pas pour ce rôle.
    Denied,
    /// Peut voir et lire, sans prendre la parole.
    Read,
    /// Participation normale.
    Write,
    /// Participation + outils de modération du salon.
    Moderate,
}

impl AccessMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Denied => "denied",
            Self::Read => "read",
            Self::Write => "write",
            Self::Moderate => "moderate",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_lowercase().as_str() {
            "denied" => Some(Self::Denied),
            "read" => Some(Self::Read),
            "write" => Some(Self::Write),
            "moderate" => Some(Self::Moderate),
            _ => None,
        }
    }
}

/// Une règle d'accès : un rôle, une intention.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChannelAccess {
    /// ID du rôle Discord. Celui de la guild désigne @everyone (convention
    /// Discord : le rôle @everyone porte l'ID du serveur).
    pub role_id: String,
    pub mode: AccessMode,
}

/// Un overwrite Discord prêt à être envoyé : la traduction d'une
/// [`ChannelAccess`] pour un type de salon donné.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelOverwrite {
    pub role_id: String,
    pub allow: u64,
    pub deny: u64,
}

/// Bits pertinents pour un type de salon. Une catégorie réunit les deux
/// familles : ses réglages descendent sur des salons écrits ET vocaux.
fn family_bits(kind: PlannedChannelKind) -> (bool, bool) {
    match kind {
        PlannedChannelKind::Text | PlannedChannelKind::Announcement | PlannedChannelKind::Forum => {
            (true, false)
        }
        PlannedChannelKind::Voice | PlannedChannelKind::Stage => (false, true),
        PlannedChannelKind::Category => (true, true),
    }
}

/// Traduit une intention en couple (allow, deny) pour ce type de salon.
///
/// `Read` refuse explicitement la prise de parole au lieu de se contenter de ne
/// pas l'accorder : sans refus, un rôle qui a la permission au niveau du
/// serveur parlerait quand même — c'est le piège classique du salon
/// « annonces » que tout le monde finit par pouvoir commenter.
pub fn to_overwrite(access: &ChannelAccess, kind: PlannedChannelKind) -> ChannelOverwrite {
    let (textual, voiceish) = family_bits(kind);
    let (mut allow, mut deny) = (0u64, 0u64);

    match access.mode {
        AccessMode::Denied => {
            deny |= VIEW_CHANNEL;
        }
        AccessMode::Read => {
            allow |= VIEW_CHANNEL;
            if textual {
                allow |= READ_MESSAGE_HISTORY;
                deny |= SEND_MESSAGES | SEND_MESSAGES_IN_THREADS | CREATE_PUBLIC_THREADS;
            }
            if voiceish {
                allow |= CONNECT;
                deny |= SPEAK | STREAM;
            }
        }
        AccessMode::Write | AccessMode::Moderate => {
            allow |= VIEW_CHANNEL;
            if textual {
                allow |= READ_MESSAGE_HISTORY
                    | SEND_MESSAGES
                    | SEND_MESSAGES_IN_THREADS
                    | CREATE_PUBLIC_THREADS
                    | ADD_REACTIONS
                    | EMBED_LINKS
                    | ATTACH_FILES;
            }
            if voiceish {
                allow |= CONNECT | SPEAK | STREAM;
            }
            if access.mode == AccessMode::Moderate {
                if textual {
                    allow |= MANAGE_MESSAGES;
                }
                if voiceish {
                    allow |= MUTE_MEMBERS | DEAFEN_MEMBERS | MOVE_MEMBERS;
                }
            }
        }
    }

    ChannelOverwrite {
        role_id: access.role_id.trim().to_string(),
        allow,
        deny,
    }
}

/// Valide les règles d'accès d'un salon.
///
/// `private` reste la façon rapide de dire « @everyone : refusé ». Poser les
/// deux serait ambigu — on demande de choisir plutôt que de trancher à la place
/// de l'utilisateur.
pub fn validate_access(
    access: &[ChannelAccess],
    private: bool,
    guild_id: &str,
    channel_name: &str,
) -> Result<(), DomainError> {
    let mut seen: Vec<&str> = Vec::with_capacity(access.len());
    for rule in access {
        let id = rule.role_id.trim();
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
            return Err(DomainError::ValidationError(format!(
                "Règle d'accès de « {channel_name} » : identifiant de rôle invalide ({id})."
            )));
        }
        if seen.contains(&id) {
            return Err(DomainError::ValidationError(format!(
                "Le salon « {channel_name} » définit deux fois l'accès du même rôle."
            )));
        }
        seen.push(id);
        if private && id == guild_id {
            return Err(DomainError::ValidationError(format!(
                "Le salon « {channel_name} » est marqué privé ET définit un accès pour @everyone : \
                 choisissez l'un ou l'autre."
            )));
        }
    }
    Ok(())
}

/// Overwrites finaux d'un salon : les règles explicites, plus celle qu'implique
/// `private` si elle n'a pas déjà été écrite à la main.
pub fn overwrites_for(
    access: &[ChannelAccess],
    private: bool,
    kind: PlannedChannelKind,
    guild_id: &str,
) -> Vec<ChannelOverwrite> {
    let mut out: Vec<ChannelOverwrite> = access.iter().map(|a| to_overwrite(a, kind)).collect();
    if private && !access.iter().any(|a| a.role_id.trim() == guild_id) {
        out.push(to_overwrite(
            &ChannelAccess {
                role_id: guild_id.to_string(),
                mode: AccessMode::Denied,
            },
            kind,
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn access(role: &str, mode: AccessMode) -> ChannelAccess {
        ChannelAccess {
            role_id: role.into(),
            mode,
        }
    }

    #[test]
    fn denied_only_hides_the_channel() {
        let ow = to_overwrite(&access("1", AccessMode::Denied), PlannedChannelKind::Text);
        assert_eq!(ow.allow, 0);
        assert_eq!(ow.deny, VIEW_CHANNEL);
    }

    #[test]
    fn read_grants_sight_and_actively_refuses_speech() {
        let text = to_overwrite(&access("1", AccessMode::Read), PlannedChannelKind::Text);
        assert!(text.allow & VIEW_CHANNEL != 0);
        assert!(text.allow & READ_MESSAGE_HISTORY != 0);
        // Le refus doit être explicite, sinon une permission de serveur passe.
        assert!(text.deny & SEND_MESSAGES != 0);
        assert!(text.allow & SEND_MESSAGES == 0);

        let voice = to_overwrite(&access("1", AccessMode::Read), PlannedChannelKind::Voice);
        assert!(voice.allow & CONNECT != 0);
        assert!(voice.deny & SPEAK != 0);
    }

    #[test]
    fn write_uses_the_bits_of_the_channel_family() {
        let text = to_overwrite(&access("1", AccessMode::Write), PlannedChannelKind::Text);
        assert!(text.allow & SEND_MESSAGES != 0);
        assert!(text.allow & CONNECT == 0); // rien de vocal sur un salon écrit
        assert_eq!(text.deny, 0);

        let voice = to_overwrite(&access("1", AccessMode::Write), PlannedChannelKind::Voice);
        assert!(voice.allow & SPEAK != 0);
        assert!(voice.allow & SEND_MESSAGES == 0);
    }

    #[test]
    fn a_category_carries_both_families() {
        let ow = to_overwrite(
            &access("1", AccessMode::Write),
            PlannedChannelKind::Category,
        );
        assert!(ow.allow & SEND_MESSAGES != 0);
        assert!(ow.allow & SPEAK != 0);
    }

    #[test]
    fn moderate_adds_channel_tools_on_top_of_write() {
        let text = to_overwrite(&access("1", AccessMode::Moderate), PlannedChannelKind::Text);
        let write = to_overwrite(&access("1", AccessMode::Write), PlannedChannelKind::Text);
        assert!(text.allow & MANAGE_MESSAGES != 0);
        assert_eq!(text.allow & write.allow, write.allow);

        let voice = to_overwrite(
            &access("1", AccessMode::Moderate),
            PlannedChannelKind::Voice,
        );
        assert!(voice.allow & (MUTE_MEMBERS | DEAFEN_MEMBERS | MOVE_MEMBERS) != 0);
    }

    #[test]
    fn no_mode_ever_grants_administration() {
        const ADMINISTRATOR: u64 = 1 << 3;
        const MANAGE_ROLES: u64 = 1 << 28;
        const MANAGE_CHANNELS: u64 = 1 << 4;
        let dangerous = ADMINISTRATOR | MANAGE_ROLES | MANAGE_CHANNELS;
        for mode in [
            AccessMode::Denied,
            AccessMode::Read,
            AccessMode::Write,
            AccessMode::Moderate,
        ] {
            for kind in [
                PlannedChannelKind::Text,
                PlannedChannelKind::Voice,
                PlannedChannelKind::Category,
                PlannedChannelKind::Stage,
                PlannedChannelKind::Forum,
                PlannedChannelKind::Announcement,
            ] {
                let ow = to_overwrite(&access("1", mode), kind);
                assert_eq!(ow.allow & dangerous, 0, "{mode:?} / {kind:?}");
            }
        }
    }

    #[test]
    fn private_adds_the_everyone_denial() {
        let out = overwrites_for(&[], true, PlannedChannelKind::Text, "42");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].role_id, "42");
        assert_eq!(out[0].deny, VIEW_CHANNEL);
    }

    #[test]
    fn explicit_rules_are_kept_as_written() {
        let out = overwrites_for(
            &[access("7", AccessMode::Write)],
            false,
            PlannedChannelKind::Text,
            "42",
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].role_id, "7");
    }

    #[test]
    fn private_and_an_explicit_everyone_rule_is_rejected() {
        let rules = [access("42", AccessMode::Read)];
        assert!(validate_access(&rules, true, "42", "salon").is_err());
        assert!(validate_access(&rules, false, "42", "salon").is_ok());
    }

    #[test]
    fn duplicate_or_malformed_role_ids_are_rejected() {
        let dup = [
            access("7", AccessMode::Read),
            access("7", AccessMode::Write),
        ];
        assert!(validate_access(&dup, false, "42", "salon").is_err());

        let bad = [access("pas-un-id", AccessMode::Read)];
        assert!(validate_access(&bad, false, "42", "salon").is_err());
    }

    #[test]
    fn mode_parsing_round_trips() {
        for mode in [
            AccessMode::Denied,
            AccessMode::Read,
            AccessMode::Write,
            AccessMode::Moderate,
        ] {
            assert_eq!(AccessMode::parse(mode.as_str()), Some(mode));
        }
        assert_eq!(AccessMode::parse("inconnu"), None);
    }
}
