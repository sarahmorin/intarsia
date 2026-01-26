use crate::testlang::QueryOps;
use optimizer::*;

fn main() {
    // Create a ruleset
    let ruleset = vec![
        mk_rule!("select_to_filter", "Select(?a, ?b)", "Filter(?a, ?b)"),
        mk_rule!("table_to_scan", "Table(?a)", "TableScan(?a, Null)"),
        mk_rule!(
            "join_to_mergejoin",
            "Join(?a, ?b, ?c)",
            "MergeJoin(?a, ?b, ?c)"
        ),
        mk_rule!(
            "join_to_hashjoin",
            "Join(?a, ?b, ?c)",
            "HashJoin(?a, ?b, ?c)"
        ),
        mk_rule!("join_to_nljoin", "Join(?a, ?b, ?c)", "NLJoin(?a, ?b, ?c)"),
        mk_rule!(
            "filter_pushdown",
            "Filter(Join(?a, ?b, ?c), ?d)",
            "Join(Filter(?a, ?d), Filter(?b, ?d), ?c)"
        ),
        mk_rule!(
            "factor_mul_addition",
            "Add(Mul(?a, ?b), Mul(?a, ?c))",
            "Mul(?a, Add(?b, ?c))"
        ),
        mk_rule!(
            "distribute_mul_addition",
            "Mul(?a, Add(?b, ?c))",
            "Add(Mul(?a, ?b), Mul(?a, ?c))"
        ),
        mk_rule!("commute_addition", "Add(?a, ?b)", "Add(?b, ?a)"),
        mk_rule!("commute_multiplication", "Mul(?a, ?b)", "Mul(?b, ?a)"),
        mk_rule!("mul_division", "Eq(Mul(?a, ?b), ?c)", "Eq(?b, Div(?c, ?a))"),
        mk_rule!("division_mul", "Eq(?a, Div(?b, ?c))", "Eq(Mul(?a, ?c), ?b)"),
    ];

    // Arithmetic Example Query
    // SELECT * FROM A JOIN B ON (A.x * B.y) + (A.x * B.z) = A.w;
    let query = parser::Parser::<QueryOps>::parse_expr(
        "Select(\
            Join(\
                Scan(Table(A), Null, Null), \
                Scan(Table(B), Null, Null), \
                Eq(\
                    Add(\
                        Mul(TableCol(A, x), TableCol(B, y)), \
                        Mul(TableCol(A, x), TableCol(B, z))\
                    ), \
                    TableCol(A, w)\
                )\
            ), \
            Cols(A.x, A.w, B.y, B.z), \
            Null\
        )",
    )
    .unwrap();
    println!("{:?}", query);
}
