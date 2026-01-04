use thiserror::Error;

#[derive(Debug, Error)]
pub enum PetrolError {
    #[error("Schema parse error: {0}")]
    SchemaParse(String),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    #[error("Unsupported feature: {0}")]
    Unsupported(String),
    #[error("{0}")]
    Other(String),
}

impl PetrolError {
    pub fn validation<T: Into<String>>(msg: T) -> Self {
        Self::Validation(msg.into())
    }
}
