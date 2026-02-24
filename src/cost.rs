use egg::{CostFunction, Id};
use log::warn;
use std::cmp::max;

use crate::optimizer::OptimizerContext;
use crate::optlang::{Optlang, SimpleProperty};

#[derive(Debug, Clone, Eq, Hash)]
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
            properties: SimpleProperty::Bottom,
        }
    }
}

impl Default for Cost {
    fn default() -> Self {
        Self {
            cost: usize::MAX,
            cardinality: None,
            blocks: None,
            properties: SimpleProperty::Bottom,
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

// TODO: Update to satisfy property requirements inherently
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
                                // If block estimates are also equal, compare properties (Sorted < Unsorted < Bottom)
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
pub const CPU_COST: usize = 1;
pub const IO_COST: usize = 1000;
pub const TRANSFER_COST: usize = 10;
// HACK: Selectivity factor for filters, right now it is constant but eventually we will want to estimate it based on statistics
pub const SELECTIVITY_FACTOR: f64 = 0.5;
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
