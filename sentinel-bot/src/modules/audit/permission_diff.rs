//! Diff de permissions : l'algorithme (comparaison de bitmasks + formatage)
//! vit dans le core hexagonal. Le bot garde la table des flags construite sur
//! les constantes Serenity et passe les bits bruts au core.

use serenity::model::permissions::Permissions;

use platform_core::sentinel::domain::services::audit::permission_diff::diff_flags;
pub use platform_core::sentinel::domain::services::audit::permission_diff::{
    format_diff, PermissionChange,
};

/// Liste de toutes les permissions avec leur flag et nom lisible.
const PERMISSION_FLAGS: &[(Permissions, &str)] = &[
    (Permissions::CREATE_INSTANT_INVITE, "CREATE_INSTANT_INVITE"),
    (Permissions::KICK_MEMBERS, "KICK_MEMBERS"),
    (Permissions::BAN_MEMBERS, "BAN_MEMBERS"),
    (Permissions::ADMINISTRATOR, "ADMINISTRATOR"),
    (Permissions::MANAGE_CHANNELS, "MANAGE_CHANNELS"),
    (Permissions::MANAGE_GUILD, "MANAGE_GUILD"),
    (Permissions::ADD_REACTIONS, "ADD_REACTIONS"),
    (Permissions::VIEW_AUDIT_LOG, "VIEW_AUDIT_LOG"),
    (Permissions::PRIORITY_SPEAKER, "PRIORITY_SPEAKER"),
    (Permissions::STREAM, "STREAM"),
    (Permissions::VIEW_CHANNEL, "VIEW_CHANNEL"),
    (Permissions::SEND_MESSAGES, "SEND_MESSAGES"),
    (Permissions::SEND_TTS_MESSAGES, "SEND_TTS_MESSAGES"),
    (Permissions::MANAGE_MESSAGES, "MANAGE_MESSAGES"),
    (Permissions::EMBED_LINKS, "EMBED_LINKS"),
    (Permissions::ATTACH_FILES, "ATTACH_FILES"),
    (Permissions::READ_MESSAGE_HISTORY, "READ_MESSAGE_HISTORY"),
    (Permissions::MENTION_EVERYONE, "MENTION_EVERYONE"),
    (Permissions::USE_EXTERNAL_EMOJIS, "USE_EXTERNAL_EMOJIS"),
    (Permissions::CONNECT, "CONNECT"),
    (Permissions::SPEAK, "SPEAK"),
    (Permissions::MUTE_MEMBERS, "MUTE_MEMBERS"),
    (Permissions::DEAFEN_MEMBERS, "DEAFEN_MEMBERS"),
    (Permissions::MOVE_MEMBERS, "MOVE_MEMBERS"),
    (Permissions::USE_VAD, "USE_VAD"),
    (Permissions::CHANGE_NICKNAME, "CHANGE_NICKNAME"),
    (Permissions::MANAGE_NICKNAMES, "MANAGE_NICKNAMES"),
    (Permissions::MANAGE_ROLES, "MANAGE_ROLES"),
    (Permissions::MANAGE_WEBHOOKS, "MANAGE_WEBHOOKS"),
    (Permissions::MANAGE_EVENTS, "MANAGE_EVENTS"),
    (
        Permissions::USE_APPLICATION_COMMANDS,
        "USE_APPLICATION_COMMANDS",
    ),
    (Permissions::MODERATE_MEMBERS, "MODERATE_MEMBERS"),
];

/// Compare deux sets de permissions et retourne les changements.
pub fn diff_permissions(old: Permissions, new: Permissions) -> Vec<PermissionChange> {
    let flags: Vec<(u64, &'static str)> = PERMISSION_FLAGS
        .iter()
        .map(|&(flag, name)| (flag.bits(), name))
        .collect();
    diff_flags(old.bits(), new.bits(), &flags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_changes() {
        let perms = Permissions::SEND_MESSAGES | Permissions::VIEW_CHANNEL;
        let changes = diff_permissions(perms, perms);
        assert!(changes.is_empty());
    }

    #[test]
    fn permission_added() {
        let old = Permissions::SEND_MESSAGES;
        let new = Permissions::SEND_MESSAGES | Permissions::BAN_MEMBERS;
        let changes = diff_permissions(old, new);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].name, "BAN_MEMBERS");
        assert!(changes[0].added);
    }

    #[test]
    fn permission_removed() {
        let old = Permissions::SEND_MESSAGES | Permissions::BAN_MEMBERS;
        let new = Permissions::SEND_MESSAGES;
        let changes = diff_permissions(old, new);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].name, "BAN_MEMBERS");
        assert!(!changes[0].added);
    }

    #[test]
    fn multiple_changes() {
        let old = Permissions::SEND_MESSAGES | Permissions::BAN_MEMBERS;
        let new =
            Permissions::SEND_MESSAGES | Permissions::KICK_MEMBERS | Permissions::MANAGE_MESSAGES;
        let changes = diff_permissions(old, new);
        assert_eq!(changes.len(), 3);

        let added: Vec<_> = changes.iter().filter(|c| c.added).map(|c| c.name).collect();
        let removed: Vec<_> = changes
            .iter()
            .filter(|c| !c.added)
            .map(|c| c.name)
            .collect();

        assert!(added.contains(&"KICK_MEMBERS"));
        assert!(added.contains(&"MANAGE_MESSAGES"));
        assert!(removed.contains(&"BAN_MEMBERS"));
    }

    #[test]
    fn empty_to_some() {
        let old = Permissions::empty();
        let new = Permissions::ADMINISTRATOR;
        let changes = diff_permissions(old, new);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].name, "ADMINISTRATOR");
        assert!(changes[0].added);
    }
}
