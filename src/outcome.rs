//! Outcome space: allocations, social welfare, Pareto efficiency.
//!
//! An **outcome** is the result of a mechanism — who gets what, and at what
//! cost. This module provides types for representing discrete allocations,
//! computing social welfare, and checking Pareto efficiency.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A named outcome in a discrete outcome space.
///
/// Each outcome is identified by a string label (e.g. `"agent_0_wins"`,
/// `"split_evenly"`). Valuations map each agent to their utility for the
/// outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outcome {
    /// Human-readable label for this outcome.
    pub label: String,
    /// Agent → utility mapping. `valuations[agent_id]` is the utility agent
    /// `agent_id` receives from this outcome.
    pub valuations: HashMap<String, f64>,
}

impl Outcome {
    /// Create a new outcome with the given label and no valuations.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            valuations: HashMap::new(),
        }
    }

    /// Create an outcome with pre-populated valuations.
    pub fn with_valuations(label: impl Into<String>, valuations: HashMap<String, f64>) -> Self {
        Self {
            label: label.into(),
            valuations,
        }
    }

    /// Set the valuation for a single agent.
    pub fn set_valuation(&mut self, agent_id: impl Into<String>, value: f64) {
        self.valuations.insert(agent_id.into(), value);
    }

    /// Compute the **utilitarian social welfare**: sum of all agents' valuations.
    pub fn social_welfare(&self) -> f64 {
        self.valuations.values().sum()
    }
}

/// A collection of outcomes forming the full outcome space Ω.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeSpace {
    /// All outcomes in the space.
    pub outcomes: Vec<Outcome>,
}

impl OutcomeSpace {
    /// Create an empty outcome space.
    pub fn new() -> Self {
        Self {
            outcomes: Vec::new(),
        }
    }

    /// Create an outcome space from a vector of outcomes.
    pub fn from_outcomes(outcomes: Vec<Outcome>) -> Self {
        Self { outcomes }
    }

    /// Add an outcome to the space.
    pub fn add(&mut self, outcome: Outcome) {
        self.outcomes.push(outcome);
    }

    /// Return the outcome that maximizes utilitarian social welfare.
    ///
    /// Returns `None` if the outcome space is empty.
    pub fn welfare_maximizing(&self) -> Option<&Outcome> {
        self.outcomes.iter().max_by(|a, b| {
            a.social_welfare()
                .partial_cmp(&b.social_welfare())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Check if `outcome` is **Pareto efficient** — no other outcome in the
    /// space makes at least one agent strictly better off without making any
    /// agent worse off.
    pub fn is_pareto_efficient(&self, outcome: &Outcome) -> bool {
        let agents: Vec<&String> = outcome.valuations.keys().collect();
        for candidate in &self.outcomes {
            if candidate.label == outcome.label {
                continue;
            }
            let mut dominates = false;
            let mut dominated = false;
            for agent in &agents {
                let c_val = candidate.valuations.get(*agent).copied().unwrap_or(0.0);
                let o_val = outcome.valuations.get(*agent).copied().unwrap_or(0.0);
                if c_val > o_val {
                    dominates = true;
                } else if c_val < o_val {
                    dominated = true;
                }
            }
            if dominates && !dominated {
                return false;
            }
        }
        true
    }

    /// Return all Pareto efficient outcomes.
    pub fn pareto_frontier(&self) -> Vec<&Outcome> {
        self.outcomes
            .iter()
            .filter(|o| self.is_pareto_efficient(o))
            .collect()
    }

    /// Number of outcomes in the space.
    pub fn len(&self) -> usize {
        self.outcomes.len()
    }

    /// Returns true if the outcome space is empty.
    pub fn is_empty(&self) -> bool {
        self.outcomes.is_empty()
    }
}

impl Default for OutcomeSpace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outcome_social_welfare() {
        let mut o = Outcome::new("test");
        o.set_valuation("a", 10.0);
        o.set_valuation("b", 20.0);
        assert!((o.social_welfare() - 30.0).abs() < 1e-9);
    }

    #[test]
    fn test_welfare_maximizing() {
        let mut o1 = Outcome::new("low");
        o1.set_valuation("a", 5.0);
        let mut o2 = Outcome::new("high");
        o2.set_valuation("a", 15.0);
        let space = OutcomeSpace::from_outcomes(vec![o1, o2]);
        let best = space.welfare_maximizing().unwrap();
        assert_eq!(best.label, "high");
    }

    #[test]
    fn test_pareto_efficient_simple() {
        let mut o1 = Outcome::new("good");
        o1.set_valuation("a", 10.0);
        o1.set_valuation("b", 10.0);
        let mut o2 = Outcome::new("bad");
        o2.set_valuation("a", 5.0);
        o2.set_valuation("b", 5.0);
        let space = OutcomeSpace::from_outcomes(vec![o1.clone(), o2.clone()]);
        assert!(space.is_pareto_efficient(&o1));
        assert!(!space.is_pareto_efficient(&o2));
    }

    #[test]
    fn test_pareto_frontier() {
        // o1 and o2 are Pareto-efficient; o3 is dominated by o1
        let mut o1 = Outcome::new("high_a");
        o1.set_valuation("a", 10.0);
        o1.set_valuation("b", 2.0);
        let mut o2 = Outcome::new("high_b");
        o2.set_valuation("a", 2.0);
        o2.set_valuation("b", 10.0);
        let mut o3 = Outcome::new("dominated");
        o3.set_valuation("a", 1.0);
        o3.set_valuation("b", 1.0);
        let space = OutcomeSpace::from_outcomes(vec![o1, o2, o3]);
        let frontier = space.pareto_frontier();
        // o1 dominates o3 (10>1, 2>1). o2 dominates o3 (2>1, 10>1).
        // o1 and o2 don't dominate each other.
        assert_eq!(frontier.len(), 2);
    }

    #[test]
    fn test_empty_space() {
        let space = OutcomeSpace::new();
        assert!(space.is_empty());
        assert!(space.welfare_maximizing().is_none());
    }

    #[test]
    fn test_outcome_with_valuations() {
        let o = Outcome::with_valuations(
            "test",
            HashMap::from([("a".into(), 5.0), ("b".into(), 10.0)]),
        );
        assert_eq!(o.label, "test");
        assert!((o.social_welfare() - 15.0).abs() < 1e-9);
    }

    #[test]
    fn test_outcome_serialization() {
        let o = Outcome::new("serializable");
        let json = serde_json::to_string(&o).unwrap();
        assert!(json.contains("serializable"));
    }

    #[test]
    fn test_outcome_space_default() {
        let space = OutcomeSpace::default();
        assert!(space.is_empty());
    }

    #[test]
    fn test_outcome_space_len() {
        let mut space = OutcomeSpace::new();
        assert_eq!(space.len(), 0);
        space.add(Outcome::new("a"));
        space.add(Outcome::new("b"));
        assert_eq!(space.len(), 2);
    }
}
