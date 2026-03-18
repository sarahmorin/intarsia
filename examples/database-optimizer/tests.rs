use intarsia::{CostDomain, CostFunction};
use log::{debug, info};

use super::catalog::Catalog;
use super::cost::DbCost;
use super::language::Optlang;
use super::types::{ColSetId, DataType, IndexId, TableId};
use super::{CPU_COST, DbOptimizer, DbUserData, IO_COST, TRANSFER_COST};
use egg::{CostFunction as EggCostFunction, Extractor, Id, RecExpr};

/// Wrapper struct to adapt the framework's CostFunction to egg's CostFunction trait.
/// This allows us to use egg's Extractor directly with our cost function for testing.
struct CostFunctionAdapter<'a> {
    optimizer: &'a DbOptimizer,
}

impl<'a> EggCostFunction<Optlang> for CostFunctionAdapter<'a> {
    type Cost = DbCost;

    fn cost<C>(&mut self, enode: &Optlang, costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        // Use the optimizer's compute_cost method directly
        // The optimizer's cost function already returns DbCost
        self.optimizer.compute_cost(enode, costs)
    }
}

/// Helper function to extract an expression with cost without running optimization.
/// This uses egg's Extractor directly with the optimizer's cost function.
fn extract_with_cost_only(ctx: &DbOptimizer, id: Id) -> (usize, RecExpr<Optlang>) {
    let cost_fn = CostFunctionAdapter { optimizer: ctx };
    let extractor = Extractor::new(&ctx.egraph, cost_fn);
    let (db_cost, expr) = extractor.find_best(id);
    // Return just the numeric cost part
    (db_cost.cost(), expr)
}

/// Initialize the logger for tests at debug level
fn init_logger() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();
}

/// Generate a diagram of the e-graph for debugging purposes
fn generate_egraph_diagram(ctx: &DbOptimizer, filename: &str) {
    info!("Generating e-graph diagram to {}", filename);
    ctx.egraph
        .dot()
        .to_png(format!("target/{}.png", filename))
        .expect("Failed to generate e-graph diagram");
}

/// Helper function to create a test catalog with some sample tables
fn create_test_catalog() -> Catalog {
    let mut catalog = Catalog::new();

    // Create a test table with a few columns
    catalog
        .create_table_with_cols(
            "users".to_string(),
            vec![
                ("id".to_string(), DataType::Int),
                ("name".to_string(), DataType::String),
                ("age".to_string(), DataType::Int),
            ],
        )
        .unwrap();

    catalog
}

/// Helper function to create a RecExpr with a Table node
/// The parser can't distinguish between Int(n) and Table(n), so we need to construct these manually
fn make_table_expr(table_id: TableId) -> RecExpr<Optlang> {
    let mut expr = RecExpr::default();
    expr.add(Optlang::Table(table_id));
    expr
}

/// Helper function to create a RecExpr with a TableScan node referencing a Table
fn make_table_scan_expr(table_id: TableId) -> RecExpr<Optlang> {
    let mut expr = RecExpr::default();
    let table_node_id = expr.add(Optlang::Table(table_id));
    expr.add(Optlang::TableScan(table_node_id));
    expr
}

/// Helper function to create a RecExpr with an Index node
fn make_index_expr(index_id: IndexId) -> RecExpr<Optlang> {
    let mut expr = RecExpr::default();
    expr.add(Optlang::Index(index_id));
    expr
}

/// Helper function to create a RecExpr with an IndexScan node referencing an Index
fn make_index_scan_expr(index_id: IndexId) -> RecExpr<Optlang> {
    let mut expr = RecExpr::default();
    let index_node_id = expr.add(Optlang::Index(index_id));
    expr.add(Optlang::IndexScan(index_node_id));
    expr
}

/// Helper function to create a RecExpr with a ColSet node
fn make_colset_expr(colset_id: ColSetId) -> RecExpr<Optlang> {
    let mut expr = RecExpr::default();
    expr.add(Optlang::ColSet(colset_id));
    expr
}

#[test]
fn test_create_context() {
    init_logger();
    let catalog = Catalog::new();
    let ctx = DbOptimizer::new(DbUserData::new(catalog));
    assert_eq!(ctx.egraph.total_number_of_nodes(), 0);
}

#[test]
fn test_catalog_loaded_correctly() {
    init_logger();

    // Create a complex catalog with multiple tables and indexes
    let mut catalog = Catalog::new();

    // Create users table
    let users_table_id = catalog
        .create_table_with_cols(
            "users".to_string(),
            vec![
                ("user_id".to_string(), DataType::Int),
                ("username".to_string(), DataType::String),
                ("email".to_string(), DataType::String),
                ("age".to_string(), DataType::Int),
                ("created_at".to_string(), DataType::String),
            ],
        )
        .expect("Failed to create users table");

    // Create orders table
    let orders_table_id = catalog
        .create_table_with_cols(
            "orders".to_string(),
            vec![
                ("order_id".to_string(), DataType::Int),
                ("user_id".to_string(), DataType::Int),
                ("product_name".to_string(), DataType::String),
                ("quantity".to_string(), DataType::Int),
                ("price".to_string(), DataType::Int),
                ("status".to_string(), DataType::String),
            ],
        )
        .expect("Failed to create orders table");

    // Create products table
    let products_table_id = catalog
        .create_table_with_cols(
            "products".to_string(),
            vec![
                ("product_id".to_string(), DataType::Int),
                ("name".to_string(), DataType::String),
                ("category".to_string(), DataType::String),
                ("in_stock".to_string(), DataType::Bool),
            ],
        )
        .expect("Failed to create products table");

    // Create indexes
    let users_id_index = catalog
        .create_table_index(
            Some("idx_users_id".to_string()),
            "users".to_string(),
            vec!["user_id".to_string()],
        )
        .expect("Failed to create users id index");

    let orders_user_index = catalog
        .create_table_index(
            None, // Let catalog generate the name
            "orders".to_string(),
            vec!["user_id".to_string()],
        )
        .expect("Failed to create orders user_id index");

    let products_name_index = catalog
        .create_table_index(
            Some("idx_products_name".to_string()),
            "products".to_string(),
            vec!["name".to_string()],
        )
        .expect("Failed to create products name index");

    // Create optimizer context with the catalog
    let ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Verify that all tables are present in the catalog
    assert_eq!(
        ctx.user_data.catalog.table_ids.len(),
        3,
        "Should have 3 tables"
    );
    assert_eq!(
        ctx.user_data.catalog.tables.len(),
        3,
        "Should have 3 table entries"
    );

    // Verify table IDs are correct
    assert_eq!(users_table_id, 1, "users should have table_id 1");
    assert_eq!(orders_table_id, 2, "orders should have table_id 2");
    assert_eq!(products_table_id, 3, "products should have table_id 3");

    // Verify table names exist
    assert!(ctx.user_data.catalog.table_ids.contains_key("users"));
    assert!(ctx.user_data.catalog.table_ids.contains_key("orders"));
    assert!(ctx.user_data.catalog.table_ids.contains_key("products"));

    // Verify table IDs map correctly
    assert_eq!(ctx.user_data.catalog.table_ids.get("users"), Some(&1));
    assert_eq!(ctx.user_data.catalog.table_ids.get("orders"), Some(&2));
    assert_eq!(ctx.user_data.catalog.table_ids.get("products"), Some(&3));

    // Verify users table structure
    let users = ctx
        .user_data
        .catalog
        .get_table("users")
        .expect("users table should exist");
    assert_eq!(users.id, users_table_id);
    assert_eq!(users.name, "users");
    assert_eq!(users.num_columns(), 5, "users should have 5 columns");
    assert!(users.get_column("user_id").is_some());
    assert!(users.get_column("username").is_some());
    assert!(users.get_column("email").is_some());
    assert!(users.get_column("age").is_some());
    assert!(users.get_column("created_at").is_some());

    // Verify orders table structure
    let orders = ctx
        .user_data
        .catalog
        .get_table("orders")
        .expect("orders table should exist");
    assert_eq!(orders.id, orders_table_id);
    assert_eq!(orders.name, "orders");
    assert_eq!(orders.num_columns(), 6, "orders should have 6 columns");
    assert!(orders.get_column("order_id").is_some());
    assert!(orders.get_column("user_id").is_some());
    assert!(orders.get_column("product_name").is_some());

    // Verify products table structure
    let products = ctx
        .user_data
        .catalog
        .get_table("products")
        .expect("products table should exist");
    assert_eq!(products.id, products_table_id);
    assert_eq!(products.name, "products");
    assert_eq!(products.num_columns(), 4, "products should have 4 columns");
    assert!(products.get_column("product_id").is_some());
    assert!(products.get_column("name").is_some());
    assert!(products.get_column("in_stock").is_some());

    // Verify indexes are present
    assert_eq!(
        ctx.user_data.catalog.index_ids.len(),
        3,
        "Should have 3 indexes"
    );
    assert_eq!(
        ctx.user_data.catalog.indexes.len(),
        3,
        "Should have 3 index entries"
    );

    // Verify index IDs are correct
    assert_eq!(users_id_index, 1, "users_id index should have index_id 1");
    assert_eq!(
        orders_user_index, 2,
        "orders_user index should have index_id 2"
    );
    assert_eq!(
        products_name_index, 3,
        "products_name index should have index_id 3"
    );

    // Verify index names exist
    assert!(ctx.user_data.catalog.index_ids.contains_key("idx_users_id"));
    assert!(
        ctx.user_data
            .catalog
            .index_ids
            .contains_key("orders_user_id")
    ); // Auto-generated name
    assert!(
        ctx.user_data
            .catalog
            .index_ids
            .contains_key("idx_products_name")
    );

    // Verify index structures
    let users_idx = ctx
        .user_data
        .catalog
        .get_index("idx_users_id")
        .expect("users index should exist");
    assert_eq!(users_idx.id, users_id_index);
    assert_eq!(users_idx.table_id, users_table_id);
    assert_eq!(users_idx.column_ids.len(), 1);

    let orders_idx = ctx
        .user_data
        .catalog
        .get_index("orders_user_id")
        .expect("orders index should exist");
    assert_eq!(orders_idx.id, orders_user_index);
    assert_eq!(orders_idx.table_id, orders_table_id);
    assert_eq!(orders_idx.column_ids.len(), 1);

    let products_idx = ctx
        .user_data
        .catalog
        .get_index("idx_products_name")
        .expect("products index should exist");
    assert_eq!(products_idx.id, products_name_index);
    assert_eq!(products_idx.table_id, products_table_id);
    assert_eq!(products_idx.column_ids.len(), 1);

    // Verify the e-graph is empty (no expressions added yet)
    assert_eq!(ctx.egraph.total_number_of_nodes(), 0);

    // Verify colsets is initially empty
    assert_eq!(ctx.user_data.colsets.len(), 0);
    assert_eq!(ctx.user_data.next_colset_id, 1);

    // Note: task_stack, explored_groups, optimized_groups are now private fields
    // and cannot be accessed directly in tests
}

#[test]
fn test_init_simple_expression() {
    init_logger();
    let catalog = Catalog::new();
    let mut ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Create a simple arithmetic expression: 1 + 2
    let expr: RecExpr<Optlang> = "(+ 1 2)".parse().unwrap();
    let id = ctx.init(expr);

    // The e-graph should now contain nodes
    assert!(ctx.egraph.total_number_of_nodes() > 0);
}

#[test]
fn test_init_and_extract_identity() {
    init_logger();
    let catalog = Catalog::new();
    let mut ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Create a simple constant expression
    let expr: RecExpr<Optlang> = "42".parse().unwrap();
    let id = ctx.init(expr.clone());

    // Extract without running optimization - should get back the same expression
    let result = ctx.extract(id);
    assert_eq!(result.to_string(), "42");
}

#[test]
fn test_arithmetic_expression() {
    init_logger();
    let catalog = Catalog::new();
    let mut ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Create an arithmetic expression: (1 + 2) * 3
    let expr: RecExpr<Optlang> = "(* (+ 1 2) 3)".parse().unwrap();
    let id = ctx.init(expr);

    // Extract the expression
    let result = ctx.extract(id);
    assert_eq!(result.to_string(), "(* (+ 1 2) 3)");
}

#[test]
fn test_comparison_operations() {
    init_logger();
    let catalog = Catalog::new();
    let mut ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Test equality comparison: 5 == 5
    let expr: RecExpr<Optlang> = "(== 5 5)".parse().unwrap();
    let id = ctx.init(expr);
    let result = ctx.extract(id);
    assert_eq!(result.to_string(), "(== 5 5)");

    // Test less than: 3 < 7
    let mut ctx2 = DbOptimizer::new(DbUserData::new(Catalog::new()));
    let expr2: RecExpr<Optlang> = "(< 3 7)".parse().unwrap();
    let id2 = ctx2.init(expr2);
    let result2 = ctx2.extract(id2);
    assert_eq!(result2.to_string(), "(< 3 7)");
}

#[test]
fn test_logical_operations() {
    init_logger();
    let catalog = Catalog::new();
    let mut ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Test AND operation: true AND false
    let expr: RecExpr<Optlang> = "(AND true false)".parse().unwrap();
    let id = ctx.init(expr);
    let result = ctx.extract(id);
    assert_eq!(result.to_string(), "(AND true false)");

    // Test NOT operation
    let mut ctx2 = DbOptimizer::new(DbUserData::new(Catalog::new()));
    let expr2: RecExpr<Optlang> = "(NOT true)".parse().unwrap();
    let id2 = ctx2.init(expr2);
    let result2 = ctx2.extract(id2);
    assert_eq!(result2.to_string(), "(NOT true)");
}

#[test]
fn test_run_optimization() {
    init_logger();
    let catalog = Catalog::new();
    let mut ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Create a simple expression and run optimization
    let expr: RecExpr<Optlang> = "(+ 1 2)".parse().unwrap();
    let id = ctx.init(expr);

    // Run optimization - this should explore and optimize the expression
    ctx.run(id);

    // The e-graph should still be valid after running
    assert!(ctx.egraph.total_number_of_nodes() > 0);
}

#[test]
fn test_nested_expressions() {
    init_logger();
    let catalog = Catalog::new();
    let mut ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Create a nested expression: ((1 + 2) * (3 - 4))
    let expr: RecExpr<Optlang> = "(* (+ 1 2) (- 3 4))".parse().unwrap();
    let id = ctx.init(expr);

    ctx.run(id);
    let result = ctx.extract(id);

    // Should successfully extract some expression
    assert!(result.as_ref().len() > 0);
}

#[test]
fn test_table_expression() {
    init_logger();
    let catalog = create_test_catalog();
    let mut ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Get the table ID for "users" - it should be 1 (first table created)
    let table_id = *ctx.user_data.catalog.table_ids.get("users").unwrap();

    // Create a TableScan expression programmatically
    // Note: We can't use the parser because it can't distinguish between Int(1) and Table(1)
    let expr = make_table_scan_expr(table_id);

    println!("Initial expression: {}", expr);
    let id = ctx.init(expr);

    ctx.run(id);
    let result = ctx.extract(id);

    // Should successfully extract the table scan expression
    assert!(result.as_ref().len() > 0);
    assert!(result.to_string().contains("TABLE_SCAN"));
}

#[test]
fn test_complex_logical_expression() {
    init_logger();
    let catalog = Catalog::new();
    let mut ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Create a complex logical expression: (a > 5) AND (b < 10)
    let expr: RecExpr<Optlang> = "(AND (> 10 5) (< 3 10))".parse().unwrap();
    let id = ctx.init(expr);

    ctx.run(id);
    let result = ctx.extract(id);

    assert!(result.to_string().contains("AND"));
}

#[test]
fn test_multiple_operations() {
    init_logger();
    let catalog = Catalog::new();
    let mut ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Test that we can add multiple expressions to the same context
    let expr1: RecExpr<Optlang> = "(+ 1 2)".parse().unwrap();
    let id1 = ctx.init(expr1);

    let expr2: RecExpr<Optlang> = "(* 3 4)".parse().unwrap();
    let id2 = ctx.egraph.add_expr(&expr2);

    // Both expressions should be in the e-graph
    assert!(ctx.egraph.total_number_of_nodes() > 0);

    // Should be able to extract both
    let result1 = ctx.extract(id1);
    let result2 = ctx.extract(id2);

    assert_eq!(result1.to_string(), "(+ 1 2)");
    assert_eq!(result2.to_string(), "(* 3 4)");
}

// ==================== Cost Function Tests ====================

#[test]
fn test_cost_arithmetic_simple() {
    init_logger();
    let catalog = Catalog::new();
    let mut ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Create a simple arithmetic expression: 1 + 2
    // Cost should be: 1 (for +) + 0 (for 1) + 0 (for 2) = 1
    let expr: RecExpr<Optlang> = "(+ 1 2)".parse().unwrap();
    let id = ctx.egraph.add_expr(&expr);

    let (cost, result) = extract_with_cost_only(&ctx, id);

    assert_eq!(cost, 2, "Cost of (+ 1 2) should be 2");
    assert_eq!(result.to_string(), "(+ 1 2)");
}

#[test]
fn test_cost_arithmetic_nested() {
    init_logger();
    let catalog = Catalog::new();
    let mut ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Create: (1 + 2) * 3
    // Cost should be: 1 (* op) + 1 (+ op) + 0 (constants) = 2
    let expr: RecExpr<Optlang> = "(* (+ 1 2) 3)".parse().unwrap();
    let id = ctx.egraph.add_expr(&expr);

    let (cost, result) = extract_with_cost_only(&ctx, id);

    assert_eq!(cost, 6, "Cost of (* (+ 1 2) 3) should be 6");
    assert_eq!(result.to_string(), "(* (+ 1 2) 3)");
}

#[test]
fn test_cost_arithmetic_complex() {
    init_logger();
    let catalog = Catalog::new();
    let mut ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Create: ((1 + 2) * (3 - 4)) / 5
    // Cost: 1 (/) + 1 (*) + 1 (+) + 1 (-) + 0 (constants) = 4
    let expr: RecExpr<Optlang> = "(/ (* (+ 1 2) (- 3 4)) 5)".parse().unwrap();
    let id = ctx.egraph.add_expr(&expr);

    let (cost, result) = extract_with_cost_only(&ctx, id);

    assert_eq!(cost, 12, "Cost should be 12 (4 operators)");
    assert_eq!(result.to_string(), "(/ (* (+ 1 2) (- 3 4)) 5)");
}

#[test]
fn test_cost_comparison_operations() {
    init_logger();
    let catalog = Catalog::new();
    let mut ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Create: (a > 5) AND (b < 10)
    // Cost: 1 (AND) + 1 (>) + 1 (<) + 0 (constants) = 3
    let expr: RecExpr<Optlang> = "(AND (> 10 5) (< 3 10))".parse().unwrap();
    let id = ctx.egraph.add_expr(&expr);

    let (cost, result) = extract_with_cost_only(&ctx, id);

    assert_eq!(cost, 3, "Cost should be 3 (AND + > + <)");
}

#[test]
fn test_cost_with_catalog_table_sizes() {
    init_logger();
    let mut catalog = Catalog::new();

    // Create tables with different sizes
    let small_table_id = catalog
        .create_table_with_cols(
            "small_table".to_string(),
            vec![("id".to_string(), DataType::Int)],
        )
        .unwrap();

    let large_table_id = catalog
        .create_table_with_cols(
            "large_table".to_string(),
            vec![("id".to_string(), DataType::Int)],
        )
        .unwrap();

    // Set table sizes
    catalog
        .get_table_by_id(small_table_id)
        .unwrap()
        .clone()
        .set_est_num_rows(100);
    catalog
        .tables
        .get_mut(&small_table_id)
        .unwrap()
        .set_est_num_rows(100);

    catalog
        .tables
        .get_mut(&large_table_id)
        .unwrap()
        .set_est_num_rows(10000);

    let mut ctx = DbOptimizer::new(DbUserData::new(catalog.clone()));

    // Create TableScan for small table
    let small_scan = make_table_scan_expr(small_table_id);
    let small_id = ctx.egraph.add_expr(&small_scan);

    let (small_cost, _) = extract_with_cost_only(&ctx, small_id);
    let small_table = catalog.get_table_by_id(small_table_id).unwrap();
    let expected_cost =
        small_table.get_est_num_blocks() * IO_COST + small_table.get_est_num_rows() * TRANSFER_COST;
    assert_eq!(
        small_cost, expected_cost,
        "Small table scan cost should be {expected_cost}"
    );

    // Create TableScan for large table
    let large_scan = make_table_scan_expr(large_table_id);
    let large_id = ctx.egraph.add_expr(&large_scan);

    let (large_cost, _) = extract_with_cost_only(&ctx, large_id);
    let large_table = catalog.get_table_by_id(large_table_id).unwrap();
    let expected_large_cost =
        large_table.get_est_num_blocks() * IO_COST + large_table.get_est_num_rows() * TRANSFER_COST;

    assert_eq!(
        large_cost, expected_large_cost,
        "Large table scan cost should be {expected_large_cost}"
    );
}

#[test]
fn test_cost_chooses_cheaper_equivalent() {
    init_logger();
    let catalog = Catalog::new();
    let mut ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Create two equivalent expressions with different costs
    // Expression 1: (1 + 2) + 3 = cost 2 (two + operators)
    let expr1: RecExpr<Optlang> = "(+ (+ 1 2) 3)".parse().unwrap();
    let id1 = ctx.egraph.add_expr(&expr1);

    // Expression 2: 1 + (2 + 3) = cost 2 (two + operators)
    let expr2: RecExpr<Optlang> = "(+ 1 (+ 2 3))".parse().unwrap();
    let id2 = ctx.egraph.add_expr(&expr2);

    // Union them to make them equivalent
    ctx.egraph.union(id1, id2);
    ctx.egraph.rebuild();

    // Extract should return one of them (both have same cost)
    let (cost, result) = extract_with_cost_only(&ctx, id1);
    assert_eq!(cost, 4, "Cost should be 4 for either expression");

    // Result should be one of the two forms
    let result_str = result.to_string();
    assert!(
        result_str == "(+ (+ 1 2) 3)" || result_str == "(+ 1 (+ 2 3))",
        "Result should be one of the equivalent forms"
    );
}

#[test]
fn test_cost_prefers_cheaper_when_different() {
    init_logger();
    let catalog = Catalog::new();
    let mut ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Create two expressions with significantly different costs
    // Cheap: 1 + 2 = cost 1
    let cheap_expr: RecExpr<Optlang> = "(+ 1 2)".parse().unwrap();
    let cheap_id = ctx.egraph.add_expr(&cheap_expr);

    // Expensive: ((1 + 2) * (3 - 4)) / 5 = cost 4
    let expensive_expr: RecExpr<Optlang> = "(/ (* (+ 1 2) (- 3 4)) 5)".parse().unwrap();
    let expensive_id = ctx.egraph.add_expr(&expensive_expr);

    // Union them (pretending they're equivalent for testing purposes)
    ctx.egraph.union(cheap_id, expensive_id);
    ctx.egraph.rebuild();

    // Extract should choose the cheaper one
    let (cost, result) = extract_with_cost_only(&ctx, cheap_id);
    assert_eq!(cost, 2, "Should choose cheaper expression with cost 2");
    assert_eq!(
        result.to_string(),
        "(+ 1 2)",
        "Should extract the cheaper expression"
    );
}

#[test]
fn test_cost_index_scan_vs_table_scan() {
    init_logger();
    let mut catalog = Catalog::new();

    // Create a table
    let table_id = catalog
        .create_table_with_cols(
            "users".to_string(),
            vec![
                ("id".to_string(), DataType::Int),
                ("name".to_string(), DataType::String),
            ],
        )
        .unwrap();

    // Create an index on the table
    let index_id = catalog
        .create_table_index(
            Some("idx_users_id".to_string()),
            "users".to_string(),
            vec!["id".to_string()],
        )
        .unwrap();

    // Set table size
    catalog
        .tables
        .get_mut(&table_id)
        .unwrap()
        .set_est_num_rows(1000);

    let mut ctx = DbOptimizer::new(DbUserData::new(catalog.clone()));

    // Create TableScan
    let table_scan = make_table_scan_expr(table_id);
    let table_scan_id = ctx.egraph.add_expr(&table_scan);

    // Create IndexScan
    let index_scan = make_index_scan_expr(index_id);
    let index_scan_id = ctx.egraph.add_expr(&index_scan);

    // Both should have same cost (table size) but different properties
    let (table_cost, _) = extract_with_cost_only(&ctx, table_scan_id);
    let table = catalog.get_table_by_id(table_id).unwrap();
    let expected_table_cost =
        table.get_est_num_blocks() * IO_COST + table.get_est_num_rows() * TRANSFER_COST;

    let (index_cost, _) = extract_with_cost_only(&ctx, index_scan_id);
    let expected_index_cost = (IO_COST + TRANSFER_COST) * table.get_est_num_rows(); // Index scan cost is proportional to number of rows, but cheaper than full table scan

    assert_eq!(
        table_cost, expected_table_cost,
        "TableScan cost should match expected table cost"
    );
    assert_eq!(
        index_cost, expected_index_cost,
        "IndexScan cost should also match table size"
    );
}

#[test]
fn test_cost_sort_optimization() {
    init_logger();
    let mut catalog = Catalog::new();

    // Create a table and index
    let table_id = catalog
        .create_table_with_cols(
            "data".to_string(),
            vec![("value".to_string(), DataType::Int)],
        )
        .unwrap();

    let index_id = catalog
        .create_table_index(None, "data".to_string(), vec!["value".to_string()])
        .unwrap();

    catalog
        .tables
        .get_mut(&table_id)
        .unwrap()
        .set_est_num_rows(500);

    let mut ctx = DbOptimizer::new(DbUserData::new(catalog.clone()));

    // Create: SORT(TABLE_SCAN(table)) - unsorted input needs sorting
    let table_scan = make_table_scan_expr(table_id);
    let table_scan_id = ctx.egraph.add_expr(&table_scan);

    let mut sort_table_expr = RecExpr::default();
    let colset_id = sort_table_expr.add(Optlang::ColSet(1)); // Dummy colset
    let table_ref = sort_table_expr.add(Optlang::Table(table_id));
    let scan_node = sort_table_expr.add(Optlang::TableScan(table_ref));
    sort_table_expr.add(Optlang::Sort([scan_node, colset_id]));

    let sort_table_id = ctx.egraph.add_expr(&sort_table_expr);

    // Cost: 500 (table scan) + 0 (colset) + 50 (sort) = 550
    let (sort_table_cost, _) = extract_with_cost_only(&ctx, sort_table_id);
    let table = catalog.get_table_by_id(table_id).unwrap();
    let expected_sort_table_cost = 3 * IO_COST * table.get_est_num_blocks()
        + table.get_est_num_rows() * ((table.get_est_num_rows() as f64).log2() as usize) * CPU_COST; // Sorting cost is proportional to number of rows and log(rows)

    assert_eq!(
        sort_table_cost, expected_sort_table_cost,
        "Sorting unsorted table should cost {} (expected {})",
        sort_table_cost, expected_sort_table_cost
    );

    // Create: SORT(INDEX_SCAN(index)) - already sorted, sort is free
    let mut sort_index_expr = RecExpr::default();
    let colset_id2 = sort_index_expr.add(Optlang::ColSet(1));
    let index_ref = sort_index_expr.add(Optlang::Index(index_id));
    let index_scan_node = sort_index_expr.add(Optlang::IndexScan(index_ref));
    let index_id = ctx.egraph.add_expr(&sort_index_expr);
    sort_index_expr.add(Optlang::Sort([index_scan_node, colset_id2]));

    let sort_index_id = ctx.egraph.add_expr(&sort_index_expr);

    // Cost: 500 (index scan) + 0 (colset) + 0 (sort skipped) = 500
    let (sort_index_cost, _) = extract_with_cost_only(&ctx, sort_index_id);
    let (index_cost, _) = extract_with_cost_only(&ctx, index_id);
    assert_eq!(
        sort_index_cost, index_cost,
        "Sorting already-sorted index should cost 500 (no sort)"
    );
}

// ==================== Full Optimizer Workflow Tests ====================

#[test]
fn test_merge_join_property_aware_optimization() {
    init_logger();
    let mut catalog = Catalog::new();

    // Create two tables
    let table1_id = catalog
        .create_table_with_cols(
            "table1".to_string(),
            vec![("id".to_string(), DataType::Int)],
        )
        .unwrap();

    let table2_id = catalog
        .create_table_with_cols(
            "table2".to_string(),
            vec![("id".to_string(), DataType::Int)],
        )
        .unwrap();

    // Create indices on both tables
    let index1_id = catalog
        .create_table_index(None, "table1".to_string(), vec!["id".to_string()])
        .unwrap();

    let index2_id = catalog
        .create_table_index(None, "table2".to_string(), vec!["id".to_string()])
        .unwrap();

    // Set table sizes to make index scans more expensive than table scans
    // This ensures that without property requirements, table scans would be chosen
    catalog
        .tables
        .get_mut(&table1_id)
        .unwrap()
        .set_est_num_rows(100);
    catalog
        .tables
        .get_mut(&table2_id)
        .unwrap()
        .set_est_num_rows(200);

    let mut ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Build initial expression with MergeJoin
    // The optimizer should add both TableScan and IndexScan alternatives via exploration
    // Then optimization should select IndexScan for MergeJoin inputs because they're sorted
    let mut initial_expr = RecExpr::default();

    let t1 = initial_expr.add(Optlang::Table(table1_id));
    let scan1 = initial_expr.add(Optlang::Scan(t1));

    let t2 = initial_expr.add(Optlang::Table(table2_id));
    let scan2 = initial_expr.add(Optlang::Scan(t2));

    let pred = initial_expr.add(Optlang::Bool(true));

    // Use logical Join which should be converted to physical joins during exploration
    initial_expr.add(Optlang::Join([scan1, scan2, pred]));

    let root_id = ctx.egraph.add_expr(&initial_expr);

    debug!("E-graph before optimization:");
    debug!("{:#?}", ctx.egraph);

    // Run optimization
    ctx.run(root_id);

    debug!("E-graph after optimization:");
    debug!("{:#?}", ctx.egraph);
    // Note: optimized_groups and costs are now private fields
    // debug!("Optimized groups: {:?}", ctx.optimized_groups);
    // debug!("Costs: {:?}", ctx.costs);

    // TODO: These assertions access private fields and need to be refactored
    // Check that we have optimized the root with Bottom properties
    // assert!(
    //     ctx.optimized_groups
    //         .contains_key(&(root_id, SimpleProperty::Bottom)),
    //     "Root should be optimized with Bottom properties"
    // );

    // Verify that both scan eclasses have both Sorted and Unsorted options optimized
    // The scan eclasses should have multiple expressions (TableScan and IndexScan)
    let scan1_eclass = ctx.egraph.find(scan1);
    let scan2_eclass = ctx.egraph.find(scan2);

    debug!("Scan1 eclass {} members:", scan1_eclass);
    for (node_id, node) in ctx.egraph.nodes_in_class(scan1_eclass) {
        debug!("  Node {:?}: {:?}", node_id, node);
    }

    debug!("Scan2 eclass {} members:", scan2_eclass);
    for (node_id, node) in ctx.egraph.nodes_in_class(scan2_eclass) {
        debug!("  Node {:?}: {:?}", node_id, node);
    }

    // TODO: These assertions access private ctx.costs field
    // Check costs for scan1 with different properties
    // if let Some(cost_sorted) = ctx.costs.get(&(scan1_eclass, SimpleProperty::Sorted)) {
    //     debug!("Scan1 with Sorted property: cost = {:?}", cost_sorted);
    //     // Should select IndexScan for sorted
    //     assert_eq!(cost_sorted.properties, SimpleProperty::Sorted);
    // }

    // if let Some(cost_unsorted) = ctx.costs.get(&(scan1_eclass, SimpleProperty::Unsorted)) {
    //     debug!("Scan1 with Unsorted property: cost = {:?}", cost_unsorted);
    //     // Should select TableScan for unsorted
    //     assert_eq!(cost_unsorted.properties, SimpleProperty::Unsorted);
    // }

    // TODO: The key test accesses private ctx.optimized_groups field
    // The key test: if MergeJoin was selected in the root, verify its inputs are sorted
    // let root_eclass = ctx.egraph.find(root_id);
    // if let Some(&best_expr_id) = ctx
    //     .optimized_groups
    //     .get(&(root_eclass, SimpleProperty::Bottom))
    // {
    //     let best_node = ctx.egraph.get_node(best_expr_id);
    //     debug!("Best expression for root: {:?}", best_node);

    //     if let Optlang::MergeJoin([left, right, _]) = best_node {
    //         debug!("MergeJoin selected! Checking if inputs are sorted...");

    //         // Check that left input was optimized with Sorted property
    //         let left_eclass = ctx.egraph.find(*left);
    //         let left_cost = ctx.costs.get(&(left_eclass, SimpleProperty::Sorted));
    //         assert!(
    //             left_cost.is_some() && left_cost.unwrap().properties == SimpleProperty::Sorted,
    //             "MergeJoin left input should have Sorted property"
    //         );

    //         // Check that right input was optimized with Sorted property
    //         let right_eclass = ctx.egraph.find(*right);
    //         let right_cost = ctx.costs.get(&(right_eclass, SimpleProperty::Sorted));
    //         assert!(
    //             right_cost.is_some()
    //                 && right_cost.unwrap().properties == SimpleProperty::Sorted,
    //             "MergeJoin right input should have Sorted property"
    //         );

    //         debug!("✓ MergeJoin correctly uses sorted inputs!");
    //     } else {
    //         debug!("Note: MergeJoin was not selected (another join type chosen)");
    //     }
    // }
}

#[test]
fn test_merge_join_extraction_uses_sorted_inputs() {
    init_logger();
    let mut catalog = Catalog::new();

    // Create two tables with indices
    let table1_id = catalog
        .create_table_with_cols(
            "table1".to_string(),
            vec![("id".to_string(), DataType::Int)],
        )
        .unwrap();

    let table2_id = catalog
        .create_table_with_cols(
            "table2".to_string(),
            vec![("id".to_string(), DataType::Int)],
        )
        .unwrap();

    let index1_id = catalog
        .create_table_index(None, "table1".to_string(), vec!["id".to_string()])
        .unwrap();

    let index2_id = catalog
        .create_table_index(None, "table2".to_string(), vec!["id".to_string()])
        .unwrap();

    catalog
        .tables
        .get_mut(&table1_id)
        .unwrap()
        .set_est_num_rows(100);
    catalog
        .tables
        .get_mut(&table2_id)
        .unwrap()
        .set_est_num_rows(200);

    let mut ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Build initial expression with logical Join
    let mut initial_expr = RecExpr::default();
    let t1 = initial_expr.add(Optlang::Table(table1_id));
    let scan1 = initial_expr.add(Optlang::Scan(t1));
    let t2 = initial_expr.add(Optlang::Table(table2_id));
    let scan2 = initial_expr.add(Optlang::Scan(t2));
    let pred = initial_expr.add(Optlang::Bool(true));
    initial_expr.add(Optlang::Join([scan1, scan2, pred]));

    let root_id = ctx.egraph.add_expr(&initial_expr);

    // Run optimization
    ctx.run(root_id);

    // Extract the best expression
    let (cost, extracted_expr) = ctx.extract_with_cost(root_id);
    let extracted_str = extracted_expr.to_string();

    debug!("Extracted expression: {}", extracted_str);
    debug!("Cost: {:?}", cost);

    // Verify that if MergeJoin is in the extracted plan, its inputs are sorted (IndexScan)
    if extracted_str.contains("MERGE_JOIN") {
        debug!("✓ MergeJoin found in extracted plan");

        // Count IndexScans and TableScans
        let index_scan_count = extracted_str.matches("INDEX_SCAN").count();
        let table_scan_count = extracted_str.matches("TABLE_SCAN").count();

        debug!(
            "IndexScans: {}, TableScans: {}",
            index_scan_count, table_scan_count
        );

        // If MergeJoin is used, we should have at least 2 IndexScans (for the two inputs)
        assert!(
            index_scan_count >= 2,
            "MergeJoin should use IndexScans (sorted inputs), but found {} IndexScans",
            index_scan_count
        );

        // Verify the indices are correct
        assert!(
            extracted_str.contains(&index1_id.to_string()),
            "Should use index1"
        );
        assert!(
            extracted_str.contains(&index2_id.to_string()),
            "Should use index2"
        );

        debug!("✓ MergeJoin correctly uses IndexScans for sorted inputs in extracted plan!");
    } else {
        debug!("Note: MergeJoin not used in final plan (different join type selected)");

        // Even if MergeJoin wasn't selected, the plan should still be valid
        // and should have some physical join operator
        assert!(
            extracted_str.contains("HASH_JOIN")
                || extracted_str.contains("NESTED_LOOP_JOIN")
                || extracted_str.contains("MERGE_JOIN"),
            "Plan should contain a physical join operator"
        );
    }

    // Verify no logical operators in the extracted plan
    assert!(
        !extracted_str.contains("(JOIN ") && !extracted_str.contains("(SCAN "),
        "Extracted plan should not contain logical operators, got: {}",
        extracted_str
    );
}

#[test]
fn test_selection_pushdown_through_join() {
    init_logger();
    let mut catalog = Catalog::new();

    // Create two tables
    let customers_id = catalog
        .create_table_with_cols(
            "customers".to_string(),
            vec![
                ("id".to_string(), DataType::Int),
                ("name".to_string(), DataType::String),
                ("age".to_string(), DataType::Int),
            ],
        )
        .unwrap();

    let orders_id = catalog
        .create_table_with_cols(
            "orders".to_string(),
            vec![
                ("id".to_string(), DataType::Int),
                ("customer_id".to_string(), DataType::Int),
                ("amount".to_string(), DataType::Int),
            ],
        )
        .unwrap();

    // Set table sizes - customers is much smaller
    catalog
        .tables
        .get_mut(&customers_id)
        .unwrap()
        .set_est_num_rows(1000);
    catalog
        .tables
        .get_mut(&orders_id)
        .unwrap()
        .set_est_num_rows(10000);

    let mut ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Build initial (unoptimized) expression:
    // SELECT(JOIN(customers, orders), age > 30)
    // This applies the filter AFTER the join (expensive!)
    let mut initial_expr = RecExpr::default();

    // Create scans
    let cust_table = initial_expr.add(Optlang::Table(customers_id));
    let cust_scan = initial_expr.add(Optlang::Scan(cust_table));

    let orders_table = initial_expr.add(Optlang::Table(orders_id));
    let orders_scan = initial_expr.add(Optlang::Scan(orders_table));

    // Join predicate (simplified - just true for this test)
    let join_pred = initial_expr.add(Optlang::Bool(true));

    // Join the tables
    let join_node = initial_expr.add(Optlang::Join([cust_scan, orders_scan, join_pred]));

    // Selection predicate: age > 30
    let age_val = initial_expr.add(Optlang::Int(30));
    let age_ref = initial_expr.add(Optlang::Int(0)); // Simplified column reference
    let select_pred = initial_expr.add(Optlang::Gt([age_ref, age_val]));

    // Apply selection after join (unoptimized!)
    initial_expr.add(Optlang::Select([join_node, select_pred]));

    let root_id = ctx.egraph.add_expr(&initial_expr);

    // Get initial cost
    let (initial_cost, initial_result) = ctx.extract_with_cost(root_id);
    debug!("Initial expression: {}", initial_result);
    debug!("Initial cost: {:?}", initial_cost);

    // Run optimizer to explore equivalent expressions
    ctx.run(root_id);

    // Extract the best plan
    let (optimized_cost, optimized_result) = ctx.extract_with_cost(root_id);
    debug!("Optimized expression: {}", optimized_result);
    debug!("Optimized cost: {:?}", optimized_cost);

    // The optimized plan should have equal or lower cost
    assert!(
        optimized_cost <= initial_cost,
        "Optimized cost {:?} should be <= initial cost {:?}",
        optimized_cost,
        initial_cost
    );

    let optimized_str = optimized_result.to_string();
    debug!("Complete optimized plan: {}", optimized_str);

    // EXPECTED STRUCTURE:
    // Initial:  SELECT(JOIN(SCAN(customers), SCAN(orders)), age > 30)
    // Expected: PHYSICAL_JOIN(SELECT(TABLE_SCAN(customers), age > 30), TABLE_SCAN(orders), pred)
    //
    // The selection should be pushed down INSIDE the join, not wrapping it

    // 1. Must use physical join
    assert!(
        optimized_str.contains("HASH_JOIN")
            || optimized_str.contains("NESTED_LOOP_JOIN")
            || optimized_str.contains("MERGE_JOIN"),
        "Expected physical join, got: {}",
        optimized_str
    );

    // 2. Must use physical scans, not logical SCAN
    assert!(
        optimized_str.contains("TABLE_SCAN"),
        "Expected TABLE_SCAN, got: {}",
        optimized_str
    );

    // 3. Must NOT contain logical operators
    assert!(
        !optimized_str.contains("(SCAN ") && !optimized_str.contains("(JOIN "),
        "Expected no logical operators, got: {}",
        optimized_str
    );

    // 4. Check if selection was pushed down
    // If SELECT appears BEFORE the first JOIN, it wasn't pushed down
    if let Some(select_pos) = optimized_str.find("SELECT") {
        if let Some(join_pos) = optimized_str.find("JOIN") {
            assert!(
                select_pos > join_pos,
                "FAILED: SelectionSELECT was NOT pushed down through join.\\n\\\n                     Expected: PHYSICAL_JOIN(SELECT(TABLE_SCAN(...), pred), ...)\\n\\\n                     Got:      SELECT(PHYSICAL_JOIN(...), pred)\\n\\\n                     Plan: {}",
                optimized_str
            );
        }
    }
}

#[test]
fn test_selection_pushdown_through_projection() {
    init_logger();
    let mut catalog = Catalog::new();

    let table_id = catalog
        .create_table_with_cols(
            "employees".to_string(),
            vec![
                ("id".to_string(), DataType::Int),
                ("name".to_string(), DataType::String),
                ("salary".to_string(), DataType::Int),
                ("department".to_string(), DataType::String),
            ],
        )
        .unwrap();

    catalog
        .tables
        .get_mut(&table_id)
        .unwrap()
        .set_est_num_rows(5000);

    let mut ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Build: SELECT(PROJECT(columns, scan), salary > 50000)
    // Optimizer should push selection before projection
    let mut initial_expr = RecExpr::default();

    let table = initial_expr.add(Optlang::Table(table_id));
    let scan = initial_expr.add(Optlang::Scan(table));

    // Project to subset of columns
    let colset = initial_expr.add(Optlang::ColSet(1)); // Simplified
    let project = initial_expr.add(Optlang::Project([colset, scan]));

    // Selection: salary > 50000
    let salary_val = initial_expr.add(Optlang::Int(50000));
    let salary_ref = initial_expr.add(Optlang::Int(2)); // Column 2 = salary
    let predicate = initial_expr.add(Optlang::Gt([salary_ref, salary_val]));

    initial_expr.add(Optlang::Select([project, predicate]));

    let root_id = ctx.egraph.add_expr(&initial_expr);
    let (initial_cost, initial_best) = ctx.extract_with_cost(root_id);

    // Run optimizer
    ctx.run(root_id);

    let (optimized_cost, optimized_result) = ctx.extract_with_cost(root_id);
    debug!(
        "Initial cost: {:?}, Optimized cost: {:?}",
        initial_cost, optimized_cost
    );
    debug!("Initial plan: {}", initial_best);
    debug!("Optimized plan: {}", optimized_result);

    // Optimized should be equal or better
    assert!(
        optimized_cost <= initial_cost,
        "Optimized cost {:?} should be <= initial cost {:?}",
        optimized_cost,
        initial_cost
    );

    let optimized_str = optimized_result.to_string();
    let initial_str = initial_best.to_string();
    debug!("Initial plan:    {}", initial_str);
    debug!("Optimized plan:  {}", optimized_str);

    // EXPECTED STRUCTURE:
    // Initial:  SELECT(PROJECT(cols, SCAN(table)), salary > 50000)
    // Expected: PROJECT(cols, SELECT(TABLE_SCAN(table), salary > 50000))
    //
    // The selection should be pushed down INSIDE the projection

    // 1. PROJECT should be the outermost operator
    assert!(
        optimized_str.starts_with("(PROJECT"),
        "FAILED: PROJECT should be outermost operator.\\n\\\n             Expected: (PROJECT ... (SELECT ...))\\n\\\n             Got:      {}\\n\\\n             Selection was NOT pushed through projection!",
        optimized_str
    );

    // 2. SELECT should be nested inside PROJECT
    let project_pos = optimized_str.find("PROJECT").unwrap();
    let select_pos = optimized_str.find("SELECT");

    assert!(
        select_pos.is_some(),
        "FAILED: SELECT missing from plan. Got: {}",
        optimized_str
    );

    assert!(
        project_pos < select_pos.unwrap(),
        "FAILED: SELECT should be nested inside PROJECT.\\n\\\n             Expected: (PROJECT cols (SELECT (TABLE_SCAN) pred))\\n\\\n             Got:      {}\\n\\\n             Selection was NOT pushed through projection!",
        optimized_str
    );

    // 3. Must use physical scan
    assert!(
        optimized_str.contains("TABLE_SCAN"),
        "Expected TABLE_SCAN, got: {}",
        optimized_str
    );

    // 4. Must NOT have logical SCAN
    assert!(
        !optimized_str.contains("(SCAN "),
        "Expected no logical SCAN, got: {}",
        optimized_str
    );
}

#[test]
fn test_combine_consecutive_selections() {
    init_logger();
    let mut catalog = Catalog::new();

    let table_id = catalog
        .create_table_with_cols(
            "products".to_string(),
            vec![
                ("id".to_string(), DataType::Int),
                ("price".to_string(), DataType::Int),
                ("quantity".to_string(), DataType::Int),
            ],
        )
        .unwrap();

    catalog
        .tables
        .get_mut(&table_id)
        .unwrap()
        .set_est_num_rows(2000);

    let mut ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Build: SELECT(SELECT(scan, price > 100), quantity > 10)
    // Should be combined into: SELECT(scan, price > 100 AND quantity > 10)
    let mut initial_expr = RecExpr::default();

    let table = initial_expr.add(Optlang::Table(table_id));
    let scan = initial_expr.add(Optlang::Scan(table));

    // First selection: price > 100
    let price_val = initial_expr.add(Optlang::Int(100));
    let price_ref = initial_expr.add(Optlang::Int(1));
    let pred1 = initial_expr.add(Optlang::Gt([price_ref, price_val]));
    let select1 = initial_expr.add(Optlang::Select([scan, pred1]));

    // Second selection: quantity > 10
    let qty_val = initial_expr.add(Optlang::Int(10));
    let qty_ref = initial_expr.add(Optlang::Int(2));
    let pred2 = initial_expr.add(Optlang::Gt([qty_ref, qty_val]));

    initial_expr.add(Optlang::Select([select1, pred2]));

    let root_id = ctx.egraph.add_expr(&initial_expr);
    let (initial_cost, initial_result) = ctx.extract_with_cost(root_id);
    debug!("Initial: {}", initial_result);

    // Run optimizer
    ctx.run(root_id);

    let (optimized_cost, optimized_result) = ctx.extract_with_cost(root_id);
    debug!("Optimized: {}", optimized_result);

    // Cost should be same or better
    assert!(
        optimized_cost <= initial_cost,
        "Optimized cost {:?} should be <= initial cost {:?}",
        optimized_cost,
        initial_cost
    );

    let optimized_str = optimized_result.to_string();
    let initial_str = initial_result.to_string();
    debug!("Initial:    {}", initial_str);
    debug!("Optimized:  {}", optimized_str);

    // EXPECTED STRUCTURE:
    // Initial:  SELECT(SELECT(SCAN(table), pred1), pred2)
    // Expected: SELECT(TABLE_SCAN(table), AND(pred1, pred2))
    //
    // Two consecutive selections should be combined into one with AND

    // 1. Should have exactly ONE SELECT node
    let select_count = optimized_str.matches("SELECT").count();
    assert_eq!(
        select_count, 1,
        "FAILED: Should have exactly 1 SELECT (combined), got {}.\\n\\\n             Expected: (SELECT scan (AND pred1 pred2))\\n\\\n             Got:      {}\\n\\\n             Consecutive selections were NOT combined!",
        select_count, optimized_str
    );

    // 2. Must have AND combining the predicates
    assert!(
        optimized_str.contains("(AND "),
        "FAILED: Predicates should be combined with AND.\\n\\\n             Expected: (SELECT TABLE_SCAN (AND (> ...) (> ...)))\\n\\\n             Got:      {}\\n\\\n             Consecutive selections were NOT combined!",
        optimized_str
    );

    // 3. Verify structure: SELECT should be outermost
    assert!(
        optimized_str.starts_with("(SELECT"),
        "FAILED: SELECT should be outermost. Got: {}",
        optimized_str
    );

    // 4. Must use physical scan
    assert!(
        optimized_str.contains("TABLE_SCAN"),
        "Expected TABLE_SCAN, got: {}",
        optimized_str
    );

    // 5. Must NOT contain logical SCAN
    assert!(
        !optimized_str.contains("(SCAN "),
        "Expected no logical SCAN, got: {}",
        optimized_str
    );
}

#[test]
fn test_join_physical_implementation_selection() {
    init_logger();
    let mut catalog = Catalog::new();

    // Create small and large tables
    let small_table_id = catalog
        .create_table_with_cols(
            "small_table".to_string(),
            vec![("id".to_string(), DataType::Int)],
        )
        .unwrap();

    let large_table_id = catalog
        .create_table_with_cols(
            "large_table".to_string(),
            vec![("id".to_string(), DataType::Int)],
        )
        .unwrap();

    // Create index on small table
    let _small_index = catalog
        .create_table_index(None, "small_table".to_string(), vec!["id".to_string()])
        .unwrap();

    catalog
        .tables
        .get_mut(&small_table_id)
        .unwrap()
        .set_est_num_rows(100);
    catalog
        .tables
        .get_mut(&large_table_id)
        .unwrap()
        .set_est_num_rows(10000);

    let mut ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Build logical join
    let mut initial_expr = RecExpr::default();

    let small_table = initial_expr.add(Optlang::Table(small_table_id));
    let small_scan = initial_expr.add(Optlang::Scan(small_table));

    let large_table = initial_expr.add(Optlang::Table(large_table_id));
    let large_scan = initial_expr.add(Optlang::Scan(large_table));

    let pred = initial_expr.add(Optlang::Bool(true));

    initial_expr.add(Optlang::Join([small_scan, large_scan, pred]));

    let root_id = ctx.egraph.add_expr(&initial_expr);

    // Run optimizer - should explore different physical implementations
    ctx.run(root_id);

    let (optimized_cost, optimized_result) = ctx.extract_with_cost(root_id);
    let optimized_str = optimized_result.to_string();
    debug!("Optimized join plan: {}", optimized_str);
    debug!("Optimized cost: {:?}", optimized_cost);

    // EXPECTED STRUCTURE:
    // Initial:  JOIN(SCAN(t1), SCAN(t2), pred)
    // Expected: PHYSICAL_JOIN(TABLE_SCAN(t1), TABLE_SCAN(t2), pred)
    //           where PHYSICAL_JOIN is one of: HASH_JOIN, NESTED_LOOP_JOIN, MERGE_JOIN

    // 1. Must use physical join
    let has_physical_join = optimized_str.contains("HASH_JOIN")
        || optimized_str.contains("NESTED_LOOP_JOIN")
        || optimized_str.contains("MERGE_JOIN");
    assert!(
        has_physical_join,
        "FAILED: Must use physical join implementation.\\n\\\n             Expected: one of HASH_JOIN, NESTED_LOOP_JOIN, MERGE_JOIN\\n\\\n             Got:      {}\\n\\\n             Logical JOIN was NOT converted to physical!",
        optimized_str
    );

    // 2. Must NOT contain logical JOIN
    assert!(
        !optimized_str.contains("(JOIN "),
        "FAILED: Should not contain logical JOIN. Got: {}",
        optimized_str
    );

    // 3. Must use physical scans
    assert!(
        optimized_str.contains("TABLE_SCAN") || optimized_str.contains("INDEX_SCAN"),
        "FAILED: Must use physical scan. Got: {}",
        optimized_str
    );

    // 4. Must NOT contain logical SCAN
    assert!(
        !optimized_str.contains("(SCAN "),
        "FAILED: Should not contain logical SCAN. Got: {}",
        optimized_str
    );

    // 5. Should have exactly 1 join and 2 scans
    let join_count = optimized_str.matches("JOIN").count();
    assert_eq!(
        join_count, 1,
        "Expected exactly 1 join, got {}: {}",
        join_count, optimized_str
    );

    let scan_count = optimized_str.matches("SCAN").count();
    assert_eq!(
        scan_count, 2,
        "Expected exactly 2 scans, got {}: {}",
        scan_count, optimized_str
    );
}

#[test]
fn test_complex_nested_optimization() {
    init_logger();
    let mut catalog = Catalog::new();

    // Create three tables for a complex query
    let users_id = catalog
        .create_table_with_cols(
            "users".to_string(),
            vec![
                ("user_id".to_string(), DataType::Int),
                ("age".to_string(), DataType::Int),
            ],
        )
        .unwrap();

    let orders_id = catalog
        .create_table_with_cols(
            "orders".to_string(),
            vec![
                ("order_id".to_string(), DataType::Int),
                ("user_id".to_string(), DataType::Int),
                ("total".to_string(), DataType::Int),
            ],
        )
        .unwrap();

    let items_id = catalog
        .create_table_with_cols(
            "items".to_string(),
            vec![
                ("item_id".to_string(), DataType::Int),
                ("order_id".to_string(), DataType::Int),
                ("price".to_string(), DataType::Int),
            ],
        )
        .unwrap();

    catalog
        .tables
        .get_mut(&users_id)
        .unwrap()
        .set_est_num_rows(1000);
    catalog
        .tables
        .get_mut(&orders_id)
        .unwrap()
        .set_est_num_rows(5000);
    catalog
        .tables
        .get_mut(&items_id)
        .unwrap()
        .set_est_num_rows(20000);

    let mut ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Build complex query:
    // SELECT(JOIN(JOIN(users, orders), items), age > 25)
    // Unoptimized - joins everything then filters
    let mut initial_expr = RecExpr::default();

    let users_table = initial_expr.add(Optlang::Table(users_id));
    let users_scan = initial_expr.add(Optlang::Scan(users_table));

    let orders_table = initial_expr.add(Optlang::Table(orders_id));
    let orders_scan = initial_expr.add(Optlang::Scan(orders_table));

    let items_table = initial_expr.add(Optlang::Table(items_id));
    let items_scan = initial_expr.add(Optlang::Scan(items_table));

    // Join users and orders
    let pred1 = initial_expr.add(Optlang::Bool(true));
    let join1 = initial_expr.add(Optlang::Join([users_scan, orders_scan, pred1]));

    // Join result with items
    let pred2 = initial_expr.add(Optlang::Bool(true));
    let join2 = initial_expr.add(Optlang::Join([join1, items_scan, pred2]));

    // Filter by age
    let age_val = initial_expr.add(Optlang::Int(25));
    let age_ref = initial_expr.add(Optlang::Int(1));
    let age_pred = initial_expr.add(Optlang::Gt([age_ref, age_val]));

    initial_expr.add(Optlang::Select([join2, age_pred]));

    let root_id = ctx.egraph.add_expr(&initial_expr);
    let (initial_cost, initial_result) = ctx.extract_with_cost(root_id);

    debug!("=== Complex Query Optimization ===");
    debug!("Initial plan: {}", initial_result);
    debug!("Initial cost: {:?}", initial_cost);

    // Run optimizer
    ctx.run(root_id);

    let (optimized_cost, optimized_result) = ctx.extract_with_cost(root_id);
    debug!("Optimized plan: {}", optimized_result);
    debug!("Optimized cost: {:?}", optimized_cost);

    // Should find a better plan
    assert!(
        optimized_cost <= initial_cost,
        "Optimized cost {:?} should be <= initial cost {:?}",
        optimized_cost,
        initial_cost
    );

    let optimized_str = optimized_result.to_string();

    // EXPECTED STRUCTURE:
    // Initial:  SELECT(JOIN(JOIN(SCAN(users), SCAN(orders)), SCAN(items)), age > 25)
    // Ideally expected:
    //   PHYSICAL_JOIN(
    //     PHYSICAL_JOIN(
    //       SELECT(TABLE_SCAN(users), age > 25),  // filter pushed to users scan
    //       TABLE_SCAN(orders)
    //     ),
    //     TABLE_SCAN(items)
    //   )
    // At minimum: all logical operators converted to physical

    // 1. All logical operators should be converted to physical
    assert!(
        !optimized_str.contains("(JOIN ") && !optimized_str.contains("(SCAN "),
        "FAILED: All logical operators should be physical.\\n\\\n             Expected: physical JOIN and SCAN only\\n\\\n             Got:      {}",
        optimized_str
    );

    // 2. Should have 2 physical joins for 3 tables
    let join_count = optimized_str.matches("JOIN").count();
    assert_eq!(
        join_count, 2,
        "FAILED: Should have exactly 2 joins for 3 tables, got {}. Got: {}",
        join_count, optimized_str
    );

    // 3. Should have 3 table scans (one per table)
    let scan_count =
        optimized_str.matches("TABLE_SCAN").count() + optimized_str.matches("INDEX_SCAN").count();
    assert_eq!(
        scan_count, 3,
        "FAILED: Should have exactly 3 scans for 3 tables, got {}. Got: {}",
        scan_count, optimized_str
    );

    // 4. Selection should be present
    assert!(
        optimized_str.contains("SELECT"),
        "FAILED: Selection should be present. Got: {}",
        optimized_str
    );

    // 5. Warn if selection wasn't pushed down (but don't fail - it's an optimization)
    if let Some(first_join_pos) = optimized_str.find("JOIN") {
        if let Some(select_pos) = optimized_str.find("SELECT") {
            if select_pos < first_join_pos {
                eprintln!(
                    "\\nWARNING: Selection was NOT pushed down through joins!\\n\\\n                         Expected: PHYSICAL_JOIN(... SELECT(...) ..., ...)\\n\\\n                         Got:      SELECT(PHYSICAL_JOIN(...), ...)\\n\\\n                         Plan: {}\\n",
                    optimized_str
                );
            }
        }
    }
}

#[test]
fn test_arithmetic_simplification_in_query() {
    init_logger();
    let mut catalog = Catalog::new();

    let table_id = catalog
        .create_table_with_cols(
            "products".to_string(),
            vec![
                ("id".to_string(), DataType::Int),
                ("price".to_string(), DataType::Int),
            ],
        )
        .unwrap();

    catalog
        .tables
        .get_mut(&table_id)
        .unwrap()
        .set_est_num_rows(1000);

    let mut ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Build query with arithmetic that can be simplified:
    // SELECT(scan, (price * 1) + 0 > 100)
    // Should simplify to: SELECT(scan, price > 100)
    let mut initial_expr = RecExpr::default();

    let table = initial_expr.add(Optlang::Table(table_id));
    let scan = initial_expr.add(Optlang::Scan(table));

    // Build: (price * 1) + 0
    let price_ref = initial_expr.add(Optlang::Int(1)); // Column reference
    let one = initial_expr.add(Optlang::Int(1));
    let mul_result = initial_expr.add(Optlang::Mul([price_ref, one]));
    let zero = initial_expr.add(Optlang::Int(0));
    let add_result = initial_expr.add(Optlang::Add([mul_result, zero]));

    // Compare to 100
    let hundred = initial_expr.add(Optlang::Int(100));
    let predicate = initial_expr.add(Optlang::Gt([add_result, hundred]));

    initial_expr.add(Optlang::Select([scan, predicate]));

    let root_id = ctx.egraph.add_expr(&initial_expr);
    let (initial_cost, initial_result) = ctx.extract_with_cost(root_id);

    debug!("Initial with complex arithmetic: {}", initial_result);

    // Run optimizer
    ctx.run(root_id);

    let (optimized_cost, optimized_result) = ctx.extract_with_cost(root_id);
    debug!("Optimized simplified: {}", optimized_result);

    // Should be cheaper or equal
    assert!(
        optimized_cost <= initial_cost,
        "Optimized cost {:?} should be <= initial cost {:?}",
        optimized_cost,
        initial_cost
    );

    let initial_str = initial_result.to_string();
    let optimized_str = optimized_result.to_string();

    debug!("Initial arithmetic: {}", initial_str);
    debug!("Optimized arithmetic: {}", optimized_str);

    // EXPECTED STRUCTURE:
    // Initial:  SELECT(SCAN(table), (price * 1) + 0 > 100)
    // Expected: SELECT(TABLE_SCAN(table), price > 100)
    //
    // The arithmetic should be simplified: (x * 1) + 0 => x

    // 1. Should NOT contain \"* 1\"
    assert!(
        !optimized_str.contains("* 1"),
        "FAILED: Multiplication by 1 should be eliminated.\\n\\\n             Expected: ... price > 100\\n\\\n             Got:      {}\\n\\\n             Rule 'x * 1 => x' was NOT applied!",
        optimized_str
    );

    // 2. Should NOT contain \"+ 0\"
    assert!(
        !optimized_str.contains("+ 0"),
        "FAILED: Addition of 0 should be eliminated.\\n\\\n             Expected: ... price > 100\\n\\\n             Got:      {}\\n\\\n             Rule 'x + 0 => x' was NOT applied!",
        optimized_str
    );

    // 3. Should have fewer operators
    let initial_mul = initial_str.matches('*').count();
    let initial_add = initial_str.matches('+').count();
    let optimized_mul = optimized_str.matches('*').count();
    let optimized_add = optimized_str.matches('+').count();

    let initial_ops = initial_mul + initial_add;
    let optimized_ops = optimized_mul + optimized_add;

    assert!(
        optimized_ops < initial_ops,
        "FAILED: Should have fewer arithmetic operators.\\n\\\n             Initial: {} operators (* and +)\\n\\\n             Optimized: {} operators\\n\\\n             Initial:  {}\\n\\\n             Optimized: {}",
        initial_ops,
        optimized_ops,
        initial_str,
        optimized_str
    );

    // 4. Must use physical scan
    assert!(
        optimized_str.contains("TABLE_SCAN"),
        "Expected TABLE_SCAN, got: {}",
        optimized_str
    );
}

#[test]
fn test_join_associativity_optimization() {
    init_logger();
    let mut catalog = Catalog::new();

    // Create three tables with different sizes
    let small_id = catalog
        .create_table_with_cols("small".to_string(), vec![("id".to_string(), DataType::Int)])
        .unwrap();

    let medium_id = catalog
        .create_table_with_cols(
            "medium".to_string(),
            vec![("id".to_string(), DataType::Int)],
        )
        .unwrap();

    let large_id = catalog
        .create_table_with_cols("large".to_string(), vec![("id".to_string(), DataType::Int)])
        .unwrap();

    catalog
        .tables
        .get_mut(&small_id)
        .unwrap()
        .set_est_num_rows(10);
    catalog
        .tables
        .get_mut(&medium_id)
        .unwrap()
        .set_est_num_rows(1000);
    catalog
        .tables
        .get_mut(&large_id)
        .unwrap()
        .set_est_num_rows(100000);

    let mut ctx = DbOptimizer::new(DbUserData::new(catalog));

    // Build: JOIN(small, JOIN(medium, large))
    // Optimizer might reorder to: JOIN(JOIN(small, medium), large) if beneficial
    let mut initial_expr = RecExpr::default();

    let small_table = initial_expr.add(Optlang::Table(small_id));
    let small_scan = initial_expr.add(Optlang::Scan(small_table));

    let medium_table = initial_expr.add(Optlang::Table(medium_id));
    let medium_scan = initial_expr.add(Optlang::Scan(medium_table));

    let large_table = initial_expr.add(Optlang::Table(large_id));
    let large_scan = initial_expr.add(Optlang::Scan(large_table));

    // Join medium and large first (potentially expensive)
    let pred1 = initial_expr.add(Optlang::Bool(true));
    let join1 = initial_expr.add(Optlang::Join([medium_scan, large_scan, pred1]));

    // Then join with small
    let pred2 = initial_expr.add(Optlang::Bool(true));
    initial_expr.add(Optlang::Join([small_scan, join1, pred2]));

    let root_id = ctx.egraph.add_expr(&initial_expr);
    let (initial_cost, initial_result) = ctx.extract_with_cost(root_id);

    debug!("Initial join order: {}", initial_result);
    debug!("Initial cost: {:?}", initial_cost);

    // Run optimizer
    ctx.run(root_id);

    let (optimized_cost, optimized_result) = ctx.extract_with_cost(root_id);
    debug!("Optimized join order: {}", optimized_result);
    debug!("Optimized cost: {:?}", optimized_cost);

    // Optimizer should find equal or better plan
    assert!(
        optimized_cost <= initial_cost,
        "Optimized cost {:?} should be <= initial cost {:?}",
        optimized_cost,
        initial_cost
    );

    let optimized_str = optimized_result.to_string();

    // EXPECTED STRUCTURE:
    // Initial:  JOIN(SCAN(small), JOIN(SCAN(medium), SCAN(large)))
    // The optimizer should explore different join orders and physical implementations
    // Ideal might be different order, but must have physical operators

    // 1. No logical operators
    assert!(
        !optimized_str.contains("(JOIN ") && !optimized_str.contains("(SCAN "),
        "FAILED: Should use physical operators only. Got: {}",
        optimized_str
    );

    // 2. Should have exactly 2 joins for 3 tables
    let join_count = optimized_str.matches("JOIN").count();
    assert_eq!(
        join_count, 2,
        "FAILED: Should have exactly 2 joins for 3 tables, got {}. Got: {}",
        join_count, optimized_str
    );

    // 3. Should have exactly 3 physical scans
    let scan_count =
        optimized_str.matches("TABLE_SCAN").count() + optimized_str.matches("INDEX_SCAN").count();
    assert_eq!(
        scan_count, 3,
        "FAILED: Should have exactly 3 scans for 3 tables, got {}. Got: {}",
        scan_count, optimized_str
    );

    // 4. All table IDs should be present
    assert!(
        optimized_str.contains(&small_id.to_string())
            && optimized_str.contains(&medium_id.to_string())
            && optimized_str.contains(&large_id.to_string()),
        "FAILED: All 3 tables should be in plan. Got: {}",
        optimized_str
    );

    // 5. Cost should be reasonable (not overflow)
    assert!(
        optimized_cost.cost < usize::MAX / 2,
        "FAILED: Join cost should be reasonable, got {}",
        optimized_cost.cost
    );

    // Note: We don't assert a specific join order here, as the optimizer
    // might choose different orderings. The key is it explores alternatives
    // and the cost model drives the selection.
}
