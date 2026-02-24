/// Database Query Optimizer Example
///
/// This example demonstrates how to use the Kymetica optimizer framework to build
/// a database query optimizer with a cascades-style optimization algorithm.
// Submodules
pub mod catalog;
pub mod language;
pub mod property;
pub mod types;

// ISLE-generated Context trait implementation (must be after types/language/property)
#[allow(dead_code, unused_variables, unused_imports, non_snake_case)]
#[allow(irrefutable_let_patterns, unused_assignments, non_camel_case_types)]
#[allow(unreachable_patterns, unreachable_code)]
pub mod context;

// Module imports
use bimap::BiMap;
use egg::Id;
use kymetica::{CostFunction, CostResult, ExplorerHooks, OptimizerFramework, isle_integration};
use log::{debug, warn};
use std::cmp::max;

use catalog::Catalog;
use language::Optlang;
use property::SimpleProperty;
use types::{ColSet, ColSetId};

// Cost constants
pub const CPU_COST: usize = 1;
pub const IO_COST: usize = 1000;
pub const TRANSFER_COST: usize = 10;
pub const SELECTIVITY_FACTOR: f64 = 0.5;

// --------------------------------------------
// ISLE Generated Code Integration
// --------------------------------------------
// Declare the ISLE-generated rules module
#[allow(dead_code, unused_variables, unused_imports, non_snake_case)]
#[allow(irrefutable_let_patterns, unused_assignments, non_camel_case_types)]
#[allow(unreachable_patterns, unreachable_code)]
#[path = "isle/rules.rs"]
pub(crate) mod rules;

// Generate required type definitions for ISLE
isle_integration!();
// --------------------------------------------

/// User data for the database optimizer.
///
/// This contains domain-specific information that the optimizer needs:
/// - catalog: Database metadata (tables, columns, indexes)
/// - colsets: Mapping of column sets to IDs for the optimizer
#[derive(Debug, Clone)]
pub struct DbUserData {
    pub catalog: Catalog,
    pub colsets: BiMap<ColSet, ColSetId>,
    pub next_colset_id: ColSetId,
}

impl DbUserData {
    pub fn new(catalog: Catalog) -> Self {
        Self {
            catalog,
            colsets: BiMap::new(),
            next_colset_id: 1,
        }
    }
}

/// Type alias for the database optimizer.
///
/// This is an instance of the generic OptimizerFramework specialized for:
/// - Language: Optlang (database query language)
/// - Property: SimpleProperty (sortedness tracking)
/// - UserData: DbUserData (catalog and column sets)
pub type DbOptimizer = OptimizerFramework<Optlang, SimpleProperty, DbUserData>;

// Implement CostFunction trait for the database optimizer
impl CostFunction<Optlang, SimpleProperty> for DbOptimizer {
    fn compute_cost<C>(&self, node: &Optlang, mut costs: C) -> CostResult<SimpleProperty>
    where
        C: FnMut(Id) -> CostResult<SimpleProperty>,
    {
        match node {
            // Constant values have zero cost
            Optlang::Int(_) | Optlang::Bool(_) | Optlang::Str(_) => CostResult::simple(0),

            // Arithmetic operations: 2*CPU for add/sub
            Optlang::Add([x, y]) | Optlang::Sub([x, y]) => {
                let cost = 2usize
                    .saturating_mul(CPU_COST)
                    .saturating_add(costs(*x).cost)
                    .saturating_add(costs(*y).cost);
                CostResult::simple(cost)
            }

            // Arithmetic operations: 4*CPU for mul/div
            Optlang::Mul([x, y]) | Optlang::Div([x, y]) => {
                let cost = 4usize
                    .saturating_mul(CPU_COST)
                    .saturating_add(costs(*x).cost)
                    .saturating_add(costs(*y).cost);
                CostResult::simple(cost)
            }

            // Comparison and logical operators: 1*CPU
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
                CostResult::simple(cost)
            }

            Optlang::Not(x) => {
                let cost = 1usize.saturating_add(costs(*x).cost);
                CostResult::simple(cost)
            }

            // Data sources: zero cost, but mark properties
            Optlang::Table(_table_id) => CostResult::new(0, SimpleProperty::Unsorted),

            Optlang::Index(_index_id) => CostResult::new(0, SimpleProperty::Sorted),

            Optlang::ColSet(_) => CostResult::simple(0),

            // Logical operators: max cost to prevent extraction
            Optlang::Join(_) | Optlang::Scan(_) => CostResult::default(),

            // Physical operators: Table Scan
            Optlang::TableScan(arg_id) => {
                let table_cost = costs(*arg_id);
                // Simplified: just use the child cost plus I/O overhead
                let cost = table_cost.cost.saturating_add(1000);
                CostResult::new(cost, SimpleProperty::Unsorted)
            }

            // Physical operators: Index Scan
            Optlang::IndexScan(arg_id) => {
                let index_cost = costs(*arg_id);
                // Simplified: index scan cost
                let cost = index_cost.cost.saturating_add(500);
                CostResult::new(cost, SimpleProperty::Sorted)
            }

            // Select: cost of source + predicate evaluation
            Optlang::Select([source, pred]) => {
                let source_cost = costs(*source);
                let pred_cost = costs(*pred);
                let cost = source_cost
                    .cost
                    .saturating_add(pred_cost.cost)
                    .saturating_add(100);
                // Preserve source properties (filter doesn't change sortedness)
                CostResult::new(cost, source_cost.properties)
            }

            // Project: cost of source + projection overhead
            Optlang::Project([_cols, source]) => {
                let source_cost = costs(*source);
                let cost = source_cost.cost.saturating_add(50);
                // Preserve source properties
                CostResult::new(cost, source_cost.properties)
            }

            // Nested Loop Join: expensive
            Optlang::NestedLoopJoin([left, right, pred]) => {
                let left_cost = costs(*left);
                let right_cost = costs(*right);
                let pred_cost = costs(*pred);
                let cost = left_cost
                    .cost
                    .saturating_add(right_cost.cost.saturating_mul(100))
                    .saturating_add(pred_cost.cost);
                CostResult::new(cost, left_cost.properties)
            }

            // Hash Join: moderate cost
            Optlang::HashJoin([left, right, pred]) => {
                let left_cost = costs(*left);
                let right_cost = costs(*right);
                let pred_cost = costs(*pred);
                let cost = left_cost
                    .cost
                    .saturating_add(right_cost.cost)
                    .saturating_add(pred_cost.cost)
                    .saturating_add(200);
                CostResult::new(cost, SimpleProperty::Unsorted)
            }

            // Merge Join: cheap but requires sorted inputs
            Optlang::MergeJoin([left, right, pred]) => {
                let left_cost = costs(*left);
                let right_cost = costs(*right);
                let pred_cost = costs(*pred);
                let cost = left_cost
                    .cost
                    .saturating_add(right_cost.cost)
                    .saturating_add(pred_cost.cost)
                    .saturating_add(50);
                CostResult::new(cost, SimpleProperty::Sorted)
            }

            // Sort: expensive if not already sorted
            Optlang::Sort([source, _cols]) => {
                let source_cost = costs(*source);
                if source_cost.properties == SimpleProperty::Sorted {
                    // Already sorted - no additional cost
                    source_cost
                } else {
                    // Need to sort - N log N cost
                    let cost = source_cost.cost.saturating_add(1000);
                    CostResult::new(cost, SimpleProperty::Sorted)
                }
            }
        }
    }
}

// Implement ExplorerHooks to integrate ISLE rewrite rules
impl ExplorerHooks<Optlang> for DbOptimizer {
    fn explore(&mut self, id: Id) -> Vec<Id> {
        let mut new_ids = Vec::new();
        // Call ISLE-generated explore function
        rules::constructor_explore(self, id, &mut new_ids);
        new_ids
    }
}
