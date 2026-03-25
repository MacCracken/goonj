use thiserror::Error;

/// Errors produced by goonj operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum GoonjError {
    /// Invalid room or wall geometry (e.g. too few vertices).
    #[error("invalid geometry: {0}")]
    InvalidGeometry(String),

    /// Invalid material parameters (e.g. absorption outside 0.0–1.0).
    #[error("invalid material: {0}")]
    InvalidMaterial(String),

    /// Invalid frequency value (e.g. negative Hz).
    #[error("invalid frequency: {0}")]
    InvalidFrequency(String),

    /// Sound propagation computation failed (reserved for future use).
    #[error("propagation failed: {0}")]
    PropagationFailed(String),

    /// General computation error (e.g. I/O failure during WAV export).
    #[error("computation error: {0}")]
    ComputationError(String),
}

/// Convenience result type for goonj operations.
pub type Result<T> = std::result::Result<T, GoonjError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let e = GoonjError::InvalidGeometry("no walls".into());
        assert_eq!(e.to_string(), "invalid geometry: no walls");
    }

    #[test]
    fn error_display_material() {
        let e = GoonjError::InvalidMaterial("negative absorption".into());
        assert!(e.to_string().contains("negative absorption"));
    }

    #[test]
    fn error_display_frequency() {
        let e = GoonjError::InvalidFrequency("below 0 Hz".into());
        assert!(e.to_string().contains("below 0 Hz"));
    }

    #[test]
    fn result_type_works() {
        let ok: Result<f32> = Ok(1.0);
        assert!(ok.is_ok());
        let err: Result<f32> = Err(GoonjError::ComputationError("fail".into()));
        assert!(err.is_err());
    }
}
