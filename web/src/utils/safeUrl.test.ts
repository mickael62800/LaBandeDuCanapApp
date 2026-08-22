import { describe, expect, it } from "vitest";
import { safeHttpsImageUrl, safeImageUrl, safeLinkUrl } from "./safeUrl";

describe("safeLinkUrl", () => {
  it("accepte http et https", () => {
    expect(safeLinkUrl("https://exemple.fr/a")).toBe("https://exemple.fr/a");
    expect(safeLinkUrl("http://exemple.fr/a")).toBe("http://exemple.fr/a");
  });

  it("bloque javascript:, data: et vbscript:", () => {
    expect(safeLinkUrl("javascript:alert(1)")).toBeNull();
    expect(safeLinkUrl("data:text/html,<script>1</script>")).toBeNull();
    expect(safeLinkUrl("vbscript:msgbox(1)")).toBeNull();
  });

  it("bloque les valeurs vides et malformees", () => {
    expect(safeLinkUrl(null)).toBeNull();
    expect(safeLinkUrl(undefined)).toBeNull();
    expect(safeLinkUrl("")).toBeNull();
    expect(safeLinkUrl("pas une url")).toBeNull();
  });
});

describe("safeHttpsImageUrl", () => {
  it("accepte uniquement https", () => {
    expect(safeHttpsImageUrl("https://cdn.exemple.fr/img.png")).toBe("https://cdn.exemple.fr/img.png");
    expect(safeHttpsImageUrl("http://cdn.exemple.fr/img.png")).toBeNull();
  });

  it("bloque javascript: et les valeurs vides", () => {
    expect(safeHttpsImageUrl("javascript:alert(1)")).toBeNull();
    expect(safeHttpsImageUrl(null)).toBeNull();
    expect(safeHttpsImageUrl("")).toBeNull();
    expect(safeHttpsImageUrl("malformee")).toBeNull();
  });
});

describe("safeImageUrl", () => {
  it("accepte les hostes whitelistes en https", () => {
    expect(safeImageUrl("https://cdn.discordapp.com/emoji/1.png")).toBe("https://cdn.discordapp.com/emoji/1.png");
    expect(safeImageUrl("https://media.discordapp.net/images/1.png")).toBe("https://media.discordapp.net/images/1.png");
    expect(safeImageUrl("https://avatars.githubusercontent.com/u/1")).toBe("https://avatars.githubusercontent.com/u/1");
  });

  it("rejette un hoste hors whitelist", () => {
    expect(safeImageUrl("https://autre.exemple.fr/img.png")).toBeNull();
  });

  it("rejette http, javascript: et les valeurs vides", () => {
    expect(safeImageUrl("http://cdn.discordapp.com/emoji/1.png")).toBeNull();
    expect(safeImageUrl("javascript:alert(1)")).toBeNull();
    expect(safeImageUrl(null)).toBeNull();
    expect(safeImageUrl("malformee")).toBeNull();
  });
});
