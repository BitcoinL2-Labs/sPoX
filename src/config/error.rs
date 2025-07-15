/// Configuration error variants.
#[derive(Debug, thiserror::Error)]
pub enum SpoxConfigError {
    /// An error returned for duration parameters that must be positive.
    #[error("Duration for {0} must be nonzero")]
    ZeroDurationForbidden(&'static str),
}
