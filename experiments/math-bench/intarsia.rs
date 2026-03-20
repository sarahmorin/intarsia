use egg::{Id, RecExpr};
use intarsia::Task;
use intarsia::framework::property::NoProperty;
use intarsia::metrics::init_rerun_stream;
use intarsia::{
    ExplorerHooks, OptimizerFramework, PropertyAwareLanguage, SimpleCost, default_explorer_hook,
};
use intarsia_macros::{isle_integration_full, isle_multi_accessors};
use log::info;
use ordered_float::NotNan;
use std::time::{Duration, Instant};

use crate::shared;

isle_integration_full! {
    path: "isle/rules.rs",
}

pub type MathOptimizer = OptimizerFramework<shared::Math, NoProperty, SimpleCost<NoProperty>, ()>;

impl PropertyAwareLanguage<NoProperty> for shared::Math {
    fn property_req(&self, _child_index: usize) -> NoProperty {
        NoProperty
    }
}

#[allow(non_camel_case_types)]
impl Context for MathOptimizer {
    type extractor_constant_returns = ContextIterWrapper<Vec<i64>, Self>;
    type constructor_constant_returns = ContextIterWrapper<Vec<Id>, Self>;
    type extractor_symbol_returns = ContextIterWrapper<Vec<String>, Self>;
    type constructor_symbol_returns = ContextIterWrapper<Vec<Id>, Self>;

    fn extractor_constant(
        &mut self,
        arg0: Id,
        returns: &mut Self::extractor_constant_returns,
    ) -> () {
        let eclass = self.egraph.find(arg0);
        for (_node_id, node) in self.egraph.nodes_in_class(eclass) {
            if let shared::Math::Constant(x) = node {
                let value = x.into_inner();
                if value.fract() == 0.0 {
                    returns.push(value as i64);
                }
            }
        }
    }

    fn constructor_constant(
        &mut self,
        arg0: i64,
        returns: &mut Self::constructor_constant_returns,
    ) -> () {
        let c = NotNan::new(arg0 as f64).expect("constant must not be NaN");
        let (id, _is_new) = self.egraph.add_with_flag(shared::Math::Constant(c));
        returns.push(id);
    }

    fn extractor_symbol(&mut self, arg0: Id, returns: &mut Self::extractor_symbol_returns) -> () {
        let eclass = self.egraph.find(arg0);
        for (_node_id, node) in self.egraph.nodes_in_class(eclass) {
            if let shared::Math::Symbol(sym) = node {
                returns.push(sym.to_string());
            }
        }
    }

    fn constructor_symbol(
        &mut self,
        arg0: String,
        returns: &mut Self::constructor_symbol_returns,
    ) -> () {
        let (id, _is_new) = self.egraph.add_with_flag(shared::Math::Symbol(arg0.into()));
        returns.push(id);
    }

    isle_multi_accessors! {
        shared::Math::Diff(extractor_diff, constructor_diff, 2);
        shared::Math::Integral(extractor_integral, constructor_integral, 2);
        shared::Math::Add(extractor_add, constructor_add, 2);
        shared::Math::Sub(extractor_sub, constructor_sub, 2);
        shared::Math::Mul(extractor_mul, constructor_mul, 2);
        shared::Math::Div(extractor_div, constructor_div, 2);
        shared::Math::Pow(extractor_pow, constructor_pow, 2);
        shared::Math::Ln(extractor_ln, constructor_ln, 1);
        shared::Math::Sqrt(extractor_sqrt, constructor_sqrt, 1);
        shared::Math::Sin(extractor_sin, constructor_sin, 1);
        shared::Math::Cos(extractor_cos, constructor_cos, 1);
    }
}

impl ExplorerHooks<shared::Math> for MathOptimizer {
    default_explorer_hook!();
}

fn isle_rule_count() -> usize {
    include_str!("isle/rules.isle")
        .lines()
        .map(str::trim_start)
        .filter(|line| line.starts_with("(rule"))
        .count()
}

#[cfg(feature = "rerun-metrics")]
pub fn run() {
    let task_limit = shared::env_var("INTARSIA_TASK_LIMIT", shared::DEFAULT_INTARSIA_TASK_LIMIT);
    let node_limit = shared::env_var("INTARSIA_NODE_LIMIT", shared::DEFAULT_NODE_LIMIT);
    let time_limit_s = shared::env_var("INTARSIA_TIME_LIMIT", shared::DEFAULT_TIME_LIMIT_S);

    let (rec, output_path) =
        init_rerun_stream("intarsia-math-bench").expect("failed to create rerun stream");

    info!("Running intarsia math benchmark");
    info!("Rule set size (from ISLE file): {}", isle_rule_count());
    info!(
        "limits: tasks={}, nodes={}, time={}s",
        task_limit, node_limit, time_limit_s
    );

    info!("Intarsia metrics file: {}", output_path.display());

    let total_start = Instant::now();

    let mut optimizer = MathOptimizer::new(())
        .with_task_limit(task_limit)
        .with_node_limit(node_limit)
        .with_time_limit(Duration::from_secs(time_limit_s))
        .with_rerun_metrics(rec);

    let mut roots = Vec::with_capacity(shared::BENCH_EXPRS.len());
    for expr in shared::BENCH_EXPRS {
        let parsed: RecExpr<shared::Math> = expr.parse().expect("invalid benchmark expression");
        roots.push(optimizer.init(parsed));
    }

    // TODO: Lower this per-run logging to debug/trace after test setup stabilizes.
    for (idx, root) in roots.iter().enumerate() {
        let run_start = Instant::now();
        let stop_reason = optimizer.run(*root);

        info!(
            "execution #{idx}: stop={stop:?}, egraph_nodes={nodes}, egraph_classes={classes}, time={elapsed_s:.3}s",
            idx = idx + 1,
            stop = stop_reason,
            nodes = optimizer.egraph.total_size(),
            classes = optimizer.egraph.number_of_classes(),
            elapsed_s = run_start.elapsed().as_secs_f64()
        );
    }

    info!("Intarsia run complete");
    info!(
        "Final e-graph stats: nodes={}, classes={}",
        optimizer.egraph.total_size(),
        optimizer.egraph.number_of_classes()
    );
    info!(
        "Total run time: {:.3}s",
        total_start.elapsed().as_secs_f64()
    );
}
