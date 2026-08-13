//! Parsers generiques pour les configurations "key separator value par ligne".
//! La logique vit dans le core hexagonal ; ce module ne fait que re-exporter.

pub use platform_core::sentinel::domain::entities::system::config_parsers::{
    lookup_u64, parse_id_u64_lines, parse_pipe_lines, split_csv,
};
