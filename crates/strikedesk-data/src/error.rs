use thiserror::Error;

#[derive(Debug, Error)]
pub enum DataError {
    #[error("unknown symbol {0}")]
    UnknownSymbol(String),
    #[error("unknown fixture recipe {0}")]
    UnknownRecipe(String),
    #[error("failed to read {path}: {source}")]
    Read {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to write {path}: {source}")]
    Write {
        path: String,
        source: std::io::Error,
    },
    #[error("invalid fixture file: {0}")]
    Fixture(String),
    #[error("yahoo chart for {symbol}: {message}")]
    Yahoo { symbol: String, message: String },
    #[error("cache decode: {0}")]
    Cache(String),
}
