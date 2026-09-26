use crate::model::{BadgeParams, Tone};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleDoc {
    pub id: String,
    pub label: String,
    pub tone: Tone,
    pub title: String,
    pub summary: String,
    pub body: String,
    pub approximation: bool,
}

/// Human-readable rules for the active parameters.
///
/// Steve Jacobs publishes a Qullamaggie-style desk, but the exact pill
/// thresholds behind that screenshot are not a public spec. Every rule here is
/// the one Strikedesk actually runs. Approximations are marked.
pub fn rulebook(params: &BadgeParams) -> Vec<RuleDoc> {
    let p = params;
    vec![
        RuleDoc {
            id: "KQ".into(),
            label: "KQ".into(),
            tone: Tone::Gold,
            title: "Qullamaggie leader".into(),
            approximation: true,
            summary: "Prior move, short-MA stack, tradable ADR, and a coil.".into(),
            body: format!(
                "Passes only when all are true: close > SMA(50); SMA(10) > SMA(20); ADR(20) ≥ {}; average dollar volume over 20 sessions ≥ {}; prior move ≥ {}. Prior move is last close divided by the lowest low in the {} sessions that end {} sessions ago. The close must also sit within {} of EMA(10) or EMA(20) and at or above EMA(20). Inspired by Kristjan Kullamägi's public breakout screen, not a copy of a private scan.",
                pct(p.min_adr),
                money(p.min_dollar_volume),
                pct(p.kq_prior_move),
                p.kq_prior_window,
                p.kq_prior_exclude_recent,
                pct(p.kq_coil_pct)
            ),
        },
        RuleDoc {
            id: "MM".into(),
            label: "MM".into(),
            tone: Tone::Gold,
            title: "Minervini momentum stack".into(),
            approximation: true,
            summary: "Trend template without the separate RS pill.".into(),
            body: format!(
                "Passes when close > SMA(50) > SMA(150) > SMA(200), SMA(50) is also above SMA(200), SMA(200) is higher than {} sessions ago, close ≥ {} × the 252-session low, and close ≥ {} × the 252-session high. IBD-style RS is the RS square, not a hidden clause of MM. This follows Mark Minervini's published trend template.",
                p.mm_slope_bars,
                trim(p.mm_above_low),
                trim(p.mm_within_high)
            ),
        },
        RuleDoc {
            id: "ON".into(),
            label: "ON".into(),
            tone: Tone::Cyan,
            title: "On the highs".into(),
            approximation: true,
            summary: "Close is pinned to the recent high and not a weak intraday finish.".into(),
            body: format!(
                "Passes when the close is at least {} of the highest high of the last {} sessions and the close sits at least {} of the way from that day's low to its high.",
                pct(p.on_near_high),
                p.on_lookback,
                pct(p.on_close_pos)
            ),
        },
        RuleDoc {
            id: "DB".into(),
            label: "DB".into(),
            tone: Tone::Cyan,
            title: "Darvas-style dry box".into(),
            approximation: true,
            summary: "A tight box, a close near the top, and drying volume.".into(),
            body: format!(
                "Passes when the last {} sessions have range ≤ {} of the close, the close is at least {} of the way up that box, the average volume of the last 5 sessions is ≤ {:.2} × the average of the 15 sessions before them, and the close is above SMA(20).",
                p.db_box_bars,
                pct(p.db_max_range),
                pct(p.db_close_pos),
                p.db_volume_ratio
            ),
        },
        RuleDoc {
            id: "SB4".into(),
            label: "SB4".into(),
            tone: Tone::Orange,
            title: "4-session thrust".into(),
            approximation: true,
            summary: "A short momentum burst with expansion.".into(),
            body: format!(
                "Passes when the close is up at least {} versus {} sessions ago, the close is above SMA(10), and at least one of those {} sessions has volume ≥ {:.1} × the prior average (up to 20 sessions) or a range ≥ {:.1} × that prior ADR.",
                pct(p.sb4_min_return),
                p.sb4_lookback,
                p.sb4_lookback,
                p.sb4_vol_mult,
                p.sb4_range_mult
            ),
        },
        RuleDoc {
            id: "SBW".into(),
            label: "SBW".into(),
            tone: Tone::Cyan,
            title: "Weekly surf".into(),
            approximation: true,
            summary: "Price is riding a rising 10-week average.".into(),
            body: format!(
                "A trading week is a block of 5 sessions aligned to the last bar, not a calendar week. Passes when the close is between 0 and {} above the average of the last 10 weekly closes, that average is higher than the 10-week average ending 4 weeks earlier, and the close is no more than {} below the highest weekly high of the last 26 weeks.",
                pct(p.sbw_max_above),
                pct(p.sbw_max_drawdown)
            ),
        },
        RuleDoc {
            id: "SB9".into(),
            label: "SB9".into(),
            tone: Tone::Orange,
            title: "9-session flag".into(),
            approximation: true,
            summary: "A tight pause after a thrust.".into(),
            body: format!(
                "Passes when the last {} sessions range ≤ {} of the close, the close at the start of that flag is up at least {} versus {} sessions before the flag, the flag has not given back more than 2% from its starting close, and the close is above SMA(20).",
                p.sb9_bars,
                pct(p.sb9_max_range),
                pct(p.sb9_prior_move),
                p.sb9_prior_bars
            ),
        },
        RuleDoc {
            id: "TML".into(),
            label: "TML".into(),
            tone: Tone::Cyan,
            title: "Ten-day MA lift".into(),
            approximation: true,
            summary: "Price tagged a rising 10-day EMA and held just above it.".into(),
            body: format!(
                "Passes when EMA(10) is higher than {} sessions ago, EMA(10) > EMA(20), the close is between 0 and {} above EMA(10), and the lowest low of the last 3 sessions is at or below EMA(10) × 1.01.",
                p.tml_rise_bars,
                pct(p.tml_max_above_ema)
            ),
        },
        RuleDoc {
            id: "97C".into(),
            label: "97C".into(),
            tone: Tone::Purple,
            title: "97 close".into(),
            approximation: true,
            summary: "The session closed in the top of its own range.".into(),
            body: format!(
                "Passes when (close − low) / (high − low) ≥ {}. A zero-range bar passes only if the close is at the high. This is a strong close, not “within 3% of the 52-week high” — that idea is closer to 52W.",
                pct(p.c97_close_pos)
            ),
        },
        RuleDoc {
            id: "52W".into(),
            label: "52W".into(),
            tone: Tone::Gold,
            title: "Recent 52-week high".into(),
            approximation: false,
            summary: "The window high was printed recently.".into(),
            body: format!(
                "Needs at least {} sessions. Passes when the highest high of the last 252 sessions (or the whole history, if shorter) occurred fewer than {} sessions ago. Ties take the most recent print.",
                p.week52_min_bars, p.week52_lookback
            ),
        },
        RuleDoc {
            id: "ER".into(),
            label: "ER-n".into(),
            tone: Tone::Pink,
            title: "Earnings-style reaction".into(),
            approximation: true,
            summary: "A positive shock on volume. n is how many sessions ago.".into(),
            body: format!(
                "Scans the last {} sessions that still have 20 sessions of history. The most recent bar that closes above the prior close, trades at least {}× the prior 20-session average volume, and either gaps up at least {} or ranges at least {}× the prior ADR, becomes ER-n. n = 0 means the reaction is the last bar. This is a price/volume proxy, not a confirmed earnings date.",
                p.er_lookback,
                trim(p.er_volume_mult),
                pct(p.er_gap),
                trim(p.er_range_mult)
            ),
        },
        RuleDoc {
            id: "2A".into(),
            label: "2A / 2B / 2C".into(),
            tone: Tone::Green,
            title: "Weinstein stage 2 phase".into(),
            approximation: true,
            summary: "One phase only: early, mid, or extended.".into(),
            body: format!(
                "Stage 2 requires close > SMA(150) and SMA(150) higher than {} sessions ago. Advance = close / SMA(150) − 1. 2A if advance < {}, 2B if advance < {}, otherwise 2C. Names outside stage 2 get none of the three.",
                p.mm_slope_bars,
                pct(p.stage_early),
                pct(p.stage_mid)
            ),
        },
        RuleDoc {
            id: "RS".into(),
            label: "RS".into(),
            tone: Tone::Green,
            title: "Relative strength square".into(),
            approximation: true,
            summary: "1–99 percentile inside this scan, not an IBD feed.".into(),
            body: format!(
                "Each name gets a weighted return: {}×21 sessions, {}×63, {}×126, {}×252. Missing windows are dropped and the remaining weights are renormalized. When a benchmark is loaded, each window is stock return minus benchmark return on the same dates; otherwise the raw stock return is used. Scores are ranked in the scanned universe. RS = 1 + percentile × 98, rounded, with ties sharing a mid-rank. A one-name universe is 50. The square is drawn either way. It counts as a hot RS badge at ≥ {}.",
                pct_w(p.rs.w21),
                pct_w(p.rs.w63),
                pct_w(p.rs.w126),
                pct_w(p.rs.w252),
                p.rs_hot
            ),
        },
        RuleDoc {
            id: "STRIKE".into(),
            label: "Strike".into(),
            tone: Tone::Green,
            title: "Strike zone, 0–100".into(),
            approximation: true,
            summary: "How many independent conditions line up. Not an order.".into(),
            body: format!(
                "Points sum and round to 0–100. Moving-average stack up to {} (8/20 for close > SMA(50), 6/20 for SMA(50) > SMA(150), 6/20 for SMA(150) > a rising SMA(200)). RS percentile up to {} (RS/99 × cap). Location vs the 63-session high up to {}: full points within {}, two-thirds within {}, one-third within {}; 8 points are removed when the close is more than {} above EMA(10). Tightness of the last 10 sessions up to {}: full ≤ 8%, two-thirds ≤ 12%, one-third ≤ 18%, and the score is capped at 8/15 of the cap when the 63-session return is under 15%. Close and volume up to {}: half for a ≥97% close (or 30% of the cap for a ≥70% close) plus half for volume ≥ 1.2× the 20-session average (or 20% of the cap for ≥ 0.8×). ADR band up to {}: full for 3.5–8%, 60% for 2.5–3.5% or 8–12%, 30% for 1.5–2.5% or 12–18%. Liquidity up to {}: full ≥ $20M average dollar volume, 70% ≥ $5M, 40% ≥ $1M. The circle is green at ≥ 70, gold at ≥ 50, and red below 50.",
                trim(p.strike.ma),
                trim(p.strike.rs),
                trim(p.strike.location),
                pct(p.strike.location_tight),
                pct(p.strike.location_mid),
                pct(p.strike.location_wide),
                pct(p.strike.extension_penalty_above),
                trim(p.strike.tight),
                trim(p.strike.close_volume),
                trim(p.strike.adr),
                trim(p.strike.liquidity)
            ),
        },
    ]
}

fn pct(value: f64) -> String {
    format!("{:.1}%", value * 100.0)
}

fn pct_w(value: f64) -> String {
    format!("{:.0}%", value * 100.0)
}

fn money(value: f64) -> String {
    if value >= 1_000_000.0 {
        format!("${:.0}M", value / 1_000_000.0)
    } else {
        format!("${value:.0}")
    }
}

fn trim(value: f64) -> String {
    if (value - value.round()).abs() < 1e-9 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}
