//! Consensus module - Block structure, validation, difficulty, and rewards

mod block;
mod validation;
mod difficulty;
mod rewards;

pub use block::*;
pub use validation::*;
pub use difficulty::*;
pub use rewards::*;
