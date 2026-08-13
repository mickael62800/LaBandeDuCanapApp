use super::*;
use chrono::Duration;

fn t(s: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(1_700_000_000 + s, 0).unwrap()
}

fn membre(nom: &str, self_mute: bool, server_mute: bool, self_deaf: bool) -> VoiceMember {
    VoiceMember {
        user_id: nom.into(),
        username: nom.into(),
        self_mute,
        self_deaf,
        server_mute,
        streaming: false,
        video: false,
    }
}

fn salon(nom: &str, n: usize) -> VoiceChannelPresence {
    VoiceChannelPresence {
        channel_id: nom.into(),
        channel_name: nom.into(),
        members: (0..n)
            .map(|i| membre(&format!("u{i}"), false, false, false))
            .collect(),
        restreint: false,
    }
}

fn salon_restreint(nom: &str, n: usize) -> VoiceChannelPresence {
    VoiceChannelPresence {
        restreint: true,
        ..salon(nom, n)
    }
}

fn presence(salons: Vec<VoiceChannelPresence>, publie_a: i64) -> VoicePresence {
    VoicePresence {
        channels: salons,
        updated_at: t(publie_a),
    }
}

#[test]
fn membre_sans_restriction_peut_parler() {
    assert!(membre("a", false, false, false).can_speak());
}

#[test]
fn micro_coupe_par_le_membre_empeche_de_parler() {
    assert!(!membre("a", true, false, false).can_speak());
}

/// Coupe par un moderateur : ce n'est pas le meme fait qu'un micro coupe
/// soi-meme, mais l'effet sur la parole est identique.
#[test]
fn coupure_moderateur_empeche_de_parler() {
    assert!(!membre("a", false, true, false).can_speak());
}

/// Discord coupe le micro quand on coupe le casque : ne pas le refleter
/// afficherait quelqu'un comme actif alors qu'il n'entend rien.
#[test]
fn casque_coupe_empeche_de_parler() {
    assert!(!membre("a", false, false, true).can_speak());
}

#[test]
fn instantane_recent_est_considere_frais() {
    assert!(presence(vec![], 0).is_fresh(t(10)));
}

/// Montrer « 11 en vocal » alors que le bot est tombe il y a une heure est
/// pire que ne rien montrer.
#[test]
fn instantane_trop_vieux_n_est_plus_frais() {
    assert!(!presence(vec![], 0).is_fresh(t(STALE_AFTER_SECONDS)));
}

#[test]
fn total_additionne_les_salons() {
    let p = presence(vec![salon("a", 3), salon("b", 2)], 0);
    assert_eq!(p.total_members(), 5);
}

/// La liste montre où il se passe quelque chose, pas l'arborescence du
/// serveur.
#[test]
fn salons_vides_sont_ecartes() {
    let p = presence(vec![salon("plein", 2), salon("vide", 0)], 0);
    let noms: Vec<_> = p
        .occupied_channels()
        .iter()
        .map(|c| c.channel_name.as_str())
        .collect();
    assert_eq!(noms, vec!["plein"]);
}

#[test]
fn salons_sont_tries_du_plus_peuple_au_moins_peuple() {
    let p = presence(
        vec![salon("petit", 1), salon("gros", 5), salon("moyen", 3)],
        0,
    );
    let noms: Vec<_> = p
        .occupied_channels()
        .iter()
        .map(|c| c.channel_name.as_str())
        .collect();
    assert_eq!(noms, vec!["gros", "moyen", "petit"]);
}

#[test]
fn presence_totalement_vide_ne_remonte_aucun_salon() {
    let p = presence(vec![salon("a", 0)], 0);
    assert!(p.occupied_channels().is_empty());
    assert_eq!(p.total_members(), 0);
}

fn activite(dernier_message_a: i64) -> TextChannelActivity {
    TextChannelActivity {
        channel_id: "c".into(),
        channel_name: "general".into(),
        recent_authors: vec!["Kalyx".into()],
        last_message_at: t(dernier_message_a),
    }
}

#[test]
fn activite_recente_est_dans_la_fenetre() {
    assert!(activite(0).is_within_window(t(60)));
}

/// Annoncer une conversation finie il y a une heure ferait passer un salon
/// mort pour un salon vivant.
#[test]
fn activite_trop_ancienne_sort_de_la_fenetre() {
    assert!(!activite(0).is_within_window(t(TEXT_WINDOW_SECONDS)));
}

#[test]
fn fenetre_ecrite_couvre_un_quart_d_heure() {
    let a = activite(0);
    assert!(a.is_within_window(t(TEXT_WINDOW_SECONDS - 1)));
    assert!(!a.is_within_window(t(TEXT_WINDOW_SECONDS + 1)));
    assert_eq!(Duration::seconds(TEXT_WINDOW_SECONDS).num_minutes(), 15);
}

#[test]
fn sans_restreints_ne_garde_que_les_salons_publics() {
    let p = presence(vec![salon("general", 2), salon_restreint("staff", 3)], 0);
    let filtre = p.sans_restreints();

    let noms: Vec<&str> = filtre
        .channels
        .iter()
        .map(|c| c.channel_name.as_str())
        .collect();
    assert_eq!(noms, vec!["general"]);
}

#[test]
fn sans_restreints_retire_aussi_du_total() {
    // Le total sert d'accroche (« 5 personnes en vocal »). S'il comptait les
    // salons prives, il trahirait leur existence malgre le filtrage.
    let p = presence(vec![salon("general", 2), salon_restreint("staff", 3)], 0);
    assert_eq!(p.clone().total_members(), 5);
    assert_eq!(p.sans_restreints().total_members(), 2);
}
