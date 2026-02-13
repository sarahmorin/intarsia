/// Implements the context for ISLE generated code.

// --------------------------------------------
// -- ISLE Generated Code Integration
// --------------------------------------------
// Include the ISLE-generated code
#[allow(dead_code, unused_variables, unused_imports, non_snake_case)]
#[allow(irrefutable_let_patterns, unused_assignments, non_camel_case_types)]
#[allow(unreachable_patterns, unreachable_code)]
#[path = "isle/rules.rs"]
mod rules;
use egg::{CostFunction, Extractor, RecExpr};
use rules::*;

// -- Required for error reporting in generated ISLE code --
// NOTE: When using a multiconstructor, you must set a maximum number of returns.
// You also need to define the ConstructorVec type for the multiconstructor.
type ConstructorVec<T> = Vec<T>;
const MAX_ISLE_RETURNS: usize = 100;
// --------------------------------------------
use crate::{
    catalog::Catalog,
    types::{ColSet, ColSetId, IndexId, TableId},
};

use bimap::BiMap;
use egg::{EGraph, Id, define_language};
use log::warn;
use std::collections::HashSet;

// Operator Language for optimization
define_language! {
    pub enum Optlang {
        // Constant Values
        Int(i64),
        Bool(bool),
        Str(String),
        // Arithmetic Operations
        "+" = Add([Id; 2]),
        "-" = Sub([Id; 2]),
        "*" = Mul([Id; 2]),
        "/" = Div([Id; 2]),
        // Comparison Operations
        "==" = Eq([Id; 2]),
        "<" = Lt([Id; 2]),
        ">" = Gt([Id; 2]),
        "<=" = Le([Id; 2]),
        ">=" = Ge([Id; 2]),
        "!=" = Ne([Id; 2]),
        // Logical Operations
        "AND" = And([Id; 2]),
        "OR" = Or([Id; 2]),
        "NOT" = Not(Id),
        // Relational Operations
        "SCAN" = Scan(Id),
        "SELECT" = Select([Id; 2]),
        "PROJECT" = Project([Id; 2]),
        "JOIN" = Join([Id; 3]),
        "TABLE_SCAN" = TableScan(Id),
        "INDEX_SCAN" = IndexScan(Id),
        "NESTED_LOOP_JOIN" = NestedLoopJoin([Id; 3]),
        "HASH_JOIN" = HashJoin([Id; 3]),
        "MERGE_JOIN" = MergeJoin([Id; 3]),
        "SORT" = Sort([Id; 2]),
        // Data Sources
        Table(TableId),
        ColSet(ColSetId),
        Index(IndexId),
    }
}

/// The context structure for ISLE-generated code.
#[derive(Debug, Clone)]
pub struct OptimizerContext {
    /// E-Graph to hold all expressions and their equivalences
    pub egraph: EGraph<Optlang, ()>,
    /// Catalog to hold metadata about tables, columns, and indexes
    pub catalog: Catalog,
    /// Colsets represent references to groups of columns for projections and predicates
    pub colsets: BiMap<ColSet, ColSetId>,
    next_colset_id: ColSetId,
    processed: HashSet<Id>, // Track e-classes that have been explored/optimized to prevent infinite recursion
}

/// A very simple example of properties we might want to track in our optimizer context.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SimpleProperty {
    Sorted,
    Unsorted,
    Irrelevant,
}

/// Implement a simple cost function for the optimizer context
impl CostFunction<Optlang> for OptimizerContext {
    type Cost = (usize, SimpleProperty); // (cost, is_sorted) HACK: this doesn't check how its sorted, its a very simple use of properties

    fn cost<C>(&mut self, enode: &Optlang, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        // HACK: Arbitrary base costs...do we need these??
        const JOIN_COST: usize = 100;
        const SORT_COST: usize = 50;
        const PROJECT_COST: usize = 20;
        const FILTER_COST: usize = 10;

        match enode {
            // Constant values have a cost of 0
            Optlang::Int(_) | Optlang::Bool(_) | Optlang::Str(_) => (0, SimpleProperty::Irrelevant),
            // CPU operations have a cost of 1
            Optlang::Add(_)
            | Optlang::Sub(_)
            | Optlang::Mul(_)
            | Optlang::Div(_)
            | Optlang::Eq(_)
            | Optlang::Lt(_)
            | Optlang::Gt(_)
            | Optlang::Le(_)
            | Optlang::Ge(_)
            | Optlang::Ne(_)
            | Optlang::And(_)
            | Optlang::Or(_)
            | Optlang::Not(_) => (1, SimpleProperty::Irrelevant),
            // Data sources have a cost of 0 but different properties
            Optlang::Table(_) => (0, SimpleProperty::Unsorted),
            Optlang::ColSet(_) => (0, SimpleProperty::Irrelevant),
            Optlang::Index(_) => (0, SimpleProperty::Sorted),
            // Selection
            // FIXME: Eventually these will be cost 0 and have physical implementation operators instead
            Optlang::Select([source, pred]) => {
                let (source_cost, source_prop) = costs(*source);
                let (pred_cost, _) = costs(*pred);
                (source_cost + pred_cost + FILTER_COST, source_prop) // The output of a filter is as sorted as its input
            }
            // Projection
            // FIXME: Eventually these will be cost 0 and have physical implementation operators instead
            Optlang::Project([cols, source]) => {
                let (cols_cost, _) = costs(*cols);
                let (source_cost, source_prop) = costs(*source);
                (cols_cost + source_cost + PROJECT_COST, source_prop) // The output of a project is as sorted as its input
            }
            // Logical Operators have a cost of usize::MAX to prevent extraction
            Optlang::Join(_) | Optlang::Scan(_) => (usize::MAX, SimpleProperty::Irrelevant),
            // NestedLoopJoin
            Optlang::NestedLoopJoin([left, right, pred]) => {
                let (left_cost, left_prop) = costs(*left);
                let (right_cost, right_prop) = costs(*right);
                let (pred_cost, _) = costs(*pred);
                // Cost of the nested loop join is left x right + the cost of the predicate evaluation
                let cost = left_cost * right_cost + pred_cost + JOIN_COST;
                let prop = if left_prop == SimpleProperty::Sorted
                    && right_prop == SimpleProperty::Sorted
                {
                    SimpleProperty::Sorted
                } else {
                    SimpleProperty::Unsorted
                };
                (cost, prop)
            }
            // HashJoin
            Optlang::HashJoin([left, right, pred]) => {
                let (left_cost, left_prop) = costs(*left);
                let (right_cost, right_prop) = costs(*right);
                let (pred_cost, _) = costs(*pred);
                todo!("Finish")
            }
            // MergeJoin
            Optlang::MergeJoin([left, right, pred]) => {
                let (left_cost, left_prop) = costs(*left);
                let (right_cost, right_prop) = costs(*right);
                let (pred_cost, _) = costs(*pred);
                if left_prop == SimpleProperty::Sorted && right_prop == SimpleProperty::Sorted {
                    (
                        left_cost + right_cost + pred_cost + JOIN_COST,
                        SimpleProperty::Sorted,
                    )
                } else {
                    // HACK: For now, if the inputs aren't sorted we use MAX cost
                    (usize::MAX, SimpleProperty::Sorted)
                }
            }
            // Sort
            Optlang::Sort([source, cols]) => {
                let (source_cost, source_prop) = costs(*source);
                if source_prop == SimpleProperty::Sorted {
                    // If the source is already sorted, we can skip the sort and just return the cost of the source
                    return (source_cost, SimpleProperty::Sorted);
                }
                let (cols_cost, _) = costs(*cols);

                (source_cost + cols_cost + SORT_COST, SimpleProperty::Sorted) // The output of a sort is always sorted
            }
            // TableScan
            Optlang::TableScan(arg_id) => {
                let table_id = match self.egraph.get_node(*arg_id) {
                    Optlang::Table(table_id) => table_id,
                    _ => {
                        warn!(
                            "TableScan node with id {} does not contain a Table node",
                            arg_id
                        );
                        return (usize::MAX, SimpleProperty::Unsorted);
                    }
                };
                // Lookup table in catalog to get number of rows, which we use as the cost for now
                if let Some(table) = self.catalog.get_table_by_id(*table_id) {
                    (table.get_est_num_rows(), SimpleProperty::Unsorted)
                } else {
                    (usize::MAX, SimpleProperty::Unsorted)
                }
            }
            // IndexScan
            Optlang::IndexScan(arg_id) => {
                let index_id = match self.egraph.get_node(*arg_id) {
                    Optlang::Index(index_id) => index_id,
                    _ => {
                        warn!(
                            "IndexScan node with id {} does not contain an Index node",
                            arg_id
                        );
                        return (usize::MAX, SimpleProperty::Sorted);
                    }
                };
                // Lookup index in catalog to verify exists, then lookup corresponding table to get number of rows, which we use as the cost for now
                if let Some(index) = self.catalog.get_index_by_id(*index_id) {
                    if let Some(table) = self.catalog.get_table_by_id(index.table_id) {
                        return (table.get_est_num_rows(), SimpleProperty::Sorted);
                    }
                }
                (usize::MAX, SimpleProperty::Sorted)
            }
        }
    }
}

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
impl Context for OptimizerContext {
    fn extractor_combine_columns(&mut self, arg0: usize) -> Option<(usize, usize)> {
        warn!(
            "extractor_combine_columns doesn't make sense, we shouldn't call it in the first place"
        );
        None
    }

    fn constructor_combine_columns(&mut self, arg0: usize, arg1: usize) -> Option<usize> {
        let colset1 = self
            .colsets
            .get_by_right(&arg0)
            .expect("Invalid ColSetId for constructor_combine_columns");

        let colset2 = self
            .colsets
            .get_by_right(&arg1)
            .expect("Invalid ColSetId for constructor_combine_columns");
        let combined_colset = ColSet::combine(colset1, colset2)?;
        // Check if we already have this combined colset in our BiMap, if not add it with a new ColSetId
        if let Some(id) = self.colsets.get_by_left(&combined_colset) {
            Some(*id)
        } else {
            let new_id = self.next_colset_id;
            self.colsets.insert(combined_colset.clone(), new_id);
            self.next_colset_id += 1;
            Some(new_id)
        }
    }

    fn extractor_access_table(&mut self, arg0: Id) -> Option<usize> {
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::Table(table_id) = node {
                // QUESTION: should we verify that the table_id is valid (i.e. exists in the catalog) before returning it
                return Some(*table_id);
            }
        }
        None
    }

    fn constructor_access_table(&mut self, arg0: usize) -> Option<Id> {
        // Verify that table exists in the catalog before constructing the node
        if !self.catalog.tables.contains_key(&arg0) {
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
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::Index(index_id) = node {
                return Some(*index_id);
            }
        }
        None
    }

    fn constructor_access_index(&mut self, arg0: usize) -> Option<Id> {
        // Verify that index exists in the catalog before constructing the node
        if !self.catalog.indexes.contains_key(&arg0) {
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
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::ColSet(colset_id) = node {
                return Some(*colset_id);
            }
        }
        None
    }

    fn constructor_ref_colset(&mut self, arg0: usize) -> Option<Id> {
        // Verify that colset exists in the catalog before constructing the node
        if !self.colsets.contains_right(&arg0) {
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

    fn extractor_const_val(&mut self, arg0: Id) -> Option<value> {
        for node in self.egraph.nodes_in_class(arg0) {
            match node {
                Optlang::Int(i) => return Some(value::Int { val: *i }),
                Optlang::Bool(b) => return Some(value::Bool { val: *b }),
                Optlang::Str(s) => return Some(value::Str { val: s.clone() }),
                _ => {}
            }
        }
        None
    }

    fn constructor_const_val(&mut self, arg0: &value) -> Id {
        let node = match arg0 {
            value::Int { val } => Optlang::Int(*val),
            value::Bool { val } => Optlang::Bool(*val),
            value::Str { val } => Optlang::Str(val.clone()),
        };
        let (id, _) = self.egraph.add_with_flag(node);
        id
    }

    type extractor_add_returns = ContextIterWrapper<ConstructorVec<(Id, Id)>, Self>;
    fn extractor_add(&mut self, arg0: Id, returns: &mut Self::extractor_add_returns) -> () {
        // Collect all matching terms first
        let mut matches = Vec::new();
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::Add([id1, id2]) = node {
                matches.push((*id1, *id2));
            }
        }

        // For every match, run the arguments and then return the match
        for (lhs, rhs) in matches {
            self.run(lhs);
            self.run(rhs);
            returns.extend(Some((lhs, rhs)));
            if returns.len() >= MAX_ISLE_RETURNS {
                break;
            }
        }
    }

    type constructor_add_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn constructor_add(
        &mut self,
        arg0: Id,
        arg1: Id,
        returns: &mut Self::constructor_add_returns,
    ) -> () {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Add([arg0, arg1]));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }

        // If we created a new e-class, we need to run it to explore/optimize further
        if is_new {
            self.run(id);
        }
    }

    type extractor_sub_returns = ContextIterWrapper<ConstructorVec<(Id, Id)>, Self>;
    fn extractor_sub(&mut self, arg0: Id, returns: &mut Self::extractor_sub_returns) -> () {
        // Collect all matching terms first
        let mut matches = Vec::new();
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::Sub([id1, id2]) = node {
                matches.push((*id1, *id2));
            }
        }

        // For every match, run the arguments and then return the match
        for (lhs, rhs) in matches {
            self.run(lhs);
            self.run(rhs);
            returns.extend(Some((lhs, rhs)));
            if returns.len() >= MAX_ISLE_RETURNS {
                break;
            }
        }
    }

    type constructor_sub_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn constructor_sub(
        &mut self,
        arg0: Id,
        arg1: Id,
        returns: &mut Self::constructor_sub_returns,
    ) -> () {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Sub([arg0, arg1]));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }

        // If we created a new e-class, we need to run it to explore/optimize further
        if is_new {
            self.run(id);
        }
    }

    type extractor_mul_returns = ContextIterWrapper<ConstructorVec<(Id, Id)>, Self>;
    fn extractor_mul(&mut self, arg0: Id, returns: &mut Self::extractor_mul_returns) -> () {
        // Collect all matching terms first
        let mut matches = Vec::new();
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::Mul([id1, id2]) = node {
                matches.push((*id1, *id2));
            }
        }

        // For every match, run the arguments and then return the match
        for (lhs, rhs) in matches {
            self.run(lhs);
            self.run(rhs);
            returns.extend(Some((lhs, rhs)));
            if returns.len() >= MAX_ISLE_RETURNS {
                break;
            }
        }
    }

    type constructor_mul_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn constructor_mul(
        &mut self,
        arg0: Id,
        arg1: Id,
        returns: &mut Self::constructor_mul_returns,
    ) -> () {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Mul([arg0, arg1]));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }

        // If we created a new e-class, we need to run it to explore/optimize further
        if is_new {
            self.run(id);
        }
    }

    type extractor_div_returns = ContextIterWrapper<ConstructorVec<(Id, Id)>, Self>;
    fn extractor_div(&mut self, arg0: Id, returns: &mut Self::extractor_div_returns) -> () {
        // Collect all matching terms first
        let mut matches = Vec::new();
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::Div([id1, id2]) = node {
                matches.push((*id1, *id2));
            }
        }

        // For every match, run the arguments and then return the match
        for (lhs, rhs) in matches {
            self.run(lhs);
            self.run(rhs);
            returns.extend(Some((lhs, rhs)));
            if returns.len() >= MAX_ISLE_RETURNS {
                break;
            }
        }
    }

    type constructor_div_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn constructor_div(
        &mut self,
        arg0: Id,
        arg1: Id,
        returns: &mut Self::constructor_div_returns,
    ) -> () {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Div([arg0, arg1]));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }

        // If we created a new e-class, we need to run it to explore/optimize further
        if is_new {
            self.run(id);
        }
    }

    type extractor_eq_returns = ContextIterWrapper<ConstructorVec<(Id, Id)>, Self>;
    fn extractor_eq(&mut self, arg0: Id, returns: &mut Self::extractor_eq_returns) -> () {
        // Collect all matching terms first
        let mut matches = Vec::new();
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::Eq([id1, id2]) = node {
                matches.push((*id1, *id2));
            }
        }

        // For every match, run the arguments and then return the match
        for (lhs, rhs) in matches {
            self.run(lhs);
            self.run(rhs);
            returns.extend(Some((lhs, rhs)));
            if returns.len() >= MAX_ISLE_RETURNS {
                break;
            }
        }
    }

    type constructor_eq_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn constructor_eq(
        &mut self,
        arg0: Id,
        arg1: Id,
        returns: &mut Self::constructor_eq_returns,
    ) -> () {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Eq([arg0, arg1]));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }

        // If we created a new e-class, we need to run it to explore/optimize further
        if is_new {
            self.run(id);
        }
    }

    type extractor_lt_returns = ContextIterWrapper<ConstructorVec<(Id, Id)>, Self>;
    fn extractor_lt(&mut self, arg0: Id, returns: &mut Self::extractor_lt_returns) -> () {
        // Collect all matching terms first
        let mut matches = Vec::new();
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::Lt([id1, id2]) = node {
                matches.push((*id1, *id2));
            }
        }

        // For every match, run the arguments and then return the match
        for (lhs, rhs) in matches {
            self.run(lhs);
            self.run(rhs);
            returns.extend(Some((lhs, rhs)));
            if returns.len() >= MAX_ISLE_RETURNS {
                break;
            }
        }
    }

    type constructor_lt_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn constructor_lt(
        &mut self,
        arg0: Id,
        arg1: Id,
        returns: &mut Self::constructor_lt_returns,
    ) -> () {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Lt([arg0, arg1]));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }

        // If we created a new e-class, we need to run it to explore/optimize further
        if is_new {
            self.run(id);
        }
    }

    type extractor_gt_returns = ContextIterWrapper<ConstructorVec<(Id, Id)>, Self>;
    fn extractor_gt(&mut self, arg0: Id, returns: &mut Self::extractor_gt_returns) -> () {
        // Collect all matching terms first
        let mut matches = Vec::new();
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::Gt([id1, id2]) = node {
                matches.push((*id1, *id2));
            }
        }

        // For every match, run the arguments and then return the match
        for (lhs, rhs) in matches {
            self.run(lhs);
            self.run(rhs);
            returns.extend(Some((lhs, rhs)));
            if returns.len() >= MAX_ISLE_RETURNS {
                break;
            }
        }
    }

    type constructor_gt_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn constructor_gt(
        &mut self,
        arg0: Id,
        arg1: Id,
        returns: &mut Self::constructor_gt_returns,
    ) -> () {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Gt([arg0, arg1]));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }

        // If we created a new e-class, we need to run it to explore/optimize further
        if is_new {
            self.run(id);
        }
    }

    type extractor_le_returns = ContextIterWrapper<ConstructorVec<(Id, Id)>, Self>;
    fn extractor_le(&mut self, arg0: Id, returns: &mut Self::extractor_le_returns) -> () {
        // Collect all matching terms first
        let mut matches = Vec::new();
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::Le([id1, id2]) = node {
                matches.push((*id1, *id2));
            }
        }

        // For every match, run the arguments and then return the match
        for (lhs, rhs) in matches {
            self.run(lhs);
            self.run(rhs);
            returns.extend(Some((lhs, rhs)));
            if returns.len() >= MAX_ISLE_RETURNS {
                break;
            }
        }
    }

    type constructor_le_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn constructor_le(
        &mut self,
        arg0: Id,
        arg1: Id,
        returns: &mut Self::constructor_le_returns,
    ) -> () {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Le([arg0, arg1]));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }

        // If we created a new e-class, we need to run it to explore/optimize further
        if is_new {
            self.run(id);
        }
    }

    type extractor_ge_returns = ContextIterWrapper<ConstructorVec<(Id, Id)>, Self>;
    fn extractor_ge(&mut self, arg0: Id, returns: &mut Self::extractor_ge_returns) -> () {
        // Collect all matching terms first
        let mut matches = Vec::new();
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::Ge([id1, id2]) = node {
                matches.push((*id1, *id2));
            }
        }

        // For every match, run the arguments and then return the match
        for (lhs, rhs) in matches {
            self.run(lhs);
            self.run(rhs);
            returns.extend(Some((lhs, rhs)));
            if returns.len() >= MAX_ISLE_RETURNS {
                break;
            }
        }
    }

    type constructor_ge_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn constructor_ge(
        &mut self,
        arg0: Id,
        arg1: Id,
        returns: &mut Self::constructor_ge_returns,
    ) -> () {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Ge([arg0, arg1]));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }

        // If we created a new e-class, we need to run it to explore/optimize further
        if is_new {
            self.run(id);
        }
    }

    type extractor_ne_returns = ContextIterWrapper<ConstructorVec<(Id, Id)>, Self>;
    fn extractor_ne(&mut self, arg0: Id, returns: &mut Self::extractor_ne_returns) -> () {
        // Collect all matching terms first
        let mut matches = Vec::new();
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::Ne([id1, id2]) = node {
                matches.push((*id1, *id2));
            }
        }

        // For every match, run the arguments and then return the match
        for (lhs, rhs) in matches {
            self.run(lhs);
            self.run(rhs);
            returns.extend(Some((lhs, rhs)));
            if returns.len() >= MAX_ISLE_RETURNS {
                break;
            }
        }
    }

    type constructor_ne_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn constructor_ne(
        &mut self,
        arg0: Id,
        arg1: Id,
        returns: &mut Self::constructor_ne_returns,
    ) -> () {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Ne([arg0, arg1]));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }

        // If we created a new e-class, we need to run it to explore/optimize further
        if is_new {
            self.run(id);
        }
    }

    type extractor_and_returns = ContextIterWrapper<ConstructorVec<(Id, Id)>, Self>;
    fn extractor_and(&mut self, arg0: Id, returns: &mut Self::extractor_and_returns) -> () {
        // Collect all matching terms first
        let mut matches = Vec::new();
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::And([id1, id2]) = node {
                matches.push((*id1, *id2));
            }
        }

        // For every match, run the arguments and then return the match
        for (lhs, rhs) in matches {
            self.run(lhs);
            self.run(rhs);
            returns.extend(Some((lhs, rhs)));
            if returns.len() >= MAX_ISLE_RETURNS {
                break;
            }
        }
    }

    type constructor_and_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn constructor_and(
        &mut self,
        arg0: Id,
        arg1: Id,
        returns: &mut Self::constructor_and_returns,
    ) -> () {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::And([arg0, arg1]));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }

        // If we created a new e-class, we need to run it to explore/optimize further
        if is_new {
            self.run(id);
        }
    }

    type extractor_or_returns = ContextIterWrapper<ConstructorVec<(Id, Id)>, Self>;
    fn extractor_or(&mut self, arg0: Id, returns: &mut Self::extractor_or_returns) -> () {
        // Collect all matching terms first
        let mut matches = Vec::new();
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::Or([id1, id2]) = node {
                matches.push((*id1, *id2));
            }
        }

        // For every match, run the arguments and then return the match
        for (lhs, rhs) in matches {
            self.run(lhs);
            self.run(rhs);
            returns.extend(Some((lhs, rhs)));
            if returns.len() >= MAX_ISLE_RETURNS {
                break;
            }
        }
    }

    type constructor_or_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn constructor_or(
        &mut self,
        arg0: Id,
        arg1: Id,
        returns: &mut Self::constructor_or_returns,
    ) -> () {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Or([arg0, arg1]));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }

        // If we created a new e-class, we need to run it to explore/optimize further
        if is_new {
            self.run(id);
        }
    }

    type extractor_not_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn extractor_not(&mut self, arg0: Id, returns: &mut Self::extractor_not_returns) -> () {
        // Collect all matching terms first
        let mut matches = Vec::new();
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::Not(id1) = node {
                matches.push(*id1);
            }
        }

        // For every match, run the argument and then return the match
        for arg in matches {
            self.run(arg);
            returns.extend(Some(arg));
            if returns.len() >= MAX_ISLE_RETURNS {
                break;
            }
        }
    }

    type constructor_not_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn constructor_not(&mut self, arg0: Id, returns: &mut Self::constructor_not_returns) -> () {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Not(arg0));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }

        // If we created a new e-class, we need to run it to explore/optimize further
        if is_new {
            self.run(id);
        }
    }

    type extractor_select_returns = ContextIterWrapper<ConstructorVec<(Id, Id)>, Self>;
    fn extractor_select(&mut self, arg0: Id, returns: &mut Self::extractor_select_returns) -> () {
        // Collect all matching terms first
        let mut matches = Vec::new();
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::Select([id1, id2]) = node {
                matches.push((*id1, *id2));
            }
        }

        // For every match, run the arguments and then return the match
        for (lhs, rhs) in matches {
            self.run(lhs);
            self.run(rhs);
            returns.extend(Some((lhs, rhs)));
            if returns.len() >= MAX_ISLE_RETURNS {
                break;
            }
        }
    }

    type constructor_select_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn constructor_select(
        &mut self,
        arg0: Id,
        arg1: Id,
        returns: &mut Self::constructor_select_returns,
    ) -> () {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Select([arg0, arg1]));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }

        // If we created a new e-class, we need to run it to explore/optimize further
        if is_new {
            self.run(id);
        }
    }

    type extractor_project_returns = ContextIterWrapper<ConstructorVec<(Id, Id)>, Self>;
    fn extractor_project(&mut self, arg0: Id, returns: &mut Self::extractor_project_returns) -> () {
        // Collect all matching terms first
        let mut matches = Vec::new();
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::Project([id1, id2]) = node {
                matches.push((*id1, *id2));
            }
        }

        // For every match, run the arguments and then return the match
        for (lhs, rhs) in matches {
            self.run(lhs);
            self.run(rhs);
            returns.extend(Some((lhs, rhs)));
            if returns.len() >= MAX_ISLE_RETURNS {
                break;
            }
        }
    }

    type constructor_project_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn constructor_project(
        &mut self,
        arg0: Id,
        arg1: Id,
        returns: &mut Self::constructor_project_returns,
    ) -> () {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Project([arg0, arg1]));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }

        // If we created a new e-class, we need to run it to explore/optimize further
        if is_new {
            self.run(id);
        }
    }

    type extractor_join_returns = ContextIterWrapper<ConstructorVec<(Id, Id, Id)>, Self>;
    fn extractor_join(&mut self, arg0: Id, returns: &mut Self::extractor_join_returns) -> () {
        // Collect all matching terms first
        let mut matches = Vec::new();
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::Join([id1, id2, id3]) = node {
                matches.push((*id1, *id2, *id3));
            }
        }

        // For every match, run the arguments and then return the match
        for (arg1, arg2, arg3) in matches {
            self.run(arg1);
            self.run(arg2);
            self.run(arg3);
            returns.extend(Some((arg1, arg2, arg3)));
            if returns.len() >= MAX_ISLE_RETURNS {
                break;
            }
        }
    }

    type constructor_join_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn constructor_join(
        &mut self,
        arg0: Id,
        arg1: Id,
        arg2: Id,
        returns: &mut Self::constructor_join_returns,
    ) -> () {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Join([arg0, arg1, arg2]));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }

        // If we created a new e-class, we need to run it to explore/optimize further
        if is_new {
            self.run(id);
        }
    }

    type extractor_scan_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn extractor_scan(&mut self, arg0: Id, returns: &mut Self::extractor_scan_returns) -> () {
        let mut matches = Vec::new();
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::Scan(id) = node {
                matches.push(*id);
            }
        }

        // For every match, run the argument and then return the match
        for id in matches {
            self.run(id);
            returns.extend(Some(id));
            if returns.len() >= MAX_ISLE_RETURNS {
                break;
            }
        }
    }

    type constructor_scan_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn constructor_scan(&mut self, arg0: Id, returns: &mut Self::constructor_scan_returns) -> () {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Scan(arg0));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }

        // If we created a new e-class, we need to run it to explore/optimize further
        if is_new {
            self.run(id);
        }
    }

    fn extractor_table_scan(&mut self, arg0: Id) -> Option<Id> {
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::TableScan(id) = node {
                return Some(*id);
            }
        }
        None
    }

    fn constructor_table_scan(&mut self, arg0: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::TableScan(arg0));
        if is_new {
            self.run(id);
        }
        id
    }

    fn extractor_index_scan(&mut self, arg0: Id) -> Option<Id> {
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::IndexScan(id) = node {
                return Some(*id);
            }
        }
        None
    }

    fn constructor_index_scan(&mut self, arg0: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::IndexScan(arg0));
        if is_new {
            self.run(id);
        }
        id
    }

    type extractor_nested_loop_join_returns =
        ContextIterWrapper<ConstructorVec<(Id, Id, Id)>, Self>;
    fn extractor_nested_loop_join(
        &mut self,
        arg0: Id,
        returns: &mut Self::extractor_nested_loop_join_returns,
    ) -> () {
        // Collect all matching terms first
        let mut matches = Vec::new();
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::NestedLoopJoin([id1, id2, id3]) = node {
                matches.push((*id1, *id2, *id3));
            }
        }

        // For every match, run the arguments and then return the match
        for (arg1, arg2, arg3) in matches {
            self.run(arg1);
            self.run(arg2);
            self.run(arg3);
            returns.extend(Some((arg1, arg2, arg3)));
            if returns.len() >= MAX_ISLE_RETURNS {
                break;
            }
        }
    }

    type constructor_nested_loop_join_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn constructor_nested_loop_join(
        &mut self,
        arg0: Id,
        arg1: Id,
        arg2: Id,
        returns: &mut Self::constructor_nested_loop_join_returns,
    ) -> () {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self
            .egraph
            .add_with_flag(Optlang::NestedLoopJoin([arg0, arg1, arg2]));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }

        // If we created a new e-class, we need to run it to explore/optimize further
        if is_new {
            self.run(id);
        }
    }

    type extractor_hash_join_returns = ContextIterWrapper<ConstructorVec<(Id, Id, Id)>, Self>;
    fn extractor_hash_join(
        &mut self,
        arg0: Id,
        returns: &mut Self::extractor_hash_join_returns,
    ) -> () {
        // Collect all matching terms first
        let mut matches = Vec::new();
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::HashJoin([id1, id2, id3]) = node {
                matches.push((*id1, *id2, *id3));
            }
        }

        // For every match, run the arguments and then return the match
        for (arg1, arg2, arg3) in matches {
            self.run(arg1);
            self.run(arg2);
            self.run(arg3);
            returns.extend(Some((arg1, arg2, arg3)));
            if returns.len() >= MAX_ISLE_RETURNS {
                break;
            }
        }
    }

    type constructor_hash_join_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn constructor_hash_join(
        &mut self,
        arg0: Id,
        arg1: Id,
        arg2: Id,
        returns: &mut Self::constructor_hash_join_returns,
    ) -> () {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self
            .egraph
            .add_with_flag(Optlang::HashJoin([arg0, arg1, arg2]));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }

        // If we created a new e-class, we need to run it to explore/optimize further
        if is_new {
            self.run(id);
        }
    }

    type extractor_merge_join_returns = ContextIterWrapper<ConstructorVec<(Id, Id, Id)>, Self>;
    fn extractor_merge_join(
        &mut self,
        arg0: Id,
        returns: &mut Self::extractor_merge_join_returns,
    ) -> () {
        // Collect all matching terms first
        let mut matches = Vec::new();
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::MergeJoin([id1, id2, id3]) = node {
                matches.push((*id1, *id2, *id3));
            }
        }

        // For every match, run the arguments and then return the match
        for (arg1, arg2, arg3) in matches {
            self.run(arg1);
            self.run(arg2);
            self.run(arg3);
            returns.extend(Some((arg1, arg2, arg3)));
            if returns.len() >= MAX_ISLE_RETURNS {
                break;
            }
        }
    }

    type constructor_merge_join_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn constructor_merge_join(
        &mut self,
        arg0: Id,
        arg1: Id,
        arg2: Id,
        returns: &mut Self::constructor_merge_join_returns,
    ) -> () {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self
            .egraph
            .add_with_flag(Optlang::MergeJoin([arg0, arg1, arg2]));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }

        // If we created a new e-class, we need to run it to explore/optimize further
        if is_new {
            self.run(id);
        }
    }

    type extractor_sort_returns = ContextIterWrapper<ConstructorVec<(Id, Id)>, Self>;
    fn extractor_sort(&mut self, arg0: Id, returns: &mut Self::extractor_sort_returns) -> () {
        // Collect all matching terms first
        let mut matches = Vec::new();
        for node in self.egraph.nodes_in_class(arg0) {
            if let Optlang::Sort([id1, id2]) = node {
                matches.push((*id1, *id2));
            }
        }

        // For every match, run the arguments and then return the match
        for (lhs, rhs) in matches {
            self.run(lhs);
            self.run(rhs);
            returns.extend(Some((lhs, rhs)));
            if returns.len() >= MAX_ISLE_RETURNS {
                break;
            }
        }
    }

    type constructor_sort_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn constructor_sort(
        &mut self,
        arg0: Id,
        arg1: Id,
        returns: &mut Self::constructor_sort_returns,
    ) -> () {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Sort([arg0, arg1]));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }

        // If we created a new e-class, we need to run it to explore/optimize further
        if is_new {
            self.run(id);
        }
    }

    type extractor_optimize_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn extractor_optimize(
        &mut self,
        arg0: Id,
        returns: &mut Self::extractor_optimize_returns,
    ) -> () {
        warn!("we should never call extractor_optimize");
    }

    type extractor_explore_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn extractor_explore(&mut self, arg0: Id, returns: &mut Self::extractor_explore_returns) -> () {
        warn!("we should never call extractor_explore");
    }
}

impl OptimizerContext {
    pub fn new(catalog: Catalog) -> Self {
        Self {
            egraph: EGraph::default(),
            catalog,
            next_colset_id: 1,
            colsets: BiMap::new(),
            processed: HashSet::new(),
        }
    }

    /// Initialize the context with the initial expression, and return the ID of the initial expression in the e-graph
    pub fn init(&mut self, expr: RecExpr<Optlang>) -> Id {
        // Create new context and add the initial expression to the egraph
        let id = self.egraph.add_expr(&expr);

        // Rebuild the e-graph for safety, although there shouldn't be anything to do
        // QUESTION: should we ditch this call?
        self.egraph.rebuild();
        id
    }

    pub fn run(&mut self, id: Id) {
        // Canonicalize the ID and check if we've already processed this e-class
        let canon_id = self.egraph.find(id);
        if self.processed.contains(&canon_id) {
            return;
        }
        self.processed.insert(canon_id);

        eprintln!(
            "DEBUG: run() processing e-class {} (processed count: {})",
            canon_id,
            self.processed.len()
        );

        // Explore phase
        let mut id_set = Vec::new();
        rules::constructor_explore(self, canon_id, &mut id_set);

        for id2 in id_set {
            self.egraph.union(canon_id, id2);
        }

        self.egraph.rebuild();

        // Re-canonicalize after rebuild (the canonical ID may have changed)
        let canon_id = self.egraph.find(canon_id);
        self.processed.insert(canon_id);

        // Optimize phase
        let mut id_set = Vec::new();
        rules::constructor_optimize(self, canon_id, &mut id_set);

        for id2 in id_set {
            self.egraph.union(canon_id, id2);
        }
        self.egraph.rebuild();

        // Mark final canonical ID as processed
        let canon_id = self.egraph.find(canon_id);
        self.processed.insert(canon_id);
    }

    pub fn extract(self, id: Id) -> RecExpr<Optlang> {
        let extractor = Extractor::new(&self.egraph, self.clone());
        let (best_cost, best_expr) = extractor.find_best(id);
        best_expr
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DataType;

    /// Helper function to create a test catalog with some sample tables
    fn create_test_catalog() -> Catalog {
        let mut catalog = Catalog::new();

        // Create a test table with a few columns
        catalog
            .create_table_with_cols(
                "users".to_string(),
                vec![
                    ("id".to_string(), DataType::Int),
                    ("name".to_string(), DataType::String),
                    ("age".to_string(), DataType::Int),
                ],
            )
            .unwrap();

        catalog
    }

    #[test]
    fn test_create_context() {
        let catalog = Catalog::new();
        let ctx = OptimizerContext::new(catalog);
        assert_eq!(ctx.egraph.total_number_of_nodes(), 0);
    }

    #[test]
    fn test_init_simple_expression() {
        let catalog = Catalog::new();
        let mut ctx = OptimizerContext::new(catalog);

        // Create a simple arithmetic expression: 1 + 2
        let expr: RecExpr<Optlang> = "(+ 1 2)".parse().unwrap();
        let id = ctx.init(expr);

        // The e-graph should now contain nodes
        assert!(ctx.egraph.total_number_of_nodes() > 0);
    }

    #[test]
    fn test_init_and_extract_identity() {
        let catalog = Catalog::new();
        let mut ctx = OptimizerContext::new(catalog);

        // Create a simple constant expression
        let expr: RecExpr<Optlang> = "42".parse().unwrap();
        let id = ctx.init(expr.clone());

        // Extract without running optimization - should get back the same expression
        let result = ctx.extract(id);
        assert_eq!(result.to_string(), "42");
    }

    #[test]
    fn test_arithmetic_expression() {
        let catalog = Catalog::new();
        let mut ctx = OptimizerContext::new(catalog);

        // Create an arithmetic expression: (1 + 2) * 3
        let expr: RecExpr<Optlang> = "(* (+ 1 2) 3)".parse().unwrap();
        let id = ctx.init(expr);

        // Extract the expression
        let result = ctx.extract(id);
        assert_eq!(result.to_string(), "(* (+ 1 2) 3)");
    }

    #[test]
    fn test_comparison_operations() {
        let catalog = Catalog::new();
        let mut ctx = OptimizerContext::new(catalog);

        // Test equality comparison: 5 == 5
        let expr: RecExpr<Optlang> = "(== 5 5)".parse().unwrap();
        let id = ctx.init(expr);
        let result = ctx.extract(id);
        assert_eq!(result.to_string(), "(== 5 5)");

        // Test less than: 3 < 7
        let mut ctx2 = OptimizerContext::new(Catalog::new());
        let expr2: RecExpr<Optlang> = "(< 3 7)".parse().unwrap();
        let id2 = ctx2.init(expr2);
        let result2 = ctx2.extract(id2);
        assert_eq!(result2.to_string(), "(< 3 7)");
    }

    #[test]
    fn test_logical_operations() {
        let catalog = Catalog::new();
        let mut ctx = OptimizerContext::new(catalog);

        // Test AND operation: true AND false
        let expr: RecExpr<Optlang> = "(AND true false)".parse().unwrap();
        let id = ctx.init(expr);
        let result = ctx.extract(id);
        assert_eq!(result.to_string(), "(AND true false)");

        // Test NOT operation
        let mut ctx2 = OptimizerContext::new(Catalog::new());
        let expr2: RecExpr<Optlang> = "(NOT true)".parse().unwrap();
        let id2 = ctx2.init(expr2);
        let result2 = ctx2.extract(id2);
        assert_eq!(result2.to_string(), "(NOT true)");
    }

    #[test]
    fn test_run_optimization() {
        let catalog = Catalog::new();
        let mut ctx = OptimizerContext::new(catalog);

        // Create a simple expression and run optimization
        let expr: RecExpr<Optlang> = "(+ 1 2)".parse().unwrap();
        let id = ctx.init(expr);

        // Run optimization - this should explore and optimize the expression
        ctx.run(id);

        // The e-graph should still be valid after running
        assert!(ctx.egraph.total_number_of_nodes() > 0);
    }

    #[test]
    fn test_nested_expressions() {
        let catalog = Catalog::new();
        let mut ctx = OptimizerContext::new(catalog);

        // Create a nested expression: ((1 + 2) * (3 - 4))
        let expr: RecExpr<Optlang> = "(* (+ 1 2) (- 3 4))".parse().unwrap();
        let id = ctx.init(expr);

        ctx.run(id);
        let result = ctx.extract(id);

        // Should successfully extract some expression
        assert!(result.as_ref().len() > 0);
    }

    #[test]
    fn test_table_expression() {
        let catalog = create_test_catalog();
        let mut ctx = OptimizerContext::new(catalog);

        // Get the table ID for "users" - it should be 1 (first table created)
        let table_id = ctx.catalog.table_ids.get("users").unwrap();

        // Create a TableScan expression
        let expr_str = format!("(TABLE_SCAN {})", *table_id as TableId);
        let expr: RecExpr<Optlang> = expr_str.parse().unwrap();
        println!("Initial expression: {}", expr);
        let id = ctx.init(expr);

        ctx.run(id);
        let result = ctx.extract(id);

        // Should successfully extract the table scan expression
        assert!(result.as_ref().len() > 0);
        assert!(result.to_string().contains("TABLE_SCAN"));
    }

    #[test]
    fn test_complex_logical_expression() {
        let catalog = Catalog::new();
        let mut ctx = OptimizerContext::new(catalog);

        // Create a complex logical expression: (a > 5) AND (b < 10)
        let expr: RecExpr<Optlang> = "(AND (> 10 5) (< 3 10))".parse().unwrap();
        let id = ctx.init(expr);

        ctx.run(id);
        let result = ctx.extract(id);

        assert!(result.to_string().contains("AND"));
    }

    #[test]
    fn test_multiple_operations() {
        let catalog = Catalog::new();
        let mut ctx = OptimizerContext::new(catalog);

        // Test that we can add multiple expressions to the same context
        let expr1: RecExpr<Optlang> = "(+ 1 2)".parse().unwrap();
        let id1 = ctx.init(expr1);

        let expr2: RecExpr<Optlang> = "(* 3 4)".parse().unwrap();
        let id2 = ctx.egraph.add_expr(&expr2);

        // Both expressions should be in the e-graph
        assert!(ctx.egraph.total_number_of_nodes() > 0);

        // Should be able to extract both
        let ctx_clone = ctx.clone();
        let result1 = ctx.extract(id1);
        let result2 = ctx_clone.extract(id2);

        assert_eq!(result1.to_string(), "(+ 1 2)");
        assert_eq!(result2.to_string(), "(* 3 4)");
    }
}
