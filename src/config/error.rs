//! Configuration errors
use config::ConfigError;

/// Configuration error variants
#[derive(Debug, thiserror::Error)]
pub enum SpoxConfigError {
    /// An error returned for duration parameters that must be positive
    #[error("duration for {0} must be nonzero")]
    ZeroDurationForbidden(&'static str),

    /// An error returned during parsing and building the configuration object
    #[error("cannot parse and build configuration")]
    ConfigError(#[from] ConfigError),
}
