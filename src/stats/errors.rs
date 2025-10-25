use thiserror::Error;

#[derive(Debug, Error)]
pub enum StatsError {
    #[error("Repository error: {0}")]
    #[allow(dead_code)] // Error variant for repository failures
    Repository(String),

    #[error("Collector error: {0}")]
    #[allow(dead_code)] // Error variant for collector failures
    Collector(String),

    #[error("Calculator error: {0}")]
    #[allow(dead_code)] // Error variant for calculator failures
    Calculator(String),

    #[error("Validation error: {0}")]
    Validation(String),
}
