/// A simple optimizer example for boolean expressions, demonstrating the basic framework
/// and ISLE integration. Uses the property-free `SimpleCostAnalysis` since boolean
/// expressions don't have a meaningful property dimension — we just want cheapest AST.
use egg::{DidMerge, Id, define_language};
use intarsia::framework::{SimpleCostAnalysis, SimpleCostData, SimpleCostFn, Task};
use intarsia::{ExplorerHooks, OptimizerFramework, default_explorer_hook};
use intarsia_macros::{isle_integration_full, isle_multi_accessors};

// 0. ISLE integration: generate rewrite rules from the .isle file and link them to our optimizer.
isle_integration_full! {
    path: "isle/rules.rs",
}

// 1. Define the language of boolean expressions.
define_language! {
    pub enum BoolLang {
        // Boolean constants
        Bool(bool),
        Var(String),

        // Logical operations
        "AND" = And([Id; 2]),
        "OR" = Or([Id; 2]),
        "NOT" = Not(Id),
    }
}

// 2. Define the cost function. AST size: 1 for this node plus the sum of child winner costs.
//    `make_logical` and `merge_logical` are no-ops because we don't track any logical sub-analysis.
#[derive(Debug, Clone)]
pub struct BoolCost;

impl SimpleCostFn<BoolLang> for BoolCost {
    type Cost = usize;
    type LogicalData = ();

    fn make_logical(
        &self,
        _node: &BoolLang,
        _children: &[&SimpleCostData<usize, ()>],
    ) -> () {}

    fn merge_logical(&mut self, _a: &mut (), _b: ()) -> DidMerge {
        DidMerge(false, false)
    }

    fn cost(
        &self,
        _node: &BoolLang,
        children: &[&SimpleCostData<usize, ()>],
    ) -> Option<usize> {
        let total = children.iter().fold(1usize, |acc, c| {
            acc.saturating_add(c.winner.as_ref().map(|w| w.cost).unwrap_or(0))
        });
        Some(total)
    }
}

// 3. Type aliases.
pub type BoolAnalysis = SimpleCostAnalysis<BoolLang, usize, (), BoolCost>;
pub type BoolOptimizer = OptimizerFramework<BoolLang, BoolAnalysis, ()>;

/// Convenience constructor.
pub fn new_optimizer() -> BoolOptimizer {
    OptimizerFramework::new(SimpleCostAnalysis::new(BoolCost), ())
}

// 4. Implement the Context trait for our optimizer to link to ISLE ruleset.
#[allow(non_camel_case_types)]
impl Context for BoolOptimizer {
    // Define the associated types for manually-implemented multi terms (const and var)
    type extractor_const_returns = ContextIterWrapper<Vec<bool>, Self>;
    type constructor_const_returns = ContextIterWrapper<Vec<Id>, Self>;
    type extractor_var_returns = ContextIterWrapper<Vec<String>, Self>;
    type constructor_var_returns = ContextIterWrapper<Vec<Id>, Self>;

    fn extractor_const(&mut self, arg0: Id, returns: &mut Self::extractor_const_returns) -> () {
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
        returns.push(id);
    }

    fn extractor_var(&mut self, arg0: Id, returns: &mut Self::extractor_var_returns) -> () {
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
        returns.push(id);
    }

    isle_multi_accessors! {
        BoolLang::And(extractor_and, constructor_and, 2);
        BoolLang::Or(extractor_or, constructor_or, 2);
        BoolLang::Not(extractor_not, constructor_not, 1);
    }
}

// 5. Implement ExplorerHooks to link to the ISLE ruleset entrypoint.
impl ExplorerHooks<BoolLang> for BoolOptimizer {
    default_explorer_hook!();
}

#[cfg(test)]
mod tests;
