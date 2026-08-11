use super::*;

impl ApiClient {
    pub async fn challenge_coussin(
        &self,
        guild_id: &str,
        body: &CoussinChallengeRequest,
    ) -> Result<CoussinChallengeResponse, String> {
        let url = format!(
            "{}/api/coussin/{}/combats",
            self.base_url,
            encode_segment(guild_id)
        );
        self.send(self.http.post(url).json(body)).await
    }

    pub async fn accept_coussin(&self, id: &str, defender_id: &str) -> Result<bool, String> {
        let url = format!(
            "{}/api/coussin/combats/{}/accept",
            self.base_url,
            encode_segment(id)
        );
        let response: serde_json::Value = self
            .send(self.http.post(url).json(&CoussinDefenderRequest {
                defender_id: defender_id.into(),
            }))
            .await?;
        response["ok"]
            .as_bool()
            .ok_or_else(|| "reponse nexus-api invalide".into())
    }

    pub async fn refuse_coussin(&self, id: &str, defender_id: &str) -> Result<bool, String> {
        let url = format!(
            "{}/api/coussin/combats/{}/refuse",
            self.base_url,
            encode_segment(id)
        );
        let response: serde_json::Value = self
            .send(self.http.post(url).json(&CoussinDefenderRequest {
                defender_id: defender_id.into(),
            }))
            .await?;
        response["ok"]
            .as_bool()
            .ok_or_else(|| "reponse nexus-api invalide".into())
    }

    pub async fn resolve_coussin(&self, id: &str) -> Result<bool, String> {
        let url = format!(
            "{}/api/coussin/combats/{}/resolve",
            self.base_url,
            encode_segment(id)
        );
        let response: serde_json::Value = self.send(self.http.post(url)).await?;
        response["ok"]
            .as_bool()
            .ok_or_else(|| "reponse nexus-api invalide".into())
    }

    pub async fn coussin_profile(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
    ) -> Result<CoussinProfileResponse, String> {
        let url = format!(
            "{}/api/coussin/{}/{}/profile?username={}",
            self.base_url,
            encode_segment(guild_id),
            encode_segment(user_id),
            encode_segment(username)
        );
        self.send(self.http.get(url)).await
    }
    pub async fn choose_coussin_class(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        class: &str,
    ) -> Result<CoussinProfileResponse, String> {
        let url = format!(
            "{}/api/coussin/{}/{}/class",
            self.base_url,
            encode_segment(guild_id),
            encode_segment(user_id)
        );
        self.send(self.http.post(url).json(&CoussinClassRequest {
            username: username.into(),
            class: class.into(),
        }))
        .await
    }
    pub async fn train_coussin(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        stat: &str,
    ) -> Result<CoussinProfileResponse, String> {
        let url = format!(
            "{}/api/coussin/{}/{}/train",
            self.base_url,
            encode_segment(guild_id),
            encode_segment(user_id)
        );
        self.send(self.http.post(url).json(&CoussinTrainRequest {
            username: username.into(),
            stat: stat.into(),
        }))
        .await
    }
    pub async fn buy_coussin_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<i64, String> {
        let url = format!(
            "{}/api/coussin/{}/{}/shop",
            self.base_url,
            encode_segment(guild_id),
            encode_segment(user_id)
        );
        let value: serde_json::Value = self
            .send(self.http.post(url).json(&CoussinBuyItemRequest {
                item_key: item_key.into(),
            }))
            .await?;
        value["balance_after"]
            .as_i64()
            .ok_or_else(|| "reponse nexus-api invalide".into())
    }
    pub async fn buy_coussin_insurance(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(bool, String), String> {
        let url = format!(
            "{}/api/coussin/{}/{}/insurance",
            self.base_url,
            encode_segment(guild_id),
            encode_segment(user_id)
        );
        let value: serde_json::Value = self.send(self.http.post(url)).await?;
        Ok((
            value["is_scam"]
                .as_bool()
                .ok_or_else(|| "reponse nexus-api invalide".to_string())?,
            value["expires_at"].as_str().unwrap_or("").to_string(),
        ))
    }
    pub async fn steal_coussin(
        &self,
        guild: &str,
        user: &str,
        body: &CoussinStealRequest,
    ) -> Result<(bool, i64), String> {
        let url = format!(
            "{}/api/coussin/{}/{}/steal",
            self.base_url,
            encode_segment(guild),
            encode_segment(user)
        );
        let v: serde_json::Value = self.send(self.http.post(url).json(body)).await?;
        Ok((
            v["success"].as_bool().unwrap_or(false),
            v["amount"].as_i64().unwrap_or(0),
        ))
    }
    pub async fn prime_coussin(
        &self,
        guild: &str,
        user: &str,
        body: &CoussinPrimeRequest,
    ) -> Result<(), String> {
        let url = format!(
            "{}/api/coussin/{}/{}/prime",
            self.base_url,
            encode_segment(guild),
            encode_segment(user)
        );
        let _: serde_json::Value = self.send(self.http.post(url).json(body)).await?;
        Ok(())
    }
    pub async fn inventory_coussin(
        &self,
        guild: &str,
        user: &str,
    ) -> Result<Vec<CoussinInventoryItem>, String> {
        let url = format!(
            "{}/api/coussin/{}/{}/inventory",
            self.base_url,
            encode_segment(guild),
            encode_segment(user)
        );
        self.send(self.http.get(url)).await
    }
    pub async fn bet_coussin(
        &self,
        guild: &str,
        user: &str,
        body: &CoussinBetRequest,
    ) -> Result<(), String> {
        let url = format!(
            "{}/api/coussin/{}/{}/bets",
            self.base_url,
            encode_segment(guild),
            encode_segment(user)
        );
        let _: serde_json::Value = self.send(self.http.post(url).json(body)).await?;
        Ok(())
    }
}
