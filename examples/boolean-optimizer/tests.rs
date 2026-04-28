/// Unit tests for the boolean optimizer to diagnose optimization issues.
///
/// These tests help identify whether:
/// 1. The optimal expressions exist in the e-graph after exploration
/// 2. The cost function assigns correct costs to expressions
/// 3. The optimization correctly selects the lowest-cost expression
use super::{BoolCost, BoolLang, BoolOptimizer, new_optimizer};
use egg::{Id, Language, RecExpr};
use intarsia::framework::{SimpleCostData, SimpleCostFn};

/// Helper to build and run optimization on an expression.
fn optimize_expr(expr: RecExpr<BoolLang>) -> (BoolOptimizer, Id) {
    let mut optimizer = new_optimizer();
    let root_id = optimizer.init(expr);
    optimizer.run(root_id);
    (optimizer, root_id)
}

/// Manually compute the cost a node *would* receive given its children's class data.
/// Mirrors the optimizer's phase-3 cost lookup.
fn compute_node_cost(optimizer: &BoolOptimizer, node_id: Id) -> Option<usize> {
    let node = optimizer.egraph.get_node(node_id);
    let children: Vec<&SimpleCostData<usize, ()>> = node
        .children()
        .iter()
        .map(|&c| &optimizer.egraph[c].data)
        .collect();
    BoolCost.cost(node, &children)
}

#[test]
fn test_example3_egraph_contents() {
    println!("\n=== Testing Example 3: (NOT(NOT(x)) OR (x OR false)) AND true ===\n");

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

    // Read winner directly from analysis data.
    let data = &optimizer.egraph[root_eclass].data;
    let best_node = data.winner.as_ref().map(|w| optimizer.egraph.get_node(w.node_id));
    println!("\nOptimizer selected: {:?}", best_node);

    let has_just_x = optimizer
        .egraph
        .nodes_in_class(root_eclass)
        .any(|(_, node)| matches!(node, BoolLang::Var(s) if s == "x"));
    println!("\nDoes eclass contain just Var(\"x\")? {}", has_just_x);

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

    for (node_id, node) in optimizer.egraph.nodes_in_class(root_eclass) {
        if let BoolLang::Var(s) = node
            && s == "x"
        {
            println!("\nFound Var(\"x\") at node {:?}", node_id);
            let var_eclass = optimizer.egraph.find(node_id);
            println!("  Its eclass: {:?}", var_eclass);
            println!("  Same as root? {}", var_eclass == root_eclass);
        }
    }

    println!("\n=== Recorded node costs in root class ===");
    for (nid, c) in &optimizer.egraph[root_eclass].data.node_costs {
        println!("  node {:?}: cost {}", nid, c);
    }

    println!("\n=== Extraction Result ===");
    let extracted = optimizer.extract(root_id);
    println!("Extracted: {:?}", extracted);
}

#[test]
fn test_example4_egraph_contents() {
    println!("\n=== Testing Example 4: (x AND false) OR (NOT(true) AND y) ===\n");

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

    let data = &optimizer.egraph[root_eclass].data;
    let best_node = data.winner.as_ref().map(|w| optimizer.egraph.get_node(w.node_id));
    println!("\nOptimizer selected: {:?}", best_node);

    let has_just_false = optimizer
        .egraph
        .nodes_in_class(root_eclass)
        .any(|(_, node)| matches!(node, BoolLang::Bool(false)));
    println!("\nDoes eclass contain Bool(false)? {}", has_just_false);

    println!("\n=== Recorded node costs in root class ===");
    for (nid, c) in &optimizer.egraph[root_eclass].data.node_costs {
        println!("  node {:?}: cost {}", nid, c);
    }

    println!("\n=== Extraction Result ===");
    let extracted = optimizer.extract(root_id);
    println!("Extracted: {:?}", extracted);
}

#[test]
fn test_cost_function_correctness() {
    println!("\n=== Testing Cost Function ===\n");

    let mut optimizer = new_optimizer();

    // Cost of a leaf: 1 (no children).
    let expr1: RecExpr<BoolLang> = "x".parse().unwrap();
    let id1 = optimizer.egraph.add_expr(&expr1);
    optimizer.egraph.rebuild();
    let x_cost = compute_node_cost(&optimizer, id1).unwrap();
    println!("Cost of Var(\"x\"): {}", x_cost);
    assert_eq!(x_cost, 1, "leaf cost should be 1");

    // Cost of Or(x, x): with x's class data filled in (winner cost 1 each), should be 1 + 1 + 1 = 3.
    // But we haven't run the optimizer, so x's class has no recorded winner — children will look
    // up cost as 0. So the formula yields 1 (just self).
    let expr2: RecExpr<BoolLang> = "(OR x x)".parse().unwrap();
    let id2 = optimizer.egraph.add_expr(&expr2);
    optimizer.egraph.rebuild();
    let or_eclass = optimizer.egraph.find(id2);
    let or_node_id = optimizer
        .egraph
        .nodes_in_class(or_eclass)
        .find(|(_, n)| matches!(n, BoolLang::Or(_)))
        .map(|(id, _)| id)
        .unwrap();
    let or_cost_no_winners = compute_node_cost(&optimizer, or_node_id).unwrap();
    println!(
        "Cost of Or(x, x) before optimization (no child winners): {}",
        or_cost_no_winners
    );

    // Now run the optimizer; x's winner gets recorded. Re-cost Or(x, x): should be 3.
    let root_id = id2;
    optimizer.run(root_id);
    let or_cost_after = compute_node_cost(&optimizer, or_node_id);
    println!("Cost of Or(x, x) after optimization: {:?}", or_cost_after);
}

#[test]
fn test_idempotent_rule_application() {
    println!("\n=== Testing Idempotent Rule Application ===\n");

    let expr_str = "(OR x x)";
    let expr: RecExpr<BoolLang> = expr_str.parse().unwrap();

    let (optimizer, root_id) = optimize_expr(expr);
    let root_eclass = optimizer.egraph.find(root_id);

    println!("All nodes in root eclass:");
    for (node_id, node) in optimizer.egraph.nodes_in_class(root_eclass) {
        println!("  {:?}: {:?}", node_id, node);
    }

    let has_var_x = optimizer
        .egraph
        .nodes_in_class(root_eclass)
        .any(|(_, node)| matches!(node, BoolLang::Var(s) if s == "x"));

    println!("\nDoes root eclass contain Var(\"x\")? {}", has_var_x);
    assert!(has_var_x, "ISLE rule Or(x, x) => x should have been applied");

    let extracted = optimizer.extract(root_id);
    println!("\nExtracted: {:?}", extracted);

    let is_just_var = extracted.as_ref().len() == 1
        && matches!(&extracted.as_ref()[0], BoolLang::Var(s) if s == "x");
    assert!(is_just_var, "extraction should pick Var(\"x\") as the cheaper alternative");
}
