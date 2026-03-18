/// A simple optimizer example for boolean expressions, demonstrating the basic framework and ISLE integration.
/// This example defines a simple language of boolean expressions and a cost function that counts the number of operations.
/// The optimizer will find the cheapest equivalent expression, demonstrating how to use the framework with a custom language and cost domain.
use egg::{Id, define_language};
use intarsia::framework::{PropertyAwareLanguage, property::NoProperty};
use intarsia::{ExplorerHooks, Task};
use intarsia::{SimpleOptimizerFramework, default_explorer_hook};
use intarsia_macros::{isle_integration_full, isle_multi_accessors};

// 0. ISLE integration: Generate rewrite rules from the .isle file and link them to our optimizer framework.
isle_integration_full! {
    path: "isle/rules.rs",
}

// 1. Define the language of boolean expressions.
// 1 (a) -> Define the syntax of our language using egg's define_language macro. This will create an enum with variants for each operator and constant in our language.
// 1 (b) -> Define the language as ISLE terms in the .isle file. See isle/rules.isle for this example.
define_language! {
    pub enum BoolLang {
        // Boolean constants
        Bool(bool),
        Var(String), // Variables represented as strings for simplicity

        // Logical operations
        "AND" = And([Id; 2]),
        "OR" = Or([Id; 2]),
        "NOT" = Not(Id),
    }
}

// 2. Implement the PropertyAwareLanguage trait for our language, using NoProperty since we don't have any properties.
// TODO: Make a macro to do this
impl PropertyAwareLanguage<NoProperty> for BoolLang {
    fn property_req(&self, _child_index: usize) -> NoProperty {
        NoProperty
    }
}

// 4. Define the optimizer framework for our language and cost domain.
// The SimpleOptimizerFramework is a convenient type alias for an OptimizerFramework with SimpleCost and no user data.
pub type BoolOptimizer = SimpleOptimizerFramework<BoolLang, NoProperty>;

// 5. Implement the Context trait for our optimizer to link to ISLE ruleset.
#[allow(non_camel_case_types)]
impl Context for BoolOptimizer {
    // Define the associated types for manually-implemented multi terms (const and var)
    // TODO: Make a macro to generate these type defs and function signatures
    type extractor_const_returns = ContextIterWrapper<Vec<bool>, Self>;
    type constructor_const_returns = ContextIterWrapper<Vec<Id>, Self>;
    type extractor_var_returns = ContextIterWrapper<Vec<String>, Self>;
    type constructor_var_returns = ContextIterWrapper<Vec<Id>, Self>;

    // Implement extracting and constructing raw/constant values manually for the BoolLang
    fn extractor_const(&mut self, arg0: Id, returns: &mut Self::extractor_const_returns) -> () {
        // Search the entire e-class for all Bool nodes
        // arg0 might be a canonical e-class ID that contains multiple nodes
        let eclass = self.egraph.find(arg0);
        for (_node_id, node) in self.egraph.nodes_in_class(eclass) {
            if let BoolLang::Bool(x) = node {
                returns.push(*x);
            }
        }
    }

    fn constructor_const(
        &mut self,
        arg0: bool,
        returns: &mut Self::constructor_const_returns,
    ) -> () {
        let node = if arg0 {
            BoolLang::Bool(true)
        } else {
            BoolLang::Bool(false)
        };
        let (id, _is_new) = self.egraph.add_with_flag(node);
        // Since this is terminal, we don't need to explore children

        returns.push(id);
    }

    fn extractor_var(&mut self, arg0: Id, returns: &mut Self::extractor_var_returns) -> () {
        // Search the entire e-class for all Var nodes
        let eclass = self.egraph.find(arg0);
        for (_node_id, node) in self.egraph.nodes_in_class(eclass) {
            if let BoolLang::Var(name) = node {
                returns.push(name.clone());
            }
        }
    }

    fn constructor_var(&mut self, arg0: String, returns: &mut Self::constructor_var_returns) -> () {
        let node = BoolLang::Var(arg0);
        let (id, _is_new) = self.egraph.add_with_flag(node);
        // Since this is terminal, we don't need to explore children
        returns.push(id);
    }

    // The rest of the operators can be generated using the isle_multi_accessors macro, which creates both extractors and constructors for each operator based on the patterns we define in the .isle file.
    isle_multi_accessors! {
        BoolLang::And(extractor_and, constructor_and, 2);
        BoolLang::Or(extractor_or, constructor_or, 2);
        BoolLang::Not(extractor_not, constructor_not, 1);
    }
}

// 6. Implement ExplorerHooks to link to the ISLE ruleset entrypoint.
impl ExplorerHooks<BoolLang> for BoolOptimizer {
    default_explorer_hook!();
}

// Now, we can use this optimizer in our tests/main. See an example execution in main.rs.

#[cfg(test)]
mod tests;
