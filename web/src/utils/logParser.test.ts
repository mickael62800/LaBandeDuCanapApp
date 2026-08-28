import { describe, expect, it } from "vitest";
import {
  ansiToHtml,
  detectLogCategory,
  detectLogLevel,
  escapeHtml,
  extractTimestamp,
  highlightSearchInHtml,
  parseLogLines,
  stripAnsi,
} from "./logParser";

describe("logParser", () => {
  describe("escapeHtml", () => {
    it("échappe les balises et caractères spéciaux", () => {
      expect(escapeHtml("<script>alert('XSS')</script>")).toBe(
        "&lt;script&gt;alert(&#039;XSS&#039;)&lt;/script&gt;",
      );
      expect(escapeHtml('foo & "bar"')).toBe("foo &amp; &quot;bar&quot;");
    });
  });

  describe("stripAnsi", () => {
    it("supprime les codes ANSI SGR", () => {
      const input = "\u001b[31mError:\u001b[0m Failed to connect";
      expect(stripAnsi(input)).toBe("Error: Failed to connect");
    });
  });

  describe("ansiToHtml", () => {
    it("convertit les couleurs ANSI en spans sécurisés", () => {
      const input = "\u001b[31mErreur fatale\u001b[0m: impossible de bind";
      const html = ansiToHtml(input);
      expect(html).toContain('<span class="ansi-red">Erreur fatale</span>');
      expect(html).toContain(": impossible de bind");
      expect(html).not.toContain("\u001b");
    });

    it("gère les styles multiples (bold + color)", () => {
      const input = "\u001b[1;32mServeur prêt\u001b[0m";
      const html = ansiToHtml(input);
      expect(html).toContain("ansi-bold");
      expect(html).toContain("ansi-green");
      expect(html).toContain("Serveur prêt");
    });
  });

  describe("detectLogLevel", () => {
    it("détecte les erreurs et exceptions", () => {
      expect(detectLogLevel("Exception in thread main java.lang.NullPointerException")).toBe("error");
      expect(detectLogLevel("[ERROR] Failed to load world")).toBe("error");
      expect(detectLogLevel("fatal: database connection failed")).toBe("error");
      expect(detectLogLevel("Server crashed with signal 11")).toBe("error");
    });

    it("détecte les avertissements", () => {
      expect(detectLogLevel("[WARN] Memory usage above 85%")).toBe("warn");
      expect(detectLogLevel("Warning: deprecated mod loaded")).toBe("warn");
    });

    it("détecte les succès et démarrages", () => {
      expect(detectLogLevel("Server started successfully on port 25565")).toBe("success");
      expect(detectLogLevel("[SUCCESS] World loaded")).toBe("success");
      expect(detectLogLevel("Done (2.54s)! For help, type help")).toBe("success");
      expect(detectLogLevel("Listening on 0.0.0.0:16261")).toBe("success");
    });

    it("détecte le debug", () => {
      expect(detectLogLevel("[DEBUG] Packet received from 127.0.0.1")).toBe("debug");
      expect(detectLogLevel("Trace: parsing config file line 42")).toBe("debug");
    });

    it("retourne info par défaut", () => {
      expect(detectLogLevel("Loading configuration...")).toBe("info");
    });
  });

  describe("detectLogCategory", () => {
    it("détecte les événements joueurs", () => {
      expect(detectLogCategory("Player Steve joined the game")).toBe("player");
      expect(detectLogCategory("User Alice left the server")).toBe("player");
      expect(detectLogCategory("<Bob> Hello everyone!")).toBe("player");
      expect(detectLogCategory("Player John was kicked")).toBe("player");
    });

    it("détecte les événements de sauvegarde", () => {
      expect(detectLogCategory("Autosave completed in 45ms")).toBe("save");
      expect(detectLogCategory("Saving world chunks...")).toBe("save");
      expect(detectLogCategory("Backup created: world_2026-08-28.tar.gz")).toBe("save");
    });

    it("détecte les événements réseau", () => {
      expect(detectLogCategory("Binding UDP port 16261")).toBe("network");
      expect(detectLogCategory("Steam dedicated server registered")).toBe("network");
      expect(detectLogCategory("TCP socket accepted connection")).toBe("network");
    });

    it("retourne general pour les autres", () => {
      expect(detectLogCategory("Starting game server v1.4.2")).toBe("general");
    });
  });

  describe("extractTimestamp", () => {
    it("extrait l'horodatage ISO", () => {
      const res = extractTimestamp("2026-08-28T21:15:30.123Z [INFO] Server started");
      expect(res.timestamp).toBe("2026-08-28T21:15:30.123Z");
      expect(res.message).toBe("[INFO] Server started");
    });

    it("extrait l'horodatage entre crochets", () => {
      const res = extractTimestamp("[21:15:30] [Server thread/INFO]: Ready!");
      expect(res.timestamp).toBe("21:15:30");
      expect(res.message).toBe("[Server thread/INFO]: Ready!");
    });

    it("retourne null si aucun timestamp n'est détecté", () => {
      const res = extractTimestamp("Plain text log line without time");
      expect(res.timestamp).toBeNull();
      expect(res.message).toBe("Plain text log line without time");
    });
  });

  describe("highlightSearchInHtml", () => {
    it("surligne les occurrences du mot recherché", () => {
      const html = "<span>Error connecting to Steam</span>";
      const result = highlightSearchInHtml(html, "steam");
      expect(result).toBe('<span>Error connecting to <mark class="log-match">Steam</mark></span>');
    });

    it("retourne la chaîne intacte si la recherche est vide", () => {
      expect(highlightSearchInHtml("test", "   ")).toBe("test");
    });
  });

  describe("parseLogLines", () => {
    it("parse et enrichit une liste de lignes", () => {
      const lines = [
        "2026-08-28T21:00:00Z [INFO] Server starting...",
        "[ERROR] Could not load mod 'broken_mod'",
        "Player 'Max' joined the game",
      ];
      const parsed = parseLogLines(lines, "broken");

      expect(parsed).toHaveLength(3);
      expect(parsed[0]?.level).toBe("info");
      expect(parsed[0]?.timestamp).toBe("2026-08-28T21:00:00Z");

      expect(parsed[1]?.level).toBe("error");
      expect(parsed[1]?.html).toContain('<mark class="log-match">broken</mark>');

      expect(parsed[2]?.category).toBe("player");
    });
  });
});
