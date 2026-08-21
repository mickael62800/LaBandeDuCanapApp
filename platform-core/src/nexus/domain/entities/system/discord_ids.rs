//! Newtypes pour les IDs Discord (snowflakes).
//!
//! Discord stocke ses IDs comme des `u64` mais les transmet en string sur le
//! wire (REST + gRPC + JS qui ne gere pas u64 precis). On garde la repr en
//! `String` ici, mais on encapsule dans des newtypes pour avoir un typage fort
//!
//! Derives :
//! - `serde(transparent)` : JSON identique a String (pas d'enveloppe).
//! - `sqlx(transparent)` : SQL = VARCHAR direct, queries inchangees.
//! - `From<String>` / `From<&str>` / `AsRef<str>` / `Display` : ergonomie.
//!
//! Migration : non utilise pour l'instant. Les BCs migrent progressivement
//! `String` -> `GuildId`/`UserId`/etc. dans des PRs separees (cf. roadmap
//! PR9+). Tant qu'un field reste en `String`, le code legacy continue de
//! marcher.

use serde::Deserialize;
use serde::Serialize;
use std::fmt;

macro_rules! discord_id {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            #[inline]
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            #[inline]
            pub fn as_str(&self) -> &str {
                &self.0
            }

            #[inline]
            pub fn into_inner(self) -> String {
                self.0
            }
        }

        impl From<String> for $name {
            #[inline]
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl From<&str> for $name {
            #[inline]
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }

        impl From<$name> for String {
            #[inline]
            fn from(id: $name) -> Self {
                id.0
            }
        }

        impl AsRef<str> for $name {
            #[inline]
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl std::ops::Deref for $name {
            type Target = str;
            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
}

discord_id!(
    GuildId,
    "ID d'une guild Discord (snowflake u64 stocke en VARCHAR)."
);
discord_id!(
    UserId,
    "ID d'un user Discord (snowflake u64 stocke en VARCHAR)."
);
discord_id!(
    ChannelId,
    "ID d'un channel Discord (text/voice/category/thread)."
);
discord_id!(
    MessageId,
    "ID d'un message Discord (unique au sein d'un channel)."
);
discord_id!(RoleId, "ID d'un role Discord (unique au sein d'une guild).");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_string() {
        let id = GuildId::new("123456789012345678");
        let s: String = id.clone().into();
        assert_eq!(s, "123456789012345678");
        assert_eq!(GuildId::from(s), id);
    }

    #[test]
    fn from_str_and_display() {
        let id: UserId = "abc".into();
        assert_eq!(format!("{id}"), "abc");
    }

    #[test]
    fn deref_str_methods() {
        let id = ChannelId::new("hello");
        assert_eq!(id.len(), 5);
        assert!(id.starts_with("hel"));
    }

    #[test]
    fn serde_transparent() {
        let id = MessageId::new("xyz");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"xyz\"");
        let back: MessageId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn distinct_types() {
        // Verification : les newtypes sont bien distincts (le compile checkers
        // de Rust empeche d'utiliser un GuildId la ou un UserId est attendu).
        fn _need_guild(_: GuildId) {}
        fn _need_user(_: UserId) {}
        let g = GuildId::new("g1");
        let u = UserId::new("u1");
        _need_guild(g);
        _need_user(u);
        // _need_guild(u);  // <- compile error, ce qui est l'objectif
    }

    #[test]
    fn test_as_str_and_into_inner_and_as_ref() {
        let id = RoleId::new("role1");
        assert_eq!(id.as_str(), "role1");
        assert_eq!(id.as_ref(), "role1");
        assert_eq!(id.into_inner(), "role1");
    }
}
