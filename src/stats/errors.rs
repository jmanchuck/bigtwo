use thiserror::Error;

#[derive(Debug, Error)]
pub enum StatsError {
    #[error("Repository error: {0}")]
    Repository(String),

    #[error("Collector error: {0}")]
    Collector(String),

    #[error("Calculator error: {0}")]
    Calculator(String),

    #[error("Validation error: {0}")]
    Validation(String),
}
