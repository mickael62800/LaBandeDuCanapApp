//! Diff de bitmasks de permissions. L'algorithme est pur : il travaille sur
//! des `u64` et une table de flags fournie par l'adaptateur (le bot garde sa
//! table construite sur les constantes Serenity — pas de bits codés en dur
//! dans le core).

/// Un changement de permission individuel.
#[derive(Debug, Clone, PartialEq)]
pub struct PermissionChange {
    pub name: &'static str,
    pub added: bool,
}

/// Compare deux bitmasks de permissions et retourne les changements, dans
/// l'ordre de la table `flags`.
pub fn diff_flags(old: u64, new: u64, flags: &[(u64, &'static str)]) -> Vec<PermissionChange> {
    let mut changes = Vec::new();

    for &(flag, name) in flags {
        let had = old & flag != 0;
        let has = new & flag != 0;

        if !had && has {
            changes.push(PermissionChange { name, added: true });
        } else if had && !has {
            changes.push(PermissionChange { name, added: false });
        }
    }

    changes
}

/// Formate les changements en texte lisible.
pub fn format_diff(changes: &[PermissionChange]) -> String {
    if changes.is_empty() {
        return "(aucun changement)".to_string();
    }

    changes
        .iter()
        .map(|c| {
            if c.added {
                format!("+ {}", c.name)
            } else {
                format!("- {}", c.name)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLAGS: &[(u64, &str)] = &[(1 << 0, "A"), (1 << 1, "B"), (1 << 2, "C")];

    #[test]
    fn no_changes() {
        assert!(diff_flags(0b101, 0b101, FLAGS).is_empty());
    }

    #[test]
    fn flag_added() {
        let changes = diff_flags(0b001, 0b011, FLAGS);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].name, "B");
        assert!(changes[0].added);
    }

    #[test]
    fn flag_removed() {
        let changes = diff_flags(0b011, 0b001, FLAGS);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].name, "B");
        assert!(!changes[0].added);
    }

    #[test]
    fn unknown_bits_ignored() {
        // Un bit hors table ne produit aucun changement.
        assert!(diff_flags(0, 1 << 40, FLAGS).is_empty());
    }

    #[test]
    fn format_diff_empty() {
        assert_eq!(format_diff(&[]), "(aucun changement)");
    }

    #[test]
    fn format_diff_mixed() {
        let changes = vec![
            PermissionChange {
                name: "BAN_MEMBERS",
                added: true,
            },
            PermissionChange {
                name: "KICK_MEMBERS",
                added: false,
            },
        ];
        let result = format_diff(&changes);
        assert!(result.contains("+ BAN_MEMBERS"));
        assert!(result.contains("- KICK_MEMBERS"));
    }
}
