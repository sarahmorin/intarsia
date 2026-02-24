/// Language extension traits for property-aware optimization.
///
/// This module provides traits that extend egg's `Language` trait with property-related
/// functionality needed by the cascades optimizer framework.
use egg::Language;

use crate::framework::property::Property;

/// Extension trait for languages that work with the property-aware optimizer.
///
/// This trait extends egg's `Language` trait by adding the ability to specify
/// what properties each operator requires from its children. This enables the
/// optimizer to propagate property requirements down the expression tree and
/// ensure that each operator receives inputs with the properties it needs.
///
/// # Type Parameters
///
/// * `P` - The property type used to describe requirements
///
/// # Examples
///
/// For a database query language, you might implement property requirements like this:
///
/// ```rust,ignore
/// impl PropertyAwareLanguage<SimpleProperty> for QueryLang {
///     fn property_req(&self, child_index: usize) -> SimpleProperty {
///         match self {
///             // Merge join requires both inputs to be sorted
///             QueryLang::MergeJoin(_) if child_index <= 1 => SimpleProperty::Sorted,
///             
///             // Hash join doesn't require sorted inputs
///             QueryLang::HashJoin(_) => SimpleProperty::Bottom,
///             
///             // Most operators have no special requirements
///             _ => SimpleProperty::Bottom,
///         }
///     }
/// }
/// ```
///
/// # Implementation Notes
///
/// - Return `P::bottom()` for children with no special requirements
/// - Consider the semantics of your operators when specifying requirements
/// - Requirements should be minimal (don't over-specify)
/// - Child index is 0-based (first child is index 0)
pub trait PropertyAwareLanguage<P: Property>: Language {
    /// Determine what properties this operator requires from a specific child.
    ///
    /// # Arguments
    ///
    /// * `child_index` - Zero-based index of the child (0 = first child, 1 = second child, etc.)
    ///
    /// # Returns
    ///
    /// The property that this operator requires from the child at `child_index`.
    /// Returns `P::bottom()` if there are no specific requirements.
    ///
    /// # Behavior for Invalid Indices
    ///
    /// If `child_index >= self.children().len()`, implementations should return `P::bottom()`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Sort operator requires no special properties from its input
    /// let sort_node = QueryLang::Sort([input_id, column_id]);
    /// assert_eq!(sort_node.property_req(0), SimpleProperty::Bottom);
    ///
    /// // Merge join requires both inputs to be sorted
    /// let merge_node = QueryLang::MergeJoin([left_id, right_id, pred_id]);
    /// assert_eq!(merge_node.property_req(0), SimpleProperty::Sorted);
    /// assert_eq!(merge_node.property_req(1), SimpleProperty::Sorted);
    /// assert_eq!(merge_node.property_req(2), SimpleProperty::Bottom); // predicate
    /// ```
    fn property_req(&self, child_index: usize) -> P;
}
