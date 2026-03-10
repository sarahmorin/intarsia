//! Intarsia: A Cascades-Style Optimizer Framework
//!
//! Intarsia is an extensible optimizer framework for building property-aware,
//! cost-based optimizers in Rust. It combines the power of [egg]'s e-graphs with
//! cascades-style optimization and [ISLE] DSL integration for declarative rewrite rules.
//!
//! [egg]: https://docs.rs/egg/
//! [ISLE]: https://github.com/bytecodealliance/wasmtime/blob/main/cranelift/isle/docs/language-reference.md
//!
//! # Key Features
//!
//! - **Property-Aware Optimization**: Track semantic properties (sorted, distributed, etc.)
//!   through the optimization process
//! - **Cascades-Style Search**: Explore equivalent expressions and select optimal plans
//! - **ISLE Integration**: Write rewrite rules in a declarative DSL
//! - **Generic Framework**: Works with any language defined using egg's `define_language!` macro
//! - **Extensible Cost Model**: Define custom cost functions for your domain
//!
//! # Quick Start
//!
//! ## 1. Define Your Language
//!
//! ```rust,ignore
//! use egg::{define_language, Id};
//!
//! define_language! {
//!     pub enum QueryLang {
//!         "scan" = Scan(i64),
//!         "filter" = Filter([Id; 2]),
//!         "join" = Join([Id; 2]),
//!     }
//! }
//! ```
//!
//! ## 2. Define Properties
//!
//! ```rust,ignore
//! use intarsia::Property;
//!
//! #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd)]
//! enum QueryProperty {
//!     Sorted,
//!     Unsorted,
//!     Bottom,
//! }
//!
//! impl Property for QueryProperty {
//!     fn satisfies(&self, required: &Self) -> bool {
//!         match required {
//!             QueryProperty::Bottom => true,
//!             QueryProperty::Unsorted => true,
//!             QueryProperty::Sorted => matches!(self, QueryProperty::Sorted),
//!         }
//!     }
//!
//!     fn bottom() -> Self {
//!         QueryProperty::Bottom
//!     }
//! }
//! ```
//!
//! ## 3. Implement Required Traits
//!
//! ```rust,ignore
//! use intarsia::{
//!     PropertyAwareLanguage, CostFunction, ExplorerHooks,
//!     SimpleCost, SimpleOptimizerFramework,
//! };
//!
//! impl PropertyAwareLanguage<QueryProperty> for QueryLang {
//!     fn property_req(&self, child_index: usize) -> QueryProperty {
//!         // Define property requirements for each operator
//!         QueryProperty::Bottom
//!     }
//! }
//!
//! type MyOptimizer = SimpleOptimizerFramework<QueryLang, QueryProperty>;
//!
//! impl CostFunction<QueryLang, QueryProperty, SimpleCost<QueryProperty>> for MyOptimizer {
//!     fn compute_cost<F>(&self, node: &QueryLang, mut get_child_cost: F) -> SimpleCost<QueryProperty>
//!     where
//!         F: FnMut(egg::Id) -> SimpleCost<QueryProperty>,
//!     {
//!         // Compute cost based on operator and child costs
//!         SimpleCost::simple(1)
//!     }
//! }
//!
//! impl ExplorerHooks<QueryLang> for MyOptimizer {
//!     fn explore(&mut self, id: egg::Id) -> Vec<egg::Id> {
//!         // Apply rewrite rules (typically ISLE-generated)
//!         vec![]
//!     }
//! }
//! ```
//!
//! ## 4. Run Optimization
//!
//! ```rust,ignore
//! // Create optimizer
//! let mut optimizer = MyOptimizer::new(());
//!
//! // Add initial expression
//! let expr = /* build your expression */;
//! let root = optimizer.init(expr);
//!
//! // Run optimization
//! optimizer.run(root);
//!
//! // Extract best plan
//! let best_plan = optimizer.extract(root);
//! ```
//!
//! # ISLE Integration
//!
//! Use [`intarsia-build`] in your `build.rs` to compile ISLE rules:
//!
//! [`intarsia-build`]: https://docs.rs/intarsia-build/
//!
//! ```no_run
//! // build.rs
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     intarsia_build::compile_isle_auto()?;
//!     Ok(())
//! }
//! ```
//!
//! Then use [`intarsia-macros`] to integrate the generated code:
//!
//! [`intarsia-macros`]: https://docs.rs/intarsia-macros/
//!
//! ```rust,ignore
//! use intarsia_macros::{isle_integration, isle_accessors};
//!
//! isle_integration!();
//!
//! impl rules::Context for MyOptimizer {
//!     isle_accessors! {
//!         Filter(extractor_filter, constructor_filter, 2);
//!         Join(extractor_join, constructor_join, 2);
//!     }
//! }
//! ```
//!
//! # Feature Flags
//!
//! - `build-helpers`: Re-exports build script utilities (from `intarsia-build`)
//!
//! # Further Reading
//!
//! - [Examples](https://github.com/sarahmorin/intarsia/tree/main/examples) - Database and boolean optimizers
//! - [Documentation](https://github.com/sarahmorin/intarsia/wiki) - Detailed guides and API reference

// Generic optimizer framework
pub mod framework;

// Re-export commonly used items from framework
pub use framework::{
    CostDomain, CostFunction, ExplorerHooks, OptimizerFramework, Property, PropertyAwareLanguage,
    SimpleCost, SimpleOptimizerFramework, Task,
};
