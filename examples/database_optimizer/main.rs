<<<<<<<< HEAD:examples/database_optimizer/main.rs
// Database optimizer example demonstrating the generic framework

// Include all the database optimizer modules as a single unit
#[path = "mod.rs"]
mod db_opt;

use egg::RecExpr;

// Import from the database optimizer module
use db_opt::{DbOptimizer, DbUserData, catalog, language::Optlang, types::DataType};

fn main() {
    // ============================================================
    // Example: Query Optimizer Framework Demonstration
    // ============================================================
    // This example demonstrates a realistic query optimization scenario
    // where we optimize a complex multi-table join query with predicates.

    println!("=== Database Query Optimizer Demo ===\n");

    // ============================================================
    // Step 1: Create Database Catalog
    // ============================================================
    // Define catalog with tables and indices representing an e-commerce database

    println!("Step 1: Creating database catalog...");
    let mut catalog = catalog::Catalog::new();

    // Create CUSTOMERS table (10,000 customers)
    let customers_id = catalog
        .create_table_with_cols(
            "customers".to_string(),
            vec![
                ("customer_id".to_string(), DataType::Int),
                ("name".to_string(), DataType::String),
                ("email".to_string(), DataType::String),
                ("age".to_string(), DataType::Int),
                ("country".to_string(), DataType::String),
            ],
        )
        .expect("Failed to create customers table");

    catalog
        .tables
        .get_mut(&customers_id)
        .unwrap()
        .set_est_num_rows(10_000);

    // Create ORDERS table (50,000 orders)
    let orders_id = catalog
        .create_table_with_cols(
            "orders".to_string(),
            vec![
                ("order_id".to_string(), DataType::Int),
                ("customer_id".to_string(), DataType::Int),
                ("order_date".to_string(), DataType::String),
                ("total_amount".to_string(), DataType::Int),
                ("status".to_string(), DataType::String),
            ],
        )
        .expect("Failed to create orders table");

    catalog
        .tables
        .get_mut(&orders_id)
        .unwrap()
        .set_est_num_rows(50_000);

    // Create PRODUCTS table (5,000 products)
    let products_id = catalog
        .create_table_with_cols(
            "products".to_string(),
            vec![
                ("product_id".to_string(), DataType::Int),
                ("name".to_string(), DataType::String),
                ("price".to_string(), DataType::Int),
                ("category".to_string(), DataType::String),
            ],
        )
        .expect("Failed to create products table");

    catalog
        .tables
        .get_mut(&products_id)
        .unwrap()
        .set_est_num_rows(5_000);

    // Create ORDER_ITEMS table (150,000 line items)
    let order_items_id = catalog
        .create_table_with_cols(
            "order_items".to_string(),
            vec![
                ("item_id".to_string(), DataType::Int),
                ("order_id".to_string(), DataType::Int),
                ("product_id".to_string(), DataType::Int),
                ("quantity".to_string(), DataType::Int),
            ],
        )
        .expect("Failed to create order_items table");

    catalog
        .tables
        .get_mut(&order_items_id)
        .unwrap()
        .set_est_num_rows(150_000);

    // Create indices to enable index scans
    let _customers_idx = catalog
        .create_table_index(
            Some("idx_customers_id".to_string()),
            "customers".to_string(),
            vec!["customer_id".to_string()],
        )
        .expect("Failed to create customers index");

    let _orders_cust_idx = catalog
        .create_table_index(
            Some("idx_orders_customer".to_string()),
            "orders".to_string(),
            vec!["customer_id".to_string()],
        )
        .expect("Failed to create orders customer index");

    let _order_items_order_idx = catalog
        .create_table_index(
            Some("idx_order_items_order".to_string()),
            "order_items".to_string(),
            vec!["order_id".to_string()],
        )
        .expect("Failed to create order_items order index");

    println!("  ✓ Created 4 tables: customers, orders, products, order_items");
    println!("  ✓ Created 3 indices for join optimization");
    println!("  ✓ Total estimated rows: 215,000\n");

    // ============================================================
    // Step 2: Construct Initial Query Expression
    // ============================================================
    // Build a complex query that finds high-value orders for young customers:
    //
    // SELECT *
    // FROM customers
    // JOIN orders ON customers.customer_id = orders.customer_id
    // JOIN order_items ON orders.order_id = order_items.order_id
    // WHERE customers.age < 30
    //   AND orders.total_amount > (1000 + 500)  -- Nested arithmetic
    //
    // Initial plan: Logical operators, no optimization applied yet

    println!("Step 2: Building initial query expression...");

    use Optlang;
    let mut initial_expr = RecExpr::default();

    // Data sources (tables)
    let customers_table = initial_expr.add(Optlang::Table(customers_id));
    let customers_scan = initial_expr.add(Optlang::Scan(customers_table));

    let orders_table = initial_expr.add(Optlang::Table(orders_id));
    let orders_scan = initial_expr.add(Optlang::Scan(orders_table));

    let order_items_table = initial_expr.add(Optlang::Table(order_items_id));
    let order_items_scan = initial_expr.add(Optlang::Scan(order_items_table));

    // First join: customers JOIN orders
    let join_pred1 = initial_expr.add(Optlang::Bool(true)); // Simplified join condition
    let join1 = initial_expr.add(Optlang::Join([customers_scan, orders_scan, join_pred1]));

    // Second join: (customers JOIN orders) JOIN order_items
    let join_pred2 = initial_expr.add(Optlang::Bool(true));
    let join2 = initial_expr.add(Optlang::Join([join1, order_items_scan, join_pred2]));

    // Build complex predicate: age < 30 AND total_amount > (1000 + 500)

    // Predicate 1: customers.age < 30
    let age_ref = initial_expr.add(Optlang::Int(3)); // Column index for age
    let age_threshold = initial_expr.add(Optlang::Int(30));
    let age_predicate = initial_expr.add(Optlang::Lt([age_ref, age_threshold]));

    // Predicate 2: orders.total_amount > (1000 + 500)
    // First compute: 1000 + 500 = 1500
    let base_amount = initial_expr.add(Optlang::Int(1000));
    let bonus_amount = initial_expr.add(Optlang::Int(500));
    let computed_threshold = initial_expr.add(Optlang::Add([base_amount, bonus_amount]));

    let amount_ref = initial_expr.add(Optlang::Int(3)); // Column index for total_amount in orders
    let amount_predicate = initial_expr.add(Optlang::Gt([amount_ref, computed_threshold]));

    // Combine predicates: age_predicate AND amount_predicate
    let combined_predicate = initial_expr.add(Optlang::And([age_predicate, amount_predicate]));

    // Apply selection filter
    let _final_select = initial_expr.add(Optlang::Select([join2, combined_predicate]));

    println!("  ✓ Query structure:");
    println!("    - 2 nested joins (3 tables total)");
    println!("    - Complex predicate: age < 30 AND total_amount > (1000 + 500)");
    println!("    - Uses nested arithmetic expression (addition)");
    println!("    - Starting with logical operators (not physical)\n");

    // ============================================================
    // Step 3: Initialize Optimizer
    // ============================================================

    println!("Step 3: Initializing optimizer...");

    // Create DbUserData
    let user_data = DbUserData::new(catalog);

    // Create optimizer using the generic framework
    let mut optimizer = DbOptimizer::new(user_data);

    // Add the initial expression to the e-graph
    let root_id = optimizer.init(initial_expr);

    println!("  ✓ Initial expression added to e-graph");
    println!(
        "  ✓ E-graph initialized with {} nodes\n",
        optimizer.egraph.total_number_of_nodes()
    );

    // Note: We extract the initial plan after optimization for comparison
    // since we don't have Clone on OptimizerFramework

    // ============================================================
    // Step 4: Run Optimizer
    // ============================================================
    // The optimizer will:
    //   - Explore equivalent expressions (e.g., join reordering)
    //   - Convert logical operators to physical implementations
    //   - Consider index scans vs table scans
    //   - Push down selections close to base tables
    //   - Choose optimal join algorithms (nested loop, hash join, merge join)

    println!("Step 4: Running optimizer...");
    println!("  [Exploring equivalent expressions and finding optimal plan]");

    optimizer.run(root_id);

    println!("  ✓ Optimization complete");
    println!(
        "  ✓ E-graph now contains {} nodes (explored alternatives)\n",
        optimizer.egraph.total_number_of_nodes()
    );

    // ============================================================
    // Step 5: Extract Optimized Plan
    // ============================================================
    // Extract the lowest-cost plan from the e-graph

    println!("Step 5: Extracting optimized query plan...");
    let (optimized_cost, optimized_plan) = optimizer.extract_with_cost(root_id);

    println!("Optimized Plan (after optimization):");
    println!("  Expression: {}", optimized_plan);
    println!("  Cost: {:?}\n", optimized_cost);

    // ============================================================
    // Step 6: Summary
    // ============================================================

    println!("=== Optimization Summary ===");
    println!("Optimized cost:  {:?}", optimized_cost);
    println!("Optimized plan has been selected from explored alternatives\n");

    println!("✓ Demo complete!");
}
========
fn main() {}
>>>>>>>> d44f82f (generalized framework):src/main.rs
