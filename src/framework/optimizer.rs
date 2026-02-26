/// The core OptimizerFramework struct and its implementation.
///
/// This module provides a generic cascades-style optimizer framework that can work
/// with any language and property system.
use egg::{EGraph, Extractor, Id, Language, RecExpr};
use log::{debug, warn};
use std::collections::{HashMap, HashSet};

use crate::framework::{
    cost::{CostDomain, CostFunction, SimpleCost},
    hooks::ExplorerHooks,
    language_ext::PropertyAwareLanguage,
    property::Property,
    task::Task,
};

/// The main optimizer framework implementing cascades-style optimization.
///
/// This struct holds all state needed for optimization including the e-graph,
/// task queue, memoization tables, and user-defined data.
///
/// # Type Parameters
///
/// * `L` - The language type (must implement `Language` and `PropertyAwareLanguage<P>`)
/// * `P` - The property type (must implement `Property`)
/// * `C` - The cost domain type (must implement `CostDomain<P>`)
/// * `UserData` - User-defined data accessible during optimization
///
/// # Examples
///
/// ```rust,ignore
/// // Define your user data struct
/// struct DbUserData {
///     catalog: Catalog,
///     colsets: BiMap<ColSet, ColSetId>,
/// }
///
/// // Create type alias for convenience using simple CostResult
/// type MyOptimizer = OptimizerFramework<QueryLang, SimpleProperty, SimpleCost<SimpleProperty>, DbUserData>;
///
/// // Or with a custom cost domain
/// type DbOptimizer = OptimizerFramework<QueryLang, SimpleProperty, DbCost<SimpleProperty>, DbUserData>;
///
/// // Create optimizer instance
/// let user_data = DbUserData { /* ... */ };
/// let mut optimizer = MyOptimizer::new(user_data);
///
/// // Initialize with an expression
/// let expr = /* build your expression */;
/// let root_id = optimizer.init(expr);
///
/// // Run optimization
/// optimizer.run(root_id);
///
/// // Extract best plan
/// let best_plan = optimizer.extract(root_id);
/// ```
#[derive(Debug, Clone)]
pub struct OptimizerFramework<L, P, C, D, UserData>
where
    L: Language + PropertyAwareLanguage<P>,
    P: Property,
    D: Ord,
    C: CostDomain<P, D>,
{
    __phantom_data: std::marker::PhantomData<(L, P, C, D)>, // To hold generic type parameters
    /// The e-graph holding all expressions and their equivalences
    pub egraph: EGraph<L, ()>,

    /// User-defined data accessible during optimization
    ///
    /// This field allows you to store domain-specific data that your cost function
    /// and ISLE rules need to access (e.g., database catalog, statistics, configuration).
    pub user_data: UserData,

    /// Task stack for the cascades optimization algorithm
    pub(crate) task_stack: Vec<Task<P>>,

    /// Groups currently being explored (to detect cycles)
    exploring_groups: HashSet<Id>,

    /// Groups that have been fully explored
    explored_groups: HashSet<Id>,

    /// Groups that are currently being optimized (to detect cycles)
    optimizing_groups: HashSet<(Id, P)>,

    /// Best expression for each (group, property) pair
    ///
    /// Maps (group_id, required_properties) to the ID of the best expression node
    /// that satisfies those properties.
    optimized_groups: HashMap<(Id, P), Id>,

    /// Memoized costs for each (group, property) pair
    ///
    /// Stores the cost of the best expression for each (group_id, required_properties) pair.
    costs: HashMap<(Id, P), C>,
}

impl<L, P, C, D, UserData> OptimizerFramework<L, P, C, D, UserData>
where
    L: Language + PropertyAwareLanguage<P>,
    P: Property,
    C: CostDomain<P, D>,
    D: Ord,
{
    /// Create a new optimizer framework instance.
    ///
    /// # Arguments
    ///
    /// * `user_data` - User-defined data accessible during optimization
    pub fn new(user_data: UserData) -> Self {
        Self {
            __phantom_data: std::marker::PhantomData,
            egraph: EGraph::default(),
            user_data,
            task_stack: Vec::new(),
            exploring_groups: HashSet::new(),
            explored_groups: HashSet::new(),
            optimizing_groups: HashSet::new(),
            optimized_groups: HashMap::new(),
            costs: HashMap::new(),
        }
    }

    /// Initialize the optimizer with an initial expression.
    ///
    /// Adds the expression to the e-graph and returns its ID.
    ///
    /// # Arguments
    ///
    /// * `expr` - The initial expression to optimize
    ///
    /// # Returns
    ///
    /// The ID of the root of the expression in the e-graph
    pub fn init(&mut self, expr: RecExpr<L>) -> Id {
        let id = self.egraph.add_expr(&expr);
        self.egraph.rebuild();
        id
    }

    /// Run optimization starting from the given expression ID.
    ///
    /// This performs cascades-style optimization: explore the expression space
    /// to find equivalent expressions, then select the lowest-cost expression
    /// satisfying the required properties.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the expression to optimize
    ///
    /// # Optimization Process
    ///
    /// 1. Push initial OptimizeGroup task with bottom (no requirements) properties
    /// 2. Process tasks from the stack:
    ///    - ExploreGroup/ExploreExpr: Generate equivalent expressions via rewrite rules
    ///    - OptimizeExpr: Compute costs of expressions
    ///    - OptimizeGroup: Select best expression for required properties
    /// 3. Continue until task stack is empty
    ///
    /// After `run()` completes, use `extract()` to get the best expression.
    pub fn run(&mut self, id: Id)
    where
        Self: ExplorerHooks<L> + CostFunction<L, P, D, C>,
    {
        // Push the initial optimization task with no property requirements
        self.task_stack
            .push(Task::OptimizeGroup(id, P::bottom(), false, false));

        // Process all tasks in the stack
        while let Some(task) = self.task_stack.pop() {
            match task {
                Task::OptimizeGroup(_, _, _, _) => self.run_optimize_group(task),
                Task::OptimizeExpr(_, _) => self.run_optimize_expr(task),
                Task::ExploreExpr(_, _) => self.run_explore_expr(task),
                Task::ExploreGroup(_, _) => self.run_explore_group(task),
            }
        }
    }

    /// Push a task onto the task stack for processing.
    ///
    /// This is used by ISLE-generated code to schedule exploration of new expressions.
    ///
    /// # Arguments
    ///
    /// * `task` - The task to schedule
    pub fn push_task(&mut self, task: Task<P>) {
        self.task_stack.push(task);
    }

    /// Extract the best expression for the given group.
    ///
    /// Returns the lowest-cost expression satisfying the default (bottom) properties.
    /// Assumes that `run()` has already been called to perform optimization.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the group to extract
    ///
    /// # Returns
    ///
    /// A RecExpr containing the best expression tree
    ///
    /// # Panics
    ///
    /// May panic if the expression hasn't been optimized yet (call `run()` first).
    pub fn extract(&self, id: Id) -> RecExpr<L> {
        let (_cost, best_expr) = self.extract_with_cost(id);
        best_expr
    }

    /// Extract the best expression along with its cost.
    ///
    /// Assumes that `run()` has already been called to perform optimization.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the group to extract
    ///
    /// # Returns
    ///
    /// A tuple of (cost, expression) for the best plan
    pub fn extract_with_cost(&self, id: Id) -> (C, RecExpr<L>) {
        self.extract_with_property(id, P::bottom())
    }

    /// Extract the best expression satisfying specific property requirements.
    ///
    /// Assumes that `run()` has already been called to perform optimization.
    /// If no expression satisfies the required properties, this will fall back to extracting
    /// the best expression by AstSize without property requirements, and will log a warning.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the group to extract
    /// * `props` - The properties required from the expression
    ///
    /// # Returns
    ///
    /// A tuple of (cost, expression) for the best plan satisfying the properties
    fn extract_with_property(&self, id: Id, props: P) -> (C, RecExpr<L>) {
        let eclass = self.egraph.find(id);

        // Look up the best node for this (eclass, property) pair
        let best_node_id = self.optimized_groups.get(&(eclass, props.clone()));

        if best_node_id.is_none() {
            warn!(
                "No optimized expression found for eclass {:?} with property {:?}\n\nFalling back to arbitrary AstSize extraction without property requirements.\nThis may indicate that optimization did not complete or that no expression satisfies the required properties.",
                eclass, props
            );
            // Fall back to egg's extractor if we haven't optimized this eclass
            let extractor = Extractor::new(&self.egraph, AstSize);
            let (_cost, rec_expr) = extractor.find_best(eclass);
            return (C::default(), rec_expr);
        }

        let best_node_id = *best_node_id.unwrap();
        let best_node = self.egraph.get_node(best_node_id);

        // Get the memoized cost
        let cost = self
            .costs
            .get(&(eclass, props.clone()))
            .cloned()
            .unwrap_or_else(C::default);

        // Recursively extract children with their required properties
        let mut expr = RecExpr::default();
        self.extract_node_to_recexpr(best_node, &mut expr);

        (cost, expr)
    }

    /// Recursively extract a node and its children into a RecExpr.
    ///
    /// This looks up required properties for each child and extracts them accordingly,
    /// building up the RecExpr from the bottom up.
    fn extract_node_to_recexpr(&self, node: &L, expr: &mut RecExpr<L>) -> Id {
        // For nodes with children, extract each child with required properties
        let children = node.children();
        if children.is_empty() {
            // Leaf node - just add it
            expr.add(node.clone())
        } else {
            // Extract each child
            let mut extracted_children = Vec::with_capacity(children.len());
            for (i, &child_id) in children.iter().enumerate() {
                let required_props = node.property_req(i);
                let extracted_child_id =
                    self.extract_child_for_node(child_id, required_props, expr);
                extracted_children.push(extracted_child_id);
            }

            // Create a new node with the extracted children
            let mut new_node = node.clone();
            let mut iter = extracted_children.into_iter();
            new_node.update_children(|_| iter.next().unwrap());
            expr.add(new_node)
        }
    }

    /// Extract a child eclass with required properties and add it to the RecExpr.
    ///
    /// Returns the Id of the extracted subtree in the RecExpr.
    fn extract_child_for_node(
        &self,
        child_eclass_id: Id,
        required_props: P,
        expr: &mut RecExpr<L>,
    ) -> Id {
        let child_eclass = self.egraph.find(child_eclass_id);

        // Look up the best node for this (child_eclass, required_props) pair
        if let Some(&best_node_id) = self
            .optimized_groups
            .get(&(child_eclass, required_props.clone()))
        {
            let best_node = self.egraph.get_node(best_node_id);
            self.extract_node_to_recexpr(best_node, expr)
        } else {
            // Fallback: if we don't have the required property memoized, try Bottom
            if required_props != P::bottom() {
                if let Some(&best_node_id) = self.optimized_groups.get(&(child_eclass, P::bottom()))
                {
                    let best_node = self.egraph.get_node(best_node_id);
                    return self.extract_node_to_recexpr(best_node, expr);
                }
            }

            // Last resort: extract any node from the eclass
            warn!(
                "No optimized node found for child eclass {:?} with property {:?}, using arbitrary node",
                child_eclass, required_props
            );
            if let Some((node_id, _)) = self.egraph.nodes_in_class(child_eclass).next() {
                let node = self.egraph.get_node(node_id);
                self.extract_node_to_recexpr(node, expr)
            } else {
                panic!("Eclass {:?} has no nodes!", child_eclass);
            }
        }
    }

    /// Run an optimize group task.
    ///
    /// This explores the group and optimizes all expressions, then selects the best one.
    fn run_optimize_group(&mut self, task: Task<P>)
    where
        Self: ExplorerHooks<L> + CostFunction<L, P, D, C>,
    {
        let (id, props, explored, optimized) = match task {
            Task::OptimizeGroup(id, props, explored, optimized) => (id, props, explored, optimized),
            _ => panic!("run_optimize_group called with non-optimize task"),
        };
        debug!(
            "run_optimize_group: {:?}",
            Task::OptimizeGroup(id, props.clone(), explored, optimized)
        );

        // If we have already optimized this group, skip it
        if self.optimized_groups.contains_key(&(id, props.clone())) {
            debug!("Group {:?} already optimized, skipping", id);
            return;
        }

        // Mark as in progress
        self.optimizing_groups.insert((id, props.clone()));

        // If we haven't explored this group yet, explore it first
        if !explored {
            self.task_stack
                .push(Task::OptimizeGroup(id, props.clone(), true, optimized));
            self.task_stack.push(Task::ExploreGroup(id, false));
            return;
        }

        // If we haven't optimized each expression yet, optimize them first
        if !optimized {
            self.task_stack
                .push(Task::OptimizeGroup(id, props.clone(), explored, true));
            for (node_id, _) in self.egraph.nodes_in_class(id) {
                self.task_stack.push(Task::OptimizeExpr(node_id, false));
            }
            return;
        }

        // Select the best expression in this group
        let mut best_expr: Option<Id> = None;
        let mut best_cost = C::default(); // Start with max cost

        for (node_id, node) in self.egraph.nodes_in_class(id) {
            // Compute cost with required properties
            if let Some(cost) = self.compute_expr_cost_with_props(node, props.clone()) {
                if cost < best_cost {
                    best_cost = cost;
                    best_expr = Some(node_id);
                }
            }
        }

        // Store the best expression in memo tables
        if let Some(expr_id) = best_expr {
            debug!(
                "Optimized group {:?} with props {:?}: selected expr {:?} with cost {:?}",
                id, props, expr_id, best_cost
            );
            self.costs.insert((id, props.clone()), best_cost);
            self.optimized_groups.insert((id, props.clone()), expr_id);
        } else {
            debug!(
                "Warning: No valid expression found for group {:?} with props {:?}",
                id, props
            );
            // QUESTION: Should we not mark this group as optimized if we didn't find any valid expression?
            // Or should we store a sentinel value to indicate that we've optimized but found nothing?
        }

        self.optimizing_groups.remove(&(id, props));
    }

    /// Run an optimize expr task.
    ///
    /// This optimizes all children of the expression first.
    fn run_optimize_expr(&mut self, task: Task<P>)
    where
        Self: CostFunction<L, P, D, C>,
    {
        let (id, children_optimized) = match task {
            Task::OptimizeExpr(id, children_optimized) => (id, children_optimized),
            _ => panic!("run_optimize_expr called with non-optimize task"),
        };
        debug!(
            "run_optimize_expr: id={:?}, children_optimized={:?}",
            id, children_optimized
        );

        // If children haven't been optimized yet, optimize them first
        if !children_optimized {
            self.task_stack.push(Task::OptimizeExpr(id, true));

            let node = self.egraph.get_node(id);

            // Optimize each child with its required properties
            for (i, child) in node.children().iter().enumerate() {
                let props = node.property_req(i);
                if self.optimized_groups.contains_key(&(*child, props.clone()))
                    || self.optimizing_groups.contains(&(*child, props.clone()))
                {
                    // Skip if already optimized or in progress
                    continue;
                }
                self.task_stack
                    .push(Task::OptimizeGroup(*child, props, false, false));
            }

            return;
        }

        // Children are optimized - the cost computation happens in run_optimize_group
        // QUESTION: Should we do expression level bookkeeping here or just rely on group-level optimization?
    }

    /// Run an explore group task.
    ///
    /// This explores all expressions in the group.
    fn run_explore_group(&mut self, task: Task<P>)
    where
        Self: ExplorerHooks<L>,
    {
        let (id, explored) = match task {
            Task::ExploreGroup(id, explored) => (id, explored),
            _ => panic!("run_explore_group called with non-explore task"),
        };
        debug!("run_explore_group: id={:?}, explored={:?}", id, explored);

        // If already explored, skip
        if self.explored_groups.contains(&id) {
            debug!("Group {:?} already explored, skipping", id);
            return;
        }
        self.exploring_groups.insert(id);

        // If expressions haven't been explored yet, explore them
        if !explored {
            self.task_stack.push(Task::ExploreGroup(id, true));
            for (node_id, _) in self.egraph.nodes_in_class(id) {
                self.task_stack.push(Task::ExploreExpr(node_id, false));
            }
            return;
        }

        // Mark group as explored
        self.explored_groups.insert(id);
        self.exploring_groups.remove(&id);
    }

    /// Run an explore expr task.
    ///
    /// This explores children first, then applies rewrite rules to this expression.
    fn run_explore_expr(&mut self, task: Task<P>)
    where
        Self: ExplorerHooks<L>,
    {
        let (id, children_explored) = match task {
            Task::ExploreExpr(id, children_explored) => (id, children_explored),
            _ => panic!("run_explore_expr called with non-explore task"),
        };
        debug!(
            "run_explore_expr: id={:?}, children_explored={:?}",
            id, children_explored
        );

        // If children haven't been explored yet, explore them first
        if !children_explored {
            self.task_stack.push(Task::ExploreExpr(id, true));
            for child in self.egraph.get_node(id).children() {
                if self.explored_groups.contains(child) || self.exploring_groups.contains(child) {
                    // Skip if already explored or in progress (cycle detection)
                    continue;
                }
                self.task_stack.push(Task::ExploreGroup(*child, false));
            }
            return;
        }

        // Apply rewrite rules to this expression
        let new_ids = self.explore(id);

        // Union new expressions with this one
        for new_id in new_ids {
            self.egraph.union(id, new_id);
        }

        // Rebuild e-graph to propagate equivalences
        self.egraph.rebuild();
    }

    /// Compute the cost of an expression with required properties using memoized costs.
    ///
    /// Returns None if any child doesn't have memoized costs or if the expression
    /// doesn't satisfy the required properties.
    // QUESTION: Should we distinguish between "child doesn't have memoized cost yet" vs "child's best expression doesn't satisfy required properties"?
    fn compute_expr_cost_with_props(&self, node: &L, required_props: P) -> Option<C>
    where
        Self: CostFunction<L, P, D, C>,
    {
        // Build a map of child costs with their required properties
        let mut child_cost_map: HashMap<Id, C> = HashMap::new();
        for (i, child_id) in node.children().iter().enumerate() {
            let child_req_props = node.property_req(i);
            let cost = self.costs.get(&(*child_id, child_req_props)).cloned()?;
            child_cost_map.insert(*child_id, cost);
        }

        // Create closure that returns memoized costs
        let cost_closure = |child_id: Id| {
            child_cost_map
                .get(&child_id)
                .cloned()
                .unwrap_or_else(C::default)
        };

        // Compute the cost
        let computed_cost = self.compute_cost(node, cost_closure);

        // Check if it satisfies required properties
        if computed_cost.properties().satisfies(&required_props) {
            Some(computed_cost)
        } else {
            debug!(
                "Expression {:?} provides {:?} but requires {:?}",
                node,
                computed_cost.properties(),
                required_props
            );
            None
        }
    }
}

/// Simple AST size cost function for fallback extraction.
struct AstSize;
impl<L: Language> egg::CostFunction<L> for AstSize {
    type Cost = usize;
    fn cost<C>(&mut self, enode: &L, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        enode.fold(1, |sum, id| sum + costs(id))
    }
}

/// A convenient type alias for a simple optimizer framework over a language and property using SimpleCost and usize as the cost domain.
pub type SimpleOptimizerFramework<L, P> = OptimizerFramework<L, P, SimpleCost<P>, usize, ()>;
