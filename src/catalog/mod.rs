/// Minimal Database Catalog implementation. This is used to store metadata about tables and other database objects.
/// This is a very basic implementation and can be extended in the future to support more features.
pub(crate) mod column;
pub(crate) mod index;
pub(crate) mod table;

use column::Column;
use index::Index;
use table::Table;

use crate::types::{ColumnId, DataType, IndexId, TableId};
use indexmap::IndexMap;

/// The Catalog struct is the main entry point for managing database metadata. It provides methods for creating tables, adding columns, and managing indexes.
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

    /// Try to insert a new table into the catalog. If a table with the same name already exists, it will return an error.
    fn try_new_table_id(&mut self, name: String) -> Result<TableId, String> {
        if self.table_ids.contains_key(&name) {
            Err(format!("Table with name '{}' already exists", name))
        } else {
            let new_id = self.table_ids.len() as TableId + 1;
            self.table_ids.insert(name, new_id);
            Ok(new_id)
        }
    }

    /// Try to insert a new index into the catalog. If an index with the same name already exists, it will return an error.
    fn try_new_index_id(&mut self, name: String) -> Result<IndexId, String> {
        if self.index_ids.contains_key(&name) {
            Err(format!("Index with name '{}' already exists", name))
        } else {
            let new_id = self.index_ids.len() as IndexId + 1;
            self.index_ids.insert(name, new_id);
            Ok(new_id)
        }
    }

    /// Create a new table with the given name and columns. If a table with the same name already exists, it will return an error.
    /// Assumes the table is actually new (and empty) and sets the estimated number of rows to 0.
    pub fn create_table_with_cols(
        &mut self,
        name: String,
        cols: Vec<(String, DataType)>,
    ) -> Result<TableId, String> {
        // Try to create a new table ID. If a table with the same name already exists, this will return an error.
        let table_id = self.try_new_table_id(name.clone())?;
        // Create the column metadata for each column and insert it into the table.
        let table = Table::new(table_id, name, cols, 0)?;
        self.tables.insert(table_id, table);
        Ok(table_id)
    }

    fn generate_index_name(&self, table_name: &str, column_names: &[String]) -> String {
        let col_part = column_names.join(".");
        format!("{}_{}", table_name, col_part)
    }

    /// Create a new index on the specified table and columns. If an index with the same name already exists, it will return an error.
    /// Verifies that the specified table and columns exist before creating the index. If any of the specified columns do not exist in the table, it will return an error.
    pub fn create_table_index(
        &mut self,
        name: Option<String>,
        table_name: String,
        columns: Vec<String>,
    ) -> Result<IndexId, String> {
        // If name is not provided, generate a name based on the table and columns.
        let name = match name {
            Some(n) => n,
            None => self.generate_index_name(&table_name, &columns),
        };
        // Check if an index with the same name already exists.
        let index_id = self.try_new_index_id(name.clone())?;
        // Verify that the specified table exists and get its ID.
        if let Some(table_id) = self.table_ids.get(&table_name) {
            let table = self.tables.get(table_id).unwrap();
            let mut column_ids = Vec::new();
            // Verify that each specified column exists in the table and get its ID.
            for col_name in columns {
                if let Some(col_id) = table.column_ids.get(&col_name) {
                    column_ids.push(*col_id);
                } else {
                    return Err(format!(
                        "Column with name '{}' does not exist in table '{}'",
                        col_name, table_name
                    ));
                }
            }
            // Create the index and insert it into the catalog.
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

    /// Get a reference to a table by its name. Returns None if the table does not exist.
    pub fn get_table(&self, name: &str) -> Option<&Table> {
        if let Some(table_id) = self.table_ids.get(name) {
            self.tables.get(table_id)
        } else {
            None
        }
    }

    /// Get a reference to a table by its ID. Returns None if the table does not exist.
    pub fn get_table_by_id(&self, table_id: TableId) -> Option<&Table> {
        self.tables.get(&table_id)
    }

    /// Get a reference to an index by its name. Returns None if the index does not exist.
    pub fn get_index(&self, name: &str) -> Option<&Index> {
        if let Some(index_id) = self.index_ids.get(name) {
            self.indexes.get(index_id)
        } else {
            None
        }
    }

    /// Get a reference to an index by its ID. Returns None if the index does not exist.
    pub fn get_index_by_id(&self, index_id: IndexId) -> Option<&Index> {
        self.indexes.get(&index_id)
    }

    // QUESTION: Do we want to add methods for updating tables and indexes (e.g., adding columns to an existing table, etc.)? This would require additional logic to handle updates and ensure consistency.
    // For now, we can keep it simple and only allow creating new tables and indexes. We can always add update methods in the future if needed.
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DataType;

    #[test]
    fn test_new_catalog() {
        let catalog = Catalog::new();
        assert_eq!(catalog.table_ids.len(), 0);
        assert_eq!(catalog.tables.len(), 0);
        assert_eq!(catalog.index_ids.len(), 0);
        assert_eq!(catalog.indexes.len(), 0);
    }

    #[test]
    fn test_create_table_basic() {
        let mut catalog = Catalog::new();
        let cols = vec![
            ("id".to_string(), DataType::Int),
            ("name".to_string(), DataType::String),
        ];
        let result = catalog.create_table_with_cols("users".to_string(), cols);
        assert!(result.is_ok());
        let table_id = result.unwrap();
        assert_eq!(table_id, 1);
        assert_eq!(catalog.table_ids.len(), 1);
        assert_eq!(catalog.tables.len(), 1);
        assert!(catalog.table_ids.contains_key("users"));
    }

    #[test]
    fn test_duplicate_table_name() {
        let mut catalog = Catalog::new();
        let cols1 = vec![("id".to_string(), DataType::Int)];
        let cols2 = vec![("name".to_string(), DataType::String)];

        // First table should succeed
        let result1 = catalog.create_table_with_cols("users".to_string(), cols1);
        assert!(result1.is_ok());

        // Second table with same name should fail
        let result2 = catalog.create_table_with_cols("users".to_string(), cols2);
        assert!(result2.is_err());
        assert!(result2.unwrap_err().contains("already exists"));

        // Catalog should still only have one table
        assert_eq!(catalog.table_ids.len(), 1);
        assert_eq!(catalog.tables.len(), 1);
    }

    #[test]
    fn test_create_index_with_explicit_name() {
        let mut catalog = Catalog::new();
        let cols = vec![
            ("id".to_string(), DataType::Int),
            ("email".to_string(), DataType::String),
        ];
        catalog
            .create_table_with_cols("users".to_string(), cols)
            .unwrap();

        let result = catalog.create_table_index(
            Some("idx_users_email".to_string()),
            "users".to_string(),
            vec!["email".to_string()],
        );
        assert!(result.is_ok());
        let index_id = result.unwrap();
        assert_eq!(index_id, 1);
        assert_eq!(catalog.index_ids.len(), 1);
        assert_eq!(catalog.indexes.len(), 1);
        assert!(catalog.index_ids.contains_key("idx_users_email"));
    }

    #[test]
    fn test_duplicate_index_name() {
        let mut catalog = Catalog::new();
        let cols = vec![
            ("id".to_string(), DataType::Int),
            ("email".to_string(), DataType::String),
            ("name".to_string(), DataType::String),
        ];
        catalog
            .create_table_with_cols("users".to_string(), cols)
            .unwrap();

        // First index should succeed
        let result1 = catalog.create_table_index(
            Some("idx_users".to_string()),
            "users".to_string(),
            vec!["email".to_string()],
        );
        assert!(result1.is_ok());

        // Second index with same name should fail, even on different column
        let result2 = catalog.create_table_index(
            Some("idx_users".to_string()),
            "users".to_string(),
            vec!["name".to_string()],
        );
        assert!(result2.is_err());
        assert!(result2.unwrap_err().contains("already exists"));

        // Catalog should still only have one index
        assert_eq!(catalog.index_ids.len(), 1);
        assert_eq!(catalog.indexes.len(), 1);
    }

    #[test]
    fn test_autogenerate_index_name() {
        let mut catalog = Catalog::new();
        let cols = vec![
            ("id".to_string(), DataType::Int),
            ("email".to_string(), DataType::String),
        ];
        catalog
            .create_table_with_cols("users".to_string(), cols)
            .unwrap();

        // Create index with None for name - should autogenerate
        let result =
            catalog.create_table_index(None, "users".to_string(), vec!["email".to_string()]);
        assert!(result.is_ok());
        assert_eq!(catalog.index_ids.len(), 1);
        assert_eq!(catalog.indexes.len(), 1);

        // Check that a name was generated
        let index_names = catalog.list_indexes();
        assert_eq!(index_names.len(), 1);
        assert!(index_names[0].contains("users"));
        assert!(index_names[0].contains("email"));
    }

    #[test]
    fn test_autogenerate_index_name_produces_duplicate_error() {
        let mut catalog = Catalog::new();
        let cols = vec![
            ("id".to_string(), DataType::Int),
            ("email".to_string(), DataType::String),
        ];
        catalog
            .create_table_with_cols("users".to_string(), cols)
            .unwrap();

        // First autogenerated index should succeed
        let result1 =
            catalog.create_table_index(None, "users".to_string(), vec!["email".to_string()]);
        assert!(result1.is_ok());

        // Second autogenerated index on same table/column should fail with duplicate error
        let result2 =
            catalog.create_table_index(None, "users".to_string(), vec!["email".to_string()]);
        assert!(result2.is_err());
        assert!(result2.unwrap_err().contains("already exists"));

        // Should still have only one index
        assert_eq!(catalog.index_ids.len(), 1);
        assert_eq!(catalog.indexes.len(), 1);
    }

    #[test]
    fn test_autogenerate_multi_column_index_name() {
        let mut catalog = Catalog::new();
        let cols = vec![
            ("id".to_string(), DataType::Int),
            ("first_name".to_string(), DataType::String),
            ("last_name".to_string(), DataType::String),
        ];
        catalog
            .create_table_with_cols("users".to_string(), cols)
            .unwrap();

        // Create multi-column index with autogenerated name
        let result = catalog.create_table_index(
            None,
            "users".to_string(),
            vec!["first_name".to_string(), "last_name".to_string()],
        );
        assert!(result.is_ok());

        let index_names = catalog.list_indexes();
        assert_eq!(index_names.len(), 1);
        // Should contain both column names
        assert!(index_names[0].contains("first_name"));
        assert!(index_names[0].contains("last_name"));
    }

    #[test]
    fn test_get_table_exists() {
        let mut catalog = Catalog::new();
        let cols = vec![("id".to_string(), DataType::Int)];
        catalog
            .create_table_with_cols("users".to_string(), cols)
            .unwrap();

        let table = catalog.get_table("users");
        assert!(table.is_some());
        let table_ref = table.unwrap();
        assert_eq!(table_ref.name, "users");
    }

    #[test]
    fn test_get_table_not_exists() {
        let catalog = Catalog::new();
        let table = catalog.get_table("nonexistent");
        assert!(table.is_none());
    }

    #[test]
    fn test_get_table_by_id_exists() {
        let mut catalog = Catalog::new();
        let cols = vec![("id".to_string(), DataType::Int)];
        let table_id = catalog
            .create_table_with_cols("users".to_string(), cols)
            .unwrap();

        let table = catalog.get_table_by_id(table_id);
        assert!(table.is_some());
        let table_ref = table.unwrap();
        assert_eq!(table_ref.id, table_id);
        assert_eq!(table_ref.name, "users");
    }

    #[test]
    fn test_get_table_by_id_not_exists() {
        let catalog = Catalog::new();
        let table = catalog.get_table_by_id(999);
        assert!(table.is_none());
    }

    #[test]
    fn test_get_index_exists() {
        let mut catalog = Catalog::new();
        let cols = vec![("id".to_string(), DataType::Int)];
        catalog
            .create_table_with_cols("users".to_string(), cols)
            .unwrap();
        catalog
            .create_table_index(
                Some("idx_id".to_string()),
                "users".to_string(),
                vec!["id".to_string()],
            )
            .unwrap();

        let index = catalog.get_index("idx_id");
        assert!(index.is_some());
        let index_ref = index.unwrap();
        assert_eq!(index_ref.name, "idx_id");
    }

    #[test]
    fn test_get_index_not_exists() {
        let catalog = Catalog::new();
        let index = catalog.get_index("nonexistent");
        assert!(index.is_none());
    }

    #[test]
    fn test_get_index_by_id_exists() {
        let mut catalog = Catalog::new();
        let cols = vec![("id".to_string(), DataType::Int)];
        catalog
            .create_table_with_cols("users".to_string(), cols)
            .unwrap();
        let index_id = catalog
            .create_table_index(
                Some("idx_id".to_string()),
                "users".to_string(),
                vec!["id".to_string()],
            )
            .unwrap();

        let index = catalog.get_index_by_id(index_id);
        assert!(index.is_some());
        let index_ref = index.unwrap();
        assert_eq!(index_ref.id, index_id);
        assert_eq!(index_ref.name, "idx_id");
    }

    #[test]
    fn test_get_index_by_id_not_exists() {
        let catalog = Catalog::new();
        let index = catalog.get_index_by_id(999);
        assert!(index.is_none());
    }

    #[test]
    fn test_list_tables() {
        let mut catalog = Catalog::new();
        let cols1 = vec![("id".to_string(), DataType::Int)];
        let cols2 = vec![("id".to_string(), DataType::Int)];

        catalog
            .create_table_with_cols("users".to_string(), cols1)
            .unwrap();
        catalog
            .create_table_with_cols("products".to_string(), cols2)
            .unwrap();

        let tables = catalog.list_tables();
        assert_eq!(tables.len(), 2);
        assert!(tables.contains(&"users".to_string()));
        assert!(tables.contains(&"products".to_string()));
    }

    #[test]
    fn test_list_indexes() {
        let mut catalog = Catalog::new();
        let cols = vec![
            ("id".to_string(), DataType::Int),
            ("email".to_string(), DataType::String),
        ];
        catalog
            .create_table_with_cols("users".to_string(), cols)
            .unwrap();

        catalog
            .create_table_index(
                Some("idx_id".to_string()),
                "users".to_string(),
                vec!["id".to_string()],
            )
            .unwrap();
        catalog
            .create_table_index(
                Some("idx_email".to_string()),
                "users".to_string(),
                vec!["email".to_string()],
            )
            .unwrap();

        let indexes = catalog.list_indexes();
        assert_eq!(indexes.len(), 2);
        assert!(indexes.contains(&"idx_id".to_string()));
        assert!(indexes.contains(&"idx_email".to_string()));
    }

    #[test]
    fn test_create_index_on_nonexistent_table() {
        let mut catalog = Catalog::new();

        let result = catalog.create_table_index(
            Some("idx_fail".to_string()),
            "nonexistent".to_string(),
            vec!["id".to_string()],
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_create_index_on_nonexistent_column() {
        let mut catalog = Catalog::new();
        let cols = vec![("id".to_string(), DataType::Int)];
        catalog
            .create_table_with_cols("users".to_string(), cols)
            .unwrap();

        let result = catalog.create_table_index(
            Some("idx_fail".to_string()),
            "users".to_string(),
            vec!["nonexistent_column".to_string()],
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_multiple_tables_independent_ids() {
        let mut catalog = Catalog::new();
        let cols1 = vec![("id".to_string(), DataType::Int)];
        let cols2 = vec![("id".to_string(), DataType::Int)];
        let cols3 = vec![("id".to_string(), DataType::Int)];

        let id1 = catalog
            .create_table_with_cols("table1".to_string(), cols1)
            .unwrap();
        let id2 = catalog
            .create_table_with_cols("table2".to_string(), cols2)
            .unwrap();
        let id3 = catalog
            .create_table_with_cols("table3".to_string(), cols3)
            .unwrap();

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }

    #[test]
    fn test_multiple_indexes_independent_ids() {
        let mut catalog = Catalog::new();
        let cols = vec![
            ("col1".to_string(), DataType::Int),
            ("col2".to_string(), DataType::Int),
            ("col3".to_string(), DataType::Int),
        ];
        catalog
            .create_table_with_cols("table1".to_string(), cols)
            .unwrap();

        let id1 = catalog
            .create_table_index(
                Some("idx1".to_string()),
                "table1".to_string(),
                vec!["col1".to_string()],
            )
            .unwrap();
        let id2 = catalog
            .create_table_index(
                Some("idx2".to_string()),
                "table1".to_string(),
                vec!["col2".to_string()],
            )
            .unwrap();
        let id3 = catalog
            .create_table_index(
                Some("idx3".to_string()),
                "table1".to_string(),
                vec!["col3".to_string()],
            )
            .unwrap();

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }
}
