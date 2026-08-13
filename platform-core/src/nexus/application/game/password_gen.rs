//! Generation d'un mot de passe RCON robuste.
//!
//! Pas besoin de crate externe : on utilise rand de la stdlib via le RNG
//! crypto-grade fourni par OS (uuid::Uuid::new_v4 utilise getrandom). On
//! genere 32 chars alphanumeriques.

use uuid::Uuid;

const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

/// Genere un mot de passe RCON aleatoire (32 chars). Utilise uuid::Uuid::new_v4
/// comme source d'entropie crypto (getrandom), puis derive l'alphabet desire.
pub fn generate_rcon_password() -> String {
    let mut out = String::with_capacity(32);
    for _ in 0..2 {
        let raw = Uuid::new_v4().as_bytes().to_vec();
        for byte in raw {
            out.push(CHARSET[byte as usize % CHARSET.len()] as char);
            if out.len() >= 32 {
                return out;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rcon_password_length() {
        let p = generate_rcon_password();
        assert_eq!(p.len(), 32);
    }

    #[test]
    fn rcon_password_alphanumeric() {
        let p = generate_rcon_password();
        assert!(p.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn rcon_password_uniqueness() {
        let p1 = generate_rcon_password();
        let p2 = generate_rcon_password();
        assert_ne!(p1, p2);
    }
}
