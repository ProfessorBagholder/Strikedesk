//! Screener service. Loads config, pulls bars through a market-data adapter,
//! and returns a ranked desk. It never submits an order.

mod config;
mod scan;

pub use config::{load_config, Config, DataConfig, Preset, ServerConfig};
pub use scan::{scan, MetricDto, ScanError, ScanReport, ScanRow, SymbolError};
