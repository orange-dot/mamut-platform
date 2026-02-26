use thiserror::Error;

#[derive(Debug, Error)]
pub enum MamutError {
    #[error("invalid parameter: {0}")]
    InvalidParam(String),
    #[error("timeout")]
    Timeout,
    #[error("device fault: {0}")]
    Fault(String),
    #[error("not ready")]
    NotReady,
    #[error("communication error: {0}")]
    Communication(String),
}
