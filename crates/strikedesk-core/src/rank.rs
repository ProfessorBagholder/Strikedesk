/// Map raw momentum scores to a 1–99 relative-strength percentile.
///
/// The highest score becomes 99 and the lowest becomes 1. Ties share a
/// mid-rank so equal momentum does not invent a winner. A one-name universe
/// is undefined as a percentile and returns 50.
pub fn assign_rs(scores: &[f64]) -> Vec<u8> {
    let n = scores.len();
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![50];
    }
    let finite: Vec<f64> = scores
        .iter()
        .map(|score| if score.is_finite() { *score } else { f64::MIN })
        .collect();
    finite
        .iter()
        .map(|score| {
            let below = finite.iter().filter(|other| *other < score).count();
            let equal = finite.iter().filter(|other| *other == score).count();
            let pct = (below as f64 + (equal as f64 - 1.0) * 0.5) / (n as f64 - 1.0);
            let rs = (1.0 + pct * 98.0).round();
            rs.clamp(1.0, 99.0) as u8
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn best_and_worst_anchor_the_scale() {
        let rs = assign_rs(&[0.10, 0.50, 0.20]);
        assert_eq!(rs[1], 99);
        assert_eq!(rs[0], 1);
        assert!(rs[2] > rs[0] && rs[2] < rs[1]);
    }

    #[test]
    fn ties_share_a_rank() {
        let rs = assign_rs(&[0.2, 0.2]);
        assert_eq!(rs[0], rs[1]);
    }

    #[test]
    fn singleton_is_neutral() {
        assert_eq!(assign_rs(&[1.0]), vec![50]);
    }
}
