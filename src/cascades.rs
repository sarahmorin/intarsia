use indexmap::IndexMap;
use lattices::cc_traits::Collection;

/// E-graph Cascades module
/// Perform a Cascades style search over the e-graph
use crate::egraph::EGraph;
use crate::explore::{Explorer, RuleMatch, StopResult};
use crate::rule::{Rule, RuleSet};
use crate::types::*;
use log::warn;
use std::collections::HashMap;

/// Possible tasks in the Cascades exploration. Most tasks simply generate more tasks and ensure proper dependency ordering.
/// The main work is done in the ApplyRule task.
///
/// We use ExploreClass on the final ruleset as our entry point for exploration.
/// Explore class will ensure that previous rulesets have been explored first before exploring the class with the current ruleset.
///
/// 1. OptimizeClass(eclass_id, extract_best): Explore the e-class for all rules.
/// - Create ExploreClass tasks for each ruleset in order.
/// - (Optionally) Extract the best expression from the e-class after exploration is complete.
/// 2. ExploreClass(eclass_id, ruleset_index): Explore the given e-class using the specified ruleset.
/// - Make sure we have run ExploreClass for the previous ruleset first.
/// - Enqueue ExploreNode tasks for each e-node in the e-class.
/// - Finally, set class,ruleset explored status to true.
/// 3. ExploreNode(eclass_id, enode_id, ruleset_index): Explore the given e-node using the specified ruleset.
/// - For any unexplored inputs, create OptimizeClass tasks for those inputs first.
/// - Iterate through all rules in the ruleset, find matches for this e-node.
/// - For each match, enqueue an ApplyRule task.
/// - Finally, set enode,ruleset explored status to true.
/// 4. FindMatches(eclass_id, enode_id, ruleset_index): Find applicable rules for the given e-node.
/// - For each rule in the ruleset, perform e-matching on the e-node.
/// - For each match found, enqueue an ApplyRule task.
/// 5. ApplyRule(eclass_id, rule_match): Apply the given rule match to the e-graph.
/// - Apply the rule to the e-graph, adding any new e-nodes/e-classes as needed.
/// 6. FinishExploreClass(eclass_id, ruleset_index): Mark the e-class as explored for the given ruleset.
/// - Verify that all e-nodes in the e-class have been explored for the given ruleset.
/// - Set the e-class,ruleset explored status to true.
/// 7. FinishExploreNode(eclass_id, enode_id, ruleset_index): Mark the e-node as explored for the given ruleset.
/// - Set the e-node,ruleset explored status to true.
/// 8. FinishOptimizeClass(eclass_id): Mark the e-class as fully optimized.
/// - Verify that all rulesets have been explored for this e-class.
pub enum CascadesTask<L>
where
    L: OpLang,
{
    /// OptimizeClass(eclass_id)
    OptimizeClass(Id),
    /// ExploreClass(eclass_id, ruleset_index)
    ExploreClass(Id, usize),
    /// ExploreNode(eclass_id, enode_id, ruleset_index)
    ExploreNode(Id, Id, usize),
    /// Find applicable rules for the given e-node
    FindMatches(Id, Id, usize),
    /// ApplyRule(rule_match)
    ApplyRule(RuleMatch<L>),
    /// Repair EGraph after writing
    Repair,
    /// FinishExploreClass(eclass_id, ruleset_index)
    FinishExploreClass(Id, usize),
    /// FinishExploreNode(eclass_id, enode_id, ruleset_index)
    FinishExploreNode(Id, Id, usize),
    /// FinishOptimizeClass(eclass_id)
    FinishOptimizeClass(Id),
}

pub struct Cascades<L, R>
where
    L: OpLang,
    R: RuleSet<L>,
{
    // The e-graph being explored
    eg: EGraph<L>,
    // The set of rulesets to use for exploration in order of execution.
    rulesets: Vec<R>,
    // The task queue for exploration
    stack: Vec<CascadesTask<L>>,
    // Status of ruleset explorations per e-node
    node_status: HashMap<Id, IndexMap<usize, bool>>,
    // Status of ruleset explorations per e-class
    class_status: HashMap<Id, IndexMap<usize, bool>>,
    // TODO: stats
}

impl<L, R> Cascades<L, R>
where
    L: OpLang,
    R: RuleSet<L>,
{
    /// Create a new Cascades explorer with the given rulesets
    pub fn new(rulesets: Vec<R>) -> Self {
        Self {
            eg: EGraph::new(),
            rulesets: rulesets,
            stack: Vec::new(),
            node_status: HashMap::new(),
            class_status: HashMap::new(),
        }
    }

    /// Initialize the Cascades explorer with an expression
    pub fn init(&mut self, expr: &Expr<L>) -> Id {
        // Add the expression to the e-graph
        let id = self.eg.add_expr(expr);
        // Push the initial exploration task onto the stack
        self.stack.push(CascadesTask::OptimizeClass(id));
        id
    }

    /// Get a reference to the e-graph
    pub fn egraph(&self) -> &EGraph<L> {
        &self.eg
    }

    /// Get a mutable reference to the e-graph
    pub fn egraph_mut(&mut self) -> &mut EGraph<L> {
        &mut self.eg
    }

    /// Get a reference to the ruleset at the given index
    pub fn ruleset(&self, index: usize) -> &R {
        &self.rulesets[index]
    }

    /// Get a mutable reference to the ruleset at the given index
    pub fn ruleset_mut(&mut self, index: usize) -> &mut R {
        &mut self.rulesets[index]
    }

    /// Set the ruleset at the given index
    pub fn set_ruleset(&mut self, index: usize, ruleset: R) {
        self.rulesets[index] = ruleset;
    }

    /// Get enode/ruleset explored status
    pub fn get_enode_ruleset_explored(&mut self, enode_id: &Id, rs: usize) -> bool {
        if let Some(rs_map) = self.node_status.get(enode_id) {
            *rs_map.get(&rs).unwrap_or(&false)
        } else {
            let mut rule_status = IndexMap::new();
            for i in 0..self.rulesets.len() {
                rule_status.insert(i, false);
            }
            self.node_status.insert(*enode_id, rule_status);
            false
        }
    }

    /// Get eclass/ruleset explored status
    pub fn get_eclass_ruleset_explored(&mut self, eclass_id: &Id, rs: usize) -> bool {
        let canonical_id = self.eg.find(*eclass_id);
        if let Some(rs_map) = self.class_status.get(&canonical_id) {
            *rs_map.get(&rs).unwrap_or(&false)
        } else {
            let mut rule_status = IndexMap::new();
            for i in 0..self.rulesets.len() {
                rule_status.insert(i, false);
            }
            self.class_status.insert(canonical_id, rule_status);
            false
        }
    }
}

impl<L, R> Explorer for Cascades<L, R>
where
    L: OpLang,
    R: RuleSet<L>,
{
    /// Run a single step of the Cascades exploration
    fn run_step(&mut self) -> StopResult {
        fn check_stopping_condition() -> StopResult {
            // TODO: Add logic to check for saturation, max iterations, etc.
            StopResult::Continue
        }
        // Pop a task from the task queue
        let task = match self.stack.pop() {
            Some(task) => task,
            None => return StopResult::NoTasks,
        };

        match task {
            CascadesTask::OptimizeClass(eclass_id) => {
                let eclass_id = self.eg.find(eclass_id);
                // If we have already marked this e-class as explored for all rulesets, skip
                if self.get_eclass_ruleset_explored(&eclass_id, self.rulesets.len() - 1) {
                    return check_stopping_condition();
                }
                // Push FinishOptimizeClass task onto stack
                self.stack
                    .push(CascadesTask::FinishOptimizeClass(eclass_id));
                // Push ExploreClass tasks for each ruleset in order (so they will be popped off in 0 to n order)
                for rs_index in (0..self.rulesets.len()).rev() {
                    self.stack
                        .push(CascadesTask::ExploreClass(eclass_id, rs_index));
                }
            }
            CascadesTask::ExploreClass(eclass_id, rs_index) => {
                let eclass_id = self.eg.find(eclass_id);
                // Verify that the previous ruleset has been explored for this e-class
                // if not, push myself back into the stack and push the previous ruleset's ExploreClass task
                // ahead of me.
                if rs_index > 0 && !self.get_eclass_ruleset_explored(&eclass_id, rs_index - 1) {
                    // Push myself back onto the stack
                    self.stack
                        .push(CascadesTask::ExploreClass(eclass_id, rs_index));
                    // Push previous ruleset's ExploreClass task onto the stack
                    self.stack
                        .push(CascadesTask::ExploreClass(eclass_id, rs_index - 1));
                    return check_stopping_condition();
                }
                // Push FinishExploreClass task onto stack
                self.stack
                    .push(CascadesTask::FinishExploreClass(eclass_id, rs_index));
                // Push ExploreNode tasks for each e-node in the e-class
                for enode_id in self.eg.get_enode_ids_in_eclass(&eclass_id) {
                    self.stack
                        .push(CascadesTask::ExploreNode(eclass_id, enode_id, rs_index));
                }
            }
            CascadesTask::ExploreNode(eclass_id, enode_id, rs_index) => {
                let eclass_id = self.eg.find(eclass_id);
                // If we have already explored this e-node for the given ruleset, skip.
                // (Also ensures the status map is initialized.)
                if self.get_enode_ruleset_explored(&enode_id, rs_index) {
                    return check_stopping_condition();
                }
                // Push FinishExploreNode task onto stack
                self.stack.push(CascadesTask::FinishExploreNode(
                    eclass_id, enode_id, rs_index,
                ));
                // Push FindMatches task onto stack
                self.stack
                    .push(CascadesTask::FindMatches(eclass_id, enode_id, rs_index));
                // For any unexplored inputs, create OptimizeClass tasks for those inputs first.
                let enode = self.eg.get_enode(&enode_id).unwrap();
                for input_id in enode.arg_ids() {
                    // Push OptimizeClass task for the input e-class
                    self.stack.push(CascadesTask::OptimizeClass(*input_id));
                }
            }
            CascadesTask::FindMatches(eclass_id, enode_id, rs_index) => {
                let eclass_id = self.eg.find(eclass_id);
                let mut matches = Vec::<RuleMatch<L>>::new();
                // For each rule in the ruleset, perform e-matching on the e-node.
                let ruleset = self.ruleset(rs_index);
                for rule in ruleset.rules() {
                    let subst_set = self.eg.ematch_enode(&rule.pattern, enode_id, &Subst::new());
                    if !subst_set.is_empty() {
                        // We have matches for this rule on this e-node
                        let rule_match = RuleMatch {
                            rule: rule.clone(),
                            eclass: eclass_id,
                            enode: enode_id,
                            subst_set: subst_set,
                        };
                        // Collect rule matches
                        matches.push(rule_match);
                    }
                }

                // TODO: This is where we can prioritize and filter matches later

                // Make sure we repair e-graph after apply rules
                if matches.len() > 0 {
                    self.stack.push(CascadesTask::Repair);
                }

                for rule_match in matches {
                    // Push ApplyRule task onto stack
                    self.stack.push(CascadesTask::ApplyRule(rule_match));
                }
            }
            CascadesTask::ApplyRule(rule_match) => {
                let eclass_id = self.eg.find(rule_match.eclass);
                // Apply the rule to the e-graph
                for subst in rule_match.subst_set {
                    // Add the rewritten pattern to the e-graph
                    let new_id = self.eg.add_match(&rule_match.rule.replacement, &subst);
                    // Merge the new e-class with the original e-class
                    self.eg.merge(eclass_id, new_id);
                }
            }
            CascadesTask::Repair => {
                // Repair the e-graph after writing
                self.eg.rebuild();
                // TODO: Add an optimization to make sure we only call repair when we need to
                // (i.e. if the next task after this is apply rule we should do all repairs at once after all apply rules are done)
            }
            CascadesTask::FinishExploreClass(eclass_id, rs_index) => {
                let eclass_id = self.eg.find(eclass_id);
                // Verify that all e-nodes in the e-class have been explored for the given ruleset
                let mut all_explored = true;
                for enode_id in self.eg.get_enode_ids_in_eclass(&eclass_id) {
                    if !self.get_enode_ruleset_explored(&enode_id, rs_index) {
                        all_explored = false;
                        warn!(
                            "E-Node {:?} in E-Class {:?} not fully explored, ruleset {} not explored",
                            enode_id, eclass_id, rs_index
                        );
                    }
                }
                if !all_explored {
                    self.stack
                        .push(CascadesTask::ExploreClass(eclass_id, rs_index))
                } else {
                    // Mark the e-class as explored for this ruleset
                    if let Some(rs_map) = self.class_status.get_mut(&eclass_id) {
                        rs_map.insert(rs_index, true);
                    }
                }
            }
            CascadesTask::FinishExploreNode(_eclass_id, enode_id, rs_index) => {
                // Mark the e-node as explored for this ruleset
                if !self.node_status.contains_key(&enode_id) {
                    let mut rule_status = IndexMap::new();
                    for i in 0..self.rulesets.len() {
                        rule_status.insert(i, false);
                    }
                    self.node_status.insert(enode_id, rule_status);
                }
                if let Some(rs_map) = self.node_status.get_mut(&enode_id) {
                    rs_map.insert(rs_index, true);
                }
            }
            CascadesTask::FinishOptimizeClass(eclass_id) => {
                let eclass_id = self.eg.find(eclass_id);
                // Verify that all rulesets have been explored for this e-class
                let mut all_explored = true;
                for rs_index in 0..self.rulesets.len() {
                    if !self.get_eclass_ruleset_explored(&eclass_id, rs_index) {
                        all_explored = false;
                        warn!(
                            "E-Class {:?} not fully explored, ruleset {} not explored",
                            eclass_id, rs_index
                        );
                    }
                }
                if !all_explored {
                    self.stack.push(CascadesTask::OptimizeClass(eclass_id))
                }
            }
        }

        check_stopping_condition()
    }

    fn update_stats(&mut self) {
        // TODO: Implement statistics tracking and updating
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Parseable, Parser};
    use crate::rule::Rule;
    use crate::types::{Expr, OpLang, Pattern, Subst};
    use crate::{impl_oplang_default, mk_rule};
    use std::fmt::{Display, Formatter};
    use std::hash::Hash;

    /// A minimal arithmetic language for cascades/eg tests.
    ///
    /// Contract:
    /// - No evaluation; only structural rewrites.
    /// - Constants are terminals.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    enum Arith {
        Add,
        Mul,
        Const(i64),
    }

    impl Display for Arith {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                Arith::Add => write!(f, "Add"),
                Arith::Mul => write!(f, "Mul"),
                Arith::Const(n) => write!(f, "{}", n),
            }
        }
    }

    impl Parseable for Arith {
        fn parse(s: &str) -> Result<Self, String> {
            let trimmed = s.trim();
            match trimmed {
                "Add" => Ok(Arith::Add),
                "Mul" => Ok(Arith::Mul),
                _ => trimmed
                    .parse::<i64>()
                    .map(Arith::Const)
                    .map_err(|_| format!("Unknown Arith token: {trimmed}")),
            }
        }
    }

    impl OpLang for Arith {
        impl_oplang_default!();

        fn arity(&self) -> usize {
            match self {
                Arith::Add | Arith::Mul => 2,
                Arith::Const(_) => 0,
            }
        }
    }

    fn c(n: i64) -> Expr<Arith> {
        Expr::new(Arith::Const(n), vec![])
    }

    fn add(a: Expr<Arith>, b: Expr<Arith>) -> Expr<Arith> {
        Expr::new(Arith::Add, vec![a, b])
    }

    fn mul(a: Expr<Arith>, b: Expr<Arith>) -> Expr<Arith> {
        Expr::new(Arith::Mul, vec![a, b])
    }

    fn pattern(s: &str) -> Pattern<Arith> {
        Parser::<Arith>::parse_pattern(s).expect("pattern parse")
    }

    fn run_cascades(
        expr: &Expr<Arith>,
        rulesets: Vec<Vec<Rule<Arith>>>,
        steps: usize,
    ) -> (Cascades<Arith, Vec<Rule<Arith>>>, Id) {
        let mut runner = Cascades::<Arith, Vec<Rule<Arith>>>::new(rulesets);
        let root = runner.init(expr);
        let _ = runner.run(steps);
        (runner, root)
    }

    #[test]
    fn cascades_add_commutativity_makes_add_terms_equivalent() {
        let rulesets = vec![vec![mk_rule!(
            Arith,
            "comm_add",
            "Add(?a, ?b)",
            "Add(?b, ?a)"
        )]];

        let (runner, root) = run_cascades(&add(c(1), c(2)), rulesets, 200);

        let pat = pattern("Add(2, 1)");
        let matches = runner.egraph().ematch(&pat, root, &Subst::new());
        assert!(
            !matches.is_empty(),
            "expected Add(2,1) to be equivalent to Add(1,2)"
        );
    }

    #[test]
    fn cascades_mul_left_identity_discovers_equivalence_to_x() {
        let rulesets = vec![vec![mk_rule!(Arith, "mul_one", "Mul(1, ?x)", "?x")]];

        let (runner, root) = run_cascades(&mul(c(1), add(c(2), c(3))), rulesets, 300);

        let pat = pattern("Add(2, 3)");
        let matches = runner.egraph().ematch(&pat, root, &Subst::new());
        assert!(
            !matches.is_empty(),
            "expected Mul(1, Add(2,3)) to be equivalent to Add(2,3)"
        );
    }

    #[test]
    fn cascades_distributivity_and_factorization_put_both_forms_in_one_eclass() {
        let rulesets = vec![vec![
            mk_rule!(
                Arith,
                "factor",
                "Add(Mul(?a, ?b), Mul(?a, ?c))",
                "Mul(?a, Add(?b, ?c))"
            ),
            mk_rule!(
                Arith,
                "distribute",
                "Mul(?a, Add(?b, ?c))",
                "Add(Mul(?a, ?b), Mul(?a, ?c))"
            ),
        ]];

        // Start with expanded form: (2*3) + (2*4)
        let (runner, root) = run_cascades(&add(mul(c(2), c(3)), mul(c(2), c(4))), rulesets, 500);

        // Expected: factored form is equivalent: 2 * (3 + 4)
        let pat = pattern("Mul(2, Add(3, 4))");
        let matches = runner.egraph().ematch(&pat, root, &Subst::new());
        assert!(
            !matches.is_empty(),
            "expected Add(Mul(2,3), Mul(2,4)) to be equivalent to Mul(2, Add(3,4))"
        );
    }

    #[test]
    fn cascades_multiple_rulesets_apply_in_order() {
        // Ruleset 0: commutativity of addition
        // Ruleset 1: 1 * x => x
        let rulesets = vec![
            vec![mk_rule!(Arith, "comm_add", "Add(?a, ?b)", "Add(?b, ?a)")],
            vec![mk_rule!(Arith, "mul_one", "Mul(1, ?x)", "?x")],
        ];

        let (runner, root) = run_cascades(&mul(c(1), add(c(1), c(2))), rulesets, 800);

        // Should be able to reach the inner add after removing the Mul(1, _).
        let pat = pattern("Add(1, 2)");
        assert!(
            !runner.egraph().ematch(&pat, root, &Subst::new()).is_empty(),
            "expected Mul(1, Add(1,2)) to be equivalent to Add(1,2)"
        );

        // And commutativity should make Add(2,1) equivalent as well.
        let pat = pattern("Add(2, 1)");
        assert!(
            !runner.egraph().ematch(&pat, root, &Subst::new()).is_empty(),
            "expected Add(2,1) to be equivalent via commutativity"
        );
    }

    #[test]
    fn cascades_reexplores_when_rule_creates_new_enode_enabling_another_rule() {
        // This test is specifically about the cascades scheduling behavior:
        // - Rule 1 (commutativity) creates a *new* enode in the same e-class.
        // - Rule 2 (add-zero-left) only matches that new enode.
        // Expected: starting from Add(5, 0), we can reach 5.

        let rulesets = vec![vec![
            // Make the intermediate enode deterministic and bounded:
            // Add(x, 0) -> Add(0, x)
            mk_rule!(Arith, "add_zero_right_to_left", "Add(?x, 0)", "Add(0, ?x)"),
            mk_rule!(Arith, "add_zero_left", "Add(0, ?x)", "?x"),
        ]];

        let (runner, root) = run_cascades(&add(c(5), c(0)), rulesets, 100);

        let pat = pattern("5");
        let matches = runner.egraph().ematch(&pat, root, &Subst::new());
        assert!(
            !matches.is_empty(),
            "expected Add(5,0) to rewrite to 5 via commutativity then add-zero"
        );
    }

    #[test]
    fn cascades_noop_ruleset_finishes_without_panicking() {
        // A single empty ruleset should cause exploration to complete and the task stack to drain.
        let rulesets: Vec<Vec<Rule<Arith>>> = vec![vec![]];
        let mut runner = Cascades::<Arith, Vec<Rule<Arith>>>::new(rulesets);
        let _root = runner.init(&add(c(1), c(2)));

        let res = runner.run(200);
        assert!(
            matches!(res, StopResult::NoTasks | StopResult::MaxIterations),
            "expected exploration to make progress and terminate (got a different stop result)"
        );

        // Graph should be queryable.
        assert!(!runner.egraph().eclass_ids().is_empty());
    }
}
