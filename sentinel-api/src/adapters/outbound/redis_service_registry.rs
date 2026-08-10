//! Adapter Redis du port `ServiceRegistry`. Lit `bots:known` (SET) puis
//! ping `bot:online:<name>` (EXISTS) pour distinguer online/total.

use async_trait::async_trait;
use tracing::warn;

use sentinel_core::domain::entities::system::config_parsers::is_worker_service;
use sentinel_core::ports::outbound::ops::service_registry::{ServiceCounts, ServiceRegistry};

pub struct RedisServiceRegistry {
    client: redis::Client,
}

impl RedisServiceRegistry {
    pub fn new(client: redis::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl ServiceRegistry for RedisServiceRegistry {
    async fn count_services(&self) -> ServiceCounts {
        let mut conn = match self.client.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "Redis indisponible pour count_services");
                return ServiceCounts {
                    bots_online: 0,
                    bots_total: 0,
                    workers_online: 0,
                    workers_total: 0,
                };
            }
        };

        use redis::AsyncCommands;
        let known: Vec<String> = match conn.smembers("bots:known").await {
            Ok(k) => k,
            Err(e) => {
                warn!(error = %e, "Echec Redis SMEMBERS bots:known");
                return ServiceCounts {
                    bots_online: 0,
                    bots_total: 0,
                    workers_online: 0,
                    workers_total: 0,
                };
            }
        };

        let mut counts = ServiceCounts {
            bots_online: 0,
            bots_total: 0,
            workers_online: 0,
            workers_total: 0,
        };

        for name in &known {
            let is_worker = is_worker_service(name);
            let exists: bool = match conn.exists(format!("bot:online:{}", name)).await {
                Ok(v) => v,
                Err(e) => {
                    warn!(error = %e, bot = %name, "Echec Redis EXISTS bot:online");
                    false
                }
            };
            if is_worker {
                counts.workers_total += 1;
                if exists {
                    counts.workers_online += 1;
                }
            } else {
                counts.bots_total += 1;
                if exists {
                    counts.bots_online += 1;
                }
            }
        }
        counts
    }

    async fn ping(&self) -> bool {
        let mut conn = match self.client.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "Redis indisponible pour health check");
                return false;
            }
        };
        use redis::AsyncCommands;
        match conn.get::<_, Option<String>>("ping_test").await {
            Ok(_) => true,
            Err(e) => {
                warn!(error = %e, "Redis ping_test echoue");
                false
            }
        }
    }
}
