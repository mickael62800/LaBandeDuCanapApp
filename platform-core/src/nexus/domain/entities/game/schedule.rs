//! Horaires d'ouverture recurrents d'un serveur de jeu.
//!
//! Un serveur de soiree n'a pas besoin de tourner la journee : il consomme
//! memoire et processeur pour personne. Ce module decide, a un instant donne,
//! si un serveur DOIT etre allume, eteint, ou si ses joueurs doivent etre
//! prevenus d'une fermeture prochaine.
//!
//! Trois pieges, tous couverts par des tests :
//!
//!   - **le fuseau horaire.** Les plages sont saisies en heure LOCALE
//!     (« 19h-minuit »), et l'heure d'ete decale l'UTC d'une heure deux fois
//!     par an. Un decalage fixe ferait ouvrir le serveur a 18h ou 20h la
//!     moitie de l'annee ;
//!   - **les plages qui passent minuit.** « 22h-02h » ne s'ecrit pas
//!     `debut <= maintenant < fin` : sans traiter le franchissement, la plage
//!     n'est jamais active ;
//!   - **la fin de session.** Passe la date de fermeture, plus rien ne
//!     redemarre — sans quoi un serveur ressusciterait chaque jour a 19h
//!     jusqu'a la fin des temps.

use chrono::{DateTime, NaiveTime, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

/// Une plage d'ouverture quotidienne, en heure locale.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimeRange {
    /// Minutes depuis minuit (0..1440). `19:00` vaut 1140.
    pub start_minute: u16,
    pub end_minute: u16,
}

impl TimeRange {
    /// La plage franchit-elle minuit ? (`22:00` -> `02:00`)
    pub fn crosses_midnight(&self) -> bool {
        self.end_minute <= self.start_minute
    }

    /// La minute donnee tombe-t-elle dans la plage ?
    ///
    /// La fin est EXCLUE : deux plages qui se touchent (`12:00-14:00` puis
    /// `14:00-16:00`) ne doivent pas se chevaucher a 14h00 pile.
    pub fn contains(&self, minute_of_day: u16) -> bool {
        if self.crosses_midnight() {
            minute_of_day >= self.start_minute || minute_of_day < self.end_minute
        } else {
            minute_of_day >= self.start_minute && minute_of_day < self.end_minute
        }
    }

    /// Minutes restantes avant la fin de la plage, si l'on est dedans.
    pub fn minutes_until_end(&self, minute_of_day: u16) -> Option<u16> {
        if !self.contains(minute_of_day) {
            return None;
        }
        Some(if self.end_minute > minute_of_day {
            self.end_minute - minute_of_day
        } else {
            // Traverse minuit : le reste de la journee, plus le debut de la
            // suivante.
            (1440 - minute_of_day) + self.end_minute
        })
    }
}

/// Les deux facons de piloter un serveur dans le temps. Elles s'excluent : un
/// serveur eteint la nuit ne peut pas etre un serveur qui tourne en permanence.
///
/// La colonne `mode` porte cette exclusion a elle seule. Deux interrupteurs
/// separes auraient laisse exister l'etat « les deux actifs », ou les plages
/// eteindraient le serveur pendant que les redemarrages le rallument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleMode {
    /// Plages d'ouverture : le serveur ne tourne que sur les creneaux declares.
    Ranges,
    /// Permanence : le serveur tourne en continu, et redemarre a intervalle
    /// regulier pour rendre la memoire que les jeux ne rendent pas seuls.
    Restart,
}

impl ScheduleMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ranges => "ranges",
            Self::Restart => "restart",
        }
    }

    /// Un mode inconnu retombe sur `Ranges` : c'est le comportement historique,
    /// et le seul des deux qui ne redemarre rien tout seul.
    pub fn from_str(s: &str) -> Self {
        match s {
            "restart" => Self::Restart,
            _ => Self::Ranges,
        }
    }
}

/// Intervalles proposes, en heures. Tous DIVISENT 24 : c'est ce qui permet aux
/// creneaux de retomber a la meme heure locale chaque jour. Un intervalle de 5 h
/// deriverait d'une journee a l'autre (0h, 5h, 10h, 15h, 20h, puis 1h...), et
/// l'annonce « redemarrage a 20h » cesserait d'etre vraie des le lendemain.
pub const RESTART_INTERVALS_HOURS: [u8; 8] = [1, 2, 3, 4, 6, 8, 12, 24];

/// Retard tolere sur un creneau de redemarrage.
///
/// Le passage periodique peut manquer une minute : API redemarree, verrou tenu
/// par un autre noeud, machine chargee. Sans ce rattrapage, le redemarrage
/// serait purement et simplement saute. Au-dela, on renonce plutot que de
/// couper le serveur a une heure que personne n'a annoncee.
pub const RESTART_GRACE_MINUTES: i64 = 10;

/// Preavis final, juste avant la coupure.
pub const FINAL_WARN_MINUTES: u16 = 1;

/// Reglages d'ouverture automatique d'un serveur.
#[derive(Debug, Clone)]
pub struct AutoSchedule {
    pub enabled: bool,
    /// Lequel des deux systemes pilote ce serveur.
    pub mode: ScheduleMode,
    /// Nom IANA du fuseau (« Europe/Paris »). Les plages et les creneaux de
    /// redemarrage sont exprimes dedans.
    pub timezone: String,
    pub ranges: Vec<TimeRange>,
    /// Preavis avant fermeture ou redemarrage, en minutes. 0 = pas d'annonce.
    pub warn_minutes: u16,
    /// Date de fin de session : au-dela, plus aucun demarrage.
    pub closes_at: Option<DateTime<Utc>>,
    /// Mode `Restart` : heures entre deux redemarrages. `None` = aucun
    /// redemarrage programme (fail closed : on ne devine pas un intervalle).
    pub restart_interval_hours: Option<u8>,
    /// Minute de l'heure a laquelle tombent les creneaux (`0` = a l'heure pile).
    pub restart_anchor_minute: u8,
    /// Dernier redemarrage programme execute, pour ne pas le rejouer.
    pub last_restart_at: Option<DateTime<Utc>>,
    /// Dernier preavis envoye.
    pub last_warned_at: Option<DateTime<Utc>>,
    /// Dernier preavis FINAL envoye. Distinct du precedent : les deux annonces
    /// portent sur le meme creneau et ne doivent pas s'annuler l'une l'autre.
    pub last_final_warned_at: Option<DateTime<Utc>>,
}

/// Ce qu'il faut faire du serveur, maintenant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleAction {
    /// Rien a faire : l'etat courant est le bon.
    Nothing,
    /// Demarrer : on est dans une plage et le serveur est eteint.
    Start,
    /// Arreter : hors plage, ou session terminee.
    Stop { reason: StopReason },
    /// Prevenir les joueurs d'une fermeture prochaine.
    Warn { minutes_left: u16 },
    /// Prevenir les joueurs d'un redemarrage prochain (mode `Restart`).
    RestartWarn { minutes_left: u16 },
    /// Derniere annonce avant la coupure : le temps de se deconnecter.
    RestartFinalWarn,
    /// Sauvegarder, arreter, relancer.
    Restart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// Fin de la plage horaire du jour : le serveur rouvrira.
    OutsideRange,
    /// Date de fin de session atteinte : il ne rouvrira plus.
    SessionOver,
}

/// Decide de l'action a mener.
///
/// `running` est l'etat courant du conteneur. `already_warned` evite de
/// repeter l'annonce a chaque passage du job : sans lui, les joueurs
/// recevraient le meme message chaque minute pendant tout le preavis.
pub fn decide(
    schedule: &AutoSchedule,
    running: bool,
    already_warned: bool,
    now: DateTime<Utc>,
) -> ScheduleAction {
    if !schedule.enabled {
        return ScheduleAction::Nothing;
    }

    // Session terminee : on eteint, et plus rien ne redemarre. Sans cette
    // porte, un serveur ressusciterait chaque jour a 19h indefiniment.
    //
    // La regle vaut pour les DEUX modes : une permanence appartient a une
    // session comme une plage, et « c'est fini » ne se discute pas.
    if let Some(fin) = schedule.closes_at {
        if now >= fin {
            return if running {
                ScheduleAction::Stop {
                    reason: StopReason::SessionOver,
                }
            } else {
                ScheduleAction::Nothing
            };
        }
    }

    // Un fuseau inconnu ne doit pas faire tourner un serveur a contretemps :
    // on s'abstient plutot que de retomber sur UTC en silence.
    let Ok(tz) = schedule.timezone.parse::<Tz>() else {
        return ScheduleAction::Nothing;
    };

    if schedule.mode == ScheduleMode::Restart {
        return decide_restart(schedule, running, now);
    }

    let locale = now.with_timezone(&tz);
    let minute_of_day = (locale.hour() * 60 + locale.minute()) as u16;

    let plage_active = schedule
        .ranges
        .iter()
        .find(|plage| plage.contains(minute_of_day));

    match plage_active {
        Some(plage) => {
            if !running {
                return ScheduleAction::Start;
            }
            // Preavis : seulement dans la fenetre, et une seule fois.
            if schedule.warn_minutes > 0 && !already_warned {
                if let Some(restant) = plage.minutes_until_end(minute_of_day) {
                    if restant <= schedule.warn_minutes {
                        return ScheduleAction::Warn {
                            minutes_left: restant,
                        };
                    }
                }
            }
            ScheduleAction::Nothing
        }
        None => {
            if running {
                ScheduleAction::Stop {
                    reason: StopReason::OutsideRange,
                }
            } else {
                ScheduleAction::Nothing
            }
        }
    }
}

/// Mode permanence : le serveur tourne, et redemarre a heures fixes.
///
/// L'ordre des controles n'est pas indifferent. Le redemarrage passe AVANT les
/// annonces : a la minute du creneau, le prochain creneau est deja celui d'apres
/// et la fenetre de preavis pourrait s'ouvrir pour lui, ce qui repousserait la
/// coupure d'un tour entier.
fn decide_restart(schedule: &AutoSchedule, running: bool, now: DateTime<Utc>) -> ScheduleAction {
    // Permanence : un serveur eteint est rallume. C'est ce qui distingue ce
    // mode d'un simple redemarrage periodique.
    if !running {
        return ScheduleAction::Start;
    }

    // Sans intervalle, il n'y a pas de creneau a calculer. On ne devine pas :
    // le serveur reste allume, et rien ne le coupe.
    if schedule.restart_interval_hours.is_none() {
        return ScheduleAction::Nothing;
    }

    // Le creneau qui vient de passer, s'il n'a pas encore ete honore.
    if let Some(precedent) = previous_restart_at(schedule, now) {
        let deja_fait = schedule
            .last_restart_at
            .is_some_and(|dernier| dernier >= precedent);
        let en_retard = (now - precedent).num_minutes() > RESTART_GRACE_MINUTES;
        if !deja_fait && !en_retard {
            return ScheduleAction::Restart;
        }
    }

    let Some(prochain) = next_restart_at(schedule, now) else {
        return ScheduleAction::Nothing;
    };
    let restant = (prochain - now).num_minutes().max(0) as u16;

    // Preavis final. Sa fenetre s'ouvre a `prochain - 1 min` : un envoi date
    // d'apres cet instant porte forcement sur ce creneau-ci.
    if restant <= FINAL_WARN_MINUTES {
        let deja = schedule
            .last_final_warned_at
            .is_some_and(|t| t >= prochain - chrono::Duration::minutes(FINAL_WARN_MINUTES as i64));
        return if deja {
            ScheduleAction::Nothing
        } else {
            ScheduleAction::RestartFinalWarn
        };
    }

    if schedule.warn_minutes > 0 && restant <= schedule.warn_minutes {
        let deja = schedule.last_warned_at.is_some_and(|t| {
            t >= prochain - chrono::Duration::minutes(schedule.warn_minutes as i64)
        });
        if !deja {
            return ScheduleAction::RestartWarn {
                minutes_left: restant,
            };
        }
    }

    ScheduleAction::Nothing
}

/// Minutes depuis minuit auxquelles tombent les redemarrages d'une journee.
///
/// Vide si l'intervalle ne divise pas 24 : mieux vaut ne rien redemarrer que
/// de deriver d'un jour a l'autre.
fn restart_slots_of_day(interval_hours: u8, anchor_minute: u8) -> Vec<u16> {
    if interval_hours == 0 || 24 % interval_hours != 0 || anchor_minute >= 60 {
        return Vec::new();
    }
    let pas = interval_hours as u16 * 60;
    (0..(1440 / pas))
        .map(|k| k * pas + anchor_minute as u16)
        .collect()
}

/// Instant UTC d'un creneau local, ou `None` si cette heure locale n'existe pas.
///
/// Au passage a l'heure d'ete, 2h30 n'existe pas : la reclamer a `chrono` ne
/// donne rien, et forcer une valeur ferait redemarrer le serveur a un moment
/// qui n'a pas ete annonce. Deux fois par an, ce creneau-la saute.
fn slot_instant(tz: &Tz, date: chrono::NaiveDate, minute_of_day: u16) -> Option<DateTime<Utc>> {
    let heure =
        NaiveTime::from_hms_opt((minute_of_day / 60) as u32, (minute_of_day % 60) as u32, 0)?;
    match tz.from_local_datetime(&date.and_time(heure)) {
        chrono::LocalResult::Single(instant) => Some(instant.with_timezone(&Utc)),
        // Heure jouee deux fois (retour a l'heure d'hiver) : on retient la
        // premiere, sinon le redemarrage aurait une heure de retard.
        chrono::LocalResult::Ambiguous(premier, _) => Some(premier.with_timezone(&Utc)),
        chrono::LocalResult::None => None,
    }
}

/// Tous les creneaux de redemarrage de la veille, du jour et du lendemain.
///
/// Trois jours parce que le fuseau decale la journee locale par rapport a UTC :
/// autour de minuit, le creneau precedent comme le suivant peuvent tomber de
/// l'autre cote de la frontiere de date.
fn restart_slots_around(schedule: &AutoSchedule, now: DateTime<Utc>) -> Vec<DateTime<Utc>> {
    let Some(intervalle) = schedule.restart_interval_hours else {
        return Vec::new();
    };
    let Ok(tz) = schedule.timezone.parse::<Tz>() else {
        return Vec::new();
    };
    let minutes = restart_slots_of_day(intervalle, schedule.restart_anchor_minute);
    if minutes.is_empty() {
        return Vec::new();
    }

    let locale = now.with_timezone(&tz);
    let mut instants = Vec::with_capacity(minutes.len() * 3);
    for decalage in -1..=1 {
        let date = (locale + chrono::Duration::days(decalage)).date_naive();
        for minute in &minutes {
            if let Some(instant) = slot_instant(&tz, date, *minute) {
                instants.push(instant);
            }
        }
    }
    instants.sort_unstable();
    instants.dedup();
    instants
}

/// Prochain redemarrage programme, pour l'annoncer et l'afficher.
pub fn next_restart_at(schedule: &AutoSchedule, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    if !schedule.enabled || schedule.mode != ScheduleMode::Restart {
        return None;
    }
    restart_slots_around(schedule, now)
        .into_iter()
        .find(|instant| *instant > now)
        .filter(|instant| schedule.closes_at.is_none_or(|fin| *instant < fin))
}

/// Dernier creneau de redemarrage echu.
pub fn previous_restart_at(schedule: &AutoSchedule, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    if !schedule.enabled || schedule.mode != ScheduleMode::Restart {
        return None;
    }
    restart_slots_around(schedule, now)
        .into_iter()
        .rfind(|instant| *instant <= now)
}

/// Prochaine ouverture, pour l'annoncer aux joueurs.
///
/// Cherche sur 48 h : au-dela, une configuration sans plage exploitable ne
/// donnerait de toute facon jamais de reponse.
pub fn next_opening(schedule: &AutoSchedule, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    if !schedule.enabled || schedule.ranges.is_empty() {
        return None;
    }
    let tz = schedule.timezone.parse::<Tz>().ok()?;
    let locale = now.with_timezone(&tz);

    let mut debuts: Vec<u16> = schedule.ranges.iter().map(|r| r.start_minute).collect();
    debuts.sort_unstable();

    for jour in 0..2 {
        let date = (locale + chrono::Duration::days(jour)).date_naive();
        for debut in &debuts {
            let heure = NaiveTime::from_hms_opt((*debut / 60) as u32, (*debut % 60) as u32, 0)?;
            let naive = date.and_time(heure);
            // Heure locale inexistante (passage a l'heure d'ete) : on passe au
            // creneau suivant plutot que d'annoncer une heure qui n'existe pas.
            let chrono::LocalResult::Single(instant) = tz.from_local_datetime(&naive) else {
                continue;
            };
            let instant = instant.with_timezone(&Utc);
            if instant > now && schedule.closes_at.is_none_or(|fin| instant < fin) {
                return Some(instant);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// « 19:00 » -> 1140.
    fn hm(h: u16, m: u16) -> u16 {
        h * 60 + m
    }

    fn plage(h1: u16, m1: u16, h2: u16, m2: u16) -> TimeRange {
        TimeRange {
            start_minute: hm(h1, m1),
            end_minute: hm(h2, m2),
        }
    }

    /// Instant UTC. En aout, Paris est a UTC+2 : 17h UTC = 19h locales.
    fn utc(jour: u32, heure: u32, minute: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, jour, heure, minute, 0)
            .unwrap()
    }

    fn horaire(ranges: Vec<TimeRange>) -> AutoSchedule {
        AutoSchedule {
            enabled: true,
            mode: ScheduleMode::Ranges,
            timezone: "Europe/Paris".into(),
            ranges,
            warn_minutes: 10,
            closes_at: None,
            restart_interval_hours: None,
            restart_anchor_minute: 0,
            last_restart_at: None,
            last_warned_at: None,
            last_final_warned_at: None,
        }
    }

    /// Permanence : redemarrage toutes les `heures`, preavis de 15 min.
    fn permanence(heures: u8) -> AutoSchedule {
        AutoSchedule {
            enabled: true,
            mode: ScheduleMode::Restart,
            timezone: "Europe/Paris".into(),
            ranges: vec![],
            warn_minutes: 15,
            closes_at: None,
            restart_interval_hours: Some(heures),
            restart_anchor_minute: 0,
            last_restart_at: None,
            last_warned_at: None,
            last_final_warned_at: None,
        }
    }

    #[test]
    fn une_plage_normale_ouvre_et_ferme_a_l_heure_locale() {
        // 12h-14h locales = 10h-12h UTC en aout.
        let h = horaire(vec![plage(12, 0, 14, 0)]);

        assert_eq!(
            decide(&h, false, false, utc(19, 10, 30)),
            ScheduleAction::Start
        );
        assert_eq!(
            decide(&h, true, false, utc(19, 12, 30)),
            ScheduleAction::Stop {
                reason: StopReason::OutsideRange
            }
        );
    }

    #[test]
    fn l_heure_d_ete_ne_decale_pas_l_ouverture() {
        // C'est le piege qu'un decalage fixe ne verrait pas : la meme plage
        // locale correspond a deux heures UTC differentes selon la saison.
        let h = horaire(vec![plage(19, 0, 23, 0)]);

        // Aout (UTC+2) : 19h locales = 17h UTC.
        assert_eq!(
            decide(&h, false, false, utc(19, 17, 5)),
            ScheduleAction::Start
        );
        assert_eq!(
            decide(&h, false, false, utc(19, 16, 30)),
            ScheduleAction::Nothing
        );

        // Janvier (UTC+1) : 19h locales = 18h UTC.
        let hiver = Utc.with_ymd_and_hms(2026, 1, 15, 18, 5, 0).unwrap();
        let avant = Utc.with_ymd_and_hms(2026, 1, 15, 17, 30, 0).unwrap();
        assert_eq!(decide(&h, false, false, hiver), ScheduleAction::Start);
        assert_eq!(decide(&h, false, false, avant), ScheduleAction::Nothing);
    }

    #[test]
    fn une_plage_qui_passe_minuit_reste_active() {
        // « 19h - minuit » puis « 22h - 02h » : sans traitement du
        // franchissement, ces plages ne seraient jamais actives.
        let p = plage(22, 0, 2, 0);
        assert!(p.crosses_midnight());
        assert!(p.contains(hm(23, 30)));
        assert!(p.contains(hm(1, 0)));
        assert!(!p.contains(hm(3, 0)));
        assert!(!p.contains(hm(21, 0)));
    }

    #[test]
    fn deux_plages_qui_se_touchent_ne_se_chevauchent_pas() {
        // A 14h00 pile, la premiere est finie et la seconde commence : la fin
        // est exclue, sinon un serveur serait « dans deux plages » a la fois.
        let midi = plage(12, 0, 14, 0);
        let apres = plage(14, 0, 16, 0);
        assert!(!midi.contains(hm(14, 0)));
        assert!(apres.contains(hm(14, 0)));
    }

    #[test]
    fn le_preavis_ne_part_qu_une_fois_et_dans_la_fenetre() {
        let h = horaire(vec![plage(12, 0, 14, 0)]);

        // 13h55 locales = 11h55 UTC : 5 minutes avant la fin.
        assert_eq!(
            decide(&h, true, false, utc(19, 11, 55)),
            ScheduleAction::Warn { minutes_left: 5 }
        );
        // Deja annonce : on ne repete pas a chaque passage du job.
        assert_eq!(
            decide(&h, true, true, utc(19, 11, 55)),
            ScheduleAction::Nothing
        );
        // Trop tot pour prevenir.
        assert_eq!(
            decide(&h, true, false, utc(19, 11, 0)),
            ScheduleAction::Nothing
        );
    }

    #[test]
    fn le_preavis_traverse_minuit_sans_se_tromper() {
        // Plage 22h-02h, il est 01h55 locales : 5 minutes restent, pas 1435.
        let h = horaire(vec![plage(22, 0, 2, 0)]);
        let p = h.ranges[0];
        assert_eq!(p.minutes_until_end(hm(1, 55)), Some(5));
        assert_eq!(p.minutes_until_end(hm(23, 0)), Some(180));
    }

    #[test]
    fn la_fin_de_session_eteint_et_ne_rouvre_plus() {
        // Sans cette porte, le serveur ressusciterait chaque jour a 12h.
        let mut h = horaire(vec![plage(12, 0, 14, 0)]);
        h.closes_at = Some(utc(19, 9, 0));

        assert_eq!(
            decide(&h, true, false, utc(19, 10, 30)),
            ScheduleAction::Stop {
                reason: StopReason::SessionOver
            }
        );
        // Deja eteint : on n'insiste pas, et on ne redemarre surtout pas.
        assert_eq!(
            decide(&h, false, false, utc(19, 10, 30)),
            ScheduleAction::Nothing
        );
        assert_eq!(
            decide(&h, false, false, utc(20, 10, 30)),
            ScheduleAction::Nothing
        );
    }

    #[test]
    fn desactive_le_systeme_ne_touche_a_rien() {
        // L'administrateur garde la main : hors plage, un serveur qu'il a
        // demarre lui-meme doit le rester.
        let mut h = horaire(vec![plage(12, 0, 14, 0)]);
        h.enabled = false;
        assert_eq!(
            decide(&h, true, false, utc(19, 20, 0)),
            ScheduleAction::Nothing
        );
        assert_eq!(
            decide(&h, false, false, utc(19, 10, 30)),
            ScheduleAction::Nothing
        );
    }

    #[test]
    fn un_fuseau_inconnu_ne_declenche_rien() {
        // Retomber sur UTC en silence ferait tourner le serveur a contretemps
        // — deux heures a cote en ete, sans que personne ne comprenne.
        let mut h = horaire(vec![plage(12, 0, 14, 0)]);
        h.timezone = "Mars/Olympus".into();
        assert_eq!(
            decide(&h, false, false, utc(19, 10, 30)),
            ScheduleAction::Nothing
        );
    }

    #[test]
    fn la_prochaine_ouverture_est_annoncable() {
        let h = horaire(vec![plage(12, 0, 14, 0), plage(19, 0, 23, 0)]);

        // 08h UTC = 10h locales : la prochaine ouverture est a 12h locales.
        let suivante = next_opening(&h, utc(19, 8, 0)).unwrap();
        assert_eq!(suivante, utc(19, 10, 0));

        // 13h UTC = 15h locales : la plage de midi est passee, reste 19h.
        let suivante = next_opening(&h, utc(19, 13, 0)).unwrap();
        assert_eq!(suivante, utc(19, 17, 0));

        // Apres la derniere plage du jour : on bascule au lendemain.
        let suivante = next_opening(&h, utc(19, 22, 0)).unwrap();
        assert_eq!(suivante, utc(20, 10, 0));
    }

    #[test]
    fn aucune_ouverture_apres_la_fin_de_session() {
        let mut h = horaire(vec![plage(12, 0, 14, 0)]);
        h.closes_at = Some(utc(19, 9, 0));
        assert_eq!(next_opening(&h, utc(19, 8, 0)), None);
    }

    // ── Mode permanence : redemarrages periodiques ──
    //
    // Repere pour tout ce qui suit : en aout, Paris est a UTC+2. Avec un
    // intervalle de 3 h ancre a l'heure pile, les creneaux locaux tombent a
    // 0h/3h/6h/9h/12h/15h/18h/21h, soit 13h UTC pour le creneau de 15h locales.

    #[test]
    fn le_mode_se_lit_et_s_ecrit() {
        assert_eq!(ScheduleMode::Ranges.as_str(), "ranges");
        assert_eq!(ScheduleMode::Restart.as_str(), "restart");
        assert_eq!(ScheduleMode::from_str("restart"), ScheduleMode::Restart);
        assert_eq!(ScheduleMode::from_str("ranges"), ScheduleMode::Ranges);
        // Fail closed : l'inconnu retombe sur le mode qui ne coupe rien seul.
        assert_eq!(
            ScheduleMode::from_str("n_importe_quoi"),
            ScheduleMode::Ranges
        );
    }

    #[test]
    fn les_creneaux_couvrent_la_journee_sans_deriver() {
        // 3 h : huit creneaux, du premier a minuit au dernier a 21h.
        assert_eq!(
            restart_slots_of_day(3, 0),
            vec![0, 180, 360, 540, 720, 900, 1080, 1260]
        );
        // 24 h : un seul rendez-vous par jour.
        assert_eq!(restart_slots_of_day(24, 0), vec![0]);
        // 1 h : vingt-quatre.
        assert_eq!(restart_slots_of_day(1, 0).len(), 24);
    }

    #[test]
    fn un_intervalle_qui_ne_divise_pas_la_journee_ne_donne_aucun_creneau() {
        // 5 h deriverait d'un jour a l'autre : « redemarrage a 20h » cesserait
        // d'etre vrai des le lendemain. Mieux vaut aucun creneau qu'un creneau
        // mouvant.
        assert!(restart_slots_of_day(5, 0).is_empty());
        assert!(restart_slots_of_day(7, 0).is_empty());
        assert!(restart_slots_of_day(0, 0).is_empty());
        assert!(restart_slots_of_day(3, 60).is_empty());
    }

    #[test]
    fn l_ancre_decale_tous_les_creneaux_de_la_meme_minute() {
        assert_eq!(restart_slots_of_day(6, 30), vec![30, 390, 750, 1110]);
    }

    #[test]
    fn le_preavis_part_a_l_avance_choisie() {
        let p = permanence(3); // preavis 15 min
                               // 14h45 locales = 12h45 UTC : 15 min avant le creneau de 15h.
        assert_eq!(
            decide(&p, true, false, utc(19, 12, 45)),
            ScheduleAction::RestartWarn { minutes_left: 15 }
        );
        // Une minute plus tot, la fenetre n'est pas encore ouverte.
        assert_eq!(
            decide(&p, true, false, utc(19, 12, 44)),
            ScheduleAction::Nothing
        );
    }

    #[test]
    fn le_preavis_est_parametrable() {
        let mut p = permanence(3);
        p.warn_minutes = 30;
        // 14h30 locales : 30 min avant, la fenetre s'ouvre maintenant.
        assert_eq!(
            decide(&p, true, false, utc(19, 12, 30)),
            ScheduleAction::RestartWarn { minutes_left: 30 }
        );

        // A zero, plus aucune annonce anticipee — mais l'annonce finale reste.
        p.warn_minutes = 0;
        assert_eq!(
            decide(&p, true, false, utc(19, 12, 30)),
            ScheduleAction::Nothing
        );
        assert_eq!(
            decide(&p, true, false, utc(19, 12, 59)),
            ScheduleAction::RestartFinalWarn
        );
    }

    #[test]
    fn le_preavis_ne_se_repete_pas_a_chaque_passage() {
        // Sans cette memoire, les joueurs recevraient le meme message chaque
        // minute pendant tout le preavis.
        let mut p = permanence(3);
        p.last_warned_at = Some(utc(19, 12, 45));
        assert_eq!(
            decide(&p, true, false, utc(19, 12, 50)),
            ScheduleAction::Nothing
        );
    }

    #[test]
    fn un_preavis_du_creneau_precedent_ne_bloque_pas_le_suivant() {
        // Le piege : avec un intervalle de 3 h, une annonce vieille d'une heure
        // appartient au creneau d'avant et ne doit pas valoir pour celui-ci.
        let mut p = permanence(3);
        p.last_warned_at = Some(utc(19, 9, 45)); // preavis du creneau de 12h locales
        assert_eq!(
            decide(&p, true, false, utc(19, 12, 45)),
            ScheduleAction::RestartWarn { minutes_left: 15 }
        );
    }

    #[test]
    fn l_annonce_finale_part_une_minute_avant_la_coupure() {
        let p = permanence(3);
        assert_eq!(
            decide(&p, true, false, utc(19, 12, 59)),
            ScheduleAction::RestartFinalWarn
        );
    }

    #[test]
    fn l_annonce_finale_ne_se_repete_pas() {
        let mut p = permanence(3);
        p.last_final_warned_at = Some(utc(19, 12, 59));
        assert_eq!(
            decide(&p, true, false, utc(19, 12, 59)),
            ScheduleAction::Nothing
        );
    }

    #[test]
    fn les_deux_preavis_ne_s_annulent_pas_l_un_l_autre() {
        // Le preavis a quinze minutes est parti ; l'annonce finale doit partir
        // quand meme. Un seul marqueur pour les deux aurait avale la seconde.
        let mut p = permanence(3);
        p.last_warned_at = Some(utc(19, 12, 45));
        assert_eq!(
            decide(&p, true, false, utc(19, 12, 59)),
            ScheduleAction::RestartFinalWarn
        );
    }

    #[test]
    fn le_redemarrage_tombe_sur_le_creneau() {
        let p = permanence(3);
        // 15h00 locales = 13h00 UTC, pile.
        assert_eq!(
            decide(&p, true, false, utc(19, 13, 0)),
            ScheduleAction::Restart
        );
    }

    #[test]
    fn un_redemarrage_deja_fait_ne_se_rejoue_pas() {
        // Le passage periodique repasse quelques minutes plus tard : sans cette
        // memoire, le serveur redemarrerait en boucle pendant tout le creneau.
        let mut p = permanence(3);
        p.last_restart_at = Some(utc(19, 13, 0));
        assert_eq!(
            decide(&p, true, false, utc(19, 13, 5)),
            ScheduleAction::Nothing
        );
    }

    #[test]
    fn un_creneau_manque_est_rattrape_puis_abandonne() {
        let p = permanence(3);
        // Huit minutes de retard : API redemarree, verrou tenu ailleurs. On
        // rattrape plutot que de sauter le redemarrage.
        assert_eq!(
            decide(&p, true, false, utc(19, 13, 8)),
            ScheduleAction::Restart
        );
        // Au-dela de la tolerance, on renonce : couper a une heure que
        // personne n'a annoncee serait pire que de sauter un tour.
        assert_eq!(
            decide(&p, true, false, utc(19, 13, 11)),
            ScheduleAction::Nothing
        );
    }

    #[test]
    fn la_permanence_rallume_un_serveur_eteint() {
        // C'est ce qui fait la permanence : 24 h sur 24, meme apres un arret.
        let p = permanence(6);
        assert_eq!(
            decide(&p, false, false, utc(19, 10, 30)),
            ScheduleAction::Start
        );
    }

    #[test]
    fn sans_intervalle_la_permanence_ne_coupe_jamais() {
        // Fail closed : on ne devine pas une cadence de redemarrage.
        let mut p = permanence(3);
        p.restart_interval_hours = None;
        assert_eq!(
            decide(&p, true, false, utc(19, 13, 0)),
            ScheduleAction::Nothing
        );
        assert_eq!(next_restart_at(&p, utc(19, 13, 0)), None);
    }

    #[test]
    fn en_permanence_les_plages_ne_ferment_plus_rien() {
        // Le coeur de l'exclusion : des plages restees en base ne doivent plus
        // eteindre un serveur passe en permanence.
        let mut p = permanence(3);
        p.ranges = vec![plage(12, 0, 14, 0)];
        // 20h locales = 18h UTC : hors plage. En mode plages, ce serait un Stop.
        let action = decide(&p, true, false, utc(19, 18, 0));
        assert_ne!(
            action,
            ScheduleAction::Stop {
                reason: StopReason::OutsideRange
            }
        );
        assert_eq!(action, ScheduleAction::Nothing);
    }

    #[test]
    fn en_mode_plages_aucun_redemarrage_periodique() {
        // La reciproque : un intervalle reste en base ne doit rien declencher
        // tant que le mode est « plages ».
        let mut h = horaire(vec![plage(12, 0, 23, 0)]);
        h.restart_interval_hours = Some(3);
        assert_eq!(
            decide(&h, true, false, utc(19, 13, 0)),
            ScheduleAction::Nothing
        );
        assert_eq!(next_restart_at(&h, utc(19, 13, 0)), None);
    }

    #[test]
    fn la_fin_de_session_arrete_aussi_une_permanence() {
        let mut p = permanence(3);
        p.closes_at = Some(utc(19, 9, 0));
        assert_eq!(
            decide(&p, true, false, utc(19, 10, 0)),
            ScheduleAction::Stop {
                reason: StopReason::SessionOver
            }
        );
        // Et elle ne rallume plus rien, malgre la permanence.
        assert_eq!(
            decide(&p, false, false, utc(19, 10, 0)),
            ScheduleAction::Nothing
        );
    }

    #[test]
    fn une_permanence_desactivee_ne_touche_a_rien() {
        let mut p = permanence(3);
        p.enabled = false;
        assert_eq!(
            decide(&p, true, false, utc(19, 13, 0)),
            ScheduleAction::Nothing
        );
        assert_eq!(
            decide(&p, false, false, utc(19, 13, 0)),
            ScheduleAction::Nothing
        );
        assert_eq!(next_restart_at(&p, utc(19, 13, 0)), None);
    }

    #[test]
    fn un_fuseau_inconnu_ne_declenche_aucun_redemarrage() {
        let mut p = permanence(3);
        p.timezone = "Mars/Olympus".into();
        assert_eq!(
            decide(&p, true, false, utc(19, 13, 0)),
            ScheduleAction::Nothing
        );
        assert_eq!(next_restart_at(&p, utc(19, 13, 0)), None);
    }

    #[test]
    fn le_prochain_creneau_est_annoncable() {
        let p = permanence(3);
        // 12h50 UTC = 14h50 locales : le prochain est 15h locales = 13h UTC.
        assert_eq!(next_restart_at(&p, utc(19, 12, 50)), Some(utc(19, 13, 0)));
        // Pile sur un creneau, on annonce deja le suivant (18h locales).
        assert_eq!(next_restart_at(&p, utc(19, 13, 0)), Some(utc(19, 16, 0)));
    }

    #[test]
    fn le_creneau_de_minuit_traverse_la_frontiere_de_date() {
        // Piege du fuseau : minuit local tombe la VEILLE en UTC. Sans balayer
        // le jour d'avant et celui d'apres, ce creneau serait invisible.
        let p = permanence(24); // un seul creneau : minuit local
                                // 23h UTC le 19 = 1h locales le 20 : le creneau vient de passer (22h UTC).
        assert_eq!(
            previous_restart_at(&p, utc(19, 23, 0)),
            Some(utc(19, 22, 0))
        );
        assert_eq!(next_restart_at(&p, utc(19, 23, 0)), Some(utc(20, 22, 0)));
    }

    #[test]
    fn aucun_redemarrage_annonce_apres_la_fin_de_session() {
        let mut p = permanence(3);
        p.closes_at = Some(utc(19, 12, 0));
        assert_eq!(next_restart_at(&p, utc(19, 11, 0)), None);
    }

    #[test]
    fn le_preavis_ne_saute_pas_le_redemarrage() {
        // L'ordre des controles : a la minute du creneau, le prochain creneau
        // est deja celui d'apres. Si les annonces passaient avant, la coupure
        // serait repoussee d'un tour entier.
        let mut p = permanence(1); // creneaux toutes les heures
        p.warn_minutes = 59; // fenetre de preavis quasi permanente
        assert_eq!(
            decide(&p, true, false, utc(19, 13, 0)),
            ScheduleAction::Restart
        );
    }
}
