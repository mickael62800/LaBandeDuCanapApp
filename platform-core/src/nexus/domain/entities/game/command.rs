//! Catalogue de commandes d'administration d'un serveur de jeu.
//!
//! Chaque jeu parle sa propre langue : Palworld bannit avec `BanPlayer`,
//! Minecraft avec `ban`, 7 Days to Die avec `ban add`. Demander a un
//! administrateur de retenir trois syntaxes revient a lui demander de ne pas
//! se tromper au moment ou il est presse — c'est-a-dire au pire moment.
//!
//! Le catalogue vit donc en base, par modele de jeu, et l'ecran se construit
//! a partir de lui : ajouter une commande ne demande pas de toucher au front.
//!
//! ## Le navigateur n'envoie jamais de commande
//!
//! Il envoie une CLE de commande et des parametres. Le serveur retrouve le
//! gabarit, valide chaque parametre et compose la commande lui-meme. Sans
//! cette regle, un bouton « bannir » serait une console RCON ouverte a
//! quiconque sait forger une requete : l'API tranche, jamais le web.

use serde::{Deserialize, Serialize};

use crate::nexus::domain::errors::DomainError;

/// Nature d'un parametre, qui decide du controle a l'ecran et de la
/// validation cote serveur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandParamType {
    /// Joueur, choisi dans la liste des connectes.
    Player,
    Text,
    Number,
    Enum,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandParam {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub param_type: CommandParamType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    /// Un parametre facultatif absent efface simplement son emplacement dans
    /// le gabarit (motif d'expulsion, par exemple).
    #[serde(default)]
    pub required: bool,
}

/// Une commande proposee a l'administrateur.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameCommand {
    pub key: String,
    pub label: String,
    /// Section d'affichage : Joueurs, Monde, Maintenance...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Ce que la commande CASSE, par opposition a ce qu'elle fait.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    /// Gabarit RCON, avec les emplacements `{cle}` des parametres.
    ///
    /// Jamais expose au navigateur : c'est le secret de fabrication de la
    /// commande, et le connaitre n'apporte rien a l'ecran.
    #[serde(skip_serializing)]
    pub template: String,
    /// Exige une confirmation explicite avant execution.
    #[serde(default)]
    pub confirm: bool,
    /// Geste irreversible ou visible de tous : l'ecran doit le signaler.
    #[serde(default)]
    pub danger: bool,
    #[serde(default)]
    pub params: Vec<CommandParam>,
}

/// Longueur maximale par defaut d'un parametre texte.
const DEFAULT_MAX_LENGTH: usize = 200;

impl GameCommand {
    pub fn find_param(&self, key: &str) -> Option<&CommandParam> {
        self.params.iter().find(|p| p.key == key)
    }

    /// Compose la commande RCON a partir des valeurs fournies.
    ///
    /// Toute valeur est validee avant substitution, et une cle inconnue est
    /// REFUSEE plutot qu'ignoree : accepter en silence un parametre hors
    /// gabarit reviendrait a faire confiance a ce que le navigateur invente.
    pub fn build(&self, values: &[(String, String)]) -> Result<String, DomainError> {
        for (key, _) in values {
            if self.find_param(key).is_none() {
                return Err(DomainError::Validation(format!(
                    "'{key}' n'est pas un parametre de cette commande"
                )));
            }
        }

        let mut command = self.template.clone();

        for param in &self.params {
            let raw = values
                .iter()
                .find(|(key, _)| key == &param.key)
                .map(|(_, value)| value.trim())
                .filter(|value| !value.is_empty());

            let value = match raw {
                Some(value) => {
                    validate_param(param, value)?;
                    value.to_string()
                }
                None if param.required => {
                    return Err(DomainError::Validation(format!(
                        "'{}' est obligatoire",
                        param.label
                    )));
                }
                // Facultatif et absent : l'emplacement disparait, et l'espace
                // qui le precedait avec lui.
                None => String::new(),
            };

            command = command.replace(&format!("{{{}}}", param.key), &value);
        }

        // Un emplacement laisse en place signalerait un gabarit et un jeu de
        // parametres qui ne se correspondent plus. Envoyer `BanPlayer {uid}`
        // au serveur ne bannirait personne, mais le ferait croire.
        if command.contains('{') && command.contains('}') {
            return Err(DomainError::Internal(format!(
                "commande '{}' : le gabarit contient un emplacement sans parametre",
                self.key
            )));
        }

        let command = command.split_whitespace().collect::<Vec<_>>().join(" ");
        if command.is_empty() {
            return Err(DomainError::Internal(format!(
                "commande '{}' : gabarit vide",
                self.key
            )));
        }
        Ok(command)
    }
}

/// Valide une valeur selon la nature de son parametre.
fn validate_param(param: &CommandParam, value: &str) -> Result<(), DomainError> {
    // SECURITE : un retour a la ligne dans une valeur, et le serveur de jeu
    // lit DEUX commandes la ou l'administrateur en a demande une. C'est la
    // faille d'injection de ce systeme ; elle se ferme ici, pour tous les
    // types de parametres.
    if value.chars().any(|c| c.is_control()) {
        return Err(DomainError::Validation(format!(
            "'{}' contient un caractere interdit",
            param.label
        )));
    }

    let max_length = param.max_length.unwrap_or(DEFAULT_MAX_LENGTH);
    if value.chars().count() > max_length {
        return Err(DomainError::Validation(format!(
            "'{}' depasse {max_length} caracteres",
            param.label
        )));
    }

    match param.param_type {
        CommandParamType::Number => {
            let parsed: i64 = value.parse().map_err(|_| {
                DomainError::Validation(format!("'{}' attend un nombre", param.label))
            })?;
            if let Some(min) = param.min {
                if parsed < min {
                    return Err(DomainError::Validation(format!(
                        "'{}' doit valoir au moins {min}",
                        param.label
                    )));
                }
            }
            if let Some(max) = param.max {
                if parsed > max {
                    return Err(DomainError::Validation(format!(
                        "'{}' ne peut pas depasser {max}",
                        param.label
                    )));
                }
            }
        }
        CommandParamType::Enum => {
            let autorise = param
                .options
                .as_ref()
                .is_some_and(|options| options.iter().any(|o| o == value));
            if !autorise {
                return Err(DomainError::Validation(format!(
                    "'{}' n'accepte pas cette valeur",
                    param.label
                )));
            }
        }
        CommandParamType::Player => {
            // Un identifiant de joueur ne contient jamais d'espace : l'exiger
            // empeche qu'un nom compose ne se transforme en arguments
            // supplementaires pour le serveur de jeu.
            if value.split_whitespace().count() != 1 {
                return Err(DomainError::Validation(format!(
                    "'{}' doit etre un identifiant sans espace",
                    param.label
                )));
            }
        }
        CommandParamType::Text => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn param(key: &str, param_type: CommandParamType, required: bool) -> CommandParam {
        CommandParam {
            key: key.into(),
            label: key.into(),
            param_type,
            description: None,
            options: None,
            min: None,
            max: None,
            max_length: None,
            required,
        }
    }

    fn commande(template: &str, params: Vec<CommandParam>) -> GameCommand {
        GameCommand {
            key: "test".into(),
            label: "Test".into(),
            group: None,
            description: None,
            warning: None,
            template: template.into(),
            confirm: false,
            danger: false,
            params,
        }
    }

    fn valeurs(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn la_commande_se_compose_a_partir_du_gabarit() {
        let cmd = commande(
            "BanPlayer {steamid}",
            vec![param("steamid", CommandParamType::Player, true)],
        );
        assert_eq!(
            cmd.build(&valeurs(&[("steamid", "76561198000000000")]))
                .unwrap(),
            "BanPlayer 76561198000000000"
        );
    }

    #[test]
    fn un_retour_a_la_ligne_est_refuse() {
        // SECURITE : sans ce refus, le serveur de jeu lirait DEUX commandes la
        // ou l'administrateur en a demande une.
        let cmd = commande(
            "Broadcast {message}",
            vec![param("message", CommandParamType::Text, true)],
        );
        assert!(cmd
            .build(&valeurs(&[("message", "bonjour\nShutdown 1")]))
            .is_err());
        assert!(cmd.build(&valeurs(&[("message", "bonjour")])).is_ok());
    }

    #[test]
    fn un_parametre_hors_gabarit_est_refuse() {
        // Fail closed : accepter en silence reviendrait a faire confiance a ce
        // que le navigateur invente.
        let cmd = commande(
            "Broadcast {message}",
            vec![param("message", CommandParamType::Text, true)],
        );
        assert!(cmd
            .build(&valeurs(&[("message", "salut"), ("extra", "x")]))
            .is_err());
    }

    #[test]
    fn un_parametre_obligatoire_manquant_arrete_tout() {
        let cmd = commande(
            "BanPlayer {steamid}",
            vec![param("steamid", CommandParamType::Player, true)],
        );
        assert!(cmd.build(&[]).is_err());
    }

    #[test]
    fn un_parametre_facultatif_absent_disparait_du_resultat() {
        let cmd = commande(
            "kick {joueur} {motif}",
            vec![
                param("joueur", CommandParamType::Player, true),
                param("motif", CommandParamType::Text, false),
            ],
        );
        // Pas d'espace en trop la ou le motif manquait.
        assert_eq!(
            cmd.build(&valeurs(&[("joueur", "Bob")])).unwrap(),
            "kick Bob"
        );
        assert_eq!(
            cmd.build(&valeurs(&[("joueur", "Bob"), ("motif", "spam")]))
                .unwrap(),
            "kick Bob spam"
        );
    }

    #[test]
    fn un_identifiant_de_joueur_ne_peut_pas_contenir_d_espace() {
        // Sinon « Bob ; Shutdown » deviendrait deux arguments de plus.
        let cmd = commande(
            "BanPlayer {steamid}",
            vec![param("steamid", CommandParamType::Player, true)],
        );
        assert!(cmd.build(&valeurs(&[("steamid", "Bob Martin")])).is_err());
    }

    #[test]
    fn les_bornes_numeriques_sont_tenues() {
        let mut p = param("secondes", CommandParamType::Number, true);
        p.min = Some(1);
        p.max = Some(60);
        let cmd = commande("Shutdown {secondes}", vec![p]);
        assert!(cmd.build(&valeurs(&[("secondes", "0")])).is_err());
        assert!(cmd.build(&valeurs(&[("secondes", "61")])).is_err());
        assert!(cmd.build(&valeurs(&[("secondes", "douze")])).is_err());
        assert_eq!(
            cmd.build(&valeurs(&[("secondes", "30")])).unwrap(),
            "Shutdown 30"
        );
    }

    #[test]
    fn une_liste_de_choix_refuse_ce_qui_nest_pas_dedans() {
        let mut p = param("mode", CommandParamType::Enum, true);
        p.options = Some(vec!["survival".into(), "creative".into()]);
        let cmd = commande("gamemode {mode}", vec![p]);
        assert!(cmd.build(&valeurs(&[("mode", "spectator")])).is_err());
        assert!(cmd.build(&valeurs(&[("mode", "creative")])).is_ok());
    }

    #[test]
    fn un_gabarit_incomplet_ne_part_jamais_au_serveur() {
        // `BanPlayer {uid}` sans parametre `uid` ne bannirait personne, mais le
        // ferait croire. Mieux vaut echouer bruyamment.
        let cmd = commande("BanPlayer {uid}", vec![]);
        assert!(cmd.build(&[]).is_err());
    }

    #[test]
    fn le_gabarit_ne_part_pas_vers_le_navigateur() {
        // Le connaitre n'apporte rien a l'ecran, et l'exposer invite a le
        // rejouer a la main.
        let cmd = commande("BanPlayer {steamid}", vec![]);
        let json = serde_json::to_value(&cmd).unwrap();
        assert!(json.get("template").is_none());
    }
}
