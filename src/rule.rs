/// This module defines structure of rewrite rules.
/// Rules specify rewrite transformations on the abstract syntax tree (AST) in some language T.
use crate::types::*;

#[derive(Clone)]
pub struct Rule<L>
where
    L: OpLang,
{
    pub name: String,
    pub pattern: Pattern<L>,
    pub replacement: Pattern<L>,
    // TODO: Add priority and other annotations later
}

impl<L> Rule<L>
where
    L: OpLang,
{
    pub fn new(name: String, pattern: Pattern<L>, replacement: Pattern<L>) -> Self {
        Self {
            name,
            pattern,
            replacement,
        }
    }
}

/// Macro to create rewrite rules easily.
///
/// This macro provides two forms:
///
/// 1. **Generic form** (recommended for new code):
///    ```rust
///    let rule = mk_rule!(MyLanguage, "rule_name", "pattern_string", "replacement_string");
///    ```
///    
/// 2. **Legacy form** (for backward compatibility):
///    ```rust  
///    let rule = mk_rule!("rule_name", "pattern_string", "replacement_string");
///    ```
///    This defaults to using the `Ops` language from the test module.
///
/// # Arguments
///
/// * `$lang` (optional) - The language type that implements `OpLang + Parseable`
/// * `$name` - A string expression for the rule name
/// * `$pattern` - A string expression representing the pattern to match
/// * `$replacement` - A string expression representing the replacement pattern
///
/// # Examples
///
/// ```rust
/// use crate::mk_rule;
/// use crate::testlang::Ops;
///
/// // Using the generic form
/// let rule1 = mk_rule!(Ops, "and_to_or", "And(?x, ?y)", "Or(?x, ?y)");
///
/// // Using the legacy form (defaults to Ops)
/// let rule2 = mk_rule!("double_negation", "Not(Not(?x))", "?x");
/// ```
///
/// # Panics
///
/// This macro will panic if the pattern or replacement strings cannot be parsed.
/// Make sure your patterns are valid for the specified language.
#[macro_export]
macro_rules! mk_rule {
    ($lang:ty, $name:expr, $pattern:expr, $replacement:expr) => {{
        use $crate::parser::Parser;
        use $crate::rule::Rule;

        Rule::new(
            $name.to_string(),
            Parser::<$lang>::parse_pattern($pattern).expect("Failed to parse pattern"),
            Parser::<$lang>::parse_pattern($replacement).expect("Failed to parse replacement"),
        )
    }};
    // Backward compatibility - defaults to Ops for existing code
    ($name:expr, $pattern:expr, $replacement:expr) => {{
        use $crate::parser::Parser;
        use $crate::rule::Rule;
        use $crate::testlang::QueryOps;

        Rule::new(
            $name.to_string(),
            Parser::<QueryOps>::parse_pattern($pattern).expect("Failed to parse pattern"),
            Parser::<QueryOps>::parse_pattern($replacement).expect("Failed to parse replacement"),
        )
    }};
}

// NOTE: We can pick more interesting structs here, could be a place to allow for user-defined organization
pub trait RuleSet<L>
where
    L: OpLang,
{
    /// Get all rules in the set.
    fn rules(&self) -> &Vec<Rule<L>>;
    /// Get rule by some index.
    fn get_rule(&self, i: usize) -> Option<&Rule<L>>;
    /// Get rule by Name.
    fn get_rule_by_name(&self, name: &str) -> Option<&Rule<L>>;
    /// Add a rule to the set.
    fn add_rule(&mut self, rule: Rule<L>);
    /// Remove a rule from the set.
    fn remove_rule(&mut self, rule: &Rule<L>);
}

impl<L> RuleSet<L> for Vec<Rule<L>>
where
    L: OpLang,
{
    fn rules(&self) -> &Vec<Rule<L>> {
        self
    }

    fn get_rule(&self, index: usize) -> Option<&Rule<L>> {
        self.get(index)
    }

    fn get_rule_by_name(&self, name: &str) -> Option<&Rule<L>> {
        self.iter().find(|r| r.name == name)
    }

    fn add_rule(&mut self, rule: Rule<L>) {
        self.push(rule);
    }

    fn remove_rule(&mut self, rule: &Rule<L>) {
        if let Some(pos) = self.iter().position(|r| r.name == rule.name) {
            self.remove(pos);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use crate::testlang::QueryOps;

    #[test]
    fn test_rule_creation() {
        let pattern =
            Parser::<QueryOps>::parse_pattern("And(?a, ?b)").expect("Failed to parse pattern");
        let replacement =
            Parser::<QueryOps>::parse_pattern("Or(?a, ?b)").expect("Failed to parse replacement");

        let rule = Rule::new(
            "test_rule".to_string(),
            pattern.clone(),
            replacement.clone(),
        );

        assert_eq!(rule.name, "test_rule");
        assert_eq!(rule.pattern, pattern);
        assert_eq!(rule.replacement, replacement);
    }

    #[test]
    fn test_rule_macro_basic() {
        let rule = mk_rule!("and_to_or", "And(?x, ?y)", "Or(?x, ?y)");

        assert_eq!(rule.name, "and_to_or");

        // Check pattern structure
        match rule.pattern.op() {
            OpOrVar::Op(QueryOps::And) => {}
            _ => panic!("Expected And operation in pattern"),
        }
        assert_eq!(rule.pattern.args().len(), 2);

        // Check replacement structure
        match rule.replacement.op() {
            OpOrVar::Op(QueryOps::Or) => {}
            _ => panic!("Expected Or operation in replacement"),
        }
        assert_eq!(rule.replacement.args().len(), 2);
    }

    #[test]
    fn test_rule_macro_with_variables() {
        let rule = mk_rule!("double_negation", "Not(Not(?x))", "?x");

        assert_eq!(rule.name, "double_negation");

        // Check pattern: Not(Not(?x))
        match rule.pattern.op() {
            OpOrVar::Op(QueryOps::Not) => {}
            _ => panic!("Expected Not operation in pattern"),
        }
        assert_eq!(rule.pattern.args().len(), 1);

        // Inner Not(?x)
        let inner_not = &rule.pattern.args()[0];
        match inner_not.op() {
            OpOrVar::Op(QueryOps::Not) => {}
            _ => panic!("Expected inner Not operation"),
        }
        assert_eq!(inner_not.args().len(), 1);

        // Variable x
        match inner_not.args()[0].op() {
            OpOrVar::Var(var) => assert_eq!(var, "x"),
            _ => panic!("Expected variable x"),
        }

        // Check replacement: ?x
        match rule.replacement.op() {
            OpOrVar::Var(var) => assert_eq!(var, "x"),
            _ => panic!("Expected variable x in replacement"),
        }
        assert!(rule.replacement.args().is_empty());
    }

    #[test]
    fn test_rule_macro_with_constants() {
        let rule = mk_rule!("simplify_and_true", "And(?x, true)", "?x");

        assert_eq!(rule.name, "simplify_and_true");

        // Check pattern: And(?x, true)
        match rule.pattern.op() {
            OpOrVar::Op(QueryOps::And) => {}
            _ => panic!("Expected And operation in pattern"),
        }
        assert_eq!(rule.pattern.args().len(), 2);

        // First argument: ?x
        match rule.pattern.args()[0].op() {
            OpOrVar::Var(var) => assert_eq!(var, "x"),
            _ => panic!("Expected variable x"),
        }

        // Second argument: true
        match rule.pattern.args()[1].op() {
            OpOrVar::Op(QueryOps::ConstBool(true)) => {}
            _ => panic!("Expected true constant"),
        }

        // Check replacement: ?x
        match rule.replacement.op() {
            OpOrVar::Var(var) => assert_eq!(var, "x"),
            _ => panic!("Expected variable x in replacement"),
        }
    }

    #[test]
    fn test_rule_macro_complex_expression() {
        let rule = mk_rule!(
            "complex_rule",
            "Or(And(?a, ?b), Not(?c))",
            "And(Or(?a, Not(?c)), Or(?b, Not(?c)))"
        );

        assert_eq!(rule.name, "complex_rule");

        // Check pattern: Or(And(?a, ?b), Not(?c))
        match rule.pattern.op() {
            OpOrVar::Op(QueryOps::Or) => {}
            _ => panic!("Expected Or operation in pattern"),
        }
        assert_eq!(rule.pattern.args().len(), 2);

        // First argument: And(?a, ?b)
        let and_expr = &rule.pattern.args()[0];
        match and_expr.op() {
            OpOrVar::Op(QueryOps::And) => {}
            _ => panic!("Expected And operation"),
        }
        assert_eq!(and_expr.args().len(), 2);

        // Second argument: Not(?c)
        let not_expr = &rule.pattern.args()[1];
        match not_expr.op() {
            OpOrVar::Op(QueryOps::Not) => {}
            _ => panic!("Expected Not operation"),
        }
        assert_eq!(not_expr.args().len(), 1);

        // Check replacement structure
        match rule.replacement.op() {
            OpOrVar::Op(QueryOps::And) => {}
            _ => panic!("Expected And operation in replacement"),
        }
        assert_eq!(rule.replacement.args().len(), 2);
    }

    #[test]
    fn test_rule_macro_with_database_ops() {
        let rule = mk_rule!(
            "scan_to_index_scan",
            "Scan(Table[users])",
            "IndexScan(Table[users], Col[id])"
        );

        assert_eq!(rule.name, "scan_to_index_scan");

        // Check pattern: Scan(Table[users])
        match rule.pattern.op() {
            OpOrVar::Op(QueryOps::Scan) => {}
            _ => panic!("Expected Scan operation in pattern"),
        }
        assert_eq!(rule.pattern.args().len(), 1);

        match rule.pattern.args()[0].op() {
            OpOrVar::Op(QueryOps::Table(name)) => assert_eq!(name, "users"),
            _ => panic!("Expected Table[users]"),
        }

        // Check replacement: IndexScan(Table[users], Col[id])
        match rule.replacement.op() {
            OpOrVar::Op(QueryOps::IndexScan) => {}
            _ => panic!("Expected IndexScan operation in replacement"),
        }
        assert_eq!(rule.replacement.args().len(), 2);

        match rule.replacement.args()[0].op() {
            OpOrVar::Op(QueryOps::Table(name)) => assert_eq!(name, "users"),
            _ => panic!("Expected Table[users] in replacement"),
        }

        match rule.replacement.args()[1].op() {
            OpOrVar::Op(QueryOps::Col(name)) => assert_eq!(name, "id"),
            _ => panic!("Expected Col[id] in replacement"),
        }
    }

    #[test]
    fn test_rule_set_add_and_get() {
        let mut rule_set: Vec<Rule<QueryOps>> = Vec::new();

        let rule1 = mk_rule!("rule1", "And(?a, ?b)", "Or(?a, ?b)");
        let rule2 = mk_rule!("rule2", "Not(Not(?x))", "?x");

        rule_set.add_rule(rule1);
        rule_set.add_rule(rule2);

        assert_eq!(rule_set.rules().len(), 2);

        // Test get_rule by index
        let retrieved_rule = rule_set.get_rule(0).expect("Rule should exist");
        assert_eq!(retrieved_rule.name, "rule1");

        let retrieved_rule = rule_set.get_rule(1).expect("Rule should exist");
        assert_eq!(retrieved_rule.name, "rule2");

        assert!(rule_set.get_rule(2).is_none());
    }

    #[test]
    fn test_rule_set_get_by_name() {
        let mut rule_set: Vec<Rule<QueryOps>> = Vec::new();

        let rule1 = mk_rule!("and_to_or", "And(?a, ?b)", "Or(?a, ?b)");
        let rule2 = mk_rule!("double_negation", "Not(Not(?x))", "?x");
        let rule3 = mk_rule!("simplify_true", "And(?x, true)", "?x");

        rule_set.add_rule(rule1);
        rule_set.add_rule(rule2);
        rule_set.add_rule(rule3);

        // Test get_rule_by_name
        let retrieved_rule = rule_set
            .get_rule_by_name("double_negation")
            .expect("Rule should exist");
        assert_eq!(retrieved_rule.name, "double_negation");

        let retrieved_rule = rule_set
            .get_rule_by_name("and_to_or")
            .expect("Rule should exist");
        assert_eq!(retrieved_rule.name, "and_to_or");

        let retrieved_rule = rule_set
            .get_rule_by_name("simplify_true")
            .expect("Rule should exist");
        assert_eq!(retrieved_rule.name, "simplify_true");

        assert!(rule_set.get_rule_by_name("nonexistent").is_none());
    }

    #[test]
    fn test_rule_set_remove() {
        let mut rule_set: Vec<Rule<QueryOps>> = Vec::new();

        let rule1 = mk_rule!("rule1", "And(?a, ?b)", "Or(?a, ?b)");
        let rule2 = mk_rule!("rule2", "Not(Not(?x))", "?x");
        let rule3 = mk_rule!("rule3", "And(?x, true)", "?x");

        rule_set.add_rule(rule1.clone());
        rule_set.add_rule(rule2.clone());
        rule_set.add_rule(rule3.clone());

        assert_eq!(rule_set.rules().len(), 3);

        // Remove rule2
        rule_set.remove_rule(&rule2);
        assert_eq!(rule_set.rules().len(), 2);

        // Check that rule2 is gone
        assert!(rule_set.get_rule_by_name("rule2").is_none());

        // Check that other rules are still there
        assert!(rule_set.get_rule_by_name("rule1").is_some());
        assert!(rule_set.get_rule_by_name("rule3").is_some());

        // Remove rule1
        rule_set.remove_rule(&rule1);
        assert_eq!(rule_set.rules().len(), 1);

        // Only rule3 should remain
        assert!(rule_set.get_rule_by_name("rule1").is_none());
        assert!(rule_set.get_rule_by_name("rule3").is_some());
    }

    #[test]
    fn test_rule_set_remove_nonexistent() {
        let mut rule_set: Vec<Rule<QueryOps>> = Vec::new();

        let rule1 = mk_rule!("rule1", "And(?a, ?b)", "Or(?a, ?b)");
        let rule2 = mk_rule!("rule2", "Not(Not(?x))", "?x");

        rule_set.add_rule(rule1);

        // Try to remove rule2 which was never added
        let initial_len = rule_set.rules().len();
        rule_set.remove_rule(&rule2);

        // Length should remain the same
        assert_eq!(rule_set.rules().len(), initial_len);
    }

    #[test]
    fn test_rule_set_rules_reference() {
        let mut rule_set: Vec<Rule<QueryOps>> = Vec::new();

        let rule1 = mk_rule!("rule1", "And(?a, ?b)", "Or(?a, ?b)");
        let rule2 = mk_rule!("rule2", "Not(Not(?x))", "?x");

        rule_set.add_rule(rule1);
        rule_set.add_rule(rule2);

        let rules_ref = rule_set.rules();
        assert_eq!(rules_ref.len(), 2);
        assert_eq!(rules_ref[0].name, "rule1");
        assert_eq!(rules_ref[1].name, "rule2");
    }

    #[test]
    fn test_rule_with_multiple_variables() {
        let rule = mk_rule!("commutative_and", "And(?x, ?y)", "And(?y, ?x)");

        assert_eq!(rule.name, "commutative_and");

        // Verify pattern has correct structure
        match rule.pattern.op() {
            OpOrVar::Op(QueryOps::And) => {}
            _ => panic!("Expected And operation in pattern"),
        }

        // Check variables in pattern
        match rule.pattern.args()[0].op() {
            OpOrVar::Var(var) => assert_eq!(var, "x"),
            _ => panic!("Expected variable x"),
        }

        match rule.pattern.args()[1].op() {
            OpOrVar::Var(var) => assert_eq!(var, "y"),
            _ => panic!("Expected variable y"),
        }

        // Verify replacement has swapped variables
        match rule.replacement.op() {
            OpOrVar::Op(QueryOps::And) => {}
            _ => panic!("Expected And operation in replacement"),
        }

        // Check swapped variables in replacement
        match rule.replacement.args()[0].op() {
            OpOrVar::Var(var) => assert_eq!(var, "y"),
            _ => panic!("Expected variable y"),
        }

        match rule.replacement.args()[1].op() {
            OpOrVar::Var(var) => assert_eq!(var, "x"),
            _ => panic!("Expected variable x"),
        }
    }

    #[test]
    fn test_rule_with_nested_variables() {
        let rule = mk_rule!("distribute_not", "Not(And(?a, ?b))", "Or(Not(?a), Not(?b))");

        assert_eq!(rule.name, "distribute_not");

        // Check pattern: Not(And(?a, ?b))
        match rule.pattern.op() {
            OpOrVar::Op(QueryOps::Not) => {}
            _ => panic!("Expected Not operation in pattern"),
        }

        let inner_and = &rule.pattern.args()[0];
        match inner_and.op() {
            OpOrVar::Op(QueryOps::And) => {}
            _ => panic!("Expected And operation inside Not"),
        }

        // Check replacement: Or(Not(?a), Not(?b))
        match rule.replacement.op() {
            OpOrVar::Op(QueryOps::Or) => {}
            _ => panic!("Expected Or operation in replacement"),
        }

        let first_not = &rule.replacement.args()[0];
        match first_not.op() {
            OpOrVar::Op(QueryOps::Not) => {}
            _ => panic!("Expected Not operation in first argument"),
        }

        let second_not = &rule.replacement.args()[1];
        match second_not.op() {
            OpOrVar::Op(QueryOps::Not) => {}
            _ => panic!("Expected Not operation in second argument"),
        }
    }

    #[test]
    fn test_rule_with_database_variables() {
        let rule = mk_rule!(
            "filter_pushdown",
            "Select(Join(?t1, ?t2), ?columns, ?condition)",
            "Join(Select(?t1, ?columns, ?condition), ?t2)"
        );

        assert_eq!(rule.name, "filter_pushdown");

        // Check pattern structure
        match rule.pattern.op() {
            OpOrVar::Op(QueryOps::Select) => {}
            _ => panic!("Expected Select operation in pattern"),
        }

        let join_expr = &rule.pattern.args()[0];
        match join_expr.op() {
            OpOrVar::Op(QueryOps::Join) => {}
            _ => panic!("Expected Join operation"),
        }

        // Should have 3 arguments: input (Join), columns, condition
        assert_eq!(rule.pattern.args().len(), 3);

        // Check replacement structure
        match rule.replacement.op() {
            OpOrVar::Op(QueryOps::Join) => {}
            _ => panic!("Expected Join operation in replacement"),
        }

        let filter_expr = &rule.replacement.args()[0];
        match filter_expr.op() {
            OpOrVar::Op(QueryOps::Select) => {}
            _ => panic!("Expected Select operation in replacement"),
        }
    }

    #[test]
    fn test_rule_macro_with_explicit_type() {
        use crate::testlang::QueryOps;

        // Test new generic form of the macro
        let rule = mk_rule!(QueryOps, "explicit_type_rule", "And(?x, ?y)", "Or(?x, ?y)");

        assert_eq!(rule.name, "explicit_type_rule");

        // Check pattern structure
        match rule.pattern.op() {
            OpOrVar::Op(QueryOps::And) => {}
            _ => panic!("Expected And operation in pattern"),
        }

        // Check replacement structure
        match rule.replacement.op() {
            OpOrVar::Op(QueryOps::Or) => {}
            _ => panic!("Expected Or operation in replacement"),
        }
    }

    #[test]
    fn test_empty_rule_set() {
        let rule_set: Vec<Rule<QueryOps>> = Vec::new();

        assert_eq!(rule_set.rules().len(), 0);
        assert!(rule_set.get_rule(0).is_none());
        assert!(rule_set.get_rule_by_name("any_name").is_none());
    }

    #[test]
    fn test_rule_macro_error_handling() {
        // Test that the macro properly panics on invalid patterns
        // This is expected behavior since we use .expect() in the macro
        let result = std::panic::catch_unwind(|| {
            let _rule: Rule<QueryOps> = mk_rule!("invalid_rule", "And(?a", "Or(?a, ?b)");
            _rule
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_rule_with_same_variable_multiple_times() {
        let rule = mk_rule!("identity_rule", "And(?x, ?x)", "?x");

        assert_eq!(rule.name, "identity_rule");

        // Check pattern: And(?x, ?x)
        match rule.pattern.op() {
            OpOrVar::Op(QueryOps::And) => {}
            _ => panic!("Expected And operation in pattern"),
        }

        // Both arguments should be the same variable
        match rule.pattern.args()[0].op() {
            OpOrVar::Var(var) => assert_eq!(var, "x"),
            _ => panic!("Expected variable x"),
        }

        match rule.pattern.args()[1].op() {
            OpOrVar::Var(var) => assert_eq!(var, "x"),
            _ => panic!("Expected variable x"),
        }

        // Replacement should be ?x
        match rule.replacement.op() {
            OpOrVar::Var(var) => assert_eq!(var, "x"),
            _ => panic!("Expected variable x in replacement"),
        }
    }

    #[test]
    fn test_rule_set_operations_ordering() {
        let mut rule_set: Vec<Rule<QueryOps>> = Vec::new();

        let rule1 = mk_rule!("first", "And(?a, ?b)", "Or(?a, ?b)");
        let rule2 = mk_rule!("second", "Not(Not(?x))", "?x");
        let rule3 = mk_rule!("third", "Or(?x, false)", "?x");

        rule_set.add_rule(rule1);
        rule_set.add_rule(rule2);
        rule_set.add_rule(rule3);

        // Verify order is preserved
        assert_eq!(rule_set.get_rule(0).unwrap().name, "first");
        assert_eq!(rule_set.get_rule(1).unwrap().name, "second");
        assert_eq!(rule_set.get_rule(2).unwrap().name, "third");

        // Remove middle rule
        let rule_to_remove = mk_rule!("second", "Not(Not(?x))", "?x");
        rule_set.remove_rule(&rule_to_remove);

        // Check that ordering is maintained after removal
        assert_eq!(rule_set.rules().len(), 2);
        assert_eq!(rule_set.get_rule(0).unwrap().name, "first");
        assert_eq!(rule_set.get_rule(1).unwrap().name, "third");
    }

    #[test]
    fn test_rule_with_integer_constants() {
        let rule = mk_rule!("constant_folding", "Add(42, 0)", "42");

        assert_eq!(rule.name, "constant_folding");

        // Check pattern: Add(42, 0)
        match rule.pattern.op() {
            OpOrVar::Op(QueryOps::ConstStr(op)) => assert_eq!(op, "Add"),
            _ => panic!("Expected Add operation in pattern"),
        }

        match rule.pattern.args()[0].op() {
            OpOrVar::Op(QueryOps::ConstInt(42)) => {}
            _ => panic!("Expected constant 42"),
        }

        match rule.pattern.args()[1].op() {
            OpOrVar::Op(QueryOps::ConstInt(0)) => {}
            _ => panic!("Expected constant 0"),
        }

        // Check replacement: 42
        match rule.replacement.op() {
            OpOrVar::Op(QueryOps::ConstInt(42)) => {}
            _ => panic!("Expected constant 42 in replacement"),
        }
    }

    #[test]
    fn test_rule_comparison() {
        let rule1 = mk_rule!("test_rule", "And(?a, ?b)", "Or(?a, ?b)");
        let rule2 = mk_rule!("test_rule", "And(?a, ?b)", "Or(?a, ?b)");
        let rule3 = mk_rule!("different_rule", "And(?a, ?b)", "Or(?a, ?b)");

        // Rules with same name should be considered equal for removal purposes
        let mut rule_set: Vec<Rule<QueryOps>> = Vec::new();
        rule_set.add_rule(rule1);
        assert_eq!(rule_set.rules().len(), 1);

        // Remove using a different instance with same name
        rule_set.remove_rule(&rule2);
        assert_eq!(rule_set.rules().len(), 0);

        // Add rule back and try removing with different name
        rule_set.add_rule(mk_rule!("test_rule", "And(?a, ?b)", "Or(?a, ?b)"));
        rule_set.remove_rule(&rule3);
        assert_eq!(rule_set.rules().len(), 1); // Should not be removed
    }
}
