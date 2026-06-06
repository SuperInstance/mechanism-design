//! Strategy-proofness: verifying no agent benefits from misreporting.
//!
//! Strategy-proofness is equivalent to DSIC — a mechanism is
//! **strategy-proof** if no agent can improve their utility by misreporting
//! their type, regardless of what others report.
//!
//! The **revelation principle** states that any outcome that can be implemented
//! by some mechanism can also be implemented by a strategy-proof direct
//! mechanism. This means we can restrict attention to direct revelation
//! mechanisms where agents simply report their types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::mechanism::{AgentType, Mechanism, Message};
use crate::outcome::Outcome;

/// Result of a strategy-proofness check comparing truthful vs. misreported
/// utilities for a specific agent and misreport.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyProofResult {
    /// Agent being tested.
    pub agent_id: String,
    /// True type of the agent.
    pub true_valuations: HashMap<String, f64>,
    /// Misreported valuations.
    pub misreported_valuations: HashMap<String, f64>,
    /// Utility from truthful reporting.
    pub truthful_utility: f64,
    /// Utility from misreporting.
    pub misreport_utility: f64,
    /// Benefit from misreporting (positive = can exploit).
    pub benefit: f64,
    /// Whether strategy-proofness holds for this specific misreport.
    pub is_strategy_proof: bool,
}

/// Check strategy-proofness for a specific agent and misreport.
pub fn check_strategy_proof_single(
    mechanism: &Mechanism,
    agent: &AgentType,
    misreport: &HashMap<String, f64>,
    others_messages: &[Message],
    payment_fn: &dyn Fn(&[Message], &Outcome, &str) -> f64,
) -> StrategyProofResult {
    // Truthful
    let truthful_msg = Message::from_type(agent);
    let all_truthful = {
        let mut m = others_messages.to_vec();
        m.push(truthful_msg);
        m
    };
    let truthful_outcome = mechanism.select_outcome(&all_truthful);
    let truthful_payment = match &truthful_outcome {
        Some(o) => payment_fn(&all_truthful, o, &agent.id),
        None => 0.0,
    };
    let truthful_utility = match &truthful_outcome {
        Some(o) => Mechanism::utility(agent, o, truthful_payment),
        None => 0.0,
    };

    // Misreported
    let mis_msg = Message::new(&agent.id, misreport.clone());
    let all_mis = {
        let mut m = others_messages.to_vec();
        m.push(mis_msg);
        m
    };
    let mis_outcome = mechanism.select_outcome(&all_mis);
    let mis_payment = match &mis_outcome {
        Some(o) => payment_fn(&all_mis, o, &agent.id),
        None => 0.0,
    };
    let mis_utility = match &mis_outcome {
        Some(o) => Mechanism::utility(agent, o, mis_payment),
        None => 0.0,
    };

    let benefit = mis_utility - truthful_utility;

    StrategyProofResult {
        agent_id: agent.id.clone(),
        true_valuations: agent.valuations.clone(),
        misreported_valuations: misreport.clone(),
        truthful_utility,
        misreport_utility: mis_utility,
        benefit,
        is_strategy_proof: benefit <= 1e-9,
    }
}

/// Verify the revelation principle claim: check that the given direct mechanism
/// is strategy-proof, meaning truthful reporting is optimal.
///
/// Tests against a set of candidate misreports for each agent.
pub fn verify_revelation_principle(
    mechanism: &Mechanism,
    agents: &[AgentType],
    misreports_per_agent: &[Vec<HashMap<String, f64>>],
    payment_fn: &dyn Fn(&[Message], &Outcome, &str) -> f64,
) -> Vec<Vec<StrategyProofResult>> {
    agents
        .iter()
        .enumerate()
        .map(|(i, agent)| {
            let others: Vec<Message> = agents
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, a)| Message::from_type(a))
                .collect();
            misreports_per_agent
                .get(i)
                .map(|misreports| {
                    misreports
                        .iter()
                        .map(|mis| {
                            check_strategy_proof_single(mechanism, agent, mis, &others, payment_fn)
                        })
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect()
}

/// Check if all strategy-proofness results pass.
pub fn is_strategy_proof(results: &[Vec<StrategyProofResult>]) -> bool {
    results
        .iter()
        .all(|agent_results| agent_results.iter().all(|r| r.is_strategy_proof))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::outcome::{Outcome, OutcomeSpace};

    fn zero_payment(_: &[Message], _: &Outcome, _: &str) -> f64 {
        0.0
    }

    #[test]
    fn test_strategy_proof_no_benefit() {
        let mut space = OutcomeSpace::new();
        space.add(Outcome::new("only_outcome"));

        let mech = Mechanism::new("trivial", space);
        let agent =
            AgentType::with_valuations("alice", HashMap::from([("only_outcome".into(), 100.0)]));
        let misreport = HashMap::from([("only_outcome".into(), 0.0)]);

        let result = check_strategy_proof_single(&mech, &agent, &misreport, &[], &zero_payment);
        assert!(result.is_strategy_proof);
        assert!(result.benefit <= 1e-9);
    }

    #[test]
    fn test_revelation_principle_single_agent() {
        let mut space = OutcomeSpace::new();
        space.add(Outcome::new("x"));
        space.add(Outcome::new("y"));

        let mech = Mechanism::new("test", space);
        let agent =
            AgentType::with_valuations("a", HashMap::from([("x".into(), 10.0), ("y".into(), 5.0)]));

        let misreports = vec![HashMap::from([("x".into(), 0.0), ("y".into(), 20.0)])];

        let results = verify_revelation_principle(&mech, &[agent], &[misreports], &zero_payment);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_is_strategy_proof_all_pass() {
        let results = vec![vec![StrategyProofResult {
            agent_id: "a".into(),
            true_valuations: HashMap::new(),
            misreported_valuations: HashMap::new(),
            truthful_utility: 10.0,
            misreport_utility: 9.0,
            benefit: -1.0,
            is_strategy_proof: true,
        }]];
        assert!(is_strategy_proof(&results));
    }

    #[test]
    fn test_is_strategy_proof_fails() {
        let results = vec![vec![StrategyProofResult {
            agent_id: "a".into(),
            true_valuations: HashMap::new(),
            misreported_valuations: HashMap::new(),
            truthful_utility: 5.0,
            misreport_utility: 10.0,
            benefit: 5.0,
            is_strategy_proof: false,
        }]];
        assert!(!is_strategy_proof(&results));
    }

    #[test]
    fn test_strategy_proof_preserves_truthtelling() {
        // When the misreport equals the true type, benefit must be zero
        let mut space = OutcomeSpace::new();
        space.add(Outcome::new("item"));

        let mech = Mechanism::new("test", space);
        let agent = AgentType::with_valuations("alice", HashMap::from([("item".into(), 50.0)]));
        let same_report = HashMap::from([("item".into(), 50.0)]);

        let result = check_strategy_proof_single(&mech, &agent, &same_report, &[], &zero_payment);
        assert!(result.benefit.abs() < 1e-9);
        assert!(result.is_strategy_proof);
    }
}
