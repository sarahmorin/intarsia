//! # ISLE Integration Macros
//!
//! This crate provides macros to help integrate ISLE-generated code into your Rust optimizer framework.
//! The main macros are:
//! - `isle_extractor!`: Generates extractor functions for ISLE operators.
//! - `isle_constructor!`: Generates constructor functions for ISLE operators.
//! - `isle_accessors!`: Generates both extractors and constructors for ISLE operators
//! - `isle_integration!`: Generates type definitions required by ISLE.
//! - `isle_integration_full!`: Combines module declaration and type definitions for ISLE integration.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Error, Ident, LitInt, Token, parse::Parse, parse::ParseStream, parse_macro_input};

/// Parsed item for isle_extractor macro
struct ExtractorItem {
    _fn_token: Token![fn],
    fn_name: Ident,
    _paren_token: syn::token::Paren,
    variant: syn::Path,
    _comma1: Token![,],
    arity: LitInt,
    _semi: Token![;],
}

impl Parse for ExtractorItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(ExtractorItem {
            _fn_token: input.parse()?,
            fn_name: input.parse()?,
            _paren_token: syn::parenthesized!(content in input),
            variant: content.parse()?,
            _comma1: content.parse()?,
            arity: content.parse()?,
            _semi: input.parse()?,
        })
    }
}

/// Parsed list of extractor items
struct ExtractorList {
    items: Vec<ExtractorItem>,
}

impl Parse for ExtractorList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut items = Vec::new();
        while !input.is_empty() {
            items.push(input.parse()?);
        }
        Ok(ExtractorList { items })
    }
}

/// Generate extractor functions for ISLE-integrated operators.
///
/// # Syntax
/// ```ignore
/// isle_extractor! {
///     fn extractor_or(Or, 2);
///     fn extractor_not(Not, 1);
///     fn extractor_join(Join, 3);
/// }
/// ```
#[proc_macro]
pub fn isle_extractor(input: TokenStream) -> TokenStream {
    let ExtractorList { items } = parse_macro_input!(input as ExtractorList);

    let functions = items.iter().map(|item| {
        let fn_name = &item.fn_name;
        let variant = &item.variant;
        let arity: usize = item
            .arity
            .base10_parse()
            .expect("Arity must be a positive integer");

        if arity == 0 {
            return Error::new_spanned(&item.arity, "Arity must be at least 1").to_compile_error();
        }

        if arity == 1 {
            // Single child case: Variant(id)
            quote! {
                fn #fn_name(&mut self, arg0: egg::Id) -> Option<egg::Id> {
                    let node = self.egraph.get_node(arg0);
                    if let #variant(id) = node {
                        // Canonicalize the ID so pattern matching works with e-class equality
                        Some(self.egraph.find(*id))
                    } else {
                        None
                    }
                }
            }
        } else {
            // Multiple children case: Variant([id1, id2, ...])
            let id_names: Vec<_> = (1..=arity)
                .map(|i| Ident::new(&format!("id{}", i), proc_macro2::Span::call_site()))
                .collect();

            let canonical_ids = id_names.iter().map(|id| quote! { self.egraph.find(*#id) });

            // Generate tuple type: (egg::Id, egg::Id, ...)
            let id_types = (0..arity).map(|_| quote! { egg::Id });

            quote! {
                fn #fn_name(&mut self, arg0: egg::Id) -> Option<(#(#id_types),*)> {
                    let node = self.egraph.get_node(arg0);
                    if let #variant([#(#id_names),*]) = node {
                        // Canonicalize all IDs so pattern matching works with e-class equality
                        Some((#(#canonical_ids),*))
                    } else {
                        None
                    }
                }
            }
        }
    });

    let output = quote! {
        #(#functions)*
    };

    output.into()
}

/// Parsed item for isle_constructor macro
struct ConstructorItem {
    _fn_token: Token![fn],
    fn_name: Ident,
    _paren_token: syn::token::Paren,
    variant: syn::Path,
    _comma1: Token![,],
    arity: LitInt,
    _semi: Token![;],
}

impl Parse for ConstructorItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(ConstructorItem {
            _fn_token: input.parse()?,
            fn_name: input.parse()?,
            _paren_token: syn::parenthesized!(content in input),
            variant: content.parse()?,
            _comma1: content.parse()?,
            arity: content.parse()?,
            _semi: input.parse()?,
        })
    }
}

/// Parsed list of constructor items
struct ConstructorList {
    items: Vec<ConstructorItem>,
}

impl Parse for ConstructorList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut items = Vec::new();
        while !input.is_empty() {
            items.push(input.parse()?);
        }
        Ok(ConstructorList { items })
    }
}

/// Generate constructor functions for ISLE-integrated operators.
///
/// # Syntax
/// ```ignore
/// isle_constructor! {
///     fn constructor_or(Or, 2);
///     fn constructor_not(Not, 1);
///     fn constructor_join(Join, 3);
/// }
/// ```
#[proc_macro]
pub fn isle_constructor(input: TokenStream) -> TokenStream {
    let ConstructorList { items } = parse_macro_input!(input as ConstructorList);

    let functions = items.iter().map(|item| {
        let fn_name = &item.fn_name;
        let variant = &item.variant;
        let arity: usize = item
            .arity
            .base10_parse()
            .expect("Arity must be a positive integer");

        if arity == 0 {
            return Error::new_spanned(&item.arity, "Arity must be at least 1").to_compile_error();
        }

        // Generate parameter names: arg0, arg1, arg2, ...
        let arg_names: Vec<_> = (0..arity)
            .map(|i| Ident::new(&format!("arg{}", i), proc_macro2::Span::call_site()))
            .collect();

        if arity == 1 {
            // Single child case: Variant(arg0)
            let arg0 = &arg_names[0];
            quote! {
                fn #fn_name(&mut self, #arg0: egg::Id) -> egg::Id {
                    let (id, is_new) = self.egraph.add_with_flag(#variant(#arg0));
                    if is_new {
                        self.push_task(Task::ExploreExpr(id, false));
                    }
                    id
                }
            }
        } else {
            // Multiple children case: Variant([arg0, arg1, ...])
            let params = arg_names.iter().map(|arg| quote! { #arg: egg::Id });
            let args = &arg_names;

            quote! {
                fn #fn_name(&mut self, #(#params),*) -> egg::Id {
                    let (id, is_new) = self.egraph.add_with_flag(#variant([#(#args),*]));
                    if is_new {
                        self.push_task(Task::ExploreExpr(id, false));
                    }
                    id
                }
            }
        }
    });

    let output = quote! {
        #(#functions)*
    };

    output.into()
}

/// Parsed item for isle_accessors macro
struct AccessorItem {
    variant: syn::Path,
    _paren_token: syn::token::Paren,
    extractor: Ident,
    _comma1: Token![,],
    constructor: Ident,
    _comma2: Token![,],
    arity: LitInt,
    _semi: Token![;],
}

impl Parse for AccessorItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(AccessorItem {
            variant: input.parse()?,
            _paren_token: syn::parenthesized!(content in input),
            extractor: content.parse()?,
            _comma1: content.parse()?,
            constructor: content.parse()?,
            _comma2: content.parse()?,
            arity: content.parse()?,
            _semi: input.parse()?,
        })
    }
}

/// Parsed list of accessor items
struct AccessorList {
    items: Vec<AccessorItem>,
}

impl Parse for AccessorList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut items = Vec::new();
        while !input.is_empty() {
            items.push(input.parse()?);
        }
        Ok(AccessorList { items })
    }
}

/// Generate both extractor and constructor functions for ISLE-integrated operators.
///
/// # Syntax
/// ```ignore
/// isle_accessors! {
///     Or(extractor_or, constructor_or, 2);
///     Not(extractor_not, constructor_not, 1);
///     Join(extractor_join, constructor_join, 3);
/// }
/// ```
#[proc_macro]
pub fn isle_accessors(input: TokenStream) -> TokenStream {
    let AccessorList { items } = parse_macro_input!(input as AccessorList);

    let functions = items.iter().flat_map(|item| {
        let extractor_name = &item.extractor;
        let constructor_name = &item.constructor;
        let variant = &item.variant;
        let arity: usize = item
            .arity
            .base10_parse()
            .expect("Arity must be a positive integer");

        if arity == 0 {
            let err =
                Error::new_spanned(&item.arity, "Arity must be at least 1").to_compile_error();
            return vec![err.clone(), err];
        }

        let extractor = if arity == 1 {
            quote! {
                fn #extractor_name(&mut self, arg0: egg::Id) -> Option<egg::Id> {
                    let node = self.egraph.get_node(arg0);
                    if let #variant(id) = node {
                        // Canonicalize the ID so pattern matching works with e-class equality
                        Some(self.egraph.find(*id))
                    } else {
                        None
                    }
                }
            }
        } else {
            let id_names: Vec<_> = (1..=arity)
                .map(|i| Ident::new(&format!("id{}", i), proc_macro2::Span::call_site()))
                .collect();
            let canonical_ids = id_names.iter().map(|id| quote! { self.egraph.find(*#id) });

            // Generate tuple type: (egg::Id, egg::Id, ...)
            let id_types = (0..arity).map(|_| quote! { egg::Id });

            quote! {
                fn #extractor_name(&mut self, arg0: egg::Id) -> Option<(#(#id_types),*)> {
                    let node = self.egraph.get_node(arg0);
                    if let #variant([#(#id_names),*]) = node {
                        // Canonicalize all IDs so pattern matching works with e-class equality
                        Some((#(#canonical_ids),*))
                    } else {
                        None
                    }
                }
            }
        };

        let arg_names: Vec<_> = (0..arity)
            .map(|i| Ident::new(&format!("arg{}", i), proc_macro2::Span::call_site()))
            .collect();

        let constructor = if arity == 1 {
            let arg0 = &arg_names[0];
            quote! {
                fn #constructor_name(&mut self, #arg0: egg::Id) -> egg::Id {
                    let (id, is_new) = self.egraph.add_with_flag(#variant(#arg0));
                    if is_new {
                        self.push_task(Task::ExploreExpr(id, false));
                    }
                    id
                }
            }
        } else {
            let params = arg_names.iter().map(|arg| quote! { #arg: egg::Id });
            let args = &arg_names;

            quote! {
                fn #constructor_name(&mut self, #(#params),*) -> egg::Id {
                    let (id, is_new) = self.egraph.add_with_flag(#variant([#(#args),*]));
                    if is_new {
                        self.push_task(Task::ExploreExpr(id, false));
                    }
                    id
                }
            }
        };

        vec![extractor, constructor]
    });

    let output = quote! {
        #(#functions)*
    };

    output.into()
}

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
/// use intarsia::isle_integration;
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
            return Err(Error::new_spanned(path_key, "Expected 'path' as first argument"));
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
                return Err(Error::new_spanned(max_returns_key, "Expected 'max_returns'"));
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
/// use intarsia::isle_integration_full;
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
#[proc_macro]
pub fn isle_integration_full(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as IsleIntegrationFullArgs);
    let path = args.path;
    let max_returns = args.max_returns.unwrap_or(100);

    let output = quote! {
        // Declare the rules module with appropriate attributes
        #[allow(dead_code, unused_variables, unused_imports, non_snake_case)]
        #[allow(irrefutable_let_patterns, unused_assignments, non_camel_case_types)]
        #[allow(unreachable_patterns, unreachable_code)]
        #[path = #path]
        pub(crate) mod rules;

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
