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
use egg::{CostFunction, Extractor, Language, RecExpr};
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
use log::{debug, warn};
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

#[derive(Debug, Clone)]
enum Task {
    /// OptimizeGroup(group_id, group_explored, exprs_optimized)
    OptimizeGroup(Id, bool, bool),
    /// OptimizeExpr(expr_id, children_optimized)
    OptimizeExpr(Id, bool),
    /// ExploreGroup(group_id, exprs_explored)
    ExploreGroup(Id, bool),
    /// ExploreExpr(expr_id, children_explored)
    ExploreExpr(Id, bool),
    // NOTE: ApplyRules task is automatically handled by ISLE-generated code when we run the rules
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
    // Task stack
    task_stack: Vec<Task>,
    // Groups that are currently being explored (to prevent cycles)
    exploring_groups: HashSet<Id>,
    // Groups that have been fully explored
    explored_groups: HashSet<Id>,
    // Groups that have been fully optimized
    optimized_groups: HashSet<Id>,
    // Groups that are currently being optimized (to prevent cycles)
    optimizing_groups: HashSet<Id>,
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
                    x => {
                        warn!("TableScan node wrapped around: {:?}", x);
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
        let node = self.egraph.get_node(arg0);
        if let Optlang::Index(index_id) = node {
            Some(*index_id)
        } else {
            None
        }
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
        let node = self.egraph.get_node(arg0);
        if let Optlang::ColSet(colset_id) = node {
            Some(*colset_id)
        } else {
            None
        }
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
        let node = self.egraph.get_node(arg0);
        match node {
            Optlang::Int(i) => Some(value::Int { val: *i }),
            Optlang::Bool(b) => Some(value::Bool { val: *b }),
            Optlang::Str(s) => Some(value::Str { val: s.clone() }),
            _ => None,
        }
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

    fn extractor_add(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Add([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_add(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Add([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::OptimizeExpr(id, false));
        }
        id
    }

    fn extractor_sub(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Sub([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_sub(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Sub([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::OptimizeExpr(id, false));
        }
        id
    }

    fn extractor_mul(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Mul([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_mul(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Mul([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::OptimizeExpr(id, false));
        }
        id
    }

    fn extractor_div(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Div([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_div(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Div([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::OptimizeExpr(id, false));
        }
        id
    }

    fn extractor_eq(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Eq([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_eq(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Eq([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::OptimizeExpr(id, false));
        }
        id
    }

    fn extractor_lt(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Lt([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_lt(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Lt([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::OptimizeExpr(id, false));
        }
        id
    }

    fn extractor_gt(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Gt([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_gt(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Gt([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::OptimizeExpr(id, false));
        }
        id
    }

    fn extractor_le(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Le([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_le(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Le([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::OptimizeExpr(id, false));
        }
        id
    }

    fn extractor_ge(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Ge([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_ge(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Ge([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::OptimizeExpr(id, false));
        }
        id
    }

    fn extractor_ne(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Ne([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_ne(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Ne([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::OptimizeExpr(id, false));
        }
        id
    }

    fn extractor_and(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::And([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_and(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::And([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::OptimizeExpr(id, false));
        }
        id
    }

    fn extractor_or(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Or([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_or(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Or([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::OptimizeExpr(id, false));
        }
        id
    }

    fn extractor_not(&mut self, arg0: Id) -> Option<Id> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Not(id1) = node {
            Some(*id1)
        } else {
            None
        }
    }

    fn constructor_not(&mut self, arg0: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Not(arg0));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::OptimizeExpr(id, false));
        }
        id
    }

    fn extractor_select(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Select([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_select(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Select([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::OptimizeExpr(id, false));
        }
        id
    }

    fn extractor_project(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Project([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_project(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Project([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::OptimizeExpr(id, false));
        }
        id
    }

    fn extractor_join(&mut self, arg0: Id) -> Option<(Id, Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Join([id1, id2, id3]) = node {
            Some((*id1, *id2, *id3))
        } else {
            None
        }
    }

    fn constructor_join(&mut self, arg0: Id, arg1: Id, arg2: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Join([arg0, arg1, arg2]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::OptimizeExpr(id, false));
        }
        id
    }

    fn extractor_scan(&mut self, arg0: Id) -> Option<Id> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Scan(id) = node {
            Some(*id)
        } else {
            None
        }
    }

    fn constructor_scan(&mut self, arg0: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Scan(arg0));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::OptimizeExpr(id, false));
        }
        id
    }

    fn extractor_table_scan(&mut self, arg0: Id) -> Option<Id> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::TableScan(id) = node {
            Some(*id)
        } else {
            None
        }
    }

    fn constructor_table_scan(&mut self, arg0: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::TableScan(arg0));
        if is_new {
            self.task_stack.push(Task::OptimizeExpr(id, false));
        }
        id
    }

    fn extractor_index_scan(&mut self, arg0: Id) -> Option<Id> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::IndexScan(id) = node {
            Some(*id)
        } else {
            None
        }
    }

    fn constructor_index_scan(&mut self, arg0: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::IndexScan(arg0));
        if is_new {
            self.task_stack.push(Task::OptimizeExpr(id, false));
        }
        id
    }

    fn extractor_nested_loop_join(&mut self, arg0: Id) -> Option<(Id, Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::NestedLoopJoin([id1, id2, id3]) = node {
            Some((*id1, *id2, *id3))
        } else {
            None
        }
    }

    fn constructor_nested_loop_join(&mut self, arg0: Id, arg1: Id, arg2: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self
            .egraph
            .add_with_flag(Optlang::NestedLoopJoin([arg0, arg1, arg2]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::OptimizeExpr(id, false));
        }
        id
    }

    fn extractor_hash_join(&mut self, arg0: Id) -> Option<(Id, Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::HashJoin([id1, id2, id3]) = node {
            Some((*id1, *id2, *id3))
        } else {
            None
        }
    }

    fn constructor_hash_join(&mut self, arg0: Id, arg1: Id, arg2: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self
            .egraph
            .add_with_flag(Optlang::HashJoin([arg0, arg1, arg2]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::OptimizeExpr(id, false));
        }
        id
    }

    fn extractor_merge_join(&mut self, arg0: Id) -> Option<(Id, Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::MergeJoin([id1, id2, id3]) = node {
            Some((*id1, *id2, *id3))
        } else {
            None
        }
    }

    fn constructor_merge_join(&mut self, arg0: Id, arg1: Id, arg2: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self
            .egraph
            .add_with_flag(Optlang::MergeJoin([arg0, arg1, arg2]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::OptimizeExpr(id, false));
        }
        id
    }

    fn extractor_sort(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Sort([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_sort(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Sort([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::OptimizeExpr(id, false));
        }
        id
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
            task_stack: Vec::new(),
            exploring_groups: HashSet::new(),
            explored_groups: HashSet::new(),
            optimizing_groups: HashSet::new(),
            optimized_groups: HashSet::new(),
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

    /// Run an optimize group task.
    /// This will explore the group and optimize all expressions in the group, and then mark the group as optimized.
    fn run_optimize_group(&mut self, task: Task) {
        // Unpack the task to get the ID, explored flag, and optimized flag
        let (id, explored, optimized) = match task {
            Task::OptimizeGroup(id, explored, optimized) => (id, explored, optimized),
            _ => panic!("run_optimize_group called with non-optimize task"),
        };
        debug!(
            "run_optimize_group: {:?}",
            Task::OptimizeGroup(id, explored, optimized)
        );
        // If we have already optimized this group, we can skip it
        if self.optimized_groups.contains(&id) {
            return;
        }
        // Otherwise, mark as in progress
        self.optimizing_groups.insert(id);
        // If we haven't explored this group yet, we need to explore it first
        if !explored {
            // Mark the task as having explored the group and push it back onto the queue
            self.task_stack
                .push(Task::OptimizeGroup(id, true, optimized));
            self.task_stack.push(Task::ExploreGroup(id, false));
            return;
        }
        // If we haven't optimized each expression in this group yet, we need to optimize them first
        if !optimized {
            // Mark the task as having optimized the group and push it back onto the queue
            self.task_stack
                .push(Task::OptimizeGroup(id, explored, true));
            // For each expression in the group, we need to run an optimize task on it first
            for (id, _) in self.egraph.nodes_in_class(id) {
                self.task_stack.push(Task::OptimizeExpr(id, false));
            }
            return;
        }
        // Otherwise, we have explored and optimized all expressions in the group, so we can mark this group as optimized
        self.optimized_groups.insert(id);
        self.optimizing_groups.remove(&id);
    }

    /// Run an optimize expr task.
    /// This will optimize all children of the expression, and then optimize the expression itself.
    fn run_optimize_expr(&mut self, task: Task) {
        // Extract the ID and optimized args from the task
        let (id, children_optimized) = match task {
            Task::OptimizeExpr(id, children_optimized) => (id, children_optimized),
            _ => panic!("run_optimize_expr called with non-optimize task"),
        };
        debug!(
            "run_optimize_expr: {:?}",
            Task::OptimizeExpr(id, children_optimized)
        );

        // If the args haven't been optimized yet, we need to optimize them first
        if !children_optimized {
            // Mark the task as having optimized args and push it back onto the queue
            self.task_stack.push(Task::OptimizeExpr(id, true));
            // For each argument, we need to run an optimize task on it first
            for child in self.egraph.get_node(id).children() {
                if self.optimized_groups.contains(child) || self.optimizing_groups.contains(child) {
                    // If the child group is currently being optimized or already optimized, we should skip optimizing this child for now and come back to it later
                    continue;
                }
                self.task_stack
                    .push(Task::OptimizeGroup(*child, false, false));
            }

            return;
        }

        // Once we've optimized the args, we can now optimize this node
        // TODO: Implement optimization rules here.
        warn!("run_optimize_expr not implemented yet, we should add optimization rules here");
    }

    /// Run an explore group task.
    /// This will explore all expressions in the group.
    fn run_explore_group(&mut self, task: Task) {
        // Extract the ID and explored flag from the task
        let (id, explored) = match task {
            Task::ExploreGroup(id, explored) => (id, explored),
            _ => panic!("run_explore_group called with non-explore task"),
        };
        debug!("run_explore_group: {:?}", Task::ExploreGroup(id, explored));

        // If we have already explored this group, we can skip it
        if self.explored_groups.contains(&id) {
            return;
        }
        self.exploring_groups.insert(id);

        // If we haven't explored group expression yet, create those tasks
        if !explored {
            // Mark the task as having explored group expression and push it back onto the queue
            self.task_stack.push(Task::ExploreGroup(id, true));
            // For each expression in the group, we need to run an explore task on it first
            for (node_id, _) in self.egraph.nodes_in_class(id) {
                self.task_stack.push(Task::ExploreExpr(node_id, false));
            }

            return;
        }

        // Otherwise, we have explored all expressions in the group, so we can mark this group as explored
        self.explored_groups.insert(id);
        self.exploring_groups.remove(&id);
    }

    /// Run an explore expr task.
    /// This will explore all children of the expression, and then explore the expression itself by running
    /// the constructor functions for the expression, which may add new expressions to the e-graph that are equivalent to this expression.
    fn run_explore_expr(&mut self, task: Task) {
        // Extract the ID and explored args from the task
        let (id, children_explored) = match task {
            Task::ExploreExpr(id, children_explored) => (id, children_explored),
            _ => panic!("run_explore_expr called with non-explore task"),
        };
        debug!(
            "run_explore_expr: {:?}",
            Task::ExploreExpr(id, children_explored)
        );

        // If the args haven't been explored yet, we need to explore them first
        if !children_explored {
            // Mark the task as having explored args and push it back onto the queue
            self.task_stack.push(Task::ExploreExpr(id, true));
            // For each argument, we need to run an explore task on it first
            for child in self.egraph.get_node(id).children() {
                if self.explored_groups.contains(child) || self.exploring_groups.contains(child) {
                    // If the child group is currently being explored, we have a cycle, so we should skip exploring this child for now and come back to it later
                    continue;
                }
                self.task_stack.push(Task::ExploreGroup(*child, false));
            }

            return;
        }

        // If we've already explored the args, we can now explore this node
        // NOTE: the constructor functions will add new tasks to explore newly created nodes
        let mut id_set = Vec::new();
        rules::constructor_explore(self, id, &mut id_set);

        // Process the merge queue to make sure the e-graph is up to date before we run any more tasks
        for new_id in id_set {
            self.egraph.union(id, new_id);
        }
        self.egraph.rebuild();
    }

    /// Run the optimizer starting from the given ID. This will explore and optimize all expressions equivalent to the given ID.
    pub fn run(&mut self, id: Id) {
        // Push the initial ID onto the _to_process stack
        self.task_stack.push(Task::OptimizeGroup(id, false, false));

        // Process all tasks in the stack
        while let Some(task) = self.task_stack.pop() {
            match task {
                Task::OptimizeGroup(_, _, _) => self.run_optimize_group(task),
                Task::OptimizeExpr(_, _) => self.run_optimize_expr(task),
                Task::ExploreExpr(_, _) => self.run_explore_expr(task),
                Task::ExploreGroup(_, _) => self.run_explore_group(task),
            }
        }
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

    /// Initialize the logger for tests at debug level
    fn init_logger() {
        let _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    }

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

    /// Helper function to create a RecExpr with a Table node
    /// The parser can't distinguish between Int(n) and Table(n), so we need to construct these manually
    fn make_table_expr(table_id: TableId) -> RecExpr<Optlang> {
        let mut expr = RecExpr::default();
        expr.add(Optlang::Table(table_id));
        expr
    }

    /// Helper function to create a RecExpr with a TableScan node referencing a Table
    fn make_table_scan_expr(table_id: TableId) -> RecExpr<Optlang> {
        let mut expr = RecExpr::default();
        let table_node_id = expr.add(Optlang::Table(table_id));
        expr.add(Optlang::TableScan(table_node_id));
        expr
    }

    /// Helper function to create a RecExpr with an Index node
    fn make_index_expr(index_id: IndexId) -> RecExpr<Optlang> {
        let mut expr = RecExpr::default();
        expr.add(Optlang::Index(index_id));
        expr
    }

    /// Helper function to create a RecExpr with an IndexScan node referencing an Index
    fn make_index_scan_expr(index_id: IndexId) -> RecExpr<Optlang> {
        let mut expr = RecExpr::default();
        let index_node_id = expr.add(Optlang::Index(index_id));
        expr.add(Optlang::IndexScan(index_node_id));
        expr
    }

    /// Helper function to create a RecExpr with a ColSet node
    fn make_colset_expr(colset_id: ColSetId) -> RecExpr<Optlang> {
        let mut expr = RecExpr::default();
        expr.add(Optlang::ColSet(colset_id));
        expr
    }

    #[test]
    fn test_create_context() {
        init_logger();
        let catalog = Catalog::new();
        let ctx = OptimizerContext::new(catalog);
        assert_eq!(ctx.egraph.total_number_of_nodes(), 0);
    }

    #[test]
    fn test_catalog_loaded_correctly() {
        init_logger();

        // Create a complex catalog with multiple tables and indexes
        let mut catalog = Catalog::new();

        // Create users table
        let users_table_id = catalog
            .create_table_with_cols(
                "users".to_string(),
                vec![
                    ("user_id".to_string(), DataType::Int),
                    ("username".to_string(), DataType::String),
                    ("email".to_string(), DataType::String),
                    ("age".to_string(), DataType::Int),
                    ("created_at".to_string(), DataType::String),
                ],
            )
            .expect("Failed to create users table");

        // Create orders table
        let orders_table_id = catalog
            .create_table_with_cols(
                "orders".to_string(),
                vec![
                    ("order_id".to_string(), DataType::Int),
                    ("user_id".to_string(), DataType::Int),
                    ("product_name".to_string(), DataType::String),
                    ("quantity".to_string(), DataType::Int),
                    ("price".to_string(), DataType::Int),
                    ("status".to_string(), DataType::String),
                ],
            )
            .expect("Failed to create orders table");

        // Create products table
        let products_table_id = catalog
            .create_table_with_cols(
                "products".to_string(),
                vec![
                    ("product_id".to_string(), DataType::Int),
                    ("name".to_string(), DataType::String),
                    ("category".to_string(), DataType::String),
                    ("in_stock".to_string(), DataType::Bool),
                ],
            )
            .expect("Failed to create products table");

        // Create indexes
        let users_id_index = catalog
            .create_table_index(
                Some("idx_users_id".to_string()),
                "users".to_string(),
                vec!["user_id".to_string()],
            )
            .expect("Failed to create users id index");

        let orders_user_index = catalog
            .create_table_index(
                None, // Let catalog generate the name
                "orders".to_string(),
                vec!["user_id".to_string()],
            )
            .expect("Failed to create orders user_id index");

        let products_name_index = catalog
            .create_table_index(
                Some("idx_products_name".to_string()),
                "products".to_string(),
                vec!["name".to_string()],
            )
            .expect("Failed to create products name index");

        // Create optimizer context with the catalog
        let ctx = OptimizerContext::new(catalog);

        // Verify that all tables are present in the catalog
        assert_eq!(ctx.catalog.table_ids.len(), 3, "Should have 3 tables");
        assert_eq!(ctx.catalog.tables.len(), 3, "Should have 3 table entries");

        // Verify table IDs are correct
        assert_eq!(users_table_id, 1, "users should have table_id 1");
        assert_eq!(orders_table_id, 2, "orders should have table_id 2");
        assert_eq!(products_table_id, 3, "products should have table_id 3");

        // Verify table names exist
        assert!(ctx.catalog.table_ids.contains_key("users"));
        assert!(ctx.catalog.table_ids.contains_key("orders"));
        assert!(ctx.catalog.table_ids.contains_key("products"));

        // Verify table IDs map correctly
        assert_eq!(ctx.catalog.table_ids.get("users"), Some(&1));
        assert_eq!(ctx.catalog.table_ids.get("orders"), Some(&2));
        assert_eq!(ctx.catalog.table_ids.get("products"), Some(&3));

        // Verify users table structure
        let users = ctx
            .catalog
            .get_table("users")
            .expect("users table should exist");
        assert_eq!(users.id, users_table_id);
        assert_eq!(users.name, "users");
        assert_eq!(users.num_columns(), 5, "users should have 5 columns");
        assert!(users.get_column("user_id").is_some());
        assert!(users.get_column("username").is_some());
        assert!(users.get_column("email").is_some());
        assert!(users.get_column("age").is_some());
        assert!(users.get_column("created_at").is_some());

        // Verify orders table structure
        let orders = ctx
            .catalog
            .get_table("orders")
            .expect("orders table should exist");
        assert_eq!(orders.id, orders_table_id);
        assert_eq!(orders.name, "orders");
        assert_eq!(orders.num_columns(), 6, "orders should have 6 columns");
        assert!(orders.get_column("order_id").is_some());
        assert!(orders.get_column("user_id").is_some());
        assert!(orders.get_column("product_name").is_some());

        // Verify products table structure
        let products = ctx
            .catalog
            .get_table("products")
            .expect("products table should exist");
        assert_eq!(products.id, products_table_id);
        assert_eq!(products.name, "products");
        assert_eq!(products.num_columns(), 4, "products should have 4 columns");
        assert!(products.get_column("product_id").is_some());
        assert!(products.get_column("name").is_some());
        assert!(products.get_column("in_stock").is_some());

        // Verify indexes are present
        assert_eq!(ctx.catalog.index_ids.len(), 3, "Should have 3 indexes");
        assert_eq!(ctx.catalog.indexes.len(), 3, "Should have 3 index entries");

        // Verify index IDs are correct
        assert_eq!(users_id_index, 1, "users_id index should have index_id 1");
        assert_eq!(
            orders_user_index, 2,
            "orders_user index should have index_id 2"
        );
        assert_eq!(
            products_name_index, 3,
            "products_name index should have index_id 3"
        );

        // Verify index names exist
        assert!(ctx.catalog.index_ids.contains_key("idx_users_id"));
        assert!(ctx.catalog.index_ids.contains_key("orders_user_id")); // Auto-generated name
        assert!(ctx.catalog.index_ids.contains_key("idx_products_name"));

        // Verify index structures
        let users_idx = ctx
            .catalog
            .get_index("idx_users_id")
            .expect("users index should exist");
        assert_eq!(users_idx.id, users_id_index);
        assert_eq!(users_idx.table_id, users_table_id);
        assert_eq!(users_idx.column_ids.len(), 1);

        let orders_idx = ctx
            .catalog
            .get_index("orders_user_id")
            .expect("orders index should exist");
        assert_eq!(orders_idx.id, orders_user_index);
        assert_eq!(orders_idx.table_id, orders_table_id);
        assert_eq!(orders_idx.column_ids.len(), 1);

        let products_idx = ctx
            .catalog
            .get_index("idx_products_name")
            .expect("products index should exist");
        assert_eq!(products_idx.id, products_name_index);
        assert_eq!(products_idx.table_id, products_table_id);
        assert_eq!(products_idx.column_ids.len(), 1);

        // Verify the e-graph is empty (no expressions added yet)
        assert_eq!(ctx.egraph.total_number_of_nodes(), 0);

        // Verify colsets is initially empty
        assert_eq!(ctx.colsets.len(), 0);
        assert_eq!(ctx.next_colset_id, 1);

        // Verify task stack is empty
        assert_eq!(ctx.task_stack.len(), 0);

        // Verify no groups are marked as explored or optimized
        assert_eq!(ctx.explored_groups.len(), 0);
        assert_eq!(ctx.optimized_groups.len(), 0);
    }

    #[test]
    fn test_init_simple_expression() {
        init_logger();
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
        init_logger();
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
        init_logger();
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
        init_logger();
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
        init_logger();
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
        init_logger();
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
        init_logger();
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
        init_logger();
        let catalog = create_test_catalog();
        let mut ctx = OptimizerContext::new(catalog);

        // Get the table ID for "users" - it should be 1 (first table created)
        let table_id = *ctx.catalog.table_ids.get("users").unwrap();

        // Create a TableScan expression programmatically
        // Note: We can't use the parser because it can't distinguish between Int(1) and Table(1)
        let expr = make_table_scan_expr(table_id);
        
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
        init_logger();
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
        init_logger();
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
