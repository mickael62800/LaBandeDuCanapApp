mod atrium;
mod nexus;
mod ops;
mod sentinel;

pub fn start(config: &crate::config::Config) -> usize {
    let mut started = 0;
    if let Some(config) = config.atrium.clone() {
        atrium::start(config);
        started += 1;
    }
    if let Some(config) = config.nexus.clone() {
        nexus::start(config);
        started += 1;
    }
    if let Some(config) = config.sentinel.clone() {
        sentinel::start(config);
        started += 1;
    }
    if let Some(config) = config.ops.clone() {
        ops::start(config);
        started += 1;
    }
    started
}
