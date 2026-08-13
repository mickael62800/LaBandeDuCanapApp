//! Hierarchie des roles RBAC, partagee entre middleware HTTP, gRPC et tout
//! consommateur du domain.

/// Hierarchie des roles RBAC (le plus faible en premier).
///
/// L'ordre de declaration EST la hierarchie : `Ord` en decoule et `satisfies`
/// s'y appuie. Inserer un role au milieu changerait donc silencieusement le
/// sens de toutes les comparaisons existantes.
///
/// Les roles sont persistes sous forme de TEXTE (`as_str` / `from_str`),
/// jamais par leur valeur numerique : reordonner l'enum ne corrompt aucune
/// donnee deja enregistree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    /// Membre de la communaute, sans aucun acces au back-office.
    ///
    /// Role par defaut de toute personne du serveur Discord qui se connecte
    /// au site : elle consulte l'espace membre, s'inscrit aux evenements,
    /// vote, joue — mais ne voit RIEN de l'administration.
    ///
    /// Avant ce palier, ces personnes retombaient sur `Viewer`, ce qui leur
    /// ouvrait en lecture tout le back-office (journaux, membres,
    /// moderation). Il existe donc autant pour fermer cet acces que pour
    /// permettre aux membres d'agir sur le site.
    Member = 0,
    /// Read-only sur le back-office.
    Viewer = 1,
    /// Sanctions, tickets, notes.
    Moderator = 2,
    /// Full CRUD sauf RBAC.
    Admin = 3,
    /// Full access + gestion du RBAC.
    Owner = 4,
}

impl Role {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "member" => Some(Role::Member),
            "viewer" => Some(Role::Viewer),
            "moderator" => Some(Role::Moderator),
            "admin" => Some(Role::Admin),
            "owner" => Some(Role::Owner),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Member => "member",
            Role::Viewer => "viewer",
            Role::Moderator => "moderator",
            Role::Admin => "admin",
            Role::Owner => "owner",
        }
    }

    /// `true` si ce role peut faire une action necessitant au moins `required`.
    pub fn satisfies(&self, required: Role) -> bool {
        *self >= required
    }

    /// Ce role donne-t-il acces au back-office ?
    ///
    /// Sert a decider ou envoyer quelqu'un apres connexion : un membre vers
    /// l'espace membre, le staff vers le tableau de bord. Nomme explicitement
    /// plutot que laisse a chaque appelant : un `>= Viewer` recopie a dix
    /// endroits se trompera quelque part le jour ou la hierarchie bougera.
    pub fn has_backoffice_access(&self) -> bool {
        self.satisfies(Role::Viewer)
    }
}

#[cfg(test)]
#[path = "tests/role.rs"]
mod tests;
