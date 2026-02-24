/// The Property trait defines a property system for the optimizer framework.
///
/// Properties represent semantic attributes that expressions can have or require,
/// such as "sorted", "distributed", "partitioned", etc. The optimizer uses properties
/// to ensure that physical operators receive inputs with the properties they require.
///
/// # Examples
///
/// A simple property system might track whether data is sorted:
///
/// ```rust,ignore
/// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// enum SimpleProperty {
///     Sorted,
///     Unsorted,
///     Bottom, // No requirement
/// }
///
/// impl Property for SimpleProperty {
///     fn satisfies(&self, required: &Self) -> bool {
///         match required {
///             SimpleProperty::Bottom => true, // No requirement is always satisfied
///             SimpleProperty::Unsorted => {
///                 matches!(self, SimpleProperty::Unsorted | SimpleProperty::Sorted)
///             }
///             SimpleProperty::Sorted => matches!(self, SimpleProperty::Sorted),
///         }
///     }
///
///     fn bottom() -> Self {
///         SimpleProperty::Bottom
///     }
/// }
/// ```
use std::fmt::Debug;
use std::hash::Hash;

pub trait Property: Clone + Eq + Hash + Debug {
    /// Check if a provided property satisfies a required property.
    ///
    /// Returns `true` if an expression providing `self` can be used where `required` is needed.
    ///
    /// # Arguments
    ///
    /// * `required` - The property that is required by a consumer
    ///
    /// # Returns
    ///
    /// `true` if `self` satisfies `required`, `false` otherwise
    ///
    /// # Semantics
    ///
    /// This method defines a partial order on properties where:
    /// - More specific properties satisfy less specific requirements
    /// - Bottom (no requirement) is satisfied by any property
    /// - The method should be transitive: if A satisfies B and B satisfies C, then A satisfies C
    fn satisfies(&self, required: &Self) -> bool;

    /// Return the "bottom" element representing no requirements.
    ///
    /// This is the least restrictive property that any expression can satisfy.
    /// Used as the default when no specific properties are required.
    fn bottom() -> Self;
}
