//! Procedural macros for integrating [ISLE] DSL with Intarsia optimizers.
//!
//! This crate provides macros to help integrate ISLE-generated code into your Rust optimizer
//! framework. [ISLE] (Instruction Selection Lowering Expressions) is a domain-specific language
//! for pattern matching and rewriting, originally developed for [Cranelift].
//!
//! [ISLE]: https://github.com/bytecodealliance/wasmtime/blob/main/cranelift/isle/docs/language-reference.md
//! [Cranelift]: https://cranelift.dev/
//!
//! # Overview
//!
//! The macros fall into three categories:
//!
//! ## 1. Accessor Generation (Non-Multi)
//!
//! For basic ISLE operators that match/construct single nodes:
//! - [`isle_extractor!`] - Pattern matching (e-node → children IDs)
//! - [`isle_constructor!`] - Node construction (children IDs → e-node)
//! - [`isle_accessors!`] - Both extractor and constructor
//!
//! ## 2. Multi-Accessor Generation
//!
//! For ISLE multi-extractors/constructors that explore all nodes in an e-class:
//! - [`isle_multi_extractor!`] - Match all nodes in an e-class
//! - [`isle_multi_constructor!`] - Construct and add to results vector
//! - [`isle_multi_accessors!`] - Both multi-extractor and multi-constructor
//!
//! ## 3. Integration Setup
//!
//! - [`isle_integration!`] - Generate required type definitions
//! - [`isle_integration_full!`] - Module declaration + type definitions
//!
//! # Quick Start Example
//!
//! ```ignore
//! // In your optimizer module (e.g., src/optimizer.rs)
//! use intarsia_macros::{isle_integration, isle_accessors};
//!
//! // 1. Declare the ISLE-generated rules module
//! #[allow(dead_code, unused_variables, unused_imports, non_snake_case)]
//! #[path = "isle/rules.rs"]
//! pub(crate) mod rules;
//!
//! // 2. Generate required ISLE type definitions
//! isle_integration!();
//!
//! // 3. In your Context implementation, generate accessor functions
//! impl rules::Context for MyContext {
//!     isle_accessors! {
//!         And(extractor_and, constructor_and, 2);
//!         Or(extractor_or, constructor_or, 2);
//!         Not(extractor_not, constructor_not, 1);
//!     }
//!     // ... other required methods
//! }
//! ```
//!
//! # Build Script Integration
//!
//! Use [`intarsia-build`] to compile ISLE files in your `build.rs`:
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

use proc_macro::TokenStream;
use quote::quote;
use syn::{Error, Ident, LitInt, Token, parse::Parse, parse::ParseStream, parse_macro_input};

// Parser for isle_integration macro arguments
struct IsleIntegrationArgs {
    max_returns: Option<usize>,
}

impl Parse for IsleIntegrationArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(IsleIntegrationArgs { max_returns: None });
        }

        let key: Ident = input.parse()?;
        if key != "max_returns" {
            return Err(Error::new_spanned(key, "Expected 'max_returns'"));
        }
        let _colon: Token![:] = input.parse()?;
        let value: LitInt = input.parse()?;
        let max_returns = value.base10_parse()?;

        // Optional trailing comma
        let _ = input.parse::<Token![,]>();

        Ok(IsleIntegrationArgs {
            max_returns: Some(max_returns),
        })
    }
}

/// Generate type definitions required by ISLE-generated code.
///
/// This macro generates `ConstructorVec<T>` and `MAX_ISLE_RETURNS` which are referenced
/// by ISLE-generated multi-constructor functions. You still need to manually declare the
/// `rules` module with a [`#[path]`][path-attr] attribute.
///
/// See [`isle_integration_full!`] for a version that also declares the module.
///
/// [path-attr]: https://doc.rust-lang.org/reference/items/modules.html#the-path-attribute
///
/// # Arguments
///
/// * `max_returns: usize` - (Optional) Maximum number of values a multi-constructor can return.
///   Defaults to 100. Increase if you have rules generating many alternatives.
///
/// # Example
///
/// ```ignore
/// // In your optimizer module (e.g., src/optimizer/mod.rs)
/// use intarsia_macros::isle_integration;
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
/// isle_integration!(max_returns: 200);
/// ```
///
/// # Generated Code
///
/// ```ignore
/// /// Type alias for vectors returned by ISLE multi-constructors.
/// pub type ConstructorVec<T> = Vec<T>;
///
/// /// Maximum number of values an ISLE multi-constructor can return.
/// pub const MAX_ISLE_RETURNS: usize = 100;
/// ```
#[proc_macro]
pub fn isle_integration(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as IsleIntegrationArgs);
    let max_returns = args.max_returns.unwrap_or(100);

    let output = quote! {
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
        pub const MAX_ISLE_RETURNS: usize = #max_returns;
    };

    output.into()
}

// Parser for isle_integration_full macro arguments
struct IsleIntegrationFullArgs {
    path: String,
    max_returns: Option<usize>,
}

impl Parse for IsleIntegrationFullArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse "path"
        let path_key: Ident = input.parse()?;
        if path_key != "path" {
            return Err(Error::new_spanned(
                path_key,
                "Expected 'path' as first argument",
            ));
        }
        let _colon: Token![:] = input.parse()?;
        let path_lit: syn::LitStr = input.parse()?;
        let path = path_lit.value();

        // Optional comma
        let has_comma = input.parse::<Token![,]>().is_ok();

        let max_returns = if has_comma && !input.is_empty() {
            // Parse "max_returns"
            let max_returns_key: Ident = input.parse()?;
            if max_returns_key != "max_returns" {
                return Err(Error::new_spanned(
                    max_returns_key,
                    "Expected 'max_returns'",
                ));
            }
            let _colon: Token![:] = input.parse()?;
            let value: LitInt = input.parse()?;
            Some(value.base10_parse()?)
        } else {
            None
        };

        // Optional trailing comma
        let _ = input.parse::<Token![,]>();

        Ok(IsleIntegrationFullArgs { path, max_returns })
    }
}

/// Complete ISLE integration including module declaration and type definitions.
///
/// This is a convenience macro that combines module declaration (with appropriate `#[allow]`
/// attributes) and [`isle_integration!`] in a single invocation.
///
/// Use this for simpler integration, or use [`isle_integration!`] if you need custom
/// module attributes.
///
/// # Arguments
///
/// * `path: "..."` - Path to the generated `.rs` file, relative to current module
/// * `max_returns: usize` - (Optional) Maximum number of values a multi-constructor can return
///
/// # Example
///
/// ```ignore
/// use intarsia_macros::isle_integration_full;
///
/// // In your optimizer module, assuming isle/rules.rs exists
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
///
/// # Generated Code
///
/// This generates a `rules` module declaration with lint suppressions plus the type
/// definitions from [`isle_integration!`].
#[proc_macro]
pub fn isle_integration_full(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as IsleIntegrationFullArgs);
    let path = args.path;
    let name = path
        .split('/')
        .last()
        .and_then(|s| s.strip_suffix(".rs"))
        .unwrap_or("rules");
    let mod_name = Ident::new(name, proc_macro2::Span::call_site());
    let max_returns = args.max_returns.unwrap_or(100);

    let output = quote! {
        // Declare the rules module with appropriate attributes
        #[allow(dead_code, unused_variables, unused_imports, non_snake_case)]
        #[allow(irrefutable_let_patterns, unused_assignments, non_camel_case_types)]
        #[allow(unreachable_patterns, unreachable_code)]
        #[path = #path]
        pub(crate) mod #mod_name;

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
        pub const MAX_ISLE_RETURNS: usize = #max_returns;

        /// Import the context trait and wrapper type generated by ISLE.
        use #mod_name::{Context, ContextIterWrapper}; // Pull in the Context trait and ContextIterWrapper generated by ISLE.
    };

    output.into()
}
