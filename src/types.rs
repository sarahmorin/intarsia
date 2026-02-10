use std::fmt::Display;

/// Common type definitions used across the project.
///

/// Supported Datatypes in a databse.
// TODO: This is a very basic implementation and can be extended in the future to support more features (e.g., Float, String, etc.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataType {
    Int,    // i64
    String, // String
    Bool,   // bool
            // Future data types can be added here (e.g., Float, String, etc.)
}

impl Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataType::Int => write!(f, "DT::Int"),
            DataType::String => write!(f, "DT::String"),
            DataType::Bool => write!(f, "DT::Bool"),
        }
    }
}

/// Table object identifiers
pub type TableId = u64;
pub type ColumnId = u64;
pub type IndexId = u64;
