//! Tests du service d'edition des cases de la Roue.

use std::sync::Mutex;

use super::*;
use crate::nexus::domain::entities::wheel::WheelSpin;

#[derive(Default)]
struct MockRepo {
    cases: Mutex<Vec<WheelCaseData>>,
}

#[async_trait]
impl WheelRepository for MockRepo {
    async fn try_claim(&self, _g: &str, _u: &str, _h: i64) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn has_claimed_recently(&self, _g: &str, _u: &str, _h: i64) -> Result<bool, DomainError> {
        Ok(false)
    }
    async fn log_spin(&self, _spin: &WheelSpin) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_cases(&self, _g: &str) -> Result<Vec<WheelCaseData>, DomainError> {
        Ok(self.cases.lock().unwrap().clone())
    }
    async fn replace_cases(&self, _g: &str, cases: &[WheelCaseData]) -> Result<(), DomainError> {
        *self.cases.lock().unwrap() = cases.to_vec();
        Ok(())
    }
    async fn execute_spin_transaction(
        &self,
        _guild_id: &str,
        _user_id: &str,
        _cooldown_hours: i64,
        _spin: &WheelSpin,
        _wallet: &crate::nexus::domain::entities::wallet::Wallet,
        _mutation: Option<&crate::nexus::domain::entities::wallet::WalletMutation>,
    ) -> Result<bool, DomainError> {
        Ok(true)
    }
}

fn case(key: &str, weight: u32) -> WheelCaseData {
    WheelCaseData {
        key: key.into(),
        label: format!("Case {key}"),
        payout: 100,
        weight,
    }
}

#[tokio::test]
async fn sans_personnalisation_la_roue_historique_est_rendue() {
    let service = WheelCasesService::new(Arc::new(MockRepo::default()));
    let roue = service.list("g").await.unwrap();
    assert!(!roue.customized, "aucune ligne : la roue n'est pas custom");
    assert_eq!(roue.cases.len(), 10, "les dix cases d'origine");
}

#[tokio::test]
async fn une_roue_valide_est_enregistree_et_marquee_custom() {
    let service = WheelCasesService::new(Arc::new(MockRepo::default()));
    let roue = service
        .replace("g", vec![case("a", 1), case("b", 3)])
        .await
        .unwrap();
    assert!(roue.customized);
    assert_eq!(roue.cases.len(), 2);
}

#[tokio::test]
async fn une_case_de_poids_nul_est_refusee() {
    let service = WheelCasesService::new(Arc::new(MockRepo::default()));
    assert!(service.replace("g", vec![case("a", 0)]).await.is_err());
}

#[tokio::test]
async fn deux_cases_de_meme_cle_sont_refusees() {
    let service = WheelCasesService::new(Arc::new(MockRepo::default()));
    assert!(service
        .replace("g", vec![case("a", 1), case("a", 2)])
        .await
        .is_err());
}

/// Le geste « je reviens a la roue de base » : on efface, et la lecture
/// suivante rend les dix cases d'origine sans marquer la roue personnalisee.
#[tokio::test]
async fn vider_la_roue_ramene_la_roue_historique() {
    let service = WheelCasesService::new(Arc::new(MockRepo::default()));
    service.replace("g", vec![case("a", 1)]).await.unwrap();
    let roue = service.replace("g", vec![]).await.unwrap();
    assert!(!roue.customized);
    assert_eq!(roue.cases.len(), 10);
}

/// Une roue invalide ne doit RIEN ecrire : sinon un refus laisserait la roue
/// a moitie remplacee, dans un etat que personne n'a choisi.
#[tokio::test]
async fn un_refus_laisse_la_roue_precedente_intacte() {
    let repo = Arc::new(MockRepo::default());
    let service = WheelCasesService::new(repo.clone());
    service.replace("g", vec![case("a", 1)]).await.unwrap();
    let _ = service.replace("g", vec![case("b", 0)]).await;
    let roue = service.list("g").await.unwrap();
    assert_eq!(roue.cases.len(), 1);
    assert_eq!(roue.cases[0].key, "a");
}
