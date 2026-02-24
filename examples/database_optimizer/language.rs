/// Query language definition and property requirements.
///
/// This module defines the Optlang language for database query optimization
/// and implements property requirements for each operator.

use egg::{define_language, Id};
use kymetica::framework::PropertyAwareLanguage;

use super::property::SimpleProperty;
use super::types::{ColSetId, IndexId, TableId};

// Define the operator language for database query optimization
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
        
        // Logical Relational Operations
        "SCAN" = Scan(Id),
        "SELECT" = Select([Id; 2]),
        "PROJECT" = Project([Id; 2]),
        "JOIN" = Join([Id; 3]),
        
        // Physical Relational Operations
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

/// Implement property requirements for the query language.
///
/// This defines what properties each operator requires from its children.
impl PropertyAwareLanguage<SimpleProperty> for Optlang {
    fn property_req(&self, child_index: usize) -> SimpleProperty {
        match self {
            // MergeJoin requires both inputs to be sorted (children 0 and 1)
            // The predicate (child 2) has no specific requirements
            Optlang::MergeJoin(_) => {
                if child_index == 0 || child_index == 1 {
                    SimpleProperty::Sorted
                } else {
                    SimpleProperty::Bottom
                }
            }
            
            // Most operators don't require specific properties from their children
            // They work with any sortedness
            _ => SimpleProperty::Bottom,
        }
    }
}
