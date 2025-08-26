use indexmap::IndexMap;
use lattices::Lattice;
use std::{
    fmt::{Debug, Display},
    hash::Hash,
};

/// Type to represent a unique identifier for an entity.
pub type Id = usize;
pub type MulteId = (Id, Id);

/// Type to represent a variable in the language
pub type Var = String;

/// Operator trait for expressions and terms.

/// Alias for Expr/Term types used in the egraph.
///
/// Typically, an AST is an enum or a struct that represents the operations in the expression or term.
/// For example, it could be an enum with variants for different operations like `Add`, `Sub`, etc.

/// ```
/// pub enum MyOp {
///     Add,
///     Sub,
///     Mul,
///     Div,
///     // Terminal values can also be included
///     Const(i32),
///     Var(String),
/// }
/// ```
// FIXME: Come up with a better name for this trait.
pub trait AST: Clone + Debug + PartialEq + Eq + Display + Hash {}

/// Alias for generic analysis type.
pub trait Analysis: Lattice + Clone + Debug + PartialEq + Eq {
    /// Creates a default instance of the analysis type.
    fn default() -> Self;
}

/// Property trait for properties of operators and terms.
pub trait Property: Clone + Debug + PartialEq + Eq + PartialOrd + Display + Hash {
    /// Compares two properties for containment.
    /// NOTE: This is basically >= but for now I like the separate name so we can avoid accidentally using a derived `PartialOrd` trait.
    fn contains(&self, other: &Self) -> bool;

    /// Returns the "no properties" bottom element of the property set
    fn bottom() -> Self;

    /// Returns a vector of `n` bottom elements of the property set.
    fn n_bottoms(n: usize) -> Vec<Self>
    where
        Self: Sized,
    {
        vec![Self::bottom(); n]
    }
}

/// Type alias for a substitution map.
/// Maps variables (of type T) to their corresponding ENode IDs (Id).
/// This is used to represent substitutions in the context of pattern matching and rewriting.
pub type Subst<T, I> = IndexMap<T, I>;

/// Information about an operator in the language.
/// Contains its arity, output properties, and input properties indexed by argument index
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpInfo<P>
where
    P: Property,
{
    /// The arity of the operator.
    pub arity: usize,
    /// Properties of operator output
    pub output_props: P,
    /// Properties of operator input indexed by argument index.
    /// If an argument does not have a property requirement,
    /// its value in the vec should satisfy `P.is_bot()`, i.e., the bottom element of the lattice.
    pub input_props: Vec<P>,
}

impl<P> OpInfo<P>
where
    P: Property,
{
    /// Creates a new OpInfo instance with the given operator, arity, output properties, and input properties.
    pub fn new(arity: usize, output_props: P, input_props: Vec<P>) -> Self {
        Self {
            arity,
            output_props,
            input_props,
        }
    }

    /// Returns a new OpInfo instance with the given arity and all properties set to bottom.
    pub fn default(arity: usize) -> Self {
        Self {
            arity,
            output_props: P::bottom(),
            input_props: P::n_bottoms(arity),
        }
    }

    /// Returns arity
    pub fn arity(&self) -> usize {
        self.arity
    }

    /// Returns output properties
    pub fn output_props(&self) -> &P {
        &self.output_props  
    }

    /// Returns input properties at argument index
    pub fn input_props(&self, index: usize) -> &P {
        self.input_props.get(index).unwrap_or_else(|| {
            panic!(
                "Index {} out of bounds for input_props with length {}",
                index,
                self.input_props.len()
            )
        })
    }
}

impl<P> Default for OpInfo<P>
where
    P: Property,
{
    fn default() -> Self {
        Self {
            arity: 0,
            output_props: P::bottom(),
            input_props: vec![],
        }
    }
}

impl<P> Display for OpInfo<P>
where
    P: Property,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "OpInfo(arity: {}, output_props: {}, input_props: {:?})",
            self.arity, self.output_props, self.input_props
        )
    }
}

/// Trait for languages over operators and properties.
/// The `PropLang` trait defines the interface for mapping from Expressions to operator information.
///
/// When defining a new language:
///     - define a set of operators with the AST trait
///     - define a set of properties with the Property trait (requires implementing PartialOrd and Display)
///     - implement From<T> for P
///     - implement From<Expr<T>> for OpInfo<P>
pub trait PropLang<T, P>
where
    T: AST,
    P: Property + From<T>,
    OpInfo<P>: From<Expr<T>> + From<Pattern<T>>,
{}

/// Generic Recursive Expression structure
/// Typically we use the expression to represent the input expression to the system
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

    /// Returns true if expr is terminal, i.e., has no arguments.
    pub fn is_terminal(&self) -> bool {
        self.args.is_empty()
    }
}

impl<T> Display for Expr<T>
where
    T: AST,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Expr(op: {}, args: {:?})", self.op, self.args)
    }
}

/// Enum to represent either an expression or a variable.
/// This is useful for representing rewrite rules
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OpOrVar<T>
where
    T: AST,
{
    Op(T),
    Var(Var),
}

impl<T> Display for OpOrVar<T>
where
    T: AST,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpOrVar::Op(op) => write!(f, "OpOrVar::Op({})", op),
            OpOrVar::Var(var) => write!(f, "OpOrVar::Var({})", var),
        }
    }
}

impl<T> OpOrVar<T>
where
    T: AST,
{
    pub fn is_expr(&self) -> bool {
        matches!(self, OpOrVar::Op(_))
    }

    pub fn is_var(&self) -> bool {
        matches!(self, OpOrVar::Var(_))
    }
}

impl<T> AST for OpOrVar<T> where
    T: AST,
{}

impl<T> From<Var> for OpOrVar<T>
where
    T: AST,
{
    fn from(var: Var) -> Self {
        OpOrVar::Var(var)
    }
}

pub type Pattern<T> = Expr<OpOrVar<T>>;

/// Generic Term structure
/// Represents a term in a hashconsed structure.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

    /// Returns true if Expr matches the term in operator and arity.
    pub fn matches_expr(&self, expr: &Expr<T>) -> bool {
        self.op() == expr.op() && self.args().len() == expr.args().len()
    }

    /// Returns true if term matches pattern in operator and arity
    /// Note: This does not check properties or arguments.
    pub fn matches_pattern(&self, pattern: &Pattern<T>) -> bool {
        match pattern.op() {
            OpOrVar::Op(t) => self.op() == t && self.args().len() == pattern.args().len(),
            OpOrVar::Var(_) => true, // If the pattern is a variable, it matches any term
        }
    }
}

/// Generic MulteTerm structure
/// Represents a term in a hashconsed structure.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MulteTerm<T, P>
where
    T: AST,
    P: Property,
{
    op: T,
    args: Vec<MulteId>,
    props: P,
}

impl<T, P> MulteTerm<T, P>
where
    T: AST + Into<P>,
    P: Property + From<T>,
    OpInfo<P>: From<Expr<T>> + From<Pattern<T>>,
    Expr<T>: PropLang<T, P>,
{
    /// Creates a new term with the given operator and arguments.
    pub fn new(op: T, args: Vec<MulteId>, props: P) -> Self {
        Self { op, args, props }
    }

    /// Returns the operator of the term.
    pub fn op(&self) -> &T {
        &self.op
    }

    /// Returns the properties of the term.
    pub fn props(&self) -> &P {
        &self.props
    }

    /// Returns the arguments of the term.
    pub fn args(&self) -> &Vec<MulteId> {
        &self.args
    }

    /// Returns true if Expr matches the term in operator and arity.
    /// Note: This does not check properties or arguments.
    pub fn matches_expr(&self, expr: &Expr<T>) -> bool {
        self.op() == expr.op() && self.args().len() == expr.args().len()
    }

    /// Returns true if term matches pattern in operator and arity
    /// Note: This does not check properties or arguments.
    pub fn matches_pattern(&self, pattern: &Pattern<T>) -> bool {
        match pattern.op() {
            OpOrVar::Op(t) => self.op() == t && self.args().len() == pattern.args().len(),
            OpOrVar::Var(_) => true, // If the pattern is a variable, it matches any term
        }
    }

    /// Returns true if the term satisfies a given set of properties
    /// by checking if the term's properties contain the given property.
    pub fn satisfies_property(&self, prop: &P) -> bool {
        self.props.contains(prop)
    }
}

impl<T, P> Into<Term<T>> for MulteTerm<T, P>
where
    T: AST,
    P: Property,
{
    fn into(self) -> Term<T> {
        Term::new(
            self.op,
            self.args.into_iter().map(|(id1, _id2)| id1).collect(),
        )
    }
}
