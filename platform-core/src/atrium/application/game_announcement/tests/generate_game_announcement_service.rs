use std::sync::Arc;

use super::*;
use crate::atrium::ports::outbound::AiProviderError;

/// Passerelle IA doublee : rend ce qu'on lui dit de rendre.
struct IaDoublee(Result<String, AiProviderError>);

#[async_trait]
impl WelcomeAiGateway for IaDoublee {
    async fn generate(&self, _: WelcomePrompt) -> Result<String, AiProviderError> {
        match &self.0 {
            Ok(texte) => Ok(texte.clone()),
            Err(_) => Err(AiProviderError),
        }
    }
}

fn service(reponse: Result<String, AiProviderError>) -> GameAnnouncementService {
    GameAnnouncementService::new(Arc::new(IaDoublee(reponse)))
}

fn demande() -> GameAnnouncementRequest {
    GameAnnouncementRequest {
        guild_id: "123456789012345678".into(),
        game_name: "Project Zomboid".into(),
        server_name: "Le Canap sur Zomboid".into(),
        max_players: Some(10),
        opening_label: Some("vendredi 29 aout a 19h".into()),
        schedule_label: None,
        admin_context: String::new(),
        rules: None,
    }
}

#[tokio::test]
async fn le_texte_du_modele_est_rendu_tel_quel() {
    let s = service(Ok("  Les morts arrivent. Amenez du bandage.  ".into()));
    let annonce = s.announce(demande()).await.unwrap();
    assert_eq!(annonce.content, "Les morts arrivent. Amenez du bandage.");
}

/// LE POINT CENTRAL DE LA DECISION PRISE : pas de repli. Une panne doit
/// remonter, pour que Nexus s'abstienne de poster et retente plus tard.
#[tokio::test]
async fn une_panne_de_l_ia_ne_produit_aucun_texte_de_secours() {
    let s = service(Err(AiProviderError));
    assert_eq!(
        s.announce(demande()).await,
        Err(GameAnnouncementError::Unavailable)
    );
}

/// Une reponse vide est une panne deguisee : la servir posterait un message
/// blanc avant le panneau d'inscription.
#[tokio::test]
async fn une_reponse_vide_vaut_une_panne() {
    for vide in ["", "   ", "\n\n"] {
        let s = service(Ok(vide.into()));
        assert_eq!(
            s.announce(demande()).await,
            Err(GameAnnouncementError::Unavailable),
            "reponse {vide:?} acceptee a tort"
        );
    }
}

/// Une demande invalide ne se repare PAS en reessayant : elle doit se
/// distinguer de l'indisponibilite, sinon la reprise tournerait en boucle sur
/// une erreur qui ne passera jamais.
#[tokio::test]
async fn une_demande_invalide_est_distincte_d_une_panne() {
    let mut d = demande();
    d.game_name = String::new();
    let s = service(Ok("peu importe".into()));

    assert_eq!(
        s.announce(d).await,
        Err(GameAnnouncementError::Missing("game_name"))
    );
}

#[tokio::test]
async fn un_texte_trop_long_est_tronque_sur_une_phrase() {
    let long = format!("{} FIN.", "Une phrase de remplissage. ".repeat(80));
    let s = service(Ok(long));
    let annonce = s.announce(demande()).await.unwrap();

    assert!(annonce.content.chars().count() <= MAX_ANNOUNCEMENT_CHARS);
    assert!(annonce.content.ends_with('.'));
}

/// Le prompt doit porter les faits, et seulement eux : c'est ce qui empeche le
/// modele d'inventer une adresse ou un horaire.
#[test]
fn le_prompt_porte_les_faits_fournis() {
    let prompt = build_prompt(&demande());

    assert!(prompt.user.contains("Project Zomboid"));
    assert!(prompt.user.contains("Joueurs maximum : 10"));
    assert!(prompt.user.contains("vendredi 29 aout a 19h"));
    assert!(!prompt.user.contains("Horaires"));
    assert!(prompt.system.contains("N'INVENTE AUCUN FAIT"));
}

#[test]
fn la_consigne_du_serveur_remplace_le_ton_par_defaut() {
    let mut d = demande();
    d.admin_context = "Sois franchement sarcastique.".into();
    let prompt = build_prompt(&d);

    assert!(prompt.system.contains("Sois franchement sarcastique."));
    assert!(!prompt.system.contains("TON PAR DEFAUT"));
}

#[test]
fn sans_consigne_un_ton_par_defaut_est_pose() {
    let prompt = build_prompt(&demande());
    assert!(prompt.system.contains("TON PAR DEFAUT"));
}

// ── Reglement ──────────────────────────────────────────────────────────────

#[test]
fn le_reglement_entre_dans_le_prompt_comme_contexte() {
    let mut d = demande();
    d.rules = Some("Pas de PvP hors de la zone rouge.".into());
    let prompt = build_prompt(&d);

    assert!(prompt.system.contains("Pas de PvP hors de la zone rouge."));
    // L'interdiction de recopier compte autant que le texte lui-meme : sans
    // elle, le modele reprendrait le reglement et il apparaitrait deux fois.
    assert!(prompt.system.contains("Ne le recopie pas"));
    assert!(prompt.system.contains("affiche INTEGRALEMENT"));
}

/// Le reglement ne doit pas partir dans le message utilisateur, ou il se
/// melerait aux faits a mettre en forme.
#[test]
fn le_reglement_ne_se_melange_pas_aux_faits() {
    let mut d = demande();
    d.rules = Some("Pas de PvP hors de la zone rouge.".into());

    assert!(!build_prompt(&d).user.contains("PvP"));
}

#[test]
fn sans_reglement_aucune_consigne_n_est_ajoutee() {
    let prompt = build_prompt(&demande());
    assert!(!prompt.system.contains("REGLEMENT"));

    let mut vide = demande();
    vide.rules = Some("   ".into());
    assert!(!build_prompt(&vide).system.contains("REGLEMENT"));
}
