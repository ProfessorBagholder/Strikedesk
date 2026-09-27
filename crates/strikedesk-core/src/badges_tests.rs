use crate::{
    assign_rs, compose_bar, evaluate, momentum_score, prepare, rulebook, BadgeParams, Bar,
    Evaluation,
};

fn day(i: usize) -> String {
    format!("D{i:04}")
}

struct Path {
    last: f64,
    bars: Vec<Bar>,
}

impl Path {
    fn new(start: f64) -> Self {
        Self {
            last: start,
            bars: Vec::new(),
        }
    }

    fn add(
        &mut self,
        n: usize,
        daily: f64,
        range: f64,
        pos: f64,
        vol: f64,
        gap_first: Option<f64>,
    ) -> &mut Self {
        for step in 0..n {
            let prev = self.last;
            let gap = if step == 0 { gap_first } else { None };
            let close = if let Some(gap) = gap {
                prev * (1.0 + gap) * (1.0 + daily)
            } else {
                prev * (1.0 + daily)
            };
            self.bars.push(compose_bar(
                &day(self.bars.len()),
                prev,
                close,
                range,
                pos,
                vol,
                gap,
            ));
            self.last = close;
        }
        self
    }

    fn done(&self) -> &[Bar] {
        &self.bars
    }
}

fn run(bars: &[Bar], rs: u8) -> Evaluation {
    let prepared = prepare(bars, None).expect("bars");
    evaluate(&prepared, rs, &BadgeParams::default())
}

fn has(eval: &Evaluation, code: &str) -> bool {
    eval.checks
        .iter()
        .any(|check| check.id.code() == code && check.passed)
}

fn detail<'a>(eval: &'a Evaluation, code: &str) -> &'a str {
    eval.checks
        .iter()
        .find(|check| check.id.code() == code)
        .map(|check| check.detail.as_str())
        .unwrap_or("")
}

#[test]
fn strong_close_earns_97c_and_a_weak_close_does_not() {
    let mut up = Path::new(10.0);
    up.add(3, 0.0, 0.02, 0.99, 100_000.0, None);
    let pass = run(up.done(), 50);
    assert!(has(&pass, "97C"), "{}", detail(&pass, "97C"));

    let mut down = Path::new(10.0);
    down.add(3, 0.0, 0.02, 0.20, 100_000.0, None);
    let fail = run(down.done(), 50);
    assert!(!has(&fail, "97C"), "{}", detail(&fail, "97C"));
}

#[test]
fn coiled_leader_earns_kq_and_a_downtrend_does_not() {
    let mut leader = Path::new(20.0);
    leader.add(80, 0.008, 0.05, 0.62, 1_000_000.0, None).add(
        12,
        0.0,
        0.045,
        0.55,
        1_000_000.0,
        None,
    );
    let eval = run(leader.done(), 90);
    assert!(has(&eval, "KQ"), "{}", detail(&eval, "KQ"));

    let mut down = Path::new(40.0);
    down.add(80, -0.004, 0.05, 0.4, 1_000_000.0, None);
    let fail = run(down.done(), 90);
    assert!(!has(&fail, "KQ"), "{}", detail(&fail, "KQ"));
}

#[test]
fn smooth_uptrend_earns_mm_and_a_downtrend_does_not() {
    let mut up = Path::new(50.0);
    up.add(260, 0.002, 0.02, 0.7, 2_000_000.0, None);
    let eval = run(up.done(), 90);
    assert!(has(&eval, "MM"), "{}", detail(&eval, "MM"));

    let mut down = Path::new(80.0);
    down.add(260, -0.0015, 0.02, 0.4, 2_000_000.0, None);
    let fail = run(down.done(), 40);
    assert!(!has(&fail, "MM"), "{}", detail(&fail, "MM"));
}

#[test]
fn dry_box_earns_db() {
    let mut path = Path::new(15.0);
    path.add(30, 0.005, 0.04, 0.6, 2_000_000.0, None)
        .add(10, 0.0, 0.02, 0.9, 700_000.0, None);
    let eval = run(path.done(), 80);
    assert!(has(&eval, "DB"), "{}", detail(&eval, "DB"));
}

#[test]
fn four_day_thrust_earns_sb4() {
    let mut path = Path::new(30.0);
    path.add(25, 0.001, 0.03, 0.6, 1_000_000.0, None)
        .add(4, 0.02, 0.04, 0.8, 2_500_000.0, None);
    let eval = run(path.done(), 70);
    assert!(has(&eval, "SB4"), "{}", detail(&eval, "SB4"));
}

#[test]
fn nine_day_flag_earns_sb9() {
    let mut path = Path::new(12.0);
    path.add(25, 0.001, 0.03, 0.6, 1_000_000.0, None)
        .add(15, 0.012, 0.035, 0.7, 1_500_000.0, None)
        .add(9, 0.0, 0.02, 0.65, 900_000.0, None);
    let eval = run(path.done(), 80);
    assert!(has(&eval, "SB9"), "{}", detail(&eval, "SB9"));
}

#[test]
fn gap_on_volume_is_labeled_with_sessions_ago() {
    let mut path = Path::new(18.0);
    path.add(30, 0.001, 0.02, 0.55, 1_000_000.0, None)
        .add(1, 0.01, 0.03, 0.85, 4_000_000.0, Some(0.08))
        .add(4, 0.001, 0.02, 0.6, 1_000_000.0, None);
    let eval = run(path.done(), 75);
    assert!(has(&eval, "ER"), "{}", detail(&eval, "ER"));
    let label = eval
        .badges
        .iter()
        .find(|badge| badge.id.code() == "ER")
        .map(|badge| badge.label.as_str());
    assert_eq!(label, Some("ER-4"));
}

#[test]
fn early_and_late_stage_two_are_mutually_exclusive() {
    let mut early = Path::new(100.0);
    early.add(180, 0.00005, 0.01, 0.55, 1_000_000.0, None).add(
        25,
        0.0035,
        0.02,
        0.7,
        1_200_000.0,
        None,
    );
    let early_eval = run(early.done(), 60);
    assert!(has(&early_eval, "2A"), "{}", detail(&early_eval, "2A"));
    assert!(!has(&early_eval, "2B"));
    assert!(!has(&early_eval, "2C"));

    let mut late = Path::new(20.0);
    late.add(200, 0.001, 0.03, 0.6, 1_000_000.0, None).add(
        40,
        0.018,
        0.05,
        0.85,
        2_000_000.0,
        None,
    );
    let late_eval = run(late.done(), 95);
    assert!(has(&late_eval, "2C"), "{}", detail(&late_eval, "2C"));
    assert!(!has(&late_eval, "2A"));
    assert!(!has(&late_eval, "2B"));
}

#[test]
fn recent_high_earns_52w_and_a_stale_high_does_not() {
    let mut fresh = Path::new(10.0);
    fresh.add(110, 0.002, 0.02, 0.7, 500_000.0, None);
    let eval = run(fresh.done(), 70);
    assert!(has(&eval, "52W"), "{}", detail(&eval, "52W"));

    let mut stale = Path::new(10.0);
    stale
        .add(100, 0.003, 0.02, 0.7, 500_000.0, None)
        .add(20, -0.004, 0.02, 0.35, 500_000.0, None);
    let fail = run(stale.done(), 40);
    assert!(!has(&fail, "52W"), "{}", detail(&fail, "52W"));
}

#[test]
fn close_at_the_high_earns_on() {
    let mut path = Path::new(22.0);
    path.add(15, 0.004, 0.03, 0.98, 800_000.0, None);
    let eval = run(path.done(), 70);
    assert!(has(&eval, "ON"), "{}", detail(&eval, "ON"));
}

#[test]
fn relative_strength_ranks_the_faster_name_higher() {
    let mut fast = Path::new(10.0);
    fast.add(80, 0.01, 0.03, 0.6, 1_000_000.0, None);
    let mut slow = Path::new(10.0);
    slow.add(80, 0.0, 0.02, 0.5, 1_000_000.0, None);
    let fast_p = prepare(fast.done(), None).unwrap();
    let slow_p = prepare(slow.done(), None).unwrap();
    let params = BadgeParams::default();
    let scores = [
        momentum_score(
            &fast_p,
            params.rs.w21,
            params.rs.w63,
            params.rs.w126,
            params.rs.w252,
        ),
        momentum_score(
            &slow_p,
            params.rs.w21,
            params.rs.w63,
            params.rs.w126,
            params.rs.w252,
        ),
    ];
    let rs = assign_rs(&scores);
    assert!(rs[0] > rs[1], "{rs:?} scores={scores:?}");
    let hot = evaluate(&fast_p, rs[0], &params);
    assert!(has(&hot, "RS"));
    assert!(hot.rs >= params.rs_hot);
}

fn shelf_then(extra: &[(usize, f64, f64, f64, f64)]) -> Vec<Bar> {
    let mut path = Path::new(20.0);
    path.add(180, 0.0016, 0.028, 0.62, 1_800_000.0, None).add(
        12,
        0.0,
        0.03,
        0.5,
        1_200_000.0,
        None,
    );
    for (n, daily, range, pos, vol) in extra {
        path.add(*n, *daily, *range, *pos, *vol, None);
    }
    path.done().to_vec()
}

#[test]
fn fresh_pivot_outranks_a_similar_late_chase() {
    let fresh = run(&shelf_then(&[(1, 0.03, 0.028, 0.97, 3_200_000.0)]), 92);
    let chase = run(
        &shelf_then(&[
            (4, 0.045, 0.04, 0.85, 2_800_000.0),
            (9, 0.0, 0.02, 0.92, 1_600_000.0),
        ]),
        92,
    );
    let fresh_dock = part_points(&fresh, "chase");
    let chase_dock = part_points(&chase, "chase");
    assert!(
        fresh.metrics.bars_since_breakout.unwrap_or(99) <= 2,
        "fresh break should still be the print, got {:?}",
        fresh.metrics.bars_since_breakout
    );
    assert!(
        fresh.metrics.pivot_extension.unwrap_or(1.0) < 0.05,
        "fresh close should still be inside 5% of the pivot, got {:?}",
        fresh.metrics.pivot_extension
    );
    assert!(
        chase.metrics.bars_since_breakout.unwrap_or(0) >= 8,
        "chase breakout should already be stale, got {:?}",
        chase.metrics.bars_since_breakout
    );
    assert!(
        chase.metrics.pivot_extension.unwrap_or(0.0) > 0.10,
        "chase close should be well through the pivot, got {:?}",
        chase.metrics.pivot_extension
    );
    assert!(
        fresh_dock > -1.0,
        "a close still at the pivot is not a chase ({fresh_dock})"
    );
    assert!(
        chase_dock <= -15.0,
        "running past the shelf must dock strike, got {chase_dock}"
    );
    assert!(
        fresh.strike >= chase.strike + 10,
        "fresh strike {} chase {} docks {} / {}",
        fresh.strike,
        chase.strike,
        fresh_dock,
        chase_dock
    );
}

#[test]
fn fresh_pivot_outranks_a_parabolic_extension() {
    let fresh = run(&shelf_then(&[(1, 0.03, 0.028, 0.97, 3_200_000.0)]), 92);
    let mut parabolic = Path::new(60.0);
    parabolic
        .add(160, 0.0004, 0.018, 0.55, 8_000_000.0, None)
        .add(12, 0.0, 0.025, 0.5, 6_000_000.0, None)
        .add(26, 0.012, 0.032, 0.74, 9_000_000.0, None);
    let chase = run(parabolic.done(), 92);
    let sma_ext = chase.last / chase.metrics.sma20.expect("sma20") - 1.0;
    assert!(
        sma_ext > 0.06,
        "parabolic close should be stretched above SMA(20), got {sma_ext}"
    );
    assert!(
        chase.metrics.month_pct.unwrap_or(0.0) > 0.18,
        "21-session return should be parabolic, got {:?}",
        chase.metrics.month_pct
    );
    assert!(
        chase.avg_dollar_volume.unwrap_or(0.0) > 20_000_000.0,
        "larger-float path should stay liquid"
    );
    let chase_dock = part_points(&chase, "chase");
    assert!(
        chase_dock <= -20.0,
        "SMA(20) stretch after a large advance must dock, got {chase_dock}"
    );
    assert!(
        fresh.strike >= chase.strike + 10,
        "fresh {} parabolic {} dock {chase_dock} sma_ext {sma_ext}",
        fresh.strike,
        chase.strike
    );
}

fn stair_step(break_daily: Option<f64>) -> Vec<Bar> {
    let mut path = Path::new(40.0);
    path.add(200, 0.0012, 0.028, 0.6, 2_000_000.0, None)
        .add(12, 0.0, 0.03, 0.5, 1_400_000.0, None)
        .add(8, 0.028, 0.035, 0.8, 3_000_000.0, None)
        .add(18, 0.0, 0.025, 0.55, 1_600_000.0, None);
    if let Some(daily) = break_daily {
        path.add(1, daily, 0.028, 0.9, 2_800_000.0, None);
    }
    path.done().to_vec()
}

#[test]
fn coiled_post_thrust_base_outranks_late_chases() {
    let coil = run(&stair_step(None), 92);
    let early = run(&stair_step(Some(0.02)), 92);
    let thin = run(
        &shelf_then(&[
            (4, 0.045, 0.04, 0.85, 2_800_000.0),
            (9, 0.0, 0.02, 0.92, 1_600_000.0),
        ]),
        92,
    );
    let mut parabolic = Path::new(60.0);
    parabolic
        .add(160, 0.0004, 0.018, 0.55, 8_000_000.0, None)
        .add(12, 0.0, 0.025, 0.5, 6_000_000.0, None)
        .add(26, 0.012, 0.032, 0.74, 9_000_000.0, None);
    let chase = run(parabolic.done(), 92);

    let coil_sma = coil.last / coil.metrics.sma20.expect("sma20") - 1.0;
    let coil_ema = (coil.last / coil.metrics.ema10.expect("ema10") - 1.0).abs();
    assert!(
        (-0.03..=0.05).contains(&coil_sma),
        "coil should sit on SMA(20), got {coil_sma}"
    );
    assert!(
        coil_ema <= 0.04,
        "coil should sit on EMA(10), got {coil_ema}"
    );
    assert!(
        coil.metrics.pivot_extension.unwrap_or(0.0) > 0.12,
        "the launch shelf should still be the pivot, got {:?}",
        coil.metrics.pivot_extension
    );
    assert!(
        coil.metrics.bars_since_breakout.unwrap_or(0) >= 15,
        "the coil is weeks after the thrust, got {:?}",
        coil.metrics.bars_since_breakout
    );
    let coil_dock = part_points(&coil, "chase");
    assert!(
        coil_dock > -1.0,
        "a pause on the short averages is not a chase ({coil_dock})"
    );

    assert!(
        early.metrics.bars_since_breakout.unwrap_or(99) <= 2,
        "early break should be the print, got {:?}",
        early.metrics.bars_since_breakout
    );
    assert!(
        early.metrics.pivot_extension.unwrap_or(1.0) < 0.05,
        "early break should still be at the coil high, got {:?}",
        early.metrics.pivot_extension
    );
    let early_sma = early.last / early.metrics.sma20.expect("sma20") - 1.0;
    assert!(
        early_sma <= 0.05,
        "early break should not have left SMA(20), got {early_sma}"
    );
    let early_dock = part_points(&early, "chase");
    assert!(
        early_dock > -1.0,
        "the first push off the coil is not a chase ({early_dock})"
    );

    let thin_dock = part_points(&thin, "chase");
    let chase_dock = part_points(&chase, "chase");
    assert!(thin_dock <= -15.0, "thin runaway dock {thin_dock}");
    assert!(chase_dock <= -20.0, "parabolic dock {chase_dock}");
    assert!(
        coil.strike >= thin.strike + 10 && coil.strike >= chase.strike + 10,
        "coil {} thin {} ({thin_dock}) parabolic {} ({chase_dock})",
        coil.strike,
        thin.strike,
        chase.strike
    );
    assert!(
        early.strike >= thin.strike + 10 && early.strike >= chase.strike + 10,
        "early {} thin {} parabolic {}",
        early.strike,
        thin.strike,
        chase.strike
    );
}

fn part_points(eval: &Evaluation, id: &str) -> f64 {
    eval.strike_parts
        .iter()
        .find(|part| part.id == id)
        .map(|part| part.points)
        .unwrap_or(0.0)
}

#[test]
fn strike_stays_in_range_and_prefers_a_coiled_leader_over_chop() {
    let mut leader = Path::new(18.0);
    leader
        .add(220, 0.0022, 0.045, 0.7, 1_500_000.0, None)
        .add(20, 0.0002, 0.02, 0.8, 600_000.0, None);
    let mut chop = Path::new(18.0);
    for block in 0..12 {
        let drift = if block % 2 == 0 { 0.004 } else { -0.004 };
        chop.add(10, drift, 0.015, 0.5, 200_000.0, None);
    }
    let leader_eval = run(leader.done(), 96);
    let chop_eval = run(chop.done(), 12);
    assert!(leader_eval.strike <= 100 && chop_eval.strike <= 100);
    assert!(
        leader_eval.strike + 10 <= 100 || leader_eval.strike > chop_eval.strike,
        "leader {} chop {}",
        leader_eval.strike,
        chop_eval.strike
    );
    assert!(
        leader_eval.strike >= chop_eval.strike + 15,
        "leader {} chop {}",
        leader_eval.strike,
        chop_eval.strike
    );
}

#[test]
fn short_history_does_not_panic() {
    let mut path = Path::new(8.0);
    path.add(5, 0.01, 0.03, 0.4, 10_000.0, None);
    let eval = run(path.done(), 50);
    assert!(eval.strike <= 100);
    assert!(!has(&eval, "MM"));
    assert!(!has(&eval, "KQ"));
}

#[test]
fn partial_params_keep_the_documented_defaults() {
    let parsed: BadgeParams = serde_json::from_str(r#"{"min_adr":0.05}"#).unwrap();
    assert!((parsed.min_adr - 0.05).abs() < 1e-12);
    assert!((parsed.kq_prior_move - 0.30).abs() < 1e-12);
    assert_eq!(parsed.rs_hot, 80);
    let empty: BadgeParams = serde_json::from_str("{}").unwrap();
    assert!((empty.min_adr - BadgeParams::default().min_adr).abs() < 1e-12);
    assert!((empty.sbw_max_above - 0.06).abs() < 1e-12);
    assert!((empty.chase_max_extension - 0.05).abs() < 1e-12);
    assert_eq!(empty.chase_fresh_bars, 2);
}

#[test]
fn rulebook_names_every_badge_and_the_strike_dial() {
    let docs = rulebook(&BadgeParams::default());
    let ids: Vec<_> = docs.iter().map(|doc| doc.id.as_str()).collect();
    for id in [
        "KQ", "MM", "ON", "DB", "SB4", "SBW", "SB9", "TML", "97C", "52W", "ER", "2A", "RS",
        "STRIKE",
    ] {
        assert!(ids.contains(&id), "missing {id}");
    }
    assert!(docs.iter().any(|doc| doc.approximation));
    assert!(docs
        .iter()
        .all(|doc| !doc.body.to_lowercase().contains("market order")));
}

#[test]
fn slow_grind_can_surf_the_weekly_average() {
    let mut path = Path::new(40.0);
    path.add(100, 0.00035, 0.012, 0.72, 2_000_000.0, None);
    let eval = run(path.done(), 70);
    assert!(has(&eval, "SBW"), "{}", detail(&eval, "SBW"));
}

#[test]
fn rising_ten_day_average_can_earn_tml() {
    let mut path = Path::new(25.0);
    path.add(40, 0.0012, 0.028, 0.78, 1_000_000.0, None);
    let eval = run(path.done(), 70);
    assert!(has(&eval, "TML"), "{}", detail(&eval, "TML"));
}
