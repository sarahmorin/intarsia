/// Columns are the basic unit of data in a database, and they are used to store and organize data in tables.
/// In this module, we define the `Column` struct, which represents a column in a table, and we also define the `ColumnType` enum, which represents the different types of columns that can exist in a database.
/// This is a very basic implementation and can be extended in the future to support more features (e.g., constraints, default values, etc.).
use crate::types::{ColumnId, DataType};
use std::fmt::Display;

/// Column struct represents a column in a table.
#[derive(Debug, Clone)]
pub struct Column {
    pub id: ColumnId, // Unique identifier for the column
    pub name: String, // Name of the column
    pub data_type: DataType, // Data type of the column
                      // Future fields can be added here (e.g., constraints, default values, etc.)
}

impl Display for Column {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Column {{ id: {}, name: {}, data_type: {} }}",
            self.id, self.name, self.data_type
        )
    }
}

impl Column {
    pub fn new(id: ColumnId, name: String, data_type: DataType) -> Self {
        Self {
            id,
            name,
            data_type,
        }
    }
}

// TODO: Implement comparison traits for Column (e.g., PartialEq, Eq, PartialOrd, Ord) if needed in the future.
