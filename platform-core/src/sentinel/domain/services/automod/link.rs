use regex::Regex;
use std::sync::LazyLock;

/// Regex pour détecter les URLs (http, https, discord invites).
static URL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(https?://\S+|discord\.gg/\S+|discord\.com/invite/\S+)")
        .expect("regex invalide")
});

/// Retourne `true` si le message contient un lien non autorisé.
///
/// - `allow_discord_invites` : si true, les liens discord.gg/* et discord.com/invite/* sont ignorés.
/// - `allowed_domains` : liste de domaines autorisés (ex: ["twitch.tv", "youtube.com"]).
///   Un URL dont l'HÔTE est exactement ce domaine — ou un de ses sous-domaines —
///   n'est pas flagué. La comparaison se fait sur l'hôte (pas un simple
///   `contains`) pour éviter qu'un `evil-twitch.tv.attacker.com` ne passe la
///   whitelist en contenant `twitch.tv`.
pub fn detect(content: &str, allow_discord_invites: bool, allowed_domains: &[String]) -> bool {
    for m in URL_PATTERN.find_iter(content) {
        let url = m.as_str().to_lowercase();

        if allow_discord_invites && is_discord_invite(&url) {
            continue;
        }

        let host = extract_host(&url);
        if allowed_domains.iter().any(|d| host_matches(host, d)) {
            continue;
        }

        return true;
    }
    false
}

fn is_discord_invite(url: &str) -> bool {
    url.contains("discord.gg/") || url.contains("discord.com/invite/")
}

/// Extrait l'hôte d'une URL (sans schéma, userinfo, port ni chemin).
/// L'entrée est supposée déjà en minuscules. Gère aussi les invitations
/// sans schéma type `discord.gg/abc`.
pub(super) fn extract_host(url: &str) -> &str {
    // Retire le schéma (`https://`, `http://`).
    let after_scheme = match url.find("://") {
        Some(i) => &url[i + 3..],
        None => url,
    };
    // Hôte = autorité jusqu'au premier '/', '?' ou '#'.
    let authority = after_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(after_scheme);
    // Retire l'éventuel userinfo (`user:pass@host`).
    let host_port = authority.rsplit('@').next().unwrap_or(authority);
    // Retire l'éventuel port (`host:443`).
    host_port.split(':').next().unwrap_or(host_port)
}

/// `true` si `host` est exactement `domain` ou un de ses sous-domaines
/// (frontière de label), insensible à la casse. `clips.twitch.tv` matche
/// `twitch.tv`, mais `evil-twitch.tv.attacker.com` ne matche pas.
pub(super) fn host_matches(host: &str, domain: &str) -> bool {
    let domain = domain.trim().trim_start_matches('.').to_lowercase();
    if domain.is_empty() {
        return false;
    }
    host == domain || host.ends_with(&format!(".{domain}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── URLs avec protocole ──

    #[test]
    fn https_simple() {
        assert!(detect("https://example.com", false, &[]));
    }
    #[test]
    fn http_simple() {
        assert!(detect("http://example.com", false, &[]));
    }
    #[test]
    fn https_with_path() {
        assert!(detect("https://example.com/page/sub?q=1", false, &[]));
    }
    #[test]
    fn url_in_text() {
        assert!(detect(
            "Va voir https://example.com pour plus d'infos",
            false,
            &[]
        ));
    }
    #[test]
    fn multiple_urls() {
        assert!(detect("https://a.com et https://b.com", false, &[]));
    }

    // ── Discord invites ──

    #[test]
    fn discord_gg_blocked_by_default() {
        assert!(detect("Rejoins discord.gg/abc123", false, &[]));
    }
    #[test]
    fn discord_com_invite_blocked_by_default() {
        assert!(detect("discord.com/invite/test", false, &[]));
    }

    #[test]
    fn discord_gg_allowed_when_configured() {
        assert!(!detect("Rejoins discord.gg/abc123", true, &[]));
    }
    #[test]
    fn discord_com_invite_allowed_when_configured() {
        assert!(!detect("discord.com/invite/test", true, &[]));
    }
    #[test]
    fn discord_invite_in_text_allowed() {
        assert!(!detect("Mon serveur discord.gg/monserv venez", true, &[]));
    }

    // ── Domaines autorisés ──

    #[test]
    fn allowed_domain_not_flagged() {
        let allowed = vec!["twitch.tv".to_string()];
        assert!(!detect("https://twitch.tv/monstream", false, &allowed));
    }
    #[test]
    fn allowed_domain_case_insensitive() {
        let allowed = vec!["youtube.com".to_string()];
        assert!(!detect("https://YOUTUBE.COM/watch?v=abc", false, &allowed));
    }
    #[test]
    fn non_allowed_domain_still_flagged() {
        let allowed = vec!["twitch.tv".to_string()];
        assert!(detect("https://badsite.com/hack", false, &allowed));
    }
    #[test]
    fn multiple_urls_one_allowed_one_not() {
        let allowed = vec!["twitch.tv".to_string()];
        // https://badsite.com n'est pas autorisé → true
        assert!(detect(
            "https://twitch.tv/ok et https://badsite.com/hack",
            false,
            &allowed
        ));
    }
    #[test]
    fn subdomain_of_allowed_still_allowed() {
        // Un vrai sous-domaine du domaine autorisé reste autorisé.
        let allowed = vec!["twitch.tv".to_string()];
        assert!(!detect("https://clips.twitch.tv/abc", false, &allowed));
    }
    #[test]
    fn subdomain_spoof_not_whitelisted() {
        // `evil-twitch.tv.attacker.com` CONTIENT `twitch.tv` mais l'hôte est
        // `...attacker.com` → ne doit PAS être whitelisté.
        let allowed = vec!["twitch.tv".to_string()];
        assert!(detect(
            "https://evil-twitch.tv.attacker.com/login",
            false,
            &allowed
        ));
    }
    #[test]
    fn path_containing_allowed_domain_not_whitelisted() {
        // Le domaine autorisé n'apparaît que dans le chemin → toujours flagué.
        let allowed = vec!["twitch.tv".to_string()];
        assert!(detect("https://attacker.com/twitch.tv/x", false, &allowed));
    }
    #[test]
    fn allowed_domain_with_query_after() {
        // Query-string après l'hôte : l'hôte matche toujours.
        let allowed = vec!["youtube.com".to_string()];
        assert!(!detect(
            "https://youtube.com/watch?v=twitch.tv",
            false,
            &allowed
        ));
    }
    #[test]
    fn allowed_domain_with_port() {
        let allowed = vec!["example.com".to_string()];
        assert!(!detect("https://example.com:8443/x", false, &allowed));
    }
    #[test]
    fn host_matches_helper() {
        assert!(host_matches("twitch.tv", "twitch.tv"));
        assert!(host_matches("clips.twitch.tv", "twitch.tv"));
        assert!(!host_matches("eviltwitch.tv", "twitch.tv"));
        assert!(!host_matches("twitch.tv.attacker.com", "twitch.tv"));
    }

    #[test]
    fn all_urls_allowed() {
        let allowed = vec!["twitch.tv".to_string(), "youtube.com".to_string()];
        assert!(!detect(
            "https://twitch.tv/ok https://youtube.com/watch?v=abc",
            false,
            &allowed
        ));
    }

    // ── Pas de lien ──

    #[test]
    fn no_protocol() {
        assert!(!detect("Mon site est example.com", false, &[]));
    }
    #[test]
    fn clean_text() {
        assert!(!detect("Salut tout le monde", false, &[]));
    }
    #[test]
    fn empty() {
        assert!(!detect("", false, &[]));
    }
    #[test]
    fn email_not_url() {
        assert!(!detect("contact@example.com", false, &[]));
    }
    #[test]
    fn dotted_words() {
        assert!(!detect("e.g. c'est a dire", false, &[]));
    }
    #[test]
    fn ip_without_protocol() {
        assert!(!detect("192.168.1.1", false, &[]));
    }
    #[test]
    fn ip_with_protocol() {
        assert!(detect("http://192.168.1.1/admin", false, &[]));
    }
}
