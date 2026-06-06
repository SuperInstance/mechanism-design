//! Mechanism: the core abstraction mapping agent types to outcomes.
//!
//! A **mechanism** is the rules of the game. Formally, it's a tuple
//! (M, x, p) where:
//! - **M** is the message space (what agents can report)
//! - **x** is the outcome function (messages → allocation)
//! - **p** is the payment function (messages → transfers)
//!
//! This module provides the fundamental building blocks that higher-level
//! modules (VCG, incentive compatibility, strategy-proofness) build upon.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::outcome::{Outcome, OutcomeSpace};

/// An agent's type — their private valuation for each possible outcome.
///
/// `valuations[outcome_label]` gives the agent's true value for that outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentType {
    /// Agent identifier.
    pub id: String,
    /// Outcome label → true valuation.
    pub valuations: HashMap<String, f64>,
}

impl AgentType {
    /// Create a new agent type.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            valuations: HashMap::new(),
        }
    }

    /// Create an agent type from a valuation map.
    pub fn with_valuations(id: impl Into<String>, valuations: HashMap<String, f64>) -> Self {
        Self {
            id: id.into(),
            valuations,
        }
    }

    /// Set the valuation for a specific outcome.
    pub fn set_valuation(&mut self, outcome: impl Into<String>, value: f64) {
        self.valuations.insert(outcome.into(), value);
    }

    /// Get the agent's valuation for a specific outcome.
    pub fn valuation(&self, outcome: &str) -> f64 {
        self.valuations.get(outcome).copied().unwrap_or(0.0)
    }
}

/// A message (bid/report) submitted by an agent to the mechanism.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Agent who sent the message.
    pub agent_id: String,
    /// Reported valuations (may differ from true type).
    pub reported: HashMap<String, f64>,
}

impl Message {
    /// Create a message from an agent type (truthful report).
    pub fn from_type(agent_type: &AgentType) -> Self {
        Self {
            agent_id: agent_type.id.clone(),
            reported: agent_type.valuations.clone(),
        }
    }

    /// Create a potentially untruthful message.
    pub fn new(agent_id: impl Into<String>, reported: HashMap<String, f64>) -> Self {
        Self {
            agent_id: agent_id.into(),
            reported,
        }
    }
}

/// A mechanism (M, x, p).
///
/// The outcome function `x` selects an outcome from the outcome space given
/// all agents' messages. The payment function `p` determines monetary
/// transfers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mechanism {
    /// Name/label for this mechanism.
    pub name: String,
    /// The outcome space Ω.
    pub outcome_space: OutcomeSpace,
}

impl Mechanism {
    /// Create a new mechanism with the given outcome space.
    pub fn new(name: impl Into<String>, outcome_space: OutcomeSpace) -> Self {
        Self {
            name: name.into(),
            outcome_space,
        }
    }

    /// Select the welfare-maximizing outcome given reported messages.
    ///
    /// This is the standard outcome function for efficient mechanisms.
    /// It picks the outcome that maximizes the sum of reported valuations.
    pub fn select_outcome(&self, messages: &[Message]) -> Option<Outcome> {
        if messages.is_empty() || self.outcome_space.is_empty() {
            return None;
        }

        let mut best: Option<(Outcome, f64)> = None;
        for outcome in &self.outcome_space.outcomes {
            let welfare: f64 = messages
                .iter()
                .map(|m| m.reported.get(&outcome.label).copied().unwrap_or(0.0))
                .sum();
            match &best {
                None => best = Some((outcome.clone(), welfare)),
                Some((_, best_w)) => {
                    if welfare > *best_w {
                        best = Some((outcome.clone(), welfare));
                    }
                }
            }
        }
        best.map(|(o, _)| o)
    }

    /// Compute each agent's utility given an outcome and payment.
    ///
    /// `utility = true_valuation(outcome) - payment`
    pub fn utility(agent_type: &AgentType, outcome: &Outcome, payment: f64) -> f64 {
        agent_type.valuation(&outcome.label) - payment
    }
}

/// Result of running a mechanism on a set of messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MechanismResult {
    /// The mechanism that was run.
    pub mechanism_name: String,
    /// The selected outcome.
    pub outcome: Option<Outcome>,
    /// Payments from each agent: agent_id → payment (positive = agent pays).
    pub payments: HashMap<String, f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_type_valuation() {
        let mut t = AgentType::new("alice");
        t.set_valuation("item", 42.0);
        assert!((t.valuation("item") - 42.0).abs() < 1e-9);
        assert!((t.valuation("nonexistent") - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_message_from_type() {
        let mut t = AgentType::new("bob");
        t.set_valuation("x", 10.0);
        let msg = Message::from_type(&t);
        assert_eq!(msg.agent_id, "bob");
        assert!((msg.reported["x"] - 10.0).abs() < 1e-9);
    }

    #[test]
    fn test_mechanism_select_outcome() {
        let mut space = OutcomeSpace::new();
        space.add(Outcome::new("a"));
        space.add(Outcome::new("b"));

        let mech = Mechanism::new("test", space);

        let msg1 = Message::new("1", HashMap::from([("a".into(), 10.0), ("b".into(), 5.0)]));
        let msg2 = Message::new("2", HashMap::from([("a".into(), 3.0), ("b".into(), 20.0)]));

        let result = mech.select_outcome(&[msg1, msg2]).unwrap();
        // a: 10+3=13, b: 5+20=25 → b wins
        assert_eq!(result.label, "b");
    }

    #[test]
    fn test_utility_computation() {
        let mut t = AgentType::new("alice");
        t.set_valuation("item", 100.0);
        let o = Outcome::with_valuations("item", HashMap::new());
        let util = Mechanism::utility(&t, &o, 30.0);
        assert!((util - 70.0).abs() < 1e-9);
    }

    #[test]
    fn test_empty_mechanism() {
        let mech = Mechanism::new("empty", OutcomeSpace::new());
        assert!(mech.select_outcome(&[]).is_none());
    }

    #[test]
    fn test_mechanism_name() {
        let mech = Mechanism::new("my_mech", OutcomeSpace::new());
        assert_eq!(mech.name, "my_mech");
    }

    #[test]
    fn test_mechanism_result_serialization() {
        let result = MechanismResult {
            mechanism_name: "test".into(),
            outcome: None,
            payments: HashMap::new(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("test"));
    }

    #[test]
    fn test_agent_type_with_valuations() {
        let agent = AgentType::with_valuations("alice", HashMap::from([("x".into(), 5.0)]));
        assert_eq!(agent.id, "alice");
        assert!((agent.valuation("x") - 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_message_new() {
        let msg = Message::new("bob", HashMap::from([("y".into(), 3.0)]));
        assert_eq!(msg.agent_id, "bob");
        assert!((msg.reported["y"] - 3.0).abs() < 1e-9);
    }
}
