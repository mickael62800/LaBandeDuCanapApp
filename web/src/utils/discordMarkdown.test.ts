import { describe, expect, it } from "vitest";

import { renderDiscordMarkdown } from "./discordMarkdown";

describe("renderDiscordMarkdown", () => {
  it("rend le markdown Discord courant", () => {
    expect(renderDiscordMarkdown("**gras**")).toBe("<strong>gras</strong>");
    expect(renderDiscordMarkdown("~~barre~~")).toBe("<del>barre</del>");
    expect(renderDiscordMarkdown("# Titre")).toBe('<div class="md-h1">Titre</div>');
  });

  it("produit un lien pour une URL valide", () => {
    const html = renderDiscordMarkdown("[doc](https://example.com/a)");
    expect(html).toContain('href="https://example.com/a"');
    expect(html).toContain('rel="noopener noreferrer"');
    expect(html).toContain(">doc</a>");
  });

  // Le coeur de W2. Sans echappement des guillemets, l'URL sortait de
  // l'attribut `href` et introduisait un gestionnaire d'evenement.
  it("ne laisse pas une URL sortir de l'attribut href", () => {
    const html = renderDiscordMarkdown('[clic](https://x.test/"onmouseover="alert(1))');

    // Deux barrieres se cumulent, et la seconde suffit ici : `new URL`
    // POURCENT-ENCODE le guillemet (%22), qui ne peut donc plus fermer
    // l'attribut. L'echappement `&quot;` reste la barriere pour tout le reste
    // du texte (cf. le test suivant).
    expect(html).toContain("%22");
    expect(html).not.toContain('onmouseover="');
    // Aucun attribut ne doit exister apres la fermeture de `href`, hormis ceux
    // que ce module pose lui-meme.
    expect(html).toMatch(
      /^<a href="[^"]*" target="_blank" rel="noopener noreferrer">clic<\/a>\)$/,
    );
  });

  it("echappe les guillemets simples et doubles du texte", () => {
    const html = renderDiscordMarkdown(`il a dit "bonjour" et l'a fait`);
    expect(html).toContain("&quot;bonjour&quot;");
    expect(html).toContain("l&#39;a fait");
  });

  it("laisse le markdown brut quand l'URL est invalide", () => {
    // `https://` seul passe la regex (`[^\s)]+` accepte n'importe quoi) mais
    // n'est pas une URL analysable.
    const html = renderDiscordMarkdown("[clic](https://)");
    expect(html).not.toContain("<a ");
    expect(html).toContain("[clic](https://)");
  });

  it("conserve les paramètres d'une URL", () => {
    const html = renderDiscordMarkdown("[recherche](https://example.com/?a=1&b=2)");
    // `&` echappe en `&amp;` DANS l'attribut : c'est la forme HTML correcte,
    // le navigateur la decode en `&`.
    expect(html).toContain("href=\"https://example.com/?a=1&amp;b=2\"");
  });

  it("n'introduit aucune balise depuis le contenu", () => {
    const html = renderDiscordMarkdown('<img src=x onerror="alert(1)">');
    expect(html).not.toContain("<img");
    expect(html).toContain("&lt;img");
  });

  it("protège le contenu des blocs de code", () => {
    const html = renderDiscordMarkdown('```\n<script>alert("x")</script>\n```');
    expect(html).not.toContain("<script>");
    expect(html).toContain("&lt;script&gt;");
  });
});
