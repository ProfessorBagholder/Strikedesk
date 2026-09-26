/// Simple moving average aligned to `values`. Warmup slots are `None`.
pub fn sma(values: &[f64], n: usize) -> Vec<Option<f64>> {
    if n == 0 {
        return vec![None; values.len()];
    }
    let mut out = Vec::with_capacity(values.len());
    let mut sum = 0.0;
    for (i, value) in values.iter().enumerate() {
        sum += value;
        if i >= n {
            sum -= values[i - n];
        }
        if i + 1 >= n && value.is_finite() {
            out.push(Some(sum / n as f64));
        } else {
            out.push(None);
        }
    }
    out
}

/// Exponential moving average. Seeded with the SMA of the first `n` values.
pub fn ema(values: &[f64], n: usize) -> Vec<Option<f64>> {
    if n == 0 {
        return vec![None; values.len()];
    }
    let k = 2.0 / (n as f64 + 1.0);
    let mut out = Vec::with_capacity(values.len());
    let mut seed = 0.0;
    let mut prev: Option<f64> = None;
    for (i, value) in values.iter().enumerate() {
        if !value.is_finite() {
            out.push(None);
            prev = None;
            seed = 0.0;
            continue;
        }
        if prev.is_none() && i + 1 < n {
            seed += value;
            out.push(None);
        } else if prev.is_none() && i + 1 == n {
            seed += value;
            let next = seed / n as f64;
            prev = Some(next);
            out.push(Some(next));
        } else if let Some(p) = prev {
            let next = value * k + p * (1.0 - k);
            prev = Some(next);
            out.push(Some(next));
        } else {
            out.push(None);
        }
    }
    out
}

pub fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let sum: f64 = values.iter().copied().sum();
    Some(sum / values.len() as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sma_warmup_and_value() {
        let values = [1.0, 2.0, 3.0, 4.0];
        let out = sma(&values, 3);
        assert_eq!(out[0], None);
        assert_eq!(out[1], None);
        assert!((out[2].unwrap() - 2.0).abs() < 1e-12);
        assert!((out[3].unwrap() - 3.0).abs() < 1e-12);
    }

    #[test]
    fn ema_matches_seed_then_smooths() {
        let values = [1.0, 2.0, 3.0, 4.0];
        let out = ema(&values, 3);
        assert_eq!(out[0], None);
        assert_eq!(out[1], None);
        assert!((out[2].unwrap() - 2.0).abs() < 1e-12);
        let k = 2.0 / 4.0;
        let expected = 4.0 * k + 2.0 * (1.0 - k);
        assert!((out[3].unwrap() - expected).abs() < 1e-12);
    }
}
