use std::time::Duration;

use reqwest::{Client, RequestBuilder};
use serde::de::DeserializeOwned;

#[derive(Clone)]
pub struct HttpJobClient {
    client: Client,
    base_url: String,
    token: String,
}

impl HttpJobClient {
    pub fn new(base_url: String, token: String, timeout: Duration) -> Self {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_default();
        Self {
            client,
            base_url: base_url.trim_end_matches('/').into(),
            token,
        }
    }

    pub async fn post_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let response = self.send(self.client.post(self.url(path)), path).await?;
        response
            .json()
            .await
            .map_err(|error| format!("decode reponse: {error}"))
    }

    pub async fn post_json_with_timeout<T: DeserializeOwned>(
        &self,
        path: &str,
        timeout: Duration,
    ) -> Result<T, String> {
        let response = self
            .send(self.client.post(self.url(path)).timeout(timeout), path)
            .await?;
        response
            .json()
            .await
            .map_err(|error| format!("decode reponse: {error}"))
    }

    pub async fn post_empty(&self, path: &str) -> Result<(), String> {
        self.send(self.client.post(self.url(path)), path)
            .await
            .map(|_| ())
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    async fn send(&self, request: RequestBuilder, path: &str) -> Result<reqwest::Response, String> {
        let response = request
            .bearer_auth(&self.token)
            .header("x-scheduler-job", normalized_job_name(path))
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

fn normalized_job_name(path: &str) -> String {
    path.trim_matches('/').replace(['/', '_'], "-")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalizes_base_url() {
        let client = HttpJobClient::new(
            "http://api:3000/".into(),
            "secret".into(),
            Duration::from_secs(1),
        );
        assert_eq!(client.url("/jobs/run"), "http://api:3000/jobs/run");
    }

    #[test]
    fn normalizes_job_header() {
        assert_eq!(normalized_job_name("/api/internal/jobs/cleanup_old"), "api-internal-jobs-cleanup-old");
    }
}
