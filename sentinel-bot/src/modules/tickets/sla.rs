//! SLA tickets : la mesure du delai de premiere reponse staff vit dans le
//! core hexagonal. La decision d'escalade/breach vit dans les workers API.

pub use platform_core::sentinel::domain::services::tickets::sla::{
    format_sla_duration, SlaTracker,
};
