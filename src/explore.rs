// Generic exploration module
// Provides trait definitions for exploration strategies over e-graphs
// and common data structures for profiling and statistics.

use crate::rule::Rule;
use crate::types::*;

/// Represents a rule match within an e-graph
/// Holds the rule, the e-class where it matched, and the set of substitutions
pub struct RuleMatch<L>
where
    L: OpLang,
{
    // TODO: Optimize later
    pub rule: Rule<L>,
    pub eclass: Id,
    pub enode: Id,
    pub subst_set: Vec<Subst<Var, Id>>,
}

// TODO: Partial rule matches

/// Represents the result of a single exploration step and stopping reason
/// Most steps simple Continue, but when we terminate its useful to know why.
pub enum StopResult {
    Continue,
    Saturated,
    MaxIterations,
    NoTasks,
}

/// The Explorer trait defines the interface for exploration strategies over e-graphs
///
/// Implementors of this trait should define how to run a single step of exploration
/// over the e-graph and a function to record statistics or progress.
/// Then they can use the run method to execute multiple steps.
///
/// Additionally, the explorer requires maintaining state and statistics.
pub trait Explorer {
    /// Run a single step of exploration over the e-graph
    fn run_step(&mut self) -> StopResult;

    /// Update statistics or progress information after a step
    /// This can be used to extract metrics from the e-graph, log progress, etc.
    /// It can also include periodic displays of the current state for testing.
    fn update_stats(&mut self);

    fn run(&mut self, steps: usize) -> StopResult {
        for _ in 0..steps {
            let res = self.run_step();
            self.update_stats();
            match res {
                StopResult::Continue => {
                    continue;
                }
                _ => {
                    return res;
                }
            }
        }

        StopResult::MaxIterations
    }
}
