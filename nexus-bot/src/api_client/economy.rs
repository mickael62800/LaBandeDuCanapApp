use super::*;

impl ApiClient {
    pub async fn grand_salon_join(
        &self,
        guild_id: &str,
        user_id: &str,
        display_name: &str,
    ) -> Result<GrandSalonProfileResponse, String> {
        let url = format!(
            "{}/api/grand-salon/{}/habitues/{}",
            self.base_url,
            encode_segment(guild_id),
            encode_segment(user_id)
        );
        self.send(self.http.post(url).json(&GrandSalonJoinRequest {
            display_name: display_name.into(),
        }))
        .await
    }
    pub async fn grand_salon_profile(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<GrandSalonProfileResponse, String> {
        let url = format!(
            "{}/api/grand-salon/{}/habitues/{}",
            self.base_url,
            encode_segment(guild_id),
            encode_segment(user_id)
        );
        self.send(self.http.get(url)).await
    }
    /// GET /api/wallet/{guild_id}/{user_id}.
    pub async fn get_wallet(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<WalletResponse, String> {
        let url = format!("{}/api/wallet/{guild_id}/{user_id}", self.base_url);
        self.send(self.http.get(&url)).await
    }

    /// POST /api/wallet/{guild_id}/transfer.
    pub async fn transfer_coins(
        &self,
        guild_id: &str,
        req: &TransferRequest,
    ) -> Result<TransferResponse, String> {
        let url = format!("{}/api/wallet/{guild_id}/transfer", self.base_url);
        self.send(self.http.post(&url).json(req)).await
    }

    /// GET /api/wallet/{guild_id}/leaderboard?limit=N.
    pub async fn wallet_leaderboard(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<WalletResponse>, String> {
        let url = format!(
            "{}/api/wallet/{guild_id}/leaderboard?limit={limit}",
            self.base_url
        );
        self.send(self.http.get(&url)).await
    }

    /// POST /api/wheel/{guild_id}/{user_id}/spin.
    /// Err(message affichable) sur 4xx (ex: daily deja claim) ou erreur reseau.
    pub async fn spin_wheel(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
    ) -> Result<WheelSpinResponse, String> {
        let url = format!("{}/api/wheel/{guild_id}/{user_id}/spin", self.base_url);
        let mut req = self.http.post(&url).json(&WheelSpinRequest {
            username: username.to_string(),
        });
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| format!("nexus-api injoignable: {e}"))?;

        let status = resp.status();
        if status.is_success() {
            resp.json::<WheelSpinResponse>()
                .await
                .map_err(|e| format!("reponse nexus-api invalide: {e}"))
        } else {
            let msg = resp
                .json::<ApiErrorBody>()
                .await
                .map(|b| b.error)
                .unwrap_or_else(|_| format!("erreur nexus-api ({status})"));
            Err(msg)
        }
    }
}
