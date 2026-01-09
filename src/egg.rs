/// Egg Module
/// Runs the traditional egg-style e-graph exploration loop
use crate::egraph::*;
use crate::rule::Rule;
use crate::types::*;

pub struct RuleMatch<L>
where
    L: OpLang,
{
    pub rule: Rule<L>,
    pub eclass: Id,
    pub subst_set: Vec<Subst<Var, Id>>,
}

pub struct Runner<L>
where
    L: OpLang,
{
    eg: EGraph<L>,
    ruleset: Vec<Rule<L>>, // TODO: Make this more generic
}

impl<L> Runner<L>
where
    L: OpLang,
{
    pub fn new(ruleset: Vec<Rule<L>>) -> Self {
        Self {
            eg: EGraph::new(),
            ruleset: ruleset,
        }
    }

    pub fn egraph(&self) -> &EGraph<L> {
        &self.eg
    }

    pub fn egraph_mut(&mut self) -> &mut EGraph<L> {
        &mut self.eg
    }

    pub fn ruleset(&self) -> &Vec<Rule<L>> {
        &self.ruleset
    }

    pub fn ruleset_mut(&mut self) -> &mut Vec<Rule<L>> {
        &mut self.ruleset
    }

    pub fn run(&mut self, iterations: usize) {
        // TODO: Do something more efficient like semi-naive evaluation later.
        // Essentially, for each iteration:
        // For every e-class, for every rule, try to apply the rule via e-matching
        // If any new e-classes or e-nodes are added, continue
        for _ in 0..iterations {
            let mut matches = Vec::<RuleMatch<L>>::new();
            // For every e-class, rule pair try to find matches
            for eclass_id in self.eg.eclass_ids() {
                for rule in &self.ruleset {
                    let subst_set = self.eg.ematch(&rule.pattern, eclass_id, &Subst::new());
                    if !subst_set.is_empty() {
                        // We have matches for this rule in this e-class
                        let rule_match = RuleMatch {
                            rule: rule.clone(),
                            eclass: eclass_id,
                            subst_set: subst_set,
                        };
                        matches.push(rule_match);
                    }
                }
            }

            // Apply all matches
            for rule_match in matches {
                for subst in rule_match.subst_set {
                    // Add the rewritten pattern to the e-graph
                    let new_id = self.eg.add_match(&rule_match.rule.replacement, &subst);
                    // Merge the new e-class with the existing one
                    self.eg.merge(rule_match.eclass, new_id);
                }
            }

            // Check if the e-graph was modified
            // We can stop early if no modifications were made
            // NOTE: we check this before rebuilding, as rebuilding resets the modified flag
            if !self.eg.modified {
                break;
            }

            // Rebuild the e-graph to maintain invariants
            self.eg.rebuild();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use crate::types::{Expr, OpLang, Pattern};
    use crate::{impl_oplang_default, mk_rule};
    use std::fmt::{Display, Formatter};
    use std::hash::Hash;

    /// A minimal arithmetic language for egg/eg tests.
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

    impl crate::parser::Parseable for Arith {
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

    #[test]
    fn egg_add_commutativity_makes_add_terms_equivalent() {
        // Ruleset: commutativity of addition.
        let rules = vec![mk_rule!(Arith, "comm_add", "Add(?a, ?b)", "Add(?b, ?a)")];
        let mut runner = Runner::<Arith>::new(rules);

        // Seed the e-graph with a term.
        let root = runner.egraph_mut().add_expr(&add(c(1), c(2)));

        // Run saturation.
        runner.run(3);

        // Expected behavior: `Add(2,1)` is now in the same eclass.
        let pat = pattern("Add(2, 1)");
        let matches = runner
            .egraph()
            .ematch(&pat, root, &crate::types::Subst::new());
        assert!(
            !matches.is_empty(),
            "expected Add(2,1) to be equivalent to Add(1,2)"
        );
    }

    #[test]
    fn egg_mul_left_identity_discovers_equivalence_to_x() {
        // Ruleset: 1 * x => x
        let rules = vec![mk_rule!(Arith, "mul_one", "Mul(1, ?x)", "?x")];
        let mut runner = Runner::<Arith>::new(rules);

        let root = runner.egraph_mut().add_expr(&mul(c(1), add(c(2), c(3))));

        runner.run(5);

        // Expected: Mul(1, Add(2,3)) is equivalent to Add(2,3).
        let pat = pattern("Add(2, 3)");
        let matches = runner
            .egraph()
            .ematch(&pat, root, &crate::types::Subst::new());
        assert!(
            !matches.is_empty(),
            "expected Mul(1, Add(2,3)) to be equivalent to Add(2,3)"
        );
    }

    #[test]
    fn egg_distributivity_and_factorization_put_both_forms_in_one_eclass() {
        // Ruleset: distributivity both directions.
        // This should create an equivalence class containing both expanded and factored forms.
        let rules = vec![
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
        ];
        let mut runner = Runner::<Arith>::new(rules);

        // Seed with expanded form: (2*3) + (2*4)
        let root = runner
            .egraph_mut()
            .add_expr(&add(mul(c(2), c(3)), mul(c(2), c(4))));

        runner.run(6);

        // Expected: factored form is equivalent: 2 * (3 + 4)
        let pat = pattern("Mul(2, Add(3, 4))");
        let matches = runner
            .egraph()
            .ematch(&pat, root, &crate::types::Subst::new());
        assert!(
            !matches.is_empty(),
            "expected Add(Mul(2,3), Mul(2,4)) to be equivalent to Mul(2, Add(3,4))"
        );
    }

    #[test]
    fn egg_stops_early_when_no_rules_apply() {
        // No-op ruleset; running should not modify the graph.
        let rules: Vec<crate::rule::Rule<Arith>> = vec![];
        let mut runner = Runner::<Arith>::new(rules);
        let _root = runner.egraph_mut().add_expr(&add(c(1), c(2)));

        runner.run(10);

        // Expected behavior: modified flag should end as false after rebuild, and graph should be stable.
        // We only test that the runner doesn't panic and the egraph is queryable.
        assert_eq!(runner.egraph().eclass_ids().len(), 3);
    }
}
