use crate::impl_oplang_default;
use crate::parser::{Parseable, Parser};
use crate::types::*;
use bitmaps::Bitmap;
/// A stand in language for testing purposes.
use std::fmt::{Debug, Display};
use std::hash::Hash;

/// Represents the test language.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum QueryOps {
    // ============================
    // Constants Values
    ConstStr(String), // String value
    ConstInt(i32),    // Integer value
    ConstBool(bool),  // Boolean value
    Null,             // Null value
    // ============================
    // Column and Table Access
    Col(String),              // Col(col_name)    QUESTION: Do we need both types?
    Cols(Vec<String>),        // Col(col_names) -> unordered list of columns
    Order(Vec<String>),       // Order(col_names) -> ordered list of columns
    Table(String),            // Table(table_name)
    TableCol(String, String), // TableCol(table_name, col_name)
    Index,                    // Index(Table, Cols)
    Access,                   // Access(Table | Index, Column, Predicate | Null)
    // ============================
    // Predicate Operators
    Predicate, // TODO: Placeholder for predicate structure
    // All of these operators take arguments of the form TableCol or other predicates
    // All take two arguments, except Not which takes one
    Eq,  // Eq(x, y)
    Neq, // Neq(x, y)
    Lt,  // Lt(x, y)
    Gt,  // Gt(x, y)
    Le,  // Le(x, y)
    Ge,  // Ge(x, y)
    And, // And(x, y)
    Or,  // Or(x, y)
    Not, // Not(x)
    // TODO: Add arithmetic operators (Add, Sub, Mul, Div) if needed
    // ============================
    // Logical Query Ops
    Select,  // Select(input, Cols, Predicate)
    Join,    // Join(left, right, Predicate)
    Project, // Project(input, Cols, Cols | Null)
    // ============================
    // Physical Query Ops
    Scan,      // Scan(Table, Cols | Null, Predicate | Null)
    IndexScan, // IndexScan(Index, Predicate | Null)
    Sort,      // Sort(input, Order)
    NLJoin,    // NLJoin(left, right, Predicate)
    SortJoin,  // SortJoin(left, right, Predicate)
               // TODO: Physical projection operator?
               // ============================
}

impl Parseable for QueryOps {
    /// Parse a string into an Ops enum variant.
    fn parse(s: &str) -> Result<Self, String> {
        let trimmed = s.trim();

        match trimmed {
            "Null" => Ok(QueryOps::Null),
            "Index" => Ok(QueryOps::Index),
            "Access" => Ok(QueryOps::Access),
            "Predicate" => Ok(QueryOps::Predicate),
            "Eq" => Ok(QueryOps::Eq),
            "Neq" => Ok(QueryOps::Neq),
            "Lt" => Ok(QueryOps::Lt),
            "Gt" => Ok(QueryOps::Gt),
            "Le" => Ok(QueryOps::Le),
            "Ge" => Ok(QueryOps::Ge),
            "And" => Ok(QueryOps::And),
            "Or" => Ok(QueryOps::Or),
            "Not" => Ok(QueryOps::Not),
            "Select" => Ok(QueryOps::Select),
            "Join" => Ok(QueryOps::Join),
            "Project" => Ok(QueryOps::Project),
            "Scan" => Ok(QueryOps::Scan),
            "IndexScan" => Ok(QueryOps::IndexScan),
            "Sort" => Ok(QueryOps::Sort),
            "NLJoin" => Ok(QueryOps::NLJoin),
            "SortJoin" => Ok(QueryOps::SortJoin),
            // Boolean constants
            "true" => Ok(QueryOps::ConstBool(true)),
            "false" => Ok(QueryOps::ConstBool(false)),
            _ => {
                // Try parsing as Col[x] format
                if trimmed.starts_with("Col[") && trimmed.ends_with(']') {
                    let inner = &trimmed[4..trimmed.len() - 1]; // Extract content between Col[ and ]
                    return Ok(QueryOps::Col(inner.to_string()));
                }

                // Try parsing as Table[x] format
                if trimmed.starts_with("Table[") && trimmed.ends_with(']') {
                    let inner = &trimmed[6..trimmed.len() - 1]; // Extract content between Table[ and ]
                    return Ok(QueryOps::Table(inner.to_string()));
                }

                // Try parsing as integer
                if let Ok(i) = trimmed.parse::<i32>() {
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
            QueryOps::Col(s) => write!(f, "Col({})", s),
            QueryOps::Cols(cols) => write!(f, "Cols({:?})", cols),
            QueryOps::Order(cols) => write!(f, "Order({:?})", cols),
            QueryOps::Table(s) => write!(f, "Table({})", s),
            QueryOps::TableCol(table, col) => write!(f, "TableCol({}, {})", table, col),
            QueryOps::Index => write!(f, "Index"),
            QueryOps::Access => write!(f, "Access"),
            QueryOps::Predicate => write!(f, "Predicate"),
            QueryOps::Eq => write!(f, "=="),
            QueryOps::Neq => write!(f, "!="),
            QueryOps::Lt => write!(f, "<"),
            QueryOps::Gt => write!(f, ">"),
            QueryOps::Le => write!(f, "<="),
            QueryOps::Ge => write!(f, ">="),
            QueryOps::And => write!(f, "And"),
            QueryOps::Or => write!(f, "Or"),
            QueryOps::Not => write!(f, "Not"),
            QueryOps::Select => write!(f, "Select"),
            QueryOps::Join => write!(f, "Join"),
            QueryOps::Project => write!(f, "Project"),
            QueryOps::Scan => write!(f, "Scan"),
            QueryOps::IndexScan => write!(f, "IndexScan"),
            QueryOps::Sort => write!(f, "Sort"),
            QueryOps::NLJoin => write!(f, "NLJoin"),
            QueryOps::SortJoin => write!(f, "SortJoin"),
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
            | QueryOps::Col(_)
            | QueryOps::Cols(_)
            | QueryOps::Order(_)
            | QueryOps::Table(_)
            | QueryOps::Predicate => 0,
            // Unary operators
            QueryOps::Not => 1,
            // Binary operators
            QueryOps::Eq
            | QueryOps::Neq
            | QueryOps::Lt
            | QueryOps::Gt
            | QueryOps::Le
            | QueryOps::Ge => 2,
            QueryOps::And | QueryOps::Or => 2,
            QueryOps::TableCol(_, _) => 2,
            // Query operators
            QueryOps::Index => 2,     // Index(Table, Cols)
            QueryOps::Access => 3,    // Access(Table | Index, Column, Predicate | Null)
            QueryOps::Select => 3,    // Select(input, Cols, Predicate)
            QueryOps::Project => 2,   // Project(input, Cols)
            QueryOps::Scan => 3,      // Scan(Table, Cols | Null, Predicate | Null)
            QueryOps::IndexScan => 2, // IndexScan(Index, Predicate | Null)
            QueryOps::Sort => 2,      // Sort(input, Order)
            QueryOps::Join => 3,      // Join(left, right, Predicate)
            QueryOps::NLJoin => 3,    // NLJoin(left, right, Predicate)
            QueryOps::SortJoin => 3,  // SortJoin(left, right, Predicate)
        }
    }
}

/// Represents a physical property set for query optimization.
/// This is a simple example with a single property: sorted stored in a bitmap.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PhysicalPropertySet {
    sort: Bitmap<3>,
}

impl PhysicalPropertySet {
    /// Creates a physical property set from a sort index.
    // FIXME: this is dummy test implementation
    // In a real implementation, we would call out to a larger data struct to get the real column sort bitmaps
    pub fn from_cols(cols: String) -> Self {
        let mut sort = Bitmap::new();
        match cols.as_str() {
            "x" => {
                sort.set(0, true); // Assume column "x" is sorted
            }
            "y" => {
                sort.set(1, true); // Assume column "y" is sorted
            }
            "z" => {
                sort.set(2, true); // Assume column "z" is sorted
            }
            "xy" => {
                sort.set(0, true); // Assume both "x" and "y" are sorted
                sort.set(1, true);
            }
            "yz" => {
                sort.set(1, true); // Assume both "y" and "z" are sorted
                sort.set(2, true);
            }
            "xz" => {
                sort.set(0, true); // Assume both "x" and "z" are sorted
                sort.set(2, true);
            }
            "xyz" => {
                sort.set(0, true); // Assume all "x", "y", and "z" are sorted
                sort.set(1, true);
                sort.set(2, true);
            }
            _ => {}
        }
        PhysicalPropertySet { sort }
    }
}

impl Display for PhysicalPropertySet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PhysicalPropertySet(sort: {:?})", self.sort)
    }
}

impl PropertySet for PhysicalPropertySet {
    fn bottom() -> Self {
        Self {
            sort: Bitmap::new(),
        }
    }
}

impl PartialOrd for PhysicalPropertySet {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.sort.partial_cmp(&other.sort)
    }
}

impl From<QueryOps> for PhysicalPropertySet {
    fn from(op: QueryOps) -> Self {
        match op {
            QueryOps::Col(s) => PhysicalPropertySet::from_cols(s),
            _ => PhysicalPropertySet::bottom(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let result = QueryOps::parse("Col[x]");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), QueryOps::Col("x".to_string()));

        let result = QueryOps::parse("Table[users]");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), QueryOps::Table("users".to_string()));

        let result = QueryOps::parse("Col[some_column]");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), QueryOps::Col("some_column".to_string()));
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
        let result = Parser::<QueryOps>::parse_expr("Access(Table[users], Col[id])");
        assert!(result.is_ok());
        let expr = result.unwrap();
        assert_eq!(*expr.op(), QueryOps::Access);
        assert_eq!(expr.args().len(), 2);
        assert_eq!(*expr.args()[0].op(), QueryOps::Table("users".to_string()));
        assert_eq!(*expr.args()[1].op(), QueryOps::Col("id".to_string()));
    }

    #[test]
    fn test_parse_complex_expressions_with_square_brackets() {
        // Test a more complex expression with nested Col[] and Table[]
        let result = Parser::<QueryOps>::parse_expr(
            "Select(Scan(Table[employees]), Eq(Col[department], engineering))",
        );
        assert!(result.is_ok());
        let expr = result.unwrap();
        assert_eq!(*expr.op(), QueryOps::Select);
        assert_eq!(expr.args().len(), 2);

        // First argument: Scan(Table[employees])
        let scan_expr = &expr.args()[0];
        assert_eq!(*scan_expr.op(), QueryOps::Scan);
        assert_eq!(scan_expr.args().len(), 1);
        assert_eq!(
            *scan_expr.args()[0].op(),
            QueryOps::Table("employees".to_string())
        );

        // Second argument: Eq(Col[department], engineering)
        let eq_expr = &expr.args()[1];
        assert_eq!(*eq_expr.op(), QueryOps::Eq);
        assert_eq!(eq_expr.args().len(), 2);
        assert_eq!(
            *eq_expr.args()[0].op(),
            QueryOps::Col("department".to_string())
        );
        assert_eq!(
            *eq_expr.args()[1].op(),
            QueryOps::ConstStr("engineering".to_string())
        );
    }

    #[test]
    fn test_pattern_parsing_with_square_brackets() {
        // Test pattern parsing with variables in a simpler form
        let result = Parser::<QueryOps>::parse_pattern("Access(?table_expr, ?column_expr)");
        assert!(result.is_ok());
        let pattern = result.unwrap();

        // Check top-level operator
        match pattern.op() {
            OpOrVar::Op(op) => assert_eq!(*op, QueryOps::Access),
            OpOrVar::Var(_) => panic!("Expected operator, got variable"),
        }

        assert_eq!(pattern.args().len(), 2);

        // First argument: ?table_expr - should be parsed as a variable
        match pattern.args()[0].op() {
            OpOrVar::Var(var) => assert_eq!(var, "table_expr"),
            OpOrVar::Op(_) => panic!("Expected variable, got operator"),
        }

        // Second argument: ?column_expr - should be parsed as a variable
        match pattern.args()[1].op() {
            OpOrVar::Var(var) => assert_eq!(var, "column_expr"),
            OpOrVar::Op(_) => panic!("Expected variable, got operator"),
        }
    }

    #[test]
    fn test_pattern_parsing_with_concrete_square_brackets() {
        // Test pattern parsing with concrete Table[] and Col[] mixed with variables
        let result = Parser::<QueryOps>::parse_pattern("Access(Table[users], ?column_var)");
        assert!(result.is_ok());
        let pattern = result.unwrap();

        // Check top-level operator
        match pattern.op() {
            OpOrVar::Op(op) => assert_eq!(*op, QueryOps::Access),
            OpOrVar::Var(_) => panic!("Expected operator, got variable"),
        }

        assert_eq!(pattern.args().len(), 2);

        // First argument: Table[users] - should be parsed as a concrete Table operator
        match pattern.args()[0].op() {
            OpOrVar::Op(op) => assert_eq!(*op, QueryOps::Table("users".to_string())),
            OpOrVar::Var(_) => panic!("Expected operator, got variable"),
        }

        // Second argument: ?column_var - should be parsed as a variable
        match pattern.args()[1].op() {
            OpOrVar::Var(var) => assert_eq!(var, "column_var"),
            OpOrVar::Op(_) => panic!("Expected variable, got operator"),
        }
    }
}
