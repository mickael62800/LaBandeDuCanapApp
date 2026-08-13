mod atrium;
mod nexus;
mod ops;
mod sentinel;

pub fn start(config: &crate::config::Config) -> usize {
    atrium::start(config.atrium.clone());
    nexus::start(config.nexus.clone());
    sentinel::start(config.sentinel.clone());
    ops::start(config.ops.clone());
    4
}
