import { describe, expect, it } from "vitest";
import { effectiveConfigValue, parseBoolConfig } from "./configFlags";

describe("config flags", () => {
  it("considere un module sans valeur enabled explicite comme desactive", () => {
    expect(effectiveConfigValue("enabled", undefined, "true")).toBe("false");
    expect(parseBoolConfig(effectiveConfigValue("enabled", undefined, "true"))).toBe(false);
  });

  it("respecte une valeur enabled explicitement stockee", () => {
    expect(effectiveConfigValue("enabled", "true", "false")).toBe("true");
    expect(effectiveConfigValue("enabled", "false", "true")).toBe("false");
  });

  it("conserve le default du schema pour les reglages secondaires", () => {
    expect(effectiveConfigValue("spam_detection_enabled", undefined, "true")).toBe("true");
  });
});
