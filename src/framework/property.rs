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
///
/// Note: A well-formed property system is a lattice. Thus, the Property trait requires implementing `PartialOrd`.
/// It is recommended to implement`PartialOrd` manually to ensure the correct semantics of the property lattice.
///
/// The `satisfies` method should be consistent with the `PartialOrd` implementation, where `self.satisfies(required)`
/// should return `true` if and only if `self >= required` according to the lattice ordering.
/// The `bottom` method should return the least element of the lattice, which is satisfied by all other properties.
use std::fmt::Debug;
use std::hash::Hash;

pub trait Property: Clone + Eq + Hash + Debug + PartialOrd {
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

    // TODO: Consider adding a method to combine properties (e.g., meet/join) if needed for your lattice

    // TODO: Include the function to map Property to an Id lattice if we want to support that feature in the future
}

/// A simple implementation of None properties, where all expressions satisfy the same property.
/// This can be used when you don't need to track any specific properties in your optimizer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd)]
pub struct NoProperty;

impl Property for NoProperty {
    fn satisfies(&self, _required: &Self) -> bool {
        true // All properties are satisfied since there is only one
    }

    fn bottom() -> Self {
        NoProperty
    }
}
