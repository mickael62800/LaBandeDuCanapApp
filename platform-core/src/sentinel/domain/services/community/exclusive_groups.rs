/// Groupe de roles mutuellement exclusifs.
#[derive(Debug, Clone)]
pub struct ExclusiveGroup {
    pub name: String,
    pub role_ids: Vec<u64>,
}

/// Parse les groupes exclusifs depuis le format config : "nom:role_id1,role_id2,role_id3" par ligne.
pub fn parse_groups(raw: &str) -> Vec<ExclusiveGroup> {
    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let (name, ids_str) = line.split_once(':')?;
            let name = name.trim();
            if name.is_empty() {
                return None;
            }
            let role_ids: Vec<u64> = ids_str
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            if role_ids.len() < 2 {
                return None;
            } // un groupe doit avoir au moins 2 roles
            Some(ExclusiveGroup {
                name: name.to_string(),
                role_ids,
            })
        })
        .collect()
}

/// Retourne les roles en conflit avec le role donne (roles du meme groupe, sans le role lui-meme).
pub fn get_conflicting_roles(groups: &[ExclusiveGroup], role_id: u64) -> Vec<u64> {
    let mut conflicts = Vec::new();
    for group in groups {
        if group.role_ids.contains(&role_id) {
            for &rid in &group.role_ids {
                if rid != role_id && !conflicts.contains(&rid) {
                    conflicts.push(rid);
                }
            }
        }
    }
    conflicts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple() {
        let raw = "Couleur:111,222,333\nEquipe:444,555";
        let groups = parse_groups(raw);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].name, "Couleur");
        assert_eq!(groups[0].role_ids, vec![111, 222, 333]);
        assert_eq!(groups[1].name, "Equipe");
        assert_eq!(groups[1].role_ids, vec![444, 555]);
    }

    #[test]
    fn parse_ignores_empty() {
        let raw = "\n\nA:111,222\n\n";
        assert_eq!(parse_groups(raw).len(), 1);
    }

    #[test]
    fn parse_ignores_single_role() {
        let raw = "Solo:111"; // besoin d'au moins 2
        assert!(parse_groups(raw).is_empty());
    }

    #[test]
    fn parse_ignores_invalid() {
        let raw = "No colon\n:111,222\nOk:111,222";
        assert_eq!(parse_groups(raw).len(), 1);
    }

    #[test]
    fn parse_empty() {
        assert!(parse_groups("").is_empty());
    }

    #[test]
    fn conflicting_roles_found() {
        let groups = vec![ExclusiveGroup {
            name: "Color".into(),
            role_ids: vec![111, 222, 333],
        }];
        let conflicts = get_conflicting_roles(&groups, 111);
        assert_eq!(conflicts, vec![222, 333]);
    }

    #[test]
    fn conflicting_roles_not_in_group() {
        let groups = vec![ExclusiveGroup {
            name: "Color".into(),
            role_ids: vec![111, 222],
        }];
        assert!(get_conflicting_roles(&groups, 999).is_empty());
    }

    #[test]
    fn conflicting_roles_multiple_groups() {
        let groups = vec![
            ExclusiveGroup {
                name: "A".into(),
                role_ids: vec![1, 2],
            },
            ExclusiveGroup {
                name: "B".into(),
                role_ids: vec![2, 3],
            },
        ];
        // Role 2 est dans les deux groupes → conflits avec 1 et 3
        let conflicts = get_conflicting_roles(&groups, 2);
        assert!(conflicts.contains(&1));
        assert!(conflicts.contains(&3));
    }

    #[test]
    fn no_self_in_conflicts() {
        let groups = vec![ExclusiveGroup {
            name: "X".into(),
            role_ids: vec![10, 20, 30],
        }];
        let conflicts = get_conflicting_roles(&groups, 10);
        assert!(!conflicts.contains(&10));
    }
}
