use crate::types::{ColSetId, IndexId, TableId};
use egg::{Id, define_language};

// Operator Language for optimization
define_language! {
    pub enum Optlang {
        // Constant Values
        Int(i64),
        Bool(bool),
        Str(String),
        // Arithmetic Operations
        "+" = Add([Id; 2]),
        "-" = Sub([Id; 2]),
        "*" = Mul([Id; 2]),
        "/" = Div([Id; 2]),
        // Comparison Operations
        "==" = Eq([Id; 2]),
        "<" = Lt([Id; 2]),
        ">" = Gt([Id; 2]),
        "<=" = Le([Id; 2]),
        ">=" = Ge([Id; 2]),
        "!=" = Ne([Id; 2]),
        // Logical Operations
        "AND" = And([Id; 2]),
        "OR" = Or([Id; 2]),
        "NOT" = Not(Id),
        // Relational Operations
        "SCAN" = Scan(Id),
        "SELECT" = Select([Id; 2]),
        "PROJECT" = Project([Id; 2]),
        "JOIN" = Join([Id; 3]),
        "TABLE_SCAN" = TableScan(Id),
        "INDEX_SCAN" = IndexScan(Id),
        "NESTED_LOOP_JOIN" = NestedLoopJoin([Id; 3]),
        "HASH_JOIN" = HashJoin([Id; 3]),
        "MERGE_JOIN" = MergeJoin([Id; 3]),
        "SORT" = Sort([Id; 2]),
        // Data Sources
        Table(TableId),
        ColSet(ColSetId),
        Index(IndexId),
    }
}

impl Optlang {
    pub fn property_req(&self, child_index: usize) -> SimpleProperty {
        match self {
            // MergeJoin requires its sources to be sorted
            Optlang::MergeJoin(_) => {
                if child_index == 0 || child_index == 1 {
                    SimpleProperty::Sorted
                } else {
                    SimpleProperty::Bottom
                }
            }
            // TODO: Add more property requirements
            // Most operators don't require any specific properties from their children, so we return Bottom to indicate that there are no requirements
            _ => SimpleProperty::Bottom,
        }
    }
}

/// A very simple example of properties we might want to track in our optimizer context.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy)]
pub enum SimpleProperty {
    Sorted,
    Unsorted,
    Bottom,
}

/// Check if a provided property satisfies a required property.
/// Returns true if an expression providing `provided` can be used where `required` is needed.
pub fn satisfies_property(provided: &SimpleProperty, required: &SimpleProperty) -> bool {
    match required {
        // No requirement - any property satisfies
        SimpleProperty::Bottom => true,
        SimpleProperty::Unsorted => {
            provided == &SimpleProperty::Unsorted || provided == &SimpleProperty::Sorted
        }
        SimpleProperty::Sorted => provided == &SimpleProperty::Sorted,
    }
}
