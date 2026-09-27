//! Market data for Strikedesk.
//!
//! Fixtures are deterministic synthetic paths for the demo and tests.
//! Yahoo daily bars are optional, cached on disk, and never required to boot.

mod error;
mod fixture;
mod yahoo;

pub use error::DataError;
pub use fixture::{load_book, FixtureBook, FixtureSpec};
pub use yahoo::{parse_chart, YahooSource};

use async_trait::async_trait;
use strikedesk_core::Bar;

#[derive(Debug, Clone)]
pub struct Series {
    pub symbol: String,
    pub name: Option<String>,
    pub exchange: Option<String>,
    pub market_cap: Option<f64>,
    pub bars: Vec<Bar>,
    pub stale: bool,
}

#[async_trait]
pub trait MarketData: Send + Sync {
    fn id(&self) -> &'static str;
    async fn load(&self, symbol: &str) -> Result<Series, DataError>;
}

#[async_trait]
impl MarketData for FixtureBook {
    fn id(&self) -> &'static str {
        "fixtures"
    }

    async fn load(&self, symbol: &str) -> Result<Series, DataError> {
        self.series(symbol)
    }
}

#[async_trait]
impl MarketData for YahooSource {
    fn id(&self) -> &'static str {
        "yahoo"
    }

    async fn load(&self, symbol: &str) -> Result<Series, DataError> {
        self.load_symbol(symbol).await
    }
}
