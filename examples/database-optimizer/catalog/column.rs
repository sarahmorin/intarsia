/// Database column metadata.
use super::super::types::{ColumnId, DataType};
use std::fmt::Display;

/// Column struct represents a column in a table.
#[derive(Debug, Clone)]
pub struct Column {
    pub id: ColumnId,        // Unique identifier for the column
    pub name: String,        // Name of the column
    pub data_type: DataType, // Data type of the column
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
