/// Generic optimizer framework module.
///
/// This module provides a generic cascades-style optimizer framework that can work
/// with any language and property system. It's designed to be used as a library
/// for building custom optimizers.
///
/// # Overview
///
/// The framework consists of several key components:
///
/// - **Property System** ([`property`]): Defines semantic properties of expressions
/// - **Language Extension** ([`language_ext`]): Extends egg's Language with property requirements
/// - **Cost Model** ([`cost`]): Defines cost computation interface
/// - **Task System** ([`task`]): Work units for cascades optimization
/// - **Explorer Hooks** ([`hooks`]): Integration point for rewrite rules
/// - **Optimizer** ([`optimizer`]): Main framework implementation
///
/// # Usage Pattern
///
/// To use this framework, you need to:
///
/// 1. Define your language using egg's `define_language!` macro
/// 2. Define your property type and implement [`Property`]
/// 3. Implement [`PropertyAwareLanguage`] for your language
/// 4. (Optionally) Create user data struct to hold domain-specific information
/// 5. Instantiate [`OptimizerFramework`] with your types
/// 6. Define a cost domain and implement [`CostDomain`] for it with respect to your properties
/// 6. Implement [`CostFunction`] to define cost computation in your cost domain
/// 7. Implement [`ExplorerHooks`] to integrate rewrite rules (e.g., ISLE)
/// 8. If using ISLE, implement the generated `Context` trait
///
/// # Example
///
/// ```rust,ignore
/// use egg::{define_language, Id};
/// use kymetica::framework::*;
///
/// // 1. Define language
/// define_language! {
///     pub enum MyLang {
///         Num(i64),
///         "+" = Add([Id; 2]),
///         // ... other operators
///     }
/// }
///
/// // 2. Define property
/// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// enum MyProperty {
///     Sorted,
///     Unsorted,
///     Bottom, // No requirement
/// }
///
/// impl Property for MyProperty {
///     fn satisfies(&self, required: &Self) -> bool {
///         // ... implement
///     }
///     fn bottom() -> Self { MyProperty::Bottom }
/// }
///
/// // 3. Implement PropertyAwareLanguage
/// impl PropertyAwareLanguage<MyProperty> for MyLang {
///     fn property_req(&self, child_index: usize) -> MyProperty {
///         // ... define requirements
///     }
/// }
///
/// // 4. Define user data
/// struct MyUserData {
///     // domain-specific data
/// }
///
/// // 5. Create type alias for convenience
/// type MyOptimizer = OptimizerFramework<MyLang, MyProperty, MyUserData>;
///
/// // 6-8. Implement traits on MyOptimizer
/// impl CostFunction<MyLang, MyProperty, SimpleCost<MyProperty>> for MyOptimizer { /* ... */ }
/// impl ExplorerHooks for MyOptimizer { /* ... */ }
/// // impl Context for MyOptimizer { /* ... */ } // if using ISLE
///
/// // Use the optimizer
/// let user_data = MyUserData { /* ... */ };
/// let mut optimizer = MyOptimizer::new(user_data);
/// let expr = /* build expression */;
/// let root_id = optimizer.init(expr);
/// optimizer.run(root_id);
/// let best_plan = optimizer.extract(root_id);
/// ```
pub mod cost;
pub mod hooks;
pub mod language_ext;
pub mod optimizer;
pub mod property;
pub mod task;

// Re-export main types for convenience
pub use cost::{CostDomain, CostFunction, SimpleCost};
pub use hooks::ExplorerHooks;
pub use language_ext::PropertyAwareLanguage;
pub use optimizer::{OptimizerFramework, SimpleOptimizerFramework};
pub use property::Property;
pub use task::Task;
