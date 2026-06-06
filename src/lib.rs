//! # mechanism-design
//!
//! Automated mechanism design: incentive compatibility, strategy-proofness,
//! and Vickrey-Clarke-Groves mechanisms.
//!
//! # Modules
//!
//! - [`mechanism`] — Core mechanism abstraction: message spaces, outcome
//!   functions, payment functions
//! - [`outcome`] — Outcome spaces, allocations, social welfare, Pareto
//!   efficiency
//! - [`incentive`] — Dominant-strategy incentive compatibility (DSIC)
//!   verification
//! - [`strategy_proof`] — Strategy-proofness verification, revelation
//!   principle
//! - [`vcg`] — Vickrey-Clarke-Groves mechanism with Clarke pivot payments
//! - [`social_choice`] — Social choice theory: Borda, Condorcet, approval
//!   voting, Arrow's impossibility

pub mod incentive;
pub mod mechanism;
pub mod outcome;
pub mod social_choice;
pub mod strategy_proof;
pub mod vcg;

// Re-export key types at the crate root for convenience
pub use incentive::{IncentiveCheck, check_dsic, is_dsic};
pub use mechanism::{AgentType, Mechanism, MechanismResult, Message};
pub use outcome::{Outcome, OutcomeSpace};
pub use social_choice::{ArrowProperties, Preference, PreferenceProfile};
pub use strategy_proof::{StrategyProofResult, verify_revelation_principle};
pub use vcg::{TruthfulnessReport, VCGMechanism, is_vcg_truthful};
