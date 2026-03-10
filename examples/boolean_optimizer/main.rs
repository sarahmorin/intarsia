/// Boolean optimizer example demonstrating cascades-style optimization with ISLE rules.
///
/// This example shows how a complex, redundant boolean expression gets simplified
/// through the optimizer framework applying various boolean algebra rules.

// Include the boolean optimizer module
#[path = "mod.rs"]
mod bool_opt;

use bool_opt::{BoolLang, BoolOptimizer};
use egg::RecExpr;

fn main() {
    println!("=== Boolean Expression Optimizer Demo ===\n");

    // Initialize the logger to see optimization progress
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    // ============================================================
    // Example 1: Double Negation and Identity Laws
    // ============================================================
    println!("Example 1: Double Negation Elimination");
    println!("----------------------------------------");

    // Create expression: NOT(NOT(x)) AND true
    // This should simplify to: x
    let expr1 = create_double_negation();
    demonstrate_optimization("NOT(NOT(x)) AND true", expr1);

    println!("\n");

    // ============================================================
    // Example 2: De Morgan's Laws
    // ============================================================
    println!("Example 2: De Morgan's Law Application");
    println!("----------------------------------------");

    // Create expression: NOT(AND(x, y))
    // This should transform to: OR(NOT(x), NOT(y))
    let expr2 = create_demorgans_example();
    demonstrate_optimization("NOT(AND(x, y))", expr2);

    println!("\n");

    // ============================================================
    // Example 3: Complex Redundant Expression
    // ============================================================
    println!("Example 3: Complex Redundant Expression");
    println!("----------------------------------------");

    // Create expression: (NOT(NOT(x)) OR (x OR false)) AND true
    // This should simplify through multiple steps to: x
    let expr3 = create_complex_redundant();
    demonstrate_optimization("(NOT(NOT(x)) OR (x OR false)) AND true", expr3);

    println!("\n");

    // ============================================================
    // Example 4: Constant Folding
    // ============================================================
    println!("Example 4: Constant Folding and Annihilation");
    println!("----------------------------------------");

    // Create expression: (x AND false) OR (NOT(true) AND y)
    // This should simplify to: false
    let expr4 = create_constant_folding();
    demonstrate_optimization("(x AND false) OR (NOT(true) AND y)", expr4);

    println!("\n");

    // ============================================================
    // Example 5: Idempotent Laws
    // ============================================================
    println!("Example 5: Idempotent Law Application");
    println!("----------------------------------------");

    // Create expression: (x OR x) AND (y OR y)
    // This should simplify to: x AND y
    let expr5 = create_idempotent_example();
    demonstrate_optimization("(x OR x) AND (y OR y)", expr5);

    println!("\n=== All Examples Complete ===");
}

/// Demonstrate optimization of an expression
fn demonstrate_optimization(description: &str, expr: RecExpr<BoolLang>) {
    println!("Initial expression: {}", description);
    println!("Initial AST: {:?}", expr);

    // Count nodes in initial expression
    let initial_size = expr.as_ref().len();
    println!("Initial size: {} nodes", initial_size);

    // Create optimizer instance
    let mut optimizer = BoolOptimizer::new(());

    // Initialize optimizer with the expression
    let root_id = optimizer.init(expr);

    println!("\nRunning optimization...");

    // Run the optimizer
    optimizer.run(root_id);

    println!("Optimization complete!\n");

    // Extract the optimized expression
    let optimized = optimizer.extract(root_id);

    // Count nodes in optimized expression
    let optimized_size = optimized.as_ref().len();

    println!("Optimized AST: {:?}", optimized);
    println!("Optimized size: {} nodes", optimized_size);

    // Show improvement
    if optimized_size < initial_size {
        let reduction = initial_size - optimized_size;
        let percent = (reduction as f64 / initial_size as f64) * 100.0;
        println!(
            "✓ Reduced by {} nodes ({:.1}% improvement)",
            reduction, percent
        );
    } else if optimized_size == initial_size {
        println!("= Expression already optimal");
    } else {
        println!("Note: Optimized expression is larger (may have explored alternatives)");
    }
}

/// Create expression: NOT(NOT(x)) AND true
/// Should simplify to: x
fn create_double_negation() -> RecExpr<BoolLang> {
    let expr_str = "(AND true (NOT (NOT x)))";
    let expr: RecExpr<BoolLang> = expr_str.parse().unwrap();
    expr
}

/// Create expression: NOT(AND(x, y))
/// Should transform to: OR(NOT(x), NOT(y)) via De Morgan's law
fn create_demorgans_example() -> RecExpr<BoolLang> {
    let expr_str = "(NOT (AND x y))";
    let expr: RecExpr<BoolLang> = expr_str.parse().unwrap();
    expr
}

/// Create expression: (NOT(NOT(x)) OR (x OR false)) AND true
/// Should simplify to: x through multiple optimization steps
fn create_complex_redundant() -> RecExpr<BoolLang> {
    let expr_str = "(AND (OR (NOT (NOT x)) (OR x false)) true)";
    let expr: RecExpr<BoolLang> = expr_str.parse().unwrap();
    expr
}

/// Create expression: (x AND false) OR (NOT(true) AND y)
/// Should simplify to: false through constant folding
fn create_constant_folding() -> RecExpr<BoolLang> {
    let expr_str = "(OR (AND x false) (AND (NOT true) y))";
    let expr: RecExpr<BoolLang> = expr_str.parse().unwrap();
    expr
}

/// Create expression: (x OR x) AND (y OR y)
/// Should simplify to: x AND y through idempotent laws
fn create_idempotent_example() -> RecExpr<BoolLang> {
    let expr_str = "(AND (OR x x) (OR y y))";
    let expr: RecExpr<BoolLang> = expr_str.parse().unwrap();
    expr
}
