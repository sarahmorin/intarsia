/// Query language definition for the database optimizer example.
///
/// Property requirements and cost are now expressed via [`super::cost::DbTransfer`]
/// (a `PropertyTransfer` impl), so this file is just the `define_language!` block.
use egg::{Id, define_language};

use super::types::{ColSetId, IndexId, TableId};

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
