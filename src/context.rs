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
use std::{cmp::max, collections::HashSet};

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
    pub(crate) catalog: Catalog,
    /// Colsets represent references to groups of columns for projections and predicates
    pub(crate) colsets: BiMap<ColSet, ColSetId>,
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

#[derive(Debug, Clone, Eq)]
pub struct Cost {
    pub cost: usize,
    pub cardinality: Option<usize>,
    pub blocks: Option<usize>,
    pub properties: SimpleProperty,
}

impl Cost {
    pub fn new(
        cost: usize,
        cardinality: Option<usize>,
        blocks: Option<usize>,
        properties: SimpleProperty,
    ) -> Self {
        Self {
            cost,
            cardinality,
            blocks,
            properties,
        }
    }

    pub fn simple(cost: usize) -> Self {
        Self {
            cost,
            cardinality: None,
            blocks: None,
            properties: SimpleProperty::Irrelevant,
        }
    }
}

impl Default for Cost {
    fn default() -> Self {
        Self {
            cost: usize::MAX,
            cardinality: None,
            blocks: None,
            properties: SimpleProperty::Irrelevant,
        }
    }
}

impl PartialEq for Cost {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost
            && self.cardinality == other.cardinality
            && self.blocks == other.blocks
            && self.properties == other.properties
    }
}

impl PartialOrd for Cost {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // If costs are equal, compare cardinality estimates (lower is better)
        match self.cost.partial_cmp(&other.cost) {
            Some(std::cmp::Ordering::Equal) => {
                match self.cardinality.partial_cmp(&other.cardinality) {
                    Some(std::cmp::Ordering::Equal) => {
                        // If cardinality estimates are also equal, compare block estimates (lower is better)
                        match self.blocks.partial_cmp(&other.blocks) {
                            Some(std::cmp::Ordering::Equal) => {
                                // If block estimates are also equal, compare properties (Sorted < Unsorted < Irrelevant)
                                Some(self.properties.cmp(&other.properties))
                            }
                            other => other,
                        }
                    }
                    other => other,
                }
            }
            ord => ord,
        }
    }
}

// Relative base cost of CPU vs I/O vs Mem transfer
const CPU_COST: usize = 1;
const IO_COST: usize = 1000;
const TRANSFER_COST: usize = 10;
// HACK: Selectivity factor for filters, right now it is constant but eventually we will want to estimate it based on statistics
const SELECTIVITY_FACTOR: f64 = 0.5;
/// Implement a simple cost function for the optimizer context
///
/// The basic cost model relies on the relative cost of cpu and io operations:
/// $C_{cpu}$ = 1
/// $C_{io}$ = 1000
/// $_{transfer}$ = 10
///
/// The cost of an operator is then defined as the sum of the costs of its inputs plus the cost of the operator itself.
/// Cost(Expr) = Cost(Input1) + Cost(Input2) + ... + Cost(Operator)
impl CostFunction<Optlang> for OptimizerContext {
    // Cost is a tuple of (absolute cost, optional cardinality estimate, properties)
    type Cost = Cost;

    fn cost<C>(&mut self, enode: &Optlang, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        match enode {
            // Constant values have a cost of 0
            Optlang::Int(_) | Optlang::Bool(_) | Optlang::Str(_) => Cost::simple(0),
            // Add and Sub 2*CPU + cost of inputs
            Optlang::Add([x, y]) | Optlang::Sub([x, y]) => {
                let cost = 2usize
                    .saturating_mul(CPU_COST)
                    .saturating_add(costs(*x).cost)
                    .saturating_add(costs(*y).cost);
                Cost::simple(cost)
            }

            // Mul and Div 4*CPU + cost of inputs
            Optlang::Mul([x, y]) | Optlang::Div([x, y]) => {
                let cost = 4usize
                    .saturating_mul(CPU_COST)
                    .saturating_add(costs(*x).cost)
                    .saturating_add(costs(*y).cost);
                Cost::simple(cost)
            }
            // Comparison operators 1 * CPU + cost of inputs
            Optlang::Eq([x, y])
            | Optlang::Lt([x, y])
            | Optlang::Gt([x, y])
            | Optlang::Le([x, y])
            | Optlang::Ge([x, y])
            | Optlang::Ne([x, y])
            | Optlang::And([x, y])
            | Optlang::Or([x, y]) => {
                let cost = 1usize
                    .saturating_mul(CPU_COST)
                    .saturating_add(costs(*x).cost)
                    .saturating_add(costs(*y).cost);
                Cost::simple(cost)
            }
            Optlang::Not(x) => {
                let cost = 1usize.saturating_add(costs(*x).cost);
                Cost::simple(cost)
            }
            // Data sources have a cost of 0 (they are just references) but include cardinality estimates and properties
            Optlang::Table(table_id) => {
                // Lookup table in catalog to get number of rows, which we use as a cardinality estimate
                if let Some(table) = self.catalog.get_table_by_id(*table_id) {
                    Cost::new(
                        0,
                        Some(table.get_est_num_rows()),
                        Some(table.get_est_num_blocks()),
                        SimpleProperty::Unsorted,
                    )
                } else {
                    Cost::new(0, None, None, SimpleProperty::Unsorted)
                }
            }
            Optlang::Index(index_id) => {
                // Lookup index in catalog to get number of rows, which we use as a cardinality estimate
                if let Some(index) = self.catalog.get_index_by_id(*index_id) {
                    if let Some(table) = self.catalog.get_table_by_id(index.table_id) {
                        return Cost::new(
                            0,
                            Some(table.get_est_num_rows()),
                            Some(table.get_est_num_blocks()),
                            SimpleProperty::Sorted,
                        );
                    } else {
                        Cost::new(0, None, None, SimpleProperty::Sorted)
                    }
                } else {
                    Cost::new(0, None, None, SimpleProperty::Sorted)
                }
            }
            Optlang::ColSet(_) => Cost::simple(0),
            // Logical operators have a cost of usize::MAX to prevent extraction
            Optlang::Join(_) | Optlang::Scan(_) => Default::default(),
            // FIXME: Eventually SELECT and Project will go here, but for now we consider them physical

            // Table Scan: I/O cost per block + transfer cost per row.
            Optlang::TableScan(arg_id) => {
                match self.egraph.get_node(*arg_id) {
                    Optlang::Table(table_id) => table_id,
                    x => {
                        warn!("TableScan node {} wrapped around: {:?}", arg_id, x);
                        return Default::default();
                    }
                };
                let table_cost = costs(*arg_id);
                let cost = table_cost
                    .blocks
                    .unwrap_or(0)
                    .saturating_mul(IO_COST)
                    .saturating_add(
                        table_cost
                            .cardinality
                            .unwrap_or(0)
                            .saturating_mul(TRANSFER_COST),
                    );

                Cost::new(
                    cost,
                    table_cost.cardinality,
                    table_cost.blocks,
                    SimpleProperty::Unsorted,
                )
            }
            // Index Scan: (I/O cost + Cost Transfer) * rows
            // NOTE: This is a worst case, unclustered index scan cost estimate
            Optlang::IndexScan(arg_id) => {
                match self.egraph.get_node(*arg_id) {
                    Optlang::Index(index_id) => index_id,
                    x => {
                        warn!("IndexScan node {} wrapped around: {:?}", arg_id, x);
                        return Default::default();
                    }
                };
                let index_cost = costs(*arg_id);
                let cost = index_cost
                    .cardinality
                    .unwrap_or(0)
                    .saturating_mul(IO_COST.saturating_add(TRANSFER_COST));

                Cost::new(
                    cost,
                    index_cost.cardinality,
                    index_cost.blocks,
                    SimpleProperty::Sorted,
                )
            }
            // Select: cost of args + Cost of predicate * num rows
            //  - cardianlity = num rows * selectivity
            Optlang::Select([source, pred]) => {
                let source_cost = costs(*source);
                let pred_cost = costs(*pred);
                let cost = source_cost.cost.saturating_add(
                    pred_cost
                        .cost
                        .saturating_add(TRANSFER_COST)
                        .saturating_mul(source_cost.cardinality.unwrap_or(0)),
                );
                // HACK: This is gross
                let cardinality = SELECTIVITY_FACTOR * source_cost.cardinality.unwrap_or(0) as f64;
                let blocks = SELECTIVITY_FACTOR * source_cost.blocks.unwrap_or(0) as f64;

                Cost::new(
                    cost,
                    Some(cardinality as usize),
                    Some(blocks as usize),
                    source_cost.properties,
                )
            }
            // Projection: cost of args + (cost of transfer + cost of predicate) * num rows
            Optlang::Project([cols, source]) => {
                let source_cost = costs(*source);
                let pred_cost = costs(*cols);

                let cost = source_cost.cost.saturating_add(
                    pred_cost
                        .cost
                        .saturating_add(TRANSFER_COST)
                        .saturating_mul(source_cost.cardinality.unwrap_or(0)),
                );

                Cost::new(
                    cost,
                    source_cost.cardinality,
                    source_cost.blocks, // HACK: This is likely an overestimate, projection can reduce row size
                    source_cost.properties,
                )
            }
            // NestLoopJoin: Blocks left * I/O cost + (Block left + Blocks right) * I/O cost + (N_l * N_r) * CPU cost for predicate evaluation
            // NOTE: for all join, we assume a cardinality and block estimate of the larger source
            Optlang::NestedLoopJoin([left, right, pred]) => {
                let left_cost = costs(*left);
                let right_cost = costs(*right);
                let pred_cost = costs(*pred);
                let cost = left_cost
                    .blocks
                    .unwrap_or(0)
                    .saturating_mul(IO_COST)
                    .saturating_add(
                        left_cost
                            .blocks
                            .unwrap_or(0)
                            .saturating_mul(right_cost.blocks.unwrap_or(0))
                            .saturating_mul(IO_COST),
                    )
                    .saturating_add(
                        left_cost
                            .cardinality
                            .unwrap_or(0)
                            .saturating_mul(right_cost.cardinality.unwrap_or(0))
                            .saturating_mul(pred_cost.cost),
                    );
                let cardinality = max(
                    left_cost.cardinality.unwrap_or(0),
                    right_cost.cardinality.unwrap_or(0),
                );
                let blocks = max(
                    left_cost.blocks.unwrap_or(0),
                    right_cost.blocks.unwrap_or(0),
                );
                Cost::new(cost, Some(cardinality), Some(blocks), left_cost.properties)
            }
            // HashJoin: 3 * (B_l + B_r) * I/O cost + (N_l + N_r) * CPU cost for predicate evaluation
            Optlang::HashJoin([left, right, pred]) => {
                let left_cost = costs(*left);
                let right_cost = costs(*right);
                let pred_cost = costs(*pred);
                let cost = 3usize
                    .saturating_mul(
                        left_cost
                            .blocks
                            .unwrap_or(0)
                            .saturating_add(right_cost.blocks.unwrap_or(0))
                            .saturating_mul(IO_COST),
                    )
                    .saturating_add(
                        left_cost
                            .cardinality
                            .unwrap_or(0)
                            .saturating_add(right_cost.cardinality.unwrap_or(0))
                            .saturating_mul(pred_cost.cost),
                    );
                let cardinality = max(
                    left_cost.cardinality.unwrap_or(0),
                    right_cost.cardinality.unwrap_or(0),
                );
                let blocks = max(
                    left_cost.blocks.unwrap_or(0),
                    right_cost.blocks.unwrap_or(0),
                );

                Cost::new(cost, Some(cardinality), Some(blocks), left_cost.properties)
            }

            // MergeJoin: (B_l+B_r) * I/O cost + (N_l + N_r) * CPU cost for predicate evaluation
            Optlang::MergeJoin([left, right, pred]) => {
                let left_cost = costs(*left);
                let right_cost = costs(*right);
                let pred_cost = costs(*pred);
                let cost = left_cost
                    .blocks
                    .unwrap_or(0)
                    .saturating_add(right_cost.blocks.unwrap_or(0))
                    .saturating_mul(IO_COST)
                    .saturating_add(
                        left_cost
                            .cardinality
                            .unwrap_or(0)
                            .saturating_add(right_cost.cardinality.unwrap_or(0))
                            .saturating_mul(pred_cost.cost),
                    );
                let cardinality = max(
                    left_cost.cardinality.unwrap_or(0),
                    right_cost.cardinality.unwrap_or(0),
                );
                let blocks = max(
                    left_cost.blocks.unwrap_or(0),
                    right_cost.blocks.unwrap_or(0),
                );
                Cost::new(cost, Some(cardinality), Some(blocks), left_cost.properties)
            }
            // Sort: 3 * B * IO + N * log(N) * CPU
            Optlang::Sort([source, _cols]) => {
                let source_cost = costs(*source);
                if source_cost.properties == SimpleProperty::Sorted {
                    return source_cost; // Sorting is free if already sorted
                }
                let cost = 3usize
                    .saturating_mul(source_cost.blocks.unwrap_or(0))
                    .saturating_mul(IO_COST)
                    .saturating_add(
                        source_cost
                            .cardinality
                            .unwrap_or(0)
                            .saturating_mul(
                                (source_cost.cardinality.unwrap_or(0) as f64).log2() as usize
                            )
                            .saturating_mul(CPU_COST),
                    );
                Cost::new(
                    cost,
                    source_cost.cardinality,
                    source_cost.blocks,
                    SimpleProperty::Sorted,
                )
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
            self.task_stack.push(Task::ExploreExpr(id, false));
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
            self.task_stack.push(Task::ExploreExpr(id, false));
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
            self.task_stack.push(Task::ExploreExpr(id, false));
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
            self.task_stack.push(Task::ExploreExpr(id, false));
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
            self.task_stack.push(Task::ExploreExpr(id, false));
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
            self.task_stack.push(Task::ExploreExpr(id, false));
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
            self.task_stack.push(Task::ExploreExpr(id, false));
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
            self.task_stack.push(Task::ExploreExpr(id, false));
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
            self.task_stack.push(Task::ExploreExpr(id, false));
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
            self.task_stack.push(Task::ExploreExpr(id, false));
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
            self.task_stack.push(Task::ExploreExpr(id, false));
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
            self.task_stack.push(Task::ExploreExpr(id, false));
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
            self.task_stack.push(Task::ExploreExpr(id, false));
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

        // If we created a new e-class, push tasks to explore AND optimize it
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
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

        // If we created a new e-class, push tasks to explore AND optimize it
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
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

        // If we created a new e-class, push tasks to explore AND optimize it
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
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
            self.task_stack.push(Task::ExploreExpr(id, false));
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
            self.task_stack.push(Task::ExploreExpr(id, false));
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
            self.task_stack.push(Task::ExploreExpr(id, false));
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
            self.task_stack.push(Task::ExploreExpr(id, false));
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
            self.task_stack.push(Task::ExploreExpr(id, false));
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
            self.task_stack.push(Task::ExploreExpr(id, false));
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
            self.task_stack.push(Task::ExploreExpr(id, false));
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
            debug!("Group {:?} already explored, skipping", id);
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
        // TODO: move this to explore group
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

    pub fn extract(&mut self, id: Id) -> RecExpr<Optlang> {
        let (_cost, best_expr) = self.extract_with_cost(id);
        best_expr
    }

    pub fn extract_with_cost(&mut self, id: Id) -> (Cost, RecExpr<Optlang>) {
        let extractor = Extractor::new(&self.egraph, self.clone());
        let (cost, expr) = extractor.find_best(id);
        (cost, expr)
    }
}

#[allow(unused)]
#[cfg(test)]
mod tests {
    use log::info;

    use super::*;
    use crate::types::DataType;

    /// Initialize the logger for tests at debug level
    fn init_logger() {
        let _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    }

    /// Generate a diagram of the e-graph for debugging purposes
    fn generate_egraph_diagram(ctx: &OptimizerContext, filename: &str) {
        info!("Generating e-graph diagram to {}", filename);
        ctx.egraph
            .dot()
            .to_png(format!("target/{}.png", filename))
            .expect("Failed to generate e-graph diagram");
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
        let result1 = ctx.extract(id1);
        let result2 = ctx.extract(id2);

        assert_eq!(result1.to_string(), "(+ 1 2)");
        assert_eq!(result2.to_string(), "(* 3 4)");
    }

    // ==================== Cost Function Tests ====================

    #[test]
    fn test_cost_arithmetic_simple() {
        init_logger();
        let catalog = Catalog::new();
        let mut ctx = OptimizerContext::new(catalog);

        // Create a simple arithmetic expression: 1 + 2
        // Cost should be: 1 (for +) + 0 (for 1) + 0 (for 2) = 1
        let expr: RecExpr<Optlang> = "(+ 1 2)".parse().unwrap();
        let id = ctx.egraph.add_expr(&expr);

        let (cost, result) = ctx.extract_with_cost(id);

        assert_eq!(cost.cost, 2, "Cost of (+ 1 2) should be 2");
        assert_eq!(result.to_string(), "(+ 1 2)");
    }

    #[test]
    fn test_cost_arithmetic_nested() {
        init_logger();
        let catalog = Catalog::new();
        let mut ctx = OptimizerContext::new(catalog);

        // Create: (1 + 2) * 3
        // Cost should be: 1 (* op) + 1 (+ op) + 0 (constants) = 2
        let expr: RecExpr<Optlang> = "(* (+ 1 2) 3)".parse().unwrap();
        let id = ctx.egraph.add_expr(&expr);

        let (cost, result) = ctx.extract_with_cost(id);

        assert_eq!(cost.cost, 6, "Cost of (* (+ 1 2) 3) should be 6");
        assert_eq!(result.to_string(), "(* (+ 1 2) 3)");
    }

    #[test]
    fn test_cost_arithmetic_complex() {
        init_logger();
        let catalog = Catalog::new();
        let mut ctx = OptimizerContext::new(catalog);

        // Create: ((1 + 2) * (3 - 4)) / 5
        // Cost: 1 (/) + 1 (*) + 1 (+) + 1 (-) + 0 (constants) = 4
        let expr: RecExpr<Optlang> = "(/ (* (+ 1 2) (- 3 4)) 5)".parse().unwrap();
        let id = ctx.egraph.add_expr(&expr);

        let (cost, result) = ctx.extract_with_cost(id);

        assert_eq!(cost.cost, 12, "Cost should be 12 (4 operators)");
        assert_eq!(result.to_string(), "(/ (* (+ 1 2) (- 3 4)) 5)");
    }

    #[test]
    fn test_cost_comparison_operations() {
        init_logger();
        let catalog = Catalog::new();
        let mut ctx = OptimizerContext::new(catalog);

        // Create: (a > 5) AND (b < 10)
        // Cost: 1 (AND) + 1 (>) + 1 (<) + 0 (constants) = 3
        let expr: RecExpr<Optlang> = "(AND (> 10 5) (< 3 10))".parse().unwrap();
        let id = ctx.egraph.add_expr(&expr);

        let (cost, result) = ctx.extract_with_cost(id);

        assert_eq!(cost.cost, 3, "Cost should be 3 (AND + > + <)");
    }

    #[test]
    fn test_cost_with_catalog_table_sizes() {
        init_logger();
        let mut catalog = Catalog::new();

        // Create tables with different sizes
        let small_table_id = catalog
            .create_table_with_cols(
                "small_table".to_string(),
                vec![("id".to_string(), DataType::Int)],
            )
            .unwrap();

        let large_table_id = catalog
            .create_table_with_cols(
                "large_table".to_string(),
                vec![("id".to_string(), DataType::Int)],
            )
            .unwrap();

        // Set table sizes
        catalog
            .get_table_by_id(small_table_id)
            .unwrap()
            .clone()
            .set_est_num_rows(100);
        catalog
            .tables
            .get_mut(&small_table_id)
            .unwrap()
            .set_est_num_rows(100);

        catalog
            .tables
            .get_mut(&large_table_id)
            .unwrap()
            .set_est_num_rows(10000);

        let mut ctx = OptimizerContext::new(catalog.clone());

        // Create TableScan for small table
        let small_scan = make_table_scan_expr(small_table_id);
        let small_id = ctx.egraph.add_expr(&small_scan);

        let (small_cost, _) = ctx.clone().extract_with_cost(small_id);
        let small_table = catalog.get_table_by_id(small_table_id).unwrap();
        let expected_cost = small_table.get_est_num_blocks() * IO_COST
            + small_table.get_est_num_rows() * TRANSFER_COST;
        assert_eq!(
            small_cost.cost, expected_cost,
            "Small table scan cost should be {expected_cost}"
        );

        // Create TableScan for large table
        let large_scan = make_table_scan_expr(large_table_id);
        let large_id = ctx.egraph.add_expr(&large_scan);

        let (large_cost, _) = ctx.extract_with_cost(large_id);
        let large_table = catalog.get_table_by_id(large_table_id).unwrap();
        let expected_large_cost = large_table.get_est_num_blocks() * IO_COST
            + large_table.get_est_num_rows() * TRANSFER_COST;

        assert_eq!(
            large_cost.cost, expected_large_cost,
            "Large table scan cost should be {expected_large_cost}"
        );
    }

    #[test]
    fn test_cost_chooses_cheaper_equivalent() {
        init_logger();
        let catalog = Catalog::new();
        let mut ctx = OptimizerContext::new(catalog);

        // Create two equivalent expressions with different costs
        // Expression 1: (1 + 2) + 3 = cost 2 (two + operators)
        let expr1: RecExpr<Optlang> = "(+ (+ 1 2) 3)".parse().unwrap();
        let id1 = ctx.egraph.add_expr(&expr1);

        // Expression 2: 1 + (2 + 3) = cost 2 (two + operators)
        let expr2: RecExpr<Optlang> = "(+ 1 (+ 2 3))".parse().unwrap();
        let id2 = ctx.egraph.add_expr(&expr2);

        // Union them to make them equivalent
        ctx.egraph.union(id1, id2);
        ctx.egraph.rebuild();

        // Extract should return one of them (both have same cost)
        let (cost, result) = ctx.extract_with_cost(id1);
        assert_eq!(cost.cost, 4, "Cost should be 4 for either expression");

        // Result should be one of the two forms
        let result_str = result.to_string();
        assert!(
            result_str == "(+ (+ 1 2) 3)" || result_str == "(+ 1 (+ 2 3))",
            "Result should be one of the equivalent forms"
        );
    }

    #[test]
    fn test_cost_prefers_cheaper_when_different() {
        init_logger();
        let catalog = Catalog::new();
        let mut ctx = OptimizerContext::new(catalog);

        // Create two expressions with significantly different costs
        // Cheap: 1 + 2 = cost 1
        let cheap_expr: RecExpr<Optlang> = "(+ 1 2)".parse().unwrap();
        let cheap_id = ctx.egraph.add_expr(&cheap_expr);

        // Expensive: ((1 + 2) * (3 - 4)) / 5 = cost 4
        let expensive_expr: RecExpr<Optlang> = "(/ (* (+ 1 2) (- 3 4)) 5)".parse().unwrap();
        let expensive_id = ctx.egraph.add_expr(&expensive_expr);

        // Union them (pretending they're equivalent for testing purposes)
        ctx.egraph.union(cheap_id, expensive_id);
        ctx.egraph.rebuild();

        // Extract should choose the cheaper one
        let (cost, result) = ctx.extract_with_cost(cheap_id);
        assert_eq!(cost.cost, 2, "Should choose cheaper expression with cost 2");
        assert_eq!(
            result.to_string(),
            "(+ 1 2)",
            "Should extract the cheaper expression"
        );
    }

    #[test]
    fn test_cost_index_scan_vs_table_scan() {
        init_logger();
        let mut catalog = Catalog::new();

        // Create a table
        let table_id = catalog
            .create_table_with_cols(
                "users".to_string(),
                vec![
                    ("id".to_string(), DataType::Int),
                    ("name".to_string(), DataType::String),
                ],
            )
            .unwrap();

        // Create an index on the table
        let index_id = catalog
            .create_table_index(
                Some("idx_users_id".to_string()),
                "users".to_string(),
                vec!["id".to_string()],
            )
            .unwrap();

        // Set table size
        catalog
            .tables
            .get_mut(&table_id)
            .unwrap()
            .set_est_num_rows(1000);

        let mut ctx = OptimizerContext::new(catalog.clone());

        // Create TableScan
        let table_scan = make_table_scan_expr(table_id);
        let table_scan_id = ctx.egraph.add_expr(&table_scan);

        // Create IndexScan
        let index_scan = make_index_scan_expr(index_id);
        let index_scan_id = ctx.egraph.add_expr(&index_scan);

        // Both should have same cost (table size) but different properties
        let (table_cost, _) = ctx.clone().extract_with_cost(table_scan_id);
        let table = catalog.get_table_by_id(table_id).unwrap();
        let expected_table_cost =
            table.get_est_num_blocks() * IO_COST + table.get_est_num_rows() * TRANSFER_COST;

        let (index_cost, _) = ctx.extract_with_cost(index_scan_id);
        let expected_index_cost = (IO_COST + TRANSFER_COST) * table.get_est_num_rows(); // Index scan cost is proportional to number of rows, but cheaper than full table scan

        assert_eq!(
            table_cost.cost, expected_table_cost,
            "TableScan cost should match expected table cost"
        );
        assert_eq!(
            index_cost.cost, expected_index_cost,
            "IndexScan cost should also match table size"
        );
    }

    #[test]
    fn test_cost_sort_optimization() {
        init_logger();
        let mut catalog = Catalog::new();

        // Create a table and index
        let table_id = catalog
            .create_table_with_cols(
                "data".to_string(),
                vec![("value".to_string(), DataType::Int)],
            )
            .unwrap();

        let index_id = catalog
            .create_table_index(None, "data".to_string(), vec!["value".to_string()])
            .unwrap();

        catalog
            .tables
            .get_mut(&table_id)
            .unwrap()
            .set_est_num_rows(500);

        let mut ctx = OptimizerContext::new(catalog.clone());

        // Create: SORT(TABLE_SCAN(table)) - unsorted input needs sorting
        let table_scan = make_table_scan_expr(table_id);
        let table_scan_id = ctx.egraph.add_expr(&table_scan);

        let mut sort_table_expr = RecExpr::default();
        let colset_id = sort_table_expr.add(Optlang::ColSet(1)); // Dummy colset
        let table_ref = sort_table_expr.add(Optlang::Table(table_id));
        let scan_node = sort_table_expr.add(Optlang::TableScan(table_ref));
        sort_table_expr.add(Optlang::Sort([scan_node, colset_id]));

        let sort_table_id = ctx.egraph.add_expr(&sort_table_expr);

        // Cost: 500 (table scan) + 0 (colset) + 50 (sort) = 550
        let (sort_table_cost, _) = ctx.clone().extract_with_cost(sort_table_id);
        let table = catalog.get_table_by_id(table_id).unwrap();
        let expected_sort_table_cost = 3 * IO_COST * table.get_est_num_blocks()
            + table.get_est_num_rows()
                * ((table.get_est_num_rows() as f64).log2() as usize)
                * CPU_COST; // Sorting cost is proportional to number of rows and log(rows)

        assert_eq!(
            sort_table_cost.cost, expected_sort_table_cost,
            "Sorting unsorted table should cost {} (expected {})",
            sort_table_cost.cost, expected_sort_table_cost
        );

        // Create: SORT(INDEX_SCAN(index)) - already sorted, sort is free
        let mut sort_index_expr = RecExpr::default();
        let colset_id2 = sort_index_expr.add(Optlang::ColSet(1));
        let index_ref = sort_index_expr.add(Optlang::Index(index_id));
        let index_scan_node = sort_index_expr.add(Optlang::IndexScan(index_ref));
        let index_id = ctx.egraph.add_expr(&sort_index_expr);
        sort_index_expr.add(Optlang::Sort([index_scan_node, colset_id2]));

        let sort_index_id = ctx.egraph.add_expr(&sort_index_expr);

        // Cost: 500 (index scan) + 0 (colset) + 0 (sort skipped) = 500
        let (sort_index_cost, _) = ctx.extract_with_cost(sort_index_id);
        let (index_cost, _) = ctx.extract_with_cost(index_id);
        assert_eq!(
            sort_index_cost.cost, index_cost.cost,
            "Sorting already-sorted index should cost 500 (no sort)"
        );
    }

    #[test]
    #[ignore = "Optimization pass not fully implemented to require sorted inputs for MergeJoin yet"]
    fn test_cost_merge_join_requires_sorted_inputs() {
        init_logger();
        let mut catalog = Catalog::new();

        let table1_id = catalog
            .create_table_with_cols(
                "table1".to_string(),
                vec![("id".to_string(), DataType::Int)],
            )
            .unwrap();

        let table2_id = catalog
            .create_table_with_cols(
                "table2".to_string(),
                vec![("id".to_string(), DataType::Int)],
            )
            .unwrap();

        let index1_id = catalog
            .create_table_index(None, "table1".to_string(), vec!["id".to_string()])
            .unwrap();

        let index2_id = catalog
            .create_table_index(None, "table2".to_string(), vec!["id".to_string()])
            .unwrap();

        catalog
            .tables
            .get_mut(&table1_id)
            .unwrap()
            .set_est_num_rows(100);
        catalog
            .tables
            .get_mut(&table2_id)
            .unwrap()
            .set_est_num_rows(200);

        let mut ctx = OptimizerContext::new(catalog);

        // Create MERGE_JOIN with unsorted inputs (table scans) - should be expensive
        let mut unsorted_merge = RecExpr::default();
        let t1 = unsorted_merge.add(Optlang::Table(table1_id));
        let scan1 = unsorted_merge.add(Optlang::TableScan(t1));
        let t2 = unsorted_merge.add(Optlang::Table(table2_id));
        let scan2 = unsorted_merge.add(Optlang::TableScan(t2));
        let pred = unsorted_merge.add(Optlang::Bool(true));
        unsorted_merge.add(Optlang::MergeJoin([scan1, scan2, pred]));

        let unsorted_id = ctx.egraph.add_expr(&unsorted_merge);
        let (unsorted_cost, _) = ctx.clone().extract_with_cost(unsorted_id);

        // Should be usize::MAX because inputs aren't sorted
        assert_eq!(
            unsorted_cost.cost,
            usize::MAX,
            "MergeJoin with unsorted inputs should have MAX cost"
        );

        // Create MERGE_JOIN with sorted inputs (index scans) - should be reasonable
        let mut sorted_merge = RecExpr::default();
        let i1 = sorted_merge.add(Optlang::Index(index1_id));
        let iscan1 = sorted_merge.add(Optlang::IndexScan(i1));
        let i2 = sorted_merge.add(Optlang::Index(index2_id));
        let iscan2 = sorted_merge.add(Optlang::IndexScan(i2));
        let pred2 = sorted_merge.add(Optlang::Bool(true));
        sorted_merge.add(Optlang::MergeJoin([iscan1, iscan2, pred2]));

        let sorted_id = ctx.egraph.add_expr(&sorted_merge);
        let (sorted_cost, _) = ctx.extract_with_cost(sorted_id);

        // Cost: 100 (scan1) + 200 (scan2) + 0 (pred) + 100 (join) = 400
        assert_eq!(
            sorted_cost.cost, 400,
            "MergeJoin with sorted inputs should cost 400"
        );
    }

    // ==================== Full Optimizer Workflow Tests ====================

    #[test]
    fn test_selection_pushdown_through_join() {
        init_logger();
        let mut catalog = Catalog::new();

        // Create two tables
        let customers_id = catalog
            .create_table_with_cols(
                "customers".to_string(),
                vec![
                    ("id".to_string(), DataType::Int),
                    ("name".to_string(), DataType::String),
                    ("age".to_string(), DataType::Int),
                ],
            )
            .unwrap();

        let orders_id = catalog
            .create_table_with_cols(
                "orders".to_string(),
                vec![
                    ("id".to_string(), DataType::Int),
                    ("customer_id".to_string(), DataType::Int),
                    ("amount".to_string(), DataType::Int),
                ],
            )
            .unwrap();

        // Set table sizes - customers is much smaller
        catalog
            .tables
            .get_mut(&customers_id)
            .unwrap()
            .set_est_num_rows(1000);
        catalog
            .tables
            .get_mut(&orders_id)
            .unwrap()
            .set_est_num_rows(10000);

        let mut ctx = OptimizerContext::new(catalog);

        // Build initial (unoptimized) expression:
        // SELECT(JOIN(customers, orders), age > 30)
        // This applies the filter AFTER the join (expensive!)
        let mut initial_expr = RecExpr::default();

        // Create scans
        let cust_table = initial_expr.add(Optlang::Table(customers_id));
        let cust_scan = initial_expr.add(Optlang::Scan(cust_table));

        let orders_table = initial_expr.add(Optlang::Table(orders_id));
        let orders_scan = initial_expr.add(Optlang::Scan(orders_table));

        // Join predicate (simplified - just true for this test)
        let join_pred = initial_expr.add(Optlang::Bool(true));

        // Join the tables
        let join_node = initial_expr.add(Optlang::Join([cust_scan, orders_scan, join_pred]));

        // Selection predicate: age > 30
        let age_val = initial_expr.add(Optlang::Int(30));
        let age_ref = initial_expr.add(Optlang::Int(0)); // Simplified column reference
        let select_pred = initial_expr.add(Optlang::Gt([age_ref, age_val]));

        // Apply selection after join (unoptimized!)
        initial_expr.add(Optlang::Select([join_node, select_pred]));

        let root_id = ctx.egraph.add_expr(&initial_expr);

        // Get initial cost
        let (initial_cost, initial_result) = ctx.clone().extract_with_cost(root_id);
        debug!("Initial expression: {}", initial_result);
        debug!("Initial cost: {:?}", initial_cost);

        // Run optimizer to explore equivalent expressions
        ctx.run(root_id);

        // Extract the best plan
        let (optimized_cost, optimized_result) = ctx.extract_with_cost(root_id);
        debug!("Optimized expression: {}", optimized_result);
        debug!("Optimized cost: {:?}", optimized_cost);

        // The optimized plan should have equal or lower cost
        assert!(
            optimized_cost <= initial_cost,
            "Optimized cost {:?} should be <= initial cost {:?}",
            optimized_cost,
            initial_cost
        );

        let optimized_str = optimized_result.to_string();
        debug!("Complete optimized plan: {}", optimized_str);

        // EXPECTED STRUCTURE:
        // Initial:  SELECT(JOIN(SCAN(customers), SCAN(orders)), age > 30)
        // Expected: PHYSICAL_JOIN(SELECT(TABLE_SCAN(customers), age > 30), TABLE_SCAN(orders), pred)
        //
        // The selection should be pushed down INSIDE the join, not wrapping it

        // 1. Must use physical join
        assert!(
            optimized_str.contains("HASH_JOIN")
                || optimized_str.contains("NESTED_LOOP_JOIN")
                || optimized_str.contains("MERGE_JOIN"),
            "Expected physical join, got: {}",
            optimized_str
        );

        // 2. Must use physical scans, not logical SCAN
        assert!(
            optimized_str.contains("TABLE_SCAN"),
            "Expected TABLE_SCAN, got: {}",
            optimized_str
        );

        // 3. Must NOT contain logical operators
        assert!(
            !optimized_str.contains("(SCAN ") && !optimized_str.contains("(JOIN "),
            "Expected no logical operators, got: {}",
            optimized_str
        );

        // 4. Check if selection was pushed down
        // If SELECT appears BEFORE the first JOIN, it wasn't pushed down
        if let Some(select_pos) = optimized_str.find("SELECT") {
            if let Some(join_pos) = optimized_str.find("JOIN") {
                assert!(
                    select_pos > join_pos,
                    "FAILED: SelectionSELECT was NOT pushed down through join.\\n\\\n                     Expected: PHYSICAL_JOIN(SELECT(TABLE_SCAN(...), pred), ...)\\n\\\n                     Got:      SELECT(PHYSICAL_JOIN(...), pred)\\n\\\n                     Plan: {}",
                    optimized_str
                );
            }
        }
    }

    #[test]
    fn test_selection_pushdown_through_projection() {
        init_logger();
        let mut catalog = Catalog::new();

        let table_id = catalog
            .create_table_with_cols(
                "employees".to_string(),
                vec![
                    ("id".to_string(), DataType::Int),
                    ("name".to_string(), DataType::String),
                    ("salary".to_string(), DataType::Int),
                    ("department".to_string(), DataType::String),
                ],
            )
            .unwrap();

        catalog
            .tables
            .get_mut(&table_id)
            .unwrap()
            .set_est_num_rows(5000);

        let mut ctx = OptimizerContext::new(catalog);

        // Build: SELECT(PROJECT(columns, scan), salary > 50000)
        // Optimizer should push selection before projection
        let mut initial_expr = RecExpr::default();

        let table = initial_expr.add(Optlang::Table(table_id));
        let scan = initial_expr.add(Optlang::Scan(table));

        // Project to subset of columns
        let colset = initial_expr.add(Optlang::ColSet(1)); // Simplified
        let project = initial_expr.add(Optlang::Project([colset, scan]));

        // Selection: salary > 50000
        let salary_val = initial_expr.add(Optlang::Int(50000));
        let salary_ref = initial_expr.add(Optlang::Int(2)); // Column 2 = salary
        let predicate = initial_expr.add(Optlang::Gt([salary_ref, salary_val]));

        initial_expr.add(Optlang::Select([project, predicate]));

        let root_id = ctx.egraph.add_expr(&initial_expr);
        let (initial_cost, initial_best) = ctx.clone().extract_with_cost(root_id);

        // Run optimizer
        ctx.run(root_id);

        let (optimized_cost, optimized_result) = ctx.extract_with_cost(root_id);
        debug!(
            "Initial cost: {:?}, Optimized cost: {:?}",
            initial_cost, optimized_cost
        );
        debug!("Initial plan: {}", initial_best);
        debug!("Optimized plan: {}", optimized_result);

        // Optimized should be equal or better
        assert!(
            optimized_cost <= initial_cost,
            "Optimized cost {:?} should be <= initial cost {:?}",
            optimized_cost,
            initial_cost
        );

        let optimized_str = optimized_result.to_string();
        let initial_str = initial_best.to_string();
        debug!("Initial plan:    {}", initial_str);
        debug!("Optimized plan:  {}", optimized_str);

        // EXPECTED STRUCTURE:
        // Initial:  SELECT(PROJECT(cols, SCAN(table)), salary > 50000)
        // Expected: PROJECT(cols, SELECT(TABLE_SCAN(table), salary > 50000))
        //
        // The selection should be pushed down INSIDE the projection

        // 1. PROJECT should be the outermost operator
        assert!(
            optimized_str.starts_with("(PROJECT"),
            "FAILED: PROJECT should be outermost operator.\\n\\\n             Expected: (PROJECT ... (SELECT ...))\\n\\\n             Got:      {}\\n\\\n             Selection was NOT pushed through projection!",
            optimized_str
        );

        // 2. SELECT should be nested inside PROJECT
        let project_pos = optimized_str.find("PROJECT").unwrap();
        let select_pos = optimized_str.find("SELECT");

        assert!(
            select_pos.is_some(),
            "FAILED: SELECT missing from plan. Got: {}",
            optimized_str
        );

        assert!(
            project_pos < select_pos.unwrap(),
            "FAILED: SELECT should be nested inside PROJECT.\\n\\\n             Expected: (PROJECT cols (SELECT (TABLE_SCAN) pred))\\n\\\n             Got:      {}\\n\\\n             Selection was NOT pushed through projection!",
            optimized_str
        );

        // 3. Must use physical scan
        assert!(
            optimized_str.contains("TABLE_SCAN"),
            "Expected TABLE_SCAN, got: {}",
            optimized_str
        );

        // 4. Must NOT have logical SCAN
        assert!(
            !optimized_str.contains("(SCAN "),
            "Expected no logical SCAN, got: {}",
            optimized_str
        );
    }

    #[test]
    fn test_combine_consecutive_selections() {
        init_logger();
        let mut catalog = Catalog::new();

        let table_id = catalog
            .create_table_with_cols(
                "products".to_string(),
                vec![
                    ("id".to_string(), DataType::Int),
                    ("price".to_string(), DataType::Int),
                    ("quantity".to_string(), DataType::Int),
                ],
            )
            .unwrap();

        catalog
            .tables
            .get_mut(&table_id)
            .unwrap()
            .set_est_num_rows(2000);

        let mut ctx = OptimizerContext::new(catalog);

        // Build: SELECT(SELECT(scan, price > 100), quantity > 10)
        // Should be combined into: SELECT(scan, price > 100 AND quantity > 10)
        let mut initial_expr = RecExpr::default();

        let table = initial_expr.add(Optlang::Table(table_id));
        let scan = initial_expr.add(Optlang::Scan(table));

        // First selection: price > 100
        let price_val = initial_expr.add(Optlang::Int(100));
        let price_ref = initial_expr.add(Optlang::Int(1));
        let pred1 = initial_expr.add(Optlang::Gt([price_ref, price_val]));
        let select1 = initial_expr.add(Optlang::Select([scan, pred1]));

        // Second selection: quantity > 10
        let qty_val = initial_expr.add(Optlang::Int(10));
        let qty_ref = initial_expr.add(Optlang::Int(2));
        let pred2 = initial_expr.add(Optlang::Gt([qty_ref, qty_val]));

        initial_expr.add(Optlang::Select([select1, pred2]));

        let root_id = ctx.egraph.add_expr(&initial_expr);
        let (initial_cost, initial_result) = ctx.clone().extract_with_cost(root_id);
        debug!("Initial: {}", initial_result);

        // Run optimizer
        ctx.run(root_id);

        let (optimized_cost, optimized_result) = ctx.extract_with_cost(root_id);
        debug!("Optimized: {}", optimized_result);

        // Cost should be same or better
        assert!(
            optimized_cost <= initial_cost,
            "Optimized cost {:?} should be <= initial cost {:?}",
            optimized_cost,
            initial_cost
        );

        let optimized_str = optimized_result.to_string();
        let initial_str = initial_result.to_string();
        debug!("Initial:    {}", initial_str);
        debug!("Optimized:  {}", optimized_str);

        // EXPECTED STRUCTURE:
        // Initial:  SELECT(SELECT(SCAN(table), pred1), pred2)
        // Expected: SELECT(TABLE_SCAN(table), AND(pred1, pred2))
        //
        // Two consecutive selections should be combined into one with AND

        // 1. Should have exactly ONE SELECT node
        let select_count = optimized_str.matches("SELECT").count();
        assert_eq!(
            select_count, 1,
            "FAILED: Should have exactly 1 SELECT (combined), got {}.\\n\\\n             Expected: (SELECT scan (AND pred1 pred2))\\n\\\n             Got:      {}\\n\\\n             Consecutive selections were NOT combined!",
            select_count, optimized_str
        );

        // 2. Must have AND combining the predicates
        assert!(
            optimized_str.contains("(AND "),
            "FAILED: Predicates should be combined with AND.\\n\\\n             Expected: (SELECT TABLE_SCAN (AND (> ...) (> ...)))\\n\\\n             Got:      {}\\n\\\n             Consecutive selections were NOT combined!",
            optimized_str
        );

        // 3. Verify structure: SELECT should be outermost
        assert!(
            optimized_str.starts_with("(SELECT"),
            "FAILED: SELECT should be outermost. Got: {}",
            optimized_str
        );

        // 4. Must use physical scan
        assert!(
            optimized_str.contains("TABLE_SCAN"),
            "Expected TABLE_SCAN, got: {}",
            optimized_str
        );

        // 5. Must NOT contain logical SCAN
        assert!(
            !optimized_str.contains("(SCAN "),
            "Expected no logical SCAN, got: {}",
            optimized_str
        );
    }

    #[test]
    fn test_join_physical_implementation_selection() {
        init_logger();
        let mut catalog = Catalog::new();

        // Create small and large tables
        let small_table_id = catalog
            .create_table_with_cols(
                "small_table".to_string(),
                vec![("id".to_string(), DataType::Int)],
            )
            .unwrap();

        let large_table_id = catalog
            .create_table_with_cols(
                "large_table".to_string(),
                vec![("id".to_string(), DataType::Int)],
            )
            .unwrap();

        // Create index on small table
        let _small_index = catalog
            .create_table_index(None, "small_table".to_string(), vec!["id".to_string()])
            .unwrap();

        catalog
            .tables
            .get_mut(&small_table_id)
            .unwrap()
            .set_est_num_rows(100);
        catalog
            .tables
            .get_mut(&large_table_id)
            .unwrap()
            .set_est_num_rows(10000);

        let mut ctx = OptimizerContext::new(catalog);

        // Build logical join
        let mut initial_expr = RecExpr::default();

        let small_table = initial_expr.add(Optlang::Table(small_table_id));
        let small_scan = initial_expr.add(Optlang::Scan(small_table));

        let large_table = initial_expr.add(Optlang::Table(large_table_id));
        let large_scan = initial_expr.add(Optlang::Scan(large_table));

        let pred = initial_expr.add(Optlang::Bool(true));

        initial_expr.add(Optlang::Join([small_scan, large_scan, pred]));

        let root_id = ctx.egraph.add_expr(&initial_expr);

        // Run optimizer - should explore different physical implementations
        ctx.run(root_id);

        let (optimized_cost, optimized_result) = ctx.extract_with_cost(root_id);
        let optimized_str = optimized_result.to_string();
        debug!("Optimized join plan: {}", optimized_str);
        debug!("Optimized cost: {:?}", optimized_cost);

        // EXPECTED STRUCTURE:
        // Initial:  JOIN(SCAN(t1), SCAN(t2), pred)
        // Expected: PHYSICAL_JOIN(TABLE_SCAN(t1), TABLE_SCAN(t2), pred)
        //           where PHYSICAL_JOIN is one of: HASH_JOIN, NESTED_LOOP_JOIN, MERGE_JOIN

        // 1. Must use physical join
        let has_physical_join = optimized_str.contains("HASH_JOIN")
            || optimized_str.contains("NESTED_LOOP_JOIN")
            || optimized_str.contains("MERGE_JOIN");
        assert!(
            has_physical_join,
            "FAILED: Must use physical join implementation.\\n\\\n             Expected: one of HASH_JOIN, NESTED_LOOP_JOIN, MERGE_JOIN\\n\\\n             Got:      {}\\n\\\n             Logical JOIN was NOT converted to physical!",
            optimized_str
        );

        // 2. Must NOT contain logical JOIN
        assert!(
            !optimized_str.contains("(JOIN "),
            "FAILED: Should not contain logical JOIN. Got: {}",
            optimized_str
        );

        // 3. Must use physical scans
        assert!(
            optimized_str.contains("TABLE_SCAN") || optimized_str.contains("INDEX_SCAN"),
            "FAILED: Must use physical scan. Got: {}",
            optimized_str
        );

        // 4. Must NOT contain logical SCAN
        assert!(
            !optimized_str.contains("(SCAN "),
            "FAILED: Should not contain logical SCAN. Got: {}",
            optimized_str
        );

        // 5. Should have exactly 1 join and 2 scans
        let join_count = optimized_str.matches("JOIN").count();
        assert_eq!(
            join_count, 1,
            "Expected exactly 1 join, got {}: {}",
            join_count, optimized_str
        );

        let scan_count = optimized_str.matches("SCAN").count();
        assert_eq!(
            scan_count, 2,
            "Expected exactly 2 scans, got {}: {}",
            scan_count, optimized_str
        );
    }

    #[test]
    fn test_complex_nested_optimization() {
        init_logger();
        let mut catalog = Catalog::new();

        // Create three tables for a complex query
        let users_id = catalog
            .create_table_with_cols(
                "users".to_string(),
                vec![
                    ("user_id".to_string(), DataType::Int),
                    ("age".to_string(), DataType::Int),
                ],
            )
            .unwrap();

        let orders_id = catalog
            .create_table_with_cols(
                "orders".to_string(),
                vec![
                    ("order_id".to_string(), DataType::Int),
                    ("user_id".to_string(), DataType::Int),
                    ("total".to_string(), DataType::Int),
                ],
            )
            .unwrap();

        let items_id = catalog
            .create_table_with_cols(
                "items".to_string(),
                vec![
                    ("item_id".to_string(), DataType::Int),
                    ("order_id".to_string(), DataType::Int),
                    ("price".to_string(), DataType::Int),
                ],
            )
            .unwrap();

        catalog
            .tables
            .get_mut(&users_id)
            .unwrap()
            .set_est_num_rows(1000);
        catalog
            .tables
            .get_mut(&orders_id)
            .unwrap()
            .set_est_num_rows(5000);
        catalog
            .tables
            .get_mut(&items_id)
            .unwrap()
            .set_est_num_rows(20000);

        let mut ctx = OptimizerContext::new(catalog);

        // Build complex query:
        // SELECT(JOIN(JOIN(users, orders), items), age > 25)
        // Unoptimized - joins everything then filters
        let mut initial_expr = RecExpr::default();

        let users_table = initial_expr.add(Optlang::Table(users_id));
        let users_scan = initial_expr.add(Optlang::Scan(users_table));

        let orders_table = initial_expr.add(Optlang::Table(orders_id));
        let orders_scan = initial_expr.add(Optlang::Scan(orders_table));

        let items_table = initial_expr.add(Optlang::Table(items_id));
        let items_scan = initial_expr.add(Optlang::Scan(items_table));

        // Join users and orders
        let pred1 = initial_expr.add(Optlang::Bool(true));
        let join1 = initial_expr.add(Optlang::Join([users_scan, orders_scan, pred1]));

        // Join result with items
        let pred2 = initial_expr.add(Optlang::Bool(true));
        let join2 = initial_expr.add(Optlang::Join([join1, items_scan, pred2]));

        // Filter by age
        let age_val = initial_expr.add(Optlang::Int(25));
        let age_ref = initial_expr.add(Optlang::Int(1));
        let age_pred = initial_expr.add(Optlang::Gt([age_ref, age_val]));

        initial_expr.add(Optlang::Select([join2, age_pred]));

        let root_id = ctx.egraph.add_expr(&initial_expr);
        let (initial_cost, initial_result) = ctx.clone().extract_with_cost(root_id);

        debug!("=== Complex Query Optimization ===");
        debug!("Initial plan: {}", initial_result);
        debug!("Initial cost: {:?}", initial_cost);

        // Run optimizer
        ctx.run(root_id);

        let (optimized_cost, optimized_result) = ctx.extract_with_cost(root_id);
        debug!("Optimized plan: {}", optimized_result);
        debug!("Optimized cost: {:?}", optimized_cost);

        // Should find a better plan
        assert!(
            optimized_cost <= initial_cost,
            "Optimized cost {:?} should be <= initial cost {:?}",
            optimized_cost,
            initial_cost
        );

        let optimized_str = optimized_result.to_string();

        // EXPECTED STRUCTURE:
        // Initial:  SELECT(JOIN(JOIN(SCAN(users), SCAN(orders)), SCAN(items)), age > 25)
        // Ideally expected:
        //   PHYSICAL_JOIN(
        //     PHYSICAL_JOIN(
        //       SELECT(TABLE_SCAN(users), age > 25),  // filter pushed to users scan
        //       TABLE_SCAN(orders)
        //     ),
        //     TABLE_SCAN(items)
        //   )
        // At minimum: all logical operators converted to physical

        // 1. All logical operators should be converted to physical
        assert!(
            !optimized_str.contains("(JOIN ") && !optimized_str.contains("(SCAN "),
            "FAILED: All logical operators should be physical.\\n\\\n             Expected: physical JOIN and SCAN only\\n\\\n             Got:      {}",
            optimized_str
        );

        // 2. Should have 2 physical joins for 3 tables
        let join_count = optimized_str.matches("JOIN").count();
        assert_eq!(
            join_count, 2,
            "FAILED: Should have exactly 2 joins for 3 tables, got {}. Got: {}",
            join_count, optimized_str
        );

        // 3. Should have 3 table scans (one per table)
        let scan_count = optimized_str.matches("TABLE_SCAN").count()
            + optimized_str.matches("INDEX_SCAN").count();
        assert_eq!(
            scan_count, 3,
            "FAILED: Should have exactly 3 scans for 3 tables, got {}. Got: {}",
            scan_count, optimized_str
        );

        // 4. Selection should be present
        assert!(
            optimized_str.contains("SELECT"),
            "FAILED: Selection should be present. Got: {}",
            optimized_str
        );

        // 5. Warn if selection wasn't pushed down (but don't fail - it's an optimization)
        if let Some(first_join_pos) = optimized_str.find("JOIN") {
            if let Some(select_pos) = optimized_str.find("SELECT") {
                if select_pos < first_join_pos {
                    eprintln!(
                        "\\nWARNING: Selection was NOT pushed down through joins!\\n\\\n                         Expected: PHYSICAL_JOIN(... SELECT(...) ..., ...)\\n\\\n                         Got:      SELECT(PHYSICAL_JOIN(...), ...)\\n\\\n                         Plan: {}\\n",
                        optimized_str
                    );
                }
            }
        }
    }

    #[test]
    fn test_arithmetic_simplification_in_query() {
        init_logger();
        let mut catalog = Catalog::new();

        let table_id = catalog
            .create_table_with_cols(
                "products".to_string(),
                vec![
                    ("id".to_string(), DataType::Int),
                    ("price".to_string(), DataType::Int),
                ],
            )
            .unwrap();

        catalog
            .tables
            .get_mut(&table_id)
            .unwrap()
            .set_est_num_rows(1000);

        let mut ctx = OptimizerContext::new(catalog);

        // Build query with arithmetic that can be simplified:
        // SELECT(scan, (price * 1) + 0 > 100)
        // Should simplify to: SELECT(scan, price > 100)
        let mut initial_expr = RecExpr::default();

        let table = initial_expr.add(Optlang::Table(table_id));
        let scan = initial_expr.add(Optlang::Scan(table));

        // Build: (price * 1) + 0
        let price_ref = initial_expr.add(Optlang::Int(1)); // Column reference
        let one = initial_expr.add(Optlang::Int(1));
        let mul_result = initial_expr.add(Optlang::Mul([price_ref, one]));
        let zero = initial_expr.add(Optlang::Int(0));
        let add_result = initial_expr.add(Optlang::Add([mul_result, zero]));

        // Compare to 100
        let hundred = initial_expr.add(Optlang::Int(100));
        let predicate = initial_expr.add(Optlang::Gt([add_result, hundred]));

        initial_expr.add(Optlang::Select([scan, predicate]));

        let root_id = ctx.egraph.add_expr(&initial_expr);
        let (initial_cost, initial_result) = ctx.clone().extract_with_cost(root_id);

        debug!("Initial with complex arithmetic: {}", initial_result);

        // Run optimizer
        ctx.run(root_id);

        let (optimized_cost, optimized_result) = ctx.extract_with_cost(root_id);
        debug!("Optimized simplified: {}", optimized_result);

        // Should be cheaper or equal
        assert!(
            optimized_cost <= initial_cost,
            "Optimized cost {:?} should be <= initial cost {:?}",
            optimized_cost,
            initial_cost
        );

        let initial_str = initial_result.to_string();
        let optimized_str = optimized_result.to_string();

        debug!("Initial arithmetic: {}", initial_str);
        debug!("Optimized arithmetic: {}", optimized_str);

        // EXPECTED STRUCTURE:
        // Initial:  SELECT(SCAN(table), (price * 1) + 0 > 100)
        // Expected: SELECT(TABLE_SCAN(table), price > 100)
        //
        // The arithmetic should be simplified: (x * 1) + 0 => x

        // 1. Should NOT contain \"* 1\"
        assert!(
            !optimized_str.contains("* 1"),
            "FAILED: Multiplication by 1 should be eliminated.\\n\\\n             Expected: ... price > 100\\n\\\n             Got:      {}\\n\\\n             Rule 'x * 1 => x' was NOT applied!",
            optimized_str
        );

        // 2. Should NOT contain \"+ 0\"
        assert!(
            !optimized_str.contains("+ 0"),
            "FAILED: Addition of 0 should be eliminated.\\n\\\n             Expected: ... price > 100\\n\\\n             Got:      {}\\n\\\n             Rule 'x + 0 => x' was NOT applied!",
            optimized_str
        );

        // 3. Should have fewer operators
        let initial_mul = initial_str.matches('*').count();
        let initial_add = initial_str.matches('+').count();
        let optimized_mul = optimized_str.matches('*').count();
        let optimized_add = optimized_str.matches('+').count();

        let initial_ops = initial_mul + initial_add;
        let optimized_ops = optimized_mul + optimized_add;

        assert!(
            optimized_ops < initial_ops,
            "FAILED: Should have fewer arithmetic operators.\\n\\\n             Initial: {} operators (* and +)\\n\\\n             Optimized: {} operators\\n\\\n             Initial:  {}\\n\\\n             Optimized: {}",
            initial_ops,
            optimized_ops,
            initial_str,
            optimized_str
        );

        // 4. Must use physical scan
        assert!(
            optimized_str.contains("TABLE_SCAN"),
            "Expected TABLE_SCAN, got: {}",
            optimized_str
        );
    }

    #[test]
    fn test_join_associativity_optimization() {
        init_logger();
        let mut catalog = Catalog::new();

        // Create three tables with different sizes
        let small_id = catalog
            .create_table_with_cols("small".to_string(), vec![("id".to_string(), DataType::Int)])
            .unwrap();

        let medium_id = catalog
            .create_table_with_cols(
                "medium".to_string(),
                vec![("id".to_string(), DataType::Int)],
            )
            .unwrap();

        let large_id = catalog
            .create_table_with_cols("large".to_string(), vec![("id".to_string(), DataType::Int)])
            .unwrap();

        catalog
            .tables
            .get_mut(&small_id)
            .unwrap()
            .set_est_num_rows(10);
        catalog
            .tables
            .get_mut(&medium_id)
            .unwrap()
            .set_est_num_rows(1000);
        catalog
            .tables
            .get_mut(&large_id)
            .unwrap()
            .set_est_num_rows(100000);

        let mut ctx = OptimizerContext::new(catalog);

        // Build: JOIN(small, JOIN(medium, large))
        // Optimizer might reorder to: JOIN(JOIN(small, medium), large) if beneficial
        let mut initial_expr = RecExpr::default();

        let small_table = initial_expr.add(Optlang::Table(small_id));
        let small_scan = initial_expr.add(Optlang::Scan(small_table));

        let medium_table = initial_expr.add(Optlang::Table(medium_id));
        let medium_scan = initial_expr.add(Optlang::Scan(medium_table));

        let large_table = initial_expr.add(Optlang::Table(large_id));
        let large_scan = initial_expr.add(Optlang::Scan(large_table));

        // Join medium and large first (potentially expensive)
        let pred1 = initial_expr.add(Optlang::Bool(true));
        let join1 = initial_expr.add(Optlang::Join([medium_scan, large_scan, pred1]));

        // Then join with small
        let pred2 = initial_expr.add(Optlang::Bool(true));
        initial_expr.add(Optlang::Join([small_scan, join1, pred2]));

        let root_id = ctx.egraph.add_expr(&initial_expr);
        let (initial_cost, initial_result) = ctx.clone().extract_with_cost(root_id);

        debug!("Initial join order: {}", initial_result);
        debug!("Initial cost: {:?}", initial_cost);

        // Run optimizer
        ctx.run(root_id);

        let (optimized_cost, optimized_result) = ctx.extract_with_cost(root_id);
        debug!("Optimized join order: {}", optimized_result);
        debug!("Optimized cost: {:?}", optimized_cost);

        // Optimizer should find equal or better plan
        assert!(
            optimized_cost <= initial_cost,
            "Optimized cost {:?} should be <= initial cost {:?}",
            optimized_cost,
            initial_cost
        );

        let optimized_str = optimized_result.to_string();

        // EXPECTED STRUCTURE:
        // Initial:  JOIN(SCAN(small), JOIN(SCAN(medium), SCAN(large)))
        // The optimizer should explore different join orders and physical implementations
        // Ideal might be different order, but must have physical operators

        // 1. No logical operators
        assert!(
            !optimized_str.contains("(JOIN ") && !optimized_str.contains("(SCAN "),
            "FAILED: Should use physical operators only. Got: {}",
            optimized_str
        );

        // 2. Should have exactly 2 joins for 3 tables
        let join_count = optimized_str.matches("JOIN").count();
        assert_eq!(
            join_count, 2,
            "FAILED: Should have exactly 2 joins for 3 tables, got {}. Got: {}",
            join_count, optimized_str
        );

        // 3. Should have exactly 3 physical scans
        let scan_count = optimized_str.matches("TABLE_SCAN").count()
            + optimized_str.matches("INDEX_SCAN").count();
        assert_eq!(
            scan_count, 3,
            "FAILED: Should have exactly 3 scans for 3 tables, got {}. Got: {}",
            scan_count, optimized_str
        );

        // 4. All table IDs should be present
        assert!(
            optimized_str.contains(&small_id.to_string())
                && optimized_str.contains(&medium_id.to_string())
                && optimized_str.contains(&large_id.to_string()),
            "FAILED: All 3 tables should be in plan. Got: {}",
            optimized_str
        );

        // 5. Cost should be reasonable (not overflow)
        assert!(
            optimized_cost.cost < usize::MAX / 2,
            "FAILED: Join cost should be reasonable, got {}",
            optimized_cost.cost
        );

        // Note: We don't assert a specific join order here, as the optimizer
        // might choose different orderings. The key is it explores alternatives
        // and the cost model drives the selection.
    }
}
