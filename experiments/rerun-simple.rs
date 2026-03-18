#[cfg(feature = "rerun-metrics")]
#[path = "../examples/boolean-optimizer/mod.rs"]
mod bool_opt;

#[cfg(feature = "rerun-metrics")]
use egg::{RecExpr, Rewrite, Runner, SymbolLang, rewrite as rw};
#[cfg(feature = "rerun-metrics")]
use intarsia::metrics::init_rerun_stream;

#[cfg(feature = "rerun-metrics")]
fn main() {
    run_egg_simple();
    run_intarsia_simple();
}

#[cfg(not(feature = "rerun-metrics"))]
fn main() {
    println!("Enable rerun-metrics feature to run this example.");
}

#[cfg(feature = "rerun-metrics")]
fn run_egg_simple() {
    let (rec, output_path) =
        init_rerun_stream("egg_simple").expect("failed to create rerun recording for egg run");

    let expr: RecExpr<SymbolLang> = "(+ 0 x)".parse().expect("invalid egg expression");
    let rules: &[Rewrite<SymbolLang, ()>] = &[
        rw!("add-0"; "(+ ?a 0)" => "?a"),
        rw!("commute-add"; "(+ ?a ?b)" => "(+ ?b ?a)"),
    ];

    let runner = Runner::default()
        .with_iter_limit(5)
        .with_expr(&expr)
        .with_rerun_metrics(rec)
        .run(rules);

    println!("egg run stop reason: {:?}", runner.stop_reason);
    println!("egg metrics file: {}", output_path.display());
}

#[cfg(feature = "rerun-metrics")]
fn run_intarsia_simple() {
    let (rec, output_path) = init_rerun_stream("intarsia_simple")
        .expect("failed to create rerun recording for intarsia run");

    let expr: RecExpr<bool_opt::BoolLang> = "(AND true (NOT (NOT x)))"
        .parse()
        .expect("invalid intarsia expression");

    let mut optimizer = bool_opt::BoolOptimizer::new(()).with_rerun_metrics(rec);
    let root_id = optimizer.init(expr);
    let stop_reason = optimizer.run(root_id);

    println!("intarsia run stop reason: {:?}", stop_reason);
    println!("intarsia metrics file: {}", output_path.display());
}
