/// Task definitions for the cascades-style optimizer framework.
///
/// Tasks represent units of work in the optimization process. The optimizer uses a task queue
/// to systematically explore and optimize expressions in the e-graph. Each task type corresponds
/// to a different phase of the optimization process.
use egg::Id;

use crate::framework::property::Property;

/// Represents a unit of work in the cascades-style optimization process.
///
/// The optimizer uses tasks to manage the exploration and optimization workflow.
/// Tasks are processed from a stack, with dependencies ensuring correct ordering.
///
/// # Type Parameters
///
/// * `P` - The property type used by this optimizer instance
///
/// # Task Types
///
/// ## Exploration Tasks
/// Exploration generates logically equivalent expressions using rewrite rules.
///
/// - `ExploreGroup`: Explore all expressions in an equivalence class and apply rewrite rules to generate new expressions
/// - `ExploreChildren`: Explore the children of a specific expression node
///
/// ## Optimization Tasks
/// Optimization selects the lowest-cost expression satisfying required properties.
///
/// - `OptimizeGroup`: Find the best expression in an equivalence class for given properties
/// - `OptimizeExpr`: Optimize a specific expression (compute its cost)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Task<P: Property> {
    /// Optimize a group to find the best expression satisfying required properties.
    ///
    /// Fields:
    /// - `Id`: The equivalence class (group) to optimize
    /// - `P`: The properties required from this group
    /// - `bool`: Whether the group has been explored (false on first visit)
    /// - `bool`: Whether expressions in the group have been optimized (false on first visit)
    ///
    /// The optimizer uses these flags to track progress and avoid redundant work.
    /// When both flags are true, the task selects the best expression and memoizes it.
    OptimizeGroup(Id, P, bool, bool),

    /// Optimize a specific expression by computing its cost.
    ///
    /// Fields:
    /// - `Id`: The expression node to optimize
    /// - `bool`: Whether children have been optimized (false on first visit)
    ///
    /// This task ensures all children are optimized before computing the expression's cost.
    OptimizeExpr(Id, bool),

    /// Explore a group to generate all logically equivalent expressions.
    ///
    /// Fields:
    /// - `Id`: The equivalence class (group) to explore
    /// - `bool`: Whether expressions in the group have been explored (false on first visit)
    ///
    /// Exploration applies rewrite rules to generate new equivalent expressions.
    ExploreGroup(Id, bool),

    /// Explore the children of a specific expression node to ensure they are explored and optimized.
    ///
    /// Fields:
    /// - `Id`: The expression node whose children we want to explore
    ///
    /// Generates ExploreGroup tasks for each unexplored child group.
    ExploreChildren(Id),
}
