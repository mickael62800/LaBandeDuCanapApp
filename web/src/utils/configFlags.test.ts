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

  it("traite les valeurs nulles ou vides de parseBoolConfig", () => {
    expect(parseBoolConfig(null)).toBe(false);
    expect(parseBoolConfig(undefined)).toBe(false);
    expect(parseBoolConfig("")).toBe(false);
    expect(parseBoolConfig("false")).toBe(false);
    expect(parseBoolConfig("0")).toBe(false);
    expect(parseBoolConfig("no")).toBe(false);
    expect(parseBoolConfig("TRUE")).toBe(true);
    expect(parseBoolConfig(" 1 ")).toBe(true);
    expect(parseBoolConfig("Yes")).toBe(true);
  });

  it("ne conserve pas un default de schema vide", () => {
    expect(effectiveConfigValue("spam_detection_enabled", undefined, "")).toBeUndefined();
    expect(effectiveConfigValue("spam_detection_enabled", undefined, undefined)).toBeUndefined();
  });
  it("conserve le default du schema pour les reglages secondaires", () => {
    expect(effectiveConfigValue("spam_detection_enabled", undefined, "true")).toBe("true");
  });
});
