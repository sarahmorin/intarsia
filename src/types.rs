use indexmap::IndexMap;
use std::{
    fmt::{Debug, Display},
    hash::Hash,
};

/// Type to represent EClass and ENode identifiers.
pub type Id = usize;

/// Type to represent a variable in the language.
pub type Var = String;

/// Type alias for a substitution map.
/// This is used to represent substitutions in the context of pattern matching and rewriting.
pub type Subst<Var, Id> = IndexMap<Var, Id>;

/// Trait to define the language of operators for expressions and terms.
///
/// Typically, an OpLang is an enum or a struct that represents the operations in the expression or term.
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
///
/// Using a trait here allows for more complex language definitions that may include additional domain specific features.
pub trait OpLang: Clone + Debug + PartialEq + Eq + Display + Hash {
    type Operator;
    /// Returns the operator of the OpLang node.
    fn op(&self) -> &Self::Operator;

    /// Returns arity of the operator.
    // QUESTION: What if people want variable length input?
    fn arity(&self) -> usize;

    /// Returns true if this operator is "extractable", i.e. if it can be extracted from its context.
    fn is_extractable(&self) -> bool;
}

/// Macro to implement the default OpLang trait for simple languages.
/// Note: This is largely for testing utility.
// TODO: Move to a test util library instead of here
#[macro_export]
macro_rules! impl_oplang_default {
    () => {
        type Operator = Self;

        fn op(&self) -> &Self::Operator {
            self
        }

        fn is_extractable(&self) -> bool {
            true
        }
    };
}

/// Generic Recursive Expression structure.
/// An expression is fully defined, i.e. it does not contain any unbound variables.
/// Since an expression is fully defined, we can evaluate its properties.
/// If working in a language without properties, we simply set propset to 0.
/// Typically we use the expression to represent the input expression to the system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr<L>
where
    L: OpLang,
{
    /// Operator of the expression from an `OpLang`
    op: L,
    /// Arguments as a vector of sub-expressions
    args: Vec<Expr<L>>,
    // PropertySetId of the expression
    // propset: Option<PropSetId>,
}

impl<L> Expr<L>
where
    L: OpLang,
{
    /// Creates a new expression with the given operator and arguments.
    pub fn new(op: L, args: Vec<Expr<L>>) -> Self {
        Self {
            op,
            // propset: None,
            args,
        }
    }

    /// Sets the PropertySetId of the expression.
    // pub fn set_propset(&mut self, propset: PropSetId) {
    //     self.propset = Some(propset);
    // }

    /// Returns the operator of the expression.
    pub fn op(&self) -> &L {
        &self.op
    }

    /// Returns the arguments of the expression.
    pub fn args(&self) -> &Vec<Expr<L>> {
        &self.args
    }

    /// Returns PropertySet Id of the expression.
    // pub fn propset(&self) -> &Option<PropSetId> {
    //     &self.propset
    // }

    /// Returns arity of the expression.
    pub fn arity(&self) -> usize {
        self.op.arity()
    }
    /// Returns true if expr is terminal, i.e., has no arguments.
    pub fn is_terminal(&self) -> bool {
        self.args.is_empty()
    }
}

impl<L> Display for Expr<L>
where
    L: OpLang,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Expr(op: {}, args: {:?}",
            self.op,
            self.args,
            // self.propset()
        )
    }
}

/// Enum to represent either an expression or a variable.
/// This is useful for representing patterns in rewrite rules.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OpOrVar<L>
where
    L: OpLang,
{
    Op(L),
    Var(Var),
}

/// Pattern structure for matching expressions and variables.
/// A pattern is an expression in language L that can contain unknown variables.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pattern<L>
where
    L: OpLang,
{
    op: OpOrVar<L>,
    args: Vec<Pattern<L>>,
}

impl<L> Pattern<L>
where
    L: OpLang,
{
    /// Creates a new pattern with the given operator or variable and arguments.
    pub fn new(op: OpOrVar<L>, args: Vec<Pattern<L>>) -> Self {
        Self { op, args }
    }

    /// Returns the operator or variable of the pattern.
    pub fn op(&self) -> &OpOrVar<L> {
        &self.op
    }

    /// Returns the arguments of the pattern.
    pub fn args(&self) -> &Vec<Pattern<L>> {
        &self.args
    }

    /// Returns arity of the pattern.
    pub fn arity(&self) -> usize {
        match self.op {
            OpOrVar::Var(_) => 0,
            OpOrVar::Op(ref op) => op.arity(),
        }
    }

    /// Returns true if pattern is terminal, i.e., has no arguments.
    pub fn is_terminal(&self) -> bool {
        self.args.is_empty()
    }

    /// Returns true if pattern in a variable.
    pub fn is_var(&self) -> bool {
        matches!(self.op(), OpOrVar::Var(_))
    }
}

/// Create a Pattern from a variable String.
/// ```
/// // Create Pattern {op: OpOrVar::Var("?x"), args: []}
/// let pattern_var = Pattern::from("?x");
/// ```
impl<L> From<Var> for Pattern<L>
where
    L: OpLang,
{
    fn from(var: Var) -> Self {
        Self {
            op: OpOrVar::Var(var),
            args: Vec::new(),
        }
    }
}

/// Generic Term structure.
/// Represents a term in a hashconsed structure. Rather than storing arguments directly,
/// it stores identifiers which might point to a set of equivalent argument nodes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Term<L>
where
    L: OpLang,
{
    op: L,
    args: Vec<Id>,
}

impl<L> Term<L>
where
    L: OpLang,
{
    /// Creates a new term with the given operator and arguments.
    pub fn new(op: L, args: Vec<Id>) -> Self {
        Self { op, args }
    }

    /// Returns the operator of the term.
    pub fn op(&self) -> &L {
        &self.op
    }

    /// Returns the arguments of the term.
    pub fn args(&self) -> &Vec<Id> {
        &self.args
    }

    /// Returns arity of the term.
    pub fn arity(&self) -> usize {
        self.op.arity()
    }

    /// Returns true if Expr matches the term in operator and arity.
    pub fn matches_expr(&self, expr: &Expr<L>) -> bool {
        self.op() == expr.op() && self.args().len() == expr.args().len()
    }

    /// Returns true if term matches pattern in operator and arity.
    /// Note: This does not check properties or arguments.
    pub fn matches_pattern(&self, pattern: &Pattern<L>) -> bool {
        match pattern.op() {
            OpOrVar::Op(t) => self.op() == t && self.arity() == pattern.arity(),
            OpOrVar::Var(_) => true, // If the pattern is a variable, it matches any term
        }
    }
}

/// Cost functions are used to guide extraction from the e-graph.
pub trait CostFunction<L, D>
where
    L: OpLang,
    D: PartialOrd,
{
    fn cost(&self, expr: &Expr<L>) -> D;
}
