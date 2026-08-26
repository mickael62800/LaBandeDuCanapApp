//! Annonce d'ouverture d'une session de jeu, redigee par Atrium.
//!
//! LE PARTAGE DES ROLES. Nexus detient les FAITS — le jeu, la jauge de joueurs,
//! l'horaire, la date d'ouverture. Atrium detient la PLUME. Nexus n'ecrit pas
//! une phrase, Atrium n'invente pas un chiffre : les donnees arrivent ici
//! structurees, et le modele n'a qu'a les mettre en forme.
//!
//! PAS DE REPLI STATIQUE, contrairement a l'apaisement. Un rappel apaisant rate
//! peut retomber sur une phrase figee sans grand dommage. Ici, l'annonce
//! PRECEDE le panneau d'inscription : servir un texte de secours ferait ouvrir
//! la session sur un message que personne n'a voulu. Quand Atrium ne peut pas
//! ecrire, on ne poste rien et on retente — c'est un choix explicite, et il a
//! un cout : une panne prolongee retarde l'ouverture.

/// Faits transmis par Nexus. Aucun n'est invente par Atrium.
#[derive(Debug, Clone)]
pub struct GameAnnouncementRequest {
    /// Guilde Discord concernee.
    pub guild_id: String,
    /// Nom du jeu tel que le catalogue le nomme (« Project Zomboid »).
    pub game_name: String,
    /// Nom donne au serveur par son proprietaire.
    pub server_name: String,
    /// Jauge de joueurs annoncee, si le jeu en declare une.
    pub max_players: Option<u32>,
    /// Ouverture prevue, deja mise en forme par Nexus dans le fuseau de la
    /// guilde. Atrium ne fait aucun calcul de date : il ne connait ni le fuseau
    /// ni les plages, et les recalculer ici serait une seconde verite.
    pub opening_label: Option<String>,
    /// Plages d'ouverture en clair (« vendredi et samedi, 19h-23h »).
    pub schedule_label: Option<String>,
    /// Consigne de ton configuree par serveur (`game_context`). Vide = defaut.
    pub admin_context: String,
    /// Reglement de la soiree, tel que l'exploitant l'a ecrit.
    ///
    /// LE MODELE LE LIT MAIS NE LE REECRIT PAS. Il sert a ce que l'annonce
    /// sonne juste — ne pas promettre du PvP quand le reglement l'interdit —
    /// et le texte original est affiche dessous, mot pour mot. Un reglement
    /// reformule est un reglement qui change de sens sans que personne ne s'en
    /// apercoive.
    pub rules: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameAnnouncement {
    /// Texte a publier avant le panneau d'inscription.
    pub content: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GameAnnouncementError {
    #[error("{0} est obligatoire")]
    Missing(&'static str),
    #[error("{field} depasse la limite de {limit} caracteres")]
    TooLong { field: &'static str, limit: usize },
    /// L'IA n'a rien produit d'exploitable. Distinct des erreurs de validation :
    /// celles-ci ne se reparent pas en reessayant, celle-la si.
    #[error("Atrium ne peut pas rediger l'annonce pour l'instant")]
    Unavailable,
}

/// Discord coupe a 2000 ; une annonce doit rester lisible d'un coup d'oeil.
pub const MAX_ANNOUNCEMENT_CHARS: usize = 900;

/// Limite du contexte administrateur, alignee sur les autres contextes Atrium.
pub const MAX_ADMIN_CONTEXT_CHARS: usize = 2_000;

impl GameAnnouncementRequest {
    pub fn validate(&self) -> Result<(), GameAnnouncementError> {
        for (valeur, nom) in [
            (&self.guild_id, "guild_id"),
            (&self.game_name, "game_name"),
            (&self.server_name, "server_name"),
        ] {
            if valeur.trim().is_empty() {
                return Err(GameAnnouncementError::Missing(nom));
            }
        }
        if self.admin_context.chars().count() > MAX_ADMIN_CONTEXT_CHARS {
            return Err(GameAnnouncementError::TooLong {
                field: "admin_context",
                limit: MAX_ADMIN_CONTEXT_CHARS,
            });
        }
        Ok(())
    }

    /// Les faits, en clair, tels qu'ils seront donnes au modele.
    ///
    /// Une ligne par fait connu, et RIEN pour un fait absent : ecrire
    /// « joueurs max : inconnu » inviterait le modele a broder autour du trou.
    pub fn faits(&self) -> String {
        let mut lignes = vec![
            format!("Jeu : {}", self.game_name.trim()),
            format!("Nom du serveur : {}", self.server_name.trim()),
        ];
        if let Some(max) = self.max_players {
            lignes.push(format!("Joueurs maximum : {max}"));
        }
        if let Some(ouverture) = self.opening_label.as_deref().map(str::trim) {
            if !ouverture.is_empty() {
                lignes.push(format!("Ouverture : {ouverture}"));
            }
        }
        if let Some(horaires) = self.schedule_label.as_deref().map(str::trim) {
            if !horaires.is_empty() {
                lignes.push(format!("Horaires : {horaires}"));
            }
        }
        lignes.join("\n")
    }
}

#[cfg(test)]
#[path = "tests/game_announcement.rs"]
mod tests;
