use indexmap::IndexMap;
use std::{
    fmt::{Debug, Display},
    hash::Hash,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PropSetId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MulteId(pub Id, pub PropSetId);

impl MulteId {
    /// Returns the logical Id.
    pub fn logical_id(&self) -> Id {
        self.0
    }

    /// Returns the property set Id.
    pub fn propset_id(&self) -> PropSetId {
        self.1
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Id({})", self.0)
    }
}

impl Display for PropSetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PropSetId({})", self.0)
    }
}

impl Display for MulteId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MulteId({}, {})", self.0, self.1)
    }
}

impl Id {
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

impl PropSetId {
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

// Additional From implementations for convenience
impl From<usize> for PropSetId {
    fn from(id: usize) -> Self {
        PropSetId(id)
    }
}

impl From<MulteId> for Id {
    fn from(multe_id: MulteId) -> Self {
        multe_id.0
    }
}

impl From<usize> for Id {
    fn from(id: usize) -> Self {
        Id(id)
    }
}

impl From<Id> for MulteId {
    fn from(id: Id) -> Self {
        MulteId(id, PropSetId(0)) // Default propset id to 0
    }
}

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

/// Property trait for properties of operators and terms.
/// A PropertySet defines the properties that can be associated with an expression in the language.
/// These properties can be used to enforce constraints on the expressions, such as requiring certain types of input.
/// A property set could be a simple bool flag, a bitmap, or a more complex structure.
/// It need only implement the `PartialOrd` trait and provide a bottom element.
/// Note: Implement `PartialOrd` for your property set. Deriving the trait is likely not the behavior you need here.
pub trait PropertySet: Clone + Debug + PartialEq + Eq + PartialOrd + Display + Hash {
    /// Returns the "no properties" bottom element of the property set.
    fn bottom() -> Self;

    /// Returns a vector of `n` bottom elements of the property set.
    fn n_bottoms(n: usize) -> Vec<Self>
    where
        Self: Sized,
    {
        vec![Self::bottom(); n]
    }
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
    /// PropertySetId of the expression
    propset: Option<PropSetId>,
}

impl<L> Expr<L>
where
    L: OpLang,
{
    /// Creates a new expression with the given operator and arguments.
    pub fn new(op: L, args: Vec<Expr<L>>) -> Self {
        Self {
            op,
            propset: None,
            args,
        }
    }

    /// Sets the PropertySetId of the expression.
    pub fn set_propset(&mut self, propset: PropSetId) {
        self.propset = Some(propset);
    }

    /// Returns the operator of the expression.
    pub fn op(&self) -> &L {
        &self.op
    }

    /// Returns the arguments of the expression.
    pub fn args(&self) -> &Vec<Expr<L>> {
        &self.args
    }

    /// Returns PropertySet Id of the expression.
    pub fn propset(&self) -> &Option<PropSetId> {
        &self.propset
    }

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
            "Expr(op: {}, propset: {:?}, args: {:?})",
            self.op, self.propset, self.args
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
    args: Vec<MulteId>,
}

impl<L> Term<L>
where
    L: OpLang,
{
    /// Creates a new term with the given operator and arguments.
    pub fn new(op: L, args: Vec<MulteId>) -> Self {
        Self { op, args }
    }

    /// Returns the operator of the term.
    pub fn op(&self) -> &L {
        &self.op
    }

    /// Returns the arguments of the term.
    pub fn args(&self) -> &Vec<MulteId> {
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

    /// Returns true if the term satisfies a property set.
    /// Note: This does not check properties or arguments.
    // FIXME: this needs a real solution
    pub fn satisfies_property<P>(&self, _props: &P) -> bool
    where
        P: PropertySet,
    {
        true
    }
}

/// The PropInfo struct defines a relationship between an OpLang and a PropertySet.
/// It contains functions that map expressions and patterns to property sets.
/// This allows us to derive the properties of an expression based on its structure and the properties of its arguments.
/// This is crucial for maintaining and enforcing property-based equivalence relations in the multe-graph.
///
/// It is important to note a few things. The functions...
/// - must be deterministic. Given the same input, they must always produce the same output.
/// - should respect the coarsest equivalence relation. If two expressions are logically equivalent, they should map to the same property set.
/// - should be efficient to compute, as they will be called frequently during e-graph operations.
/// - should be well-defined for all valid expressions and patterns in the language.
/// - should NOT be recursive or depend on one another. Each function should be self-contained and operate solely on its input.
// QUESTION: How do I enforce some/all of these ^?? Is there a Rust-y way to do it? Is just hopes and dreams?
pub struct PropInfo<L, P>
where
    L: OpLang,
    P: PropertySet,
{
    /// Function to map from an operator to indices of its property-deriving arguments.
    pub op_prop_args: fn(&L) -> Vec<usize>,
    /// Function to map from an operator to the indices of its property-dependent arguments.
    pub op_dep_args: fn(&L) -> Vec<usize>,
    // QUESTION: Do we need both of these?
    // TODO: At some point, do a check that prop args and dep args are disjoint, within arity bounds, and cover all arguments
    /// Function to get the property set of the expression.
    pub output_props: fn(&Expr<L>) -> P,
    /// Function to get the property srt requirements on argument at index idx.
    pub arg_req_props: fn(&Expr<L>, usize) -> P,
    // QUESTION: Should these functions taken an operator and a vec of property-deriving expressions instead?
    //   - This would enforce that the functions only depend on the operator and the relevant arguments
    //   - However, it would also require us to extract the relevant arguments before calling the function
}

impl<L, P> PropInfo<L, P>
where
    L: OpLang,
    P: PropertySet,
{
    /// Creates a new PropInfo instance with the given functions.
    pub fn new(
        op_prop_args: fn(&L) -> Vec<usize>,
        op_dep_args: fn(&L) -> Vec<usize>,
        output_props: fn(&Expr<L>) -> P,
        arg_req_props: fn(&Expr<L>, usize) -> P,
    ) -> Self {
        Self {
            op_prop_args,
            op_dep_args,
            output_props,
            arg_req_props,
        }
    }

    pub fn default() -> Self {
        Self {
            op_prop_args: |_| vec![],
            op_dep_args: |_| vec![],
            output_props: |_| P::bottom(),
            arg_req_props: |_, _| P::bottom(),
        }
    }

    pub fn op_prop_args(&self, op: &L) -> Vec<usize> {
        (self.op_prop_args)(op)
    }

    pub fn op_dep_args(&self, op: &L) -> Vec<usize> {
        (self.op_dep_args)(op)
    }

    pub fn output_props(&self, expr: &Expr<L>) -> P {
        (self.output_props)(expr)
    }

    pub fn arg_req_props(&self, expr: &Expr<L>, idx: usize) -> P {
        (self.arg_req_props)(expr, idx)
    }
}

/// Cost functions are used to guide extraction from the e-graph.
pub trait CostFunction<L>
where
    L: OpLang,
{
    // TODO: Define the cost function trait
    // QUESTION: Should the cost function depend on properties as well?
}

// =============== Here be monsters ================
// /// Information about an operator in the language.
// /// Contains its arity, output properties, and input properties indexed by argument index
// #[derive(Debug, Clone, PartialEq, Eq)]
// pub struct PropInfo<P>
// where
//     P: PropertySet,
// {
//     /// The arity of the operator.
//     pub arity: usize,
//     /// Properties of operator output
//     pub output_props: P,
//     /// Properties of operator input indexed by argument index.
//     /// If an argument does not have a property requirement,
//     /// its value in the vec should satisfy `P.is_bot()`, i.e., the bottom element of the lattice.
//     pub input_props: Vec<P>,
// }

// impl<P> PropInfo<P>
// where
//     P: PropertySet,
// {
//     /// Creates a new PropInfo instance with the given operator, arity, output properties, and input properties.
//     pub fn new(arity: usize, output_props: P, input_props: Vec<P>) -> Self {
//         Self {
//             arity,
//             output_props,
//             input_props,
//         }
//     }

//     /// Returns a new PropInfo instance with the given arity and all properties set to bottom.
//     pub fn default(arity: usize) -> Self {
//         Self {
//             arity,
//             output_props: P::bottom(),
//             input_props: P::n_bottoms(arity),
//         }
//     }

//     /// Returns arity
//     pub fn arity(&self) -> usize {
//         self.arity
//     }

//     /// Returns output properties
//     pub fn output_props(&self) -> &P {
//         &self.output_props
//     }

//     /// Returns input properties at argument index
//     pub fn input_props(&self, index: usize) -> &P {
//         self.input_props.get(index).unwrap_or_else(|| {
//             panic!(
//                 "Index {} out of bounds for input_props with length {}",
//                 index,
//                 self.input_props.len()
//             )
//         })
//     }
// }

// impl<P> Default for PropInfo<P>
// where
//     P: PropertySet,
// {
//     fn default() -> Self {
//         Self {
//             arity: 0,
//             output_props: P::bottom(),
//             input_props: vec![],
//         }
//     }
// }

// impl<P> Display for PropInfo<P>
// where
//     P: PropertySet,
// {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "PropInfo(arity: {}, output_props: {}, input_props: {:?})",
//             self.arity, self.output_props, self.input_props
//         )
//     }
// }
