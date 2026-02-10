use std::fmt::Display;

/// An Index is a data structure that allows for efficient lookup of rows in a table based on the values of one or more columns.
/// In this module, we define the `Index` struct, which represents an index in a database.
/// This is a very basic implementation and can be extended in the future to support more features.
use crate::types::{ColumnId, IndexId, TableId};

#[derive(Debug, Clone)]
pub(crate) struct Index {
    pub id: IndexId,       // Unique identifier for the index
    pub name: String,      // Name of the index
    pub table_id: TableId, // ID of the table that this index belongs to
    pub column_ids: Vec<ColumnId>, // ORDERED list of column IDs that are indexed by this index
                           // Future fields can be added here (e.g., index type, etc.)
}

impl Display for Index {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Index {{ id: {}, name: {:?}, table_id: {}, column_ids: {:?} }}",
            self.id, self.name, self.table_id, self.column_ids
        )
    }
}

impl Index {
    pub fn new(id: IndexId, name: String, table_id: TableId, column_ids: Vec<ColumnId>) -> Self {
        Self {
            id,
            name,
            table_id,
            column_ids,
        }
    }
}

// TODO: Implement comparison traits for Index (e.g., PartialEq, Eq, PartialOrd, Ord) if needed in the future.
