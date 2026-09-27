use crate::{DataError, Series};
use chrono::{Datelike, NaiveDate};
use serde::Deserialize;
use strikedesk_core::{compose_bar, Bar};

#[derive(Debug, Clone, Deserialize)]
pub struct FixtureFile {
    #[serde(default = "default_benchmark")]
    pub benchmark: String,
    pub symbols: Vec<FixtureSpec>,
}

fn default_benchmark() -> String {
    "SPY".into()
}

#[derive(Debug, Clone, Deserialize)]
pub struct FixtureSpec {
    pub symbol: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub exchange: Option<String>,
    #[serde(default)]
    pub market_cap: Option<f64>,
    pub recipe: String,
    #[serde(default)]
    pub drift: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct FixtureBook {
    pub benchmark: String,
    specs: Vec<FixtureSpec>,
}

impl FixtureBook {
    pub fn specs(&self) -> &[FixtureSpec] {
        &self.specs
    }

    pub fn symbols(&self) -> Vec<String> {
        self.specs
            .iter()
            .filter(|spec| spec.symbol != self.benchmark)
            .map(|spec| spec.symbol.clone())
            .collect()
    }

    pub fn series(&self, symbol: &str) -> Result<Series, DataError> {
        let spec = self
            .specs
            .iter()
            .find(|spec| spec.symbol.eq_ignore_ascii_case(symbol))
            .ok_or_else(|| DataError::UnknownSymbol(symbol.to_string()))?;
        let bars = synthesize(&spec.recipe, spec.drift)?;
        Ok(Series {
            symbol: spec.symbol.clone(),
            name: spec.name.clone().or_else(|| Some(spec.symbol.clone())),
            exchange: spec.exchange.clone(),
            market_cap: spec.market_cap.filter(|cap| *cap > 0.0),
            bars,
            stale: false,
        })
    }
}

pub fn load_book(path: &str) -> Result<FixtureBook, DataError> {
    let text = std::fs::read_to_string(path).map_err(|source| DataError::Read {
        path: path.to_string(),
        source,
    })?;
    let file: FixtureFile =
        toml::from_str(&text).map_err(|err| DataError::Fixture(err.to_string()))?;
    if file.symbols.is_empty() {
        return Err(DataError::Fixture("universe has no symbols".into()));
    }
    Ok(FixtureBook {
        benchmark: file.benchmark,
        specs: file.symbols,
    })
}

struct Seg {
    len: usize,
    daily: f64,
    range: f64,
    pos: f64,
    vol: f64,
    gap: Option<f64>,
}

fn synthesize(recipe: &str, drift: Option<f64>) -> Result<Vec<Bar>, DataError> {
    let segs = match recipe {
        "benchmark" => vec![Seg {
            len: 320,
            daily: 0.00035,
            range: 0.008,
            pos: 0.6,
            vol: 80_000_000.0,
            gap: None,
        }],
        // Through a shelf and still extended, with a long upper wick on the
        // last bar. This is the late-chase shape, not a fresh pivot buy.
        "leader_box" => vec![
            Seg {
                len: 210,
                daily: 0.0018,
                range: 0.03,
                pos: 0.65,
                vol: 1_600_000.0,
                gap: None,
            },
            Seg {
                len: 12,
                daily: 0.0,
                range: 0.045,
                pos: 0.5,
                vol: 1_200_000.0,
                gap: None,
            },
            Seg {
                len: 10,
                daily: 0.022,
                range: 0.05,
                pos: 0.82,
                vol: 2_600_000.0,
                gap: None,
            },
            Seg {
                len: 1,
                daily: -0.018,
                range: 0.09,
                pos: 0.22,
                vol: 3_200_000.0,
                gap: None,
            },
        ],
        "kq_coil" => vec![
            Seg {
                len: 80,
                daily: 0.008,
                range: 0.05,
                pos: 0.62,
                vol: 1_000_000.0,
                gap: None,
            },
            Seg {
                len: 12,
                daily: 0.0,
                range: 0.045,
                pos: 0.55,
                vol: 1_000_000.0,
                gap: None,
            },
        ],
        // Thrust, then a multi-week pause on the short averages. Equity stand-in
        // for that stair-step shape. The coil has not left SMA(20).
        "stair_step" => vec![
            Seg {
                len: 200,
                daily: 0.0012,
                range: 0.028,
                pos: 0.6,
                vol: 2_000_000.0,
                gap: None,
            },
            Seg {
                len: 12,
                daily: 0.0,
                range: 0.03,
                pos: 0.5,
                vol: 1_400_000.0,
                gap: None,
            },
            Seg {
                len: 8,
                daily: 0.028,
                range: 0.035,
                pos: 0.8,
                vol: 3_000_000.0,
                gap: None,
            },
            Seg {
                len: 18,
                daily: 0.0,
                range: 0.025,
                pos: 0.55,
                vol: 1_600_000.0,
                gap: None,
            },
        ],
        // Larger-float parabola: a liquid shelf, then a month-long climb that
        // rides the short average and finishes stretched above SMA(20).
        "parabolic" => vec![
            Seg {
                len: 180,
                daily: 0.00035,
                range: 0.018,
                pos: 0.55,
                vol: 8_000_000.0,
                gap: None,
            },
            Seg {
                len: 12,
                daily: 0.0,
                range: 0.028,
                pos: 0.5,
                vol: 6_000_000.0,
                gap: None,
            },
            Seg {
                len: 26,
                daily: 0.012,
                range: 0.032,
                pos: 0.74,
                vol: 9_000_000.0,
                gap: None,
            },
        ],
        "fresh_pivot" => vec![
            Seg {
                len: 180,
                daily: 0.0016,
                range: 0.028,
                pos: 0.62,
                vol: 1_800_000.0,
                gap: None,
            },
            Seg {
                len: 12,
                daily: 0.0,
                range: 0.03,
                pos: 0.5,
                vol: 1_200_000.0,
                gap: None,
            },
            Seg {
                len: 1,
                daily: 0.03,
                range: 0.028,
                pos: 0.97,
                vol: 3_200_000.0,
                gap: None,
            },
        ],
        "extended" => vec![
            Seg {
                len: 240,
                daily: 0.0025,
                range: 0.05,
                pos: 0.7,
                vol: 3_000_000.0,
                gap: None,
            },
            Seg {
                len: 18,
                daily: 0.012,
                range: 0.055,
                pos: 0.93,
                vol: 5_000_000.0,
                gap: None,
            },
        ],
        "blowoff" => vec![
            Seg {
                len: 220,
                daily: 0.002,
                range: 0.05,
                pos: 0.6,
                vol: 1_000_000.0,
                gap: None,
            },
            Seg {
                len: 25,
                daily: 0.02,
                range: 0.09,
                pos: 0.25,
                vol: 4_000_000.0,
                gap: None,
            },
        ],
        "on_high" => vec![Seg {
            len: 80,
            daily: 0.004,
            range: 0.035,
            pos: 0.99,
            vol: 1_000_000.0,
            gap: None,
        }],
        "steady" => vec![Seg {
            len: 280,
            daily: drift.unwrap_or(0.0016),
            range: 0.028,
            pos: 0.7,
            vol: 2_000_000.0,
            gap: None,
        }],
        "thrust" => vec![
            Seg {
                len: 200,
                daily: 0.0018,
                range: 0.03,
                pos: 0.65,
                vol: 1_200_000.0,
                gap: None,
            },
            Seg {
                len: 4,
                daily: 0.02,
                range: 0.045,
                pos: 0.88,
                vol: 3_200_000.0,
                gap: None,
            },
        ],
        "weekly_surf" => vec![Seg {
            len: 120,
            daily: 0.00035,
            range: 0.012,
            pos: 0.72,
            vol: 3_000_000.0,
            gap: None,
        }],
        "early_base" => vec![
            Seg {
                len: 200,
                daily: 0.00005,
                range: 0.012,
                pos: 0.5,
                vol: 800_000.0,
                gap: None,
            },
            Seg {
                len: 25,
                daily: 0.0035,
                range: 0.02,
                pos: 0.7,
                vol: 1_000_000.0,
                gap: None,
            },
        ],
        "tight_flag" => vec![
            Seg {
                len: 40,
                daily: 0.001,
                range: 0.03,
                pos: 0.6,
                vol: 1_000_000.0,
                gap: None,
            },
            Seg {
                len: 15,
                daily: 0.012,
                range: 0.035,
                pos: 0.7,
                vol: 1_500_000.0,
                gap: None,
            },
            Seg {
                len: 8,
                daily: 0.0,
                range: 0.02,
                pos: 0.7,
                vol: 900_000.0,
                gap: None,
            },
            Seg {
                len: 1,
                daily: 0.012,
                range: 0.02,
                pos: 0.95,
                vol: 900_000.0,
                gap: None,
            },
        ],
        "ma_lift" => vec![Seg {
            len: 50,
            daily: 0.0012,
            range: 0.028,
            pos: 0.78,
            vol: 8_000_000.0,
            gap: None,
        }],
        "chop" => {
            let mut segs = Vec::new();
            for block in 0..16 {
                let daily = if block % 2 == 0 { 0.006 } else { -0.006 };
                segs.push(Seg {
                    len: 10,
                    daily,
                    range: 0.02,
                    pos: 0.45,
                    vol: 400_000.0,
                    gap: None,
                });
            }
            segs
        }
        "episodic" => vec![
            Seg {
                len: 40,
                daily: 0.0015,
                range: 0.025,
                pos: 0.6,
                vol: 1_000_000.0,
                gap: None,
            },
            Seg {
                len: 1,
                daily: 0.01,
                range: 0.03,
                pos: 0.85,
                vol: 4_000_000.0,
                gap: Some(0.08),
            },
            Seg {
                len: 3,
                daily: 0.001,
                range: 0.02,
                pos: 0.62,
                vol: 1_100_000.0,
                gap: None,
            },
        ],
        "early_trend" => vec![
            Seg {
                len: 160,
                daily: 0.0002,
                range: 0.018,
                pos: 0.55,
                vol: 700_000.0,
                gap: None,
            },
            Seg {
                len: 30,
                daily: 0.004,
                range: 0.025,
                pos: 0.72,
                vol: 900_000.0,
                gap: None,
            },
        ],
        other => return Err(DataError::UnknownRecipe(other.to_string())),
    };
    Ok(render(&segs))
}

fn render(segs: &[Seg]) -> Vec<Bar> {
    let total: usize = segs.iter().map(|seg| seg.len).sum();
    let dates = business_days(total);
    let mut bars = Vec::with_capacity(total);
    let mut last = 20.0;
    let mut cursor = 0usize;
    for seg in segs {
        for step in 0..seg.len {
            let gap = if step == 0 { seg.gap } else { None };
            let close = if let Some(gap) = gap {
                last * (1.0 + gap) * (1.0 + seg.daily)
            } else {
                last * (1.0 + seg.daily)
            };
            bars.push(compose_bar(
                &dates[cursor],
                last,
                close,
                seg.range,
                seg.pos,
                seg.vol,
                gap,
            ));
            last = close;
            cursor += 1;
        }
    }
    bars
}

/// Last `n` weekdays ending 2026-09-25, inclusive.
fn business_days(n: usize) -> Vec<String> {
    let mut dates = Vec::with_capacity(n);
    let mut day = NaiveDate::from_ymd_opt(2026, 9, 25).expect("end date");
    while dates.len() < n {
        if day.weekday().number_from_monday() <= 5 {
            dates.push(day.format("%Y-%m-%d").to_string());
        }
        day = day.pred_opt().expect("calendar");
    }
    dates.reverse();
    dates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_ends_on_the_fixed_friday() {
        let dates = business_days(10);
        assert_eq!(dates.last().map(String::as_str), Some("2026-09-25"));
        assert_eq!(dates.len(), 10);
    }

    #[test]
    fn shipped_universe_builds() {
        let book = load_book(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/universe.toml"
        ))
        .unwrap();
        assert!(book.symbols().len() >= 15);
        let qmco = book.series("QMCO").unwrap();
        assert!(qmco.bars.len() > 200);
        assert_eq!(qmco.bars.last().unwrap().date, "2026-09-25");
        assert!(qmco.market_cap.unwrap() > 0.0);
        let spy = book.series("SPY").unwrap();
        assert!(spy.bars.len() >= qmco.bars.len());
    }
}
