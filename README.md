# mechanism-design

[![crates.io](https://img.shields.io/crates/v/mechanism-design.svg)](https://crates.io/crates/mechanism-design)
[![docs.rs](https://docs.rs/mechanism-design/badge.svg)](https://docs.rs/mechanism-design)
[![license](https://img.shields.io/crates/l/mechanism-design.svg)](https://github.com/SuperInstance/mechanism-design)

**Automated mechanism design in Rust: incentive-compatible auction theory, strategy-proof allocation, and Vickrey-Clarke-Groves — with the math to prove it.**

Mechanism design is the engineering side of game theory. Instead of analyzing
what happens when self-interested agents interact, you *design the rules* so
that selfish behavior leads to good outcomes. This crate gives you the tools
to construct, verify, and reason about such mechanisms — programmatically,
with types that encode the mathematical structure.

If you're building auction systems, resource allocation engines, matching
markets, voting systems, or multi-agent AI where agents have private
information, you need mechanism design. This crate makes it concrete.

---

## The Metaphor: Designing the Rules of the Game

Imagine you're running an auction. You have bidders who know how much they
value the item, but they won't tell you the truth unless it's in their
interest. You're not designing *what agents want* — you're designing the
*environment in which they act*.

Mechanism design is exactly this: **you set the rules, and the rules shape
incentives**. A well-designed mechanism aligns individual self-interest with
collective welfare. The VCG mechanism, for instance, makes truth-telling the
dominant strategy while selecting the socially optimal outcome.

This crate treats mechanisms as first-class objects. You don't just implement
an auction — you define a message space, an outcome function, and a payment
function. Then you *verify* that the mechanism is incentive compatible, that
no agent can profit by lying, that the outcome is Pareto efficient.

```
                    ┌─────────────────────────────────┐
                    │         mechanism-design         │
                    │  "The rules of the game"         │
                    └──────────┬──────────────────────┘
                               │
          ┌────────────────────┼────────────────────────┐
          │                    │                         │
    ┌─────▼─────┐     ┌───────▼───────┐     ┌──────────▼──────────┐
    │  outcome   │     │  mechanism    │     │   social_choice     │
    │            │     │               │     │                     │
    │ • Outcome  │     │ • AgentType   │     │ • Preference        │
    │ • Outcome  │     │ • Message     │     │ • PreferenceProfile │
    │   Space    │     │ • Mechanism   │     │ • Borda count       │
    │ • Pareto   │     │ • Mechanism   │     │ • Condorcet winner  │
    │ • Welfare  │     │   Result      │     │ • Approval voting   │
    └─────┬─────┘     └───────┬───────┘     │ • Arrow's theorem   │
          │                   │              └─────────────────────┘
          │           ┌───────┴───────┐
          │           │               │
          │    ┌──────▼──────┐  ┌─────▼──────────┐
          │    │  incentive  │  │ strategy_proof  │
          │    │             │  │                 │
          │    │ • DSIC      │  │ • Revelation    │
          │    │   check     │  │   principle     │
          │    │ • Grid      │  │ • Misreport     │
          │    │   search    │  │   verification  │
          │    └──────┬──────┘  └─────┬──────────┘
          │           │               │
          │           └───────┬───────┘
          │                   │
          │           ┌───────▼───────┐
          │           │     vcg       │
          │           │               │
          └──────────►│ • VCGMechanism│
                      │ • Clarke pivot│
                      │ • Budget      │
                      │   balance     │
                      │ • Individual  │
                      │   rationality │
                      │ • Truthfulness│
                      │   verification│
                      └───────────────┘
```

## Quick Start

```rust
use mechanism_design::{
    AgentType, Outcome, OutcomeSpace, VCGMechanism,
};
use std::collections::HashMap;

fn main() {
    // Define the outcome space for a single-item auction
    let mut space = OutcomeSpace::new();
    space.add(Outcome::new("alice_gets_item"));
    space.add(Outcome::new("bob_gets_item"));

    // Create the VCG mechanism
    let vcg = VCGMechanism::new("my_auction", space);

    // Define agents with their private valuations
    let agents = vec![
        AgentType::with_valuations(
            "alice",
            HashMap::from([
                ("alice_gets_item".into(), 42.0),
                ("bob_gets_item".into(), 0.0),
            ]),
        ),
        AgentType::with_valuations(
            "bob",
            HashMap::from([
                ("alice_gets_item".into(), 0.0),
                ("bob_gets_item".into(), 35.0),
            ]),
        ),
    ];

    // Run the mechanism — outcome + payments
    let result = vcg.run(&agents);

    println!("Winner: {}", result.outcome.as_ref().unwrap().label);
    // → alice_gets_item (higher social welfare)

    println!("Alice pays: {:.2}", result.payments["alice"]);
    // → 35.00 (Clarke pivot: Bob's valuation for the item)

    // Verify key properties
    assert!(vcg.check_individual_rationality(&result, &agents));
    assert!(vcg.check_budget_balance(&result));
}
```

## Social Choice

```rust
use mechanism_design::social_choice::{Preference, PreferenceProfile, ArrowProperties};

// The Condorcet Paradox: cyclic majority preferences
let profile = PreferenceProfile::new(
    vec![
        Preference::new("voter_1", vec!["a".into(), "b".into(), "c".into()]),
        Preference::new("voter_2", vec!["b".into(), "c".into(), "a".into()]),
        Preference::new("voter_3", vec!["c".into(), "a".into(), "b".into()]),
    ],
    vec!["a".into(), "b".into(), "c".into()],
);

// No Condorcet winner — a beats b, b beats c, c beats a
assert!(profile.condorcet_winner().is_none());

// Borda count breaks the tie (all tied at 3 points here)
let scores = profile.borda_count();

// Arrow's theorem: you can't have all three
let arrow = ArrowProperties {
    unanimity: true,
    iia: true,
    non_dictatorial: false, // dictatorship satisfies unanimity + IIA
};
assert!(arrow.theorem_holds()); // At least one must fail
```

## VCG Truthfulness Verification

```rust
use mechanism_design::{VCGMechanism, AgentType, Outcome, OutcomeSpace, is_vcg_truthful};
use std::collections::HashMap;

let mut space = OutcomeSpace::new();
space.add(Outcome::new("build_bridge"));
space.add(Outcome::new("no_bridge"));

let vcg = VCGMechanism::new("public_good", space);

let agents = vec![
    AgentType::with_valuations("town_north", HashMap::from([
        ("build_bridge".into(), 60.0), ("no_bridge".into(), 0.0),
    ])),
    AgentType::with_valuations("town_south", HashMap::from([
        ("build_bridge".into(), 50.0), ("no_bridge".into(), 0.0),
    ])),
    AgentType::with_valuations("town_far", HashMap::from([
        ("build_bridge".into(), -30.0), ("no_bridge".into(), 0.0),
    ])),
];

// Try various misreports — VCG should resist all of them
let misreports = vec![
    vec![HashMap::from([("build_bridge".into(), 30.0), ("no_bridge".into(), 0.0)])],
    vec![HashMap::from([("build_bridge".into(), 100.0), ("no_bridge".into(), 0.0)])],
    vec![HashMap::from([("build_bridge".into(), 0.0), ("no_bridge".into(), 0.0)])],
];

let reports = vcg.verify_truthfulness(&agents, &misreports);
assert!(is_vcg_truthful(&reports)); // VCG is strategy-proof!
```

## Module Reference

| Module | Purpose | Key Types |
|--------|---------|-----------|
| `outcome` | Outcome spaces, welfare, Pareto efficiency | `Outcome`, `OutcomeSpace` |
| `mechanism` | Core mechanism abstraction | `AgentType`, `Message`, `Mechanism`, `MechanismResult` |
| `incentive` | DSIC verification | `IncentiveCheck`, `check_dsic` |
| `strategy_proof` | Strategy-proofness, revelation principle | `StrategyProofResult`, `verify_revelation_principle` |
| `vcg` | VCG mechanism, Clarke pivot | `VCGMechanism`, `TruthfulnessReport` |
| `social_choice` | Voting theory, impossibility results | `Preference`, `PreferenceProfile`, `ArrowProperties` |

## Mathematical Foundations

### VCG Mechanism

Given agents 1, …, n with types θ₁, …, θₙ and outcome space Ω:

**Outcome selection:**
```
x*(θ) = argmax_{x ∈ Ω} Σᵢ vᵢ(x, θᵢ)
```

**Clarke pivot payment for agent i:**
```
pᵢ = Σⱼ≠ᵢ vⱼ(x*(θ₋ᵢ)) − Σⱼ≠ᵢ vⱼ(x*(θ))
```

Agent i pays the externality they impose on others — the difference between
others' welfare without them and others' welfare with them.

### Dominant Strategy Incentive Compatibility (DSIC)

A mechanism is DSIC if for every agent i:

```
uᵢ(x(θᵢ, θ₋ᵢ), θᵢ) ≥ uᵢ(x(θ̂ᵢ, θ₋ᵢ), θᵢ)    ∀θ̂ᵢ, ∀θ₋ᵢ
```

Truth-telling is a dominant strategy regardless of others' reports.

### Individual Rationality

```
uᵢ(x(θᵢ, θ₋ᵢ), θᵢ) ≥ 0    ∀θᵢ, ∀θ₋ᵢ
```

No agent is worse off participating than opting out.

### Arrow's Impossibility Theorem

For ≥ 3 alternatives, no social welfare function can simultaneously satisfy:

1. **Unanimity**: If all agents prefer a to b, society ranks a above b
2. **Independence of Irrelevant Alternatives (IIA)**: Social ranking of a vs b
   depends only on individual rankings of a vs b
3. **Non-dictatorship**: No single agent's preferences always determine the
   social ordering

### Revelation Principle

If a mechanism implements a social choice function in dominant strategies,
then the *direct* mechanism (where agents simply report their types) also
implements it in dominant strategies. This justifies restricting attention to
direct revelation mechanisms.

### Condorcet's Paradox

With ≥ 3 voters and ≥ 3 alternatives, majority voting can produce cyclic
preferences: a > b > c > a. No Condorcet winner exists, demonstrating that
pairwise majority voting is not a complete social choice rule.

## Design Decisions

### Why discrete outcome spaces?

Real mechanisms often have finite outcome spaces (who wins the auction, which
project gets funded). Discrete spaces admit exhaustive search over outcomes,
making welfare maximization tractable and verification straightforward.

### Why grid-based DSIC checking?

Proving DSIC analytically requires reasoning about all possible type profiles.
For numerical verification, we discretize the space and check a grid. This
catches violations without requiring symbolic computation. The grid approach
trades completeness for practicality — tighten the grid to increase coverage.

### Why `serde` as the only dependency?

Mechanism design is foundational infrastructure. It should be easy to
serialize types for logging, analysis, and API boundaries. Serde is the
standard choice. Beyond that, zero dependencies keeps the crate lightweight
and portable.

### Why separate `incentive` and `strategy_proof` modules?

DSIC (incentive compatibility) and strategy-proofness are formally equivalent
in the dominant-strategies setting, but they emphasize different aspects:

- **DSIC** focuses on the *definition*: truth-telling is optimal
- **Strategy-proofness** focuses on the *verification*: no misreporting helps

In practice, the verification techniques differ. `incentive` uses grid search
over misreports; `strategy_proof` compares specific truthful vs. misreported
utilities. Both converge on the same truth.

### Why public types everywhere?

Mechanism design is about transparency. The rules should be inspectable. All
types are public, all fields are public, and serialization is supported. If
you need to log exactly what a mechanism did and why, you can.

## Properties Verified

Running `cargo test` checks:

- **VCG selects the welfare-maximizing outcome** in auctions and public good
  settings
- **VCG Clarke payments equal the externality** imposed by each agent
- **VCG is truthful** — no agent benefits from misreporting (tested across
  multiple misreport profiles)
- **VCG satisfies individual rationality** — no agent gets negative utility
- **VCG satisfies weak budget balance** — total payments are non-negative
- **Condorcet paradox** produces no winner with cyclic preferences
- **Condorcet winner** is correctly identified when one exists
- **Borda count** computes correct scores and selects winners
- **Arrow's impossibility** — cannot satisfy all three conditions simultaneously
- **Pareto efficiency** correctly identifies dominated outcomes
- **Pareto frontier** returns all non-dominated outcomes
- **Strategy-proofness** detects violations when misreporting is beneficial
- **Revelation principle** verification works for direct mechanisms

## License

Dual-licensed under [MIT](./LICENSE-MIT) or [Apache 2.0](./LICENSE-APACHE).
