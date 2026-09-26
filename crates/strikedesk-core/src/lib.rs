//! Strikedesk badge engine.
//!
//! This crate is intentionally pure: bars in, badges and a strike score out.
//! It does not read files, call the network, or know what a broker is.
//! Bagholder (or any other Rust host) can depend on this crate alone.

mod evaluate;
mod indicators;
mod model;
mod prepare;
mod rank;
mod rules;

#[cfg(test)]
mod badges_tests;

pub use evaluate::{evaluate, Evaluation, Metrics};
pub use model::{
    compose_bar, Badge, BadgeId, BadgeParams, Bar, Check, RsWeights, StrikePart, StrikeWeights,
    Tone,
};
pub use prepare::{momentum_score, prepare, EvalError, Prepared};
pub use rank::assign_rs;
pub use rules::{rulebook, RuleDoc};

/// Screen a symbol once its cross-sectional RS percentile is known.
///
/// `rs` is 1–99 from [`assign_rs`]. Pass `50` only for a one-name universe.
pub fn screen(prepared: &Prepared, rs: u8, params: &BadgeParams) -> Evaluation {
    evaluate(prepared, rs, params)
}
