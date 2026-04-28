/// Database Query Optimizer Example
///
/// This example demonstrates how to use the intarsia optimizer framework to build
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
use intarsia::{ExplorerHooks, OptimizerFramework, PropertyAnalysis};
use intarsia_macros::isle_integration_full;

use catalog::Catalog;
use cost::{DbStats, DbTransfer};
use language::Optlang;
use property::SimpleProperty;
use types::{ColSet, ColSetId};

// --------------------------------------------
// ISLE Generated Code Integration
// --------------------------------------------
isle_integration_full! {
    path: "isle/rules.rs",
}
pub mod context;
// --------------------------------------------

/// User data for the database optimizer.
///
/// catalog: Database metadata (tables, columns, indexes)
/// colsets: Mapping of column sets to IDs for the optimizer
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

/// Analysis used by the database optimizer: the property mega-lattice driven by
/// `DbTransfer`, with `DbStats` as the logical sub-analysis.
pub type DbAnalysis = PropertyAnalysis<Optlang, SimpleProperty, usize, DbStats, DbTransfer>;

/// Database optimizer instance: cascades framework over `Optlang` with `DbAnalysis`.
pub type DbOptimizer = OptimizerFramework<Optlang, DbAnalysis, DbUserData>;

/// Convenience constructor that wires the catalog into both the analysis transfer
/// function and the user data. (Free function rather than inherent impl because
/// `DbOptimizer` is an alias of a foreign struct.)
pub fn new_db_optimizer(catalog: Catalog) -> DbOptimizer {
    let analysis = PropertyAnalysis::new(DbTransfer::new(catalog.clone()));
    let user_data = DbUserData::new(catalog);
    OptimizerFramework::new(analysis, user_data)
}

// Implement ExplorerHooks to integrate ISLE rewrite rules
impl ExplorerHooks<Optlang> for DbOptimizer {
    fn explore(&mut self, id: Id) -> Vec<Id> {
        let mut new_ids = Vec::new();
        rules::constructor_explore(self, id, &mut new_ids);
        new_ids
    }
}
