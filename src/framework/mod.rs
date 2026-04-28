/// Generic optimizer framework module.
///
/// This module provides a cascades-style optimizer framework that operates over an egg
/// e-graph extended with a configurable e-class analysis. The framework is generic over
/// any analysis implementing [`OptimizableAnalysis`] — including the built-in
/// property-aware [`PropertyAnalysis`] and the property-free [`SimpleCostAnalysis`].
///
/// # Overview
///
/// - **Property System** ([`property`]): The base property lattice trait, used as the
///   raw input to the property mega-lattice analysis.
/// - **Analysis** ([`analysis`]): The `OptimizableAnalysis<L>` trait the optimizer
///   drives, plus the built-in `PropertyAnalysis` and `SimpleCostAnalysis` impls.
/// - **Task System** ([`task`]): Work units for cascades optimization.
/// - **Explorer Hooks** ([`hooks`]): Integration point for rewrite rules (e.g., ISLE).
/// - **Optimizer** ([`optimizer`]): Main framework implementation.
///
/// # Usage Pattern
///
/// 1. Define your language using egg's `define_language!` macro.
/// 2. (Property-aware) Define your property type and implement [`Property`]; create a
///    marker type and implement [`PropertyTransfer`] on it (replaces the old
///    `PropertyAwareLanguage` and `CostFunction` traits).
/// 2. (Property-free) Implement [`SimpleCostFn`] on a marker type.
/// 3. Optionally create a user-data struct.
/// 4. Instantiate the analysis (`PropertyAnalysis::new(transfer)` or
///    `SimpleCostAnalysis::new(cost_fn)`) and pass it to `OptimizerFramework::new(analysis, user_data)`.
/// 5. Implement [`ExplorerHooks`] to integrate rewrite rules.
/// 6. If using ISLE, implement the generated `Context` trait.
///
pub mod analysis;
pub mod config;
pub mod hooks;
pub mod optimizer;
pub mod property;
pub mod task;

// Re-export main types for convenience
pub use analysis::{
    Demand, OptimizableAnalysis, PropertyAnalysis, PropertyData, PropertyTransfer,
    SimpleCostAnalysis, SimpleCostData, SimpleCostFn, Winner,
};
pub use hooks::ExplorerHooks;
pub use optimizer::{OptimizerFramework, StopReason};
pub use property::{NoProperty, Property};
pub use task::Task;
