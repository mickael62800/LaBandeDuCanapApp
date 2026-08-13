// Rendu d'un sous-ensemble du markdown Discord, pour l'apercu d'embed.
//
// SECURITE : on echappe TOUJOURS le HTML de l'entree d'abord, puis on
// n'introduit que des balises connues. Les liens sont restreints a http(s).
// -> aucune injection possible depuis le contenu saisi par l'utilisateur.

/// Echappe TOUT ce qui a un sens en HTML, guillemets compris.
///
/// Les deux guillemets manquaient. Ils n'ont l'air de rien tant qu'on ne pense
/// qu'au texte — mais la regle des liens reinjecte l'URL capturee dans
/// `href="$2"` : une URL contenant un guillemet double sortait de l'attribut et
/// en ouvrait un autre. `[texte](https://x/"onmouseover="alert(1))` produisait
/// ainsi un gestionnaire d'evenement dans le HTML rendu.
///
/// La CSP de production (`script-src 'self'`, sans `unsafe-inline`) empechait
/// l'execution. Ce n'est qu'une SECONDE barriere : le serveur de developpement
/// ne l'applique pas, et une relaxation future de la CSP reactiverait le
/// vecteur sans que personne ne fasse le lien avec ce fichier.
function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

/// Inverse d'`escapeHtml`, pour rendre a `new URL` la chaine que l'auteur a
/// reellement saisie. `&amp;` est le cas courant : toute URL a parametres en
/// contient apres echappement.
function unescapeHtml(s: string): string {
  return s
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&gt;/g, ">")
    .replace(/&lt;/g, "<")
    .replace(/&amp;/g, "&");
}

/// Valide une URL de lien et renvoie sa forme sure pour un attribut `href`,
/// ou `null` si elle n'est pas exploitable.
///
/// La validation se fait sur l'URL DECODEE, avec `new URL` plutot qu'avec la
/// seule expression reguliere : celle-ci atteste d'un prefixe `http(s)://`,
/// pas d'une URL valide. Le protocole est reverifie apres analyse — une URL
/// peut changer de protocole en etant normalisee.
function hrefSur(urlEchappee: string): string | null {
  try {
    const url = new URL(unescapeHtml(urlEchappee));
    if (url.protocol !== "http:" && url.protocol !== "https:") return null;
    // `href` est la forme normalisee par le navigateur ; on la re-echappe pour
    // l'attribut, ce qui garantit qu'aucun guillemet n'y subsiste.
    return escapeHtml(url.href);
  } catch {
    return null;
  }
}

/// Transforme le markdown Discord en HTML sur (a passer a v-html).
export function renderDiscordMarkdown(input: string): string {
  if (!input) return "";
  let s = escapeHtml(input);

  // Blocs de code ```lang\n...``` (proteges des autres regles : on les traite
  // en premier et leur contenu n'est plus reformate).
  s = s.replace(/```(?:[a-zA-Z0-9+-]*\n)?([\s\S]*?)```/g, (_m, code: string) => {
    return `<pre class="md-pre"><code>${code.replace(/\n$/, "")}</code></pre>`;
  });
  // Code inline `...`
  s = s.replace(/`([^`\n]+?)`/g, '<code class="md-code">$1</code>');

  // Titres (Discord : #, ##, ###) en debut de ligne.
  s = s.replace(/^### (.*)$/gm, '<div class="md-h3">$1</div>');
  s = s.replace(/^## (.*)$/gm, '<div class="md-h2">$1</div>');
  s = s.replace(/^# (.*)$/gm, '<div class="md-h1">$1</div>');

  // Citations « > … » (le > a ete echappe en &gt;).
  s = s.replace(/^&gt; (.*)$/gm, '<div class="md-quote">$1</div>');

  // Listes a puces « - » ou « * ».
  s = s.replace(/^[*-] (.*)$/gm, '<div class="md-li">• $1</div>');

  // Gras, souligne, barre, italique (ordre important).
  s = s.replace(/\*\*([^*]+?)\*\*/g, "<strong>$1</strong>");
  s = s.replace(/__([^_]+?)__/g, "<u>$1</u>");
  s = s.replace(/~~([^~]+?)~~/g, "<del>$1</del>");
  s = s.replace(/\*([^*\n]+?)\*/g, "<em>$1</em>");
  s = s.replace(/_([^_\n]+?)_/g, "<em>$1</em>");

  // Liens [texte](url) — url restreinte a http(s) ET validee par `new URL`.
  //
  // La substitution passe par une fonction et non par `$2` : il faut pouvoir
  // REFUSER une URL. Une capture directe ne laisse pas ce choix — elle
  // reinjecte ce qui a matche, quoi que ce soit.
  s = s.replace(
    /\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)/g,
    (entier: string, libelle: string, url: string) => {
      const href = hrefSur(url);
      // URL invalide : on laisse le markdown BRUT (deja echappe) plutot que de
      // produire un lien mort ou de faire disparaitre le texte de l'auteur.
      if (!href) return entier;
      return `<a href="${href}" target="_blank" rel="noopener noreferrer">${libelle}</a>`;
    },
  );

  // Sauts de ligne -> <br>, en nettoyant ceux colles aux blocs.
  s = s.replace(/\n/g, "<br>");
  s = s.replace(/(<\/(?:div|pre)>)<br>/g, "$1");
  s = s.replace(/<br>(<(?:div|pre)\b)/g, "$1");

  return s;
}
