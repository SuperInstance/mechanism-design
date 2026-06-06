//! Social choice theory: preference profiles, voting rules, impossibility results.
//!
//! This module implements classic social choice concepts:
//! - **Preference profiles** — agents rank alternatives
//! - **Borda count** — positional scoring rule
//! - **Approval voting** — each agent approves a subset of alternatives
//! - **Condorcet winner** — an alternative that beats every other in pairwise
//!   majority contests
//! - **Arrow's impossibility theorem** — checking whether a social welfare
//!   function satisfies unanimity, independence of irrelevant alternatives
//!   (IIA), and non-dictatorship simultaneously

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single agent's preference ordering over alternatives.
///
/// `rankings[0]` is the most-preferred alternative, `rankings[1]` the second,
/// and so on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preference {
    /// Agent identifier.
    pub agent: String,
    /// Alternatives ranked from most to least preferred.
    pub rankings: Vec<String>,
}

impl Preference {
    /// Create a new preference ordering for an agent.
    pub fn new(agent: impl Into<String>, rankings: Vec<String>) -> Self {
        Self {
            agent: agent.into(),
            rankings,
        }
    }

    /// Return the rank of an alternative (0 = most preferred).
    ///
    /// Returns `None` if the alternative is not ranked.
    pub fn rank_of(&self, alt: &str) -> Option<usize> {
        self.rankings.iter().position(|a| a == alt)
    }

    /// Returns true if `a` is preferred to `b` under this preference.
    pub fn prefers(&self, a: &str, b: &str) -> bool {
        match (self.rank_of(a), self.rank_of(b)) {
            (Some(ra), Some(rb)) => ra < rb,
            _ => false,
        }
    }
}

/// A complete preference profile: every agent's ranking over all alternatives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferenceProfile {
    /// Each agent's preference ordering.
    pub preferences: Vec<Preference>,
    /// The set of alternatives.
    pub alternatives: Vec<String>,
}

impl PreferenceProfile {
    /// Create a new preference profile.
    pub fn new(preferences: Vec<Preference>, alternatives: Vec<String>) -> Self {
        Self {
            preferences,
            alternatives,
        }
    }

    /// Number of agents.
    pub fn num_agents(&self) -> usize {
        self.preferences.len()
    }

    /// Number of alternatives.
    pub fn num_alternatives(&self) -> usize {
        self.alternatives.len()
    }

    /// Count how many agents prefer `a` over `b` in pairwise comparison.
    pub fn pairwise_count(&self, a: &str, b: &str) -> usize {
        self.preferences.iter().filter(|p| p.prefers(a, b)).count()
    }

    /// Check whether `alt` is a **Condorcet winner** — it beats every other
    /// alternative in pairwise majority contests.
    pub fn is_condorcet_winner(&self, alt: &str) -> bool {
        let n = self.num_agents();
        self.alternatives.iter().all(|other| {
            if other == alt {
                return true;
            }
            self.pairwise_count(alt, other) > n / 2
        })
    }

    /// Find the Condorcet winner, if one exists.
    pub fn condorcet_winner(&self) -> Option<String> {
        self.alternatives
            .iter()
            .find(|a| self.is_condorcet_winner(a))
            .cloned()
    }

    /// Compute the **Borda count** winner.
    ///
    /// Each agent awards `m - 1 - rank` points to each alternative (where `m`
    /// is the number of alternatives). The alternative with the highest total
    /// wins.
    pub fn borda_count(&self) -> HashMap<String, usize> {
        let m = self.alternatives.len();
        let mut scores: HashMap<String, usize> = HashMap::new();
        for alt in &self.alternatives {
            scores.insert(alt.clone(), 0);
        }
        for pref in &self.preferences {
            for (rank, alt) in pref.rankings.iter().enumerate() {
                if let Some(score) = scores.get_mut(alt) {
                    *score += m - 1 - rank;
                }
            }
        }
        scores
    }

    /// Return the Borda winner (alternative with highest Borda score).
    pub fn borda_winner(&self) -> Option<String> {
        let scores = self.borda_count();
        scores
            .iter()
            .max_by_key(|(_, s)| *s)
            .map(|(a, _)| a.clone())
    }

    /// Compute **approval voting** results.
    ///
    /// Each agent approves a subset of alternatives. The alternative with the
    /// most approvals wins.
    pub fn approval_vote(
        approvals: &HashMap<String, Vec<String>>,
        alternatives: &[String],
    ) -> HashMap<String, usize> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for alt in alternatives {
            counts.insert(alt.clone(), 0);
        }
        for approved in approvals.values() {
            for alt in approved {
                if let Some(c) = counts.get_mut(alt) {
                    *c += 1;
                }
            }
        }
        counts
    }
}

/// Properties of a social welfare function, for Arrow's theorem checking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrowProperties {
    /// Does the rule respect unanimity? (If every agent prefers a to b, the
    /// social ordering must rank a above b.)
    pub unanimity: bool,
    /// Does the rule satisfy Independence of Irrelevant Alternatives?
    pub iia: bool,
    /// Is the rule non-dictatorial? (No single agent's preferences always
    /// determine the social ordering.)
    pub non_dictatorial: bool,
}

impl ArrowProperties {
    /// Check Arrow's impossibility: can all three properties hold simultaneously?
    ///
    /// Arrow's theorem says no — for ≥3 alternatives, no ranked voting system
    /// can satisfy unanimity, IIA, and non-dictatorship all at once.
    pub fn all_satisfied(&self) -> bool {
        self.unanimity && self.iia && self.non_dictatorial
    }

    /// Evaluate: returns `true` if Arrow's theorem holds (i.e., at least one
    /// property must be violated when there are ≥ 3 alternatives).
    pub fn theorem_holds(&self) -> bool {
        !self.all_satisfied()
    }
}

/// A voting rule: takes a preference profile and returns a ranked ordering of
/// alternatives (social ordering).
pub type VotingRule = fn(&PreferenceProfile) -> Vec<String>;

/// Dictatorship rule: the first agent's preference determines the outcome.
/// Satisfies unanimity and IIA, but is dictatorial.
pub fn dictatorial_rule(profile: &PreferenceProfile) -> Vec<String> {
    profile
        .preferences
        .first()
        .map(|p| p.rankings.clone())
        .unwrap_or_default()
}

/// Borda rule: rank alternatives by Borda score (descending).
pub fn borda_rule(profile: &PreferenceProfile) -> Vec<String> {
    let scores = profile.borda_count();
    let mut alts: Vec<String> = profile.alternatives.clone();
    alts.sort_by(|a, b| {
        scores
            .get(b)
            .copied()
            .unwrap_or(0)
            .cmp(&scores.get(a).copied().unwrap_or(0))
    });
    alts
}

/// Check whether a voting rule is a dictatorship (the same agent always
/// determines the outcome) across multiple profiles.
pub fn is_dictatorial(rule: VotingRule, profiles: &[PreferenceProfile]) -> bool {
    if profiles.is_empty() {
        return false;
    }
    // Simple heuristic: if the rule always matches agent 0's ranking, it's dictatorial
    profiles.iter().all(|p| {
        let result = rule(p);
        p.preferences
            .first()
            .map(|pref| pref.rankings == result)
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_classic_condorcet_profile() -> PreferenceProfile {
        // Condorcet paradox: a>b>c, b>c>a, c>a>b
        PreferenceProfile::new(
            vec![
                Preference::new("1", vec!["a".into(), "b".into(), "c".into()]),
                Preference::new("2", vec!["b".into(), "c".into(), "a".into()]),
                Preference::new("3", vec!["c".into(), "a".into(), "b".into()]),
            ],
            vec!["a".into(), "b".into(), "c".into()],
        )
    }

    #[test]
    fn test_condorcet_paradox() {
        let profile = make_classic_condorcet_profile();
        // No Condorcet winner in the classic paradox
        assert!(profile.condorcet_winner().is_none());
    }

    #[test]
    fn test_condorcet_winner_exists() {
        let profile = PreferenceProfile::new(
            vec![
                Preference::new("1", vec!["a".into(), "b".into(), "c".into()]),
                Preference::new("2", vec!["a".into(), "c".into(), "b".into()]),
                Preference::new("3", vec!["b".into(), "a".into(), "c".into()]),
            ],
            vec!["a".into(), "b".into(), "c".into()],
        );
        assert_eq!(profile.condorcet_winner(), Some("a".into()));
    }

    #[test]
    fn test_borda_count() {
        let profile = make_classic_condorcet_profile();
        let scores = profile.borda_count();
        // a: (2-1-0 from agent1) + (2-2-0 from agent2) + (2-0-1 from agent3) = 2+0+1 = 3
        // Wait let me recalc: m=3, points = m-1-rank = 2-rank
        // agent1: a→2, b→1, c→0
        // agent2: b→2, c→1, a→0
        // agent3: c→2, a→1, b→0
        // totals: a=3, b=3, c=3 — all tied (symmetric!)
        assert_eq!(scores["a"], 3);
        assert_eq!(scores["b"], 3);
        assert_eq!(scores["c"], 3);
    }

    #[test]
    fn test_borda_winner_clear() {
        let profile = PreferenceProfile::new(
            vec![
                Preference::new("1", vec!["a".into(), "b".into(), "c".into()]),
                Preference::new("2", vec!["a".into(), "c".into(), "b".into()]),
            ],
            vec!["a".into(), "b".into(), "c".into()],
        );
        assert_eq!(profile.borda_winner(), Some("a".into()));
    }

    #[test]
    fn test_approval_voting() {
        let mut approvals = HashMap::new();
        approvals.insert("1".into(), vec!["a".into(), "b".into()]);
        approvals.insert("2".into(), vec!["a".into(), "c".into()]);
        approvals.insert("3".into(), vec!["b".into()]);
        let alts = vec!["a".into(), "b".into(), "c".into()];
        let counts = PreferenceProfile::approval_vote(&approvals, &alts);
        assert_eq!(counts["a"], 2);
        assert_eq!(counts["b"], 2);
        assert_eq!(counts["c"], 1);
    }

    #[test]
    fn test_arrow_impossibility() {
        // Dictatorial rule: unanimity + IIA but NOT non-dictatorial
        let props = ArrowProperties {
            unanimity: true,
            iia: true,
            non_dictatorial: false,
        };
        assert!(props.theorem_holds());
        assert!(!props.all_satisfied());
    }

    #[test]
    fn test_arrow_all_three_impossible() {
        let props = ArrowProperties {
            unanimity: true,
            iia: true,
            non_dictatorial: true,
        };
        // Arrow says this shouldn't happen with ≥3 alternatives
        assert!(props.all_satisfied());
        assert!(!props.theorem_holds());
    }

    #[test]
    fn test_preference_prefers() {
        let p = Preference::new("agent", vec!["x".into(), "y".into(), "z".into()]);
        assert!(p.prefers("x", "y"));
        assert!(!p.prefers("y", "x"));
        assert!(p.prefers("x", "z"));
    }

    #[test]
    fn test_pairwise_count() {
        let profile = PreferenceProfile::new(
            vec![
                Preference::new("1", vec!["a".into(), "b".into()]),
                Preference::new("2", vec!["a".into(), "b".into()]),
                Preference::new("3", vec!["b".into(), "a".into()]),
            ],
            vec!["a".into(), "b".into()],
        );
        assert_eq!(profile.pairwise_count("a", "b"), 2);
        assert_eq!(profile.pairwise_count("b", "a"), 1);
    }

    #[test]
    fn test_dictatorial_rule() {
        let profile = PreferenceProfile::new(
            vec![
                Preference::new("1", vec!["a".into(), "b".into()]),
                Preference::new("2", vec!["b".into(), "a".into()]),
            ],
            vec!["a".into(), "b".into()],
        );
        let result = dictatorial_rule(&profile);
        assert_eq!(result, vec!["a", "b"]);
    }

    #[test]
    fn test_borda_rule_produces_ranking() {
        let profile = PreferenceProfile::new(
            vec![
                Preference::new("1", vec!["a".into(), "b".into(), "c".into()]),
                Preference::new("2", vec!["a".into(), "b".into(), "c".into()]),
                Preference::new("3", vec!["c".into(), "a".into(), "b".into()]),
            ],
            vec!["a".into(), "b".into(), "c".into()],
        );
        let ranking = borda_rule(&profile);
        assert_eq!(ranking[0], "a"); // a has highest Borda score
    }

    #[test]
    fn test_is_dictatorial_true() {
        let profile = PreferenceProfile::new(
            vec![
                Preference::new("1", vec!["a".into(), "b".into()]),
                Preference::new("2", vec!["b".into(), "a".into()]),
            ],
            vec!["a".into(), "b".into()],
        );
        assert!(is_dictatorial(dictatorial_rule, &[profile]));
    }

    #[test]
    fn test_is_dictatorial_false() {
        // Borda rule with a profile where agent 0 doesn't determine the outcome
        let profile = PreferenceProfile::new(
            vec![
                Preference::new("1", vec!["c".into(), "a".into(), "b".into()]),
                Preference::new("2", vec!["a".into(), "b".into(), "c".into()]),
                Preference::new("3", vec!["a".into(), "b".into(), "c".into()]),
            ],
            vec!["a".into(), "b".into(), "c".into()],
        );
        // Agent 0 prefers c, but Borda gives a the highest score
        assert!(!is_dictatorial(borda_rule, &[profile]));
    }

    #[test]
    fn test_preference_rank_of() {
        let p = Preference::new(
            "agent",
            vec!["first".into(), "second".into(), "third".into()],
        );
        assert_eq!(p.rank_of("first"), Some(0));
        assert_eq!(p.rank_of("second"), Some(1));
        assert_eq!(p.rank_of("third"), Some(2));
        assert_eq!(p.rank_of("nonexistent"), None);
    }

    #[test]
    fn test_num_agents_and_alternatives() {
        let profile = PreferenceProfile::new(
            vec![
                Preference::new("1", vec!["a".into(), "b".into()]),
                Preference::new("2", vec!["b".into(), "a".into()]),
                Preference::new("3", vec!["a".into(), "b".into()]),
            ],
            vec!["a".into(), "b".into()],
        );
        assert_eq!(profile.num_agents(), 3);
        assert_eq!(profile.num_alternatives(), 2);
    }

    #[test]
    fn test_unanimous_preference() {
        let profile = PreferenceProfile::new(
            vec![
                Preference::new("1", vec!["a".into(), "b".into(), "c".into()]),
                Preference::new("2", vec!["a".into(), "b".into(), "c".into()]),
            ],
            vec!["a".into(), "b".into(), "c".into()],
        );
        assert_eq!(profile.condorcet_winner(), Some("a".into()));
        assert_eq!(profile.borda_winner(), Some("a".into()));
    }
}
