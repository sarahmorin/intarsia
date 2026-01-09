use crate::impl_oplang_default;
use crate::parser::Parseable;
use crate::types::*;
// use crate::property::{PropertySet};
// use bitmaps::Bitmap;
/// A stand in language for testing purposes.
use std::fmt::{Debug, Display};
use std::hash::Hash;

pub type TableName = String;
pub type ColumnName = String;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum QueryOps {
    // ============================
    // Basic Relational Algebra Operations
    Select,  // Select(input, predicate)
    Join,    // Join(left, right, predicate)
    Project, // Project(input, columns)
    // ============================
    // Basic Access Methods
    TableScan,  // TableScan(table, columns)
    IndexScan,  // IndexScan(table, index)
    HashLookup, // HashLookup(table, key)
    // ============================
    // Tables, Columns, and Values
    Table(TableName),
    // Column(TableName, ColumnName),
    ColumnSet(TableName, Vec<ColumnName>),
    ConstStr(String),
    ConstInt(i64),
    ConstBool(bool),
    Null,
    // ============================
    // Predicate Operators
    Eq,  // Eq(left, right)
    Neq, // Neq(left, right)
    Gt,  // Gt(left, right)
    Lt,  // Lt(left, right)
    Gte, // Gte(left, right)
    Lte, // Lte(left, right)
    And, // And(left, right)
    Or,  // Or(left, right)
    Not, // Not(expr)
    // ============================
    // Arithmetic Operators
    Add, // Add(left, right)
    Sub, // Sub(left, right)
    Mul, // Mul(left, right)
    Div, // Div(left, right)
    // ============================
    // Physical Operators
    Sort,      // Sort(input, order)
    Filter,    // Filter(input, predicate)
    NLJoin,    // NLJoin(left, right, predicate)
    HashJoin,  // HashJoin(left, right, predicate)
    MergeJoin, // MergeJoin(left, right, predicate)
}

impl Parseable for QueryOps {
    /// Parse a string into an Ops enum variant.
    fn parse(s: &str) -> Result<Self, String> {
        let trimmed = s.trim();

        match trimmed {
            "Null" => Ok(QueryOps::Null),
            // Basic Relational Algebra Operations
            "Select" => Ok(QueryOps::Select),
            "Join" => Ok(QueryOps::Join),
            "Project" => Ok(QueryOps::Project),
            // Basic Access Methods
            "TableScan" => Ok(QueryOps::TableScan),
            "IndexScan" => Ok(QueryOps::IndexScan),
            "HashLookup" => Ok(QueryOps::HashLookup),
            // Predicate Operators
            "Eq" => Ok(QueryOps::Eq),
            "Neq" => Ok(QueryOps::Neq),
            "Gt" => Ok(QueryOps::Gt),
            "Lt" => Ok(QueryOps::Lt),
            "Gte" => Ok(QueryOps::Gte),
            "Lte" => Ok(QueryOps::Lte),
            "And" => Ok(QueryOps::And),
            "Or" => Ok(QueryOps::Or),
            "Not" => Ok(QueryOps::Not),
            // Arithmetic Operators
            "Add" => Ok(QueryOps::Add),
            "Sub" => Ok(QueryOps::Sub),
            "Mul" => Ok(QueryOps::Mul),
            "Div" => Ok(QueryOps::Div),
            // Physical Operators
            "Sort" => Ok(QueryOps::Sort),
            "Filter" => Ok(QueryOps::Filter),
            "NLJoin" => Ok(QueryOps::NLJoin),
            "HashJoin" => Ok(QueryOps::HashJoin),
            "MergeJoin" => Ok(QueryOps::MergeJoin),
            // Boolean constants
            "true" => Ok(QueryOps::ConstBool(true)),
            "false" => Ok(QueryOps::ConstBool(false)),
            _ => {
                // Try parsing as Col[A][x,y,z] format for ColumnSet
                if trimmed.starts_with("Col[") {
                    if let Some(first_close) = trimmed.find(']') {
                        let table_name = &trimmed[4..first_close];
                        let remaining = &trimmed[first_close + 1..];

                        if remaining.starts_with('[') && remaining.ends_with(']') {
                            let columns_str = &remaining[1..remaining.len() - 1];
                            let columns: Vec<String> = columns_str
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                            return Ok(QueryOps::ColumnSet(table_name.to_string(), columns));
                        }
                    }
                }

                // Try parsing as Table[x] format
                if trimmed.starts_with("Table[") && trimmed.ends_with(']') {
                    let inner = &trimmed[6..trimmed.len() - 1]; // Extract content between Table[ and ]
                    return Ok(QueryOps::Table(inner.to_string()));
                }

                // Try parsing as integer
                if let Ok(i) = trimmed.parse::<i64>() {
                    return Ok(QueryOps::ConstInt(i));
                }

                // Try parsing as quoted string
                if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
                    let inner = &trimmed[1..trimmed.len() - 1];
                    return Ok(QueryOps::ConstStr(inner.to_string()));
                }

                // Default to unquoted string constant
                Ok(QueryOps::ConstStr(trimmed.to_string()))
            }
        }
    }
}

impl Display for QueryOps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryOps::ConstStr(s) => write!(f, "ConstStr({})", s),
            QueryOps::ConstInt(i) => write!(f, "ConstInt({})", i),
            QueryOps::ConstBool(b) => write!(f, "ConstBool({})", b),
            QueryOps::Null => write!(f, "Null"),
            QueryOps::Table(s) => write!(f, "Table({})", s),
            QueryOps::ColumnSet(table, cols) => write!(f, "ColumnSet({}, {:?})", table, cols),
            // Basic Relational Algebra Operations
            QueryOps::Select => write!(f, "Select"),
            QueryOps::Join => write!(f, "Join"),
            QueryOps::Project => write!(f, "Project"),
            // Basic Access Methods
            QueryOps::TableScan => write!(f, "TableScan"),
            QueryOps::IndexScan => write!(f, "IndexScan"),
            QueryOps::HashLookup => write!(f, "HashLookup"),
            // Predicate Operators
            QueryOps::Eq => write!(f, "=="),
            QueryOps::Neq => write!(f, "!="),
            QueryOps::Lt => write!(f, "<"),
            QueryOps::Gt => write!(f, ">"),
            QueryOps::Lte => write!(f, "<="),
            QueryOps::Gte => write!(f, ">="),
            QueryOps::And => write!(f, "And"),
            QueryOps::Or => write!(f, "Or"),
            QueryOps::Not => write!(f, "Not"),
            // Arithmetic Operators
            QueryOps::Add => write!(f, "Add"),
            QueryOps::Sub => write!(f, "Sub"),
            QueryOps::Mul => write!(f, "Mul"),
            QueryOps::Div => write!(f, "Div"),
            // Physical Operators
            QueryOps::Sort => write!(f, "Sort"),
            QueryOps::Filter => write!(f, "Filter"),
            QueryOps::NLJoin => write!(f, "NLJoin"),
            QueryOps::HashJoin => write!(f, "HashJoin"),
            QueryOps::MergeJoin => write!(f, "MergeJoin"),
        }
    }
}

impl OpLang for QueryOps {
    impl_oplang_default!();

    fn arity(&self) -> usize {
        match self {
            // Constants have arity 0
            QueryOps::ConstStr(_)
            | QueryOps::ConstInt(_)
            | QueryOps::ConstBool(_)
            | QueryOps::Null
            | QueryOps::Table(_)
            | QueryOps::ColumnSet(_, _) => 0,
            // Unary operators
            QueryOps::Not => 1,
            // Binary operators
            QueryOps::Eq
            | QueryOps::Neq
            | QueryOps::Lt
            | QueryOps::Gt
            | QueryOps::Lte
            | QueryOps::Gte
            | QueryOps::And
            | QueryOps::Or
            | QueryOps::Add
            | QueryOps::Sub
            | QueryOps::Mul
            | QueryOps::Div => 2,
            // Query operators with 2 arguments
            QueryOps::Select => 2,     // Select(input, predicate)
            QueryOps::Project => 2,    // Project(input, columns)
            QueryOps::Sort => 2,       // Sort(input, order)
            QueryOps::Filter => 2,     // Filter(input, predicate)
            QueryOps::IndexScan => 2,  // IndexScan(table, index)
            QueryOps::TableScan => 2,  // TableScan(table, predicate)
            QueryOps::HashLookup => 2, // HashLookup(table, key)
            // Query operators with 3 arguments
            QueryOps::Join => 3,      // Join(left, right, predicate)
            QueryOps::NLJoin => 3,    // NLJoin(left, right, predicate)
            QueryOps::HashJoin => 3,  // HashJoin(left, right, predicate)
            QueryOps::MergeJoin => 3, // MergeJoin(left, right, predicate)
        }
    }
}

// /// Represents a physical property set for query optimization.
// /// This is a simple example with a single property: sorted stored in a bitmap.
// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
// pub struct PhysicalPropertySet {
//     sort: Bitmap<3>,
// }

// impl PhysicalPropertySet {
//     /// Creates a physical property set from a sort index.
//     // FIXME: this is dummy test implementation
//     // In a real implementation, we would call out to a larger data struct to get the real column sort bitmaps
//     pub fn from_cols(cols: String) -> Self {
//         let mut sort = Bitmap::new();
//         match cols.as_str() {
//             "x" => {
//                 sort.set(0, true); // Assume column "x" is sorted
//             }
//             "y" => {
//                 sort.set(1, true); // Assume column "y" is sorted
//             }
//             "z" => {
//                 sort.set(2, true); // Assume column "z" is sorted
//             }
//             "xy" => {
//                 sort.set(0, true); // Assume both "x" and "y" are sorted
//                 sort.set(1, true);
//             }
//             "yz" => {
//                 sort.set(1, true); // Assume both "y" and "z" are sorted
//                 sort.set(2, true);
//             }
//             "xz" => {
//                 sort.set(0, true); // Assume both "x" and "z" are sorted
//                 sort.set(2, true);
//             }
//             "xyz" => {
//                 sort.set(0, true); // Assume all "x", "y", and "z" are sorted
//                 sort.set(1, true);
//                 sort.set(2, true);
//             }
//             _ => {}
//         }
//         PhysicalPropertySet { sort }
//     }
// }

// impl Display for PhysicalPropertySet {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "PhysicalPropertySet(sort: {:?})", self.sort)
//     }
// }

// impl PropertySet for PhysicalPropertySet {
//     fn bottom() -> Self {
//         Self {
//             sort: Bitmap::new(),
//         }
//     }

//     fn meet(&self, other: &Self) -> Self {
//         let mut new_sort = Bitmap::new();
//         for i in 0..3 {
//             new_sort.set(i, self.sort.get(i) && other.sort.get(i));
//         }
//         Self { sort: new_sort }
//     }

//     fn join(&self, other: &Self) -> Option<Self> {
//         None // Join is not defined for this simple property set
//     }
// }

// impl PartialOrd for PhysicalPropertySet {
//     fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
//         self.sort.partial_cmp(&other.sort)
//     }
// }

// impl From<QueryOps> for PhysicalPropertySet {
//     fn from(op: QueryOps) -> Self {
//         match op {
//             QueryOps::ColumnSet(_, cols) => {
//                 // For now, just use the first column for simplicity
//                 if let Some(first_col) = cols.first() {
//                     PhysicalPropertySet::from_cols(first_col.clone())
//                 } else {
//                     PhysicalPropertySet::bottom()
//                 }
//             }
//             _ => PhysicalPropertySet::bottom(),
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    #[test]
    fn test_parse_boolean_constants() {
        let result = QueryOps::parse("true");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), QueryOps::ConstBool(true));

        let result = QueryOps::parse("false");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), QueryOps::ConstBool(false));
    }

    #[test]
    fn test_parse_integer_constants() {
        let result = QueryOps::parse("100");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), QueryOps::ConstInt(100));

        let result = QueryOps::parse("-2");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), QueryOps::ConstInt(-2));

        let result = QueryOps::parse("0");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), QueryOps::ConstInt(0));
    }

    #[test]
    fn test_parse_col_and_table() {
        // Test new ColumnSet syntax: Col[table][col1,col2,col3]
        let result = QueryOps::parse("Col[users][id,name,email]");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            QueryOps::ColumnSet(
                "users".to_string(),
                vec!["id".to_string(), "name".to_string(), "email".to_string()]
            )
        );

        // Test Table syntax
        let result = QueryOps::parse("Table[users]");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), QueryOps::Table("users".to_string()));

        // Test single column ColumnSet
        let result = QueryOps::parse("Col[products][name]");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            QueryOps::ColumnSet("products".to_string(), vec!["name".to_string()])
        );

        // Test empty column set
        let result = QueryOps::parse("Col[table][]");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            QueryOps::ColumnSet("table".to_string(), vec![])
        );

        // Test columns with whitespace
        let result = QueryOps::parse("Col[users][ id , name , email ]");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            QueryOps::ColumnSet(
                "users".to_string(),
                vec!["id".to_string(), "name".to_string(), "email".to_string()]
            )
        );
    }

    #[test]
    fn test_parseerators() {
        let result = QueryOps::parse("And");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), QueryOps::And);

        let result = QueryOps::parse("Or");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), QueryOps::Or);

        let result = QueryOps::parse("Not");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), QueryOps::Not);

        let result = QueryOps::parse("Eq");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), QueryOps::Eq);
    }

    #[test]
    fn test_parse_string_constants() {
        let result = QueryOps::parse("hello");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), QueryOps::ConstStr("hello".to_string()));

        let result = QueryOps::parse("some_string");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            QueryOps::ConstStr("some_string".to_string())
        );
    }

    #[test]
    fn test_parse_variable_error() {
        // Variables starting with '?' should be parsed as string constants now
        // since the FromStr implementation doesn't reject them
        let result = QueryOps::parse("?x");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), QueryOps::ConstStr("?x".to_string()));
    }

    #[test]
    fn test_parse_whitespace_handling() {
        let result = QueryOps::parse("  true  ");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), QueryOps::ConstBool(true));

        let result = QueryOps::parse("  100  ");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), QueryOps::ConstInt(100));

        let result = QueryOps::parse("  And  ");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), QueryOps::And);
    }

    #[test]
    fn test_parse_expr_with_ops_parser() {
        // Test parsing a simple expression with the new Parser struct
        let result = Parser::<QueryOps>::parse_expr("And(true, false)");
        assert!(result.is_ok());
        let expr = result.unwrap();
        assert_eq!(*expr.op(), QueryOps::And);
        assert_eq!(expr.args().len(), 2);
        assert_eq!(*expr.args()[0].op(), QueryOps::ConstBool(true));
        assert_eq!(*expr.args()[1].op(), QueryOps::ConstBool(false));
    }

    #[test]
    fn test_parse_expr_with_constants() {
        // Test parsing with integer constants
        let result = Parser::<QueryOps>::parse_expr("Eq(100, -50)");
        assert!(result.is_ok());
        let expr = result.unwrap();
        assert_eq!(*expr.op(), QueryOps::Eq);
        assert_eq!(expr.args().len(), 2);
        assert_eq!(*expr.args()[0].op(), QueryOps::ConstInt(100));
        assert_eq!(*expr.args()[1].op(), QueryOps::ConstInt(-50));
    }

    #[test]
    fn test_parse_expr_with_col_table() {
        let result = Parser::<QueryOps>::parse_expr("TableScan(Table[users], Null)");
        assert!(result.is_ok());
        let expr = result.unwrap();
        assert_eq!(*expr.op(), QueryOps::TableScan);
        assert_eq!(expr.args().len(), 2);
        assert_eq!(*expr.args()[0].op(), QueryOps::Table("users".to_string()));
        assert_eq!(*expr.args()[1].op(), QueryOps::Null);
    }

    #[test]
    fn test_parse_complex_expressions_with_square_brackets() {
        // Test a more complex expression with nested ColumnSet and Table
        let result = Parser::<QueryOps>::parse_expr(
            "Select(TableScan(Table[employees], Null), Eq(Col[employees][department], engineering))",
        );
        assert!(result.is_ok());
        let expr = result.unwrap();
        assert_eq!(*expr.op(), QueryOps::Select);
        assert_eq!(expr.args().len(), 2);

        // First argument: TableScan(Table[employees], Null)
        let scan_expr = &expr.args()[0];
        assert_eq!(*scan_expr.op(), QueryOps::TableScan);
        assert_eq!(scan_expr.args().len(), 2);
        assert_eq!(
            *scan_expr.args()[0].op(),
            QueryOps::Table("employees".to_string())
        );
        assert_eq!(*scan_expr.args()[1].op(), QueryOps::Null);

        // Second argument: Eq(Col[employees][department], engineering)
        let eq_expr = &expr.args()[1];
        assert_eq!(*eq_expr.op(), QueryOps::Eq);
        assert_eq!(eq_expr.args().len(), 2);
        assert_eq!(
            *eq_expr.args()[0].op(),
            QueryOps::ColumnSet("employees".to_string(), vec!["department".to_string()])
        );
        assert_eq!(
            *eq_expr.args()[1].op(),
            QueryOps::ConstStr("engineering".to_string())
        );
    }

    #[test]
    fn test_pattern_parsing_with_square_brackets() {
        // Test pattern parsing with variables in a simpler form
        let result = Parser::<QueryOps>::parse_pattern("TableScan(?table_expr, ?predicate)");
        assert!(result.is_ok());
        let pattern = result.unwrap();

        // Check top-level operator
        match pattern.op() {
            OpOrVar::Op(op) => assert_eq!(*op, QueryOps::TableScan),
            OpOrVar::Var(_) => panic!("Expected operator, got variable"),
        }

        assert_eq!(pattern.args().len(), 2);

        // First argument: ?table_expr - should be parsed as a variable
        match pattern.args()[0].op() {
            OpOrVar::Var(var) => assert_eq!(var, "table_expr"),
            OpOrVar::Op(_) => panic!("Expected variable, got operator"),
        }

        // Second argument: ?predicate - should be parsed as a variable
        match pattern.args()[1].op() {
            OpOrVar::Var(var) => assert_eq!(var, "predicate"),
            OpOrVar::Op(_) => panic!("Expected variable, got operator"),
        }
    }

    #[test]
    fn test_pattern_parsing_with_concrete_square_brackets() {
        // Test pattern parsing with concrete Table[] mixed with variables
        let result = Parser::<QueryOps>::parse_pattern("TableScan(Table[users], ?predicate)");
        assert!(result.is_ok());
        let pattern = result.unwrap();

        // Check top-level operator
        match pattern.op() {
            OpOrVar::Op(op) => assert_eq!(*op, QueryOps::TableScan),
            OpOrVar::Var(_) => panic!("Expected operator, got variable"),
        }

        assert_eq!(pattern.args().len(), 2);

        // First argument: Table[users] - should be parsed as a concrete Table operator
        match pattern.args()[0].op() {
            OpOrVar::Op(op) => assert_eq!(*op, QueryOps::Table("users".to_string())),
            OpOrVar::Var(_) => panic!("Expected operator, got variable"),
        }

        // Second argument: ?predicate - should be parsed as a variable
        match pattern.args()[1].op() {
            OpOrVar::Var(var) => assert_eq!(var, "predicate"),
            OpOrVar::Op(_) => panic!("Expected variable, got operator"),
        }
    }
}
