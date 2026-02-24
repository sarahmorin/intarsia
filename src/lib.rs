// Generic optimizer framework
pub mod framework;

// Build-time helpers (only available with "build-helpers" feature)
#[cfg(feature = "build-helpers")]
pub mod build;

// Macros for ISLE integration
#[macro_use]
pub mod macros;

// Re-export commonly used items
pub use framework::{
    CostFunction, CostResult, ExplorerHooks, OptimizerFramework,
    Property, PropertyAwareLanguage, Task,
};

extern crate egg;
