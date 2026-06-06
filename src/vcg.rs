//! Vickrey-Clarke-Groves (VCG) mechanism implementation.
//!
//! The VCG mechanism is the canonical efficient, strategy-proof mechanism.
//! It selects the welfare-maximizing outcome and charges each agent the
//! **Clarke pivot rule** payment:
//!
//! ```text
//! pᵢ = Σⱼ≠ᵢ vⱼ(x*(θ₋ᵢ)) − Σⱼ≠ᵢ vⱼ(x*(θ))
//! ```
//!
//! where `x*(θ)` is the welfare-maximizing outcome with all reports, and
//! `x*(θ₋ᵢ)` is the welfare-maximizing outcome without agent `i`'s report.
//!
//! Properties:
//! - **Efficiency**: maximizes social welfare
//! - **Strategy-proofness (DSIC)**: truth-telling is a dominant strategy
//! - **Individual rationality**: agents never regret participating
//! - **Budget balance**: may run a surplus (weak budget balance)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::mechanism::{AgentType, Mechanism, MechanismResult, Message};
use crate::outcome::{Outcome, OutcomeSpace};

/// A VCG mechanism with Clarke pivot payments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VCGMechanism {
    /// Underlying mechanism.
    pub mechanism: Mechanism,
}

impl VCGMechanism {
    /// Create a new VCG mechanism over the given outcome space.
    pub fn new(name: impl Into<String>, outcome_space: OutcomeSpace) -> Self {
        Self {
            mechanism: Mechanism::new(name, outcome_space),
        }
    }

    /// Compute the welfare-maximizing outcome excluding a specific agent.
    fn outcome_without(&self, messages: &[Message], excluded_agent: &str) -> Option<Outcome> {
        let filtered: Vec<&Message> = messages
            .iter()
            .filter(|m| m.agent_id != excluded_agent)
            .collect();

        if filtered.is_empty() || self.mechanism.outcome_space.is_empty() {
            return None;
        }

        let mut best: Option<(Outcome, f64)> = None;
        for outcome in &self.mechanism.outcome_space.outcomes {
            let welfare: f64 = filtered
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

    /// Compute the sum of others' reported valuations for a given outcome.
    fn others_welfare(messages: &[Message], outcome: &Outcome, excluded_agent: &str) -> f64 {
        messages
            .iter()
            .filter(|m| m.agent_id != excluded_agent)
            .map(|m| m.reported.get(&outcome.label).copied().unwrap_or(0.0))
            .sum()
    }

    /// Compute Clarke pivot payments for all agents.
    ///
    /// The Clarke pivot rule charges agent `i`:
    /// ```text
    /// pᵢ = SW₋ᵢ(x*(θ₋ᵢ)) − SW₋ᵢ(x*(θ))
    /// ```
    /// where SW₋ᵢ is the social welfare of agents other than `i`.
    pub fn clarke_payments(
        &self,
        messages: &[Message],
        chosen_outcome: &Outcome,
    ) -> HashMap<String, f64> {
        let mut payments = HashMap::new();
        for msg in messages {
            let outcome_without = self.outcome_without(messages, &msg.agent_id);
            let welfare_without = match &outcome_without {
                Some(o) => Self::others_welfare(messages, o, &msg.agent_id),
                None => 0.0,
            };
            let welfare_chosen = Self::others_welfare(messages, chosen_outcome, &msg.agent_id);
            let payment = welfare_without - welfare_chosen;
            payments.insert(msg.agent_id.clone(), payment);
        }
        payments
    }

    /// Run the full VCG mechanism: select outcome + compute payments.
    pub fn run(&self, agent_types: &[AgentType]) -> MechanismResult {
        let messages: Vec<Message> = agent_types.iter().map(Message::from_type).collect();
        let outcome = self.mechanism.select_outcome(&messages);
        let payments = match &outcome {
            Some(o) => self.clarke_payments(&messages, o),
            None => HashMap::new(),
        };
        MechanismResult {
            mechanism_name: self.mechanism.name.clone(),
            outcome,
            payments,
        }
    }

    /// Check **individual rationality**: no agent pays more than their
    /// valuation, so every agent gets non-negative utility from participation.
    pub fn check_individual_rationality(
        &self,
        result: &MechanismResult,
        agent_types: &[AgentType],
    ) -> bool {
        match &result.outcome {
            None => true,
            Some(outcome) => agent_types.iter().all(|agent| {
                let payment = result.payments.get(&agent.id).copied().unwrap_or(0.0);
                let utility = Mechanism::utility(agent, outcome, payment);
                utility >= -1e-9
            }),
        }
    }

    /// Check **weak budget balance**: total payments are non-negative
    /// (the mechanism doesn't need external subsidies).
    pub fn check_budget_balance(&self, result: &MechanismResult) -> bool {
        let total: f64 = result.payments.values().sum();
        total >= -1e-9
    }

    /// Verify VCG truthfulness: check that no agent benefits from misreporting.
    ///
    /// Tests each agent against a set of candidate misreports.
    pub fn verify_truthfulness(
        &self,
        agent_types: &[AgentType],
        misreports: &[Vec<HashMap<String, f64>>],
    ) -> Vec<TruthfulnessReport> {
        agent_types
            .iter()
            .enumerate()
            .map(|(i, agent)| {
                let truthful_result = self.run(agent_types);
                let agent_misreports = misreports.get(i).cloned().unwrap_or_default();

                let truthful_utility = match (
                    &truthful_result.outcome,
                    truthful_result.payments.get(&agent.id),
                ) {
                    (Some(o), Some(&p)) => Mechanism::utility(agent, o, p),
                    _ => 0.0,
                };

                let violations: Vec<MisreportViolation> = agent_misreports
                    .iter()
                    .filter_map(|mis| {
                        let mut modified = agent_types.to_vec();
                        modified[i] = AgentType::with_valuations(&agent.id, mis.clone());
                        let mis_result = self.run(&modified);
                        let mis_utility =
                            match (&mis_result.outcome, mis_result.payments.get(&agent.id)) {
                                (Some(o), Some(&p)) => Mechanism::utility(agent, o, p),
                                _ => 0.0,
                            };
                        let gain = mis_utility - truthful_utility;
                        if gain > 1e-9 {
                            Some(MisreportViolation {
                                misreported: mis.clone(),
                                gain,
                            })
                        } else {
                            None
                        }
                    })
                    .collect();

                TruthfulnessReport {
                    agent_id: agent.id.clone(),
                    truthful_utility,
                    is_truthful: violations.is_empty(),
                    violations,
                }
            })
            .collect()
    }
}

/// Report on an agent's incentive to misreport in a VCG mechanism.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruthfulnessReport {
    /// Agent identifier.
    pub agent_id: String,
    /// Utility from truthful reporting.
    pub truthful_utility: f64,
    /// Whether VCG truthfulness holds for this agent.
    pub is_truthful: bool,
    /// List of misreports that improve utility (should be empty for VCG).
    pub violations: Vec<MisreportViolation>,
}

/// A misreport that violates truthfulness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MisreportViolation {
    /// The misreported valuations.
    pub misreported: HashMap<String, f64>,
    /// Utility gain from misreporting.
    pub gain: f64,
}

/// Check that VCG is truthful for all agents.
pub fn is_vcg_truthful(reports: &[TruthfulnessReport]) -> bool {
    reports.iter().all(|r| r.is_truthful)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auction_space() -> OutcomeSpace {
        let mut space = OutcomeSpace::new();
        // Single-item auction: two outcomes
        space.add(Outcome::new("alice_wins"));
        space.add(Outcome::new("bob_wins"));
        space
    }

    fn simple_agents() -> Vec<AgentType> {
        vec![
            AgentType::with_valuations(
                "alice",
                HashMap::from([("alice_wins".into(), 20.0), ("bob_wins".into(), 0.0)]),
            ),
            AgentType::with_valuations(
                "bob",
                HashMap::from([("alice_wins".into(), 0.0), ("bob_wins".into(), 15.0)]),
            ),
        ]
    }

    #[test]
    fn test_vcg_selects_efficient_outcome() {
        let vcg = VCGMechanism::new("test_auction", auction_space());
        let result = vcg.run(&simple_agents());
        // alice_wins: 20+0=20, bob_wins: 0+15=15 → alice wins
        assert_eq!(result.outcome.as_ref().unwrap().label, "alice_wins");
    }

    #[test]
    fn test_vcg_clarke_payment() {
        let vcg = VCGMechanism::new("test_auction", auction_space());
        let agents = simple_agents();
        let result = vcg.run(&agents);
        // Alice's payment: SW without alice for bob_wins outcome = 15
        //                  SW without alice for alice_wins outcome = 0
        //                  p_alice = 15 - 0 = 15
        let alice_payment = result.payments["alice"];
        assert!((alice_payment - 15.0).abs() < 1e-9);
    }

    #[test]
    fn test_vcg_individual_rationality() {
        let vcg = VCGMechanism::new("test_auction", auction_space());
        let agents = simple_agents();
        let result = vcg.run(&agents);
        assert!(vcg.check_individual_rationality(&result, &agents));
    }

    #[test]
    fn test_vcg_budget_balance() {
        let vcg = VCGMechanism::new("test_auction", auction_space());
        let agents = simple_agents();
        let result = vcg.run(&agents);
        assert!(vcg.check_budget_balance(&result));
    }

    #[test]
    fn test_vcg_truthfulness() {
        let vcg = VCGMechanism::new("test_auction", auction_space());
        let agents = simple_agents();

        let misreports = vec![
            // Alice tries misreporting
            vec![
                HashMap::from([("alice_wins".into(), 10.0), ("bob_wins".into(), 0.0)]),
                HashMap::from([("alice_wins".into(), 30.0), ("bob_wins".into(), 0.0)]),
            ],
            // Bob tries misreporting
            vec![
                HashMap::from([("alice_wins".into(), 0.0), ("bob_wins".into(), 25.0)]),
                HashMap::from([("alice_wins".into(), 0.0), ("bob_wins".into(), 5.0)]),
            ],
        ];

        let reports = vcg.verify_truthfulness(&agents, &misreports);
        assert!(is_vcg_truthful(&reports));
    }

    #[test]
    fn test_vcg_public_good() {
        // Public good: build a bridge or don't
        let mut space = OutcomeSpace::new();
        space.add(Outcome::new("build"));
        space.add(Outcome::new("no_build"));

        let vcg = VCGMechanism::new("public_good", space);
        let agents = vec![
            AgentType::with_valuations(
                "town_a",
                HashMap::from([("build".into(), 60.0), ("no_build".into(), 0.0)]),
            ),
            AgentType::with_valuations(
                "town_b",
                HashMap::from([("build".into(), 50.0), ("no_build".into(), 0.0)]),
            ),
            AgentType::with_valuations(
                "town_c",
                HashMap::from([("build".into(), 10.0), ("no_build".into(), 0.0)]),
            ),
        ];

        let result = vcg.run(&agents);
        // build: 60+50+10=120, no_build: 0 → build wins
        assert_eq!(result.outcome.as_ref().unwrap().label, "build");
        assert!(vcg.check_individual_rationality(&result, &agents));
    }

    #[test]
    fn test_vcg_no_build_when_costly() {
        let mut space = OutcomeSpace::new();
        space.add(Outcome::new("build"));
        space.add(Outcome::new("no_build"));

        let vcg = VCGMechanism::new("public_good", space);
        let agents = vec![
            AgentType::with_valuations(
                "a",
                HashMap::from([("build".into(), 5.0), ("no_build".into(), 0.0)]),
            ),
            AgentType::with_valuations(
                "b",
                HashMap::from([("build".into(), 5.0), ("no_build".into(), 0.0)]),
            ),
            AgentType::with_valuations(
                "c",
                HashMap::from([("build".into(), -20.0), ("no_build".into(), 0.0)]),
            ),
        ];

        let result = vcg.run(&agents);
        // build: 5+5-20=-10, no_build: 0 → no_build wins
        assert_eq!(result.outcome.as_ref().unwrap().label, "no_build");
    }

    #[test]
    fn test_vcg_three_agents_truthfulness() {
        let mut space = OutcomeSpace::new();
        space.add(Outcome::new("build"));
        space.add(Outcome::new("no_build"));

        let vcg = VCGMechanism::new("pub3", space);
        let agents = vec![
            AgentType::with_valuations(
                "a",
                HashMap::from([("build".into(), 60.0), ("no_build".into(), 0.0)]),
            ),
            AgentType::with_valuations(
                "b",
                HashMap::from([("build".into(), 50.0), ("no_build".into(), 0.0)]),
            ),
            AgentType::with_valuations(
                "c",
                HashMap::from([("build".into(), -30.0), ("no_build".into(), 0.0)]),
            ),
        ];

        let misreports = vec![
            vec![HashMap::from([
                ("build".into(), 30.0),
                ("no_build".into(), 0.0),
            ])],
            vec![HashMap::from([
                ("build".into(), 100.0),
                ("no_build".into(), 0.0),
            ])],
            vec![HashMap::from([
                ("build".into(), 0.0),
                ("no_build".into(), 0.0),
            ])],
        ];

        let reports = vcg.verify_truthfulness(&agents, &misreports);
        assert!(is_vcg_truthful(&reports));
    }

    #[test]
    fn test_vcg_single_agent() {
        let mut space = OutcomeSpace::new();
        space.add(Outcome::new("win"));
        space.add(Outcome::new("lose"));

        let vcg = VCGMechanism::new("single", space);
        let agents = vec![AgentType::with_valuations(
            "only",
            HashMap::from([("win".into(), 100.0), ("lose".into(), 0.0)]),
        )];

        let result = vcg.run(&agents);
        assert_eq!(result.outcome.as_ref().unwrap().label, "win");
        // Single agent: no externality, payment = 0
        let payment = result.payments["only"];
        assert!(payment.abs() < 1e-9);
    }

    #[test]
    fn test_vcg_equal_valuations() {
        let mut space = OutcomeSpace::new();
        space.add(Outcome::new("a_wins"));
        space.add(Outcome::new("b_wins"));

        let vcg = VCGMechanism::new("tie", space);
        let agents = vec![
            AgentType::with_valuations(
                "a",
                HashMap::from([("a_wins".into(), 10.0), ("b_wins".into(), 0.0)]),
            ),
            AgentType::with_valuations(
                "b",
                HashMap::from([("a_wins".into(), 0.0), ("b_wins".into(), 10.0)]),
            ),
        ];

        let result = vcg.run(&agents);
        // Both outcomes have welfare 10; first one found wins
        assert!(result.outcome.is_some());
    }

    #[test]
    fn test_vcg_four_agent_auction() {
        let mut space = OutcomeSpace::new();
        space.add(Outcome::new("a_wins"));
        space.add(Outcome::new("b_wins"));
        space.add(Outcome::new("c_wins"));
        space.add(Outcome::new("d_wins"));

        let vcg = VCGMechanism::new("four_auction", space);
        let agents = vec![
            AgentType::with_valuations(
                "a",
                HashMap::from([
                    ("a_wins".into(), 100.0),
                    ("b_wins".into(), 0.0),
                    ("c_wins".into(), 0.0),
                    ("d_wins".into(), 0.0),
                ]),
            ),
            AgentType::with_valuations(
                "b",
                HashMap::from([
                    ("a_wins".into(), 0.0),
                    ("b_wins".into(), 80.0),
                    ("c_wins".into(), 0.0),
                    ("d_wins".into(), 0.0),
                ]),
            ),
            AgentType::with_valuations(
                "c",
                HashMap::from([
                    ("a_wins".into(), 0.0),
                    ("b_wins".into(), 0.0),
                    ("c_wins".into(), 60.0),
                    ("d_wins".into(), 0.0),
                ]),
            ),
            AgentType::with_valuations(
                "d",
                HashMap::from([
                    ("a_wins".into(), 0.0),
                    ("b_wins".into(), 0.0),
                    ("c_wins".into(), 0.0),
                    ("d_wins".into(), 40.0),
                ]),
            ),
        ];

        let result = vcg.run(&agents);
        assert_eq!(result.outcome.as_ref().unwrap().label, "a_wins");
        // a's payment = max other without a = 80 (b_wins) - 0 (a_wins others) = 80
        assert!((result.payments["a"] - 80.0).abs() < 1e-9);
    }

    #[test]
    fn test_vcg_truthfulness_aggressive_misreport() {
        let mut space = OutcomeSpace::new();
        space.add(Outcome::new("x"));
        space.add(Outcome::new("y"));

        let vcg = VCGMechanism::new("aggressive", space);
        let agents = vec![
            AgentType::with_valuations("a", HashMap::from([("x".into(), 50.0), ("y".into(), 0.0)])),
            AgentType::with_valuations("b", HashMap::from([("x".into(), 0.0), ("y".into(), 40.0)])),
        ];

        // Aggressive misreports: try to overbid and underbid
        let misreports = vec![
            vec![
                HashMap::from([("x".into(), 1.0), ("y".into(), 0.0)]),
                HashMap::from([("x".into(), 1000.0), ("y".into(), 0.0)]),
                HashMap::from([("x".into(), 50.0), ("y".into(), 1000.0)]),
            ],
            vec![
                HashMap::from([("x".into(), 0.0), ("y".into(), 1.0)]),
                HashMap::from([("x".into(), 0.0), ("y".into(), 1000.0)]),
            ],
        ];

        let reports = vcg.verify_truthfulness(&agents, &misreports);
        assert!(is_vcg_truthful(&reports));
    }

    #[test]
    fn test_vcg_budget_balance_surplus() {
        let mut space = OutcomeSpace::new();
        space.add(Outcome::new("a_wins"));
        space.add(Outcome::new("b_wins"));

        let vcg = VCGMechanism::new("surplus_test", space);
        let agents = vec![
            AgentType::with_valuations(
                "a",
                HashMap::from([("a_wins".into(), 100.0), ("b_wins".into(), 0.0)]),
            ),
            AgentType::with_valuations(
                "b",
                HashMap::from([("a_wins".into(), 0.0), ("b_wins".into(), 50.0)]),
            ),
        ];

        let result = vcg.run(&agents);
        // Total payments = a pays 50 + b pays 0 = 50 (non-negative)
        assert!(vcg.check_budget_balance(&result));
    }

    #[test]
    fn test_vcg_run_produces_result() {
        let mut space = OutcomeSpace::new();
        space.add(Outcome::new("win"));

        let vcg = VCGMechanism::new("trivial", space);
        let agents = vec![AgentType::with_valuations(
            "a",
            HashMap::from([("win".into(), 10.0)]),
        )];

        let result = vcg.run(&agents);
        assert_eq!(result.mechanism_name, "trivial");
        assert!(result.outcome.is_some());
    }
}
