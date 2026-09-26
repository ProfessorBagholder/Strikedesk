use crate::indicators::{ema, sma};
use crate::model::Bar;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalError {
    Empty,
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "no usable daily bars"),
        }
    }
}

impl std::error::Error for EvalError {}

/// Sanitized bars plus fixed-length moving averages and raw return windows.
///
/// Badge thresholds are applied later in `evaluate`, so a config change does
/// not require rebuilding indicator series.
#[derive(Debug, Clone)]
pub struct Prepared {
    pub bars: Vec<Bar>,
    pub date: String,
    pub last: f64,
    pub prev_close: Option<f64>,
    pub change_pct: f64,
    pub sma10: Vec<Option<f64>>,
    pub sma20: Vec<Option<f64>>,
    pub sma50: Vec<Option<f64>>,
    pub sma150: Vec<Option<f64>>,
    pub sma200: Vec<Option<f64>>,
    pub ema10: Vec<Option<f64>>,
    pub ema20: Vec<Option<f64>>,
    pub excess_21: Option<f64>,
    pub excess_63: Option<f64>,
    pub excess_126: Option<f64>,
    pub excess_252: Option<f64>,
}

/// Sort, drop unusable sessions, and compute last-bar inputs.
///
/// Benchmark closes are matched on the stock's dates. With no benchmark, the
/// excess-return slots hold raw stock returns.
pub fn prepare(bars: &[Bar], benchmark: Option<&[Bar]>) -> Result<Prepared, EvalError> {
    let bars = sanitize(bars);
    if bars.is_empty() {
        return Err(EvalError::Empty);
    }
    let bench = benchmark.map(sanitize);
    let bench_by_date = bench.as_ref().map(|rows| {
        rows.iter()
            .map(|bar| (bar.date.as_str(), bar.close))
            .collect::<std::collections::HashMap<_, _>>()
    });
    let closes: Vec<f64> = bars.iter().map(|bar| bar.close).collect();
    let n = closes.len();
    let last = closes[n - 1];
    let prev_close = if n >= 2 { Some(closes[n - 2]) } else { None };
    let change_pct = prev_close
        .filter(|price| *price > 0.0)
        .map(|price| last / price - 1.0)
        .unwrap_or(0.0);
    let aligned: Vec<Option<f64>> = bars
        .iter()
        .map(|bar| {
            bench_by_date
                .as_ref()
                .and_then(|map| map.get(bar.date.as_str()).copied())
        })
        .collect();
    let has_bench = aligned.iter().any(|value| value.is_some());

    Ok(Prepared {
        date: bars[n - 1].date.clone(),
        last,
        prev_close,
        change_pct,
        sma10: sma(&closes, 10),
        sma20: sma(&closes, 20),
        sma50: sma(&closes, 50),
        sma150: sma(&closes, 150),
        sma200: sma(&closes, 200),
        ema10: ema(&closes, 10),
        ema20: ema(&closes, 20),
        excess_21: excess(&closes, &aligned, has_bench, 21),
        excess_63: excess(&closes, &aligned, has_bench, 63),
        excess_126: excess(&closes, &aligned, has_bench, 126),
        excess_252: excess(&closes, &aligned, has_bench, 252),
        bars,
    })
}

pub fn momentum_score(prepared: &Prepared, w21: f64, w63: f64, w126: f64, w252: f64) -> f64 {
    let parts = [
        (prepared.excess_21, w21),
        (prepared.excess_63, w63),
        (prepared.excess_126, w126),
        (prepared.excess_252, w252),
    ];
    let mut num = 0.0;
    let mut den = 0.0;
    for (value, weight) in parts {
        if let Some(value) = value {
            if value.is_finite() && weight > 0.0 {
                num += value * weight;
                den += weight;
            }
        }
    }
    if den == 0.0 {
        0.0
    } else {
        num / den
    }
}

pub(crate) fn series_at(values: &[Option<f64>], index: usize) -> Option<f64> {
    values.get(index).copied().flatten()
}

pub(crate) fn series_last(values: &[Option<f64>]) -> Option<f64> {
    values
        .len()
        .checked_sub(1)
        .and_then(|index| series_at(values, index))
}

pub(crate) fn series_ago(values: &[Option<f64>], sessions_ago: usize) -> Option<f64> {
    values
        .len()
        .checked_sub(sessions_ago + 1)
        .and_then(|index| series_at(values, index))
}

fn sanitize(bars: &[Bar]) -> Vec<Bar> {
    let mut out = Vec::new();
    for bar in bars {
        if bar.date.trim().is_empty() || !bar.close.is_finite() || bar.close <= 0.0 {
            continue;
        }
        if !bar.open.is_finite() || !bar.high.is_finite() || !bar.low.is_finite() {
            continue;
        }
        let mut next = bar.clone();
        next.date = bar.date.trim().to_string();
        if !next.volume.is_finite() || next.volume < 0.0 {
            next.volume = 0.0;
        }
        let peak = next.open.max(next.close).max(next.high).max(next.low);
        let trough = next.open.min(next.close).min(next.high).min(next.low);
        next.high = peak;
        next.low = trough.min(peak);
        out.push(next);
    }
    out.sort_by(|a, b| a.date.cmp(&b.date));
    let mut dedup = Vec::with_capacity(out.len());
    for bar in out {
        if dedup
            .last()
            .map(|prev: &Bar| prev.date == bar.date)
            .unwrap_or(false)
        {
            dedup.pop();
        }
        dedup.push(bar);
    }
    dedup
}

fn excess(closes: &[f64], bench: &[Option<f64>], has_bench: bool, sessions: usize) -> Option<f64> {
    if closes.len() <= sessions {
        return None;
    }
    let i = closes.len() - 1;
    let j = i - sessions;
    if closes[j] <= 0.0 {
        return None;
    }
    let stock = closes[i] / closes[j] - 1.0;
    if !has_bench {
        return Some(stock);
    }
    let b0 = bench.get(j).copied().flatten()?;
    let b1 = bench.get(i).copied().flatten()?;
    if b0 <= 0.0 {
        return None;
    }
    Some(stock - (b1 / b0 - 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose_bar;

    #[test]
    fn sorts_and_keeps_last_duplicate_date() {
        let bars = vec![
            compose_bar("B", 1.0, 2.0, 0.01, 0.5, 10.0, None),
            compose_bar("A", 1.0, 1.5, 0.01, 0.5, 10.0, None),
            compose_bar("A", 1.0, 1.7, 0.01, 0.5, 12.0, None),
        ];
        let prepared = prepare(&bars, None).unwrap();
        assert_eq!(prepared.bars.len(), 2);
        assert_eq!(prepared.date, "B");
        assert!((prepared.bars[0].close - 1.7).abs() < 1e-9);
    }
}
