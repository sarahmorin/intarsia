use std::fmt::{Debug, Display};
use lattices::Lattice;


/// Type to represent a unique identifier for an entity.
pub type Id = usize;

/// Operator trait for expressions and terms.


/// Alias for Expr/Term types used in the egraph.
// FIXME: Come up with a better name for this trait.
pub trait AST: Clone + Debug + PartialEq + Eq {}
impl<T> AST for T where T: Clone + Debug + PartialEq + Eq {}

/// Alias for generic analysis type.
pub trait Analysis: Lattice + Clone + Debug + PartialEq + Eq {
    /// Creates a default instance of the analysis type.
    fn default() -> Self;
}
impl<T> Analysis for T where T: Lattice + Clone + Debug + PartialEq + Eq {
    // FIXME: deal with this
    fn default() -> Self {
        // Default implementation can be provided here if needed
        unimplemented!()
    }
}

/// Generic Expression structure
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr<T>
where
    T: AST,
{
    op: T,
    args: Vec<Expr<T>>,
}

impl<T> Expr<T>
where
    T: AST,
{
    /// Creates a new expression with the given operator and arguments.
    pub fn new(op: T, args: Vec<Expr<T>>) -> Self {
        Self { op, args }
    }

    /// Returns the operator of the expression.
    pub fn op(&self) -> &T {
        &self.op
    }

    /// Returns the arguments of the expression.
    pub fn args(&self) -> &Vec<Expr<T>> {
        &self.args
    }
}

impl<T> Display for Expr<T>
where
    T: AST + Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Expr(op: {}, args: {:?})", self.op, self.args)
    }
}

/// Generic Term structure
/// Represents a term in a hashconsed structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Term<T>
where
    T: AST,
{
    op: T,
    args: Vec<Id>,
}

impl<T> Term<T>
where
    T: AST,
{
    /// Creates a new term with the given operator and arguments.
    pub fn new(op: T, args: Vec<Id>) -> Self {
        Self { op, args }
    }

    /// Returns the operator of the term.
    pub fn op(&self) -> &T {
        &self.op
    }

    /// Returns the arguments of the term.
    pub fn args(&self) -> &Vec<Id> {
        &self.args
    }
}
