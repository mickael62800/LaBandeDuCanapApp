//! Rendu des messages de bienvenue/depart : la logique (placeholders +
//! parsing couleur) vit dans le core hexagonal.

pub use platform_core::sentinel::domain::services::community::welcome_template::{
    parse_color, render,
};
