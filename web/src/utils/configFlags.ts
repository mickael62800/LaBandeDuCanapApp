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
