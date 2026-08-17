use crate::nexus::domain::errors::DomainError;
use async_trait::async_trait;

/// Une fouille en cours : la fenetre de defense de la victime court encore.
#[derive(Debug, Clone)]
pub struct StealAttempt {
    pub id: uuid::Uuid,
    pub guild_id: String,
    pub thief_id: String,
    pub victim_id: String,
    pub channel_id: String,
    pub message_id: Option<String>,
    /// Fin de la fenetre, en RFC3339. Le bot s'en sert pour afficher le
    /// compte a rebours au bon endroit.
    pub expires_at: String,
}

#[async_trait]
pub trait CoussinStealRepository: Send + Sync {
    /// Soldes des deux joueurs, APRES verification que le voleur a le droit
    /// de fouiller (delai entre deux fouilles). Sert a l'ouverture.
    async fn balances(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
    ) -> Result<(i64, i64), DomainError>;

    /// Soldes seuls, sans controle de delai.
    ///
    /// La resolution arrive apres la fenetre de defense : le voleur a pu
    /// entre-temps fouiller ailleurs et se poser un delai. Refuser ici
    /// laisserait une fouille ouverte sans denouement, alors que le joueur
    /// avait parfaitement le droit de la lancer.
    async fn settlement_balances(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
    ) -> Result<(i64, i64), DomainError>;
    async fn transfer(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
        amount: i64,
        success: bool,
        cooldown_minutes: i64,
    ) -> Result<(), DomainError>;

    // ── Fenetre de defense ──

    /// Ouvre une fouille et demarre le compte a rebours.
    ///
    /// Echoue si une fouille du meme voleur sur la meme victime est deja en
    /// cours : sans cela, enchainer la commande ouvrirait dix fenetres
    /// simultanees sur la meme personne.
    async fn open_attempt(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
        channel_id: &str,
        defense_window_seconds: i64,
    ) -> Result<StealAttempt, DomainError>;

    /// Rattache le message Discord a la tentative, pour que le denouement
    /// puisse etre publie au bon endroit meme apres un redemarrage du bot.
    async fn attach_message(
        &self,
        attempt_id: uuid::Uuid,
        message_id: &str,
    ) -> Result<(), DomainError>;

    /// Reclame une tentative pour la resoudre, de facon atomique.
    ///
    /// Renvoie `None` si elle a deja ete resolue — c'est la course normale
    /// entre la victime qui clique a la derniere seconde et le job qui passe
    /// au meme instant. Le premier des deux gagne, l'autre ne fait rien.
    ///
    /// `by_victim` distingue les deux : la victime ne peut reclamer que sa
    /// propre fouille, et seulement avant l'echeance.
    async fn claim_attempt(
        &self,
        attempt_id: uuid::Uuid,
        by_victim: Option<&str>,
    ) -> Result<Option<StealAttempt>, DomainError>;

    /// Tentatives dont la fenetre est fermee et que personne n'a reclamees.
    async fn claim_expired_attempts(&self, limit: i64) -> Result<Vec<StealAttempt>, DomainError>;

    /// Consigne le denouement sur la tentative deja reclamee.
    async fn record_outcome(
        &self,
        attempt_id: uuid::Uuid,
        defended: bool,
        success: bool,
        amount: i64,
    ) -> Result<(), DomainError>;
}
