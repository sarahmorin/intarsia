// Generic optimizer framework
pub mod framework;

// Build-time helpers (only available with "build-helpers" feature)
#[cfg(feature = "build-helpers")]
pub mod build;

// Re-export commonly used items
pub use framework::{
    CostDomain, CostFunction, ExplorerHooks, OptimizerFramework, Property, PropertyAwareLanguage,
    SimpleCost, Task,
};
