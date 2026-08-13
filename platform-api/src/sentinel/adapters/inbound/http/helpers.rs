use axum::Json;
use serde::{Deserialize, Serialize};

/// Capture (clone) des champs d'un DTO AVANT de le consommer par `.into()`,
/// pour pouvoir les reutiliser ensuite (typiquement dans un broadcast).
/// Retourne `(command, (champ1, champ2, ...))`.
///
/// Avant :
/// ```ignore
/// let guild_id = dto.guild_id.clone();
/// let user_id = dto.user_id.clone();
/// let command = dto.into();
/// ```
/// Apres :
/// ```ignore
/// let (command, (guild_id, user_id)) = capture_and_into!(dto, guild_id, user_id);
/// ```
#[macro_export]
macro_rules! capture_and_into {
    ($dto:expr, $($field:ident),+ $(,)?) => {{
        let captured = ( $($dto.$field.clone(),)+ );
        let command = $dto.into();
        (command, captured)
    }};
}

/// Convertit un Vec<T> en Json<Vec<D>> via From<T> pour D.
/// Remplace le pattern repete : `items.into_iter().map(Dto::from).collect()`
pub fn map_to_dtos<T, D: From<T>>(items: Vec<T>) -> Json<Vec<D>> {
    Json(items.into_iter().map(D::from).collect())
}

/// Normalise un parametre limit optionnel avec une valeur par defaut et un maximum.
/// Garantit que la valeur est >= 0.
pub fn normalize_limit(limit: Option<i64>, default: i64, max: i64) -> i64 {
    limit.unwrap_or(default).max(0).min(max)
}

/// Normalise un parametre days optionnel (i32). Garantit >= 1.
pub fn normalize_days(days: Option<i32>, default: i32, max: i32) -> i32 {
    days.unwrap_or(default).max(1).min(max)
}

/// Normalise un parametre numerique optionnel dans des bornes explicites
/// `[min, max]`. Variante generique de `normalize_limit`/`normalize_days`
/// pour les endpoints dont la borne basse n'est pas 0/1 implicite — l'objectif
/// est que TOUTES les paginations passent par un helper nomme plutot que des
/// `unwrap_or(N).clamp(a, b)` inline eparpilles.
pub fn normalize_in<T: Ord + Copy>(value: Option<T>, default: T, min: T, max: T) -> T {
    value.unwrap_or(default).clamp(min, max)
}

/// Normalise un parametre offset optionnel. Garantit >= 0.
pub fn normalize_offset(offset: Option<i64>) -> i64 {
    offset.unwrap_or(0).max(0)
}

/// Reponse JSON generique pour les operations reussies.
/// Remplace le pattern repete : `Ok(Json(serde_json::json!({ "ok": true })))`
pub fn ok_response() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ok": true }))
}

/// Reponse JSON pour une entite unique convertie en DTO.
pub fn single_dto<T, D: From<T> + Serialize>(entity: T) -> Json<D> {
    Json(D::from(entity))
}

#[cfg(test)]
#[path = "tests/helpers.rs"]
mod tests;

/// Deserialise un booleen de query string en acceptant les formes usuelles.
///
/// `serde` n'accepte que `true` / `false`. Or une query string n'a pas de
/// types : `?all=1` est la convention la plus repandue, et c'est celle
/// qu'emploie le back-office. Le resultat etait un 400 sur toute la page
/// « vie de la communaute », sans indice sur le champ fautif.
///
/// Accepte : `true`, `false`, `1`, `0`, `yes`, `no`, `on`, `off`, et le champ
/// present mais vide (`?all=`), qui vaut `true` — c'est ainsi qu'un drapeau
/// s'ecrit dans une URL.
///
/// Toute autre valeur reste une erreur : `?all=peut-etre` doit se voir.
pub fn bool_souple<'de, D>(d: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let brut = String::deserialize(d)?;
    match brut.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" | "" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        autre => Err(D::Error::custom(format!(
            "booleen attendu (true/false/1/0), recu « {autre} »"
        ))),
    }
}

#[cfg(test)]
mod tests_bool_souple {
    use super::*;

    fn lire(valeur: &str) -> Result<bool, String> {
        let json = serde_json::Value::String(valeur.to_string());
        bool_souple(json).map_err(|e: serde_json::Error| e.to_string())
    }

    #[test]
    fn accepte_les_formes_vraies() {
        for v in ["true", "1", "yes", "on", "TRUE", ""] {
            assert_eq!(lire(v), Ok(true), "{v}");
        }
    }

    #[test]
    fn accepte_les_formes_fausses() {
        for v in ["false", "0", "no", "off", "OFF"] {
            assert_eq!(lire(v), Ok(false), "{v}");
        }
    }

    #[test]
    fn valeur_incoherente_reste_une_erreur() {
        // Un drapeau mal ecrit doit se voir, pas etre devine.
        assert!(lire("peut-etre").is_err());
    }
}
