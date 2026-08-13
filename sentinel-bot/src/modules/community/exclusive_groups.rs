//! Groupes de roles mutuellement exclusifs : la logique (parsing config +
//! resolution des conflits) vit dans le core hexagonal.

pub use platform_core::sentinel::domain::services::community::exclusive_groups::{
    get_conflicting_roles, parse_groups,
};
