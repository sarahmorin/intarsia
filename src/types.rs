use std::fmt::{Debug, Display};

/// Type to represent a unique identifier for an entity.
pub type Id = usize;

/// Generic Expression structure
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr<T> {
    op: T,
    args: Vec<Expr<T>>,
}

impl<T> Expr<T>
where
    T: Clone + Debug + PartialEq + Eq,
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
    T: Display + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Expr(op: {}, args: {:?})", self.op, self.args)
    }
}

/// Generic Term structure
/// Represents a term in a hashconsed structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Term<T> {
    op: T,
    args: Vec<Id>,
}

impl<T> Term<T>
where
    T: Clone + Debug + PartialEq + Eq,
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
