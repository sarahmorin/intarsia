//! Property transfer function for database query optimization.
//!
//! In the analysis-driven framework, what was previously split between
//! `PropertyAwareLanguage::property_req` and a `CostFunction::compute_cost` impl is
//! unified into one [`PropertyTransfer`] impl. The class-invariant statistics that
//! used to live inside `DbCost` (cardinality and block counts) now live in the
//! logical sub-analysis [`DbStats`] — they're properties of the relation, not of any
//! particular physical plan.

use std::cmp::max;
use std::collections::HashSet;

use egg::DidMerge;
use intarsia::framework::{Property, PropertyData, PropertyTransfer};

use super::catalog::Catalog;
use super::language::Optlang;
use super::property::SimpleProperty;

// Cost constants
pub const CPU_COST: usize = 1;
pub const IO_COST: usize = 1000;
pub const TRANSFER_COST: usize = 10;
pub const SELECTIVITY_FACTOR: f64 = 0.5;

/// Logical sub-analysis: row-count and block-count estimates for the relation an
/// e-class represents. Class-invariant — all logically-equivalent expressions in a
/// class describe the same set of rows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DbStats {
    pub cardinality: Option<usize>,
    pub blocks: Option<usize>,
}

/// Marker type carrying the catalog reference used by stat/cost computations.
#[derive(Debug, Clone)]
pub struct DbTransfer {
    pub catalog: Catalog,
}

impl DbTransfer {
    pub fn new(catalog: Catalog) -> Self {
        Self { catalog }
    }
}

type Data = PropertyData<SimpleProperty, usize, DbStats>;

/// Helper: read the cost of a child's winner for `req`. Returns `None` when no winner
/// is recorded for that requirement on that child.
///
/// CRITICAL: this must NOT default to 0. Self-referential nodes (e.g., x * 1 in x's
/// own class after the identity rewrite) would otherwise look free during phase-3
/// cost evaluation: the parent class's own winner is mid-computation, so reading
/// "class C's winner for R" while we're computing it returns missing — defaulting
/// to 0 lets cyclic plans win and produces non-extractable winners. Returning None
/// makes the calling node infeasible, which is correct.
fn child_cost(child: &Data, req: &SimpleProperty) -> Option<usize> {
    child.winners.get(req).map(|w| w.cost)
}

/// Helper: pass-through stat propagation (Select/Project keep source's stats with
/// selectivity scaling).
fn scale_stats(stats: &DbStats, factor: f64) -> DbStats {
    DbStats {
        cardinality: stats.cardinality.map(|c| (c as f64 * factor) as usize),
        blocks: stats.blocks.map(|b| (b as f64 * factor) as usize),
    }
}

/// Helper: combine child stats for joins (cardinality/block estimates use max).
fn join_stats(left: &DbStats, right: &DbStats) -> DbStats {
    DbStats {
        cardinality: Some(max(
            left.cardinality.unwrap_or(0),
            right.cardinality.unwrap_or(0),
        )),
        blocks: Some(max(left.blocks.unwrap_or(0), right.blocks.unwrap_or(0))),
    }
}

impl PropertyTransfer<Optlang> for DbTransfer {
    type Property = SimpleProperty;
    type Cost = usize;
    type LogicalData = DbStats;

    fn make_data(&self, node: &Optlang, children: &[&Data]) -> Data {
        // Provided property set: which sortedness can this node produce given children.
        let mut provided: HashSet<SimpleProperty> = HashSet::new();
        match node {
            Optlang::Int(_) | Optlang::Bool(_) | Optlang::Str(_) | Optlang::ColSet(_) => {
                provided.insert(SimpleProperty::Bottom);
            }

            Optlang::Add(_)
            | Optlang::Sub(_)
            | Optlang::Mul(_)
            | Optlang::Div(_)
            | Optlang::Eq(_)
            | Optlang::Lt(_)
            | Optlang::Gt(_)
            | Optlang::Le(_)
            | Optlang::Ge(_)
            | Optlang::Ne(_)
            | Optlang::And(_)
            | Optlang::Or(_)
            | Optlang::Not(_) => {
                provided.insert(SimpleProperty::Bottom);
            }

            Optlang::Table(_) | Optlang::TableScan(_) => {
                provided.insert(SimpleProperty::Unsorted);
            }
            Optlang::Index(_) | Optlang::IndexScan(_) | Optlang::Sort(_) => {
                provided.insert(SimpleProperty::Sorted);
            }

            Optlang::HashJoin(_) | Optlang::NestedLoopJoin(_) => {
                provided.insert(SimpleProperty::Unsorted);
            }
            Optlang::MergeJoin(_) => {
                // Only valid if both inputs can supply Sorted.
                if children.len() >= 2
                    && children[0].provided.contains(&SimpleProperty::Sorted)
                    && children[1].provided.contains(&SimpleProperty::Sorted)
                {
                    provided.insert(SimpleProperty::Sorted);
                }
            }

            Optlang::Select([_, _]) => {
                // children[0] is source.
                if let Some(src) = children.first() {
                    provided.extend(src.provided.iter().copied());
                }
            }
            Optlang::Project([_, _]) => {
                // children[1] is source.
                if children.len() >= 2 {
                    provided.extend(children[1].provided.iter().copied());
                }
            }

            // Logical poison — these have no valid physical realization. Empty `provided`
            // means the optimizer will reject any node of this kind during winner pick.
            Optlang::Join(_) | Optlang::Scan(_) => {}
        }

        // Logical sub-analysis: row/block estimates for the relation.
        let logical = compute_stats(node, children, &self.catalog);

        PropertyData::new(provided, logical)
    }

    fn merge_logical(&mut self, a: &mut DbStats, b: DbStats) -> DidMerge {
        // Equivalent classes describe the same relation, so stats *should* agree. If
        // they disagree (different paths produced different estimates), be conservative:
        // take the smaller cardinality and smaller blocks.
        let mut a_changed = false;
        let mut b_changed = false;
        match (a.cardinality, b.cardinality) {
            (None, Some(_)) => {
                a.cardinality = b.cardinality;
                a_changed = true;
            }
            (Some(_), None) => b_changed = true,
            (Some(av), Some(bv)) if bv < av => {
                a.cardinality = Some(bv);
                a_changed = true;
            }
            (Some(av), Some(bv)) if bv > av => b_changed = true,
            _ => {}
        }
        match (a.blocks, b.blocks) {
            (None, Some(_)) => {
                a.blocks = b.blocks;
                a_changed = true;
            }
            (Some(_), None) => b_changed = true,
            (Some(av), Some(bv)) if bv < av => {
                a.blocks = Some(bv);
                a_changed = true;
            }
            (Some(av), Some(bv)) if bv > av => b_changed = true,
            _ => {}
        }
        DidMerge(a_changed, b_changed)
    }

    fn child_requirements(
        &self,
        node: &Optlang,
        desired: &SimpleProperty,
        children: &[&Data],
    ) -> Option<Vec<SimpleProperty>> {
        use SimpleProperty::*;
        match node {
            Optlang::Int(_) | Optlang::Bool(_) | Optlang::Str(_) | Optlang::ColSet(_) => {
                Bottom.satisfies(desired).then(Vec::new)
            }
            Optlang::Table(_) | Optlang::Index(_) => {
                let provides = if matches!(node, Optlang::Index(_)) { Sorted } else { Unsorted };
                provides.satisfies(desired).then(Vec::new)
            }

            Optlang::TableScan(_) => Unsorted.satisfies(desired).then(|| vec![Bottom]),
            Optlang::IndexScan(_) => Sorted.satisfies(desired).then(|| vec![Bottom]),

            // Sort: prefer demanding Sorted from source if source can supply it (free path).
            // Otherwise demand Bottom and pay the sort cost. cost() must mirror this choice.
            Optlang::Sort([_, _]) => {
                if !Sorted.satisfies(desired) {
                    return None;
                }
                let src_req = if children
                    .first()
                    .map(|c| c.provided.contains(&Sorted))
                    .unwrap_or(false)
                {
                    Sorted
                } else {
                    Bottom
                };
                Some(vec![src_req, Bottom])
            }

            Optlang::Select([_, _]) => {
                // Pass through source's property; pred gets Bottom.
                let src_provides_desired = children
                    .first()
                    .map(|c| c.provided.iter().any(|p| p.satisfies(desired)))
                    .unwrap_or(false);
                if !src_provides_desired {
                    return None;
                }
                Some(vec![desired.clone(), Bottom])
            }
            Optlang::Project([_, _]) => {
                if children.len() < 2 {
                    return None;
                }
                let src_provides_desired = children[1].provided.iter().any(|p| p.satisfies(desired));
                if !src_provides_desired {
                    return None;
                }
                // children: [cols, source]; cols gets Bottom, source gets `desired`.
                Some(vec![Bottom, desired.clone()])
            }

            Optlang::HashJoin(_) | Optlang::NestedLoopJoin(_) => {
                Unsorted.satisfies(desired).then(|| vec![Bottom, Bottom, Bottom])
            }
            Optlang::MergeJoin(_) => {
                Sorted.satisfies(desired).then(|| vec![Sorted, Sorted, Bottom])
            }

            Optlang::Add(_) | Optlang::Sub(_) | Optlang::Mul(_) | Optlang::Div(_)
            | Optlang::Eq(_) | Optlang::Lt(_) | Optlang::Gt(_) | Optlang::Le(_)
            | Optlang::Ge(_) | Optlang::Ne(_) | Optlang::And(_) | Optlang::Or(_) => {
                Bottom.satisfies(desired).then(|| vec![Bottom, Bottom])
            }
            Optlang::Not(_) => Bottom.satisfies(desired).then(|| vec![Bottom]),

            Optlang::Join(_) | Optlang::Scan(_) => None,
        }
    }

    fn cost(
        &self,
        node: &Optlang,
        desired: &SimpleProperty,
        children: &[&Data],
    ) -> Option<usize> {
        use SimpleProperty::*;
        match node {
            Optlang::Int(_) | Optlang::Bool(_) | Optlang::Str(_) | Optlang::ColSet(_) => {
                Bottom.satisfies(desired).then_some(0)
            }
            Optlang::Table(_) | Optlang::Index(_) => {
                let provides = if matches!(node, Optlang::Index(_)) { Sorted } else { Unsorted };
                provides.satisfies(desired).then_some(0)
            }

            // Arithmetic
            Optlang::Add(_) | Optlang::Sub(_) => {
                if !Bottom.satisfies(desired) {
                    return None;
                }
                let l = child_cost(children[0], &Bottom)?;
                let r = child_cost(children[1], &Bottom)?;
                Some(2usize.saturating_mul(CPU_COST).saturating_add(l).saturating_add(r))
            }
            Optlang::Mul(_) | Optlang::Div(_) => {
                if !Bottom.satisfies(desired) {
                    return None;
                }
                let l = child_cost(children[0], &Bottom)?;
                let r = child_cost(children[1], &Bottom)?;
                Some(4usize.saturating_mul(CPU_COST).saturating_add(l).saturating_add(r))
            }

            // Comparisons / logical: 1*CPU + children
            Optlang::Eq(_) | Optlang::Lt(_) | Optlang::Gt(_) | Optlang::Le(_)
            | Optlang::Ge(_) | Optlang::Ne(_) | Optlang::And(_) | Optlang::Or(_) => {
                if !Bottom.satisfies(desired) {
                    return None;
                }
                let l = child_cost(children[0], &Bottom)?;
                let r = child_cost(children[1], &Bottom)?;
                Some(CPU_COST.saturating_add(l).saturating_add(r))
            }
            Optlang::Not(_) => {
                if !Bottom.satisfies(desired) {
                    return None;
                }
                Some(CPU_COST.saturating_add(child_cost(children[0], &Bottom)?))
            }

            // Physical scans (children are leaf Table/Index nodes — no winner cost contribution)
            Optlang::TableScan(_) => {
                if !Unsorted.satisfies(desired) {
                    return None;
                }
                let stats = &children[0].logical;
                let blocks_io = stats.blocks.unwrap_or(0).saturating_mul(IO_COST);
                let row_xfer = stats.cardinality.unwrap_or(0).saturating_mul(TRANSFER_COST);
                Some(blocks_io.saturating_add(row_xfer))
            }
            Optlang::IndexScan(_) => {
                if !Sorted.satisfies(desired) {
                    return None;
                }
                let stats = &children[0].logical;
                let per_row = IO_COST.saturating_add(TRANSFER_COST);
                Some(stats.cardinality.unwrap_or(0).saturating_mul(per_row))
            }

            // Sort: free if we demanded Sorted from source; else pay sort's own work.
            // Matches the original cost function: the non-free path reports only the
            // sort operator's I/O and CPU contribution, not the recursive source cost.
            Optlang::Sort([_, _]) => {
                if !Sorted.satisfies(desired) {
                    return None;
                }
                let src = children[0];
                if src.provided.contains(&Sorted) {
                    // Free path: demanded Sorted from source.
                    Some(child_cost(src, &Sorted)?)
                } else {
                    // Verify source is costable for Bottom (i.e. some plan exists for it),
                    // but do not add it into our cost — original semantics.
                    let _ = child_cost(src, &Bottom)?;
                    let stats = &src.logical;
                    let n = stats.cardinality.unwrap_or(0);
                    let log_n = if n > 0 { (n as f64).log2() as usize } else { 0 };
                    let io = 3usize
                        .saturating_mul(stats.blocks.unwrap_or(0))
                        .saturating_mul(IO_COST);
                    let cpu = n.saturating_mul(log_n).saturating_mul(CPU_COST);
                    Some(io.saturating_add(cpu))
                }
            }

            Optlang::Select([_, _]) => {
                let src = children[0];
                let pred = children[1];
                if !src.provided.iter().any(|p| p.satisfies(desired)) {
                    return None;
                }
                let src_cost = child_cost(src, desired)?;
                let pred_cost = child_cost(pred, &Bottom)?;
                let n = src.logical.cardinality.unwrap_or(0);
                let total = src_cost.saturating_add(
                    pred_cost.saturating_add(TRANSFER_COST).saturating_mul(n),
                );
                Some(total)
            }
            Optlang::Project([_, _]) => {
                if children.len() < 2 {
                    return None;
                }
                let cols = children[0];
                let src = children[1];
                if !src.provided.iter().any(|p| p.satisfies(desired)) {
                    return None;
                }
                let src_cost = child_cost(src, desired)?;
                let cols_cost = child_cost(cols, &Bottom)?;
                let n = src.logical.cardinality.unwrap_or(0);
                let total = src_cost.saturating_add(
                    cols_cost.saturating_add(TRANSFER_COST).saturating_mul(n),
                );
                Some(total)
            }

            // Joins: report only the join operator's own work, matching the original
            // cost function. The recursive child costs (l_cost, r_cost) are required
            // to exist for feasibility (so missing-winner cycles fail cleanly), but
            // are not summed into the result.
            Optlang::NestedLoopJoin(_) => {
                if !Unsorted.satisfies(desired) {
                    return None;
                }
                let l = children[0];
                let r = children[1];
                let p = children[2];
                let _ = child_cost(l, &Bottom)?;
                let _ = child_cost(r, &Bottom)?;
                let p_cost = child_cost(p, &Bottom)?;
                let lb = l.logical.blocks.unwrap_or(0);
                let rb = r.logical.blocks.unwrap_or(0);
                let lc = l.logical.cardinality.unwrap_or(0);
                let rc = r.logical.cardinality.unwrap_or(0);
                let cost = lb
                    .saturating_mul(IO_COST)
                    .saturating_add(lb.saturating_mul(rb).saturating_mul(IO_COST))
                    .saturating_add(lc.saturating_mul(rc).saturating_mul(p_cost));
                Some(cost)
            }
            Optlang::HashJoin(_) => {
                if !Unsorted.satisfies(desired) {
                    return None;
                }
                let l = children[0];
                let r = children[1];
                let p = children[2];
                let _ = child_cost(l, &Bottom)?;
                let _ = child_cost(r, &Bottom)?;
                let p_cost = child_cost(p, &Bottom)?;
                let lb = l.logical.blocks.unwrap_or(0);
                let rb = r.logical.blocks.unwrap_or(0);
                let lc = l.logical.cardinality.unwrap_or(0);
                let rc = r.logical.cardinality.unwrap_or(0);
                let cost = 3usize
                    .saturating_mul(lb.saturating_add(rb).saturating_mul(IO_COST))
                    .saturating_add(lc.saturating_add(rc).saturating_mul(p_cost));
                Some(cost)
            }
            Optlang::MergeJoin(_) => {
                if !Sorted.satisfies(desired) {
                    return None;
                }
                let l = children[0];
                let r = children[1];
                let p = children[2];
                let _ = child_cost(l, &Sorted)?;
                let _ = child_cost(r, &Sorted)?;
                let p_cost = child_cost(p, &Bottom)?;
                let lb = l.logical.blocks.unwrap_or(0);
                let rb = r.logical.blocks.unwrap_or(0);
                let lc = l.logical.cardinality.unwrap_or(0);
                let rc = r.logical.cardinality.unwrap_or(0);
                let cost = lb
                    .saturating_add(rb)
                    .saturating_mul(IO_COST)
                    .saturating_add(lc.saturating_add(rc).saturating_mul(p_cost));
                Some(cost)
            }

            Optlang::Join(_) | Optlang::Scan(_) => None,
        }
    }
}

/// Compute the logical stats (cardinality + blocks) for a node given children's stats.
fn compute_stats(node: &Optlang, children: &[&Data], catalog: &Catalog) -> DbStats {
    match node {
        Optlang::Table(table_id) | Optlang::Index(table_id) => catalog
            .tables
            .get(table_id)
            .or_else(|| {
                catalog
                    .indexes
                    .get(table_id)
                    .and_then(|idx| catalog.tables.get(&idx.table_id))
            })
            .map(|t| DbStats {
                cardinality: Some(t.get_est_num_rows()),
                blocks: Some(t.get_est_num_blocks()),
            })
            .unwrap_or_default(),

        Optlang::TableScan(_) | Optlang::IndexScan(_) => {
            // children[0] holds the table/index node — its stats already reflect catalog.
            children
                .first()
                .map(|c| c.logical.clone())
                .unwrap_or_default()
        }

        Optlang::Select([_, _]) => children
            .first()
            .map(|c| scale_stats(&c.logical, SELECTIVITY_FACTOR))
            .unwrap_or_default(),

        Optlang::Project([_, _]) => children
            .get(1)
            .map(|c| c.logical.clone())
            .unwrap_or_default(),

        Optlang::Sort([_, _]) => children
            .first()
            .map(|c| c.logical.clone())
            .unwrap_or_default(),

        Optlang::HashJoin(_) | Optlang::MergeJoin(_) | Optlang::NestedLoopJoin(_) => {
            if children.len() >= 2 {
                join_stats(&children[0].logical, &children[1].logical)
            } else {
                DbStats::default()
            }
        }

        // Logical operators / leaves with no row-count meaning.
        _ => DbStats::default(),
    }
}
