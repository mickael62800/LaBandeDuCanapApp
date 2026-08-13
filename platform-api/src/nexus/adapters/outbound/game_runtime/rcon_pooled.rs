//! Implémentation d'un Pool de connexions RCON persistantes (PooledRconClient).
//!
//! Évite d'ouvrir/fermer une socket TCP et de ré-authentifier à chaque commande RCON.
//! Les connexions actives sont réutilisées si elles sont saines, et purgées en cas
//! d'inactivité ou d'erreur réseau.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration, Instant};

use platform_core::nexus::domain::errors::DomainError;
use platform_core::nexus::ports::outbound::game::rcon_client::{
    RconClient, RconConnectionParams, RconResponse,
};

const PKT_TYPE_LOGIN: i32 = 3;
const PKT_TYPE_COMMAND: i32 = 2;
const REQ_ID: i32 = 1;
const DEFAULT_TTL_SECS: u64 = 60;

fn build_packet(req_id: i32, pkt_type: i32, payload: &str) -> Vec<u8> {
    let payload_bytes = payload.as_bytes();
    let length: i32 = (4 + 4 + payload_bytes.len() + 2) as i32;
    let mut buf = Vec::with_capacity(4 + length as usize);
    buf.extend_from_slice(&length.to_le_bytes());
    buf.extend_from_slice(&req_id.to_le_bytes());
    buf.extend_from_slice(&pkt_type.to_le_bytes());
    buf.extend_from_slice(payload_bytes);
    buf.push(0);
    buf.push(0);
    buf
}

async fn read_packet(stream: &mut TcpStream) -> Result<(i32, i32, String), DomainError> {
    let mut len_buf = [0u8; 4];
    stream
        .read_exact(&mut len_buf)
        .await
        .map_err(|e| DomainError::Internal(format!("rcon read length: {e}")))?;
    let length = i32::from_le_bytes(len_buf);
    if !(10..=4096 + 14).contains(&length) {
        return Err(DomainError::Internal(format!(
            "rcon paquet taille invalide: {length}"
        )));
    }
    let mut buf = vec![0u8; length as usize];
    stream
        .read_exact(&mut buf)
        .await
        .map_err(|e| DomainError::Internal(format!("rcon read body: {e}")))?;
    let req_id = i32::from_le_bytes(buf[0..4].try_into().unwrap());
    let pkt_type = i32::from_le_bytes(buf[4..8].try_into().unwrap());
    let payload_end = buf
        .iter()
        .skip(8)
        .position(|b| *b == 0)
        .map(|p| p + 8)
        .unwrap_or(buf.len() - 2);
    let payload = String::from_utf8_lossy(&buf[8..payload_end]).into_owned();
    Ok((req_id, pkt_type, payload))
}

struct PooledConnection {
    stream: TcpStream,
    last_used: Instant,
}

#[derive(Clone)]
pub struct PooledRconClient {
    connections: Arc<Mutex<HashMap<String, PooledConnection>>>,
    ttl: Duration,
}

impl Default for PooledRconClient {
    fn default() -> Self {
        Self::new(Duration::from_secs(DEFAULT_TTL_SECS))
    }
}

impl PooledRconClient {
    pub fn new(ttl: Duration) -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            ttl,
        }
    }

    async fn connect_and_auth(
        &self,
        params: &RconConnectionParams,
    ) -> Result<TcpStream, DomainError> {
        let dur = Duration::from_secs(params.timeout_secs.max(1) as u64);
        let mut stream = timeout(dur, TcpStream::connect((params.host.as_str(), params.port)))
            .await
            .map_err(|_| DomainError::Internal("rcon connect timeout".into()))?
            .map_err(|e| DomainError::Internal(format!("rcon connect: {e}")))?;

        let auth_pkt = build_packet(REQ_ID, PKT_TYPE_LOGIN, &params.password);
        stream
            .write_all(&auth_pkt)
            .await
            .map_err(|e| DomainError::Internal(format!("rcon write auth: {e}")))?;

        let (auth_id, _, _) = timeout(dur, read_packet(&mut stream))
            .await
            .map_err(|_| DomainError::Internal("rcon auth read timeout".into()))??;
        if auth_id == -1 {
            return Err(DomainError::ValidationError(
                "rcon auth refusee (mot de passe incorrect)".into(),
            ));
        }

        Ok(stream)
    }

    async fn send_command_on_stream(
        stream: &mut TcpStream,
        command: &str,
        timeout_secs: u32,
    ) -> Result<String, DomainError> {
        let dur = Duration::from_secs(timeout_secs.max(1) as u64);
        let cmd_pkt = build_packet(REQ_ID, PKT_TYPE_COMMAND, command);
        stream
            .write_all(&cmd_pkt)
            .await
            .map_err(|e| DomainError::Internal(format!("rcon write cmd: {e}")))?;

        let (_, _, payload) = timeout(dur, read_packet(stream))
            .await
            .map_err(|_| DomainError::Internal("rcon cmd read timeout".into()))??;

        Ok(payload)
    }
}

#[async_trait]
impl RconClient for PooledRconClient {
    async fn execute(
        &self,
        params: &RconConnectionParams,
        command: &str,
    ) -> Result<RconResponse, DomainError> {
        let key = format!("{}:{}:{}", params.host, params.port, params.password);

        // 1. Tenter d'extraire une connexion existante du pool si elle n'a pas expiré
        let existing_stream = {
            let mut guard = self.connections.lock().await;
            if let Some(pooled) = guard.remove(&key) {
                if pooled.last_used.elapsed() < self.ttl {
                    Some(pooled.stream)
                } else {
                    None
                }
            } else {
                None
            }
        };

        // 2. Si une connexion existe, essayer d'exécuter la commande dessus
        if let Some(mut stream) = existing_stream {
            match Self::send_command_on_stream(&mut stream, command, params.timeout_secs).await {
                Ok(raw) => {
                    // Re-mettre la connexion saine dans le pool
                    let mut guard = self.connections.lock().await;
                    guard.insert(
                        key,
                        PooledConnection {
                            stream,
                            last_used: Instant::now(),
                        },
                    );
                    return Ok(RconResponse { raw });
                }
                Err(_) => {
                    // La connexion réutilisée était cassée (serveur l'a fermée) -> Ré-essayer avec une nouvelle connexion ci-dessous
                }
            }
        }

        // 3. Ouvrir une nouvelle connexion et authentifier
        let mut new_stream = self.connect_and_auth(params).await?;
        let raw =
            Self::send_command_on_stream(&mut new_stream, command, params.timeout_secs).await?;

        // 4. Stocker la nouvelle connexion saine dans le pool
        let mut guard = self.connections.lock().await;
        guard.insert(
            key,
            PooledConnection {
                stream: new_stream,
                last_used: Instant::now(),
            },
        );

        Ok(RconResponse { raw })
    }
}
