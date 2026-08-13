use crate::sentinel::domain::entities::system::config_parsers::parse_pipe_lines;

/// Template de raison de moderation pour l'autocomplete.
#[derive(Debug, Clone, PartialEq)]
pub struct ReasonTemplate {
    pub label: String,
    pub reason: String,
}

/// Parse les templates depuis le format config : "label|raison" par ligne.
pub fn parse_templates(raw: &str) -> Vec<ReasonTemplate> {
    parse_pipe_lines(raw)
        .into_iter()
        .map(|(label, reason)| ReasonTemplate { label, reason })
        .collect()
}

/// Sérialise les templates vers le format config "label|raison" par ligne
/// (inverse exact de `parse_templates` — l'aller-retour vit ici pour que le
/// format ne puisse pas diverger).
pub fn serialize_templates(templates: &[ReasonTemplate]) -> String {
    templates
        .iter()
        .map(|t| format!("{}|{}", t.label, t.reason))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Filtre les templates par un terme de recherche (pour l'autocomplete).
pub fn filter_templates<'a>(
    templates: &'a [ReasonTemplate],
    query: &str,
) -> Vec<&'a ReasonTemplate> {
    let query_lower = query.to_lowercase();
    templates
        .iter()
        .filter(|t| {
            t.label.to_lowercase().contains(&query_lower)
                || t.reason.to_lowercase().contains(&query_lower)
        })
        .take(25) // Max Discord autocomplete
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple() {
        let raw = "Spam|Envoi repetitif de messages\nInsulte|Propos insultants envers un membre";
        let templates = parse_templates(raw);
        assert_eq!(templates.len(), 2);
        assert_eq!(templates[0].label, "Spam");
        assert_eq!(templates[0].reason, "Envoi repetitif de messages");
    }

    #[test]
    fn parse_ignores_empty() {
        let raw = "\n\nSpam|Raison\n\n";
        assert_eq!(parse_templates(raw).len(), 1);
    }

    #[test]
    fn parse_ignores_invalid() {
        let raw = "No separator\n|empty\nempty|\nOk|Valid";
        assert_eq!(parse_templates(raw).len(), 1);
    }

    #[test]
    fn parse_trims() {
        let raw = "  Label  |  Raison  ";
        let t = parse_templates(raw);
        assert_eq!(t[0].label, "Label");
        assert_eq!(t[0].reason, "Raison");
    }

    #[test]
    fn parse_empty() {
        assert!(parse_templates("").is_empty());
    }

    #[test]
    fn parse_reason_with_pipe() {
        let raw = "Label|Raison avec | pipe";
        let t = parse_templates(raw);
        assert_eq!(t[0].reason, "Raison avec | pipe");
    }

    #[test]
    fn serialize_roundtrip() {
        let templates = vec![
            ReasonTemplate {
                label: "Spam".into(),
                reason: "Repetition".into(),
            },
            ReasonTemplate {
                label: "Insulte".into(),
                reason: "Propos inapproprie".into(),
            },
        ];
        let parsed = parse_templates(&serialize_templates(&templates));
        assert_eq!(parsed, templates);
    }

    #[test]
    fn serialize_empty() {
        assert_eq!(serialize_templates(&[]), "");
    }

    #[test]
    fn serialize_single() {
        let t = vec![ReasonTemplate {
            label: "A".into(),
            reason: "B".into(),
        }];
        assert_eq!(serialize_templates(&t), "A|B");
    }

    #[test]
    fn filter_by_label() {
        let templates = vec![
            ReasonTemplate {
                label: "Spam".into(),
                reason: "Messages repetitifs".into(),
            },
            ReasonTemplate {
                label: "Insulte".into(),
                reason: "Propos insultants".into(),
            },
            ReasonTemplate {
                label: "Pub".into(),
                reason: "Publicite non autorisee".into(),
            },
        ];
        let results = filter_templates(&templates, "ins");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].label, "Insulte");
    }

    #[test]
    fn filter_by_reason() {
        let templates = vec![
            ReasonTemplate {
                label: "Spam".into(),
                reason: "Messages repetitifs".into(),
            },
            ReasonTemplate {
                label: "Pub".into(),
                reason: "Publicite non autorisee".into(),
            },
        ];
        let results = filter_templates(&templates, "repet");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].label, "Spam");
    }

    #[test]
    fn filter_empty_query_returns_all() {
        let templates = vec![
            ReasonTemplate {
                label: "A".into(),
                reason: "a".into(),
            },
            ReasonTemplate {
                label: "B".into(),
                reason: "b".into(),
            },
        ];
        assert_eq!(filter_templates(&templates, "").len(), 2);
    }

    #[test]
    fn filter_max_25() {
        let templates: Vec<ReasonTemplate> = (0..30)
            .map(|i| ReasonTemplate {
                label: format!("T{}", i),
                reason: "r".into(),
            })
            .collect();
        assert_eq!(filter_templates(&templates, "").len(), 25);
    }
}
