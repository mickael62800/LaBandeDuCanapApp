//! Client HTTP minimal pour les workers qui reveillent une API interne.

use std::time::Duration;

use reqwest::{Client, RequestBuilder};
use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Clone)]
pub struct HttpJobClient {
    client: Client,
    base_url: String,
    token: String,
}

impl HttpJobClient {
    pub fn new(base_url: String, token: String, timeout: Duration) -> Self {
        Self {
            client: Client::builder()
                .timeout(timeout)
                .build()
                .unwrap_or_default(),
            base_url: base_url.trim_end_matches('/').to_owned(),
            token,
        }
    }

    pub async fn post_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let response = self.send(self.client.post(self.url(path))).await?;
        Self::decode_json(response).await
    }

    /// Variante avec un timeout propre a la requete pour les jobs longs.
    pub async fn post_json_with_timeout<T: DeserializeOwned>(
        &self,
        path: &str,
        timeout: Duration,
    ) -> Result<T, String> {
        let response = self
            .send(self.client.post(self.url(path)).timeout(timeout))
            .await?;
        Self::decode_json(response).await
    }

    /// Envoie un body JSON et decode la reponse JSON.
    pub async fn post_json_body<B, T>(&self, path: &str, body: &B) -> Result<T, String>
    where
        B: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let response = self
            .send(self.client.post(self.url(path)).json(body))
            .await?;
        Self::decode_json(response).await
    }

    /// Envoie un body JSON lorsque seul le statut HTTP compte.
    pub async fn post_json_unit<B>(&self, path: &str, body: &B) -> Result<(), String>
    where
        B: Serialize + ?Sized,
    {
        self.send(self.client.post(self.url(path)).json(body))
            .await
            .map(|_| ())
    }

    async fn decode_json<T: DeserializeOwned>(response: reqwest::Response) -> Result<T, String> {
        response
            .json::<T>()
            .await
            .map_err(|error| format!("decode reponse: {error}"))
    }

    pub async fn post_empty(&self, path: &str) -> Result<(), String> {
        self.send(self.client.post(self.url(path)))
            .await
            .map(|_| ())
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    async fn send(&self, request: RequestBuilder) -> Result<reqwest::Response, String> {
        let request = if self.token.is_empty() {
            request
        } else {
            request.bearer_auth(&self.token)
        };
        let response = request
            .send()
            .await
            .map_err(|error| format!("HTTP send: {error}"))?;
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }
        let body = response.text().await.unwrap_or_default();
        Err(format!("HTTP {status}: {body}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise_la_base() {
        let client = HttpJobClient::new(
            "http://api:3000/".into(),
            String::new(),
            Duration::from_secs(1),
        );
        assert_eq!(client.url("/jobs/run"), "http://api:3000/jobs/run");
    }
}
