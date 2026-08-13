use regex::Regex;
use std::sync::LazyLock;

/// Domaines de phishing connus et patterns de scam Discord.
static PHISHING_DOMAINS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?i)https?://(",
        // Typosquatting Discord
        r"d[il1]sc[o0]rd[\w-]*\.(gift|com|gg|app|click|ru|xyz|top|info|net|org|co)",
        r"|disc[o0]rd[\w-]*app\.\w+",
        r"|discordnitro[\w-]*\.\w+",
        r"|discord-[\w-]*\.(com|gift|click|ru|xyz|top)",
        // Typosquatting Steam
        r"|st[e3][a@]m[\w-]*community[\w-]*\.\w+",
        r"|steam[\w-]*pow[e3]r[\w-]*\.\w+",
        r"|steamcommunlty\.\w+",
        r"|steampowored\.\w+",
        // Faux cadeaux / airdrops
        r"|[\w-]*free[\w-]*nitro[\w-]*\.\w+",
        r"|[\w-]*nitro[\w-]*gift[\w-]*\.\w+",
        r"|[\w-]*crypto[\w-]*airdrop[\w-]*\.\w+",
        // IP grabbers connus
        r"|grabify\.link",
        r"|iplogger\.\w+",
        r"|blasze\.tk",
        r"|2no\.co",
        // Raccourcisseurs suspects
        r"|bit\.do",
        r"|cutt\.ly[\w/]*",
        // Phishing generique
        r"|[\w-]*login[\w-]*verify[\w-]*\.\w+",
        r"|[\w-]*verify[\w-]*account[\w-]*\.\w+",
        ")/\\S*",
    ))
    .expect("regex phishing invalide")
});

/// Patterns de messages scam classiques.
static SCAM_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    let raw = [
        // Faux cadeaux Nitro
        r"(?i)(free|gratuit)\s+(discord\s+)?nitro",
        r"(?i)discord\s*nitro\s*(for\s+)?free",
        r"(?i)(recois|claim|reclame)\s+(ton|your)\s+(cadeau|gift|nitro)",
        // Faux cadeaux Steam
        r"(?i)(free|gratuit)\s+steam\s+(gift|game|wallet|card)",
        r"(?i)steam\s+(gift|game|wallet)\s*(for\s+)?free",
        // Crypto scam
        r"(?i)(earn|gagne[rz]?)\s+\$?\d+[\w\s]*crypto",
        r"(?i)(bitcoin|ethereum|crypto)\s+(giveaway|airdrop|doubl)",
        r"(?i)send\s+\d[\d.]*\s*(btc|eth)\s*(and|et)\s*(get|receive|recois)",
        // DM scam classiques
        r"(?i)(your|ton|votre)\s+account\s+(has\s+been|will\s+be)\s+(disabled|suspended|banned|terminated)",
        r"(?i)(verify|confirm)\s+(your|ton)\s+(account|identity)\s+(before|within)\s+\d+\s*(hours?|heures?)",
        // QR code scam
        r"(?i)scan\s+(this|ce)\s+qr\s*code",
    ];
    raw.iter()
        .map(|p| Regex::new(p).expect("regex scam invalide"))
        .collect()
});

/// Domaines legitimes a ne pas flaguer.
static LEGITIMATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)https?://(www\.)?(discord\.(com|gg|new)|store\.steampowered\.com|steamcommunity\.com|cdn\.discordapp\.com)/").expect("regex legitime invalide")
});

/// Retourne `true` si le message contient un lien de phishing ou un pattern scam.
///
/// - `extra_whitelist` : domaines supplémentaires de confiance (ex: ["mycompany.com"]).
///   Un URL contenant l'un de ces domaines n'est pas flagué comme phishing.
pub fn detect(content: &str, extra_whitelist: &[String]) -> bool {
    if SCAM_PATTERNS.iter().any(|re| re.is_match(content)) {
        return true;
    }

    for m in PHISHING_DOMAINS.find_iter(content) {
        let url = m.as_str();

        if LEGITIMATE.is_match(url) {
            continue;
        }

        if !extra_whitelist.is_empty() {
            let url_lower = url.to_lowercase();
            // Match sur l'HÔTE (exact ou sous-domaine), pas un `contains` brut :
            // sinon `evil-mycompany.com.attacker.net` passerait la whitelist.
            let host = super::link::extract_host(&url_lower);
            if extra_whitelist
                .iter()
                .any(|d| super::link::host_matches(host, d))
            {
                continue;
            }
        }

        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Typosquatting Discord ──

    #[test]
    fn discord_typo_dlscord() {
        assert!(detect("https://dlscord.gift/nitro", &[]));
    }
    #[test]
    fn discord_typo_disc0rd() {
        assert!(detect("https://disc0rd-app.com/verify", &[]));
    }
    #[test]
    fn discord_typo_d1scord() {
        assert!(detect("https://d1scord.com/free", &[]));
    }
    #[test]
    fn discord_nitro_fake() {
        assert!(detect("https://discordnitro-gift.xyz/claim", &[]));
    }
    #[test]
    fn discord_dash_variant() {
        assert!(detect("https://discord-gift.ru/free", &[]));
    }
    #[test]
    fn discord_fake_app() {
        assert!(detect("https://disc0rdapp.com/login", &[]));
    }

    // ── Typosquatting Steam ──

    #[test]
    fn steam_typo_communlty() {
        assert!(detect("https://steamcommunlty.com/trade", &[]));
    }
    #[test]
    fn steam_typo_powored() {
        assert!(detect("https://steampowored.com/login", &[]));
    }
    #[test]
    fn steam_typo_st3am() {
        assert!(detect("https://st3amcommunity.com/profile", &[]));
    }

    // ── IP grabbers ──

    #[test]
    fn grabify() {
        assert!(detect("https://grabify.link/abc123", &[]));
    }
    #[test]
    fn iplogger() {
        assert!(detect("https://iplogger.org/test", &[]));
    }
    #[test]
    fn blasze() {
        assert!(detect("https://blasze.tk/track", &[]));
    }

    // ── Scam messages texte (sans lien) ──

    #[test]
    fn scam_free_nitro_en() {
        assert!(detect("Free Discord Nitro! Claim now", &[]));
    }
    #[test]
    fn scam_free_nitro_fr() {
        assert!(detect("Recois ton cadeau nitro gratuit", &[]));
    }
    #[test]
    fn scam_nitro_for_free() {
        assert!(detect("Discord Nitro for free here", &[]));
    }
    #[test]
    fn scam_free_steam() {
        assert!(detect("Free steam gift card for everyone", &[]));
    }
    #[test]
    fn scam_steam_wallet_free() {
        assert!(detect("Steam wallet for free click here", &[]));
    }
    #[test]
    fn scam_crypto_earn() {
        assert!(detect("Earn $500 in crypto today", &[]));
    }
    #[test]
    fn scam_bitcoin_giveaway() {
        assert!(detect("Bitcoin giveaway live now", &[]));
    }
    #[test]
    fn scam_send_btc() {
        assert!(detect("Send 0.1 BTC and get 1 BTC back", &[]));
    }
    #[test]
    fn scam_account_disabled() {
        assert!(detect("Your account has been disabled, verify now", &[]));
    }
    #[test]
    fn scam_verify_24h() {
        assert!(detect("Verify your account within 24 hours", &[]));
    }
    #[test]
    fn scam_qr_code() {
        assert!(detect("Scan this QR code to get Nitro", &[]));
    }
    #[test]
    fn scam_fr_gagner_crypto() {
        assert!(detect("Gagnez 1000 en crypto facilement", &[]));
    }
    #[test]
    fn scam_eth_airdrop() {
        assert!(detect("Ethereum airdrop happening now", &[]));
    }

    // ── Domaines de phishing generiques ──

    #[test]
    fn generic_login_verify() {
        assert!(detect("https://discord-login-verify.com/auth", &[]));
    }
    #[test]
    fn generic_verify_account() {
        assert!(detect("https://verify-account-now.xyz/step1", &[]));
    }
    #[test]
    fn free_nitro_domain() {
        assert!(detect("https://free-nitro-generator.com/claim", &[]));
    }

    // ── Extra whitelist ──

    #[test]
    fn extra_whitelist_allows_domain() {
        // Un domaine qui ressemble à un pattern phishing mais est dans la whitelist
        let wl = vec!["mycompany-login-verify.com".to_string()];
        assert!(!detect("https://mycompany-login-verify.com/auth", &wl));
    }

    #[test]
    fn extra_whitelist_substring_in_query_not_allowed() {
        // URL de phishing (`discord-gift.ru`) dont la query CONTIENT le domaine
        // whitelisté `discord.com`. L'ancien `contains` la blanchissait à tort ;
        // le match sur l'hôte la maintient flaguée.
        let wl = vec!["discord.com".to_string()];
        assert!(detect("https://discord-gift.ru/free?ref=discord.com", &wl));
    }

    #[test]
    fn extra_whitelist_exact_host_still_allowed() {
        // L'hôte exact whitelisté reste autorisé (non-régression).
        let wl = vec!["free-nitro-generator.com".to_string()];
        assert!(!detect("https://free-nitro-generator.com/claim", &wl));
    }

    #[test]
    fn extra_whitelist_does_not_affect_scam_text() {
        // La whitelist ne couvre que les domaines, pas les patterns texte scam
        let wl = vec!["discord.com".to_string()];
        assert!(detect("Free Discord Nitro! Claim now", &wl));
    }

    // ── Whitelist — vrais domaines NE DOIVENT PAS trigger ──

    #[test]
    fn legit_discord_com() {
        assert!(!detect("https://discord.com/channels/123/456", &[]));
    }
    #[test]
    fn legit_discord_invite() {
        assert!(!detect("https://discord.com/invite/abc123", &[]));
    }
    #[test]
    fn legit_discord_gg() {
        assert!(!detect("https://discord.gg/serveur", &[]));
    }
    #[test]
    fn legit_steam_store() {
        assert!(!detect("https://store.steampowered.com/app/730", &[]));
    }
    #[test]
    fn legit_steamcommunity() {
        assert!(!detect("https://steamcommunity.com/id/user", &[]));
    }
    #[test]
    fn legit_cdn_discord() {
        assert!(!detect(
            "https://cdn.discordapp.com/attachments/123/456/image.png",
            &[]
        ));
    }

    // ── Messages normaux — NE DOIVENT PAS trigger ──

    #[test]
    fn normal_gaming() {
        assert!(!detect("On joue a quoi ce soir ?", &[]));
    }
    #[test]
    fn normal_nitro_mention() {
        assert!(!detect("J'ai achete Nitro hier c'est cool", &[]));
    }
    #[test]
    fn normal_steam_mention() {
        assert!(!detect("Mon compte Steam est ancien", &[]));
    }
    #[test]
    fn normal_crypto_mention() {
        assert!(!detect("J'ai de la crypto sur Binance", &[]));
    }
    #[test]
    fn normal_account_mention() {
        assert!(!detect("Mon account Discord date de 2020", &[]));
    }
    #[test]
    fn normal_empty() {
        assert!(!detect("", &[]));
    }
    #[test]
    fn normal_french_chat() {
        assert!(!detect("Salut les gars, quelqu'un pour ranked ?", &[]));
    }
}
