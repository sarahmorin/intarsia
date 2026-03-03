/// Cost model and cost function traits for the optimizer framework.
///
/// This module provides a generic cost model that supports property-aware optimization.
/// Users implement the `CostFunction` trait to define how to compute costs for their
/// specific language and property system.
use egg::{Id, Language};
use std::fmt::Debug;
use std::hash::Hash;

use crate::framework::property::Property;
use crate::{OptimizerFramework, PropertyAwareLanguage};

/// Trait for cost domains used by the optimizer.
///
/// A cost domain combines a raw cost value with properties and potentially other
/// domain-specific information (e.g., cardinality estimates, resource usage).
/// The cost domain must provide a partial order for comparing costs.
///
/// # Type Parameters
///
/// * `P` - The property type
///
/// # Required Types and Methods
///
/// - `RawCost`: The type of the raw cost value used for ordering (e.g., usize, f64).
/// - `cost()`: Returns the raw numeric cost for ordering
/// - `properties()`: Returns the properties this cost represents
///
/// # Examples
///
/// ```rust,ignore
/// // Simple cost domain with just cost and properties
/// impl<P: Property> CostDomain<P> for SimpleCost<P> {
///     type RawCost = usize;
///     fn cost(&self) -> usize { self.cost }
///     fn properties(&self) -> &P { &self.properties }
/// }
///
/// // Rich cost domain with cardinality estimates
/// struct DbCost<P: Property> {
///     cost: usize,
///     properties: P,
///     cardinality: Option<usize>,
///     blocks: Option<usize>,
/// }
///
/// impl<P: Property> CostDomain<P> for DbCost<P> {
///     type RawCost = usize;
///     fn cost(&self) -> usize { self.cost }
///     fn properties(&self) -> &P { &self.properties }
/// }
/// ```
///
///
/// # Implementation Notes
/// - The `cost()` method is used for ordering costs - lower values are better.
/// - The `properties()` method is used to determine if the expression satisfies property requirements.
/// - Implement `PartialOrd` for your cost domain to define how costs are compared. Typically, you would compare raw costs first, and then use properties as a tiebreaker if needed.
/// - Implement `Default` for your cost domain to provide a default "worst" cost (e.g., maximum cost value) we should avoid extracting.
/// - Implement `PartialEq` and `Eq` to define when two costs are considered equal, especially if you are embedding extra information for computing costs that you *don't* want to use when comparing costs.
/// - Consider using saturating arithmetic for cost computations to avoid overflow (e.g., `saturating_add`, `saturating_mul
///
pub trait CostDomain<P: Property>:
    Clone + Debug + PartialEq + Eq + PartialOrd + Hash + Default
{
    /// The type of the raw cost value used for ordering.
    /// This is typically a numeric type like usize or f64, but can be any type that implements Ord.
    type RawCost: Ord;

    /// Get the raw numeric cost value.
    ///
    /// This is used for ordering costs - lower values are better.
    fn cost(&self) -> Self::RawCost;

    /// Get the properties that this cost represents.
    fn properties(&self) -> &P;

    // TODO: Should I just require a min() and max() instead of using default as max??
}

/// A simple implementation of the `CostDomain` trait.
///
/// This combines a numeric cost value with properties. It's provided as a ready-to-use
/// cost domain for simple use cases. If you need more complex cost modeling (e.g., with
/// cardinality estimates, block counts, or resource consumption), implement your own
/// type that implements `CostDomain`.
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
pub struct SimpleCost<P: Property> {
    /// The numeric cost of this expression.
    ///
    /// Lower costs are better. The specific units and scale depend on your cost model.
    /// Common approaches include:
    /// - Abstract cost units (CPU + I/O operations)
    /// - Estimated execution time (milliseconds, seconds)
    /// - Resource consumption (bytes transferred, disk I/O)
    raw_cost: usize,

    /// The properties that this expression provides.
    ///
    /// These properties are used to determine if the expression can satisfy
    /// the property requirements of parent operators.
    props: P,
}

impl<P: Property> SimpleCost<P> {
    /// Create a new cost result.
    pub fn new(cost: usize, properties: P) -> Self {
        Self {
            raw_cost: cost,
            props: properties,
        }
    }

    /// Create a cost result with bottom (no) properties.
    pub fn simple(cost: usize) -> Self {
        Self {
            raw_cost: cost,
            props: P::bottom(),
        }
    }
}

impl<P: Property> CostDomain<P> for SimpleCost<P> {
    type RawCost = usize;

    fn cost(&self) -> usize {
        self.raw_cost
    }

    fn properties(&self) -> &P {
        &self.props
    }
}

impl<P: Property> Default for SimpleCost<P> {
    /// Default cost is maximum (worst possible).
    fn default() -> Self {
        Self {
            raw_cost: usize::MAX,
            props: P::bottom(),
        }
    }
}

impl<P: Property> PartialEq for SimpleCost<P> {
    fn eq(&self, other: &Self) -> bool {
        self.raw_cost == other.raw_cost && self.props == other.props
    }
}

impl<P: Property> PartialOrd for SimpleCost<P> {
    /// Compare costs. Lower costs are better.
    ///
    /// This implementation compares raw costs first, and then uses properties as a tiebreaker if costs are equal.
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.raw_cost.partial_cmp(&other.raw_cost) {
            Some(std::cmp::Ordering::Equal) => self.props.partial_cmp(&other.props),
            other => other,
        }
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
/// * `D` - The domain type for the raw cost value (must implement `Ord`)
/// * `C` - The cost domain type (must implement `CostDomain<P, D>`)
///
/// # Implementation Notes
///
/// Your cost function should:
/// - Be deterministic (same inputs always produce same output)
/// - Use the provided closure to get child costs
/// - Return a cost domain instance with both numeric cost and properties
/// - Handle saturation for arithmetic operations (use `saturating_add`, etc.)
/// - Consider domain-specific factors (I/O, CPU, memory, network, etc.)
///
/// # Examples
///
/// ```rust,ignore
/// // Using the simple CostResult domain
/// impl CostFunction<QueryLang, SimpleProperty, SimpleCost<SimpleProperty>> for MyOptimizer {
///     fn compute_cost<CF>(&self, node: &QueryLang, mut costs: CF)
///         -> CostResult<SimpleProperty>
///     where
///         CF: FnMut(Id) -> CostResult<SimpleProperty>,
///     {
///         match node {
///             QueryLang::Constant(_) => CostResult::simple(0),
///             QueryLang::Add([left, right]) => {
///                 let left_cost = costs(*left);
///                 let right_cost = costs(*right);
///                 let total = left_cost.cost().saturating_add(right_cost.cost()).saturating_add(1);
///                 CostResult::simple(total)
///             }
///             // ... other operators
///         }
///     }
/// }
///
/// // Using a custom DbCost domain with cardinality
/// impl CostFunction<QueryLang, SimpleProperty, DbCost<SimpleProperty>> for MyOptimizer {
///     fn compute_cost<CF>(&self, node: &QueryLang, mut costs: CF)
///         -> DbCost<SimpleProperty>
///     where
///         CF: FnMut(Id) -> DbCost<SimpleProperty>,
///     {
///         match node {
///             QueryLang::TableScan(table_id) => {
///                 let rows = self.user_data.catalog.get_rows(*table_id);
///                 DbCost::new(rows * 1000, Some(rows), None, SimpleProperty::Unsorted)
///             }
///             // ... other operators
///         }
///     }
/// }
/// ```
pub trait CostFunction<L: Language, P: Property, C: CostDomain<P>> {
    /// Compute the cost of an expression node.
    ///
    /// # Arguments
    ///
    /// * `node` - The expression node to cost
    /// * `costs` - A closure that returns the cost of a child by its Id
    ///
    /// # Returns
    ///
    /// A cost domain instance (e.g., `SimpleCost<P>` or custom domain)
    /// containing the cost, properties, and any other domain-specific information.
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
    fn compute_cost<CF>(&self, node: &L, costs: CF) -> C
    where
        CF: FnMut(Id) -> C;
}

/// A simple cost function implementation that uses the `SimpleCost` domain.
/// This is provided as a ready-to-use cost function for simple use cases.
impl<L, P> CostFunction<L, P, SimpleCost<P>> for OptimizerFramework<L, P, SimpleCost<P>, ()>
where
    L: PropertyAwareLanguage<P>,
    P: Property,
{
    fn compute_cost<CF>(&self, node: &L, mut costs: CF) -> SimpleCost<P>
    where
        CF: FnMut(Id) -> SimpleCost<P>,
    {
        // Example implementation - you would replace this with your actual cost logic
        let child_costs: Vec<SimpleCost<P>> = node
            .children()
            .iter()
            .map(|&child_id| costs(child_id))
            .collect();

        // Combine child costs and add operator cost (this is just a placeholder)
        let total_cost = child_costs
            .iter()
            .fold(0, |acc: usize, c: &SimpleCost<P>| {
                acc.saturating_add(c.cost())
            })
            .saturating_add(1);

        // Determine properties (this is just a placeholder - you would compute this based on the operator and child properties)
        let properties = P::bottom();

        SimpleCost::new(total_cost, properties)
    }
}
