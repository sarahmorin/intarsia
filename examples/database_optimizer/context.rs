use super::rules::*;
/// Implements the context for ISLE generated code.
use egg::Id;

use super::ConstructorVec;
// --------------------------------------------
use super::{DbOptimizer, language::Optlang, types::ColSet};

use intarsia::framework::Task;
use intarsia_macros::isle_multi_accessors;
use log::warn;

// Implement the Context trait for OptimizerContext, which is required by ISLE-generated code.
// For every term declared in the ISLE spec, we need to implement an extractor and a constructor.
//
// Extractors take an Id corresponding to an eclass and return all nodes matching the term pattern in that eclass.
// The follow this pattern:
// - Find all matching nodes in the given eclass (e.g., all Add nodes for extractor_add).
// - For each matching node, call run on the arguments to explore/optimize subexpressions.
// - Once we finish processing arguments, return all the matches we found.
//
// Constructors take the arguments for a term and construct a new node in the egraph, returning its Id.
// They follow this pattern:
// - Construct a new node with the given arguments (e.g., an Add node for constructor_add).
// - Add the node to the egraph and get its Id.
// - If the term did not already exist, i.e. if we created a new e-class, call run on the new Id to explore/optimize it.
//
// The run method calls the explore and optimize entrypoints and handles merging and rebuilding the egraph as needed.
#[allow(unused_variables)]
impl Context for DbOptimizer {
    // Define associated types for manually-implemented multi terms
    type extractor_const_val_returns = ContextIterWrapper<Vec<value>, Self>;
    type constructor_const_val_returns = ContextIterWrapper<Vec<Id>, Self>;

    fn extractor_combine_columns(&mut self, arg0: usize) -> Option<(usize, usize)> {
        warn!(
            "extractor_combine_columns doesn't make sense, we shouldn't call it in the first place"
        );
        None
    }

    fn constructor_combine_columns(&mut self, arg0: usize, arg1: usize) -> Option<usize> {
        let colset1 = self
            .user_data
            .colsets
            .get_by_right(&arg0)
            .expect("Invalid ColSetId for constructor_combine_columns");

        let colset2 = self
            .user_data
            .colsets
            .get_by_right(&arg1)
            .expect("Invalid ColSetId for constructor_combine_columns");
        let combined_colset = ColSet::combine(colset1, colset2)?;
        // Check if we already have this combined colset in our BiMap, if not add it with a new ColSetId
        if let Some(id) = self.user_data.colsets.get_by_left(&combined_colset) {
            Some(*id)
        } else {
            let new_id = self.user_data.next_colset_id;
            self.user_data
                .colsets
                .insert(combined_colset.clone(), new_id);
            self.user_data.next_colset_id += 1;
            Some(new_id)
        }
    }

    fn extractor_access_table(&mut self, arg0: Id) -> Option<usize> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Table(table_id) = node {
            // QUESTION: should we verify that the table_id is valid (i.e. exists in the catalog) before returning it
            Some(*table_id)
        } else {
            None
        }
    }

    fn constructor_access_table(&mut self, arg0: usize) -> Option<Id> {
        // Verify that table exists in the catalog before constructing the node
        if !self.user_data.catalog.tables.contains_key(&arg0) {
            warn!(
                "constructor_access_table called with invalid table_id: {}",
                arg0
            );
            return None;
        }
        let node = Optlang::Table(arg0);
        let (id, _) = self.egraph.add_with_flag(node);
        Some(id)
    }

    fn extractor_access_index(&mut self, arg0: Id) -> Option<usize> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Index(index_id) = node {
            Some(*index_id)
        } else {
            None
        }
    }

    fn constructor_access_index(&mut self, arg0: usize) -> Option<Id> {
        // Verify that index exists in the catalog before constructing the node
        if !self.user_data.catalog.indexes.contains_key(&arg0) {
            warn!(
                "constructor_access_index called with invalid index_id: {}",
                arg0
            );
            return None;
        }
        let node = Optlang::Index(arg0);
        let (id, _) = self.egraph.add_with_flag(node);
        Some(id)
    }

    fn extractor_ref_colset(&mut self, arg0: Id) -> Option<usize> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::ColSet(colset_id) = node {
            Some(*colset_id)
        } else {
            None
        }
    }

    fn constructor_ref_colset(&mut self, arg0: usize) -> Option<Id> {
        // Verify that colset exists in the catalog before constructing the node
        if !self.user_data.colsets.contains_right(&arg0) {
            warn!(
                "constructor_ref_colset called with invalid colset_id: {}",
                arg0
            );
            return None;
        }
        let node = Optlang::ColSet(arg0);
        let (id, _) = self.egraph.add_with_flag(node);
        Some(id)
    }

    fn extractor_const_val(
        &mut self,
        arg0: Id,
        returns: &mut Self::extractor_const_val_returns,
    ) -> () {
        // Search the entire e-class for all constant value nodes
        let eclass = self.egraph.find(arg0);
        for (_node_id, node) in self.egraph.nodes_in_class(eclass) {
            match node {
                Optlang::Int(i) => returns.push(value::Int { val: *i }),
                Optlang::Bool(b) => returns.push(value::Bool { val: *b }),
                Optlang::Str(s) => returns.push(value::Str { val: s.clone() }),
                _ => {}
            }
        }
    }

    fn constructor_const_val(
        &mut self,
        arg0: &value,
        returns: &mut Self::constructor_const_val_returns,
    ) -> () {
        let node = match arg0 {
            value::Int { val } => Optlang::Int(*val),
            value::Bool { val } => Optlang::Bool(*val),
            value::Str { val } => Optlang::Str(val.clone()),
        };
        let (id, _) = self.egraph.add_with_flag(node);
        returns.push(id);
    }

    // For the extractors and constructors for the other operators, we can use the isle_multi_accessors macro to generate them automatically.
    // This macro generates both the extractor and constructor for a given operator, following the patterns described above.
    // The arguments to the macro are:
    // - The operator variant (e.g., Optlang::Add)
    // - The name of the extractor function to generate (e.g., extractor_add)
    // - The name of the constructor function to generate (e.g., constructor_add)
    // - The number of arguments the operator takes (e.g., 2 for Add)
    isle_multi_accessors! {
        // Binary logical operators
        Optlang::And(extractor_and, constructor_and, 2);
        Optlang::Or(extractor_or, constructor_or, 2);

        // Unary logical operator
        Optlang::Not(extractor_not, constructor_not, 1);

        // Arithmetic operators
        Optlang::Add(extractor_add, constructor_add, 2);
        Optlang::Sub(extractor_sub, constructor_sub, 2);
        Optlang::Mul(extractor_mul, constructor_mul, 2);
        Optlang::Div(extractor_div, constructor_div, 2);

        // Comparison operators
        Optlang::Eq(extractor_eq, constructor_eq, 2);
        Optlang::Lt(extractor_lt, constructor_lt, 2);
        Optlang::Gt(extractor_gt, constructor_gt, 2);
        Optlang::Le(extractor_le, constructor_le, 2);
        Optlang::Ge(extractor_ge, constructor_ge, 2);
        Optlang::Ne(extractor_ne, constructor_ne, 2);

        // Logical relational operators
        Optlang::Select(extractor_select, constructor_select, 2);
        Optlang::Project(extractor_project, constructor_project, 2);
        Optlang::Join(extractor_join, constructor_join, 3);
        Optlang::Scan(extractor_scan, constructor_scan, 1);

        // Physical relational operators
        Optlang::TableScan(extractor_table_scan, constructor_table_scan, 1);
        Optlang::IndexScan(extractor_index_scan, constructor_index_scan, 1);
        Optlang::NestedLoopJoin(extractor_nested_loop_join, constructor_nested_loop_join, 3);
        Optlang::HashJoin(extractor_hash_join, constructor_hash_join, 3);
        Optlang::MergeJoin(extractor_merge_join, constructor_merge_join, 3);
        Optlang::Sort(extractor_sort, constructor_sort, 2);
    }

    type extractor_explore_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn extractor_explore(&mut self, arg0: Id, returns: &mut Self::extractor_explore_returns) -> () {
        warn!("we should never call extractor_explore");
    }
}
