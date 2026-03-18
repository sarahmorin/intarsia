/// Database Query Optimizer Example
///
/// This example demonstrates how to use the Kymetica optimizer framework to build
/// a database query optimizer with a cascades-style optimization algorithm.
// Submodules
pub mod catalog;
pub mod cost;
pub mod language;
pub mod property;
pub mod types;

// Unit tests demonstrating usage patterns and validating implementation
#[cfg(test)]
mod tests;

// Module imports
use bimap::BiMap;
use egg::Id;
use intarsia::{CostDomain, CostFunction, ExplorerHooks, OptimizerFramework};
use intarsia_macros::isle_integration_full;
use std::cmp::max;

use catalog::Catalog;
use cost::DbCost;
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
// Single macro call to declare module and generate type definitions
isle_integration_full! {
    path: "isle/rules.rs",
}
pub mod context;
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
/// - CostDomain: DbCost (database-specific cost with cardinality/blocks)
/// - UserData: DbUserData (catalog and column sets)
pub type DbOptimizer = OptimizerFramework<Optlang, SimpleProperty, DbCost, DbUserData>;

// Implement CostFunction trait for the database optimizer
impl CostFunction<Optlang, SimpleProperty, DbCost> for DbOptimizer {
    fn compute_cost<CF>(&self, node: &Optlang, mut costs: CF) -> DbCost
    where
        CF: FnMut(Id) -> DbCost,
    {
        match node {
            // Constant values have zero cost
            Optlang::Int(_) | Optlang::Bool(_) | Optlang::Str(_) => DbCost::simple(0),

            // Arithmetic operations: 2*CPU for add/sub
            Optlang::Add([x, y]) | Optlang::Sub([x, y]) => {
                let cost = 2usize
                    .saturating_mul(CPU_COST)
                    .saturating_add(costs(*x).cost())
                    .saturating_add(costs(*y).cost());
                DbCost::simple(cost)
            }

            // Arithmetic operations: 4*CPU for mul/div
            Optlang::Mul([x, y]) | Optlang::Div([x, y]) => {
                let cost = 4usize
                    .saturating_mul(CPU_COST)
                    .saturating_add(costs(*x).cost())
                    .saturating_add(costs(*y).cost());
                DbCost::simple(cost)
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
                    .saturating_add(costs(*x).cost())
                    .saturating_add(costs(*y).cost());
                DbCost::simple(cost)
            }

            Optlang::Not(x) => {
                let cost = 1usize.saturating_add(costs(*x).cost());
                DbCost::simple(cost)
            }

            // Data sources: zero cost, but include cardinality estimates and properties
            Optlang::Table(table_id) => {
                // Lookup table in catalog to get number of rows
                if let Some(table) = self.user_data.catalog.tables.get(table_id) {
                    DbCost::new(
                        0,
                        Some(table.get_est_num_rows()),
                        Some(table.get_est_num_blocks()),
                        SimpleProperty::Unsorted,
                    )
                } else {
                    DbCost::new(0, None, None, SimpleProperty::Unsorted)
                }
            }

            Optlang::Index(index_id) => {
                // Lookup index in catalog to get number of rows
                if let Some(index) = self.user_data.catalog.indexes.get(index_id) {
                    if let Some(table) = self.user_data.catalog.tables.get(&index.table_id) {
                        DbCost::new(
                            0,
                            Some(table.get_est_num_rows()),
                            Some(table.get_est_num_blocks()),
                            SimpleProperty::Sorted,
                        )
                    } else {
                        DbCost::new(0, None, None, SimpleProperty::Sorted)
                    }
                } else {
                    DbCost::new(0, None, None, SimpleProperty::Sorted)
                }
            }

            Optlang::ColSet(_) => DbCost::simple(0),

            // Logical operators: max cost to prevent extraction
            Optlang::Join(_) | Optlang::Scan(_) => DbCost::default(),

            // Physical operators: Table Scan - I/O cost per block + transfer cost per row
            Optlang::TableScan(arg_id) => {
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

                DbCost::new(
                    cost,
                    table_cost.cardinality,
                    table_cost.blocks,
                    SimpleProperty::Unsorted,
                )
            }

            // Physical operators: Index Scan - (I/O cost + Transfer cost) * rows
            Optlang::IndexScan(arg_id) => {
                let index_cost = costs(*arg_id);
                let cost = index_cost
                    .cardinality
                    .unwrap_or(0)
                    .saturating_mul(IO_COST.saturating_add(TRANSFER_COST));

                DbCost::new(
                    cost,
                    index_cost.cardinality,
                    index_cost.blocks,
                    SimpleProperty::Sorted,
                )
            }

            // Select: cost of args + Cost of predicate * num rows, with selectivity
            Optlang::Select([source, pred]) => {
                let source_cost = costs(*source);
                let pred_cost = costs(*pred);
                let cost = source_cost.cost().saturating_add(
                    pred_cost
                        .cost()
                        .saturating_add(TRANSFER_COST)
                        .saturating_mul(source_cost.cardinality.unwrap_or(0)),
                );

                let cardinality =
                    (SELECTIVITY_FACTOR * source_cost.cardinality.unwrap_or(0) as f64) as usize;
                let blocks = (SELECTIVITY_FACTOR * source_cost.blocks.unwrap_or(0) as f64) as usize;

                DbCost::new(
                    cost,
                    Some(cardinality),
                    Some(blocks),
                    *source_cost.properties(),
                )
            }

            // Project: cost of args + (cost of transfer + cost of predicate) * num rows
            Optlang::Project([cols, source]) => {
                let source_cost = costs(*source);
                let pred_cost = costs(*cols);

                let cost = source_cost.cost().saturating_add(
                    pred_cost
                        .cost()
                        .saturating_add(TRANSFER_COST)
                        .saturating_mul(source_cost.cardinality.unwrap_or(0)),
                );

                DbCost::new(
                    cost,
                    source_cost.cardinality,
                    source_cost.blocks, // This may be an overestimate
                    *source_cost.properties(),
                )
            }

            // Nested Loop Join: Blocks left * I/O + (Blocks left * Blocks right) * I/O + (N_l * N_r) * CPU
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
                            .saturating_mul(pred_cost.cost()),
                    );
                let cardinality = max(
                    left_cost.cardinality.unwrap_or(0),
                    right_cost.cardinality.unwrap_or(0),
                );
                let blocks = max(
                    left_cost.blocks.unwrap_or(0),
                    right_cost.blocks.unwrap_or(0),
                );
                DbCost::new(
                    cost,
                    Some(cardinality),
                    Some(blocks),
                    *left_cost.properties(),
                )
            }

            // Hash Join: 3 * (B_l + B_r) * I/O + (N_l + N_r) * CPU
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
                            .saturating_mul(pred_cost.cost()),
                    );
                let cardinality = max(
                    left_cost.cardinality.unwrap_or(0),
                    right_cost.cardinality.unwrap_or(0),
                );
                let blocks = max(
                    left_cost.blocks.unwrap_or(0),
                    right_cost.blocks.unwrap_or(0),
                );

                DbCost::new(
                    cost,
                    Some(cardinality),
                    Some(blocks),
                    *left_cost.properties(),
                )
            }

            // Merge Join: (B_l+B_r) * I/O + (N_l + N_r) * CPU
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
                            .saturating_mul(pred_cost.cost()),
                    );
                let cardinality = max(
                    left_cost.cardinality.unwrap_or(0),
                    right_cost.cardinality.unwrap_or(0),
                );
                let blocks = max(
                    left_cost.blocks.unwrap_or(0),
                    right_cost.blocks.unwrap_or(0),
                );
                DbCost::new(
                    cost,
                    Some(cardinality),
                    Some(blocks),
                    *left_cost.properties(),
                )
            }

            // Sort: 3 * B * IO + N * log(N) * CPU - expensive if not already sorted
            Optlang::Sort([source, _cols]) => {
                let source_cost = costs(*source);
                if source_cost.properties() == &SimpleProperty::Sorted {
                    return source_cost; // Sorting is free if already sorted
                }
                let cardinality = source_cost.cardinality.unwrap_or(0);
                let log_factor = if cardinality > 0 {
                    (cardinality as f64).log2() as usize
                } else {
                    0
                };
                let cost = 3usize
                    .saturating_mul(source_cost.blocks.unwrap_or(0))
                    .saturating_mul(IO_COST)
                    .saturating_add(
                        cardinality
                            .saturating_mul(log_factor)
                            .saturating_mul(CPU_COST),
                    );
                DbCost::new(
                    cost,
                    source_cost.cardinality,
                    source_cost.blocks,
                    SimpleProperty::Sorted,
                )
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
