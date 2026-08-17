use crate::nexus::domain::errors::DomainError;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct StealResult {
    pub success: bool,
    pub amount: i64,
}

/// Fouille ouverte : la victime a jusqu'a `expires_at` pour reagir.
#[derive(Debug, Clone)]
pub struct OpenedSteal {
    pub attempt_id: uuid::Uuid,
    pub victim_id: String,
    /// Fin de la fenetre de defense (RFC3339), pour l'affichage du compte a
    /// rebours cote Discord.
    pub expires_at: String,
    pub defense_window_seconds: i64,
}

/// Denouement d'une fouille, une fois la fenetre close ou la victime reveillee.
#[derive(Debug, Clone)]
pub struct StealOutcome {
    pub attempt_id: uuid::Uuid,
    pub guild_id: String,
    pub thief_id: String,
    pub victim_id: String,
    pub channel_id: String,
    pub message_id: Option<String>,
    /// La victime a-t-elle reagi dans la fenetre ?
    pub defended: bool,
    pub success: bool,
    pub amount: i64,
    /// Detail du jet, pour que le message explique le resultat au lieu de
    /// simplement l'annoncer.
    pub thief_total: i32,
    pub victim_total: i32,
    pub absence_malus: i32,
}

#[async_trait]
pub trait CoussinStealUseCase: Send + Sync {
    /// Ouvre une fouille et laisse a la victime le temps de reagir.
    ///
    /// Ne deplace aucun coin : rien n'est joue tant que la fenetre court.
    async fn open(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
        channel_id: &str,
    ) -> Result<OpenedSteal, DomainError>;

    /// Rattache le message Discord a la fouille ouverte.
    async fn attach_message(
        &self,
        attempt_id: uuid::Uuid,
        message_id: &str,
    ) -> Result<(), DomainError>;

    /// La victime a serre les coussins : on resout tout de suite, defense
    /// pleine. `victim_id` est verifie — personne d'autre ne peut se defendre
    /// a sa place.
    async fn defend(
        &self,
        attempt_id: uuid::Uuid,
        victim_id: &str,
    ) -> Result<StealOutcome, DomainError>;

    /// Resout les fouilles dont la fenetre s'est fermee sans reaction.
    async fn resolve_expired(&self, limit: i64) -> Result<Vec<StealOutcome>, DomainError>;
}
