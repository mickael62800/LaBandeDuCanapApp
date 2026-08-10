//! Tests de l'agregation fail2ban (`total_banned_ips`).

use super::*;

fn jail(name: &str, ips: &[&str]) -> Fail2banJail {
    Fail2banJail {
        name: name.to_string(),
        total_banned: ips.len() as i64,
        banned_ips: ips.iter().map(|s| s.to_string()).collect(),
    }
}

#[test]
fn total_banned_ips_empty() {
    let s = Fail2banStatus {
        updated_at: "now".to_string(),
        jails: vec![],
    };
    assert_eq!(s.total_banned_ips(), 0);
}

#[test]
fn total_banned_ips_sums_across_jails() {
    let s = Fail2banStatus {
        updated_at: "now".to_string(),
        jails: vec![
            jail("sshd", &["1.1.1.1", "2.2.2.2"]),
            jail("nginx", &["3.3.3.3"]),
            jail("empty", &[]),
        ],
    };
    // Compte les IPs listees (pas le champ total_banned).
    assert_eq!(s.total_banned_ips(), 3);
}
