/// Minimal Database Catalog implementation.
///
/// This module stores metadata about tables, columns, and indexes in a database.

pub mod column;
pub mod index;
pub mod table;

use index::Index;
use table::Table;

use super::types::{DataType, IndexId, TableId};
use indexmap::IndexMap;

/// The Catalog struct manages database metadata.
#[derive(Debug, Clone)]
pub struct Catalog {
    // Map table name to ID
    pub table_ids: IndexMap<String, TableId>,
    // Table ID to Table metadata
    pub tables: IndexMap<TableId, Table>,
    // Map index name to ID
    pub index_ids: IndexMap<String, IndexId>,
    // Index ID to Index metadata
    pub indexes: IndexMap<IndexId, Index>,
}

impl Catalog {
    pub fn new() -> Self {
        Self {
            table_ids: IndexMap::new(),
            tables: IndexMap::new(),
            index_ids: IndexMap::new(),
            indexes: IndexMap::new(),
        }
    }

    /// Try to insert a new table into the catalog.
    fn try_new_table_id(&mut self, name: String) -> Result<TableId, String> {
        if self.table_ids.contains_key(&name) {
            Err(format!("Table with name '{}' already exists", name))
        } else {
            let new_id = self.table_ids.len() as TableId + 1;
            self.table_ids.insert(name, new_id);
            Ok(new_id)
        }
    }

    /// Try to insert a new index into the catalog.
    fn try_new_index_id(&mut self, name: String) -> Result<IndexId, String> {
        if self.index_ids.contains_key(&name) {
            Err(format!("Index with name '{}' already exists", name))
        } else {
            let new_id = self.index_ids.len() as IndexId + 1;
            self.index_ids.insert(name, new_id);
            Ok(new_id)
        }
    }

    /// Create a new table with the given name and columns.
    pub fn create_table_with_cols(
        &mut self,
        name: String,
        cols: Vec<(String, DataType)>,
    ) -> Result<TableId, String> {
        let table_id = self.try_new_table_id(name.clone())?;
        let table = Table::new(table_id, name, cols, 0)?;
        self.tables.insert(table_id, table);
        Ok(table_id)
    }

    fn generate_index_name(&self, table_name: &str, column_names: &[String]) -> String {
        let col_part = column_names.join(".");
        format!("{}_{}", table_name, col_part)
    }

    /// Create a new index on the specified table and columns.
    pub fn create_table_index(
        &mut self,
        name: Option<String>,
        table_name: String,
        columns: Vec<String>,
    ) -> Result<IndexId, String> {
        // If name is not provided, generate a name based on the table and columns
        let name = match name {
            Some(n) => n,
            None => self.generate_index_name(&table_name, &columns),
        };
        
        let index_id = self.try_new_index_id(name.clone())?;
        
        // Verify that the specified table exists
        if let Some(table_id) = self.table_ids.get(&table_name) {
            let table = self.tables.get(table_id).unwrap();
            let mut column_ids = Vec::new();
            
            // Verify that each specified column exists in the table
            for col_name in columns {
                if let Some(col_id) = table.get_column_id(&col_name) {
                    column_ids.push(col_id);
                } else {
                    return Err(format!(
                        "Column with name '{}' does not exist in table '{}'",
                        col_name, table_name
                    ));
                }
            }
            
            let index = Index::new(index_id, name, *table_id, column_ids);
            self.indexes.insert(index_id, index);
            Ok(index_id)
        } else {
            Err(format!("Table with name '{}' does not exist", table_name))
        }
    }

    pub fn list_tables(&self) -> Vec<String> {
        self.table_ids.keys().cloned().collect()
    }

    pub fn table_ids(&self) -> Vec<TableId> {
        self.table_ids.values().cloned().collect()
    }

    pub fn list_indexes(&self) -> Vec<String> {
        self.index_ids.keys().cloned().collect()
    }

    pub fn index_ids(&self) -> Vec<IndexId> {
        self.index_ids.values().cloned().collect()
    }

    /// Get a reference to a table by its name.
    pub fn get_table(&self, name: &str) -> Option<&Table> {
        if let Some(table_id) = self.table_ids.get(name) {
            self.tables.get(table_id)
        } else {
            None
        }
    }

    /// Get a reference to a table by its ID.
    pub fn get_table_by_id(&self, table_id: TableId) -> Option<&Table> {
        self.tables.get(&table_id)
    }

    /// Get a reference to an index by its name.
    pub fn get_index(&self, name: &str) -> Option<&Index> {
        if let Some(index_id) = self.index_ids.get(name) {
            self.indexes.get(index_id)
        } else {
            None
        }
    }

    /// Get a reference to an index by its ID.
    pub fn get_index_by_id(&self, index_id: IndexId) -> Option<&Index> {
        self.indexes.get(&index_id)
    }
}
