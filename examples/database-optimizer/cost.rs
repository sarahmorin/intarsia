/// Cost domain for database query optimization.
///
/// This cost domain extends the simple cost+properties model with database-specific
/// metrics like cardinality (row count estimates) and block counts.
use intarsia::CostDomain;

use super::property::SimpleProperty;

/// Database-specific cost domain.
///
/// Captures cost alongwith cardinality and block estimates for more accurate
/// cost modeling in database query optimization.
#[derive(Debug, Clone, Eq, Hash)]
pub struct DbCost {
    /// Raw cost value (lower is better)
    pub cost: usize,

    /// Estimated number of rows (cardinality)
    pub cardinality: Option<usize>,

    /// Estimated number of disk blocks
    pub blocks: Option<usize>,

    /// Physical properties (e.g., sortedness)
    pub properties: SimpleProperty,
}

impl DbCost {
    /// Create a new database cost.
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

    /// Create a simple cost with no cardinality/block estimates.
    pub fn simple(cost: usize) -> Self {
        Self {
            cost,
            cardinality: None,
            blocks: None,
            properties: SimpleProperty::Bottom,
        }
    }
}

impl CostDomain<SimpleProperty> for DbCost {
    type RawCost = usize;
    fn cost(&self) -> usize {
        self.cost
    }

    fn properties(&self) -> &SimpleProperty {
        &self.properties
    }
}

impl Default for DbCost {
    fn default() -> Self {
        Self {
            cost: usize::MAX,
            cardinality: None,
            blocks: None,
            properties: SimpleProperty::Bottom,
        }
    }
}

impl PartialEq for DbCost {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost
            && self.cardinality == other.cardinality
            && self.blocks == other.blocks
            && self.properties == other.properties
    }
}

impl PartialOrd for DbCost {
    /// Compare costs with a multi-level ordering:
    /// 1. Compare raw cost (primary)
    /// 2. If equal, compare cardinality (lower is better)
    /// 3. If equal, compare block counts (lower is better)
    /// 4. If equal, compare properties (Sorted < Unsorted < Bottom)
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DbCost {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.cost.cmp(&other.cost) {
            std::cmp::Ordering::Equal => match self.cardinality.cmp(&other.cardinality) {
                std::cmp::Ordering::Equal => match self.blocks.cmp(&other.blocks) {
                    std::cmp::Ordering::Equal => self.properties.cmp(&other.properties),
                    other => other,
                },
                other => other,
            },
            ord => ord,
        }
    }
}
