//! Transport HTTP commun vers `docker-agent`.

use std::fmt;
use std::time::Duration;

use reqwest::{Client, RequestBuilder, StatusCode};
use serde::de::DeserializeOwned;

#[derive(Debug)]
pub enum DockerAgentError {
    Transport(reqwest::Error),
    Rejected { status: StatusCode, detail: String },
    InvalidResponse(reqwest::Error),
}

impl fmt::Display for DockerAgentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(_) => formatter.write_str("docker-agent injoignable"),
            Self::Rejected { status, .. } => write!(formatter, "docker-agent: reponse {status}"),
            Self::InvalidResponse(_) => formatter.write_str("reponse docker-agent illisible"),
        }
    }
}

pub struct DockerAgentClient {
    client: Client,
    base_url: String,
    token: String,
}

impl DockerAgentClient {
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

    pub fn get(&self, path: &str) -> RequestBuilder {
        self.client.get(self.url(path))
    }

    pub fn post(&self, path: &str) -> RequestBuilder {
        self.client.post(self.url(path))
    }

    pub async fn probe(&self, path: &str, timeout: Duration) -> bool {
        self.authorize(self.get(path).timeout(timeout))
            .send()
            .await
            .is_ok_and(|response| response.status().is_success())
    }

    pub async fn send_json<T: DeserializeOwned>(
        &self,
        request: RequestBuilder,
    ) -> Result<T, DockerAgentError> {
        let response = self.send(request).await?;
        response
            .json::<T>()
            .await
            .map_err(DockerAgentError::InvalidResponse)
    }

    pub async fn send_unit(&self, request: RequestBuilder) -> Result<(), DockerAgentError> {
        self.send(request).await.map(|_| ())
    }

    pub async fn send_text(&self, request: RequestBuilder) -> Result<String, DockerAgentError> {
        let response = self.send(request).await?;
        response
            .text()
            .await
            .map_err(DockerAgentError::InvalidResponse)
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn authorize(&self, request: RequestBuilder) -> RequestBuilder {
        request.bearer_auth(&self.token)
    }

    async fn send(&self, request: RequestBuilder) -> Result<reqwest::Response, DockerAgentError> {
        let response = self
            .authorize(request)
            .send()
            .await
            .map_err(DockerAgentError::Transport)?;
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }
        let detail = response.text().await.unwrap_or_default();
        Err(DockerAgentError::Rejected { status, detail })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise_la_base_et_joint_les_chemins() {
        let client = DockerAgentClient::new(
            "http://docker-agent:8095/".into(),
            "secret".into(),
            Duration::from_secs(1),
        );
        assert_eq!(
            client.url("/containers"),
            "http://docker-agent:8095/containers"
        );
        let request = client.get("/version").build().unwrap();
        assert_eq!(request.url().as_str(), "http://docker-agent:8095/version");
    }
}
