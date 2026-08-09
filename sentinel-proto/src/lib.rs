//! Definitions protobuf compilees pour la communication gRPC interne
//! entre l'API Sentinel et les bots Discord.
//!
//! Les modules sont generes a la compilation par `tonic-build` (cf. `build.rs`).
//! Chaque package proto devient un sous-module Rust.

pub mod common {
    pub mod v1 {
        tonic::include_proto!("sentinel.common.v1");
    }
}

pub mod ai_dataset {
    pub mod v1 {
        tonic::include_proto!("sentinel.ai_dataset.v1");
    }
}

pub mod progression {
    pub mod v1 {
        tonic::include_proto!("sentinel.progression.v1");
    }
}

pub mod audit {
    pub mod v1 {
        tonic::include_proto!("sentinel.audit.v1");
    }
}

pub mod guild_backup {
    pub mod v1 {
        tonic::include_proto!("sentinel.guild_backup.v1");
    }
}

pub mod ideas {
    pub mod v1 {
        tonic::include_proto!("sentinel.ideas.v1");
    }
}

pub mod purge {
    pub mod v1 {
        tonic::include_proto!("sentinel.purge.v1");
    }
}

pub mod stats {
    pub mod v1 {
        tonic::include_proto!("sentinel.stats.v1");
    }
}

pub mod tickets {
    pub mod v1 {
        tonic::include_proto!("sentinel.tickets.v1");
    }
}

pub mod moderation {
    pub mod v1 {
        tonic::include_proto!("sentinel.moderation.v1");
    }
}

pub mod roles {
    pub mod v1 {
        tonic::include_proto!("sentinel.roles.v1");
    }
}

pub mod members {
    pub mod v1 {
        tonic::include_proto!("sentinel.members.v1");
    }
}

pub mod security {
    pub mod v1 {
        tonic::include_proto!("sentinel.security.v1");
    }
}

pub mod automod {
    pub mod v1 {
        tonic::include_proto!("sentinel.automod.v1");
    }
}

pub mod security_state {
    pub mod v1 {
        tonic::include_proto!("sentinel.security_state.v1");
    }
}

pub mod automod_review {
    pub mod v1 {
        tonic::include_proto!("sentinel.automod_review.v1");
    }
}

pub mod discord_messages {
    pub mod v1 {
        tonic::include_proto!("sentinel.discord_messages.v1");
    }
}

pub mod sursis {
    pub mod v1 {
        tonic::include_proto!("sentinel.sursis.v1");
    }
}

pub mod confessions {
    pub mod v1 {
        tonic::include_proto!("sentinel.confessions.v1");
    }
}

pub mod announcements {
    pub mod v1 {
        tonic::include_proto!("sentinel.announcements.v1");
    }
}

pub mod age_gate {
    pub mod v1 {
        tonic::include_proto!("sentinel.age_gate.v1");
    }
}

pub mod embeds {
    pub mod v1 {
        tonic::include_proto!("sentinel.embeds.v1");
    }
}

pub mod voice {
    pub mod v1 {
        tonic::include_proto!("sentinel.voice.v1");
    }
}

pub mod images {
    pub mod v1 {
        tonic::include_proto!("sentinel.images.v1");
    }
}

pub mod welcome {
    pub mod v1 {
        tonic::include_proto!("sentinel.welcome.v1");
    }
}

pub mod community {
    pub mod v1 {
        tonic::include_proto!("sentinel.community.v1");
    }
}

pub mod export {
    pub mod v1 {
        tonic::include_proto!("sentinel.export.v1");
    }
}

pub mod tls;

