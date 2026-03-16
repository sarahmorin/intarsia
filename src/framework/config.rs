//! Configuration for the Intarsia optimizer.
//!
//! This module defines the Configuration struct for the Intarsia optimizer, which holds all the necessary
//! settings and parameters for the optimization process. It also includes methods for loading and validating
//! the configuration from a file.

use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    pub(crate) time_limit: Option<Duration>,
    pub(crate) node_limit: Option<usize>,
    pub(crate) task_limit: Option<usize>,
}

impl Config {
    pub fn new() -> Self {
        Self {
            time_limit: None,
            node_limit: None,
            task_limit: None,
        }
    }

    pub fn with_time_limit(mut self, limit: Duration) -> Self {
        self.time_limit = Some(limit);
        self
    }

    pub fn with_node_limit(mut self, limit: usize) -> Self {
        self.node_limit = Some(limit);
        self
    }

    pub fn with_task_limit(mut self, limit: usize) -> Self {
        self.task_limit = Some(limit);
        self
    }

    pub fn from_file(_path: &str) -> Result<Self, String> {
        // Implement loading from a file (e.g., JSON, YAML)
        // For now, we can return an error indicating this is not implemented
        Err("Loading from file not implemented yet".to_string())
    }

    // Additional methods for loading from file, validating config, etc. can be added here
}
