use crate::types::*;
/// A stand in language for testing purposes.
use std::fmt::{Debug, Display};
use std::hash::Hash;
use bitmaps::Bitmap;

/// Represents the test language.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Ops {
    // Constants
    Col(String),
    Table(String),
    ConstStr(String),
    ConstInt(i32),
    ConstBool(bool),
    // Comparison Operators
    Eq,
    Neq,
    Lt,
    Gt,
    Le,
    Ge,
    // Bool Operators
    And,
    Or,
    Not,
    // Logical Query Ops
    Get,
    Filter,
    Join,
    Project,
    // Physical Query Ops
    Scan,
    IndexScan,
    Sort,
    NLJoin,
    HashJoin,
}

impl Display for Ops {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ops::Col(s) => write!(f, "Col({})", s),
            Ops::Table(s) => write!(f, "Table({})", s),
            Ops::ConstStr(s) => write!(f, "ConstStr({})", s),
            Ops::ConstInt(i) => write!(f, "ConstInt({})", i),
            Ops::ConstBool(b) => write!(f, "ConstBool({})", b),
            Ops::Eq => write!(f, "=="),
            Ops::Neq => write!(f, "!="),
            Ops::Lt => write!(f, "<"),
            Ops::Gt => write!(f, ">"),
            Ops::Le => write!(f, "<="),
            Ops::Ge => write!(f, ">="),
            Ops::And => write!(f, "And"),
            Ops::Or => write!(f, "Or"),
            Ops::Not => write!(f, "Not"),
            Ops::Get => write!(f, "Get"),
            Ops::Filter => write!(f, "Filter"),
            Ops::Join => write!(f, "Join"),
            Ops::Project => write!(f, "Project"),
            Ops::Scan => write!(f, "Scan"),
            Ops::IndexScan => write!(f, "IndexScan"),
            Ops::Sort => write!(f, "Sort"),
            Ops::NLJoin => write!(f, "NLJoin"),
            Ops::HashJoin => write!(f, "HashJoin"),
        }
    }
}

impl AST for Ops {}

/// Represents a physical property set for query optimization.
/// This is a simple example with a single property: sorted stored in a bitmap.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PhysicalPropertySet {
    sort: Bitmap<3>,
}

impl PhysicalPropertySet {
    /// Creates a physical property set from a sort index
    // FIXME: this is dummy test implementation
    // In a real implementation, we would call out to a larger data struct to get the real column sort bitmaps
    pub fn from_cols(cols: String) -> Self {
        let mut sort = Bitmap::new();
        match cols.as_str() {
            "x" => {
                sort.set(0, true); // Assume column "x" is sorted
            },
            "y" => {
                sort.set(1, true); // Assume column "y" is sorted
            },
            "z" => {
                sort.set(2, true); // Assume column "z" is sorted
            },
            "xy" => {
                sort.set(0, true); // Assume both "x" and "y" are sorted
                sort.set(1, true);
            },
            "yz" => {
                sort.set(1, true); // Assume both "y" and "z" are sorted
                sort.set(2, true);
            },
            "xz" => {
                sort.set(0, true); // Assume both "x" and "z" are sorted
                sort.set(2, true);
            },
            "xyz" => {
                sort.set(0, true); // Assume all "x", "y", and "z" are sorted
                sort.set(1, true);
                sort.set(2, true);
            },
            _ => {}
        }
        PhysicalPropertySet { sort }
    }
}

impl Display for PhysicalPropertySet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PhysicalPropertySet(sort: {:?})", self.sort)
    }
}

impl Property for PhysicalPropertySet {
    fn contains(&self, other: &Self) -> bool {
        self.sort & other.sort == other.sort
    }

    fn bottom() -> Self {
        Self {
            sort: Bitmap::new()
        }
    }
}

impl PartialOrd for PhysicalPropertySet {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.contains(other) {
            Some(std::cmp::Ordering::Greater)
        } else if other.contains(self) {
            Some(std::cmp::Ordering::Less)
        } else {
            Some(std::cmp::Ordering::Equal)
        }
    }
}

impl From<Ops> for PhysicalPropertySet {
    fn from(op: Ops) -> Self {
        match op {
            Ops::Col(s) => PhysicalPropertySet::from_cols(s),
            _ => PhysicalPropertySet::bottom(),
        }
    }
}

impl From<Expr<Ops>> for OpInfo<PhysicalPropertySet> {
    fn from(expr: Expr<Ops>) -> Self {
        match expr.op() {
            Ops::IndexScan => {
                let c = expr.args().get(1).unwrap_or_else(|| panic!("IndexScan requires at least 2 arguments"));
                let col_prop = PhysicalPropertySet::from(c.op().clone());
                OpInfo::new(2, col_prop.clone(), vec![PhysicalPropertySet::bottom(), col_prop])
            },
            _ => OpInfo::default(expr.args().len()),
        }
    }
}

impl From<Pattern<Ops>> for OpInfo<PhysicalPropertySet> {
    fn from(pattern: Pattern<Ops>) -> Self {
        match pattern.op() {
            OpOrVar::Op(Ops::IndexScan) => {
                let c = match pattern.args().get(1).unwrap_or_else(|| panic!("IndexScan requires at least 2 arguments")).op() {
                    OpOrVar::Op(op) => op,
                    OpOrVar::Var(v) => panic!("Expected concrete operator, found variable: {}", v),
                };
                let col_prop = PhysicalPropertySet::from(c.clone());
                OpInfo::new(2, col_prop.clone(), vec![PhysicalPropertySet::bottom(), col_prop])
            },
            _ => OpInfo::default(pattern.args().len()),
        }
    }
}


impl PropLang<Ops, PhysicalPropertySet> for Expr<Ops> {}