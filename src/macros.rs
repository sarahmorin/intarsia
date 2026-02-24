//! Macros for integrating ISLE-generated code with the optimizer framework.

/// Integrate ISLE-generated code into your optimizer.
///
/// This macro generates the type definitions required by ISLE (`ConstructorVec`, `MAX_ISLE_RETURNS`).
/// You still need to manually declare the `rules` module with `#[path]` attribute.
///
/// # Arguments
///
/// * `max_returns` - (Optional) Maximum number of values a multiconstructor can return.
///   Defaults to 100.
///
/// # Example
///
/// ```ignore
/// // In your optimizer module (e.g., examples/my_optimizer/mod.rs)
/// use kymetica::isle_integration;
///
/// // First, declare the rules module with the path to generated code
/// #[allow(dead_code, unused_variables, unused_imports, non_snake_case)]
/// #[allow(irrefutable_let_patterns, unused_assignments, non_camel_case_types)]
/// #[allow(unreachable_patterns, unreachable_code)]
/// #[path = "isle/rules.rs"]
/// pub(crate) mod rules;
///
/// // Then, generate the required type definitions
/// isle_integration!();
/// 
/// // Or with custom max_returns:
/// // isle_integration!(max_returns: 200);
/// ```
///
/// # What This Macro Generates
///
/// ```ignore
/// pub type ConstructorVec<T> = Vec<T>;
/// pub const MAX_ISLE_RETURNS: usize = 100;
/// ```
///
/// These types are required by ISLE-generated multi-constructor functions.
#[macro_export]
macro_rules! isle_integration {
    // With max_returns specified
    (
        max_returns: $max_returns:expr $(,)?
    ) => {
        /// Type alias for vectors returned by ISLE multi-constructors.
        ///
        /// ISLE multi-constructors can return multiple values. This type
        /// is used internally by the generated code.
        pub type ConstructorVec<T> = Vec<T>;

        /// Maximum number of values an ISLE multi-constructor can return.
        ///
        /// This constant is used by ISLE-generated code to limit the number
        /// of alternatives explored. Increase this if you have rules that
        /// generate many alternatives.
        pub const MAX_ISLE_RETURNS: usize = $max_returns;
    };

    // Default max_returns = 100
    () => {
        $crate::isle_integration!(max_returns: 100);
    };
}

/// Complete ISLE integration including module declaration.
///
/// This macro generates both the module declaration and type definitions.
/// It's more convenient than `isle_integration!` but requires the path to be
/// relative to the current module.
///
/// # Arguments
///
/// * `path` - Path to the generated `.rs` file, relative to current module
/// * `max_returns` - (Optional) Maximum number of values a multiconstructor can return
///
/// # Example
///
/// ```ignore
/// use kymetica::isle_integration_full;
///
/// // Assuming isle/rules.rs exists relative to current module
/// isle_integration_full! {
///     path: "isle/rules.rs",
/// }
///
/// // With custom max_returns
/// isle_integration_full! {
///     path: "isle/rules.rs",
///     max_returns: 200,
/// }
/// ```
#[macro_export]
macro_rules! isle_integration_full {
    (
        path: $module_path:literal,
        max_returns: $max_returns:expr $(,)?
    ) => {
        // Declare the rules module with appropriate attributes
        #[allow(dead_code, unused_variables, unused_imports, non_snake_case)]
        #[allow(irrefutable_let_patterns, unused_assignments, non_camel_case_types)]
        #[allow(unreachable_patterns, unreachable_code)]
        #[path = $module_path]
        pub(crate) mod rules;

        // Generate type definitions
        $crate::isle_integration!(max_returns: $max_returns);
    };

    (
        path: $module_path:literal $(,)?
    ) => {
        $crate::isle_integration_full!(path: $module_path, max_returns: 100);
    };
}

#[cfg(test)]
mod tests {
    // Test that macros compile (not that they work correctly, as that requires ISLE files)
    #[test]
    fn test_macro_compilation() {
        // This test just ensures the macro syntax is valid
        // We can't actually test functionality without real ISLE files
    }
}
