# Kymetica Optimizer Framework (until someone comes up with a better name)
An extensible optimizer framework based on ISLE and egg.

Given a set of rewrite rules (written in ISLE) and the corresponding language definition in Rust,
we build an optimizer that explores all possible rewrites (up to some work budget) and stores equivalent
terms in an egg e-graph. 

> README is a work in progress, check back later for more details.

## v0 Design and Features

- Declarative rewrites in ISLE (following a specific formatted)
- Rust macros to easily integrate compiled ISLE with the optimizer e-graph/search modules
- Support for any custom language defined as a Rust enum
- Cascades style optimization strategy


## Future Features

- A new rewrite DSL that naturally integrates with the Rust optimizer module once compiled (no more manual integration with macros)
- Better support for properties with a multe-graph
- Programmable search strategies
- Fine-grained cost-based bounding in search
