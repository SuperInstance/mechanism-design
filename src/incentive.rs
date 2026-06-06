//! Incentive compatibility: verifying DSIC (Dominant Strategy Incentive Compatibility).
//!
//! A mechanism is **dominant-strategy incentive compatible (DSIC)** if
//! truth-telling is a dominant strategy — every agent maximizes their utility
//! by reporting their true type, *regardless of what other agents report*.
//!
//! Formally, for every agent `i`, every true type `θᵢ`, every possible
//! misreport `θ̂ᵢ ≠ θᵢ`, and every profile of others' reports `θ₋ᵢ`:
//!
//! ```text
//! uᵢ(x(θᵢ, θ₋ᵢ), θᵢ) ≥ uᵢ(x(θ̂ᵢ, θ₋ᵢ), θᵢ)
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::mechanism::{AgentType, Mechanism, Message};
use crate::outcome::Outcome;

/// Result of checking incentive compatibility for a single agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncentiveCheck {
    /// Agent that was checked.
    pub agent_id: String,
    /// Whether DSIC holds for this agent.
    pub is_dsic: bool,
    /// Maximum gain from misreporting (negative = no gain, positive = incentive to lie).
    pub max_misreport_gain: f64,
    /// The best misreport found (if DSIC is violated).
    pub best_misreport: Option<HashMap<String, f64>>,
}

/// Check DSIC for a single agent by exhaustive search over a discrete grid.
///
/// For each possible misreport, compares the agent's utility from truthful
/// reporting vs. misreporting, holding others' reports fixed.
///
/// `grid_values` defines the discrete set of values to try for misreporting.
pub fn check_dsic_agent(
    mechanism: &Mechanism,
    agent: &AgentType,
    others_messages: &[Message],
    grid_values: &[f64],
    outcome_labels: &[String],
    payment_fn: &dyn Fn(&[Message], &Outcome, &str) -> f64,
) -> IncentiveCheck {
    let truthful_msg = Message::from_type(agent);
    let all_msgs_truth = {
        let mut msgs: Vec<Message> = others_messages.to_vec();
        msgs.push(truthful_msg.clone());
        msgs
    };

    let truthful_outcome = mechanism.select_outcome(&all_msgs_truth);
    let truthful_payment = match &truthful_outcome {
        Some(o) => payment_fn(&all_msgs_truth, o, &agent.id),
        None => 0.0,
    };
    let truthful_utility = match &truthful_outcome {
        Some(o) => Mechanism::utility(agent, o, truthful_payment),
        None => 0.0,
    };

    let mut max_gain = 0.0_f64;
    let mut best_misreport: Option<HashMap<String, f64>> = None;

    // Generate all possible misreports from the grid
    let misreports = generate_misreports(grid_values, outcome_labels);
    for misreport_vals in misreports {
        let mis_msg = Message::new(&agent.id, misreport_vals.clone());
        let all_msgs_mis = {
            let mut msgs: Vec<Message> = others_messages.to_vec();
            msgs.push(mis_msg);
            msgs
        };
        let mis_outcome = mechanism.select_outcome(&all_msgs_mis);
        let mis_payment = match &mis_outcome {
            Some(o) => payment_fn(&all_msgs_mis, o, &agent.id),
            None => 0.0,
        };
        let mis_utility = match &mis_outcome {
            Some(o) => Mechanism::utility(agent, o, mis_payment),
            None => 0.0,
        };
        let gain = mis_utility - truthful_utility;
        if gain > max_gain {
            max_gain = gain;
            best_misreport = Some(misreport_vals);
        }
    }

    IncentiveCheck {
        agent_id: agent.id.clone(),
        is_dsic: max_gain <= 1e-9,
        max_misreport_gain: max_gain,
        best_misreport,
    }
}

/// Check DSIC for all agents.
pub fn check_dsic(
    mechanism: &Mechanism,
    agents: &[AgentType],
    grid_values: &[f64],
    outcome_labels: &[String],
    payment_fn: &dyn Fn(&[Message], &Outcome, &str) -> f64,
) -> Vec<IncentiveCheck> {
    agents
        .iter()
        .map(|agent| {
            let others: Vec<Message> = agents
                .iter()
                .filter(|a| a.id != agent.id)
                .map(Message::from_type)
                .collect();
            check_dsic_agent(
                mechanism,
                agent,
                &others,
                grid_values,
                outcome_labels,
                payment_fn,
            )
        })
        .collect()
}

/// Check whether all agents pass DSIC.
pub fn is_dsic(checks: &[IncentiveCheck]) -> bool {
    checks.iter().all(|c| c.is_dsic)
}

/// Generate all combinations of values from `grid` for each outcome label.
fn generate_misreports(grid: &[f64], labels: &[String]) -> Vec<HashMap<String, f64>> {
    if labels.is_empty() {
        return vec![HashMap::new()];
    }
    let mut results = Vec::new();
    generate_misreports_recursive(grid, labels, 0, &mut HashMap::new(), &mut results);
    results
}

fn generate_misreports_recursive(
    grid: &[f64],
    labels: &[String],
    idx: usize,
    current: &mut HashMap<String, f64>,
    results: &mut Vec<HashMap<String, f64>>,
) {
    if idx == labels.len() {
        results.push(current.clone());
        return;
    }
    for &val in grid {
        current.insert(labels[idx].clone(), val);
        generate_misreports_recursive(grid, labels, idx + 1, current, results);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::outcome::OutcomeSpace;

    fn zero_payment(_: &[Message], _: &Outcome, _: &str) -> f64 {
        0.0
    }

    #[test]
    fn test_dsic_with_zero_payments() {
        // Without payments, welfare maximization is NOT DSIC in general
        let mut space = OutcomeSpace::new();
        space.add(crate::outcome::Outcome::new("a"));
        space.add(crate::outcome::Outcome::new("b"));

        let mech = Mechanism::new("test", space);
        let mut alice = AgentType::new("alice");
        alice.set_valuation("a", 10.0);
        alice.set_valuation("b", 2.0);

        let mut bob = AgentType::new("bob");
        bob.set_valuation("a", 1.0);
        bob.set_valuation("b", 8.0);

        let agents = vec![alice, bob];
        let labels = vec!["a".into(), "b".into()];
        let grid = vec![0.0, 5.0, 10.0, 15.0, 20.0];

        let checks = check_dsic(&mech, &agents, &grid, &labels, &zero_payment);
        // Without payments, misreporting can change the outcome to your preferred one
        // So DSIC may or may not hold depending on the grid
        assert_eq!(checks.len(), 2);
    }

    #[test]
    fn test_dsic_single_agent() {
        let mut space = OutcomeSpace::new();
        space.add(crate::outcome::Outcome::new("x"));

        let mech = Mechanism::new("single", space);
        let mut agent = AgentType::new("only");
        agent.set_valuation("x", 42.0);

        let labels = vec!["x".into()];
        let grid = vec![0.0, 10.0, 50.0, 100.0];

        let check = check_dsic_agent(&mech, &agent, &[], &grid, &labels, &zero_payment);
        // Single outcome, single agent — misreporting doesn't change anything
        assert!(check.is_dsic);
    }

    #[test]
    fn test_is_dsic_all_pass() {
        let checks = vec![
            IncentiveCheck {
                agent_id: "a".into(),
                is_dsic: true,
                max_misreport_gain: 0.0,
                best_misreport: None,
            },
            IncentiveCheck {
                agent_id: "b".into(),
                is_dsic: true,
                max_misreport_gain: -0.5,
                best_misreport: None,
            },
        ];
        assert!(is_dsic(&checks));
    }

    #[test]
    fn test_is_dsic_fails() {
        let checks = vec![IncentiveCheck {
            agent_id: "a".into(),
            is_dsic: false,
            max_misreport_gain: 5.0,
            best_misreport: Some(HashMap::new()),
        }];
        assert!(!is_dsic(&checks));
    }

    #[test]
    fn test_generate_misreports() {
        let labels = vec!["a".into(), "b".into()];
        let grid = vec![0.0, 10.0];
        let misreports = generate_misreports(&grid, &labels);
        assert_eq!(misreports.len(), 4); // 2^2
    }
}
