/// Property system for the database optimizer example.
///
/// This demonstrates a simple property system that tracks whether data is sorted.
use intarsia::framework::Property;

/// A simple property system tracking sortedness of data.
///
/// This is used by physical operators to specify their requirements and capabilities:
/// - Merge joins require sorted inputs
/// - Index scans provide sorted data
/// - Hash joins work with any sortedness
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy)]
pub enum SimpleProperty {
    /// Data is sorted (e.g., from an index scan or sort operator)
    Sorted,
    /// Data is explicitly unsorted
    Unsorted,
    /// No requirement or bottom element - any property satisfies this
    Bottom,
}

impl Property for SimpleProperty {
    /// Check if a provided property satisfies a required property.
    ///
    /// The property lattice is: Bottom < Unsorted < Sorted
    /// Where "less than" means "more general" (satisfies fewer requirements).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Sorted data can be used where sorted is required
    /// assert!(SimpleProperty::Sorted.satisfies(&SimpleProperty::Sorted));
    ///
    /// // Sorted data can be used where unsorted is acceptable
    /// assert!(SimpleProperty::Sorted.satisfies(&SimpleProperty::Unsorted));
    ///
    /// // Unsorted data cannot be used where sorted is required
    /// assert!(!SimpleProperty::Unsorted.satisfies(&SimpleProperty::Sorted));
    ///
    /// // Any property satisfies Bottom (no requirements)
    /// assert!(SimpleProperty::Sorted.satisfies(&SimpleProperty::Bottom));
    /// assert!(SimpleProperty::Unsorted.satisfies(&SimpleProperty::Bottom));
    /// ```
    fn satisfies(&self, required: &Self) -> bool {
        match required {
            // No requirement - any property satisfies
            SimpleProperty::Bottom => true,
            // Unsorted requirement - both sorted and unsorted satisfy
            // (sorted data can be used where only unsorted is required)
            SimpleProperty::Unsorted => {
                matches!(self, SimpleProperty::Unsorted | SimpleProperty::Sorted)
            }
            // Sorted requirement - only sorted data satisfies
            SimpleProperty::Sorted => matches!(self, SimpleProperty::Sorted),
        }
    }

    /// Return the bottom element (no requirements).
    fn bottom() -> Self {
        SimpleProperty::Bottom
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sorted_satisfies_sorted() {
        assert!(SimpleProperty::Sorted.satisfies(&SimpleProperty::Sorted));
    }

    #[test]
    fn test_sorted_satisfies_unsorted() {
        assert!(SimpleProperty::Sorted.satisfies(&SimpleProperty::Unsorted));
    }

    #[test]
    fn test_unsorted_does_not_satisfy_sorted() {
        assert!(!SimpleProperty::Unsorted.satisfies(&SimpleProperty::Sorted));
    }

    #[test]
    fn test_unsorted_satisfies_unsorted() {
        assert!(SimpleProperty::Unsorted.satisfies(&SimpleProperty::Unsorted));
    }

    #[test]
    fn test_any_satisfies_bottom() {
        assert!(SimpleProperty::Sorted.satisfies(&SimpleProperty::Bottom));
        assert!(SimpleProperty::Unsorted.satisfies(&SimpleProperty::Bottom));
        assert!(SimpleProperty::Bottom.satisfies(&SimpleProperty::Bottom));
    }

    #[test]
    fn test_bottom_does_not_satisfy_sorted() {
        assert!(!SimpleProperty::Bottom.satisfies(&SimpleProperty::Sorted));
    }

    #[test]
    fn test_bottom_does_not_satisfy_unsorted() {
        assert!(!SimpleProperty::Bottom.satisfies(&SimpleProperty::Unsorted));
    }
}
