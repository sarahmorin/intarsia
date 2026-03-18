use log::warn;
use std::path::PathBuf;

use const_format::formatcp;
use rerun::RecordingStream;
use uuid::Uuid;

const DEFAULT_RERUN_OUTPUT_DIR: &str = "experiments/rerun";

/// A simple wrapper around a Rerun recording stream that handles initialization and logging of metrics.
pub type RerunStream = Option<RecordingStream>;

/// Initializes a Rerun recording stream for the given test name, saving the output to the default directory.
pub fn init_rerun_stream(test_name: &str) -> Result<(RerunStream, PathBuf), String> {
    init_rerun_stream_in_dir(test_name, PathBuf::from(DEFAULT_RERUN_OUTPUT_DIR))
}

/// Initializes a Rerun recording stream for the given test name, saving the output to the specified directory.
pub fn init_rerun_stream_in_dir(
    test_name: &str,
    output_dir: PathBuf,
) -> Result<(RerunStream, PathBuf), String> {
    std::fs::create_dir_all(&output_dir).map_err(|err| {
        format!(
            "failed to create rerun output directory '{}': {}",
            output_dir.display(),
            err
        )
    })?;

    let safe_test_name = sanitize_test_name(test_name);
    let uid = Uuid::new_v4();
    let output_path = output_dir.join(format!("{}_{}.rrd", safe_test_name, uid));

    let recording = rerun::RecordingStreamBuilder::new(safe_test_name)
        .save(&output_path)
        .map_err(|err| {
            format!(
                "failed to create rerun recording '{}': {}",
                output_path.display(),
                err
            )
        })?;

    Ok((Some(recording), output_path))
}

/// Sanitizes a test name to be safe for use in file names and Rerun app IDs by replacing non-alphanumeric characters with underscores.
fn sanitize_test_name(test_name: &str) -> String {
    let out = test_name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();

    if out.is_empty() {
        "run".to_string()
    } else {
        out
    }
}

// Constants for Rerun entity paths and metric names. These are used to log metrics in a structured way that can be easily visualized in the Rerun app.
pub const INTARSIA_PREFIX: &str = "intarsia";
pub const TASK: &str = formatcp!("{INTARSIA_PREFIX}/task");
pub const SUMMARY: &str = formatcp!("{INTARSIA_PREFIX}/summary");

pub const TASK_TOTAL_TIME: &str = formatcp!("{TASK}/total_time_s");
pub const TASK_TIME_OPTIMIZE_GROUP: &str = formatcp!("{TASK}/time_optimize_group_s");
pub const TASK_TIME_OPTIMIZE_EXPR: &str = formatcp!("{TASK}/time_optimize_expr_s");
pub const TASK_TIME_EXPLORE_GROUP: &str = formatcp!("{TASK}/time_explore_group_s");
pub const TASK_TIME_EXPLORE_CHILDREN: &str = formatcp!("{TASK}/time_explore_children_s");
pub const TASK_REBUILD_COUNT: &str = formatcp!("{TASK}/rebuild_count");
pub const TASK_REBUILD_TIME: &str = formatcp!("{TASK}/rebuild_time_s");
pub const TASK_APPLY_COUNT: &str = formatcp!("{TASK}/apply_count");
pub const TASK_OPTIMIZED_MEMO_PAIRS: &str = formatcp!("{TASK}/optimized_memo_pairs");

pub const SUMM_NODES: &str = formatcp!("{SUMMARY}/egraph_nodes");
pub const SUMM_CLASSES: &str = formatcp!("{SUMMARY}/egraph_classes");
pub const SUMM_TOTAL_TIME: &str = formatcp!("{SUMMARY}/total_time_s");
pub const SUMM_TOTAL_TIME_OPTIMIZE_GROUP: &str = formatcp!("{SUMMARY}/total_time_optimize_group_s");
pub const SUMM_TOTAL_TIME_OPTIMIZE_EXPR: &str = formatcp!("{SUMMARY}/total_time_optimize_expr_s");
pub const SUMM_TOTAL_TIME_EXPLORE_GROUP: &str = formatcp!("{SUMMARY}/total_time_explore_group_s");
pub const SUMM_TOTAL_TIME_EXPLORE_CHILDREN: &str =
    formatcp!("{SUMMARY}/total_time_explore_children_s");
pub const SUMM_TOTAL_TIME_BY_TASK_TYPE_BAR: &str =
    formatcp!("{SUMMARY}/total_time_by_task_type_bar");
pub const SUMM_TOTAL_TIME_BY_TASK_TYPE_LABELS: &str =
    formatcp!("{SUMMARY}/total_time_by_task_type_labels");
pub const SUMM_REBUILD_COUNT: &str = formatcp!("{SUMMARY}/rebuild_count");
pub const SUMM_REBUILD_TIME: &str = formatcp!("{SUMMARY}/rebuild_time_s");
pub const SUMM_APPLY_COUNT: &str = formatcp!("{SUMMARY}/apply_count");
pub const SUMM_TASKS: &str = formatcp!("{SUMMARY}/tasks");

/// Metrics collected for each task execution, which can be logged to Rerun and aggregated into summaries.
pub(crate) struct TaskMetrics {
    pub(crate) egraph_nodes: usize,
    pub(crate) egraph_classes: usize,
    pub(crate) task_type: &'static str,
    pub(crate) task_time_s: f64,
    pub(crate) rebuild_count: usize,
    pub(crate) rebuild_time_s: f64,
    pub(crate) optimized_memo_pairs: usize,
    // TODO: add more fields here for other relevant task metrics, like number of rules applied, etc
}

/// Aggregated metrics across multiple tasks, which can be used to generate summaries and visualizations in Rerun.
pub(crate) struct SummaryMetrics {
    egraph_nodes: usize,
    egraph_classes: usize,
    total_time_s: f64,
    total_tasks: usize,
    total_rebuild_count: usize,
    total_rebuild_time_s: f64,
    total_time_optimize_group_s: f64,
    total_time_optimize_expr_s: f64,
    total_time_explore_group_s: f64,
    total_time_explore_children_s: f64,
}

impl SummaryMetrics {
    pub fn new() -> Self {
        Self {
            egraph_nodes: 0,
            egraph_classes: 0,
            total_time_s: 0.0,
            total_tasks: 0,
            total_rebuild_count: 0,
            total_rebuild_time_s: 0.0,
            total_time_optimize_group_s: 0.0,
            total_time_optimize_expr_s: 0.0,
            total_time_explore_group_s: 0.0,
            total_time_explore_children_s: 0.0,
        }
    }

    /// Records the metrics from a single task execution into the summary, updating aggregate counts and times.
    pub fn record_task(&mut self, task_metrics: &TaskMetrics) {
        self.egraph_nodes = task_metrics.egraph_nodes;
        self.egraph_classes = task_metrics.egraph_classes;
        self.total_time_s += task_metrics.task_time_s;
        self.total_tasks += 1;
        self.total_rebuild_count += task_metrics.rebuild_count;
        self.total_rebuild_time_s += task_metrics.rebuild_time_s;

        match task_metrics.task_type {
            "OptimizeGroup" => self.total_time_optimize_group_s += task_metrics.task_time_s,
            "OptimizeExpr" => self.total_time_optimize_expr_s += task_metrics.task_time_s,
            "ExploreGroup" => self.total_time_explore_group_s += task_metrics.task_time_s,
            "ExploreChildren" => self.total_time_explore_children_s += task_metrics.task_time_s,
            _ => {}
        }
    }
}

/// Logs the metrics from a single task execution to Rerun, as well as updating the summary metrics.
/// This function handles logging individual scalar metrics for the task, as well as the aggregate summary metrics that can be visualized in Rerun.
pub(crate) fn log_rerun_task(
    rec: &RerunStream,
    task_metrics: &TaskMetrics,
    summary_metrics: &mut SummaryMetrics,
) {
    summary_metrics.record_task(task_metrics);

    if rec.is_none() {
        warn!(
            "Attempted to log rerun metrics for task '{}', but no recording stream is available",
            task_metrics.task_type
        );
        return;
    }

    log_scalar(rec, TASK_TOTAL_TIME, task_metrics.task_time_s);
    log_scalar(rec, TASK_REBUILD_COUNT, task_metrics.rebuild_count as f64);
    log_scalar(rec, TASK_REBUILD_TIME, task_metrics.rebuild_time_s);
    log_scalar(
        rec,
        TASK_OPTIMIZED_MEMO_PAIRS,
        task_metrics.optimized_memo_pairs as f64,
    );

    let (opt_group, opt_expr, explore_group, explore_children) = task_type_task_times(task_metrics);
    log_scalar(rec, TASK_TIME_OPTIMIZE_GROUP, opt_group);
    log_scalar(rec, TASK_TIME_OPTIMIZE_EXPR, opt_expr);
    log_scalar(rec, TASK_TIME_EXPLORE_GROUP, explore_group);
    log_scalar(rec, TASK_TIME_EXPLORE_CHILDREN, explore_children);

    log_scalar(rec, SUMM_NODES, summary_metrics.egraph_nodes as f64);
    log_scalar(rec, SUMM_CLASSES, summary_metrics.egraph_classes as f64);
    log_scalar(rec, SUMM_TOTAL_TIME, summary_metrics.total_time_s);
    log_scalar(
        rec,
        SUMM_TOTAL_TIME_OPTIMIZE_GROUP,
        summary_metrics.total_time_optimize_group_s,
    );
    log_scalar(
        rec,
        SUMM_TOTAL_TIME_OPTIMIZE_EXPR,
        summary_metrics.total_time_optimize_expr_s,
    );
    log_scalar(
        rec,
        SUMM_TOTAL_TIME_EXPLORE_GROUP,
        summary_metrics.total_time_explore_group_s,
    );
    log_scalar(
        rec,
        SUMM_TOTAL_TIME_EXPLORE_CHILDREN,
        summary_metrics.total_time_explore_children_s,
    );
    log_scalar(
        rec,
        SUMM_REBUILD_COUNT,
        summary_metrics.total_rebuild_count as f64,
    );
    log_scalar(rec, SUMM_REBUILD_TIME, summary_metrics.total_rebuild_time_s);
    log_scalar(rec, SUMM_TASKS, summary_metrics.total_tasks as f64);
}

/// Logs a summary of the aggregated metrics to Rerun, including a bar chart of total time by task type.
pub(crate) fn log_rerun_summary(rec: &RerunStream, summary_metrics: &SummaryMetrics) {
    if rec.is_none() {
        warn!("Attempted to log rerun summary metrics, but no recording stream is available");
        return;
    }

    let values = [
        summary_metrics.total_time_optimize_group_s,
        summary_metrics.total_time_optimize_expr_s,
        summary_metrics.total_time_explore_group_s,
        summary_metrics.total_time_explore_children_s,
    ];
    let abscissa = [0_i64, 1, 2, 3];

    if let Err(err) = rec.as_ref().unwrap().log(
        SUMM_TOTAL_TIME_BY_TASK_TYPE_BAR,
        &rerun::BarChart::new(values.as_slice()).with_abscissa(abscissa.as_slice()),
    ) {
        warn!(
            "Failed to emit rerun bar chart '{}' with values {:?}: {}",
            SUMM_TOTAL_TIME_BY_TASK_TYPE_BAR, values, err
        );
    }

    let label_mapping = "0=OptimizeGroup, 1=OptimizeExpr, 2=ExploreGroup, 3=ExploreChildren";
    if let Err(err) = rec.as_ref().unwrap().log(
        SUMM_TOTAL_TIME_BY_TASK_TYPE_LABELS,
        &rerun::TextLog::new(label_mapping),
    ) {
        warn!(
            "Failed to emit rerun bar chart labels '{}' with value '{}': {}",
            SUMM_TOTAL_TIME_BY_TASK_TYPE_LABELS, label_mapping, err
        );
    }
}

fn task_type_task_times(task_metrics: &TaskMetrics) -> (f64, f64, f64, f64) {
    match task_metrics.task_type {
        "OptimizeGroup" => (task_metrics.task_time_s, 0.0, 0.0, 0.0),
        "OptimizeExpr" => (0.0, task_metrics.task_time_s, 0.0, 0.0),
        "ExploreGroup" => (0.0, 0.0, task_metrics.task_time_s, 0.0),
        "ExploreChildren" => (0.0, 0.0, 0.0, task_metrics.task_time_s),
        _ => (0.0, 0.0, 0.0, 0.0),
    }
}

/// Helper function to log a single scalar metric to Rerun, with error handling and logging if the recording stream is not available or if logging fails.
fn log_scalar(rec: &RerunStream, entity_path: &str, value: f64) {
    if rec.is_none() {
        warn!(
            "Attempted to log rerun scalar '{}' with value {}, but no recording stream is available",
            entity_path, value
        );
        return;
    }

    if let Err(err) = rec
        .as_ref()
        .unwrap()
        .log(entity_path, &rerun::Scalars::new([value]))
    {
        warn!(
            "Failed to emit rerun scalar '{}' with value {}: {}",
            entity_path, value, err
        );
    }
}
