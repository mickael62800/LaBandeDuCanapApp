use regex::Regex;
use std::sync::LazyLock;

/// Gravité d'un terme détecté.
///
/// La distinction est linguistique, pas morale : elle sépare ce qui VISE
/// quelqu'un de ce qui ponctue une phrase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gravite {
    /// Juron d'exclamation. « putain c'était bien », « merde j'ai oublié ».
    /// Ne vise personne : le mot exprime la surprise ou l'agacement.
    Juron,
    /// Insulte ciblée ou terme dégradant. « nique ta mère », « connard ».
    Ciblee,
}

/// Jurons d'exclamation.
///
/// Ces mots étaient traités comme des insultes, au même poids. En français ils
/// ponctuent massivement une phrase sans agresser personne : « merde j'ai
/// oublié » se faisait SUPPRIMER. Les séparer leur donne leur propre poids
/// sans désarmer la détection des vraies insultes — ce qu'un simple réglage de
/// poids ne pouvait pas faire, les deux partageant le même flag.
static JURONS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    let raw = [
        // Français
        r"(?i)\b(putain|merde|merdique|bordel|pur[eé]e|punaise|zut|crotte)\b",
        // Anglais
        r"(?i)\b(shit(ty)?|damn|crap)\b",
        r"(?i)\bsh[*]t(ty)?\b",
    ];
    raw.iter()
        .map(|p| Regex::new(p).expect("regex invalide"))
        .collect()
});

/// Insultes ciblées et termes dégradants.
///
/// `con` reste ici : « t'es con » vise quelqu'un. Sa forme exclamative existe
/// (« con de moteur ») mais reste minoritaire.
///
/// `nique` sans complément est ambigu (« ça nique tout ») ; le garder coûte
/// moins qu'une insulte manquée.
static CIBLEES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    let raw = [
        // Français
        r"(?i)\b(con(nard|nasse)?|encul[eé](r)?|fdp|ntm|nique|batard|b[aâ]tard|pd|p[eé]d[eé]|pute|salop(e|ard)?|ta\s*gueule|ferme[\s-]*la|d[eé]gage)\b",
        // Formulations hostiles courantes. Elles restent dans le signal local
        // cible : si l'IA est indisponible, une menace ne doit jamais produire
        // un score nul et passer silencieusement.
        r"(?i)\b(fils?\s+de\s+pute|sucer?\s+de\s+bite|mangeur\s+de\s+bite|je\s+vais\s+t['’]?arracher\s+la\s+t[eê]te|t['’]?es\s+un\s+homme\s+mort)\b",
        // Anglais
        r"(?i)\b(fuck(ing|er|ed)?|bitch|asshole|bastard|dick(head)?|cunt|stfu|idiot|moron|retard(ed)?|dumb(ass)?)\b",
        // Variantes avec astérisque (f*ck, b*tch…)
        r"(?i)\bf[*]ck(ing|er|ed)?\b",
        r"(?i)\bb[*]tch\b",
    ];
    raw.iter()
        .map(|p| Regex::new(p).expect("regex invalide"))
        .collect()
});

/// Normalise le leet speak d'un contenu pour détecter les variantes d'insultes.
///
/// Substitutions appliquées :
/// - `0` → `o`, `1` → `l`, `3` → `e`, `4` → `a`, `5` → `s`, `7` → `t`
/// - `@` → `a`, `$` → `s`
/// - `*` supprimé (pour f*ck, f*ck…)
fn normalize_leet(content: &str) -> String {
    content
        .chars()
        .filter_map(|c| match c {
            '0' => Some('o'),
            '1' | '!' => Some('i'),
            '3' => Some('e'),
            '4' => Some('a'),
            '5' => Some('s'),
            '6' => Some('g'),
            '7' => Some('t'),
            '8' => Some('b'),
            '9' => Some('g'),
            '@' => Some('a'),
            '$' => Some('s'),
            // NB: on NE mappe PAS '(' -> 'c' : « (on se voit ? » deviendrait
            // « con » (faux positif tres frequent en francais). Le gain
            // anti-contournement ne vaut pas ce faux positif.
            '+' => Some('t'),
            '|' => Some('l'),
            '*' | '.' | '_' | '-' => None, // caracteres de separation supprimes
            other => Some(other),
        })
        .collect()
}

/// Un des motifs correspond-il, sur le contenu brut ou sa forme normalisée ?
///
/// La forme normalisée (leet speak) n'est reparcourue que si elle diffère :
/// sinon on paierait deux fois le même balayage de regex sur le chemin chaud.
fn correspond(motifs: &[Regex], content: &str, normalized: &str) -> bool {
    motifs.iter().any(|re| re.is_match(content))
        || (normalized != content && motifs.iter().any(|re| re.is_match(normalized)))
}

/// Gravité du terme le plus grave présent dans le message, s'il y en a un.
///
/// Une insulte ciblée l'emporte sur un juron : « putain t'es con » est une
/// insulte, pas une exclamation.
///
/// Les mots personnalisés de la configuration comptent comme CIBLÉS. Un
/// administrateur qui prend la peine d'ajouter un mot veut le voir sanctionné,
/// pas toléré.
pub fn detect_gravite(content: &str, custom_words: &[String]) -> Option<Gravite> {
    let normalized = normalize_leet(content);

    if correspond(&CIBLEES, content, &normalized) {
        return Some(Gravite::Ciblee);
    }

    if !custom_words.is_empty() {
        let content_lower = content.to_lowercase();
        if custom_words
            .iter()
            .any(|w| content_lower.contains(w.as_str()))
        {
            return Some(Gravite::Ciblee);
        }
    }

    if correspond(&JURONS, content, &normalized) {
        return Some(Gravite::Juron);
    }

    None
}

/// Retourne `true` si le message contient une insulte CIBLÉE.
///
/// Les jurons d'exclamation ne comptent plus ici : ils ont leur propre flag.
pub fn detect(content: &str, custom_words: &[String]) -> bool {
    matches!(detect_gravite(content, custom_words), Some(Gravite::Ciblee))
}

/// Retourne `true` pour un juron d'exclamation SANS insulte ciblée.
pub fn detect_juron(content: &str, custom_words: &[String]) -> bool {
    matches!(detect_gravite(content, custom_words), Some(Gravite::Juron))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Insultes francaises ──

    #[test]
    fn fr_connard() {
        assert!(detect("t'es un connard", &[]));
    }
    #[test]
    fn fr_connasse() {
        assert!(detect("quelle connasse", &[]));
    }
    #[test]
    fn fr_con_seul() {
        assert!(detect("espece de con", &[]));
    }
    #[test]
    fn fr_putain() {
        // Juron desormais, plus une insulte : le terme reste detecte, mais
        // dans la categorie qui ne vise personne.
        assert!(detect_juron("putain de merde", &[]));
    }
    #[test]
    fn fr_merde() {
        assert!(detect_juron("c'est de la merde", &[]));
    }
    #[test]
    fn fr_encule() {
        assert!(detect("va te faire enculé", &[]));
    }
    #[test]
    fn fr_fdp() {
        assert!(detect("fdp va", &[]));
    }
    #[test]
    fn fr_insultes_et_menaces_composees() {
        for message in [
            "fils de pute",
            "mangeur de bite",
            "je vais t'arracher la tete",
            "t'es un homme mort",
        ] {
            assert!(detect(message, &[]), "{message}");
        }
    }
    #[test]
    fn fr_ntm() {
        assert!(detect("ntm grave", &[]));
    }
    #[test]
    fn fr_nique() {
        assert!(detect("je te nique", &[]));
    }
    #[test]
    fn fr_batard() {
        assert!(detect("sale bâtard", &[]));
    }
    #[test]
    fn fr_pd() {
        assert!(detect("espece de pd", &[]));
    }
    #[test]
    fn fr_salope() {
        assert!(detect("quelle salope", &[]));
    }
    #[test]
    fn fr_salopard() {
        assert!(detect("quel salopard", &[]));
    }
    #[test]
    fn fr_bordel() {
        assert!(detect_juron("bordel de merde", &[]));
    }
    #[test]
    fn fr_ta_gueule() {
        assert!(detect("ta gueule", &[]));
    }
    #[test]
    fn fr_ferme_la() {
        assert!(detect("ferme-la", &[]));
    }
    #[test]
    fn fr_degage() {
        assert!(detect("dégage d'ici", &[]));
    }

    // ── Insultes anglaises ──

    #[test]
    fn en_fuck() {
        assert!(detect("fuck you", &[]));
    }
    #[test]
    fn en_fucking() {
        assert!(detect("that's fucking stupid", &[]));
    }
    #[test]
    fn en_shit() {
        assert!(detect_juron("this is shit", &[]));
    }
    #[test]
    fn en_bitch() {
        assert!(detect("you bitch", &[]));
    }
    #[test]
    fn en_asshole() {
        assert!(detect("you're an asshole", &[]));
    }
    #[test]
    fn en_stfu() {
        assert!(detect("stfu noob", &[]));
    }
    #[test]
    fn en_retard() {
        assert!(detect("you retard", &[]));
    }
    #[test]
    fn en_dumbass() {
        assert!(detect("what a dumbass", &[]));
    }
    #[test]
    fn en_cunt() {
        assert!(detect("stupid cunt", &[]));
    }

    // ── Case insensitive ──

    #[test]
    fn case_insensitive_upper() {
        assert!(detect("CONNARD", &[]));
    }
    #[test]
    fn case_insensitive_mixed() {
        assert!(detect("FdP", &[]));
    }
    #[test]
    fn case_insensitive_en() {
        assert!(detect("FUCK OFF", &[]));
    }

    // ── Leet speak ──

    #[test]
    fn leet_connard_0() {
        assert!(detect("c0nnard", &[]));
    }
    #[test]
    fn leet_connard_mixed() {
        assert!(detect("c0nn4rd", &[]));
    }
    #[test]
    fn leet_fuck_star() {
        assert!(detect("f*ck you", &[]));
    }
    #[test]
    fn leet_fuck_star_full() {
        assert!(detect("f*cking idiot", &[]));
    }
    #[test]
    fn leet_shit_dollar() {
        assert!(detect_juron("$hit", &[]));
    }
    #[test]
    fn leet_asshole_at() {
        assert!(detect("@sshole", &[]));
    }
    #[test]
    fn leet_bastard_4() {
        assert!(detect("b4stard", &[]));
    }
    #[test]
    fn leet_merde_3() {
        assert!(detect_juron("m3rde", &[]));
    }
    #[test]
    fn leet_putain_4() {
        assert!(detect_juron("put4in", &[]));
    }
    #[test]
    fn leet_encule_3() {
        assert!(detect("encul3", &[]));
    }

    // ── Mots personnalisés ──

    #[test]
    fn custom_word_detected() {
        assert!(detect("tu es un noob", &["noob".to_string()]));
    }
    #[test]
    fn custom_word_case_insensitive() {
        assert!(detect("NOOB", &["noob".to_string()]));
    }
    #[test]
    fn custom_word_in_sentence() {
        assert!(detect("arrete de troll stp", &["troll".to_string()]));
    }
    #[test]
    fn multiple_custom_words_one_match() {
        let words = vec!["noob".to_string(), "troll".to_string()];
        assert!(detect("t'es un troll", &words));
    }
    #[test]
    fn custom_words_no_match() {
        assert!(!detect("Salut tout le monde", &["noob".to_string()]));
    }
    #[test]
    fn empty_custom_words_no_effect() {
        assert!(!detect("Salut tout le monde", &[]));
    }

    // ── Faux positifs a eviter ──

    #[test]
    fn clean_french() {
        assert!(!detect("Salut tout le monde !", &[]));
    }
    #[test]
    fn clean_english() {
        assert!(!detect("Hello how are you?", &[]));
    }
    #[test]
    fn clean_discussion() {
        assert!(!detect("On se retrouve a 20h pour la game", &[]));
    }
    #[test]
    fn clean_connaitre() {
        assert!(!detect("Je vais te faire connaitre ce jeu", &[]));
    }
    #[test]
    fn clean_discourse() {
        assert!(!detect("C'est un discours interessant", &[]));
    }
    #[test]
    fn clean_context_shift() {
        assert!(!detect("Le concert etait super", &[]));
    }
    #[test]
    fn clean_number() {
        assert!(!detect("1234567890", &[]));
    }
    #[test]
    fn clean_emoji() {
        assert!(!detect("Haha super game", &[]));
    }
    #[test]
    fn clean_empty() {
        assert!(!detect("", &[]));
    }

    // ── normalize_leet unitaire ──

    #[test]
    fn normalize_digits() {
        assert_eq!(normalize_leet("c0nn4rd"), "connard");
    }
    #[test]
    fn normalize_at_dollar() {
        assert_eq!(normalize_leet("@$$hole"), "asshole");
    }
    #[test]
    fn normalize_star_removed() {
        assert_eq!(normalize_leet("f*ck"), "fck");
    }
    #[test]
    fn normalize_exclamation() {
        assert_eq!(normalize_leet("b!tch"), "bitch");
    }
    #[test]
    fn normalize_parenthesis() {
        // '(' n'est plus mappe vers 'c' : « (on se voit ? » ne doit PAS devenir
        // « con » (faux positif frequent). Compromis : on perd « (unt ».
        assert_eq!(normalize_leet("(unt"), "(unt");
        assert_eq!(normalize_leet("(on se voit"), "(on se voit");
    }
    #[test]
    fn normalize_separators_removed() {
        assert_eq!(normalize_leet("c.o.n.n.a.r.d"), "connard");
    }
    #[test]
    fn leet_bitch_excl() {
        assert!(detect("b!tch", &[]));
    }
    #[test]
    fn leet_separated_dots() {
        assert!(detect("c.o.n.n.a.r.d", &[]));
    }
    #[test]
    fn normalize_clean_unchanged() {
        assert_eq!(normalize_leet("hello"), "hello");
    }
}

#[cfg(test)]
mod tests_gravite {
    use super::*;

    fn g(s: &str) -> Option<Gravite> {
        detect_gravite(s, &[])
    }

    // ── Jurons d'exclamation : ne visent personne ──

    #[test]
    fn jurons_francais_sont_classes_juron() {
        for m in [
            "putain c'etait bien hier",
            "merde j'ai oublie",
            "bordel de nom",
            "zut alors",
            "punaise il fait froid",
        ] {
            assert_eq!(g(m), Some(Gravite::Juron), "{m}");
        }
    }

    #[test]
    fn jurons_anglais_sont_classes_juron() {
        assert_eq!(g("shit i forgot"), Some(Gravite::Juron));
        assert_eq!(g("damn that was close"), Some(Gravite::Juron));
    }

    // ── Insultes ciblees : visent quelqu'un ──

    #[test]
    fn insultes_ciblees_restent_ciblees() {
        for m in [
            "nique ta mere",
            "connard",
            "ta gueule",
            "fdp",
            "t'es qu'un batard",
            "espece de salope",
            "degage d'ici",
        ] {
            assert_eq!(g(m), Some(Gravite::Ciblee), "{m}");
        }
    }

    #[test]
    fn con_vise_quelqu_un_donc_ciblee() {
        // « t'es con » qualifie une personne. On assume : le mot sert bien plus
        // souvent a qualifier quelqu'un qu'a ponctuer une phrase.
        assert_eq!(g("t'es con toi"), Some(Gravite::Ciblee));
    }

    // ── Arbitrage entre les deux ──

    #[test]
    fn une_insulte_ciblee_l_emporte_sur_un_juron() {
        // Le cas qui decide de la sanction : le message contient les deux,
        // c'est bien une insulte.
        assert_eq!(g("putain t'es con"), Some(Gravite::Ciblee));
        assert_eq!(g("merde nique ta mere"), Some(Gravite::Ciblee));
    }

    #[test]
    fn message_anodin_ne_leve_rien() {
        for m in [
            "bonjour tout le monde",
            "on se voit demain ?",
            "il est bete ce jeu",
        ] {
            assert_eq!(g(m), None, "{m}");
        }
    }

    // ── Mots personnalises ──

    #[test]
    fn mot_personnalise_compte_comme_ciblee() {
        // Un administrateur qui ajoute un mot veut le voir sanctionne, pas
        // tolere comme un juron.
        let custom = vec!["nabot".to_string()];
        assert_eq!(detect_gravite("sale nabot", &custom), Some(Gravite::Ciblee));
    }

    // ── Contournement ──

    #[test]
    fn leet_speak_reste_detecte_dans_la_bonne_categorie() {
        assert_eq!(g("c0nnard"), Some(Gravite::Ciblee));
        assert_eq!(g("m3rde"), Some(Gravite::Juron));
    }

    // ── Compatibilite des deux fonctions publiques ──

    #[test]
    fn detect_ne_retient_plus_que_les_insultes_ciblees() {
        assert!(detect("connard", &[]));
        assert!(!detect("merde j'ai oublie", &[]));
    }

    #[test]
    fn detect_juron_exclut_les_insultes_ciblees() {
        assert!(detect_juron("merde j'ai oublie", &[]));
        // Deja couvert par `detect` : ne pas lever les deux flags a la fois,
        // sinon le message compterait double dans le score.
        assert!(!detect_juron("putain t'es con", &[]));
    }
}
