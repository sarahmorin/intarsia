//! E-class analysis integration for the optimizer framework.
//!
//! This module defines the analysis abstraction the optimizer is built on top of, plus
//! two ready-made `egg::Analysis<L>` implementations:
//!
//! - [`PropertyAnalysis`]: the property-aware "mega-lattice" analysis. Per-class data
//!   is the full [`PropertyData`] — the powerset of properties any node could provide,
//!   a demand-driven winner's circle, per-node costs, and an optional traditional
//!   ("logical") sub-analysis.
//! - [`SimpleCostAnalysis`]: a non-property cost-only analysis. Use this when you want
//!   cascades-style cost-based extraction without a property dimension — including
//!   when comparing intarsia head-to-head against an existing egg analysis.
//!
//! Both implement [`OptimizableAnalysis`], the trait the optimizer drives. Users can
//! also implement [`OptimizableAnalysis`] on their own analysis types if they want a
//! shape neither of the built-ins provide.
//!
//! # A note on egg's analysis propagation
//!
//! `egg::Analysis::make` runs once at enode insertion and `merge` runs once per e-class
//! union. Egg does **not** automatically re-run analysis upward when a child's data
//! improves (e.g., a child class gains a cheaper winner). The cascades scheduler in
//! [`crate::framework::optimizer`] is responsible for that re-evaluation: when an
//! `OptimizeGroup` task runs for a class, the scheduler walks the class's nodes, calls
//! [`OptimizableAnalysis::cost`] using children's *current* winners, and writes the
//! best result back into `egraph[id].data` via [`OptimizableAnalysis::record_winner`].
//! That is why winners and per-node costs are populated by the scheduler rather than
//! by `Analysis::make`.

use egg::{Analysis, DidMerge, EGraph, Id, Language};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use crate::framework::property::{NoProperty, Property};

// ---------------------------------------------------------------------------
// Core data types
// ---------------------------------------------------------------------------

/// A single best-so-far entry in the winner's circle.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Winner<Cost> {
    /// Node id (enode handle) of the best-so-far node satisfying some requirement.
    /// This is the same `Id` type returned by `egraph.nodes_in_class(...)` pairs.
    pub node_id: Id,
    /// Cost of that node when its children supply the requirements that produced it.
    pub cost: Cost,
}

/// Output of a per-child demand decision. Returned by [`PropertyTransfer::child_requirements`]
/// and consumed by the cascades scheduler when scheduling child `OptimizeGroup` tasks.
#[derive(Debug, Clone)]
pub struct Demand<P, Cost> {
    /// Required property for each child, indexed by child position in `node.children()`.
    pub child_requirements: Vec<P>,
    /// Cost of this node yielding the desired output, assuming each child is optimized
    /// at its corresponding requirement. Filled in by [`PropertyTransfer::cost`] in
    /// phase 2; may be `Default` if the caller is only interested in requirements.
    pub cost: Cost,
}

/// Per-class data for [`PropertyAnalysis`].
///
/// Field semantics under `merge`:
/// - `provided`: set union — a class provides a property if any node provides it.
/// - `winners`: pointwise min-by-cost over P keys — the cheapest winner survives.
/// - `node_costs`: disjoint key union — node ids are unique within the egraph; on the
///   unlikely conflict, take the smaller cost.
/// - `logical`: delegated to the inner traditional analysis's `merge`.
#[derive(Debug, Clone)]
pub struct PropertyData<P: Property, Cost, LData> {
    /// Powerset element: every property any node in this class can potentially provide.
    /// Populated by `Analysis::make` (bottom-up via the transfer function).
    ///
    // TODO: revisit with an antichain representation later (only store maximal providers
    // under P's PartialOrd; lookup walks elements ≥ R). The map representation here is
    // O(|P|) per class; antichain saves memory when the lattice is dense.
    pub provided: HashSet<P>,

    /// Demand-driven winner's circle. Only populated for properties some parent has
    /// actually demanded — see goal #6 in the design (keep the circle small).
    pub winners: HashMap<P, Winner<Cost>>,

    /// Per-node cost cache. Filled by `OptimizeGroup` as it costs each node in the
    /// class for some demanded property.
    pub node_costs: HashMap<Id, Cost>,

    /// Traditional egg analysis ("logical analysis") — summarizes the class to a
    /// single value, like constant folding or row-count estimates. Driven by an
    /// inner `LA: Analysis<L>` whose `Data = LData`. Use `()` if not needed.
    pub logical: LData,
}

impl<P: Property, Cost, LData> PropertyData<P, Cost, LData> {
    /// Construct a fresh `PropertyData` for a single new node, given its `provided` set
    /// and logical data. `winners` and `node_costs` start empty — the optimizer fills
    /// them on demand.
    pub fn new(provided: HashSet<P>, logical: LData) -> Self {
        Self {
            provided,
            winners: HashMap::new(),
            node_costs: HashMap::new(),
            logical,
        }
    }

    /// Convenience: read the cost of a child's winner for a given requirement.
    pub fn winner_cost(&self, required: &P) -> Option<&Cost> {
        self.winners.get(required).map(|w| &w.cost)
    }
}

// ---------------------------------------------------------------------------
// OptimizableAnalysis — the trait the optimizer drives
// ---------------------------------------------------------------------------

/// The interface the cascades optimizer uses to read and write its winner-circle state
/// through an `egg::Analysis`.
///
/// The optimizer is generic over `A: OptimizableAnalysis<L>` — it never sees `Property`,
/// `Cost`, or the analysis's `Data` shape directly. Built-in implementations are
/// provided for [`PropertyAnalysis`] (full property-aware) and [`SimpleCostAnalysis`]
/// (cost-only, no properties). Users can implement this on their own analysis types
/// when neither built-in fits.
pub trait OptimizableAnalysis<L: Language>: Analysis<L> + Sized {
    /// The base property lattice. Use [`NoProperty`] when there are no properties.
    type Property: Property;

    /// Totally-ordered cost type. Lower is better.
    ///
    // NOTE on `Default`: we use `Default::default()` as the "worst possible cost"
    // sentinel (e.g., `usize::MAX` for `usize`). This couples cost ordering to
    // `Default` semantics. If we later want a cleaner contract, introduce a
    // `LatticeCost` trait with explicit `worst()` / `best()` methods. (Per user
    // discussion: keep `Default` for now.)
    type Cost: Ord + Clone + Debug + Default;

    // ---- read side (extraction, child-cost lookup) ----

    /// Best node for the given required property in this e-class, if any.
    fn winner<'a>(
        data: &'a Self::Data,
        required: &Self::Property,
    ) -> Option<&'a Winner<Self::Cost>>;

    // ---- write side (driven by the OptimizeGroup task handler) ----

    /// Install a winner for a given requirement. Should overwrite any existing entry —
    /// the caller is responsible for choosing the cheapest before calling.
    fn record_winner(
        data: &mut Self::Data,
        required: Self::Property,
        node_id: Id,
        cost: Self::Cost,
    );

    /// Record the cost of a specific node within this class.
    fn record_node_cost(data: &mut Self::Data, node_id: Id, cost: Self::Cost);

    // ---- demand (drives child OptimizeGroup scheduling and cost) ----

    /// Phase 1: per-child requirements for this node to yield `desired`.
    /// Returns `None` if the node cannot produce `desired` given its children's data.
    /// Called by `OptimizeExpr` to schedule child `OptimizeGroup` tasks.
    fn child_requirements(
        &self,
        node: &L,
        desired: &Self::Property,
        children: &[&Self::Data],
    ) -> Option<Vec<Self::Property>>;

    /// Phase 2: cost of this node yielding `desired`, given children's now-populated
    /// winners for the requirements returned by [`child_requirements`].
    ///
    /// Implementations must agree with `child_requirements` on the inputs read from
    /// each child — the optimizer scheduled children based on those answers.
    fn cost(
        &self,
        node: &L,
        desired: &Self::Property,
        children: &[&Self::Data],
    ) -> Option<Self::Cost>;
}

// ---------------------------------------------------------------------------
// PropertyTransfer — user-defined per-language transfer function
// ---------------------------------------------------------------------------

/// Per-language transfer function that defines property semantics and cost.
///
/// Implemented on a marker type (not the language itself) so that multiple property
/// systems can be defined against one language. Three methods that share the same
/// `match node { ... }` skeleton in practice — they correspond to the cascades phases.
///
/// # Why three methods?
///
/// At enode insertion (`Analysis::make`), children's winners are not yet populated for
/// any specific demand, so cost can't be computed yet — only the bottom-up `provided`
/// set and the logical sub-analysis can. That is [`make_data`].
///
/// During `OptimizeExpr` scheduling, the framework needs per-child requirements before
/// children can be optimized. That is [`child_requirements`]. It must not depend on
/// children's winners being populated.
///
/// During `OptimizeGroup` costing, children's winners *are* populated for the
/// requirements returned by `child_requirements`. That is [`cost`]. It reads
/// `children[i].winners[reqs[i]]` to compute this node's cost.
pub trait PropertyTransfer<L: Language>: Sized + Debug {
    type Property: Property;
    type Cost: Ord + Clone + Debug + Default;
    type LogicalData: Clone + Debug;

    /// Bottom-up at insertion: compute `provided` and `logical` for a freshly-inserted
    /// node. `winners` and `node_costs` are left empty — the cascades scheduler fills
    /// them based on demand.
    ///
    /// Children's `winners` may not be populated; do NOT depend on them here.
    /// Returning a `PropertyData` with `provided.is_empty()` signals "this node cannot
    /// validly produce anything given these children" — the optimizer will skip it.
    fn make_data(
        &self,
        node: &L,
        children: &[&PropertyData<Self::Property, Self::Cost, Self::LogicalData>],
    ) -> PropertyData<Self::Property, Self::Cost, Self::LogicalData>;

    /// Merge two `LogicalData` values (delegate to the inner traditional analysis).
    fn merge_logical(
        &mut self,
        a: &mut Self::LogicalData,
        b: Self::LogicalData,
    ) -> DidMerge;

    /// Phase 1: child requirements for this node to yield `desired`.
    /// Return `None` if this node cannot produce `desired`.
    fn child_requirements(
        &self,
        node: &L,
        desired: &Self::Property,
        children: &[&PropertyData<Self::Property, Self::Cost, Self::LogicalData>],
    ) -> Option<Vec<Self::Property>>;

    /// Phase 2: cost of this node yielding `desired`. Children's winners are populated
    /// for the requirements returned by [`child_requirements`].
    fn cost(
        &self,
        node: &L,
        desired: &Self::Property,
        children: &[&PropertyData<Self::Property, Self::Cost, Self::LogicalData>],
    ) -> Option<Self::Cost>;
}

// ---------------------------------------------------------------------------
// PropertyAnalysis — the built-in property-aware Analysis<L> impl
// ---------------------------------------------------------------------------

/// The built-in property-aware analysis. Wraps a user-defined transfer function and
/// exposes `PropertyData` as its `egg::Analysis::Data`.
#[derive(Debug, Clone)]
pub struct PropertyAnalysis<L, P, Cost, LData, TF>
where
    L: Language,
    P: Property,
    Cost: Ord + Clone + Debug + Default,
    LData: Clone + Debug,
    TF: PropertyTransfer<L, Property = P, Cost = Cost, LogicalData = LData>,
{
    pub transfer: TF,
    _phantom: PhantomData<fn(&L) -> (P, Cost, LData)>,
}

impl<L, P, Cost, LData, TF> PropertyAnalysis<L, P, Cost, LData, TF>
where
    L: Language,
    P: Property,
    Cost: Ord + Clone + Debug + Default,
    LData: Clone + Debug,
    TF: PropertyTransfer<L, Property = P, Cost = Cost, LogicalData = LData>,
{
    pub fn new(transfer: TF) -> Self {
        Self {
            transfer,
            _phantom: PhantomData,
        }
    }
}

impl<L, P, Cost, LData, TF> Analysis<L> for PropertyAnalysis<L, P, Cost, LData, TF>
where
    L: Language,
    P: Property,
    Cost: Ord + Clone + Debug + Default,
    LData: Clone + Debug,
    TF: PropertyTransfer<L, Property = P, Cost = Cost, LogicalData = LData>,
{
    type Data = PropertyData<P, Cost, LData>;

    fn make(egraph: &mut EGraph<L, Self>, enode: &L, _id: Id) -> Self::Data {
        let children: Vec<&Self::Data> = enode
            .children()
            .iter()
            .map(|&c| &egraph[c].data)
            .collect();
        egraph.analysis.transfer.make_data(enode, &children)
    }

    fn merge(&mut self, a: &mut Self::Data, b: Self::Data) -> DidMerge {
        let mut a_changed = false;
        let mut b_changed = false;

        // provided: set union
        for p in b.provided.iter() {
            if a.provided.insert(p.clone()) {
                a_changed = true;
            }
        }
        for p in a.provided.iter() {
            if !b.provided.contains(p) {
                b_changed = true;
                break;
            }
        }

        // winners: pointwise min-by-cost
        for (k, w_b) in b.winners.into_iter() {
            match a.winners.get(&k) {
                Some(w_a) if w_a.cost <= w_b.cost => {
                    if w_a.cost < w_b.cost || w_a.node_id != w_b.node_id {
                        b_changed = true;
                    }
                }
                Some(_) => {
                    a.winners.insert(k, w_b);
                    a_changed = true;
                }
                None => {
                    a.winners.insert(k, w_b);
                    a_changed = true;
                }
            }
        }

        // node_costs: disjoint key union, min on conflict
        for (id, c_b) in b.node_costs.into_iter() {
            match a.node_costs.get(&id) {
                Some(c_a) if *c_a <= c_b => {
                    if *c_a < c_b {
                        b_changed = true;
                    }
                }
                _ => {
                    a.node_costs.insert(id, c_b);
                    a_changed = true;
                }
            }
        }

        // logical: delegate to the user-supplied logical analysis
        let logical_merge = self.transfer.merge_logical(&mut a.logical, b.logical);
        a_changed |= logical_merge.0;
        b_changed |= logical_merge.1;

        DidMerge(a_changed, b_changed)
    }
}

impl<L, P, Cost, LData, TF> OptimizableAnalysis<L> for PropertyAnalysis<L, P, Cost, LData, TF>
where
    L: Language,
    P: Property,
    Cost: Ord + Clone + Debug + Default,
    LData: Clone + Debug,
    TF: PropertyTransfer<L, Property = P, Cost = Cost, LogicalData = LData>,
{
    type Property = P;
    type Cost = Cost;

    fn winner<'a>(data: &'a Self::Data, required: &P) -> Option<&'a Winner<Cost>> {
        data.winners.get(required)
    }

    fn record_winner(data: &mut Self::Data, required: P, node_id: Id, cost: Cost) {
        data.winners.insert(required, Winner { node_id, cost });
    }

    fn record_node_cost(data: &mut Self::Data, node_id: Id, cost: Cost) {
        data.node_costs.insert(node_id, cost);
    }

    fn child_requirements(
        &self,
        node: &L,
        desired: &P,
        children: &[&Self::Data],
    ) -> Option<Vec<P>> {
        self.transfer.child_requirements(node, desired, children)
    }

    fn cost(&self, node: &L, desired: &P, children: &[&Self::Data]) -> Option<Cost> {
        self.transfer.cost(node, desired, children)
    }
}

// ---------------------------------------------------------------------------
// SimpleCostAnalysis — non-property cost-only built-in
// ---------------------------------------------------------------------------

/// Per-class data for [`SimpleCostAnalysis`]. No property dimension — there's a single
/// winner slot per class.
#[derive(Debug, Clone)]
pub struct SimpleCostData<Cost, LData> {
    /// The single best (node, cost) for this class, when the optimizer has costed it.
    pub winner: Option<Winner<Cost>>,
    /// Per-node cost cache (same role as `PropertyData::node_costs`).
    pub node_costs: HashMap<Id, Cost>,
    /// Optional logical sub-analysis value. Use `()` if not needed.
    pub logical: LData,
}

impl<Cost, LData> SimpleCostData<Cost, LData> {
    pub fn new(logical: LData) -> Self {
        Self {
            winner: None,
            node_costs: HashMap::new(),
            logical,
        }
    }
}

/// User-supplied callbacks for [`SimpleCostAnalysis`]. Lighter than [`PropertyTransfer`]
/// because there's no property dimension to track.
pub trait SimpleCostFn<L: Language>: Sized + Debug {
    type Cost: Ord + Clone + Debug + Default;
    type LogicalData: Clone + Debug;

    /// Bottom-up at insertion: compute the logical sub-analysis value.
    fn make_logical(
        &self,
        node: &L,
        children: &[&SimpleCostData<Self::Cost, Self::LogicalData>],
    ) -> Self::LogicalData;

    /// Merge two logical values.
    fn merge_logical(
        &mut self,
        a: &mut Self::LogicalData,
        b: Self::LogicalData,
    ) -> DidMerge;

    /// Cost of this node given children's data (with their winners populated).
    /// Returns `None` if cost cannot be computed.
    fn cost(
        &self,
        node: &L,
        children: &[&SimpleCostData<Self::Cost, Self::LogicalData>],
    ) -> Option<Self::Cost>;
}

/// Cost-only analysis with no property dimension. Implements [`OptimizableAnalysis`]
/// with `Property = NoProperty`. Use this to compare intarsia head-to-head against an
/// existing egg analysis without introducing a property dimension.
#[derive(Debug, Clone)]
pub struct SimpleCostAnalysis<L, Cost, LData, F>
where
    L: Language,
    Cost: Ord + Clone + Debug + Default,
    LData: Clone + Debug,
    F: SimpleCostFn<L, Cost = Cost, LogicalData = LData>,
{
    pub cost_fn: F,
    _phantom: PhantomData<fn(&L) -> (Cost, LData)>,
}

impl<L, Cost, LData, F> SimpleCostAnalysis<L, Cost, LData, F>
where
    L: Language,
    Cost: Ord + Clone + Debug + Default,
    LData: Clone + Debug,
    F: SimpleCostFn<L, Cost = Cost, LogicalData = LData>,
{
    pub fn new(cost_fn: F) -> Self {
        Self {
            cost_fn,
            _phantom: PhantomData,
        }
    }
}

impl<L, Cost, LData, F> Analysis<L> for SimpleCostAnalysis<L, Cost, LData, F>
where
    L: Language,
    Cost: Ord + Clone + Debug + Default,
    LData: Clone + Debug,
    F: SimpleCostFn<L, Cost = Cost, LogicalData = LData>,
{
    type Data = SimpleCostData<Cost, LData>;

    fn make(egraph: &mut EGraph<L, Self>, enode: &L, _id: Id) -> Self::Data {
        let children: Vec<&Self::Data> = enode
            .children()
            .iter()
            .map(|&c| &egraph[c].data)
            .collect();
        let logical = egraph.analysis.cost_fn.make_logical(enode, &children);
        SimpleCostData::new(logical)
    }

    fn merge(&mut self, a: &mut Self::Data, b: Self::Data) -> DidMerge {
        let mut a_changed = false;
        let mut b_changed = false;

        // winner: min-by-cost
        match (a.winner.take(), b.winner) {
            (None, None) => {}
            (Some(wa), None) => a.winner = Some(wa),
            (None, Some(wb)) => {
                a.winner = Some(wb);
                a_changed = true;
            }
            (Some(wa), Some(wb)) => {
                if wb.cost < wa.cost {
                    a.winner = Some(wb);
                    a_changed = true;
                } else if wb.cost > wa.cost {
                    a.winner = Some(wa);
                    b_changed = true;
                } else {
                    a.winner = Some(wa);
                }
            }
        }

        // node_costs: disjoint key union, min on conflict
        for (id, c_b) in b.node_costs.into_iter() {
            match a.node_costs.get(&id) {
                Some(c_a) if *c_a <= c_b => {
                    if *c_a < c_b {
                        b_changed = true;
                    }
                }
                _ => {
                    a.node_costs.insert(id, c_b);
                    a_changed = true;
                }
            }
        }

        let logical_merge = self.cost_fn.merge_logical(&mut a.logical, b.logical);
        a_changed |= logical_merge.0;
        b_changed |= logical_merge.1;

        DidMerge(a_changed, b_changed)
    }
}

impl<L, Cost, LData, F> OptimizableAnalysis<L> for SimpleCostAnalysis<L, Cost, LData, F>
where
    L: Language,
    Cost: Ord + Clone + Debug + Default,
    LData: Clone + Debug,
    F: SimpleCostFn<L, Cost = Cost, LogicalData = LData>,
{
    type Property = NoProperty;
    type Cost = Cost;

    fn winner<'a>(
        data: &'a Self::Data,
        _required: &NoProperty,
    ) -> Option<&'a Winner<Cost>> {
        data.winner.as_ref()
    }

    fn record_winner(data: &mut Self::Data, _required: NoProperty, node_id: Id, cost: Cost) {
        data.winner = Some(Winner { node_id, cost });
    }

    fn record_node_cost(data: &mut Self::Data, node_id: Id, cost: Cost) {
        data.node_costs.insert(node_id, cost);
    }

    fn child_requirements(
        &self,
        node: &L,
        _desired: &NoProperty,
        _children: &[&Self::Data],
    ) -> Option<Vec<NoProperty>> {
        Some(vec![NoProperty; node.children().len()])
    }

    fn cost(
        &self,
        node: &L,
        _desired: &NoProperty,
        children: &[&Self::Data],
    ) -> Option<Cost> {
        self.cost_fn.cost(node, children)
    }
}

// ---------------------------------------------------------------------------
// TODOs
// ---------------------------------------------------------------------------
// TODO: extraction surfacing logical data. The optimizer's `extract_with_cost` could
// also return `&LData` for the root class so consumers don't have to re-read
// `egraph[id].data.logical`. Skipped for now to keep the refactor scope tight; users
// can read it themselves via `&egraph[id].data.logical`.
