/// Unit tests for the boolean optimizer to diagnose optimization issues
///
/// These tests help identify whether:
/// 1. The optimal expressions exist in the e-graph after exploration
/// 2. The cost function assigns correct costs to expressions
/// 3. The optimization correctly selects the lowest-cost expression
use super::{BoolLang, BoolOptimizer};
use egg::{Id, RecExpr};
use intarsia::framework::property::NoProperty;
use intarsia::{CostDomain, CostFunction, SimpleCost};

/// Helper to build and run optimization on an expression
fn optimize_expr(expr: RecExpr<BoolLang>) -> (BoolOptimizer, Id) {
    let mut optimizer = BoolOptimizer::new(());
    let root_id = optimizer.init(expr);
    optimizer.run(root_id);
    (optimizer, root_id)
}

/// Helper to manually compute cost of a node
fn compute_node_cost(optimizer: &BoolOptimizer, node_id: Id) -> Option<SimpleCost<NoProperty>> {
    let node = optimizer.egraph.get_node(node_id);

    // Build cost closure that looks up child costs from optimizer
    let cost_fn = |child_id: Id| {
        optimizer
            .costs
            .get(&(optimizer.egraph.find(child_id), NoProperty))
            .cloned()
            .unwrap_or_else(|| SimpleCost::simple(usize::MAX))
    };

    Some(optimizer.compute_cost(node, cost_fn))
}

#[test]
fn test_example3_egraph_contents() {
    println!("\n=== Testing Example 3: (NOT(NOT(x)) OR (x OR false)) AND true ===\n");

    // Build the initial expression
    let expr_str = "(AND (OR (NOT (NOT x)) (OR x false)) true)";
    let expr: RecExpr<BoolLang> = expr_str.parse().unwrap();

    let (optimizer, root_id) = optimize_expr(expr);
    let root_eclass = optimizer.egraph.find(root_id);

    println!("Root eclass: {:?}", root_eclass);
    println!("\nAll nodes in root eclass:");
    for (node_id, node) in optimizer.egraph.nodes_in_class(root_eclass) {
        println!("  Node {:?}: {:?}", node_id, node);
        if let Some(cost) = compute_node_cost(&optimizer, node_id) {
            println!("    Cost: {:?}", cost);
        }
    }

    // Check what the optimizer selected as best
    let best_node = optimizer
        .optimized_memo
        .get(&(root_eclass, NoProperty))
        .map(|&id| optimizer.egraph.get_node(id));
    println!("\nOptimizer selected: {:?}", best_node);

    // Check if just Var("x") exists in the eclass
    let has_just_x = optimizer
        .egraph
        .nodes_in_class(root_eclass)
        .any(|(_, node)| matches!(node, BoolLang::Var(s) if s == "x"));
    println!("\nDoes eclass contain just Var(\"x\")? {}", has_just_x);

    // Check if Or(x, x) exists in the eclass
    let has_or_x_x = optimizer
        .egraph
        .nodes_in_class(root_eclass)
        .any(|(_, node)| {
            if let BoolLang::Or([left, right]) = node {
                let left_eclass = optimizer.egraph.find(*left);
                let right_eclass = optimizer.egraph.find(*right);
                left_eclass == right_eclass
            } else {
                false
            }
        });
    println!("Does eclass contain Or(x, x)? {}", has_or_x_x);

    // Look for Var("x") eclass and check if it's the same as root
    for (node_id, node) in optimizer.egraph.nodes_in_class(root_eclass) {
        if let BoolLang::Var(s) = node {
            if s == "x" {
                println!("\nFound Var(\"x\") at node {:?}", node_id);
                let var_eclass = optimizer.egraph.find(node_id);
                println!("  Its eclass: {:?}", var_eclass);
                println!("  Same as root? {}", var_eclass == root_eclass);
            }
        }
    }

    println!("\n=== All costs in memo ===");
    for ((eclass, prop), cost) in &optimizer.costs {
        if *eclass == root_eclass {
            println!("  ({:?}, {:?}): {:?}", eclass, prop, cost);
        }
    }

    println!("\n=== Extraction Result ===");
    let extracted = optimizer.extract(root_id);
    println!("Extracted: {:?}", extracted);
}

#[test]
fn test_example4_egraph_contents() {
    println!("\n=== Testing Example 4: (x AND false) OR (NOT(true) AND y) ===\n");

    // Build the initial expression
    let expr_str = "(OR (AND x false) (AND (NOT true) y))";
    let expr: RecExpr<BoolLang> = expr_str.parse().unwrap();

    let (optimizer, root_id) = optimize_expr(expr);
    let root_eclass = optimizer.egraph.find(root_id);

    println!("Root eclass: {:?}", root_eclass);
    println!("\nAll nodes in root eclass:");
    for (node_id, node) in optimizer.egraph.nodes_in_class(root_eclass) {
        println!("  Node {:?}: {:?}", node_id, node);
        if let Some(cost) = compute_node_cost(&optimizer, node_id) {
            println!("    Cost: {:?}", cost);
        }
    }

    // Check what the optimizer selected as best
    let best_node = optimizer
        .optimized_memo
        .get(&(root_eclass, NoProperty))
        .map(|&id| optimizer.egraph.get_node(id));
    println!("\nOptimizer selected: {:?}", best_node);

    // Check if just Bool(false) exists in the eclass
    let has_just_false = optimizer
        .egraph
        .nodes_in_class(root_eclass)
        .any(|(_, node)| matches!(node, BoolLang::Bool(false)));
    println!("\nDoes eclass contain just Bool(false)? {}", has_just_false);

    // Look at false eclass
    for (eclass_id, eclass) in optimizer.egraph.classes().enumerate() {
        let has_false = eclass
            .nodes
            .iter()
            .any(|node| matches!(node, (_, BoolLang::Bool(false))));
        if has_false {
            println!("\nFound Bool(false) in eclass {:?}", eclass_id);
            println!("  Same as root? {}", Id::from(eclass_id) == root_eclass);
            println!("  All nodes in this eclass:");
            for node in &eclass.nodes {
                println!("    {:?}", node);
            }
        }
    }

    println!("\n=== All costs in memo ===");
    for ((eclass, prop), cost) in &optimizer.costs {
        if *eclass == root_eclass {
            println!("  ({:?}, {:?}): {:?}", eclass, prop, cost);
        }
    }

    println!("\n=== Extraction Result ===");
    let extracted = optimizer.extract(root_id);
    println!("Extracted: {:?}", extracted);
}

#[test]
fn test_cost_function_correctness() {
    println!("\n=== Testing Cost Function ===\n");

    let mut optimizer = BoolOptimizer::new(());

    // Test 1: Cost of a variable
    let expr1_str = "x";
    let expr1: RecExpr<BoolLang> = expr1_str.parse().unwrap();
    let id1 = optimizer.egraph.add_expr(&expr1);
    optimizer.egraph.rebuild();

    // Manually compute cost
    let x_eclass = optimizer.egraph.find(id1);
    let x_node = optimizer.egraph.get_node(id1);
    let cost_closure = |_: Id| SimpleCost::simple(0);
    let x_cost = optimizer.compute_cost(x_node, cost_closure);
    println!("Cost of Var(\"x\"): {:?}", x_cost);

    // Test 2: Cost of Or(x, x)
    let expr2_str = "(OR x x)";
    let expr2: RecExpr<BoolLang> = expr2_str.parse().unwrap();
    let id2 = optimizer.egraph.add_expr(&expr2);
    optimizer.egraph.rebuild();

    let or_eclass = optimizer.egraph.find(id2);
    let or_node_id = optimizer
        .egraph
        .nodes_in_class(or_eclass)
        .find(|(_, n)| matches!(n, BoolLang::Or(_)))
        .map(|(id, _)| id)
        .unwrap();
    let or_node = optimizer.egraph.get_node(or_node_id);

    // Create proper cost closure
    let cost_closure2 = |child: Id| {
        // x should have cost 1 (just the variable itself)
        SimpleCost::simple(1)
    };
    let or_cost = optimizer.compute_cost(or_node, cost_closure2);
    println!("Cost of Or(x, x) with child costs of 1 each: {:?}", or_cost);
    println!("  (Expected: 1 + 1 + 1 = 3)");

    // Test 3: Compare which should be cheaper
    println!("\nComparison:");
    println!("  Var(\"x\") cost: {}", x_cost.cost());
    println!("  Or(x, x) cost: {}", or_cost.cost());
    println!("  Var(\"x\") should be selected: {}", x_cost < or_cost);
}

#[test]
fn test_idempotent_rule_application() {
    println!("\n=== Testing Idempotent Rule Application ===\n");

    // Create Or(x, x) and see if it gets rewritten to x
    let expr_str = "(OR x x)";
    let expr: RecExpr<BoolLang> = expr_str.parse().unwrap();

    let (optimizer, root_id) = optimize_expr(expr);
    let root_eclass = optimizer.egraph.find(root_id);

    println!("All nodes in root eclass:");
    for (node_id, node) in optimizer.egraph.nodes_in_class(root_eclass) {
        println!("  {:?}: {:?}", node_id, node);
    }

    // Check if x is in the same eclass as Or(x, x)
    let has_var_x = optimizer
        .egraph
        .nodes_in_class(root_eclass)
        .any(|(_, node)| matches!(node, BoolLang::Var(s) if s == "x"));

    println!("\nDoes root eclass contain Var(\"x\")? {}", has_var_x);

    if !has_var_x {
        println!("\n❌ BUG: Idempotent rule 'Or(x, x) => x' was not applied!");
        println!("The e-graph should contain both Or(x, x) and Var(\"x\") in the same eclass.");
    } else {
        println!("\n✓ Idempotent rule was applied correctly");
    }

    // Check extraction
    let extracted = optimizer.extract(root_id);
    println!("\nExtracted: {:?}", extracted);

    let is_just_var = extracted.as_ref().len() == 1
        && matches!(&extracted.as_ref()[0], BoolLang::Var(s) if s == "x");

    if !is_just_var {
        println!("❌ BUG: Extracted expression is not just Var(\"x\")");
        println!("Even though the optimal expression exists, it wasn't selected!");
    } else {
        println!("✓ Correctly extracted just Var(\"x\")");
    }
}
