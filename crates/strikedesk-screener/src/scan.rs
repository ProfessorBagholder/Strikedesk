use std::sync::Arc;

use serde::Serialize;
use strikedesk_core::{evaluate, momentum_score, prepare, Badge, Check, StrikePart, Tone};
use strikedesk_data::{MarketData, YahooSource};
use thiserror::Error;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::{Config, Preset};

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("config: {0}")]
    Config(String),
    #[error("data: {0}")]
    Data(String),
    #[error("unknown preset {0}")]
    UnknownPreset(String),
    #[error("unknown source {0}")]
    UnknownSource(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanReport {
    pub desk: &'static str,
    pub orders_enabled: bool,
    pub source: String,
    pub benchmark: String,
    pub as_of: Option<String>,
    pub preset: PresetDto,
    pub presets: Vec<PresetDto>,
    pub count: usize,
    pub notice: String,
    pub rows: Vec<ScanRow>,
    pub errors: Vec<SymbolError>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PresetDto {
    pub id: String,
    pub name: String,
    pub require: Vec<String>,
    pub min_strike: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct SymbolError {
    pub symbol: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanRow {
    pub symbol: String,
    pub name: Option<String>,
    pub exchange: Option<String>,
    pub market_cap: Option<f64>,
    pub last: f64,
    pub change_pct: f64,
    pub month_pct: Option<f64>,
    pub adr_pct: Option<f64>,
    pub avg_dollar_volume: Option<f64>,
    pub rs: u8,
    pub strike: u8,
    pub strike_tone: Tone,
    pub as_of: String,
    pub tv_url: String,
    pub badges: Vec<Badge>,
    pub checks: Vec<Check>,
    pub strike_parts: Vec<StrikePart>,
    pub metrics: MetricDto,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricDto {
    pub dist_ema10_pct: Option<f64>,
    pub dist_year_high_pct: Option<f64>,
    pub prior_move_pct: Option<f64>,
    pub stage_advance_pct: Option<f64>,
    pub sma10: Option<f64>,
    pub sma20: Option<f64>,
    pub sma50: Option<f64>,
    pub sma150: Option<f64>,
    pub sma200: Option<f64>,
    pub ema10: Option<f64>,
    pub ema20: Option<f64>,
    pub pivot: Option<f64>,
    pub bars_since_breakout: Option<usize>,
    pub pivot_extension_pct: Option<f64>,
}

struct PreparedSymbol {
    symbol: String,
    name: Option<String>,
    exchange: Option<String>,
    market_cap: Option<f64>,
    prepared: strikedesk_core::Prepared,
    score: f64,
}

pub async fn scan(
    config: &Config,
    source_id: &str,
    preset_id: &str,
) -> Result<ScanReport, ScanError> {
    let preset = config
        .presets
        .iter()
        .find(|preset| preset.id.eq_ignore_ascii_case(preset_id))
        .cloned()
        .ok_or_else(|| ScanError::UnknownPreset(preset_id.to_string()))?;
    let book = strikedesk_data::load_book(&config.data.fixtures_path)
        .map_err(|err| ScanError::Data(err.to_string()))?;
    let benchmark = if config.data.benchmark.is_empty() {
        book.benchmark.clone()
    } else {
        config.data.benchmark.clone()
    };
    let symbols = if config.universe.symbols.is_empty() {
        book.symbols()
    } else {
        config
            .universe
            .symbols
            .iter()
            .filter(|symbol| !symbol.eq_ignore_ascii_case(&benchmark))
            .cloned()
            .collect()
    };

    let source: Arc<dyn MarketData> = match source_id {
        "fixtures" => Arc::new(book.clone()),
        "yahoo" => Arc::new(
            YahooSource::new(
                &config.data.cache_dir,
                config.data.cache_ttl_hours,
                &config.data.yahoo_base,
            )
            .map_err(|err| ScanError::Data(err.to_string()))?,
        ),
        other => return Err(ScanError::UnknownSource(other.to_string())),
    };

    let bench_series = source.load(&benchmark).await.ok();
    let bench_bars = bench_series.as_ref().map(|series| series.bars.clone());
    let mut notice = match source_id {
        "fixtures" => {
            "Demo fixtures are synthetic paths for local development. They are not live quotes. Badges choose what to chart. Strikedesk does not place broker orders.".to_string()
        }
        "yahoo" => {
            "Daily bars from the public Yahoo chart endpoint, cached on disk. Quotes can be delayed or missing. Badges choose what to chart. Strikedesk does not place broker orders.".to_string()
        }
        _ => "Badges choose what to chart. Strikedesk does not place broker orders.".into(),
    };
    if bench_bars.is_none() {
        notice.push_str(" Benchmark bars were unavailable, so RS uses raw returns.");
    }

    let loaded = load_all(source, symbols).await;
    let mut errors = Vec::new();
    let mut prepared = Vec::new();
    for (symbol, result) in loaded {
        match result {
            Ok(series) => match prepare(&series.bars, bench_bars.as_deref()) {
                Ok(features) => {
                    let weights = &config.badges.rs;
                    let score = momentum_score(
                        &features,
                        weights.w21,
                        weights.w63,
                        weights.w126,
                        weights.w252,
                    );
                    prepared.push(PreparedSymbol {
                        symbol: series.symbol,
                        name: series.name,
                        exchange: series.exchange,
                        market_cap: series.market_cap,
                        prepared: features,
                        score,
                    });
                }
                Err(err) => errors.push(SymbolError {
                    symbol,
                    message: err.to_string(),
                }),
            },
            Err(err) => errors.push(SymbolError {
                symbol,
                message: err.to_string(),
            }),
        }
    }
    prepared.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    let scores: Vec<f64> = prepared.iter().map(|row| row.score).collect();
    let rs_values = strikedesk_core::assign_rs(&scores);

    let mut rows = Vec::new();
    for (row, rs) in prepared.iter().zip(rs_values) {
        let evaluation = evaluate(&row.prepared, rs, &config.badges);
        let scan_row = ScanRow {
            symbol: row.symbol.clone(),
            name: row.name.clone(),
            exchange: row.exchange.clone(),
            market_cap: row.market_cap,
            last: round2(evaluation.last),
            change_pct: round1(evaluation.change_pct * 100.0),
            month_pct: evaluation.month_pct.map(|value| round1(value * 100.0)),
            adr_pct: evaluation.adr_pct.map(|value| round1(value * 100.0)),
            avg_dollar_volume: evaluation.avg_dollar_volume,
            rs: evaluation.rs,
            strike: evaluation.strike,
            strike_tone: evaluation.strike_tone,
            as_of: evaluation.date.clone(),
            tv_url: tv_url(row.exchange.as_deref(), &row.symbol),
            badges: evaluation.badges,
            checks: evaluation.checks,
            strike_parts: evaluation.strike_parts,
            metrics: MetricDto {
                dist_ema10_pct: evaluation
                    .metrics
                    .dist_ema10
                    .map(|value| round1(value * 100.0)),
                dist_year_high_pct: evaluation
                    .metrics
                    .dist_year_high
                    .map(|value| round1(value * 100.0)),
                prior_move_pct: evaluation
                    .metrics
                    .prior_move
                    .map(|value| round1(value * 100.0)),
                stage_advance_pct: evaluation
                    .metrics
                    .stage_advance
                    .map(|value| round1(value * 100.0)),
                sma10: evaluation.metrics.sma10,
                sma20: evaluation.metrics.sma20,
                sma50: evaluation.metrics.sma50,
                sma150: evaluation.metrics.sma150,
                sma200: evaluation.metrics.sma200,
                ema10: evaluation.metrics.ema10,
                ema20: evaluation.metrics.ema20,
                pivot: evaluation.metrics.pivot.map(round2),
                bars_since_breakout: evaluation.metrics.bars_since_breakout,
                pivot_extension_pct: evaluation
                    .metrics
                    .pivot_extension
                    .map(|value| round1(value * 100.0)),
            },
        };
        if row_matches(&scan_row, &preset) {
            rows.push(scan_row);
        }
    }
    rows.sort_by(|a, b| {
        b.strike
            .cmp(&a.strike)
            .then(b.rs.cmp(&a.rs))
            .then(a.symbol.cmp(&b.symbol))
    });
    let as_of = rows.iter().map(|row| row.as_of.clone()).max();
    let count = rows.len();
    Ok(ScanReport {
        desk: "Strikedesk",
        orders_enabled: false,
        source: source_id.to_string(),
        benchmark,
        as_of,
        preset: PresetDto::from(&preset),
        presets: config.presets.iter().map(PresetDto::from).collect(),
        count,
        notice,
        rows,
        errors,
    })
}

fn row_matches(row: &ScanRow, preset: &Preset) -> bool {
    if row.strike < preset.min_strike {
        return false;
    }
    preset.require.iter().all(|code| {
        row.badges
            .iter()
            .any(|badge| badge.id.code().eq_ignore_ascii_case(code) && badge.passed)
    })
}

impl PresetDto {
    fn from(preset: &Preset) -> Self {
        Self {
            id: preset.id.clone(),
            name: preset.name.clone(),
            require: preset.require.clone(),
            min_strike: preset.min_strike,
        }
    }
}

async fn load_all(
    source: Arc<dyn MarketData>,
    symbols: Vec<String>,
) -> Vec<(
    String,
    Result<strikedesk_data::Series, strikedesk_data::DataError>,
)> {
    let semaphore = Arc::new(Semaphore::new(4));
    let mut tasks = JoinSet::new();
    for symbol in symbols {
        let source = source.clone();
        let semaphore = semaphore.clone();
        tasks.spawn(async move {
            let Ok(_permit) = semaphore.acquire().await else {
                return (
                    symbol.clone(),
                    Err(strikedesk_data::DataError::UnknownSymbol(symbol)),
                );
            };
            let result = source.load(&symbol).await;
            (symbol, result)
        });
    }
    let mut out = Vec::new();
    while let Some(joined) = tasks.join_next().await {
        if let Ok(item) = joined {
            out.push(item);
        }
    }
    out
}

fn tv_url(exchange: Option<&str>, symbol: &str) -> String {
    match exchange.map(map_exchange) {
        Some(exchange) if !exchange.is_empty() => {
            format!("https://www.tradingview.com/chart/?symbol={exchange}:{symbol}&interval=D")
        }
        _ => format!("https://www.tradingview.com/chart/?symbol={symbol}&interval=D"),
    }
}

fn map_exchange(raw: &str) -> String {
    match raw.trim().to_ascii_uppercase().as_str() {
        "NMS" | "NGM" | "NCM" | "NASDAQ" | "NASDAQGS" | "NASDAQGM" | "NASDAQCM" => "NASDAQ".into(),
        "NYQ" | "NYSE" => "NYSE".into(),
        "PCX" | "ASE" | "AMEX" => "AMEX".into(),
        other => other.to_string(),
    }
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load_config;
    use std::path::Path;

    #[tokio::test]
    async fn fixture_desk_ranks_and_covers_badges() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/screener.toml");
        let config = load_config(&path).unwrap();
        let report = scan(&config, "fixtures", "qullamaggie").await.unwrap();
        assert!(!report.orders_enabled);
        assert!(report.count >= 15, "{}", report.count);
        assert!(report
            .rows
            .windows(2)
            .all(|pair| pair[0].strike >= pair[1].strike));
        let mut seen = std::collections::BTreeSet::new();
        let mut lines = Vec::new();
        for row in &report.rows {
            assert!(row.strike <= 100);
            assert!((1..=99).contains(&row.rs));
            assert!(row.tv_url.contains("tradingview.com"));
            let labels: Vec<_> = row
                .badges
                .iter()
                .filter(|badge| badge.passed)
                .map(|badge| badge.id.code())
                .collect();
            for label in &labels {
                seen.insert(*label);
            }
            lines.push(format!(
                "{} strike={} rs={} {}",
                row.symbol,
                row.strike,
                row.rs,
                labels.join(",")
            ));
        }
        let dump = lines.join("\n");
        let qmco = report
            .rows
            .iter()
            .find(|row| row.symbol == "QMCO")
            .expect("QMCO");
        let fresh = report
            .rows
            .iter()
            .find(|row| row.symbol == "ARM")
            .expect("ARM");
        assert!(
            fresh.strike > qmco.strike,
            "fresh pivot ARM ({}) must outrank late chase QMCO ({})\n{dump}",
            fresh.strike,
            qmco.strike
        );
        assert_ne!(
            report.rows[0].symbol, "QMCO",
            "late chase ranked first\n{dump}"
        );
        let top_dock = report.rows[0]
            .strike_parts
            .iter()
            .find(|part| part.id == "chase")
            .map(|part| part.points)
            .unwrap_or(0.0);
        assert!(
            top_dock > -8.0,
            "top {} is still a late chase ({top_dock})\n{dump}",
            report.rows[0].symbol
        );
        let qmco_dock = qmco
            .strike_parts
            .iter()
            .find(|part| part.id == "chase")
            .map(|part| part.points)
            .unwrap_or(0.0);
        assert!(
            qmco_dock <= -15.0,
            "QMCO should be docked as a late chase ({qmco_dock})\n{dump}"
        );
        let swks = report
            .rows
            .iter()
            .find(|row| row.symbol == "SWKS")
            .expect("SWKS");
        let swks_rank = report
            .rows
            .iter()
            .position(|row| row.symbol == "SWKS")
            .expect("SWKS rank");
        let swks_dock = swks
            .strike_parts
            .iter()
            .find(|part| part.id == "chase")
            .map(|part| part.points)
            .unwrap_or(0.0);
        assert!(
            swks_dock <= -15.0,
            "SWKS parabolic extension should be docked ({swks_dock})\n{dump}"
        );
        assert!(
            fresh.strike > swks.strike,
            "fresh pivot ARM ({}) must outrank parabolic SWKS ({})\n{dump}",
            fresh.strike,
            swks.strike
        );
        assert!(
            swks_rank >= 4,
            "SWKS rank {swks_rank} strike {} is still near the top\n{dump}",
            swks.strike
        );
        let avgo = report
            .rows
            .iter()
            .find(|row| row.symbol == "AVGO")
            .expect("AVGO");
        let avgo_dock = avgo
            .strike_parts
            .iter()
            .find(|part| part.id == "chase")
            .map(|part| part.points)
            .unwrap_or(0.0);
        assert!(
            avgo_dock > -8.0,
            "stair-step coil AVGO should not be docked as a chase ({avgo_dock})\n{dump}"
        );
        assert!(
            avgo.strike > qmco.strike && avgo.strike > swks.strike,
            "coiled AVGO ({}) must outrank QMCO ({}) and SWKS ({})\n{dump}",
            avgo.strike,
            qmco.strike,
            swks.strike
        );
        for code in [
            "KQ", "MM", "ON", "DB", "SB4", "SBW", "SB9", "TML", "97C", "52W", "ER", "2A", "2B",
            "2C", "RS",
        ] {
            assert!(seen.contains(code), "missing {code}\n{dump}");
        }
    }
}
