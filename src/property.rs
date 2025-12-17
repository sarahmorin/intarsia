/// Property traits for properties of expressions and terms.

use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::collections::HashSet;
use log::warn;

/// Property trait for properties of operators and terms.
/// A PropertySet defines the properties that can be associated with an expression in the language.
/// These properties can be used to enforce constraints on the expressions, such as requiring certain types of input.
/// A property set could be a simple bool flag, a bitmap, or a more complex structure.
/// It need only implement the `PartialOrd` trait and provide a bottom element.
/// NOTE: Implement `PartialOrd` for your property set. Deriving the trait is likely not the behavior you need here.
pub trait PropertySet: Clone + Debug + PartialEq + Eq + PartialOrd + Display + Hash {
    /// Returns the bottom element of the property set.
    /// This represents the absence of any properties.
    fn bottom() -> Self;
    /// Returns the meet (greatest lower bound) of two property sets.
    fn meet(&self, other: &Self) -> Self;
    /// Returns the join (least upper bound) of two property sets if one exists.
    fn join(&self, other: &Self) -> Option<Self>;
    /// Returns a vector of `n` bottom elements of the property set.
    fn n_bottoms(n: usize) -> Vec<Self>
    where
        Self: Sized,
    {
        vec![Self::bottom(); n]
    }
    /// Returns true if the property set is the bottom element.
    fn is_bot(&self) -> bool {
        self == &Self::bottom()
    }
}

/// Enum to represent the comparison result between two PropertySet ranges.
/// Excludes: The two ranges do not overlap.
/// Overlaps: The two ranges overlap but self does not contain other.
/// Contains: The self range fully contains the other range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PSComparison {
        Excludes,
        Overlaps,
        Contains,
}

/// Sets of PropertySet values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PSSet<P>
where 
    P: PropertySet,
{
    pub values: HashSet<P>,
}

impl <P> PSSet<P>
where 
    P: PropertySet,
{
    /// Creates a new, empty PSSet.
    pub fn new() -> Self {
        Self {
            values: HashSet::new(),
        }
    }

    /// Creates a new PSSet from a single element.
    pub fn from_single(element: P) -> Self {
        let mut set = Self::new();
        set.values.insert(element);
        set
    }

    /// Combines two PSSet into their union.
    pub fn union(&self, other: &PSSet<P>) -> PSSet<P> {
        PSSet { values: self.values.union(&other.values).cloned().collect() }
    }

    /// Compares two PSSet and returns their relationship as `enum PSComparison`.
    /// Note: This operation is not commutative self.compare(other) != other.compare(self) in general.
    pub fn compare(&self, other: &PSSet<P>) -> PSComparison {
        if self.values.is_superset(&other.values) {
            PSComparison::Contains
        } else if self.values.is_disjoint(&other.values) {
            PSComparison::Excludes
        } else {
            PSComparison::Overlaps
        }
    }
}

/// Range of PropertySet values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PSRange<P>
where 
    P: PropertySet,
{
    pub start: P,
    pub end: P,   
}

impl <P> PSRange<P>
where 
    P: PropertySet,
{
    /// Creates a new PSRange with the given start and end values.
    /// If start > end, the values are swapped and a warning is logged.
    pub fn new(start: P, end: P) -> Self {
        if start > end {
            warn!("PSRange created with start > end: start: {}, end: {}. Overriding to correct order.", start, end);
            return Self { start: end, end: start };
        }
        Self { start, end }
    }

    /// Creates a PSRange that represents a single value.
    pub fn from_single(value: P) -> Self {
        Self {
            start: value.clone(),
            end: value,
        }
    }

    /// Compare two ranges and return their relationship as `enum PSComparison`.
    /// Note: This operation is not commutative self.compare(other) != other.compare(self) in general.
    pub fn compare(&self, other: &PSRange<P>) -> PSComparison {
        if &self.end < &other.start || &other.end < &self.start {
            PSComparison::Excludes
        } else if &self.start <= &other.start && &self.end >= &other.end {
            PSComparison::Contains
        } else {
            // FIXME: This isn't quite right for incomparable property sets.
            PSComparison::Overlaps
        }
    }

    /// Combine two PSRanges into their union.
    pub fn union(&self, other: &PSRange<P>) -> PSRange<P> {
        // TODO: Consider adding a top, even though it potentially loses information
        Self {
            start: self.start.meet(&other.start),
            end: self.end.join(&other.end).unwrap_or_else(|| {
                panic!("Cannot compute union of PSRanges: {:?} and {:?}", self, other)
            })
        }
    }
}
