/**
 * Lecture des flags booléens de `bot_guild_config`.
 *
 * Miroir EXACT de `parse_bool_str` côté Rust
 * (platform-core/src/sentinel/domain/entities/system/config_parsers.rs) : insensible à
 * la casse, et `yes` compte comme vrai. Sans ça, une valeur `"True"` ou
 * `"yes"` faisait afficher un module « OFF » dans le dashboard alors que le
 * bot, lui, le considérait actif et continuait de tourner.
 *
 * Toute lecture d'un flag de config doit passer par ici : c'est le seul
 * endroit où cette sémantique est définie côté web.
 */
export function parseBoolConfig(value: string | null | undefined): boolean {
  if (value === null || value === undefined) return false;
  const v = value.trim().toLowerCase();
  return v === "true" || v === "1" || v === "yes";
}

/**
 * Calcule la valeur effective affichee par le formulaire de configuration.
 *
 * `enabled` est le coupe-circuit global des modules : son absence signifie
 * toujours OFF, meme si un ancien schema de definition annonce encore un
 * default a true. Cette regle doit rester identique au store et au backend.
 */
export function effectiveConfigValue(
  key: string,
  storedValue: string | undefined,
  schemaDefault: string | undefined,
): string | undefined {
  if (storedValue !== undefined) return storedValue;
  if (key === "enabled") return "false";
  return schemaDefault !== undefined && schemaDefault !== "" ? schemaDefault : undefined;
}
