use std::collections::BTreeSet as Set;
use std::{fmt::Display, str::FromStr};

/// Common type definitions used across the project.

/// Supported Datatypes in a databse.
// TODO: This is a very basic implementation and can be extended in the future to support more features (e.g., Float, String, etc.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataType {
    Int,    // i64
    String, // String
    Bool,   // bool
            // Future data types can be added here (e.g., Float, String, etc.)
}

impl DataType {
    pub fn size_in_bytes(&self) -> usize {
        match self {
            DataType::Int => 8,     // i64 is 8 bytes
            DataType::String => 24, // String is a pointer + length + capacity (on 64-bit systems)
            DataType::Bool => 1,    // bool is 1 byte
        }
    }
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
pub type TableId = usize;
pub type ColumnId = usize;
pub type IndexId = usize;
pub type ColSetId = usize;

#[derive(Debug, Clone, Eq, PartialOrd, Hash, Ord)]
pub struct ColSet {
    pub table_id: TableId,
    pub column_ids: Set<ColumnId>,
}

impl ColSet {
    pub fn new(table_id: TableId, column_ids: Set<ColumnId>) -> Self {
        ColSet {
            table_id,
            column_ids,
        }
    }

    pub fn combine(left: &ColSet, right: &ColSet) -> Option<ColSet> {
        if left.table_id != right.table_id {
            return None; // Cannot combine columns from different tables
        }
        let combined_cols = Set::union(&left.column_ids, &right.column_ids)
            .cloned()
            .collect();
        Some(ColSet {
            table_id: left.table_id,
            column_ids: combined_cols,
        })
    }
}

impl PartialEq for ColSet {
    fn eq(&self, other: &Self) -> bool {
        self.table_id == other.table_id && self.column_ids == other.column_ids
    }
}

impl Display for ColSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ColSet(TableId: {}, ColumnIds: {:?})",
            self.table_id, self.column_ids
        )
    }
}

impl FromStr for ColSet {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Expected format: "T_<TableId>[<ColId>, <ColId>,...]"
        // Example: "T_1[3,5,7]"

        // Check for "T_" prefix
        if !s.starts_with("T_") {
            return Err(format!(
                "Invalid ColSet format: expected 'T_' prefix, got '{}'",
                s
            ));
        }

        // Find the opening bracket
        let bracket_pos = s
            .find('[')
            .ok_or_else(|| format!("Invalid ColSet format: missing '[' in '{}'", s))?;

        // Parse table_id between "T_" and "["
        let table_id_str = &s[2..bracket_pos];
        let table_id = table_id_str.parse::<TableId>().map_err(|_| {
            format!(
                "Invalid table_id: could not parse '{}' as TableId",
                table_id_str
            )
        })?;

        // Check for closing bracket
        if !s.ends_with(']') {
            return Err(format!(
                "Invalid ColSet format: missing ']' at end of '{}'",
                s
            ));
        }

        // Extract column_ids between "[" and "]"
        let cols_str = &s[bracket_pos + 1..s.len() - 1];

        // Handle empty column list
        if cols_str.is_empty() {
            return Ok(ColSet {
                table_id,
                column_ids: Set::new(),
            });
        }

        // Parse comma-separated column_ids
        let mut column_ids = Set::new();
        for col_str in cols_str.split(',') {
            let col_str = col_str.trim();
            let col_id = col_str.parse::<ColumnId>().map_err(|_| {
                format!(
                    "Invalid column_id: could not parse '{}' as ColumnId",
                    col_str
                )
            })?;
            column_ids.insert(col_id);
        }

        Ok(ColSet {
            table_id,
            column_ids,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colset_from_str_basic() {
        let result = "T_1[3,5,7]".parse::<ColSet>();
        assert!(result.is_ok());
        let colset = result.unwrap();
        assert_eq!(colset.table_id, 1);
        assert_eq!(colset.column_ids, Set::from([3, 5, 7]));
    }

    #[test]
    fn test_colset_from_str_single_column() {
        let result = "T_5[10]".parse::<ColSet>();
        assert!(result.is_ok());
        let colset = result.unwrap();
        assert_eq!(colset.table_id, 5);
        assert_eq!(colset.column_ids, Set::from([10]));
    }

    #[test]
    fn test_colset_from_str_empty_columns() {
        let result = "T_2[]".parse::<ColSet>();
        assert!(result.is_ok());
        let colset = result.unwrap();
        assert_eq!(colset.table_id, 2);
        assert_eq!(colset.column_ids, Set::<ColumnId>::new());
    }

    #[test]
    fn test_colset_from_str_spaces() {
        let result = "T_1[3, 5, 7]".parse::<ColSet>();
        assert!(result.is_ok());
        let colset = result.unwrap();
        assert_eq!(colset.table_id, 1);
        assert_eq!(colset.column_ids, Set::from([3, 5, 7]));
    }

    #[test]
    fn test_colset_from_str_large_ids() {
        let result = "T_100[200,300,400]".parse::<ColSet>();
        assert!(result.is_ok());
        let colset = result.unwrap();
        assert_eq!(colset.table_id, 100);
        assert_eq!(colset.column_ids, Set::from([200, 300, 400]));
    }

    #[test]
    fn test_colset_from_str_missing_prefix() {
        let result = "1[3,5,7]".parse::<ColSet>();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("T_"));
    }

    #[test]
    fn test_colset_from_str_missing_opening_bracket() {
        let result = "T_13,5,7]".parse::<ColSet>();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("["));
    }

    #[test]
    fn test_colset_from_str_missing_closing_bracket() {
        let result = "T_1[3,5,7".parse::<ColSet>();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("]"));
    }

    #[test]
    fn test_colset_from_str_invalid_table_id() {
        let result = "T_abc[3,5,7]".parse::<ColSet>();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("table_id"));
    }

    #[test]
    fn test_colset_from_str_invalid_column_id() {
        let result = "T_1[3,xyz,7]".parse::<ColSet>();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("column_id"));
    }
}
