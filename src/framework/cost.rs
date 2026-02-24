/// Cost model and cost function traits for the optimizer framework.
///
/// This module provides a generic cost model that supports property-aware optimization.
/// Users implement the `CostFunction` trait to define how to compute costs for their
/// specific language and property system.

use egg::{Id, Language};
use std::fmt::Debug;
use std::hash::Hash;

use crate::framework::property::Property;

/// Result of computing the cost of an expression.
///
/// This combines a numeric cost value with the properties that the expression provides.
/// The optimizer uses both the cost and properties to select the best expression that
/// satisfies required properties.
///
/// # Type Parameters
///
/// * `P` - The property type
///
/// # Examples
///
/// ```rust,ignore
/// // A table scan provides unsorted data at cost 1000
/// CostResult {
///     cost: 1000,
///     properties: SimpleProperty::Unsorted,
/// }
///
/// // An index scan provides sorted data at cost 500
/// CostResult {
///     cost: 500,
///     properties: SimpleProperty::Sorted,
/// }
/// ```
#[derive(Debug, Clone, Eq, Hash)]
pub struct CostResult<P: Property> {
    /// The numeric cost of this expression.
    ///
    /// Lower costs are better. The specific units and scale depend on your cost model.
    /// Common approaches include:
    /// - Abstract cost units (CPU + I/O operations)
    /// - Estimated execution time (milliseconds, seconds)
    /// - Resource consumption (bytes transferred, disk I/O)
    pub cost: usize,

    /// The properties that this expression provides.
    ///
    /// These properties are used to determine if the expression can satisfy
    /// the property requirements of parent operators.
    pub properties: P,
}

impl<P: Property> CostResult<P> {
    /// Create a new cost result.
    pub fn new(cost: usize, properties: P) -> Self {
        Self { cost, properties }
    }

    /// Create a cost result with bottom (no) properties.
    pub fn simple(cost: usize) -> Self {
        Self {
            cost,
            properties: P::bottom(),
        }
    }
}

impl<P: Property> Default for CostResult<P> {
    /// Default cost is maximum (worst possible).
    fn default() -> Self {
        Self {
            cost: usize::MAX,
            properties: P::bottom(),
        }
    }
}

impl<P: Property> PartialEq for CostResult<P> {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost && self.properties == other.properties
    }
}

impl<P: Property> PartialOrd for CostResult<P> {
    /// Compare costs. Lower costs are better.
    ///
    /// If costs are equal, we don't impose an ordering based on properties.
    /// The optimizer will handle property requirements separately.
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.cost.partial_cmp(&other.cost)
    }
}

/// Trait for computing costs of expressions in a language.
///
/// Users of the optimizer framework must implement this trait to define how costs
/// are computed for their specific language. The cost function receives a node
/// and a closure that provides the costs of children.
///
/// # Type Parameters
///
/// * `L` - The language type (must implement `Language`)
/// * `P` - The property type
///
/// # Implementation Notes
///
/// Your cost function should:
/// - Be deterministic (same inputs always produce same output)
/// - Use the provided closure to get child costs
/// - Return both a numeric cost and the properties the expression provides
/// - Handle saturation for arithmetic operations (use `saturating_add`, etc.)
/// - Consider domain-specific factors (I/O, CPU, memory, network, etc.)
///
/// # Examples
///
/// ```rust,ignore
/// impl CostFunction<QueryLang, SimpleProperty> for MyOptimizer {
///     fn compute_cost<C>(&self, node: &QueryLang, mut costs: C) -> CostResult<SimpleProperty>
///     where
///         C: FnMut(Id) -> CostResult<SimpleProperty>,
///     {
///         match node {
///             // Leaf nodes
///             QueryLang::Constant(_) => CostResult::simple(0),
///             
///             // Table scan: I/O cost proportional to table size
///             QueryLang::TableScan(table_id) => {
///                 let size = self.user_data.catalog.get_table_size(*table_id);
///                 CostResult::new(size * IO_COST, SimpleProperty::Unsorted)
///             }
///             
///             // Binary operations: cost of children + operation cost
///             QueryLang::Add([left, right]) => {
///                 let left_cost = costs(*left);
///                 let right_cost = costs(*right);
///                 let total = left_cost.cost.saturating_add(right_cost.cost).saturating_add(1);
///                 CostResult::simple(total)
///             }
///             
///             // Sort: expensive operation that produces sorted output
///             QueryLang::Sort([input, _cols]) => {
///                 let input_cost = costs(*input);
///                 if input_cost.properties == SimpleProperty::Sorted {
///                     // Already sorted - no additional cost
///                     input_cost
///                 } else {
///                     // N log N cost for sorting
///                     let sort_cost = estimate_sort_cost(input_size);
///                     CostResult::new(
///                         input_cost.cost.saturating_add(sort_cost),
///                         SimpleProperty::Sorted
///                     )
///                 }
///             }
///             
///             // ... other operators
///         }
///     }
/// }
/// ```
pub trait CostFunction<L: Language, P: Property> {
    /// Compute the cost of an expression node.
    ///
    /// # Arguments
    ///
    /// * `node` - The expression node to cost
    /// * `costs` - A closure that returns the cost of a child by its Id
    ///
    /// # Returns
    ///
    /// A `CostResult` containing the cost and properties of this expression.
    ///
    /// # Closure Behavior
    ///
    /// The `costs` closure will return:
    /// - During exploration: computed costs from the `CostFunction` (may recursively call this method)
    /// - During optimization: memoized costs from previous optimization passes
    ///
    /// Your implementation doesn't need to handle memoization - the framework does this.
    ///
    /// # Handling Children
    ///
    /// For nodes with children:
    /// 1. Call `node.children()` to get child Ids
    /// 2. Call `costs(child_id)` for each child to get its cost
    /// 3. Combine child costs according to your cost model
    /// 4. Add the cost of the operator itself
    /// 5. Determine what properties this expression provides
    ///
    /// # Cost Arithmetic
    ///
    /// Use saturating arithmetic to avoid overflow:
    /// - `cost1.saturating_add(cost2)`
    /// - `cost.saturating_mul(factor)`
    ///
    /// # Property Propagation
    ///
    /// Consider how properties flow through operators:
    /// - Some operators preserve properties (e.g., filter preserves sortedness)
    /// - Some operators produce new properties (e.g., sort produces sorted data)
    /// - Some operators destroy properties (e.g., hash join produces unsorted data)
    fn compute_cost<C>(&self, node: &L, costs: C) -> CostResult<P>
    where
        C: FnMut(Id) -> CostResult<P>;
}
