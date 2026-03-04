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
/// Extractors are used by ISLE for pattern matching. They check if an [`egg::Id`]
/// refers to a specific e-node variant and extract its children.
///
/// For **non-multi** extractors, only the canonical node in the e-class is checked.
/// Use [`isle_multi_extractor!`] to check all nodes in an e-class.
///
/// [`egg::Id`]: https://docs.rs/egg/latest/egg/struct.Id.html
///
/// # Syntax
///
/// ```ignore
/// isle_extractor! {
///     fn extractor_name(VariantName, arity);
///     // ...
/// }
/// ```
///
/// # Example
///
/// ```ignore
/// impl rules::Context for MyContext {
///     isle_extractor! {
///         fn extractor_or(Or, 2);   // Binary operator
///         fn extractor_not(Not, 1); // Unary operator
///     }
/// }
/// ```
///
/// This generates:
/// - `fn extractor_or(&mut self, arg0: egg::Id) -> Option<(egg::Id, egg::Id)>`
/// - `fn extractor_not(&mut self, arg0: egg::Id) -> Option<egg::Id>`
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

        // Non-multi extractor: returns Option, checks only canonical node
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

/// Generate multi-extractor functions for ISLE-integrated operators.
///
/// Multi-extractors check **all nodes** in an e-class (not just the canonical node)
/// and return all matches. This is used by ISLE for exhaustive pattern matching.
///
/// See [`isle_extractor!`] for the non-multi version.
///
/// # Syntax
///
/// ```ignore
/// isle_multi_extractor! {
///     fn extractor_name(VariantName, arity);
///     // ...
/// }
/// ```
///
/// # Example
///
/// ```ignore
/// impl rules::Context for MyContext {
///     isle_multi_extractor! {
///         fn extractor_or(Or, 2);   // Returns Vec<(egg::Id, egg::Id)>
///         fn extractor_not(Not, 1); // Returns Vec<egg::Id>
///     }
/// }
/// ```
#[proc_macro]
pub fn isle_multi_extractor(input: TokenStream) -> TokenStream {
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

        // Multi-extractor: returns Vec of all matching nodes in the e-class
        if arity == 1 {
            // Single child case: Variant(id)
            quote! {
                fn #fn_name(&mut self, arg0: egg::Id) -> Vec<egg::Id> {
                    let eclass = self.egraph.find(arg0);
                    let mut results = Vec::new();
                    for (_node_id, node) in self.egraph.nodes_in_class(eclass) {
                        if let #variant(id) = node {
                            // Canonicalize the ID so pattern matching works with e-class equality
                            results.push(self.egraph.find(*id));
                        }
                    }
                    results
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
                fn #fn_name(&mut self, arg0: egg::Id) -> Vec<(#(#id_types),*)> {
                    let eclass = self.egraph.find(arg0);
                    let mut results = Vec::new();
                    for (_node_id, node) in self.egraph.nodes_in_class(eclass) {
                        if let #variant([#(#id_names),*]) = node {
                            // Canonicalize all IDs so pattern matching works with e-class equality
                            results.push((#(#canonical_ids),*));
                        }
                    }
                    results
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
/// Constructors are used by ISLE to build new e-nodes. They take child [`egg::Id`]s
/// as arguments and return the ID of the constructed node.
///
/// For **non-multi** constructors, the function returns a single [`egg::Id`].
/// Use [`isle_multi_constructor!`] for the multi-constructor protocol.
///
/// [`egg::Id`]: https://docs.rs/egg/latest/egg/struct.Id.html
///
/// # Syntax
///
/// ```ignore
/// isle_constructor! {
///     fn constructor_name(VariantName, arity);
///     // ...
/// }
/// ```
///
/// # Example
///
/// ```ignore
/// impl rules::Context for MyContext {
///     isle_constructor! {
///         fn constructor_or(Or, 2);   // Binary operator
///         fn constructor_not(Not, 1); // Unary operator
///     }
/// }
/// ```
///
/// This generates:
/// - `fn constructor_or(&mut self, arg0: egg::Id, arg1: egg::Id) -> egg::Id`
/// - `fn constructor_not(&mut self, arg0: egg::Id) -> egg::Id`
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

        // Non-multi constructor: returns Id directly
        if arity == 1 {
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

/// Generate multi-constructor functions for ISLE-integrated operators.
///
/// Multi-constructors use ISLE's multi-constructor protocol, which allows a rule
/// to produce multiple results. The function extends a `returns` parameter instead
/// of returning a value directly.
///
/// See [`isle_constructor!`] for the non-multi version.
///
/// # Syntax
///
/// ```ignore
/// isle_multi_constructor! {
///     fn constructor_name(VariantName, arity);
///     // ...
/// }
/// ```
///
/// # Example
///
/// ```ignore
/// impl rules::Context for MyContext {
///     isle_multi_constructor! {
///         fn constructor_or(Or, 2);
///         fn constructor_not(Not, 1);
///     }
/// }
/// ```
///
/// This generates functions with a `returns` parameter:
/// - `fn constructor_or(&mut self, arg0: egg::Id, arg1: egg::Id, returns: &mut impl Extend<egg::Id>)`
#[proc_macro]
pub fn isle_multi_constructor(input: TokenStream) -> TokenStream {
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

        // Multi-constructor: takes a returns parameter and extends it
        if arity == 1 {
            let arg0 = &arg_names[0];
            quote! {
                fn #fn_name(&mut self, #arg0: egg::Id, returns: &mut (impl Extend<egg::Id> + Length)) -> () {
                    let (id, is_new) = self.egraph.add_with_flag(#variant(#arg0));
                    if is_new {
                        self.push_task(Task::ExploreChildren(id));
                    }
                    returns.extend(Some(id));
                }
            }
        } else {
            let params = arg_names.iter().map(|arg| quote! { #arg: egg::Id });
            let args = &arg_names;

            quote! {
                fn #fn_name(&mut self, #(#params),*, returns: &mut (impl Extend<egg::Id> + Length)) -> () {
                    let (id, is_new) = self.egraph.add_with_flag(#variant([#(#args),*]));
                    if is_new {
                        self.push_task(Task::ExploreChildren(id));
                    }
                    returns.extend(Some(id));
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
/// This is a convenience macro that combines [`isle_extractor!`] and [`isle_constructor!`]
/// in a single invocation. Note the different syntax: variant comes first, followed by
/// both function names.
///
/// # Syntax
///
/// ```ignore
/// isle_accessors! {
///     VariantName(extractor_name, constructor_name, arity);
///     // ...
/// }
/// ```
///
/// # Example
///
/// ```ignore
/// impl rules::Context for MyContext {
///     isle_accessors! {
///         Or(extractor_or, constructor_or, 2);
///         Not(extractor_not, constructor_not, 1);
///         Constant(extractor_constant, constructor_constant, 1);
///     }
/// }
/// ```
///
/// This is equivalent to calling both [`isle_extractor!`] and [`isle_constructor!`]
/// with the same variants.
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

        // Non-multi versions
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
                        self.push_task(Task::ExploreChildren(id));
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
                        self.push_task(Task::ExploreChildren(id));
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

/// Generate both extractor and constructor functions for multi ISLE-integrated operators.
///
/// This combines [`isle_multi_extractor!`] and [`isle_multi_constructor!`], and also
/// generates the required associated types for ISLE's multi-extractor/constructor protocol.
///
/// # Syntax
///
/// ```ignore
/// isle_multi_accessors! {
///     VariantName(extractor_name, constructor_name, arity);
///     // ...
/// }
/// ```
///
/// # Example
///
/// ```ignore
/// impl rules::Context for MyContext {
///     isle_multi_accessors! {
///         Or(extractor_or, constructor_or, 2);
///         Not(extractor_not, constructor_not, 1);
///     }
/// }
/// ```
///
/// This generates:
/// - Associated types: `type extractor_or_returns = ...`
/// - Multi-extractor function with `returns` parameter
/// - Multi-constructor function with `returns` parameter
#[proc_macro]
pub fn isle_multi_accessors(input: TokenStream) -> TokenStream {
    let AccessorList { items } = parse_macro_input!(input as AccessorList);

    let mut type_defs = Vec::new();
    let mut functions = Vec::new();

    for item in items.iter() {
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
            functions.push(err.clone());
            functions.push(err);
            continue;
        }

        // Multi versions: generate associated types and functions with returns parameters

        // Generate associated type names
        let extractor_returns_type = Ident::new(
            &format!("{}_returns", extractor_name),
            proc_macro2::Span::call_site(),
        );
        let constructor_returns_type = Ident::new(
            &format!("{}_returns", constructor_name),
            proc_macro2::Span::call_site(),
        );

        // Generate return value types
        let extractor_output_type = if arity == 1 {
            quote! { egg::Id }
        } else {
            let id_types = (0..arity).map(|_| quote! { egg::Id });
            quote! { (#(#id_types),*) }
        };

        // Generate associated type definitions
        type_defs.push(quote! {
            type #extractor_returns_type = ContextIterWrapper<Vec<#extractor_output_type>, Self>;
        });
        type_defs.push(quote! {
            type #constructor_returns_type = ContextIterWrapper<Vec<egg::Id>, Self>;
        });

        // Generate extractor function
        let extractor = if arity == 1 {
            quote! {
                fn #extractor_name(&mut self, arg0: egg::Id, returns: &mut Self::#extractor_returns_type) -> () {
                    let eclass = self.egraph.find(arg0);
                    for (_node_id, node) in self.egraph.nodes_in_class(eclass) {
                        if let #variant(id) = node {
                            // Canonicalize the ID so pattern matching works with e-class equality
                            returns.push(self.egraph.find(*id));
                        }
                    }
                }
            }
        } else {
            let id_names: Vec<_> = (1..=arity)
                .map(|i| Ident::new(&format!("id{}", i), proc_macro2::Span::call_site()))
                .collect();
            let canonical_ids = id_names.iter().map(|id| quote! { self.egraph.find(*#id) });

            quote! {
                fn #extractor_name(&mut self, arg0: egg::Id, returns: &mut Self::#extractor_returns_type) -> () {
                    let eclass = self.egraph.find(arg0);
                    for (_node_id, node) in self.egraph.nodes_in_class(eclass) {
                        if let #variant([#(#id_names),*]) = node {
                            // Canonicalize all IDs so pattern matching works with e-class equality
                            returns.push((#(#canonical_ids),*));
                        }
                    }
                }
            }
        };

        // Generate constructor function
        let arg_names: Vec<_> = (0..arity)
            .map(|i| Ident::new(&format!("arg{}", i), proc_macro2::Span::call_site()))
            .collect();

        let constructor = if arity == 1 {
            let arg0 = &arg_names[0];
            quote! {
                fn #constructor_name(&mut self, #arg0: egg::Id, returns: &mut Self::#constructor_returns_type) -> () {
                    let (id, is_new) = self.egraph.add_with_flag(#variant(#arg0));
                    if is_new {
                        self.push_task(Task::ExploreChildren(id));
                    }
                    returns.push(id);
                }
            }
        } else {
            let params = arg_names.iter().map(|arg| quote! { #arg: egg::Id });
            let args = &arg_names;

            quote! {
                fn #constructor_name(&mut self, #(#params),*, returns: &mut Self::#constructor_returns_type) -> () {
                    let (id, is_new) = self.egraph.add_with_flag(#variant([#(#args),*]));
                    if is_new {
                        self.push_task(Task::ExploreChildren(id));
                    }
                    returns.push(id);
                }
            }
        };

        functions.push(extractor);
        functions.push(constructor);
    }

    let output = quote! {
        #(#type_defs)*
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
