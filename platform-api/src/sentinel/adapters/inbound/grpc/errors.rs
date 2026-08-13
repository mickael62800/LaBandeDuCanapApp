//! Mapping `DomainError` -> `tonic::Status`. Aligne sur les codes HTTP
//! deja utilises cote Axum (cf. `adapters/inbound/http/errors.rs`).

use platform_core::sentinel::domain::errors::DomainError;
use tonic::Code;
use tonic::Status;

pub fn domain_to_status(err: DomainError) -> Status {
    let (code, msg) = match &err {
        DomainError::NotFound(_) => (Code::NotFound, err.to_string()),
        DomainError::ValidationError(_) | DomainError::Validation(_) => {
            (Code::InvalidArgument, err.to_string())
        }
        DomainError::Conflict(_) => (Code::AlreadyExists, err.to_string()),
        DomainError::Forbidden(_) => (Code::PermissionDenied, err.to_string()),
        DomainError::RateLimited(_) => (Code::ResourceExhausted, err.to_string()),
        DomainError::Timeout(_) => (Code::DeadlineExceeded, err.to_string()),
        DomainError::Internal(_) | DomainError::Infrastructure(_) => {
            (Code::Internal, err.to_string())
        }
        DomainError::NotImplemented(_) => (Code::Unimplemented, err.to_string()),
    };
    Status::new(code, msg)
}

/// Convertit une erreur sqlx en `Status::Internal`. Utilise par les handlers
/// gRPC qui font du SQL direct (community, etc.) pour eviter les
/// `.map_err(|e| Status::internal(format!("...: {e}")))` inline repetes.
pub fn sqlx_to_status(context: &str) -> impl Fn(sqlx::Error) -> Status + '_ {
    move |e| Status::internal(format!("{context}: {e}"))
}

#[cfg(test)]
#[path = "tests/errors.rs"]
mod tests;
