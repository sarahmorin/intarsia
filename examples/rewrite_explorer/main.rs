/// Boolean optimizer example demonstrating cascades-style optimization with ISLE rules.
///
/// This example shows how a complex, redundant boolean expression gets simplified
/// through the optimizer framework applying various boolean algebra rules.

// Include the boolean optimizer module
#[path = "mod.rs"]
mod rewrite_opt;

use egg::RecExpr;
use rewrite_opt::{RewriteLang, RewriteOptimizer};

fn main() {
    println!("=== Rewrite Optimizer Demo ===\n");
    
    // Initialize the logger to see optimization progress
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    // ============================================================
    // Example 1: Simple Op1-Op1 Network Insertion
    // ============================================================
    println!("Example 1: Simple Op1-Op1 Network Insertion");
    println!("----------------------------------------");
    
    let expr1 = create_op1_simple_example();
    demonstrate_optimization("Op1(Op1(Source(1)))", expr1);
    
    // ============================================================
    // Example 2: Simple Op2-Op1 Network Insertion
    // ============================================================

    let expr2 = create_op2_simple_example();
    demonstrate_optimization("Op2(Op1(Source(1)), Op1(Source(2)))", expr2);

    println!("\n");
}

/// Demonstrate optimization of an expression
fn demonstrate_optimization(description: &str, expr: RecExpr<RewriteLang>) {
    println!("Initial expression: {}", description);
    println!("Initial AST: {:?}", expr);
    
    // Count nodes in initial expression
    let initial_size = expr.as_ref().len();
    println!("Initial size: {} nodes", initial_size);
    
    // Create optimizer instance
    let mut optimizer = RewriteOptimizer::new(());
    
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
        println!("✓ Reduced by {} nodes ({:.1}% improvement)", reduction, percent);
    } else if optimized_size == initial_size {
        println!("= Expression already optimal");
    } else {
        println!("Note: Optimized expression is larger (may have explored alternatives)");
    }
}
/// Create expression: (x OR x) AND (y OR y)
/// Should simplify to: x AND y through idempotent laws
fn create_op1_simple_example() -> RecExpr<RewriteLang> {
    let mut expr = RecExpr::default();
    
    // x = true, y = false
    let s = expr.add(RewriteLang::Source(1));
    let op_a = expr.add(RewriteLang::Op1(s));
    expr.add(RewriteLang::Op1(op_a));

    expr
}

fn create_op2_simple_example() -> RecExpr<RewriteLang> {
    let mut expr = RecExpr::default();
    
    // x = true, y = false
    let s_a = expr.add(RewriteLang::Source(1));
    let s_b = expr.add(RewriteLang::Source(2));
    let op_a = expr.add(RewriteLang::Op1(s_a));
    let op_b = expr.add(RewriteLang::Op1(s_b));

    expr.add(RewriteLang::Op2([op_a, op_b]));

    expr
}
