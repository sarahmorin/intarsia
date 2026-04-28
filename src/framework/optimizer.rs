#[cfg(feature = "rerun-metrics")]
use crate::metrics;
/// The core OptimizerFramework struct and its implementation.
///
/// The optimizer is generic over `A: OptimizableAnalysis<L>` — the winner's circle,
/// per-node costs, provided-property set, and any logical analysis all live inside
/// `egraph[id].data`, accessed via the `OptimizableAnalysis` trait. The framework
/// itself holds no parallel cost or winner tables.
use egg::{EGraph, Id, Language, RecExpr};
use log::{debug, info, warn};
use std::collections::HashSet;
use std::time::Instant;

use crate::framework::{
    analysis::OptimizableAnalysis, config::Config, hooks::ExplorerHooks, property::Property,
    task::Task,
};

/// StopReason indicates why the optimizer stopped before finding an optimal plan.
#[derive(Debug, Clone)]
pub enum StopReason {
    /// The optimizer stopped because it reached the configured time limit.
    TimeLimitReached,
    /// The optimizer stopped because it reached the configured node limit.
    NodeLimitReached,
    /// The optimizer stopped because it reached the configured task limit.
    TaskLimitReached,
    /// The optimizer stopped because it exhausted the search space (no more tasks to explore).
    SearchSpaceExhausted,
    /// The optimizer stopped for an unknown reason (should not happen).
    Unknown(String),
}

/// The main optimizer framework implementing cascades-style optimization.
///
/// # Type Parameters
///
/// * `L` - The language type (must implement [`Language`])
/// * `A` - The e-class analysis (must implement [`OptimizableAnalysis<L>`])
/// * `UserData` - User-defined data accessible during optimization
///
/// [`Language`]: https://docs.rs/egg/latest/egg/trait.Language.html
#[derive(Debug)]
pub struct OptimizerFramework<L, A, UserData>
where
    L: Language,
    A: OptimizableAnalysis<L>,
{
    /// The e-graph holding all expressions, their equivalences, and the analysis data
    /// (winner's circle, provided-property sets, per-node costs, logical analysis).
    pub egraph: EGraph<L, A>,

    /// Configuration for the optimizer.
    pub config: Config,

    /// User-defined data accessible during optimization.
    pub user_data: UserData,

    /// Task stack for the cascades optimization algorithm
    pub(crate) task_stack: Vec<Task<A::Property>>,

    /// Groups that have been explored
    explored_groups: HashSet<Id>,

    /// Groups that are currently being optimized (or have been; used to short-circuit
    /// re-optimization of `(class, requirement)` pairs we've already worked on).
    optimized_groups: HashSet<(Id, A::Property)>,

    #[cfg(feature = "rerun-metrics")]
    rerun_metrics: metrics::RerunStream,
}

impl<L, A, UserData> OptimizerFramework<L, A, UserData>
where
    L: Language,
    A: OptimizableAnalysis<L>,
{
    /// Create a new optimizer framework instance with an explicit analysis value.
    pub fn new(analysis: A, user_data: UserData) -> Self {
        Self {
            egraph: EGraph::new(analysis),
            config: Config::new(),
            user_data,
            task_stack: Vec::new(),
            explored_groups: HashSet::new(),
            optimized_groups: HashSet::new(),
            #[cfg(feature = "rerun-metrics")]
            rerun_metrics: None,
        }
    }

    /// Set time_limit in the optimizer configuration.
    pub fn with_time_limit(mut self, limit: std::time::Duration) -> Self {
        self.config = self.config.with_time_limit(limit);
        self
    }

    /// Set node_limit in the optimizer configuration.
    pub fn with_node_limit(mut self, limit: usize) -> Self {
        self.config = self.config.with_node_limit(limit);
        self
    }

    /// Set task_limit in the optimizer configuration.
    pub fn with_task_limit(mut self, limit: usize) -> Self {
        self.config = self.config.with_task_limit(limit);
        self
    }

    /// Enable scalar metrics logging with a caller-provided rerun recording stream.
    #[cfg(feature = "rerun-metrics")]
    pub fn with_rerun_metrics(mut self, rec: metrics::RerunStream) -> Self {
        self.rerun_metrics = rec;
        self
    }

    /// Initialize the optimizer with an initial expression.
    pub fn init(&mut self, expr: RecExpr<L>) -> Id {
        let id = self.egraph.add_expr(&expr);
        self.egraph.rebuild();
        id
    }

    /// Run optimization starting from the given expression ID.
    pub fn run(&mut self, id: Id) -> StopReason
    where
        Self: ExplorerHooks<L>,
    {
        let start_time = std::time::Instant::now();
        let mut tasks_processed: usize = 0;

        // Push the initial optimization task with bottom (no) requirement
        self.task_stack
            .push(Task::OptimizeGroup(id, A::Property::bottom(), false, false));

        let mut stop_reason = StopReason::SearchSpaceExhausted;

        #[cfg(feature = "rerun-metrics")]
        let mut summary_metrics = metrics::SummaryMetrics::new();

        while let Some(task) = self.task_stack.pop() {
            #[cfg(feature = "rerun-metrics")]
            let task_start = Instant::now();

            #[cfg(feature = "rerun-metrics")]
            let task_type = task.to_type_name();

            let _rebuild_time_s = match task {
                Task::OptimizeGroup(_, _, _, _) => {
                    self.run_optimize_group(task);
                    0.0
                }
                Task::OptimizeExpr(_, _) => {
                    self.run_optimize_expr(task);
                    0.0
                }
                Task::ExploreGroup(_, _) => self.run_explore_group(task),
                Task::ExploreChildren(_) => {
                    self.run_explore_children(task);
                    0.0
                }
            };
            tasks_processed += 1;

            #[cfg(feature = "rerun-metrics")]
            {
                let task_metrics = metrics::TaskMetrics {
                    egraph_nodes: self.egraph.total_size(),
                    egraph_classes: self.egraph.number_of_classes(),
                    task_type,
                    task_time_s: task_start.elapsed().as_secs_f64(),
                    rebuild_count: usize::from(matches!(task_type, "ExploreGroup")),
                    rebuild_time_s: _rebuild_time_s,
                    optimized_memo_pairs: self.optimized_groups.len(),
                };
                metrics::log_rerun_task(&self.rerun_metrics, &task_metrics, &mut summary_metrics);
            };

            if self.config.task_limit.is_some()
                && tasks_processed >= self.config.task_limit.unwrap()
            {
                info!("Task limit reached, stopping optimization early.");
                stop_reason = StopReason::TaskLimitReached;
                break;
            }

            if let Some(node_limit) = self.config.node_limit {
                let node_count = self.egraph.total_size();
                if node_count >= node_limit {
                    info!(
                        "Node limit reached ({} nodes), stopping optimization early.",
                        node_count
                    );
                    stop_reason = StopReason::NodeLimitReached;
                    break;
                }
            }

            if let Some(time_limit) = self.config.time_limit {
                let elapsed = std::time::Instant::now() - start_time;
                if elapsed >= time_limit {
                    info!(
                        "Time limit reached (elapsed {:?}), stopping optimization early.",
                        elapsed
                    );
                    stop_reason = StopReason::TimeLimitReached;
                    break;
                }
            }
        }

        if matches!(stop_reason, StopReason::SearchSpaceExhausted) {
            info!(
                "Search space exhausted after {:?}, no more tasks to explore.",
                std::time::Instant::now() - start_time
            );
        }

        #[cfg(feature = "rerun-metrics")]
        metrics::log_rerun_summary(&self.rerun_metrics, &summary_metrics);

        stop_reason
    }

    /// Push a task onto the task stack for processing.
    pub fn push_task(&mut self, task: Task<A::Property>) {
        self.task_stack.push(task);
    }

    /// Extract the best expression for `id` at the bottom (no) requirement.
    ///
    /// Convenience shim around [`crate::framework::extract::WinnerExtractor`]. For
    /// custom extraction strategies (different cost models, egg-style extraction,
    /// top-K, etc.) construct an extractor directly against `&self.egraph`.
    pub fn extract(&self, id: Id) -> RecExpr<L> {
        crate::framework::extract::WinnerExtractor::new(&self.egraph)
            .find_best(id)
            .1
    }

    /// Extract the best expression and its cost at the bottom requirement.
    pub fn extract_with_cost(&self, id: Id) -> (A::Cost, RecExpr<L>) {
        crate::framework::extract::WinnerExtractor::new(&self.egraph).find_best(id)
    }

    /// Extract the best expression and its cost for a specific property requirement.
    pub fn extract_for(&self, id: Id, requirement: A::Property) -> (A::Cost, RecExpr<L>) {
        crate::framework::extract::WinnerExtractor::new(&self.egraph)
            .find_best_for(id, requirement)
    }

    /// Run an optimize group task.
    fn run_optimize_group(&mut self, task: Task<A::Property>) {
        let (id, props, explored, optimized) = match task {
            Task::OptimizeGroup(id, props, explored, optimized) => (id, props, explored, optimized),
            _ => panic!("run_optimize_group called with non-optimize task"),
        };

        let id = self.egraph.find(id);

        debug!(
            "run_optimize_group: {:?}",
            Task::OptimizeGroup(id, props.clone(), explored, optimized)
        );

        self.optimized_groups.insert((id, props.clone()));

        // Phase: ensure the group has been explored before we try to optimize it.
        if !explored {
            self.task_stack
                .push(Task::OptimizeGroup(id, props.clone(), true, optimized));
            if !self.explored_groups.contains(&id) {
                self.task_stack.push(Task::ExploreGroup(id, false));
            }
            return;
        }

        // Phase: ensure each expression in the group has scheduled its child optimizations.
        //
        // In the new analysis-driven design, per-child requirements depend on the parent's
        // demanded property (`props`) — which is in scope here but not inside `OptimizeExpr`.
        // So we schedule children directly from this phase. `OptimizeExpr` is preserved as
        // a task variant (per the framework's contract) but its handler is now a no-op;
        // this phase does the work it used to do.
        if !optimized {
            // Snapshot node ids so we can iterate without holding a borrow on `self.egraph`.
            let node_ids: Vec<Id> = self.egraph.nodes_in_class(id).map(|(nid, _)| nid).collect();

            // Re-push self with optimized=true; it'll run after all child OptimizeGroups.
            self.task_stack
                .push(Task::OptimizeGroup(id, props.clone(), explored, true));

            for node_id in node_ids {
                let node = self.egraph.get_node(node_id).clone();
                let children: Vec<Id> = node.children().iter().copied().collect();
                // TODO: add something to get all child data in a veg in egg
                let child_data: Vec<&A::Data> =
                    children.iter().map(|&c| &self.egraph[c].data).collect();
                let reqs_opt = self
                    .egraph
                    .analysis
                    .child_requirements(&node, &props, &child_data);
                drop(child_data);

                let Some(reqs) = reqs_opt else {
                    // This node cannot produce `props` given its children — skip its
                    // children entirely; it will be filtered out by `cost(...)` returning
                    // None in phase 3.
                    continue;
                };

                for (child_id, req) in children.iter().zip(reqs.into_iter()) {
                    let canonical = self.egraph.find(*child_id);
                    if !self.optimized_groups.contains(&(canonical, req.clone())) {
                        self.task_stack
                            .push(Task::OptimizeGroup(canonical, req, false, false));
                    }
                }
            }
            return;
        }

        // Phase: select the cheapest node in this class that yields `props`.
        let node_ids: Vec<Id> = self.egraph.nodes_in_class(id).map(|(nid, _)| nid).collect();

        let mut best: Option<(Id, A::Cost)> = None;

        for node_id in node_ids {
            let node = self.egraph.get_node(node_id).clone();
            let child_data: Vec<&A::Data> = node
                .children()
                .iter()
                .map(|&c| &self.egraph[c].data)
                .collect();
            let cost_opt = self.egraph.analysis.cost(&node, &props, &child_data);
            // Drop child_data borrow here implicitly before we mutate egraph.
            drop(child_data);

            if let Some(cost) = cost_opt {
                A::record_node_cost(&mut self.egraph[id].data, node_id, cost.clone());
                match &best {
                    None => best = Some((node_id, cost)),
                    Some((_, c)) if cost < *c => best = Some((node_id, cost)),
                    _ => {}
                }
            }
        }

        if let Some((node_id, cost)) = best {
            debug!(
                "Optimized group {:?} with props {:?}: selected node {:?} with cost {:?}",
                id, props, node_id, cost
            );
            A::record_winner(&mut self.egraph[id].data, props, node_id, cost);
        } else {
            warn!(
                "No valid expression found for group {:?} with props {:?}",
                id, props
            );
        }
    }

    /// Run an optimize expr task.
    ///
    /// In the analysis-driven design, child scheduling moved into `run_optimize_group`
    /// (phase 2) because per-child requirements depend on the parent's demanded
    /// property (which is in scope there, not here). The `OptimizeExpr` task variant
    /// is preserved per the framework's task-structure contract, but the handler is
    /// now a no-op. The optimizer no longer pushes `OptimizeExpr` tasks of its own;
    /// users who push them via `push_task` will see them processed without effect.
    fn run_optimize_expr(&mut self, task: Task<A::Property>) {
        let (id, children_optimized) = match task {
            Task::OptimizeExpr(id, children_optimized) => (id, children_optimized),
            _ => panic!("run_optimize_expr called with non-optimize task"),
        };
        debug!(
            "run_optimize_expr (no-op in analysis-driven design): id={:?}, children_optimized={:?}",
            id, children_optimized
        );
    }

    /// Run an explore group task.
    fn run_explore_group(&mut self, task: Task<A::Property>) -> f64
    where
        Self: ExplorerHooks<L>,
    {
        let (id, explored) = match task {
            Task::ExploreGroup(id, explored) => (id, explored),
            _ => panic!("run_explore_group called with non-explore task"),
        };

        let id = self.egraph.find(id);

        debug!("run_explore_group: id={:?}, explored={:?}", id, explored);

        self.explored_groups.insert(id);

        if !explored {
            debug!("Scheduling explore of group {:?} and its children", id);
            self.task_stack.push(Task::ExploreGroup(id, true));
            let node_ids: Vec<Id> = self.egraph.nodes_in_class(id).map(|(nid, _)| nid).collect();
            for node_id in node_ids {
                self.task_stack.push(Task::ExploreChildren(node_id));
            }
            return 0.0;
        }

        self.task_stack.push(Task::ExploreGroup(id, true));

        let new_ids = self.explore(id);

        let mut changed = false;
        for new_id in &new_ids {
            if self.egraph.union(id, *new_id) {
                changed = true;
            }
        }

        let mut rebuild_time_s = 0.0;
        if changed {
            let rebuild_start = Instant::now();
            self.egraph.rebuild();
            rebuild_time_s = rebuild_start.elapsed().as_secs_f64();

            debug!(
                "Rebuilt e-graph for group {:?} in {:.6}s after discovering new equivalences.",
                id, rebuild_time_s
            );
        }

        debug!(
            "Finished exploring group {:?}. Discovered {} equivalent expressions. Changed: {}",
            id,
            new_ids.len(),
            changed
        );

        if !changed
            && let Some(Task::ExploreGroup(next_id, _)) = self.task_stack.last()
            && *next_id == id
        {
            debug!(
                "Skipping redundant explore of group {:?} since no new equivalences were found.",
                id
            );
            self.task_stack.pop();
        }

        rebuild_time_s
    }

    /// Run an explore children task.
    fn run_explore_children(&mut self, task: Task<A::Property>)
    where
        Self: ExplorerHooks<L>,
    {
        let id = match task {
            Task::ExploreChildren(id) => id,
            _ => panic!("run_explore_children called with non-explore children task"),
        };
        debug!("run_explore_children: id={:?}", id);

        let node = self.egraph.get_node(id).clone();
        for child in node.children() {
            let canonical_child = self.egraph.find(*child);
            if self.explored_groups.contains(&canonical_child) {
                debug!(
                    "Skipping explore of child group {:?} since it's already explored.",
                    canonical_child
                );
                continue;
            }
            debug!(
                "Scheduling explore of child group {:?} for parent node {:?}",
                canonical_child, id
            );
            self.task_stack
                .push(Task::ExploreGroup(canonical_child, false));
        }
    }
}
