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
use egg::RecExpr;
use rules::*;

// -- Required for error reporting in generated ISLE code --
// NOTE: When using a multiconstructor, you must set a maximum number of returns.
// You also need to define the ConstructorVec type for the multiconstructor.
const MAX_ISLE_RETURNS: usize = 100;
type ConstructorVec<T> = Vec<T>;
// --------------------------------------------
use egg::{EGraph, Id, define_language};
use log::warn;

use crate::catalog::{self, Catalog};

define_language! {
    enum optlang {
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
        "SELECT" = Select([Id; 2]),
        "PROJECT" = Project([Id; 2]),
        "JOIN" = Join([Id; 3]),
        "SCAN" = Scan(Id),
        "TABLE_SCAN" = TableScan(Id),
        "INDEX_SCAN" = IndexScan(Id),
        "NESTED_LOOP_JOIN" = NestedLoopJoin([Id; 3]),
        "HASH_JOIN" = HashJoin([Id; 3]),
        "MERGE_JOIN" = MergeJoin([Id; 3]),
        "SORT" = Sort([Id; 2]),
        // Data Sources
        "[Tab]" = Table(Id),
        "[Col]" = TableCols(Id),
        "[Idx]" = Index(Id),
    }
}

/// The context structure for ISLE-generated code.
pub struct OptimizerContext {
    pub egraph: EGraph<optlang, ()>,
    pub catalog: Catalog,
    // TODO: add schema representation that maps tables names, column sets, etc. to Ids
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
    fn extractor_combine_columns(&mut self, arg0: Id) -> Option<(Id, Id)> {
        warn!(
            "extractor_combine_columns doesn't make sense, we shouldn't call it in the first place"
        );
        None
    }

    fn constructor_combine_columns(&mut self, arg0: Id, arg1: Id) -> Option<Id> {
        todo!("constructor_combine_columns not implemented")
    }

    fn extractor_const_val(&mut self, arg0: Id) -> Option<value> {
        for node in self.egraph.nodes_in_class(arg0) {
            match node {
                optlang::Int(i) => return Some(value::Int { val: *i }),
                optlang::Bool(b) => return Some(value::Bool { val: *b }),
                optlang::Str(s) => return Some(value::Str { val: s.clone() }),
                _ => {}
            }
        }
        None
    }

    fn constructor_const_val(&mut self, arg0: &value) -> Id {
        let node = match arg0 {
            value::Int { val } => optlang::Int(*val),
            value::Bool { val } => optlang::Bool(*val),
            value::Str { val } => optlang::Str(val.clone()),
        };
        let (id, _) = self.egraph.add_with_flag(node);
        id
    }

    type extractor_add_returns = ContextIterWrapper<ConstructorVec<(Id, Id)>, Self>;
    fn extractor_add(&mut self, arg0: Id, returns: &mut Self::extractor_add_returns) -> () {
        // Collect all matching terms first
        let mut matches = Vec::new();
        for node in self.egraph.nodes_in_class(arg0) {
            if let optlang::Add([id1, id2]) = node {
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
        let (id, is_new) = self.egraph.add_with_flag(optlang::Add([arg0, arg1]));
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
            if let optlang::Sub([id1, id2]) = node {
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
        let (id, is_new) = self.egraph.add_with_flag(optlang::Sub([arg0, arg1]));
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
            if let optlang::Mul([id1, id2]) = node {
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
        let (id, is_new) = self.egraph.add_with_flag(optlang::Mul([arg0, arg1]));
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
            if let optlang::Div([id1, id2]) = node {
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
        let (id, is_new) = self.egraph.add_with_flag(optlang::Div([arg0, arg1]));
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
            if let optlang::Eq([id1, id2]) = node {
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
        let (id, is_new) = self.egraph.add_with_flag(optlang::Eq([arg0, arg1]));
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
            if let optlang::Lt([id1, id2]) = node {
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
        let (id, is_new) = self.egraph.add_with_flag(optlang::Lt([arg0, arg1]));
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
            if let optlang::Gt([id1, id2]) = node {
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
        let (id, is_new) = self.egraph.add_with_flag(optlang::Gt([arg0, arg1]));
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
            if let optlang::Le([id1, id2]) = node {
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
        let (id, is_new) = self.egraph.add_with_flag(optlang::Le([arg0, arg1]));
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
            if let optlang::Ge([id1, id2]) = node {
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
        let (id, is_new) = self.egraph.add_with_flag(optlang::Ge([arg0, arg1]));
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
            if let optlang::Ne([id1, id2]) = node {
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
        let (id, is_new) = self.egraph.add_with_flag(optlang::Ne([arg0, arg1]));
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
            if let optlang::And([id1, id2]) = node {
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
        let (id, is_new) = self.egraph.add_with_flag(optlang::And([arg0, arg1]));
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
            if let optlang::Or([id1, id2]) = node {
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
        let (id, is_new) = self.egraph.add_with_flag(optlang::Or([arg0, arg1]));
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
            if let optlang::Not(id1) = node {
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
        let (id, is_new) = self.egraph.add_with_flag(optlang::Not(arg0));
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
            if let optlang::Select([id1, id2]) = node {
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
        let (id, is_new) = self.egraph.add_with_flag(optlang::Select([arg0, arg1]));
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
            if let optlang::Project([id1, id2]) = node {
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
        let (id, is_new) = self.egraph.add_with_flag(optlang::Project([arg0, arg1]));
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
            if let optlang::Join([id1, id2, id3]) = node {
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
        let (id, is_new) = self.egraph.add_with_flag(optlang::Join([arg0, arg1, arg2]));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }

        // If we created a new e-class, we need to run it to explore/optimize further
        if is_new {
            self.run(id);
        }
    }

    type extractor_scan_returns = ContextIterWrapper<ConstructorVec<source>, Self>;
    fn extractor_scan(&mut self, arg0: Id, returns: &mut Self::extractor_scan_returns) -> () {
        todo!("extractor_scan not implemented")
    }

    type constructor_scan_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn constructor_scan(
        &mut self,
        arg0: &source,
        returns: &mut Self::constructor_scan_returns,
    ) -> () {
        todo!("constructor_scan not implemented")
    }

    type extractor_table_scan_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn extractor_table_scan(
        &mut self,
        arg0: Id,
        returns: &mut Self::extractor_table_scan_returns,
    ) -> () {
        // Collect all matching terms first
        let mut matches = Vec::new();
        for node in self.egraph.nodes_in_class(arg0) {
            if let optlang::TableScan(id1) = node {
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

    type constructor_table_scan_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn constructor_table_scan(
        &mut self,
        arg0: Id,
        returns: &mut Self::constructor_table_scan_returns,
    ) -> () {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(optlang::TableScan(arg0));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }

        // If we created a new e-class, we need to run it to explore/optimize further
        if is_new {
            self.run(id);
        }
    }

    type extractor_index_scan_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn extractor_index_scan(
        &mut self,
        arg0: Id,
        returns: &mut Self::extractor_index_scan_returns,
    ) -> () {
        // Collect all matching terms first
        let mut matches = Vec::new();
        for node in self.egraph.nodes_in_class(arg0) {
            if let optlang::IndexScan(id1) = node {
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

    type constructor_index_scan_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn constructor_index_scan(
        &mut self,
        arg0: Id,
        returns: &mut Self::constructor_index_scan_returns,
    ) -> () {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(optlang::IndexScan(arg0));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }

        // If we created a new e-class, we need to run it to explore/optimize further
        if is_new {
            self.run(id);
        }
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
            if let optlang::NestedLoopJoin([id1, id2, id3]) = node {
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
            .add_with_flag(optlang::NestedLoopJoin([arg0, arg1, arg2]));
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
            if let optlang::HashJoin([id1, id2, id3]) = node {
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
            .add_with_flag(optlang::HashJoin([arg0, arg1, arg2]));
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
            if let optlang::MergeJoin([id1, id2, id3]) = node {
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
            .add_with_flag(optlang::MergeJoin([arg0, arg1, arg2]));
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
            if let optlang::Sort([id1, id2]) = node {
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
        let (id, is_new) = self.egraph.add_with_flag(optlang::Sort([arg0, arg1]));
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
        }
    }

    pub fn init(expr: RecExpr<optlang>, catalog: Catalog) -> Self {
        let mut context = Self::new(catalog);
        let id = context.egraph.add_expr(&expr);
        // TODO: add all the tables and indices to the catalog
        context.egraph.rebuild();
        context
    }

    pub fn run(&mut self, id: Id) {
        let mut id_set = Vec::new();
        rules::constructor_explore(self, id, &mut id_set);

        for id2 in id_set {
            self.egraph.union(id, id2);
        }

        self.egraph.rebuild();
        let canon_id = self.egraph.find(id);

        let mut id_set = Vec::new();
        rules::constructor_optimize(self, canon_id, &mut id_set);

        for id2 in id_set {
            self.egraph.union(canon_id, id2);
        }
        self.egraph.rebuild();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
