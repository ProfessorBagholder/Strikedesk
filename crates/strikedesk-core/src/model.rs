use serde::{Deserialize, Serialize};

/// One daily session. Dates are opaque sortable strings (`YYYY-MM-DD`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bar {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// Build a session whose high/low surround `close` by `range_pct`.
///
/// `close_pos` is where the close sits inside that range: `1.0` closes on the high.
/// `gap` forces the open to `prev_close * (1 + gap)` and widens the bar so the
/// open stays inside the range. This is a fixture/test helper, not a data feed.
pub fn compose_bar(
    date: &str,
    prev_close: f64,
    close: f64,
    range_pct: f64,
    close_pos: f64,
    volume: f64,
    gap: Option<f64>,
) -> Bar {
    let close = if close.is_finite() && close > 0.0 {
        close
    } else if prev_close.is_finite() && prev_close > 0.0 {
        prev_close
    } else {
        1.0
    };
    let close_pos = if close_pos.is_finite() {
        close_pos.clamp(0.0, 1.0)
    } else {
        0.5
    };
    let range_pct = if range_pct.is_finite() {
        range_pct.max(0.0)
    } else {
        0.0
    };
    let volume = if volume.is_finite() {
        volume.max(0.0)
    } else {
        0.0
    };

    let (open, high, low) = if let Some(gap) = gap {
        let basis = if prev_close.is_finite() && prev_close > 0.0 {
            prev_close
        } else {
            close
        };
        let open = (basis * (1.0 + gap)).max(0.0001);
        let upper = open.max(close);
        let lower = open.min(close);
        let pad = close * range_pct;
        let mut high = upper + pad * (1.0 - close_pos);
        let mut low = (lower - pad * close_pos).max(0.0001);
        high = high.max(open).max(close);
        low = low.min(open).min(close);
        if low > high {
            low = high;
        }
        (open, high, low)
    } else {
        let range = close * range_pct;
        let mut low = (close - close_pos * range).max(0.0001);
        let mut high = low + range;
        if high < close {
            high = close;
        }
        if low > close {
            low = close;
        }
        let mut open = if prev_close.is_finite() && prev_close > 0.0 {
            prev_close
        } else {
            close
        };
        if open > high {
            open = high;
        }
        if open < low {
            open = low;
        }
        (open, high, low)
    };

    Bar {
        date: date.to_string(),
        open,
        high,
        low,
        close,
        volume,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BadgeId {
    #[serde(rename = "KQ")]
    Kq,
    #[serde(rename = "MM")]
    Mm,
    #[serde(rename = "ON")]
    On,
    #[serde(rename = "DB")]
    Db,
    #[serde(rename = "SB4")]
    Sb4,
    #[serde(rename = "SBW")]
    Sbw,
    #[serde(rename = "SB9")]
    Sb9,
    #[serde(rename = "TML")]
    Tml,
    #[serde(rename = "97C")]
    C97,
    #[serde(rename = "52W")]
    Week52,
    #[serde(rename = "ER")]
    Er,
    #[serde(rename = "2A")]
    Stage2A,
    #[serde(rename = "2B")]
    Stage2B,
    #[serde(rename = "2C")]
    Stage2C,
    #[serde(rename = "RS")]
    Rs,
}

impl BadgeId {
    pub fn code(self) -> &'static str {
        match self {
            Self::Kq => "KQ",
            Self::Mm => "MM",
            Self::On => "ON",
            Self::Db => "DB",
            Self::Sb4 => "SB4",
            Self::Sbw => "SBW",
            Self::Sb9 => "SB9",
            Self::Tml => "TML",
            Self::C97 => "97C",
            Self::Week52 => "52W",
            Self::Er => "ER",
            Self::Stage2A => "2A",
            Self::Stage2B => "2B",
            Self::Stage2C => "2C",
            Self::Rs => "RS",
        }
    }

    pub fn tone(self) -> Tone {
        match self {
            Self::Kq | Self::Mm | Self::Week52 => Tone::Gold,
            Self::On | Self::Db | Self::Sbw | Self::Tml => Tone::Cyan,
            Self::Sb4 | Self::Sb9 => Tone::Orange,
            Self::C97 => Tone::Purple,
            Self::Er => Tone::Pink,
            Self::Stage2A | Self::Stage2B | Self::Stage2C | Self::Rs => Tone::Green,
        }
    }

    /// Left-to-right order on a desk row. Strike is not a pill.
    pub fn order(self) -> u8 {
        match self {
            Self::Kq => 0,
            Self::Mm => 1,
            Self::On => 2,
            Self::Db => 3,
            Self::Sb4 => 4,
            Self::Sbw => 5,
            Self::Sb9 => 6,
            Self::Tml => 7,
            Self::C97 => 8,
            Self::Week52 => 9,
            Self::Er => 10,
            Self::Rs => 11,
            Self::Stage2A | Self::Stage2B | Self::Stage2C => 12,
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_uppercase().as_str() {
            "KQ" => Some(Self::Kq),
            "MM" => Some(Self::Mm),
            "ON" => Some(Self::On),
            "DB" => Some(Self::Db),
            "SB4" => Some(Self::Sb4),
            "SBW" => Some(Self::Sbw),
            "SB9" => Some(Self::Sb9),
            "TML" => Some(Self::Tml),
            "97C" => Some(Self::C97),
            "52W" => Some(Self::Week52),
            "ER" => Some(Self::Er),
            "2A" => Some(Self::Stage2A),
            "2B" => Some(Self::Stage2B),
            "2C" => Some(Self::Stage2C),
            "RS" => Some(Self::Rs),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tone {
    Gold,
    Cyan,
    Purple,
    Orange,
    Green,
    Pink,
    Red,
}

impl Tone {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gold => "gold",
            Self::Cyan => "cyan",
            Self::Purple => "purple",
            Self::Orange => "orange",
            Self::Green => "green",
            Self::Pink => "pink",
            Self::Red => "red",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Check {
    pub id: BadgeId,
    pub label: String,
    pub tone: Tone,
    pub passed: bool,
    pub detail: String,
}

impl Check {
    pub fn new(
        id: BadgeId,
        label: impl Into<String>,
        passed: bool,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            id,
            label: label.into(),
            tone: id.tone(),
            passed,
            detail: detail.into(),
        }
    }
}

/// A pill the desk should draw. RS is included even when it is cold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Badge {
    pub id: BadgeId,
    pub label: String,
    pub tone: Tone,
    pub passed: bool,
    pub detail: String,
    pub order: u8,
}

impl From<&Check> for Badge {
    fn from(check: &Check) -> Self {
        Self {
            id: check.id,
            label: check.label.clone(),
            tone: check.tone,
            passed: check.passed,
            detail: check.detail.clone(),
            order: check.id.order(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrikePart {
    pub id: String,
    pub label: String,
    pub points: f64,
    pub max: f64,
}

fn d_min_dollar_volume() -> f64 {
    5_000_000.0
}
fn d_min_adr() -> f64 {
    0.035
}
fn d_kq_prior_move() -> f64 {
    0.30
}
fn d_kq_prior_window() -> usize {
    63
}
fn d_kq_prior_exclude_recent() -> usize {
    10
}
fn d_kq_coil_pct() -> f64 {
    0.05
}
fn d_db_box_bars() -> usize {
    10
}
fn d_db_max_range() -> f64 {
    0.12
}
fn d_db_volume_ratio() -> f64 {
    0.85
}
fn d_db_close_pos() -> f64 {
    0.70
}
fn d_sb4_lookback() -> usize {
    4
}
fn d_sb4_min_return() -> f64 {
    0.04
}
fn d_sb4_vol_mult() -> f64 {
    1.5
}
fn d_sb4_range_mult() -> f64 {
    1.2
}
fn d_sb9_bars() -> usize {
    9
}
fn d_sb9_max_range() -> f64 {
    0.08
}
fn d_sb9_prior_bars() -> usize {
    15
}
fn d_sb9_prior_move() -> f64 {
    0.15
}
fn d_tml_max_above_ema() -> f64 {
    0.03
}
fn d_tml_rise_bars() -> usize {
    5
}
fn d_c97_close_pos() -> f64 {
    0.97
}
fn d_on_lookback() -> usize {
    10
}
fn d_on_near_high() -> f64 {
    0.98
}
fn d_on_close_pos() -> f64 {
    0.50
}
fn d_week52_lookback() -> usize {
    10
}
fn d_week52_min_bars() -> usize {
    100
}
fn d_er_lookback() -> usize {
    15
}
fn d_er_gap() -> f64 {
    0.04
}
fn d_er_volume_mult() -> f64 {
    2.0
}
fn d_er_range_mult() -> f64 {
    2.0
}
fn d_rs_hot() -> u8 {
    80
}
fn d_stage_early() -> f64 {
    0.15
}
fn d_stage_mid() -> f64 {
    0.40
}
fn d_mm_above_low() -> f64 {
    1.30
}
fn d_mm_within_high() -> f64 {
    0.75
}
fn d_mm_slope_bars() -> usize {
    21
}
fn d_w21() -> f64 {
    0.20
}
fn d_w63() -> f64 {
    0.40
}
fn d_w126() -> f64 {
    0.30
}
fn d_w252() -> f64 {
    0.10
}
fn d_loc_tight() -> f64 {
    0.08
}
fn d_loc_mid() -> f64 {
    0.15
}
fn d_loc_wide() -> f64 {
    0.25
}
fn d_extension() -> f64 {
    0.12
}
fn d_pivot_base_bars() -> usize {
    10
}
fn d_pivot_base_max_range() -> f64 {
    0.15
}
fn d_pivot_base_max_drift() -> f64 {
    0.06
}
fn d_pivot_lookback() -> usize {
    63
}
fn d_chase_max_extension() -> f64 {
    0.05
}
fn d_chase_fresh_bars() -> usize {
    2
}
fn d_chase_atr_bars() -> usize {
    14
}
fn d_chase_atr_limit() -> f64 {
    1.0
}
fn d_chase_dock_cap() -> f64 {
    50.0
}
fn d_chase_dist_step() -> f64 {
    0.10
}
fn d_chase_dist_points() -> f64 {
    8.0
}
fn d_chase_dist_cap() -> f64 {
    20.0
}
fn d_chase_atr_points() -> f64 {
    4.0
}
fn d_chase_atr_cap() -> f64 {
    12.0
}
fn d_chase_age_points() -> f64 {
    3.0
}
fn d_chase_age_cap() -> f64 {
    16.0
}
fn d_chase_fresh_pad() -> f64 {
    0.03
}
fn d_chase_ema_step() -> f64 {
    0.10
}
fn d_chase_ema_points() -> f64 {
    8.0
}
fn d_chase_ema_cap() -> f64 {
    16.0
}
fn d_chase_parabolic_window() -> usize {
    21
}
fn d_chase_parabolic_return() -> f64 {
    0.18
}
fn d_chase_sma_max() -> f64 {
    0.06
}
fn d_chase_sma_step() -> f64 {
    0.04
}
fn d_chase_sma_points() -> f64 {
    8.0
}
fn d_chase_sma_cap() -> f64 {
    20.0
}
fn d_coil_bars() -> usize {
    15
}
fn d_coil_max_range() -> f64 {
    0.10
}
fn d_coil_ema_band() -> f64 {
    0.04
}
fn d_coil_sma_max() -> f64 {
    0.05
}
fn d_coil_sma_floor() -> f64 {
    -0.03
}
fn d_coil_thrust_bars() -> usize {
    10
}
fn d_coil_thrust_return() -> f64 {
    0.15
}
fn d_sbw_max_above() -> f64 {
    0.06
}
fn d_sbw_max_drawdown() -> f64 {
    0.15
}

/// Every numeric knob the engine reads. Defaults are the documented Strikedesk rules.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BadgeParams {
    #[serde(default = "d_min_dollar_volume")]
    pub min_dollar_volume: f64,
    #[serde(default = "d_min_adr")]
    pub min_adr: f64,
    #[serde(default = "d_kq_prior_move")]
    pub kq_prior_move: f64,
    #[serde(default = "d_kq_prior_window")]
    pub kq_prior_window: usize,
    #[serde(default = "d_kq_prior_exclude_recent")]
    pub kq_prior_exclude_recent: usize,
    #[serde(default = "d_kq_coil_pct")]
    pub kq_coil_pct: f64,
    #[serde(default = "d_db_box_bars")]
    pub db_box_bars: usize,
    #[serde(default = "d_db_max_range")]
    pub db_max_range: f64,
    #[serde(default = "d_db_volume_ratio")]
    pub db_volume_ratio: f64,
    #[serde(default = "d_db_close_pos")]
    pub db_close_pos: f64,
    #[serde(default = "d_sb4_lookback")]
    pub sb4_lookback: usize,
    #[serde(default = "d_sb4_min_return")]
    pub sb4_min_return: f64,
    #[serde(default = "d_sb4_vol_mult")]
    pub sb4_vol_mult: f64,
    #[serde(default = "d_sb4_range_mult")]
    pub sb4_range_mult: f64,
    #[serde(default = "d_sb9_bars")]
    pub sb9_bars: usize,
    #[serde(default = "d_sb9_max_range")]
    pub sb9_max_range: f64,
    #[serde(default = "d_sb9_prior_bars")]
    pub sb9_prior_bars: usize,
    #[serde(default = "d_sb9_prior_move")]
    pub sb9_prior_move: f64,
    #[serde(default = "d_tml_max_above_ema")]
    pub tml_max_above_ema: f64,
    #[serde(default = "d_tml_rise_bars")]
    pub tml_rise_bars: usize,
    #[serde(default = "d_c97_close_pos")]
    pub c97_close_pos: f64,
    #[serde(default = "d_on_lookback")]
    pub on_lookback: usize,
    #[serde(default = "d_on_near_high")]
    pub on_near_high: f64,
    #[serde(default = "d_on_close_pos")]
    pub on_close_pos: f64,
    #[serde(default = "d_week52_lookback")]
    pub week52_lookback: usize,
    #[serde(default = "d_week52_min_bars")]
    pub week52_min_bars: usize,
    #[serde(default = "d_er_lookback")]
    pub er_lookback: usize,
    #[serde(default = "d_er_gap")]
    pub er_gap: f64,
    #[serde(default = "d_er_volume_mult")]
    pub er_volume_mult: f64,
    #[serde(default = "d_er_range_mult")]
    pub er_range_mult: f64,
    #[serde(default = "d_rs_hot")]
    pub rs_hot: u8,
    #[serde(default = "d_stage_early")]
    pub stage_early: f64,
    #[serde(default = "d_stage_mid")]
    pub stage_mid: f64,
    #[serde(default = "d_mm_above_low")]
    pub mm_above_low: f64,
    #[serde(default = "d_mm_within_high")]
    pub mm_within_high: f64,
    #[serde(default = "d_mm_slope_bars")]
    pub mm_slope_bars: usize,
    #[serde(default = "d_sbw_max_above")]
    pub sbw_max_above: f64,
    #[serde(default = "d_sbw_max_drawdown")]
    pub sbw_max_drawdown: f64,
    /// Sessions in the shelf that must break before a pivot exists.
    #[serde(default = "d_pivot_base_bars")]
    pub pivot_base_bars: usize,
    /// Shelf high-to-low range, as a fraction of the shelf high.
    #[serde(default = "d_pivot_base_max_range")]
    pub pivot_base_max_range: f64,
    /// Absolute close-to-close drift allowed inside that shelf.
    #[serde(default = "d_pivot_base_max_drift")]
    pub pivot_base_max_drift: f64,
    /// How far back a shelf break still counts as the active pivot.
    #[serde(default = "d_pivot_lookback")]
    pub pivot_lookback: usize,
    /// Close within this fraction of the pivot is still a break or retest.
    #[serde(default = "d_chase_max_extension")]
    pub chase_max_extension: f64,
    /// Breakout bar plus this many later sessions still count as the print.
    #[serde(default = "d_chase_fresh_bars")]
    pub chase_fresh_bars: usize,
    #[serde(default = "d_chase_atr_bars")]
    pub chase_atr_bars: usize,
    /// ATRs above EMA(10) allowed before the short-average dock starts.
    #[serde(default = "d_chase_atr_limit")]
    pub chase_atr_limit: f64,
    #[serde(default = "d_chase_dock_cap")]
    pub chase_dock_cap: f64,
    #[serde(default = "d_chase_dist_step")]
    pub chase_dist_step: f64,
    #[serde(default = "d_chase_dist_points")]
    pub chase_dist_points: f64,
    #[serde(default = "d_chase_dist_cap")]
    pub chase_dist_cap: f64,
    #[serde(default = "d_chase_atr_points")]
    pub chase_atr_points: f64,
    #[serde(default = "d_chase_atr_cap")]
    pub chase_atr_cap: f64,
    #[serde(default = "d_chase_age_points")]
    pub chase_age_points: f64,
    #[serde(default = "d_chase_age_cap")]
    pub chase_age_cap: f64,
    /// Extra room past `chase_max_extension` that still waives the ATR dock.
    #[serde(default = "d_chase_fresh_pad")]
    pub chase_fresh_pad: f64,
    #[serde(default = "d_chase_ema_step")]
    pub chase_ema_step: f64,
    #[serde(default = "d_chase_ema_points")]
    pub chase_ema_points: f64,
    #[serde(default = "d_chase_ema_cap")]
    pub chase_ema_cap: f64,
    /// Sessions used to judge a parabolic advance.
    #[serde(default = "d_chase_parabolic_window")]
    pub chase_parabolic_window: usize,
    /// Window return that, with a stretch above SMA(20), counts as parabolic.
    #[serde(default = "d_chase_parabolic_return")]
    pub chase_parabolic_return: f64,
    /// Close within this fraction of SMA(20) is not a parabolic stretch.
    #[serde(default = "d_chase_sma_max")]
    pub chase_sma_max: f64,
    #[serde(default = "d_chase_sma_step")]
    pub chase_sma_step: f64,
    #[serde(default = "d_chase_sma_points")]
    pub chase_sma_points: f64,
    #[serde(default = "d_chase_sma_cap")]
    pub chase_sma_cap: f64,
    /// Sessions that must stay tight for a post-thrust pause.
    #[serde(default = "d_coil_bars")]
    pub coil_bars: usize,
    #[serde(default = "d_coil_max_range")]
    pub coil_max_range: f64,
    /// Absolute distance to EMA(10) allowed inside the pause.
    #[serde(default = "d_coil_ema_band")]
    pub coil_ema_band: f64,
    /// Close may sit this far above SMA(20) and still be the pause, not a chase.
    #[serde(default = "d_coil_sma_max")]
    pub coil_sma_max: f64,
    #[serde(default = "d_coil_sma_floor")]
    pub coil_sma_floor: f64,
    #[serde(default = "d_coil_thrust_bars")]
    pub coil_thrust_bars: usize,
    /// Advance into the pause, measured across the coil plus this many prior sessions.
    #[serde(default = "d_coil_thrust_return")]
    pub coil_thrust_return: f64,
    #[serde(default)]
    pub rs: RsWeights,
    #[serde(default)]
    pub strike: StrikeWeights,
}

impl Default for BadgeParams {
    fn default() -> Self {
        Self {
            min_dollar_volume: d_min_dollar_volume(),
            min_adr: d_min_adr(),
            kq_prior_move: d_kq_prior_move(),
            kq_prior_window: d_kq_prior_window(),
            kq_prior_exclude_recent: d_kq_prior_exclude_recent(),
            kq_coil_pct: d_kq_coil_pct(),
            db_box_bars: d_db_box_bars(),
            db_max_range: d_db_max_range(),
            db_volume_ratio: d_db_volume_ratio(),
            db_close_pos: d_db_close_pos(),
            sb4_lookback: d_sb4_lookback(),
            sb4_min_return: d_sb4_min_return(),
            sb4_vol_mult: d_sb4_vol_mult(),
            sb4_range_mult: d_sb4_range_mult(),
            sb9_bars: d_sb9_bars(),
            sb9_max_range: d_sb9_max_range(),
            sb9_prior_bars: d_sb9_prior_bars(),
            sb9_prior_move: d_sb9_prior_move(),
            tml_max_above_ema: d_tml_max_above_ema(),
            tml_rise_bars: d_tml_rise_bars(),
            c97_close_pos: d_c97_close_pos(),
            on_lookback: d_on_lookback(),
            on_near_high: d_on_near_high(),
            on_close_pos: d_on_close_pos(),
            week52_lookback: d_week52_lookback(),
            week52_min_bars: d_week52_min_bars(),
            er_lookback: d_er_lookback(),
            er_gap: d_er_gap(),
            er_volume_mult: d_er_volume_mult(),
            er_range_mult: d_er_range_mult(),
            rs_hot: d_rs_hot(),
            stage_early: d_stage_early(),
            stage_mid: d_stage_mid(),
            mm_above_low: d_mm_above_low(),
            mm_within_high: d_mm_within_high(),
            mm_slope_bars: d_mm_slope_bars(),
            sbw_max_above: d_sbw_max_above(),
            sbw_max_drawdown: d_sbw_max_drawdown(),
            pivot_base_bars: d_pivot_base_bars(),
            pivot_base_max_range: d_pivot_base_max_range(),
            pivot_base_max_drift: d_pivot_base_max_drift(),
            pivot_lookback: d_pivot_lookback(),
            chase_max_extension: d_chase_max_extension(),
            chase_fresh_bars: d_chase_fresh_bars(),
            chase_atr_bars: d_chase_atr_bars(),
            chase_atr_limit: d_chase_atr_limit(),
            chase_dock_cap: d_chase_dock_cap(),
            chase_dist_step: d_chase_dist_step(),
            chase_dist_points: d_chase_dist_points(),
            chase_dist_cap: d_chase_dist_cap(),
            chase_atr_points: d_chase_atr_points(),
            chase_atr_cap: d_chase_atr_cap(),
            chase_age_points: d_chase_age_points(),
            chase_age_cap: d_chase_age_cap(),
            chase_fresh_pad: d_chase_fresh_pad(),
            chase_ema_step: d_chase_ema_step(),
            chase_ema_points: d_chase_ema_points(),
            chase_ema_cap: d_chase_ema_cap(),
            chase_parabolic_window: d_chase_parabolic_window(),
            chase_parabolic_return: d_chase_parabolic_return(),
            chase_sma_max: d_chase_sma_max(),
            chase_sma_step: d_chase_sma_step(),
            chase_sma_points: d_chase_sma_points(),
            chase_sma_cap: d_chase_sma_cap(),
            coil_bars: d_coil_bars(),
            coil_max_range: d_coil_max_range(),
            coil_ema_band: d_coil_ema_band(),
            coil_sma_max: d_coil_sma_max(),
            coil_sma_floor: d_coil_sma_floor(),
            coil_thrust_bars: d_coil_thrust_bars(),
            coil_thrust_return: d_coil_thrust_return(),
            rs: RsWeights::default(),
            strike: StrikeWeights::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RsWeights {
    #[serde(default = "d_w21")]
    pub w21: f64,
    #[serde(default = "d_w63")]
    pub w63: f64,
    #[serde(default = "d_w126")]
    pub w126: f64,
    #[serde(default = "d_w252")]
    pub w252: f64,
}

impl Default for RsWeights {
    fn default() -> Self {
        Self {
            w21: d_w21(),
            w63: d_w63(),
            w126: d_w126(),
            w252: d_w252(),
        }
    }
}

/// Strike component caps. The sum of the caps is 100.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StrikeWeights {
    #[serde(default = "d_cap_ma")]
    pub ma: f64,
    #[serde(default = "d_cap_rs")]
    pub rs: f64,
    #[serde(default = "d_cap_location")]
    pub location: f64,
    #[serde(default = "d_cap_tight")]
    pub tight: f64,
    #[serde(default = "d_cap_close")]
    pub close_volume: f64,
    #[serde(default = "d_cap_adr")]
    pub adr: f64,
    #[serde(default = "d_cap_liq")]
    pub liquidity: f64,
    #[serde(default = "d_loc_tight")]
    pub location_tight: f64,
    #[serde(default = "d_loc_mid")]
    pub location_mid: f64,
    #[serde(default = "d_loc_wide")]
    pub location_wide: f64,
    #[serde(default = "d_extension")]
    pub extension_penalty_above: f64,
}

fn d_cap_ma() -> f64 {
    20.0
}
fn d_cap_rs() -> f64 {
    20.0
}
fn d_cap_location() -> f64 {
    15.0
}
fn d_cap_tight() -> f64 {
    15.0
}
fn d_cap_close() -> f64 {
    10.0
}
fn d_cap_adr() -> f64 {
    10.0
}
fn d_cap_liq() -> f64 {
    10.0
}

impl Default for StrikeWeights {
    fn default() -> Self {
        Self {
            ma: d_cap_ma(),
            rs: d_cap_rs(),
            location: d_cap_location(),
            tight: d_cap_tight(),
            close_volume: d_cap_close(),
            adr: d_cap_adr(),
            liquidity: d_cap_liq(),
            location_tight: d_loc_tight(),
            location_mid: d_loc_mid(),
            location_wide: d_loc_wide(),
            extension_penalty_above: d_extension(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_bar_contains_prices_and_optional_gap() {
        let bar = compose_bar("D0001", 100.0, 110.0, 0.02, 0.9, 1_000.0, Some(0.05));
        assert!(bar.high >= bar.open && bar.high >= bar.close);
        assert!(bar.low <= bar.open && bar.low <= bar.close);
        assert!(bar.low <= bar.high);
        assert!((bar.open - 105.0).abs() < 1e-9);
    }

    #[test]
    fn badge_codes_round_trip() {
        for code in [
            "KQ", "MM", "ON", "DB", "SB4", "SBW", "SB9", "TML", "97C", "52W", "ER", "2A", "2B",
            "2C", "RS",
        ] {
            let id = BadgeId::parse(code).unwrap();
            assert_eq!(id.code(), code);
        }
        assert!(BadgeId::parse("BUY").is_none());
    }
}
