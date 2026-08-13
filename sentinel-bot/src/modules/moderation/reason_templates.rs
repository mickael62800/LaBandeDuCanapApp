//! Templates de raison de moderation : la logique (parsing "label|raison" +
//! filtre autocomplete) vit dans le core hexagonal.

pub use platform_core::sentinel::domain::services::moderation::reason_templates::{
    filter_templates, parse_templates, serialize_templates, ReasonTemplate,
};
