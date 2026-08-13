#![allow(dead_code)]

//! Implementation `RconClient` pour le protocole Source RCON (Minecraft Java).
//!
//! Format des paquets (little-endian) :
//!   i32 length (taille du reste du paquet)
//!   i32 request_id
//!   i32 type   (3 = LOGIN, 2 = COMMAND, 0 = RESPONSE_VALUE)
//!   bytes payload (terminated by null)
//!   byte 0x00 (terminator)
//!
//! Implementation minimaliste : connect TCP -> auth (type 3) -> command
//! (type 2) -> read response. Une seule commande par appel ; pas de pool.

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

use platform_core::nexus::domain::errors::DomainError;
use platform_core::nexus::ports::outbound::game::rcon_client::{
    RconClient, RconConnectionParams, RconResponse,
};

const PKT_TYPE_LOGIN: i32 = 3;
const PKT_TYPE_COMMAND: i32 = 2;
const REQ_ID: i32 = 1;

pub struct MinecraftRconClient;

impl Default for MinecraftRconClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MinecraftRconClient {
    pub fn new() -> Self {
        Self
    }
}

fn build_packet(req_id: i32, pkt_type: i32, payload: &str) -> Vec<u8> {
    let payload_bytes = payload.as_bytes();
    // length = 4 (id) + 4 (type) + payload_len + 2 (deux nuls finaux)
    let length: i32 = (4 + 4 + payload_bytes.len() + 2) as i32;
    let mut buf = Vec::with_capacity(4 + length as usize);
    buf.extend_from_slice(&length.to_le_bytes());
    buf.extend_from_slice(&req_id.to_le_bytes());
    buf.extend_from_slice(&pkt_type.to_le_bytes());
    buf.extend_from_slice(payload_bytes);
    buf.push(0); // null-terminator du payload
    buf.push(0); // null-terminator du paquet
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
    // Payload : du byte 8 jusqu'au double null final.
    let payload_end = buf
        .iter()
        .skip(8)
        .position(|b| *b == 0)
        .map(|p| p + 8)
        .unwrap_or(buf.len() - 2);
    let payload = String::from_utf8_lossy(&buf[8..payload_end]).into_owned();
    Ok((req_id, pkt_type, payload))
}

#[async_trait]
impl RconClient for MinecraftRconClient {
    async fn execute(
        &self,
        params: &RconConnectionParams,
        command: &str,
    ) -> Result<RconResponse, DomainError> {
        let dur = Duration::from_secs(params.timeout_secs.max(1) as u64);

        let mut stream = timeout(dur, TcpStream::connect((params.host.as_str(), params.port)))
            .await
            .map_err(|_| DomainError::Internal("rcon connect timeout".into()))?
            .map_err(|e| DomainError::Internal(format!("rcon connect: {e}")))?;

        // Auth
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

        // Command
        let cmd_pkt = build_packet(REQ_ID, PKT_TYPE_COMMAND, command);
        stream
            .write_all(&cmd_pkt)
            .await
            .map_err(|e| DomainError::Internal(format!("rcon write cmd: {e}")))?;

        let (_, _, payload) = timeout(dur, read_packet(&mut stream))
            .await
            .map_err(|_| DomainError::Internal("rcon cmd read timeout".into()))??;

        // Best-effort close (les serveurs Minecraft ne renvoient pas de bye).
        let _ = stream.shutdown().await;

        Ok(RconResponse { raw: payload })
    }
}
