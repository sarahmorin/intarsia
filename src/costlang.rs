/// A stand in language for testing purposes.
/// The only property we track is cost.
/// The operators are made up functions with cost requirements for testing.

use crate::property::PropertySet;
use crate::types::*;
use crate::parser::Parseable;
use std::fmt::Display;

pub type TableName = String;
pub type ColumnName = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Operators {
    // Constant Value Types
    Table(TableName),
    Column(TableName, ColumnName),
    ColumnSet(TableName, Vec<ColumnName>),
    ConstantInt(i64),
    ConstantString(String),
    ConstantBool(bool),
    Null,
    // Logical Operators
    Select,
    Join,
    Project,
    // Physical Scan Operators
    TableScan,  // Full table scan, most costly
    IndexScan,  // Uses an index, less costly
    Lookup,     // Uses an index to lookup specific rows, least costly
    // Physical Sorting Operators
    SlowSort,    // Sorts data slowly, most costly
    Sort,        // Sorts data using a standard algorithm, less costly
    QuickSort,   // Sorts data using the quicksort algorithm, least costly
    // Physical Join Operators
    CostCutJoin,     // Only accepts low cost inputs
    BigSpenderJoin,  // Only accepts high cost inputs
    NLJoin,          // Accepts any cost inputs
    // Physical Filter Operators
    FastFilter,  // Cost depends on input, if an index exists it's cheaper, otherwise its more expensive,
    Filter,      // Standard filter operator
    // Predicates and Arithmetic
    Eq,
    Neq,
    Lt,
    Gt,
    Lte,
    Gte,
    And,
    Or,
    Not,
    Add,
    Sub,
    Mul,
    Div,
}

impl Parseable for Operators {
    /// Parses a string into a cost::Operators enum.
    fn parse(s: &str) -> Result<Self, String> {
        let trimmed = s.trim();

        match trimmed {
            // Constants
            "Null" => Ok(Operators::Null),
            "true" => Ok(Operators::ConstantBool(true)),
            "false" => Ok(Operators::ConstantBool(false)),
            // Logical Operators
            "Select" => Ok(Operators::Select),
            "Join" => Ok(Operators::Join),
            "Project" => Ok(Operators::Project),
            // Physical Scan Operators
            "TableScan" => Ok(Operators::TableScan),
            "IndexScan" => Ok(Operators::IndexScan),
            "Lookup" => Ok(Operators::Lookup),
            // Physical Sorting Operators
            "SlowSort" => Ok(Operators::SlowSort),
            "Sort" => Ok(Operators::Sort),
            "QuickSort" => Ok(Operators::QuickSort),
            // Physical Join Operators
            "CostCutJoin" => Ok(Operators::CostCutJoin),
            "BigSpenderJoin" => Ok(Operators::BigSpenderJoin),
            "NLJoin" => Ok(Operators::NLJoin),
            // Physical Filter Operators
            "FastFilter" => Ok(Operators::FastFilter),
            "Filter" => Ok(Operators::Filter),
            // Predicate Operators
            "Eq" => Ok(Operators::Eq),
            "Neq" => Ok(Operators::Neq),
            "Lt" => Ok(Operators::Lt),
            "Gt" => Ok(Operators::Gt),
            "Lte" => Ok(Operators::Lte),
            "Gte" => Ok(Operators::Gte),
            "And" => Ok(Operators::And),
            "Or" => Ok(Operators::Or),
            "Not" => Ok(Operators::Not),
            // Arithmetic Operators
            "Add" => Ok(Operators::Add),
            "Sub" => Ok(Operators::Sub),
            "Mul" => Ok(Operators::Mul),
            "Div" => Ok(Operators::Div),
            _ => {
                // Try parsing as ColumnSet in the form: Col[table][c1,c2,...]
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
                            return Ok(Operators::ColumnSet(table_name.to_string(), columns));
                        }
                    }
                }

                // Try parsing as Table[x]
                if trimmed.starts_with("Table[") && trimmed.ends_with(']') {
                    let inner = &trimmed[6..trimmed.len() - 1];
                    return Ok(Operators::Table(inner.to_string()));
                }

                // Try parsing as integer
                if let Ok(i) = trimmed.parse::<i64>() {
                    return Ok(Operators::ConstantInt(i));
                }

                // Try parsing as quoted string
                if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
                    let inner = &trimmed[1..trimmed.len() - 1];
                    return Ok(Operators::ConstantString(inner.to_string()));
                }

                // Default: treat as unquoted string constant
                Ok(Operators::ConstantString(trimmed.to_string()))
            }
        }
    }
}

impl Display for Operators {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Constants
            Operators::ConstantString(s) => write!(f, "ConstantString({})", s),
            Operators::ConstantInt(i) => write!(f, "ConstantInt({})", i),
            Operators::ConstantBool(b) => write!(f, "ConstantBool({})", b),
            Operators::Null => write!(f, "Null"),
            // Table/Columns
            Operators::Table(t) => write!(f, "Table({})", t),
            Operators::Column(table, col) => write!(f, "Column({},{})", table, col),
            Operators::ColumnSet(table, cols) => write!(f, "ColumnSet({}, {:?})", table, cols),
            // Logical
            Operators::Select => write!(f, "Select"),
            Operators::Join => write!(f, "Join"),
            Operators::Project => write!(f, "Project"),
            // Scans
            Operators::TableScan => write!(f, "TableScan"),
            Operators::IndexScan => write!(f, "IndexScan"),
            Operators::Lookup => write!(f, "Lookup"),
            // Sorts
            Operators::SlowSort => write!(f, "SlowSort"),
            Operators::Sort => write!(f, "Sort"),
            Operators::QuickSort => write!(f, "QuickSort"),
            // Joins
            Operators::CostCutJoin => write!(f, "CostCutJoin"),
            Operators::BigSpenderJoin => write!(f, "BigSpenderJoin"),
            Operators::NLJoin => write!(f, "NLJoin"),
            // Filters
            Operators::FastFilter => write!(f, "FastFilter"),
            Operators::Filter => write!(f, "Filter"),
            // Predicates
            Operators::Eq => write!(f, "=="),
            Operators::Neq => write!(f, "!="),
            Operators::Lt => write!(f, "<"),
            Operators::Gt => write!(f, ">"),
            Operators::Lte => write!(f, "<="),
            Operators::Gte => write!(f, ">="),
            Operators::And => write!(f, "And"),
            Operators::Or => write!(f, "Or"),
            Operators::Not => write!(f, "Not"),
            // Arithmetic
            Operators::Add => write!(f, "Add"),
            Operators::Sub => write!(f, "Sub"),
            Operators::Mul => write!(f, "Mul"),
            Operators::Div => write!(f, "Div"),
        }
    }
}