/// Domain-specific types for the database optimizer example.
///
/// These types are specific to database query optimization and would not be
/// relevant for other optimizer use cases.
use std::collections::BTreeSet as Set;
use std::{fmt::Display, str::FromStr};

/// Supported Datatypes in a database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataType {
    Int,    // i64
    String, // String
    Bool,   // bool
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

/// A set of columns from a table, used for projections and predicates.
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
