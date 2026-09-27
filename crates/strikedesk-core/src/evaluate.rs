use crate::indicators::mean;
use crate::model::{Badge, BadgeId, BadgeParams, Bar, Check, StrikePart, StrikeWeights, Tone};
use crate::prepare::{series_ago, series_last, Prepared};

#[derive(Debug, Clone, PartialEq)]
pub struct Metrics {
    pub sma10: Option<f64>,
    pub sma20: Option<f64>,
    pub sma50: Option<f64>,
    pub sma150: Option<f64>,
    pub sma200: Option<f64>,
    pub ema10: Option<f64>,
    pub ema20: Option<f64>,
    pub dist_ema10: Option<f64>,
    pub dist_year_high: Option<f64>,
    pub prior_move: Option<f64>,
    pub stage_advance: Option<f64>,
    pub adr_pct: Option<f64>,
    pub avg_dollar_volume: Option<f64>,
    pub month_pct: Option<f64>,
    /// High of the shelf the active breakout cleared.
    pub pivot: Option<f64>,
    /// Sessions since that breakout bar. Zero means the breakout is the last bar.
    pub bars_since_breakout: Option<usize>,
    /// Close / pivot − 1. Positive means the close is above the shelf.
    pub pivot_extension: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Evaluation {
    pub date: String,
    pub last: f64,
    pub change_pct: f64,
    pub month_pct: Option<f64>,
    pub adr_pct: Option<f64>,
    pub avg_dollar_volume: Option<f64>,
    pub rs: u8,
    pub strike: u8,
    pub strike_tone: Tone,
    pub strike_parts: Vec<StrikePart>,
    pub badges: Vec<Badge>,
    pub checks: Vec<Check>,
    pub metrics: Metrics,
}

pub fn evaluate(prepared: &Prepared, rs: u8, params: &BadgeParams) -> Evaluation {
    let rs = rs.clamp(1, 99);
    let checks = all_checks(prepared, rs, params);
    let mut badges: Vec<Badge> = checks
        .iter()
        .filter(|check| check.passed || check.id == BadgeId::Rs)
        .map(Badge::from)
        .collect();
    badges.sort_by_key(|badge| badge.order);
    let (strike, strike_parts) = strike_score(prepared, rs, params);
    let strike_tone = strike_tone(strike);
    let metrics = metrics_of(prepared, params);
    Evaluation {
        date: prepared.date.clone(),
        last: prepared.last,
        change_pct: prepared.change_pct,
        month_pct: metrics.month_pct,
        adr_pct: metrics.adr_pct,
        avg_dollar_volume: metrics.avg_dollar_volume,
        rs,
        strike,
        strike_tone,
        strike_parts,
        badges,
        checks,
        metrics,
    }
}

pub fn strike_tone(score: u8) -> Tone {
    if score >= 70 {
        Tone::Green
    } else if score >= 50 {
        Tone::Gold
    } else {
        Tone::Red
    }
}

fn all_checks(prepared: &Prepared, rs: u8, params: &BadgeParams) -> Vec<Check> {
    vec![
        check_kq(prepared, params),
        check_mm(prepared, params),
        check_on(prepared, params),
        check_db(prepared, params),
        check_sb4(prepared, params),
        check_sbw(prepared, params),
        check_sb9(prepared, params),
        check_tml(prepared, params),
        check_97c(prepared, params),
        check_52w(prepared, params),
        check_er(prepared, params),
        check_rs(rs, params),
        check_stage(prepared, params, BadgeId::Stage2A),
        check_stage(prepared, params, BadgeId::Stage2B),
        check_stage(prepared, params, BadgeId::Stage2C),
    ]
}

fn check_kq(prepared: &Prepared, params: &BadgeParams) -> Check {
    let close = prepared.last;
    let sma10 = series_last(&prepared.sma10);
    let sma20 = series_last(&prepared.sma20);
    let sma50 = series_last(&prepared.sma50);
    let ema10 = series_last(&prepared.ema10);
    let ema20 = series_last(&prepared.ema20);
    let adr = adr(&prepared.bars, 20);
    let dv = avg_dollar_volume(&prepared.bars, 20);
    let prior = prior_move(
        &prepared.bars,
        params.kq_prior_window,
        params.kq_prior_exclude_recent,
    );
    let mut fails = Vec::new();
    match (sma50, close > sma50.unwrap_or(f64::MAX)) {
        (Some(_sma), true) => {}
        (Some(sma), false) => fails.push(format!("close is below SMA(50) {sma:.2}")),
        (None, _) => fails.push("needs SMA(50)".into()),
    }
    match (sma10, sma20) {
        (Some(a), Some(b)) if a > b => {}
        (Some(_), Some(_)) => fails.push("SMA(10) is not above SMA(20)".into()),
        _ => fails.push("needs SMA(10) and SMA(20)".into()),
    }
    match adr {
        Some(v) if v >= params.min_adr => {}
        Some(v) => fails.push(format!(
            "ADR(20) {} is below {}",
            fmt_pct(v),
            fmt_pct(params.min_adr)
        )),
        None => fails.push("needs 20 sessions for ADR".into()),
    }
    match dv {
        Some(v) if v >= params.min_dollar_volume => {}
        Some(v) => fails.push(format!(
            "20-day dollar volume {} is below {}",
            fmt_money(v),
            fmt_money(params.min_dollar_volume)
        )),
        None => fails.push("needs 20 sessions of volume".into()),
    }
    match prior {
        Some(v) if v >= params.kq_prior_move => {}
        Some(v) => fails.push(format!(
            "prior move {} is below {}",
            fmt_pct(v),
            fmt_pct(params.kq_prior_move)
        )),
        None => fails.push(format!(
            "needs a {}-session prior-move window",
            params.kq_prior_window
        )),
    }
    let coiled = coil(close, ema10, ema20, params.kq_coil_pct);
    if !coiled {
        fails.push(format!(
            "not coiled within {} of EMA(10) or EMA(20) while closing at or above EMA(20)",
            fmt_pct(params.kq_coil_pct)
        ));
    }
    let passed = fails.is_empty();
    let detail = if passed {
        format!(
            "SMA(10) > SMA(20), close above SMA(50), ADR {}, prior move {}, coiled within {} of a short EMA, dollar volume {}.",
            fmt_pct(adr.unwrap_or(0.0)),
            fmt_pct(prior.unwrap_or(0.0)),
            fmt_pct(params.kq_coil_pct),
            fmt_money(dv.unwrap_or(0.0))
        )
    } else {
        fails.join(" ")
    };
    Check::new(BadgeId::Kq, "KQ", passed, detail)
}

fn check_mm(prepared: &Prepared, params: &BadgeParams) -> Check {
    let close = prepared.last;
    let sma50 = series_last(&prepared.sma50);
    let sma150 = series_last(&prepared.sma150);
    let sma200 = series_last(&prepared.sma200);
    let sma200_then = series_ago(&prepared.sma200, params.mm_slope_bars);
    let high = rolling_high(&prepared.bars, 252);
    let low = rolling_low(&prepared.bars, 252);
    let mut fails = Vec::new();
    let stack = match (sma50, sma150, sma200, sma200_then) {
        (Some(s50), Some(s150), Some(s200), Some(s200_then)) => {
            if close <= s50 {
                fails.push("close is not above SMA(50)".into());
            }
            if close <= s150 || close <= s200 {
                fails.push("close is not above SMA(150) and SMA(200)".into());
            }
            if s50 <= s150 || s50 <= s200 {
                fails.push("SMA(50) is not above SMA(150) and SMA(200)".into());
            }
            if s150 <= s200 {
                fails.push("SMA(150) is not above SMA(200)".into());
            }
            if s200 <= s200_then {
                fails.push(format!(
                    "SMA(200) is not higher than {} sessions ago",
                    params.mm_slope_bars
                ));
            }
            true
        }
        _ => {
            fails.push("needs SMA(200) and a full month of slope".into());
            false
        }
    };
    match (low, high) {
        (Some(low), Some(high)) if low > 0.0 && high > 0.0 => {
            if close < low * params.mm_above_low {
                fails.push(format!(
                    "close is less than {} above the 252-session low",
                    fmt_pct(params.mm_above_low - 1.0)
                ));
            }
            if close < high * params.mm_within_high {
                fails.push(format!(
                    "close is more than {} below the 252-session high",
                    fmt_pct(1.0 - params.mm_within_high)
                ));
            }
        }
        _ => fails.push("needs a 252-session high and low".into()),
    }
    let _ = stack;
    let passed = fails.is_empty();
    let detail = if passed {
        format!(
            "Minervini-style stack: close > SMA(50) > SMA(150) > rising SMA(200), at least {} above the 252-session low and within {} of the high.",
            fmt_pct(params.mm_above_low - 1.0),
            fmt_pct(1.0 - params.mm_within_high)
        )
    } else {
        fails.join(" ")
    };
    Check::new(BadgeId::Mm, "MM", passed, detail)
}

fn check_on(prepared: &Prepared, params: &BadgeParams) -> Check {
    let bars = &prepared.bars;
    let n = params.on_lookback;
    if bars.len() < n {
        return Check::new(BadgeId::On, "ON", false, format!("needs {n} sessions"));
    }
    let high = rolling_high(bars, n).unwrap_or(0.0);
    let pos = close_position(bars.last().unwrap()).unwrap_or(0.0);
    let near = high > 0.0 && prepared.last >= high * params.on_near_high;
    let strong = pos >= params.on_close_pos;
    let passed = near && strong;
    let detail = if passed {
        format!(
            "Close is {} of the {n}-session high and finished {} up the day's range.",
            fmt_pct(prepared.last / high),
            fmt_pct(pos)
        )
    } else {
        format!(
            "Need close ≥ {} of the {n}-session high and a close in the top half of the day.",
            fmt_pct(params.on_near_high)
        )
    };
    Check::new(BadgeId::On, "ON", passed, detail)
}

fn check_db(prepared: &Prepared, params: &BadgeParams) -> Check {
    let n = params.db_box_bars;
    let range = box_range(&prepared.bars, n);
    let pos = box_close_pos(&prepared.bars, n);
    let dry = volume_dryup(&prepared.bars, 5, 15);
    let sma20 = series_last(&prepared.sma20);
    let mut fails = Vec::new();
    match range {
        Some(v) if v <= params.db_max_range => {}
        Some(v) => fails.push(format!(
            "{}-session box {} is wider than {}",
            n,
            fmt_pct(v),
            fmt_pct(params.db_max_range)
        )),
        None => fails.push(format!("needs {n} sessions for the box")),
    }
    match pos {
        Some(v) if v >= params.db_close_pos => {}
        Some(v) => fails.push(format!(
            "close sits {} up the box, below {}",
            fmt_pct(v),
            fmt_pct(params.db_close_pos)
        )),
        None => {}
    }
    match dry {
        Some(v) if v <= params.db_volume_ratio => {}
        Some(v) => fails.push(format!(
            "volume dry-up {v:.2} is above the {:.2} cap",
            params.db_volume_ratio
        )),
        None => fails.push("needs 20 sessions to measure volume dry-up".into()),
    }
    match sma20 {
        Some(sma) if prepared.last > sma => {}
        Some(_) => fails.push("close is not above SMA(20)".into()),
        None => fails.push("needs SMA(20)".into()),
    }
    let passed = fails.is_empty();
    let detail = if passed {
        format!(
            "Darvas-style box: last {n} sessions range {}, close in the top of the box, volume dry-up {:.2} ≤ {:.2}, close above SMA(20).",
            fmt_pct(range.unwrap_or(0.0)),
            dry.unwrap_or(0.0),
            params.db_volume_ratio
        )
    } else {
        fails.join(" ")
    };
    Check::new(BadgeId::Db, "DB", passed, detail)
}

fn check_sb4(prepared: &Prepared, params: &BadgeParams) -> Check {
    let look = params.sb4_lookback;
    let ret = window_return(&prepared.bars, look);
    let sma10 = series_last(&prepared.sma10);
    let expanded = expansion(
        &prepared.bars,
        look,
        20,
        params.sb4_vol_mult,
        params.sb4_range_mult,
    );
    let mut fails = Vec::new();
    match ret {
        Some(v) if v >= params.sb4_min_return => {}
        Some(v) => fails.push(format!(
            "{look}-session return {} is below {}",
            fmt_pct(v),
            fmt_pct(params.sb4_min_return)
        )),
        None => fails.push(format!("needs {look} sessions of history")),
    }
    match sma10 {
        Some(sma) if prepared.last > sma => {}
        Some(_) => fails.push("close is not above SMA(10)".into()),
        None => fails.push("needs SMA(10)".into()),
    }
    if !expanded {
        fails.push(format!(
            "none of the last {look} sessions expanded (volume ≥ {:.1}× or range ≥ {:.1}× ADR)",
            params.sb4_vol_mult, params.sb4_range_mult
        ));
    }
    let passed = fails.is_empty();
    let detail = if passed {
        format!(
            "Four-session thrust {} with close above SMA(10) and a range or volume expansion.",
            fmt_pct(ret.unwrap_or(0.0))
        )
    } else {
        fails.join(" ")
    };
    Check::new(BadgeId::Sb4, "SB4", passed, detail)
}

fn check_sbw(prepared: &Prepared, params: &BadgeParams) -> Check {
    let dist = weekly_distance(&prepared.bars);
    let rising = weekly_sma_rising(&prepared.bars);
    let dd = weekly_drawdown(&prepared.bars, 26);
    let mut fails = Vec::new();
    match dist {
        Some(v) if (0.0..=params.sbw_max_above).contains(&v) => {}
        Some(v) if v < 0.0 => fails.push("weekly close is under the 10-week average".into()),
        Some(v) => fails.push(format!(
            "price is {} above the 10-week average, past {}",
            fmt_pct(v),
            fmt_pct(params.sbw_max_above)
        )),
        None => fails.push("needs 10 trading weeks (50 sessions)".into()),
    }
    if dist.is_some() && !rising {
        fails.push("10-week average is not higher than 4 weeks ago".into());
    }
    match dd {
        Some(v) if v <= params.sbw_max_drawdown => {}
        Some(v) => fails.push(format!(
            "drawdown from the 26-week high is {}, above {}",
            fmt_pct(v),
            fmt_pct(params.sbw_max_drawdown)
        )),
        None => fails.push("needs weekly highs".into()),
    }
    let passed = fails.is_empty();
    let detail = if passed {
        format!(
            "Surfing the 10-week average ({} above it, rising, drawdown {}). Weeks are blocks of 5 sessions aligned to the last bar.",
            fmt_pct(dist.unwrap_or(0.0)),
            fmt_pct(dd.unwrap_or(0.0))
        )
    } else {
        fails.join(" ")
    };
    Check::new(BadgeId::Sbw, "SBW", passed, detail)
}

fn check_sb9(prepared: &Prepared, params: &BadgeParams) -> Check {
    let flag = params.sb9_bars;
    let range = box_range(&prepared.bars, flag);
    let prior = flag_prior(&prepared.bars, flag, params.sb9_prior_bars);
    let held = flag_held(&prepared.bars, flag);
    let sma20 = series_last(&prepared.sma20);
    let mut fails = Vec::new();
    match range {
        Some(v) if v <= params.sb9_max_range => {}
        Some(v) => fails.push(format!(
            "{flag}-session flag {} is wider than {}",
            fmt_pct(v),
            fmt_pct(params.sb9_max_range)
        )),
        None => fails.push(format!("needs {flag} sessions")),
    }
    match prior {
        Some(v) if v >= params.sb9_prior_move => {}
        Some(v) => fails.push(format!(
            "move into the flag {} is below {}",
            fmt_pct(v),
            fmt_pct(params.sb9_prior_move)
        )),
        None => fails.push("needs a thrust before the flag".into()),
    }
    if prior.is_some() && !held {
        fails.push("flag gave back more than 2% from its start".into());
    }
    match sma20 {
        Some(sma) if prepared.last > sma => {}
        Some(_) => fails.push("close is not above SMA(20)".into()),
        None => fails.push("needs SMA(20)".into()),
    }
    let passed = fails.is_empty();
    let detail = if passed {
        format!(
            "Nine-session flag {} after a {} thrust, still above SMA(20).",
            fmt_pct(range.unwrap_or(0.0)),
            fmt_pct(prior.unwrap_or(0.0))
        )
    } else {
        fails.join(" ")
    };
    Check::new(BadgeId::Sb9, "SB9", passed, detail)
}

fn check_tml(prepared: &Prepared, params: &BadgeParams) -> Check {
    let ema10 = series_last(&prepared.ema10);
    let ema10_then = series_ago(&prepared.ema10, params.tml_rise_bars);
    let ema20 = series_last(&prepared.ema20);
    let low3 = rolling_low(&prepared.bars, 3);
    let mut fails = Vec::new();
    let (above, tagged) = match (ema10, ema10_then, ema20, low3) {
        (Some(e10), Some(prev), Some(e20), Some(low)) if e10 > 0.0 => {
            if e10 <= prev {
                fails.push(format!(
                    "EMA(10) is not higher than {} sessions ago",
                    params.tml_rise_bars
                ));
            }
            if e10 <= e20 {
                fails.push("EMA(10) is not above EMA(20)".into());
            }
            let dist = prepared.last / e10 - 1.0;
            if dist < 0.0 {
                fails.push("close is under EMA(10)".into());
            } else if dist > params.tml_max_above_ema {
                fails.push(format!(
                    "close is {} above EMA(10), past {}",
                    fmt_pct(dist),
                    fmt_pct(params.tml_max_above_ema)
                ));
            }
            if low > e10 * 1.01 {
                fails.push("last 3 sessions did not tag within 1% of EMA(10)".into());
            }
            (
                dist >= 0.0 && dist <= params.tml_max_above_ema,
                low <= e10 * 1.01,
            )
        }
        _ => {
            fails.push("needs EMA(10), EMA(20), and 3 sessions".into());
            (false, false)
        }
    };
    let _ = (above, tagged);
    let passed = fails.is_empty();
    let detail = if passed {
        "Ten-day MA lift: rising EMA(10) above EMA(20), close within 3% above it, and a tag of the average in the last 3 sessions.".into()
    } else {
        fails.join(" ")
    };
    Check::new(BadgeId::Tml, "TML", passed, detail)
}

fn check_97c(prepared: &Prepared, params: &BadgeParams) -> Check {
    let Some(bar) = prepared.bars.last() else {
        return Check::new(BadgeId::C97, "97C", false, "no bar");
    };
    let pos = close_position(bar).unwrap_or(0.0);
    let passed = pos >= params.c97_close_pos;
    let detail = if passed {
        format!(
            "Close finished {} up today's range (need ≥ {}).",
            fmt_pct(pos),
            fmt_pct(params.c97_close_pos)
        )
    } else {
        format!(
            "Close position {} is below {}.",
            fmt_pct(pos),
            fmt_pct(params.c97_close_pos)
        )
    };
    Check::new(BadgeId::C97, "97C", passed, detail)
}

fn check_52w(prepared: &Prepared, params: &BadgeParams) -> Check {
    if prepared.bars.len() < params.week52_min_bars {
        return Check::new(
            BadgeId::Week52,
            "52W",
            false,
            format!(
                "needs {} sessions before a 52-week high counts",
                params.week52_min_bars
            ),
        );
    }
    let ago = sessions_since_high(&prepared.bars, 252).unwrap_or(usize::MAX);
    let passed = ago < params.week52_lookback;
    let detail = if passed {
        format!(
            "The highest high of the last {} sessions printed {ago} sessions ago (inside {}).",
            prepared.bars.len().min(252),
            params.week52_lookback
        )
    } else {
        format!(
            "The window high is {ago} sessions old, outside the last {}.",
            params.week52_lookback
        )
    };
    Check::new(BadgeId::Week52, "52W", passed, detail)
}

fn check_er(prepared: &Prepared, params: &BadgeParams) -> Check {
    match earnings_reaction(prepared, params) {
        Some(sessions) => Check::new(
            BadgeId::Er,
            format!("ER-{sessions}"),
            true,
            format!(
                "Positive reaction {sessions} sessions ago: gap ≥ {} or range ≥ {}× ADR, volume ≥ {}×, close above the prior close.",
                fmt_pct(params.er_gap),
                params.er_range_mult,
                params.er_volume_mult
            ),
        ),
        None => Check::new(
            BadgeId::Er,
            "ER",
            false,
            format!(
                "No positive earnings-style reaction in the last {} sessions.",
                params.er_lookback
            ),
        ),
    }
}

fn check_rs(rs: u8, params: &BadgeParams) -> Check {
    let passed = rs >= params.rs_hot;
    let detail = format!(
        "Cross-sectional RS {rs}. Hot threshold is {}. Weights are 21/63/126/252 sessions at {:.0}/{:.0}/{:.0}/{:.0}%, excess versus the benchmark when one is loaded. One-name universes stay at 50.",
        params.rs_hot,
        params.rs.w21 * 100.0,
        params.rs.w63 * 100.0,
        params.rs.w126 * 100.0,
        params.rs.w252 * 100.0
    );
    Check::new(BadgeId::Rs, rs.to_string(), passed, detail)
}

fn check_stage(prepared: &Prepared, params: &BadgeParams, id: BadgeId) -> Check {
    let close = prepared.last;
    let sma150 = series_last(&prepared.sma150);
    let sma150_then = series_ago(&prepared.sma150, params.mm_slope_bars);
    let (gate, advance) = match (sma150, sma150_then) {
        (Some(now), Some(then)) if now > 0.0 => {
            (close > now && now > then, Some(close / now - 1.0))
        }
        _ => (false, None),
    };
    let phase = advance.and_then(|adv| {
        if !gate {
            None
        } else if adv < params.stage_early {
            Some(BadgeId::Stage2A)
        } else if adv < params.stage_mid {
            Some(BadgeId::Stage2B)
        } else {
            Some(BadgeId::Stage2C)
        }
    });
    let passed = phase == Some(id);
    let detail = match (gate, advance) {
        (false, _) => "Stage 2 gate is closed: close must be above a rising SMA(150).".into(),
        (true, Some(adv)) if passed => format!(
            "Stage 2 advance vs SMA(150) is {}. 2A < {}, 2B < {}, else 2C.",
            fmt_pct(adv),
            fmt_pct(params.stage_early),
            fmt_pct(params.stage_mid)
        ),
        (true, Some(adv)) => format!(
            "Advance vs SMA(150) is {}, so this is not {}.",
            fmt_pct(adv),
            id.code()
        ),
        _ => "Stage 2 is not measurable.".into(),
    };
    Check::new(id, id.code(), passed, detail)
}

fn strike_score(prepared: &Prepared, rs: u8, params: &BadgeParams) -> (u8, Vec<StrikePart>) {
    let w = params.strike;
    let ma = part_ma(prepared, &w);
    let rs_pts = (rs as f64 / 99.0 * w.rs).clamp(0.0, w.rs);
    let location = part_location(prepared, &w);
    let tight = part_tight(prepared, &w);
    let close_vol = part_close_volume(prepared, &w);
    let adr_pts = part_adr(prepared, &w);
    let liq = part_liquidity(prepared, &w);
    let dock = chase_penalty(prepared, params).0;
    let parts = vec![
        part("ma", "Moving-average stack", ma, w.ma),
        part("rs", "Relative strength", rs_pts, w.rs),
        part("location", "Location vs highs", location, w.location),
        part("tight", "Tightness after a move", tight, w.tight),
        part("close", "Close and volume", close_vol, w.close_volume),
        part("adr", "ADR band", adr_pts, w.adr),
        part("liq", "Liquidity", liq, w.liquidity),
        StrikePart {
            id: "chase".to_string(),
            label: "Late chase".to_string(),
            points: if dock > 0.0 { -dock } else { 0.0 },
            max: params.chase_dock_cap,
        },
    ];
    let sum: f64 = parts.iter().map(|p| p.points).sum();
    let score = sum.round().clamp(0.0, 100.0) as u8;
    (score, parts)
}

fn part(id: &str, label: &str, points: f64, max: f64) -> StrikePart {
    StrikePart {
        id: id.to_string(),
        label: label.to_string(),
        points: (points.max(0.0)).min(max),
        max,
    }
}

fn part_ma(prepared: &Prepared, w: &StrikeWeights) -> f64 {
    let close = prepared.last;
    let s50 = series_last(&prepared.sma50);
    let s150 = series_last(&prepared.sma150);
    let s200 = series_last(&prepared.sma200);
    let s200_then = series_ago(&prepared.sma200, 21);
    let mut points = 0.0;
    let third = w.ma / 20.0;
    if s50.is_some_and(|sma| close > sma) {
        points += 8.0 * third;
    }
    if s50.zip(s150).is_some_and(|(a, b)| a > b) {
        points += 6.0 * third;
    }
    if s150
        .zip(s200)
        .zip(s200_then)
        .is_some_and(|((a, b), then)| a > b && b > then)
    {
        points += 6.0 * third;
    }
    points
}

fn part_location(prepared: &Prepared, w: &StrikeWeights) -> f64 {
    let Some(high) = rolling_high(&prepared.bars, 63) else {
        return 0.0;
    };
    if high <= 0.0 {
        return 0.0;
    }
    let drawdown = 1.0 - prepared.last / high;
    let mut points = if drawdown <= w.location_tight {
        w.location
    } else if drawdown <= w.location_mid {
        w.location * 10.0 / 15.0
    } else if drawdown <= w.location_wide {
        w.location * 5.0 / 15.0
    } else {
        0.0
    };
    if let Some(ema10) = series_last(&prepared.ema10) {
        if ema10 > 0.0 && prepared.last > ema10 * (1.0 + w.extension_penalty_above) {
            points = (points - 8.0).max(0.0);
        }
    }
    points.min(w.location)
}

fn part_tight(prepared: &Prepared, w: &StrikeWeights) -> f64 {
    let Some(range) = box_range(&prepared.bars, 10) else {
        return 0.0;
    };
    let mut points = if range <= 0.08 {
        w.tight
    } else if range <= 0.12 {
        w.tight * 10.0 / 15.0
    } else if range <= 0.18 {
        w.tight * 5.0 / 15.0
    } else {
        0.0
    };
    let prior = window_return(&prepared.bars, 63).unwrap_or(0.0);
    if prior < 0.15 {
        points = points.min(w.tight * 8.0 / 15.0);
    }
    points
}

fn part_close_volume(prepared: &Prepared, w: &StrikeWeights) -> f64 {
    let pos = prepared.bars.last().and_then(close_position).unwrap_or(0.0);
    let rel = rel_volume(&prepared.bars, 20).unwrap_or(0.0);
    let mut points = 0.0;
    if pos >= 0.97 {
        points += w.close_volume * 0.5;
    } else if pos >= 0.70 {
        points += w.close_volume * 0.3;
    }
    if rel >= 1.2 {
        points += w.close_volume * 0.5;
    } else if rel >= 0.8 {
        points += w.close_volume * 0.2;
    }
    points.min(w.close_volume)
}

fn part_adr(prepared: &Prepared, w: &StrikeWeights) -> f64 {
    let Some(adr) = adr(&prepared.bars, 20) else {
        return 0.0;
    };
    let scale = if (0.035..=0.08).contains(&adr) {
        1.0
    } else if (0.025..0.035).contains(&adr) || (0.08..=0.12).contains(&adr) {
        0.6
    } else if (0.015..0.025).contains(&adr) || (0.12..=0.18).contains(&adr) {
        0.3
    } else {
        0.0
    };
    w.adr * scale
}

fn part_liquidity(prepared: &Prepared, w: &StrikeWeights) -> f64 {
    let Some(dv) = avg_dollar_volume(&prepared.bars, 20) else {
        return 0.0;
    };
    let scale = if dv >= 20_000_000.0 {
        1.0
    } else if dv >= 5_000_000.0 {
        0.7
    } else if dv >= 1_000_000.0 {
        0.4
    } else {
        0.0
    };
    w.liquidity * scale
}

fn metrics_of(prepared: &Prepared, params: &BadgeParams) -> Metrics {
    let ema10 = series_last(&prepared.ema10);
    let high = rolling_high(&prepared.bars, 252);
    let sma150 = series_last(&prepared.sma150);
    let pivot = latest_pivot(&prepared.bars, params);
    Metrics {
        sma10: series_last(&prepared.sma10),
        sma20: series_last(&prepared.sma20),
        sma50: series_last(&prepared.sma50),
        sma150,
        sma200: series_last(&prepared.sma200),
        ema10,
        ema20: series_last(&prepared.ema20),
        dist_ema10: ema10
            .filter(|ema| *ema > 0.0)
            .map(|ema| prepared.last / ema - 1.0),
        dist_year_high: high
            .filter(|high| *high > 0.0)
            .map(|high| prepared.last / high - 1.0),
        prior_move: prior_move(
            &prepared.bars,
            params.kq_prior_window,
            params.kq_prior_exclude_recent,
        ),
        stage_advance: sma150
            .filter(|sma| *sma > 0.0)
            .map(|sma| prepared.last / sma - 1.0),
        adr_pct: adr(&prepared.bars, 20),
        avg_dollar_volume: avg_dollar_volume(&prepared.bars, 20),
        month_pct: window_return(&prepared.bars, 21),
        pivot: pivot.as_ref().map(|breakout| breakout.pivot),
        bars_since_breakout: pivot.as_ref().map(|breakout| breakout.bars_since),
        pivot_extension: pivot.as_ref().map(|breakout| breakout.extension),
    }
}

struct PivotBreak {
    pivot: f64,
    bars_since: usize,
    extension: f64,
}

/// Most recent close that cleared a sideways shelf inside the lookback.
///
/// A thrust that keeps making highs does not mint a new pivot: the prior
/// window has to be a base (limited range and limited drift). The buy is that
/// break, or a close that is still within the extension band. A tight pause
/// that never clears a new shelf leaves the earlier pivot in force.
fn latest_pivot(bars: &[Bar], params: &BadgeParams) -> Option<PivotBreak> {
    let n = bars.len();
    let base = params.pivot_base_bars;
    if base < 2 || n <= base {
        return None;
    }
    let earliest = n.saturating_sub(params.pivot_lookback.max(1)).max(base);
    for i in (earliest..n).rev() {
        let window = &bars[i - base..i];
        let mut shelf_high = f64::MIN;
        let mut shelf_low = f64::MAX;
        for bar in window {
            shelf_high = shelf_high.max(bar.high);
            shelf_low = shelf_low.min(bar.low);
        }
        if !(shelf_high.is_finite() && shelf_high > 0.0 && shelf_low.is_finite()) {
            continue;
        }
        let width = (shelf_high - shelf_low) / shelf_high;
        let first = window[0].close;
        let last = window[window.len() - 1].close;
        if first <= 0.0 || last <= 0.0 {
            continue;
        }
        let drift = (last / first - 1.0).abs();
        if width <= params.pivot_base_max_range
            && drift <= params.pivot_base_max_drift
            && bars[i].close > shelf_high
        {
            let close = bars[n - 1].close;
            return Some(PivotBreak {
                pivot: shelf_high,
                bars_since: n - 1 - i,
                extension: close / shelf_high - 1.0,
            });
        }
    }
    None
}

fn chase_penalty(prepared: &Prepared, params: &BadgeParams) -> (f64, Option<PivotBreak>) {
    let pivot = latest_pivot(&prepared.bars, params);
    let close = prepared.last;
    let atr = average_range(&prepared.bars, params.chase_atr_bars);
    let ema10 = series_last(&prepared.ema10);
    let atr_ext = match (ema10, atr) {
        (Some(ema), Some(range)) if range > 0.0 && close > ema => (close - ema) / range,
        _ => 0.0,
    };
    let near_fresh = pivot.as_ref().is_some_and(|breakout| {
        breakout.bars_since <= params.chase_fresh_bars
            && breakout.extension <= params.chase_max_extension + params.chase_fresh_pad
    });
    let atr_dock = if near_fresh {
        0.0
    } else {
        stepped(
            (atr_ext - params.chase_atr_limit).max(0.0),
            1.0,
            params.chase_atr_points,
            params.chase_atr_cap,
        )
    };
    let (dist_dock, age_dock, ema_dock) = if let Some(breakout) = &pivot {
        let dist_dock = stepped(
            (breakout.extension - params.chase_max_extension).max(0.0),
            params.chase_dist_step,
            params.chase_dist_points,
            params.chase_dist_cap,
        );
        let age_dock = if breakout.extension > params.chase_max_extension {
            let extra = breakout.bars_since.saturating_sub(params.chase_fresh_bars) as f64;
            (extra * params.chase_age_points).clamp(0.0, params.chase_age_cap.max(0.0))
        } else {
            0.0
        };
        (dist_dock, age_dock, 0.0)
    } else {
        let ema_ext = ema10
            .filter(|ema| *ema > 0.0)
            .map(|ema| close / ema - 1.0)
            .unwrap_or(0.0);
        let ema_dock = stepped(
            (ema_ext - params.chase_max_extension).max(0.0),
            params.chase_ema_step,
            params.chase_ema_points,
            params.chase_ema_cap,
        );
        (0.0, 0.0, ema_dock)
    };
    let dock =
        (dist_dock + atr_dock + age_dock + ema_dock).clamp(0.0, params.chase_dock_cap.max(0.0));
    (dock, pivot)
}

fn stepped(over: f64, step: f64, points: f64, cap: f64) -> f64 {
    if step <= 0.0 || points <= 0.0 || over <= 0.0 {
        0.0
    } else {
        (over / step * points).clamp(0.0, cap.max(0.0))
    }
}

fn average_range(bars: &[Bar], n: usize) -> Option<f64> {
    if bars.len() < n || n == 0 {
        return None;
    }
    let mut sum = 0.0;
    for bar in &bars[bars.len() - n..] {
        let span = bar.high - bar.low;
        if !span.is_finite() || span < 0.0 {
            return None;
        }
        sum += span;
    }
    let value = sum / n as f64;
    (value > 0.0).then_some(value)
}

fn coil(close: f64, ema10: Option<f64>, ema20: Option<f64>, pct: f64) -> bool {
    let near =
        |ema: Option<f64>| ema.is_some_and(|ema| ema > 0.0 && (close / ema - 1.0).abs() <= pct);
    let above20 = ema20.is_some_and(|ema| close >= ema);
    (near(ema10) || near(ema20)) && above20
}

fn close_position(bar: &Bar) -> Option<f64> {
    let span = bar.high - bar.low;
    if span <= f64::EPSILON {
        return Some(if bar.close >= bar.high { 1.0 } else { 0.0 });
    }
    Some(((bar.close - bar.low) / span).clamp(0.0, 1.0))
}

fn window_return(bars: &[Bar], sessions: usize) -> Option<f64> {
    if bars.len() <= sessions {
        return None;
    }
    let past = bars[bars.len() - 1 - sessions].close;
    let last = bars[bars.len() - 1].close;
    if past <= 0.0 {
        None
    } else {
        Some(last / past - 1.0)
    }
}

fn adr(bars: &[Bar], n: usize) -> Option<f64> {
    if bars.len() < n || n == 0 {
        return None;
    }
    let slice = &bars[bars.len() - n..];
    let mut sum = 0.0;
    for bar in slice {
        if bar.close <= 0.0 {
            return None;
        }
        sum += (bar.high - bar.low) / bar.close;
    }
    Some(sum / n as f64)
}

fn avg_dollar_volume(bars: &[Bar], n: usize) -> Option<f64> {
    if bars.len() < n || n == 0 {
        return None;
    }
    mean(
        &bars[bars.len() - n..]
            .iter()
            .map(|bar| bar.close * bar.volume)
            .collect::<Vec<_>>(),
    )
}

fn rel_volume(bars: &[Bar], n: usize) -> Option<f64> {
    if bars.len() < n || n == 0 {
        return None;
    }
    let avg = mean(
        &bars[bars.len() - n..]
            .iter()
            .map(|bar| bar.volume)
            .collect::<Vec<_>>(),
    )?;
    if avg <= 0.0 {
        return None;
    }
    Some(bars[bars.len() - 1].volume / avg)
}

fn rolling_high(bars: &[Bar], n: usize) -> Option<f64> {
    if bars.is_empty() {
        return None;
    }
    let start = bars.len().saturating_sub(n);
    bars[start..]
        .iter()
        .map(|bar| bar.high)
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
}

fn rolling_low(bars: &[Bar], n: usize) -> Option<f64> {
    if bars.is_empty() {
        return None;
    }
    let start = bars.len().saturating_sub(n);
    bars[start..]
        .iter()
        .map(|bar| bar.low)
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
}

fn sessions_since_high(bars: &[Bar], n: usize) -> Option<usize> {
    if bars.is_empty() {
        return None;
    }
    let start = bars.len().saturating_sub(n);
    let slice = &bars[start..];
    let mut best_i = 0usize;
    let mut best = f64::MIN;
    for (i, bar) in slice.iter().enumerate() {
        if bar.high >= best {
            best = bar.high;
            best_i = i;
        }
    }
    Some(slice.len() - 1 - best_i)
}

fn prior_move(bars: &[Bar], window: usize, exclude_recent: usize) -> Option<f64> {
    let n = bars.len();
    if n <= exclude_recent + 5 {
        return None;
    }
    let end = n - exclude_recent;
    let start = end.saturating_sub(window);
    let low = bars[start..end]
        .iter()
        .map(|bar| bar.low)
        .fold(f64::MAX, f64::min);
    if !low.is_finite() || low <= 0.0 {
        return None;
    }
    Some(bars[n - 1].close / low - 1.0)
}

fn box_range(bars: &[Bar], n: usize) -> Option<f64> {
    if bars.len() < n || n == 0 {
        return None;
    }
    let slice = &bars[bars.len() - n..];
    let high = slice.iter().map(|bar| bar.high).fold(f64::MIN, f64::max);
    let low = slice.iter().map(|bar| bar.low).fold(f64::MAX, f64::min);
    let close = bars[bars.len() - 1].close;
    if close <= 0.0 {
        return None;
    }
    Some((high - low) / close)
}

fn box_close_pos(bars: &[Bar], n: usize) -> Option<f64> {
    if bars.len() < n || n == 0 {
        return None;
    }
    let slice = &bars[bars.len() - n..];
    let high = slice.iter().map(|bar| bar.high).fold(f64::MIN, f64::max);
    let low = slice.iter().map(|bar| bar.low).fold(f64::MAX, f64::min);
    let span = high - low;
    if span <= f64::EPSILON {
        return Some(1.0);
    }
    Some(((bars[bars.len() - 1].close - low) / span).clamp(0.0, 1.0))
}

fn volume_dryup(bars: &[Bar], recent: usize, prior: usize) -> Option<f64> {
    let need = recent + prior;
    if bars.len() < need {
        return None;
    }
    let n = bars.len();
    let recent_avg = mean(
        &bars[n - recent..]
            .iter()
            .map(|bar| bar.volume)
            .collect::<Vec<_>>(),
    )?;
    let prior_avg = mean(
        &bars[n - need..n - recent]
            .iter()
            .map(|bar| bar.volume)
            .collect::<Vec<_>>(),
    )?;
    if prior_avg <= 0.0 {
        return None;
    }
    Some(recent_avg / prior_avg)
}

fn expansion(bars: &[Bar], window: usize, hist: usize, vol_mult: f64, range_mult: f64) -> bool {
    if bars.len() < window + 5 || window == 0 {
        return false;
    }
    let n = bars.len();
    let hist_end = n - window;
    let hist_start = hist_end.saturating_sub(hist);
    if hist_end - hist_start < 5 {
        return false;
    }
    let hist_slice = &bars[hist_start..hist_end];
    let Some(avg_vol) = mean(&hist_slice.iter().map(|bar| bar.volume).collect::<Vec<_>>()) else {
        return false;
    };
    if avg_vol <= 0.0 {
        return false;
    }
    let Some(base_adr) = adr(hist_slice, hist_slice.len().min(20)) else {
        return false;
    };
    bars[n - window..].iter().any(|bar| {
        let wide = bar.close > 0.0
            && base_adr > 0.0
            && (bar.high - bar.low) / bar.close >= range_mult * base_adr;
        bar.volume >= vol_mult * avg_vol || wide
    })
}

fn flag_prior(bars: &[Bar], flag_bars: usize, prior_bars: usize) -> Option<f64> {
    let n = bars.len();
    if flag_bars == 0 || n < flag_bars + prior_bars {
        return None;
    }
    let flag_start = n - flag_bars;
    let past = flag_start.checked_sub(prior_bars)?;
    if bars[past].close <= 0.0 {
        return None;
    }
    Some(bars[flag_start].close / bars[past].close - 1.0)
}

fn flag_held(bars: &[Bar], flag_bars: usize) -> bool {
    let n = bars.len();
    if n < flag_bars || flag_bars == 0 {
        return false;
    }
    let start = bars[n - flag_bars].close;
    start > 0.0 && bars[n - 1].close >= start * 0.98
}

fn weekly_closes(bars: &[Bar]) -> Vec<f64> {
    let weeks = bars.len() / 5;
    if weeks == 0 {
        return Vec::new();
    }
    let start = bars.len() - weeks * 5;
    (0..weeks)
        .map(|week| bars[start + week * 5 + 4].close)
        .collect()
}

fn weekly_highs(bars: &[Bar]) -> Vec<f64> {
    let weeks = bars.len() / 5;
    if weeks == 0 {
        return Vec::new();
    }
    let start = bars.len() - weeks * 5;
    (0..weeks)
        .map(|week| {
            let from = start + week * 5;
            bars[from..from + 5]
                .iter()
                .map(|bar| bar.high)
                .fold(f64::MIN, f64::max)
        })
        .collect()
}

fn weekly_distance(bars: &[Bar]) -> Option<f64> {
    let closes = weekly_closes(bars);
    if closes.len() < 10 {
        return None;
    }
    let average = mean(&closes[closes.len() - 10..])?;
    if average <= 0.0 {
        return None;
    }
    Some(bars[bars.len() - 1].close / average - 1.0)
}

fn weekly_sma_rising(bars: &[Bar]) -> bool {
    let closes = weekly_closes(bars);
    if closes.len() < 14 {
        return false;
    }
    let now = mean(&closes[closes.len() - 10..]).unwrap_or(0.0);
    let prev = mean(&closes[closes.len() - 14..closes.len() - 4]).unwrap_or(0.0);
    now > prev && prev > 0.0
}

fn weekly_drawdown(bars: &[Bar], weeks: usize) -> Option<f64> {
    let highs = weekly_highs(bars);
    if highs.is_empty() {
        return None;
    }
    let start = highs.len().saturating_sub(weeks);
    let high = highs[start..].iter().copied().fold(f64::MIN, f64::max);
    let close = bars[bars.len() - 1].close;
    if high <= 0.0 {
        return None;
    }
    Some((high - close) / high)
}

fn earnings_reaction(prepared: &Prepared, params: &BadgeParams) -> Option<usize> {
    let bars = &prepared.bars;
    let n = bars.len();
    if n < 22 {
        return None;
    }
    let lookback = params.er_lookback.min(n.saturating_sub(21));
    let start = n - lookback;
    let mut found = None;
    for i in start..n {
        let prior = bars[i - 1].close;
        if prior <= 0.0 || bars[i].close <= 0.0 {
            continue;
        }
        let hist = &bars[i - 20..i];
        let Some(avg_vol) = mean(&hist.iter().map(|bar| bar.volume).collect::<Vec<_>>()) else {
            continue;
        };
        if avg_vol <= 0.0 {
            continue;
        }
        let Some(base_adr) = adr(hist, hist.len()) else {
            continue;
        };
        if base_adr <= 0.0 {
            continue;
        }
        let gap = bars[i].open / prior - 1.0;
        let wide = (bars[i].high - bars[i].low) / bars[i].close >= params.er_range_mult * base_adr;
        let vol_ok = bars[i].volume >= params.er_volume_mult * avg_vol;
        let up = bars[i].close > prior;
        if vol_ok && up && (gap >= params.er_gap || wide) {
            found = Some(n - 1 - i);
        }
    }
    found
}

fn fmt_pct(value: f64) -> String {
    format!("{:.1}%", value * 100.0)
}

fn fmt_money(value: f64) -> String {
    if value >= 1_000_000_000.0 {
        format!("${:.1}B", value / 1_000_000_000.0)
    } else if value >= 1_000_000.0 {
        format!("${:.1}M", value / 1_000_000.0)
    } else {
        format!("${value:.0}")
    }
}
