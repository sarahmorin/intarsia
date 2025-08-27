use crate::impl_ast_default;
use crate::parser::Parser;
use crate::types::*;
use bitmaps::Bitmap;
/// A stand in language for testing purposes.
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::str::FromStr;

/// Represents the test language.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ops {
    // Constants
    Col(String),
    Table(String),
    ConstStr(String),
    ConstInt(i32),
    ConstBool(bool),
    // Comparison Operators
    Eq,
    Neq,
    Lt,
    Gt,
    Le,
    Ge,
    // Bool Operators
    And,
    Or,
    Not,
    // Logical Query Ops
    Get,
    Filter,
    Join,
    Project,
    // Physical Query Ops
    Scan,
    IndexScan,
    Sort,
    NLJoin,
    HashJoin,
}

impl FromStr for Ops {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();

        match trimmed {
            "Eq" => Ok(Ops::Eq),
            "Neq" => Ok(Ops::Neq),
            "Lt" => Ok(Ops::Lt),
            "Gt" => Ok(Ops::Gt),
            "Le" => Ok(Ops::Le),
            "Ge" => Ok(Ops::Ge),
            "And" => Ok(Ops::And),
            "Or" => Ok(Ops::Or),
            "Not" => Ok(Ops::Not),
            "Get" => Ok(Ops::Get),
            "Filter" => Ok(Ops::Filter),
            "Join" => Ok(Ops::Join),
            "Project" => Ok(Ops::Project),
            "Scan" => Ok(Ops::Scan),
            "IndexScan" => Ok(Ops::IndexScan),
            "Sort" => Ok(Ops::Sort),
            "NLJoin" => Ok(Ops::NLJoin),
            "HashJoin" => Ok(Ops::HashJoin),
            // Boolean constants
            "true" => Ok(Ops::ConstBool(true)),
            "false" => Ok(Ops::ConstBool(false)),
            _ => {
                // Try parsing as Col[x] format
                if trimmed.starts_with("Col[") && trimmed.ends_with(']') {
                    let inner = &trimmed[4..trimmed.len() - 1]; // Extract content between Col[ and ]
                    return Ok(Ops::Col(inner.to_string()));
                }

                // Try parsing as Table[x] format
                if trimmed.starts_with("Table[") && trimmed.ends_with(']') {
                    let inner = &trimmed[6..trimmed.len() - 1]; // Extract content between Table[ and ]
                    return Ok(Ops::Table(inner.to_string()));
                }

                // Try parsing as integer
                if let Ok(i) = trimmed.parse::<i32>() {
                    return Ok(Ops::ConstInt(i));
                }

                // Try parsing as quoted string
                if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
                    let inner = &trimmed[1..trimmed.len() - 1];
                    return Ok(Ops::ConstStr(inner.to_string()));
                }

                // Default to unquoted string constant
                Ok(Ops::ConstStr(trimmed.to_string()))
            }
        }
    }
}

impl Display for Ops {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ops::Col(s) => write!(f, "Col({})", s),
            Ops::Table(s) => write!(f, "Table({})", s),
            Ops::ConstStr(s) => write!(f, "ConstStr({})", s),
            Ops::ConstInt(i) => write!(f, "ConstInt({})", i),
            Ops::ConstBool(b) => write!(f, "ConstBool({})", b),
            Ops::Eq => write!(f, "=="),
            Ops::Neq => write!(f, "!="),
            Ops::Lt => write!(f, "<"),
            Ops::Gt => write!(f, ">"),
            Ops::Le => write!(f, "<="),
            Ops::Ge => write!(f, ">="),
            Ops::And => write!(f, "And"),
            Ops::Or => write!(f, "Or"),
            Ops::Not => write!(f, "Not"),
            Ops::Get => write!(f, "Get"),
            Ops::Filter => write!(f, "Filter"),
            Ops::Join => write!(f, "Join"),
            Ops::Project => write!(f, "Project"),
            Ops::Scan => write!(f, "Scan"),
            Ops::IndexScan => write!(f, "IndexScan"),
            Ops::Sort => write!(f, "Sort"),
            Ops::NLJoin => write!(f, "NLJoin"),
            Ops::HashJoin => write!(f, "HashJoin"),
        }
    }
}

impl AST for Ops {
    impl_ast_default!();
}

/// Represents a physical property set for query optimization.
/// This is a simple example with a single property: sorted stored in a bitmap.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PhysicalPropertySet {
    sort: Bitmap<3>,
}

impl PhysicalPropertySet {
    /// Creates a physical property set from a sort index
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

impl Property for PhysicalPropertySet {
    fn contains(&self, other: &Self) -> bool {
        self.sort & other.sort == other.sort
    }

    fn bottom() -> Self {
        Self {
            sort: Bitmap::new(),
        }
    }
}

impl PartialOrd for PhysicalPropertySet {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.contains(other) {
            Some(std::cmp::Ordering::Greater)
        } else if other.contains(self) {
            Some(std::cmp::Ordering::Less)
        } else {
            Some(std::cmp::Ordering::Equal)
        }
    }
}

impl From<Ops> for PhysicalPropertySet {
    fn from(op: Ops) -> Self {
        match op {
            Ops::Col(s) => PhysicalPropertySet::from_cols(s),
            _ => PhysicalPropertySet::bottom(),
        }
    }
}

impl From<Expr<Ops>> for OpInfo<PhysicalPropertySet> {
    fn from(expr: Expr<Ops>) -> Self {
        match expr.op() {
            Ops::IndexScan => {
                let c = expr
                    .args()
                    .get(1)
                    .unwrap_or_else(|| panic!("IndexScan requires at least 2 arguments"));
                let col_prop = PhysicalPropertySet::from(c.op().clone());
                OpInfo::new(
                    2,
                    col_prop.clone(),
                    vec![PhysicalPropertySet::bottom(), col_prop],
                )
            }
            _ => OpInfo::default(expr.args().len()),
        }
    }
}

impl From<Pattern<Ops>> for OpInfo<PhysicalPropertySet> {
    fn from(pattern: Pattern<Ops>) -> Self {
        match pattern.op() {
            OpOrVar::Op(Ops::IndexScan) => {
                let c = match pattern
                    .args()
                    .get(1)
                    .unwrap_or_else(|| panic!("IndexScan requires at least 2 arguments"))
                    .op()
                {
                    OpOrVar::Op(op) => op,
                    OpOrVar::Var(v) => panic!("Expected concrete operator, found variable: {}", v),
                };
                let col_prop = PhysicalPropertySet::from(c.clone());
                OpInfo::new(
                    2,
                    col_prop.clone(),
                    vec![PhysicalPropertySet::bottom(), col_prop],
                )
            }
            _ => OpInfo::default(pattern.args().len()),
        }
    }
}

impl PropLang<Ops, PhysicalPropertySet> for Expr<Ops> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_boolean_constants() {
        let result = Ops::from_str("true");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Ops::ConstBool(true));

        let result = Ops::from_str("false");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Ops::ConstBool(false));
    }

    #[test]
    fn test_parse_integer_constants() {
        let result = Ops::from_str("100");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Ops::ConstInt(100));

        let result = Ops::from_str("-2");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Ops::ConstInt(-2));

        let result = Ops::from_str("0");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Ops::ConstInt(0));
    }

    #[test]
    fn test_parse_col_and_table() {
        let result = Ops::from_str("Col[x]");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Ops::Col("x".to_string()));

        let result = Ops::from_str("Table[users]");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Ops::Table("users".to_string()));

        let result = Ops::from_str("Col[some_column]");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Ops::Col("some_column".to_string()));
    }

    #[test]
    fn test_parse_operators() {
        let result = Ops::from_str("And");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Ops::And);

        let result = Ops::from_str("Or");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Ops::Or);

        let result = Ops::from_str("Not");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Ops::Not);

        let result = Ops::from_str("Eq");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Ops::Eq);
    }

    #[test]
    fn test_parse_string_constants() {
        let result = Ops::from_str("hello");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Ops::ConstStr("hello".to_string()));

        let result = Ops::from_str("some_string");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Ops::ConstStr("some_string".to_string()));
    }

    #[test]
    fn test_parse_variable_error() {
        // Variables starting with '?' should be parsed as string constants now
        // since the FromStr implementation doesn't reject them
        let result = Ops::from_str("?x");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Ops::ConstStr("?x".to_string()));
    }

    #[test]
    fn test_parse_whitespace_handling() {
        let result = Ops::from_str("  true  ");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Ops::ConstBool(true));

        let result = Ops::from_str("  100  ");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Ops::ConstInt(100));

        let result = Ops::from_str("  And  ");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Ops::And);
    }

    #[test]
    fn test_parse_expr_with_ops_parser() {
        // Test parsing a simple expression with the new Parser struct
        let result = Parser::<Ops>::parse_expr("And(true, false)");
        assert!(result.is_ok());
        let expr = result.unwrap();
        assert_eq!(*expr.op(), Ops::And);
        assert_eq!(expr.args().len(), 2);
        assert_eq!(*expr.args()[0].op(), Ops::ConstBool(true));
        assert_eq!(*expr.args()[1].op(), Ops::ConstBool(false));
    }

    #[test]
    fn test_parse_expr_with_constants() {
        // Test parsing with integer constants
        let result = Parser::<Ops>::parse_expr("Eq(100, -50)");
        assert!(result.is_ok());
        let expr = result.unwrap();
        assert_eq!(*expr.op(), Ops::Eq);
        assert_eq!(expr.args().len(), 2);
        assert_eq!(*expr.args()[0].op(), Ops::ConstInt(100));
        assert_eq!(*expr.args()[1].op(), Ops::ConstInt(-50));
    }

    #[test]
    fn test_parse_expr_with_col_table() {
        let result = Parser::<Ops>::parse_expr("Get(Table[users], Col[id])");
        assert!(result.is_ok());
        let expr = result.unwrap();
        assert_eq!(*expr.op(), Ops::Get);
        assert_eq!(expr.args().len(), 2);
        assert_eq!(*expr.args()[0].op(), Ops::Table("users".to_string()));
        assert_eq!(*expr.args()[1].op(), Ops::Col("id".to_string()));
    }

    #[test]
    fn test_parse_complex_expressions_with_square_brackets() {
        // Test a more complex expression with nested Col[] and Table[]
        let result = Parser::<Ops>::parse_expr(
            "Filter(Scan(Table[employees]), Eq(Col[department], engineering))",
        );
        assert!(result.is_ok());
        let expr = result.unwrap();
        assert_eq!(*expr.op(), Ops::Filter);
        assert_eq!(expr.args().len(), 2);

        // First argument: Scan(Table[employees])
        let scan_expr = &expr.args()[0];
        assert_eq!(*scan_expr.op(), Ops::Scan);
        assert_eq!(scan_expr.args().len(), 1);
        assert_eq!(
            *scan_expr.args()[0].op(),
            Ops::Table("employees".to_string())
        );

        // Second argument: Eq(Col[department], engineering)
        let eq_expr = &expr.args()[1];
        assert_eq!(*eq_expr.op(), Ops::Eq);
        assert_eq!(eq_expr.args().len(), 2);
        assert_eq!(*eq_expr.args()[0].op(), Ops::Col("department".to_string()));
        assert_eq!(
            *eq_expr.args()[1].op(),
            Ops::ConstStr("engineering".to_string())
        );
    }

    #[test]
    fn test_pattern_parsing_with_square_brackets() {
        // Test pattern parsing with variables in a simpler form
        let result = Parser::<Ops>::parse_pattern("Get(?table_expr, ?column_expr)");
        assert!(result.is_ok());
        let pattern = result.unwrap();

        // Check top-level operator
        match pattern.op() {
            OpOrVar::Op(op) => assert_eq!(*op, Ops::Get),
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
        let result = Parser::<Ops>::parse_pattern("Get(Table[users], ?column_var)");
        assert!(result.is_ok());
        let pattern = result.unwrap();

        // Check top-level operator
        match pattern.op() {
            OpOrVar::Op(op) => assert_eq!(*op, Ops::Get),
            OpOrVar::Var(_) => panic!("Expected operator, got variable"),
        }

        assert_eq!(pattern.args().len(), 2);

        // First argument: Table[users] - should be parsed as a concrete Table operator
        match pattern.args()[0].op() {
            OpOrVar::Op(op) => assert_eq!(*op, Ops::Table("users".to_string())),
            OpOrVar::Var(_) => panic!("Expected operator, got variable"),
        }

        // Second argument: ?column_var - should be parsed as a variable
        match pattern.args()[1].op() {
            OpOrVar::Var(var) => assert_eq!(var, "column_var"),
            OpOrVar::Op(_) => panic!("Expected variable, got operator"),
        }
    }
}
