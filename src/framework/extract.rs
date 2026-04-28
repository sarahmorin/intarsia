//! Extraction strategies for the optimizer framework.
//!
//! The framework's `egraph` field is a regular `egg::EGraph<L, A>`, so any egg-compatible
//! extractor works on it directly. This module adds two things on top:
//!
//! - [`WinnerExtractor`]: the analysis-driven default. Reads the winner's circle that
//!   `OptimizerFramework::run` populated and rebuilds the plan in O(|output expression|),
//!   honoring property requirements at every step.
//! - [`EggExtractor`]: a thin wrapper around `egg::Extractor` so existing egg-style cost
//!   functions can be used through the same [`Extractor<L>`] trait. This makes
//!   intarsia-vs-egg comparisons straightforward — same e-graph, same root, swap the
//!   extractor.
//!
//! Users with their own extraction strategies just implement [`Extractor<L>`].

use egg::{Analysis, CostFunction as EggCostFunction, EGraph, Extractor as EggInner, Id, Language,
          RecExpr};
use std::collections::HashMap;
use std::fmt::Debug;

use crate::framework::analysis::OptimizableAnalysis;
use crate::framework::property::Property;

/// Common interface for extraction strategies.
///
/// Implementors return the lowest-cost expression rooted at a given e-class id, under
/// whatever cost model they encode. Property requirements (when applicable) are
/// configured at construction time on the concrete extractor — the trait stays
/// property-blind so it composes uniformly with property-free strategies.
///
// TODO: principled error handling. `find_best` panics on internal failures (a missing
// winner in `WinnerExtractor`, etc.). For now this matches egg's `Extractor::find_best`
// idiom and keeps the trait one-method. If we want fallible extraction, add a
// `try_find_best -> Result<_, ExtractionError>` method later, with `find_best` defaulting
// to `unwrap`.
pub trait Extractor<L: Language> {
    type Cost: Clone + Debug;

    /// Extract the best expression rooted at `id`.
    fn find_best(&self, id: Id) -> (Self::Cost, RecExpr<L>);
}

// ---------------------------------------------------------------------------
// WinnerExtractor — analysis-driven, linear time
// ---------------------------------------------------------------------------

/// Analysis-driven extractor: walks the winner's circle stored in the e-graph's
/// analysis data. Linear time in the size of the output expression.
///
/// Construction is O(1); the actual walk happens in `find_best` / `find_best_for`.
pub struct WinnerExtractor<'a, L, A>
where
    L: Language,
    A: OptimizableAnalysis<L>,
{
    egraph: &'a EGraph<L, A>,
}

impl<'a, L, A> WinnerExtractor<'a, L, A>
where
    L: Language,
    A: OptimizableAnalysis<L>,
{
    pub fn new(egraph: &'a EGraph<L, A>) -> Self {
        Self { egraph }
    }

    /// Extract the best expression for the bottom (no-requirement) demand. This is the
    /// requirement `OptimizerFramework::run` always populates from a fresh start, so
    /// this is the call you want immediately after `run(root_id)`.
    pub fn find_best(&self, id: Id) -> (A::Cost, RecExpr<L>) {
        self.find_best_for(id, A::Property::bottom())
    }

    /// Extract the best expression for a specific property requirement.
    ///
    /// Panics if no winner has been recorded for `(eclass(id), requirement)` — meaning
    /// the optimizer was never asked to optimize for this requirement at this root.
    pub fn find_best_for(&self, id: Id, requirement: A::Property) -> (A::Cost, RecExpr<L>) {
        let eclass = self.egraph.find(id);
        let data = &self.egraph[eclass].data;

        let winner = A::winner(data, &requirement).unwrap_or_else(|| {
            panic!(
                "WinnerExtractor: no winner recorded for class {:?} with requirement {:?}; \
                 optimizer must populate this pair (call `run` with appropriate top-level demand) \
                 before extraction",
                eclass, requirement
            )
        });

        let cost = winner.cost.clone();
        let root_node_id = winner.node_id;
        let mut expr = RecExpr::default();
        let mut memo: HashMap<(Id, A::Property), Id> = HashMap::new();

        // The root node was already chosen via the winner lookup above. Add it (and its
        // subtree) by descending through children.
        let root_node = self.egraph.get_node(root_node_id);
        self.add_node(root_node, &requirement, &mut expr, &mut memo);

        (cost, expr)
    }

    /// Append `node` (and its subtree) to `expr`, returning the resulting node id in
    /// `expr`. `desired` is the requirement `node` must satisfy as a whole — used to
    /// look up per-child requirements via the analysis.
    fn add_node(
        &self,
        node: &L,
        desired: &A::Property,
        expr: &mut RecExpr<L>,
        memo: &mut HashMap<(Id, A::Property), Id>,
    ) -> Id {
        let children = node.children();
        if children.is_empty() {
            return expr.add(node.clone());
        }

        // Per-child requirements come from the analysis — same query the optimizer used.
        let child_data: Vec<&A::Data> =
            children.iter().map(|&c| &self.egraph[c].data).collect();
        let reqs = self
            .egraph
            .analysis
            .child_requirements(node, desired, &child_data)
            .unwrap_or_else(|| {
                panic!(
                    "WinnerExtractor: child_requirements returned None for node {:?} with \
                     desired {:?}; the optimizer should not have selected a winner that cannot \
                     satisfy its declared requirement",
                    node, desired
                )
            });
        drop(child_data);

        let mut new_children = Vec::with_capacity(children.len());
        for (&child_class, child_req) in children.iter().zip(reqs.into_iter()) {
            let extracted = self.add_class(child_class, child_req, expr, memo);
            new_children.push(extracted);
        }

        let mut new_node = node.clone();
        let mut iter = new_children.into_iter();
        new_node.update_children(|_| iter.next().unwrap());
        expr.add(new_node)
    }

    /// Append the winner of `(class, req)` to `expr`, with memoization across the DAG.
    fn add_class(
        &self,
        class: Id,
        req: A::Property,
        expr: &mut RecExpr<L>,
        memo: &mut HashMap<(Id, A::Property), Id>,
    ) -> Id {
        let canonical = self.egraph.find(class);
        if let Some(&existing) = memo.get(&(canonical, req.clone())) {
            return existing;
        }

        let data = &self.egraph[canonical].data;
        let winner = A::winner(data, &req).unwrap_or_else(|| {
            panic!(
                "WinnerExtractor: no winner for class {:?} with requirement {:?}",
                canonical, req
            )
        });
        let node = self.egraph.get_node(winner.node_id);
        let new_id = self.add_node(node, &req, expr, memo);
        memo.insert((canonical, req), new_id);
        new_id
    }
}

impl<'a, L, A> Extractor<L> for WinnerExtractor<'a, L, A>
where
    L: Language,
    A: OptimizableAnalysis<L>,
{
    type Cost = A::Cost;

    fn find_best(&self, id: Id) -> (A::Cost, RecExpr<L>) {
        WinnerExtractor::find_best(self, id)
    }
}

// ---------------------------------------------------------------------------
// EggExtractor — bridge to egg's CostFunction + Extractor
// ---------------------------------------------------------------------------

/// Wraps `egg::Extractor` so an existing `egg::CostFunction` can be used through the
/// [`Extractor<L>`] trait. Useful for head-to-head comparisons with egg-only code paths.
///
/// You can equivalently call `egg::Extractor::new(&optimizer.egraph, cost_fn)` directly;
/// this wrapper exists only to make egg-style extraction interchangeable with intarsia
/// extractors via the [`Extractor<L>`] trait.
pub struct EggExtractor<'a, L, A, F>
where
    L: Language,
    A: Analysis<L>,
    F: EggCostFunction<L>,
{
    inner: EggInner<'a, F, L, A>,
}

impl<'a, L, A, F> EggExtractor<'a, L, A, F>
where
    L: Language,
    A: Analysis<L>,
    F: EggCostFunction<L>,
{
    pub fn new(egraph: &'a EGraph<L, A>, cost_fn: F) -> Self {
        Self {
            inner: EggInner::new(egraph, cost_fn),
        }
    }

    pub fn find_best(&self, id: Id) -> (F::Cost, RecExpr<L>) {
        self.inner.find_best(id)
    }
}

impl<'a, L, A, F> Extractor<L> for EggExtractor<'a, L, A, F>
where
    L: Language,
    A: Analysis<L>,
    F: EggCostFunction<L>,
    F::Cost: Clone + Debug,
{
    type Cost = F::Cost;

    fn find_best(&self, id: Id) -> (F::Cost, RecExpr<L>) {
        self.inner.find_best(id)
    }
}
