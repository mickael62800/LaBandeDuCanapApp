//! Suivi des arrivees « fantomes » : membres qui rejoignent puis repartent
//! dans la foulee.
//!
//! Pourquoi ici et pas en base : la seule chose a retenir est un couple
//! (instant d'arrivee, message de bienvenue poste), pendant quelques dizaines
//! de minutes. Le bot n'a pas d'acces DB (regle d'or 3) et faire un
//! aller-retour HTTP a chaque arrivee pour une donnee aussi volatile couterait
//! plus cher que le probleme qu'elle resout. La contrepartie assumee : un
//! redemarrage du bot pendant la fenetre perd la trace, et la card de
//! bienvenue reste — c'est un rate benin, jamais une suppression a tort.
//!
//! Le message est enregistre en deux temps (`remember_arrival` a l'arrivee,
//! `attach_message` apres l'envoi) parce que la fenetre doit courir meme si la
//! card n'a pas pu etre postee : sans arrivee connue, on ne saurait pas non
//! plus qu'il faut taire la card de depart.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Au-dela, une entree ne peut plus servir : la plus longue fenetre
/// raisonnable (`welcome_ghost_minutes`) reste tres en dessous de 24 h. Le
/// balayage se fait a l'insertion, ce qui suffit a borner la table sans tache
/// de fond dediee.
const RETENTION: Duration = Duration::from_secs(24 * 3600);

#[derive(Clone, Copy)]
pub struct Arrival {
    pub at: Instant,
    /// Card de bienvenue postee dans le salon public, si l'envoi a reussi.
    pub message: Option<(u64, u64)>,
}

fn store() -> &'static Mutex<HashMap<(u64, u64), Arrival>> {
    static STORE: OnceLock<Mutex<HashMap<(u64, u64), Arrival>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Note l'arrivee d'un membre. Ecrase une entree precedente : un membre qui
/// rejoint a nouveau redemarre sa fenetre.
pub fn remember_arrival(guild_id: u64, user_id: u64) {
    let Ok(mut map) = store().lock() else {
        return;
    };
    map.retain(|_, a| a.at.elapsed() < RETENTION);
    map.insert(
        (guild_id, user_id),
        Arrival {
            at: Instant::now(),
            message: None,
        },
    );
}

/// Rattache la card de bienvenue effectivement postee a l'arrivee en cours.
pub fn attach_message(guild_id: u64, user_id: u64, channel_id: u64, message_id: u64) {
    if let Ok(mut map) = store().lock() {
        if let Some(arrival) = map.get_mut(&(guild_id, user_id)) {
            arrival.message = Some((channel_id, message_id));
        }
    }
}

/// Retire et renvoie l'arrivee suivie pour ce membre (consommee au depart).
pub fn take(guild_id: u64, user_id: u64) -> Option<Arrival> {
    store().lock().ok()?.remove(&(guild_id, user_id))
}
