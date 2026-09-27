use crate::{DataError, Series};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::Duration;
use strikedesk_core::Bar;

pub struct YahooSource {
    client: reqwest::Client,
    cache_dir: PathBuf,
    ttl: Duration,
    base: String,
}

impl YahooSource {
    pub fn new(
        cache_dir: impl Into<PathBuf>,
        ttl_hours: u64,
        base: impl Into<String>,
    ) -> Result<Self, DataError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .user_agent("Mozilla/5.0 (compatible; Strikedesk/0.1; +https://github.com/ProfessorBagholder/Strikedesk)")
            .build()
            .map_err(|err| DataError::Yahoo {
                symbol: "*".into(),
                message: err.to_string(),
            })?;
        Ok(Self {
            client,
            cache_dir: cache_dir.into(),
            ttl: Duration::from_secs(ttl_hours.saturating_mul(3600).max(60)),
            base: base.into().trim_end_matches('/').to_string(),
        })
    }

    pub async fn load_symbol(&self, symbol: &str) -> Result<Series, DataError> {
        let symbol = symbol.trim().to_ascii_uppercase();
        if symbol.is_empty()
            || symbol.len() > 16
            || !symbol
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-')
        {
            return Err(DataError::UnknownSymbol(symbol));
        }
        if let Some(cached) = read_cache(&self.cache_dir, &symbol, self.ttl, false)? {
            return Ok(cached);
        }
        match self.fetch(&symbol).await {
            Ok(series) => {
                if let Err(err) = write_cache(&self.cache_dir, &series) {
                    tracing_miss(err);
                }
                Ok(series)
            }
            Err(err) => {
                if let Some(mut stale) = read_cache(&self.cache_dir, &symbol, self.ttl, true)? {
                    stale.stale = true;
                    Ok(stale)
                } else {
                    Err(err)
                }
            }
        }
    }

    async fn fetch(&self, symbol: &str) -> Result<Series, DataError> {
        let url = format!(
            "{}/v8/finance/chart/{symbol}?interval=1d&range=2y&events=div%2Csplit",
            self.base
        );
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|err| DataError::Yahoo {
                symbol: symbol.to_string(),
                message: err.to_string(),
            })?;
        if !response.status().is_success() {
            return Err(DataError::Yahoo {
                symbol: symbol.to_string(),
                message: format!("chart HTTP {}", response.status()),
            });
        }
        let body = response.text().await.map_err(|err| DataError::Yahoo {
            symbol: symbol.to_string(),
            message: err.to_string(),
        })?;
        let mut series = parse_chart(symbol, &body)?;
        if let Ok(quote) = self.fetch_quote(symbol).await {
            if series.market_cap.is_none() {
                series.market_cap = quote.market_cap;
            }
            if series.name.is_none() {
                series.name = quote.name;
            }
            if series.exchange.is_none() {
                series.exchange = quote.exchange;
            }
        }
        Ok(series)
    }

    async fn fetch_quote(&self, symbol: &str) -> Result<QuoteBits, DataError> {
        let url = format!("{}/v7/finance/quote?symbols={symbol}", self.base);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|err| DataError::Yahoo {
                symbol: symbol.to_string(),
                message: err.to_string(),
            })?;
        if !response.status().is_success() {
            return Err(DataError::Yahoo {
                symbol: symbol.to_string(),
                message: format!("quote HTTP {}", response.status()),
            });
        }
        let body = response.text().await.map_err(|err| DataError::Yahoo {
            symbol: symbol.to_string(),
            message: err.to_string(),
        })?;
        parse_quote(&body).ok_or_else(|| DataError::Yahoo {
            symbol: symbol.to_string(),
            message: "quote payload had no row".into(),
        })
    }
}

struct QuoteBits {
    market_cap: Option<f64>,
    name: Option<String>,
    exchange: Option<String>,
}

pub fn parse_chart(symbol: &str, body: &str) -> Result<Series, DataError> {
    let parsed: ChartResponse = serde_json::from_str(body).map_err(|err| DataError::Yahoo {
        symbol: symbol.to_string(),
        message: format!("chart json: {err}"),
    })?;
    if let Some(error) = parsed.chart.error {
        return Err(DataError::Yahoo {
            symbol: symbol.to_string(),
            message: error.to_string(),
        });
    }
    let result = parsed
        .chart
        .result
        .and_then(|mut rows| rows.pop())
        .ok_or_else(|| DataError::Yahoo {
            symbol: symbol.to_string(),
            message: "empty chart result".into(),
        })?;
    let quote = result
        .indicators
        .quote
        .into_iter()
        .next()
        .ok_or_else(|| DataError::Yahoo {
            symbol: symbol.to_string(),
            message: "missing quote arrays".into(),
        })?;
    let stamps = result.timestamp.unwrap_or_default();
    let mut bars = Vec::new();
    for (idx, stamp) in stamps.iter().enumerate() {
        let Some(stamp) = stamp else { continue };
        let open = quote.open.get(idx).copied().flatten();
        let high = quote.high.get(idx).copied().flatten();
        let low = quote.low.get(idx).copied().flatten();
        let close = quote.close.get(idx).copied().flatten();
        let Some(close) = close else { continue };
        if !close.is_finite() || close <= 0.0 {
            continue;
        }
        let Some(date) = DateTime::<Utc>::from_timestamp(*stamp, 0) else {
            continue;
        };
        let volume = quote.volume.get(idx).copied().flatten().unwrap_or(0.0);
        bars.push(Bar {
            date: date.format("%Y-%m-%d").to_string(),
            open: open.unwrap_or(close),
            high: high.unwrap_or(close),
            low: low.unwrap_or(close),
            close,
            volume: if volume.is_finite() {
                volume.max(0.0)
            } else {
                0.0
            },
        });
    }
    if bars.is_empty() {
        return Err(DataError::Yahoo {
            symbol: symbol.to_string(),
            message: "chart contained no daily bars".into(),
        });
    }
    Ok(Series {
        symbol: symbol.to_string(),
        name: result.meta.short_name,
        exchange: result.meta.exchange_name.or(result.meta.full_exchange_name),
        market_cap: None,
        bars,
        stale: false,
    })
}

fn parse_quote(body: &str) -> Option<QuoteBits> {
    let parsed: QuoteResponse = serde_json::from_str(body).ok()?;
    let row = parsed.quote_response.result?.into_iter().next()?;
    Some(QuoteBits {
        market_cap: row.market_cap.filter(|cap| cap.is_finite() && *cap > 0.0),
        name: row.short_name,
        exchange: row.full_exchange_name.or(row.exchange),
    })
}

#[derive(Deserialize)]
struct ChartResponse {
    chart: ChartBody,
}

#[derive(Deserialize)]
struct ChartBody {
    result: Option<Vec<ChartResult>>,
    #[serde(default)]
    error: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct ChartResult {
    #[serde(default)]
    meta: ChartMeta,
    timestamp: Option<Vec<Option<i64>>>,
    indicators: ChartIndicators,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChartMeta {
    short_name: Option<String>,
    exchange_name: Option<String>,
    full_exchange_name: Option<String>,
}

#[derive(Deserialize)]
struct ChartIndicators {
    quote: Vec<ChartQuote>,
}

#[derive(Default, Deserialize)]
struct ChartQuote {
    #[serde(default)]
    open: Vec<Option<f64>>,
    #[serde(default)]
    high: Vec<Option<f64>>,
    #[serde(default)]
    low: Vec<Option<f64>>,
    #[serde(default)]
    close: Vec<Option<f64>>,
    #[serde(default)]
    volume: Vec<Option<f64>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct QuoteResponse {
    quote_response: QuoteBody,
}

#[derive(Deserialize)]
struct QuoteBody {
    result: Option<Vec<QuoteRow>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct QuoteRow {
    market_cap: Option<f64>,
    short_name: Option<String>,
    full_exchange_name: Option<String>,
    exchange: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CacheFile {
    fetched_unix: i64,
    symbol: String,
    name: Option<String>,
    exchange: Option<String>,
    market_cap: Option<f64>,
    bars: Vec<Bar>,
}

fn cache_path(dir: &Path, symbol: &str) -> PathBuf {
    dir.join(format!("{symbol}.json"))
}

fn read_cache(
    dir: &Path,
    symbol: &str,
    ttl: Duration,
    allow_stale: bool,
) -> Result<Option<Series>, DataError> {
    let path = cache_path(dir, symbol);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(DataError::Read {
                path: path.display().to_string(),
                source,
            })
        }
    };
    let file: CacheFile =
        serde_json::from_str(&text).map_err(|err| DataError::Cache(err.to_string()))?;
    let age = Utc::now().timestamp().saturating_sub(file.fetched_unix);
    if !allow_stale && age > ttl.as_secs() as i64 {
        return Ok(None);
    }
    Ok(Some(Series {
        symbol: file.symbol,
        name: file.name,
        exchange: file.exchange,
        market_cap: file.market_cap,
        bars: file.bars,
        stale: allow_stale && age > ttl.as_secs() as i64,
    }))
}

fn write_cache(dir: &Path, series: &Series) -> Result<(), DataError> {
    std::fs::create_dir_all(dir).map_err(|source| DataError::Write {
        path: dir.display().to_string(),
        source,
    })?;
    let file = CacheFile {
        fetched_unix: Utc::now().timestamp(),
        symbol: series.symbol.clone(),
        name: series.name.clone(),
        exchange: series.exchange.clone(),
        market_cap: series.market_cap,
        bars: series.bars.clone(),
    };
    let path = cache_path(dir, &series.symbol);
    let body = serde_json::to_string(&file).map_err(|err| DataError::Cache(err.to_string()))?;
    std::fs::write(&path, body).map_err(|source| DataError::Write {
        path: path.display().to_string(),
        source,
    })
}

fn tracing_miss(err: DataError) {
    eprintln!("strikedesk cache write skipped: {err}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_small_chart_and_skips_nulls() {
        let body = r#"{
            "chart": {
                "result": [{
                    "meta": {"shortName": "Example", "exchangeName": "NMS"},
                    "timestamp": [1727222400, null, 1727308800],
                    "indicators": {
                        "quote": [{
                            "open": [10.0, 11.0, 12.0],
                            "high": [10.5, 11.5, 12.4],
                            "low": [9.8, 10.8, 11.9],
                            "close": [10.2, null, 12.2],
                            "volume": [1000, 1100, 1200]
                        }]
                    }
                }],
                "error": null
            }
        }"#;
        let series = parse_chart("EX", body).unwrap();
        assert_eq!(series.bars.len(), 2);
        assert_eq!(series.name.as_deref(), Some("Example"));
        assert_eq!(series.exchange.as_deref(), Some("NMS"));
        assert!(series.bars[0].date < series.bars[1].date);
    }

    #[test]
    fn cache_round_trip_respects_ttl() {
        let dir = std::env::temp_dir().join(format!("strikedesk-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let series = Series {
            symbol: "EX".into(),
            name: Some("Example".into()),
            exchange: Some("NASDAQ".into()),
            market_cap: Some(1_000_000.0),
            bars: vec![Bar {
                date: "2026-09-25".into(),
                open: 1.0,
                high: 1.2,
                low: 0.9,
                close: 1.1,
                volume: 10.0,
            }],
            stale: false,
        };
        write_cache(&dir, &series).unwrap();
        let fresh = read_cache(&dir, "EX", Duration::from_secs(3600), false)
            .unwrap()
            .unwrap();
        assert!(!fresh.stale);
        assert_eq!(fresh.bars.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
