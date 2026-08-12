//! Paliers de roles par niveau.
//!
//! Un membre gagne de l'XP, monte de niveau, et recoit un role a certains
//! paliers : niveau 1 le role de depart, puis un nouveau role a chaque seuil
//! franchi.
//!
//! # Pourquoi une chaine de configuration et pas une table
//!
//! La fonctionnalite avait sa propre table (`level_rewards`), supprimee depuis.
//! Elle revient ici sous forme de reglage texte, au meme format que les
//! multiplicateurs XP deja en place (`role_id:valeur`).
//!
//! La raison est concrete : le back-office genere ses formulaires depuis le
//! `config_schema` du bot. Un reglage se regle donc sans ecrire une seule ligne
//! de front, alors qu'une table demanderait des routes CRUD et un ecran dedie
//! pour un contenu qui tient en trois lignes.
//!
//! # Deux modes
//!
//! `Cumulatif` garde les roles des paliers precedents — le membre les collectionne.
//! `Remplacement` ne laisse que le palier atteint, les autres sont retires : c'est
//! ce qu'on veut quand les roles sont des rangs qui se succedent, sinon un
//! ancien porte tous les rangs a la fois.

/// Ce qu'on fait des roles des paliers precedents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModePalier {
    /// Le membre garde tous les roles deja obtenus.
    Cumulatif,
    /// Seul le palier courant est porte ; les autres sont retires.
    Remplacement,
}

impl ModePalier {
    /// Lit le mode depuis la configuration. Toute valeur inconnue vaut
    /// `Cumulatif` : c'est le mode qui n'enleve jamais rien, donc celui dont
    /// une erreur de saisie ne peut pas depouiller un membre.
    pub fn depuis_config(valeur: &str) -> Self {
        match valeur.trim() {
            "remplacement" | "replace" => Self::Remplacement,
            _ => Self::Cumulatif,
        }
    }
}

/// Un palier : a partir de `niveau`, le membre porte `role_id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palier {
    pub niveau: i32,
    pub role_id: u64,
}

/// Analyse la configuration `niveau:role_id`, separee par virgules ou retours
/// a la ligne.
///
/// Les entrees illisibles sont ignorees plutot que de faire echouer la lecture
/// entiere : une virgule en trop ne doit pas priver de roles tous les autres
/// paliers. Un niveau nul ou negatif est rejete — le niveau 0 est l'etat
/// initial, un palier a 0 attribuerait le role a tout le monde en permanence,
/// ce qui est le role de `default_role_ids`.
///
/// Le resultat est trie par niveau croissant et dedoublonne par niveau : deux
/// roles pour le meme palier, c'est une saisie ambigue, on garde le premier.
pub fn analyser_paliers(brut: &str) -> Vec<Palier> {
    let mut paliers: Vec<Palier> = brut
        .split([',', '\n'])
        .filter_map(|entree| {
            let (niveau, role) = entree.trim().split_once(':')?;
            let niveau: i32 = niveau.trim().parse().ok()?;
            let role_id: u64 = role.trim().parse().ok()?;
            (niveau > 0 && role_id > 0).then_some(Palier { niveau, role_id })
        })
        .collect();

    paliers.sort_by_key(|p| p.niveau);
    paliers.dedup_by_key(|p| p.niveau);
    paliers
}

/// Les roles a AJOUTER et a RETIRER pour un membre au niveau donne.
///
/// Retourne `(a_ajouter, a_retirer)`. Aucune des deux listes ne tient compte
/// de ce que le membre porte deja : c'est l'appelant, cote Discord, qui sait
/// lire ses roles actuels et n'appellera l'API que pour les differences.
///
/// En mode `Remplacement`, seul le palier le plus haut atteint est ajoute ; les
/// autres paliers configures partent au retrait, y compris ceux au-dessus du
/// niveau courant. Retirer les paliers superieurs importe autant que les
/// inferieurs : un membre retrograde (correction d'XP, remise a zero) doit
/// perdre le rang qu'il ne merite plus.
///
/// # Plancher au niveau 1
///
/// Un membre pris en compte ici a une ligne d'XP : dans notre modele c'est un
/// membre **niveau 1 au minimum** (`level_from_xp` est base 1). Le `niveau.max(1)`
/// est une defense residuelle : une ligne d'XP heritee d'avant la bascule en
/// base 1 peut encore porter un `level = 0` en base tant qu'elle n'a pas ete
/// recalculee. Sans ce plancher, ce membre verrait son role de depart (palier
/// niveau 1, pose a l'acceptation du reglement) juge non merite, donc RETIRE a
/// chaque reconciliation — le rajouter a la main ne tenait pas, le tick suivant
/// le reprenait.
pub fn roles_pour_niveau(
    paliers: &[Palier],
    niveau: i32,
    mode: ModePalier,
) -> (Vec<u64>, Vec<u64>) {
    let niveau = niveau.max(1);
    let atteints: Vec<u64> = paliers
        .iter()
        .filter(|p| niveau >= p.niveau)
        .map(|p| p.role_id)
        .collect();

    match mode {
        ModePalier::Cumulatif => {
            // Les paliers non atteints sont retires : c'est ce qui corrige un
            // membre retrograde. Un role donne a la main hors palier n'est
            // jamais touche, puisqu'il n'est pas dans la liste configuree.
            let a_retirer = paliers
                .iter()
                .filter(|p| niveau < p.niveau)
                .map(|p| p.role_id)
                .filter(|r| !atteints.contains(r))
                .collect();
            (atteints, a_retirer)
        }
        ModePalier::Remplacement => {
            let courant = atteints.last().copied();
            let a_retirer = paliers
                .iter()
                .map(|p| p.role_id)
                .filter(|r| Some(*r) != courant)
                .collect();
            (courant.into_iter().collect(), a_retirer)
        }
    }
}

#[cfg(test)]
#[path = "tests/role_tiers.rs"]
mod tests;
