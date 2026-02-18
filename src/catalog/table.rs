use std::fmt::Display;

/// Tables are the main data source. They are collections of columns, and they are used to store and organize data in a database.
/// In this module, we define the `Table` struct, which represents a table in a database.
/// This is a very basic implementation and can be extended in the future to support more features.
use crate::{
    catalog::column::Column,
    types::{ColumnId, DataType, TableId},
};
use indexmap::IndexMap;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct Table {
    pub id: TableId,                         // Unique identifier for the table
    pub name: String,                        // Name of the table
    column_ids: IndexMap<String, ColumnId>,  // List of column IDs that belong to this table
    column_data: BTreeMap<ColumnId, Column>, // Map of column ID to Column metadata (for easy access)
    // Future fields can be added here (e.g., constraints, indexes, etc.)
    est_num_rows: usize, // Number of rows in the table (for statistics purposes)
    est_row_size: usize, // Average size of a row in bytes (for statistics purposes)
}

impl Display for Table {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let col_str = self
            .column_ids
            .keys()
            .cloned()
            .collect::<Vec<String>>()
            .join(", ");
        write!(
            f,
            "Table {{ id: {}, name: {}, columns: [{}], est_rows: {}, est_row_size: {}}}",
            self.id, self.name, col_str, self.est_num_rows, self.est_row_size
        )
    }
}

#[allow(dead_code)]
impl Table {
    pub fn new(
        id: TableId,
        name: String,
        columns: Vec<(String, DataType)>,
        est_num_rows: usize,
    ) -> Result<Self, String> {
        let mut column_ids = IndexMap::new();
        let mut column_data = BTreeMap::new();
        for (i, (col_name, col_type)) in columns.into_iter().enumerate() {
            let col_id = i as ColumnId + 1; // Column IDs start from 1
            if column_ids.contains_key(&col_name) {
                return Err(format!(
                    "Duplicate column name '{}' in table '{}', ignoring this column",
                    col_name, name
                ));
            }
            column_ids.insert(col_name.clone(), col_id);
            column_data.insert(col_id, Column::new(col_id, col_name, col_type));
        }

        let est_row_size = column_data
            .values()
            .map(|col| col.data_type.size_in_bytes())
            .sum();

        Ok(Self {
            id,
            name,
            column_ids,
            column_data,
            est_num_rows,
            est_row_size,
        })
    }

    /// Retrieves a column by its name. Returns `None` if the column does not exist.
    pub fn get_column(&self, col_name: &str) -> Option<&Column> {
        self.column_ids
            .get(col_name)
            .and_then(|col_id| self.column_data.get(col_id))
    }

    /// Retrieves a column ID by its name. Returns `None` if the column does not exist.
    pub fn get_column_id(&self, col_name: &str) -> Option<ColumnId> {
        self.column_ids.get(col_name).cloned()
    }

    /// Retrieves a column by its ID. Returns `None` if the column does not exist.
    pub fn get_column_by_id(&self, col_id: ColumnId) -> Option<&Column> {
        self.column_data.get(&col_id)
    }

    /// Returns the number of columns in the table.
    pub fn num_columns(&self) -> usize {
        self.column_ids.len()
    }

    /// Sets the estimated number of rows in the table. This can be used for statistics purposes.
    pub fn set_est_num_rows(&mut self, est_num_rows: usize) {
        self.est_num_rows = est_num_rows;
    }

    /// Retrieves the estimated number of rows in the table.
    pub fn get_est_num_rows(&self) -> usize {
        self.est_num_rows
    }

    /// Retrieves the estimated row size in bytes.
    pub fn get_est_row_size(&self) -> usize {
        self.est_row_size
    }

    /// Get tuples per page based on the estimated row size and a fixed page size (e.g., 4096 bytes).
    pub fn get_tuples_per_page(&self) -> usize {
        const PAGE_SIZE: usize = 4096; // 4KB page size
        if self.est_row_size == 0 {
            return 0; // Avoid division by zero if row size is unknown
        }
        PAGE_SIZE / self.est_row_size
    }

    /// Get the estimated number of blocks needed to store the table based on the estimated number of rows and tuples per page.
    pub fn get_est_num_blocks(&self) -> usize {
        let tuples_per_page = self.get_tuples_per_page();
        if tuples_per_page == 0 {
            return 0; // Avoid division by zero if tuples per page is unknown
        }
        (self.est_num_rows + tuples_per_page - 1) / tuples_per_page // Round up division
    }
}
