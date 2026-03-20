use egg::{RecExpr, Runner, SimpleScheduler};
use intarsia::metrics::init_rerun_stream;
use log::info;
use std::time::{Duration, Instant};

use crate::shared;

#[cfg(feature = "rerun-metrics")]
pub fn run() {
    let iter_limit = shared::env_var("EGG_ITER_LIMIT", shared::DEFAULT_EGG_ITER_LIMIT);
    let node_limit = shared::env_var("EGG_NODE_LIMIT", shared::DEFAULT_NODE_LIMIT);
    let time_limit_s = shared::env_var("EGG_TIME_LIMIT", shared::DEFAULT_TIME_LIMIT_S);

    let (rec, output_path) =
        init_rerun_stream("egg-math-bench").expect("failed to create rerun stream");

    let rules = shared::rules();

    info!("Running egg math benchmark");
    info!("Rule set size: {}", rules.len());
    info!(
        "limits: iter={}, nodes={}, time={}s",
        iter_limit, node_limit, time_limit_s
    );

    info!("Metrics file: {}", output_path.display());

    let run_start = Instant::now();

    let mut runner: Runner<shared::Math, shared::ConstantFold> = Runner::default()
        .with_scheduler(SimpleScheduler)
        .with_iter_limit(iter_limit)
        .with_node_limit(node_limit)
        .with_time_limit(Duration::from_secs(time_limit_s))
        .with_rerun_metrics(rec);

    for expr in shared::BENCH_EXPRS {
        let parsed: RecExpr<shared::Math> = expr.parse().expect("invalid benchmark expression");
        runner = runner.with_expr(&parsed);
    }

    let runner = runner.run(&rules);
    let report = runner.report();
    let elapsed_s = run_start.elapsed().as_secs_f64();

    info!("Egg run complete");
    info!(
        "Final e-graph stats: nodes={}, classes={}",
        runner.egraph.total_size(),
        runner.egraph.number_of_classes()
    );
    info!("Total run time: {:.3}s", elapsed_s);
    info!("Runner report: {}", report);

    // NOTE: Pattern lookup microbenching is intentionally disabled for now.
    // Intarsia and egg currently share this functionality via egg internals,
    // so this is not a useful comparison dimension at this stage.
    //
    // let samples = shared::env_var("EGG_SAMPLES", 1usize);
    // let patterns = shared::build_patterns(&rules, shared::EXTRA_PATTERNS);
    // let egraph = runner.egraph;
    // let get_len = |pat: &egg::Pattern<shared::Math>| pat.to_string().len();
    // let max_width = patterns.iter().map(get_len).max().unwrap_or(0);
    //
    // for pat in &patterns {
    //     let mut times: Vec<u128> = (0..samples)
    //         .map(|_| {
    //             let start = std::time::Instant::now();
    //             let matches = pat.search(&egraph);
    //             let _n_results = matches.iter().map(|m| m.substs.len()).sum::<usize>();
    //             start.elapsed().as_nanos()
    //         })
    //         .collect();
    //     times.sort_unstable();
    //
    //     println!(
    //         "test {name:<width$} ... bench: {time:>10} ns/iter (+/- {iqr})",
    //         name = pat.to_string().replace(' ', "_"),
    //         width = max_width,
    //         time = shared::percentile(0.05, &times),
    //         iqr = shared::percentile(0.75, &times) - shared::percentile(0.25, &times),
    //     );
    // }
}
