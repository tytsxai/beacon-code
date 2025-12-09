//! Parallel execution module for same-model concurrent Auto Drive.
//!
//! When `parallel_instances > 1` in `AutoDriveSettings`, this module enables
//! dispatching multiple concurrent API calls to the same model with different
//! role prompts (coordinator, executor, reviewer).

use std::sync::Arc;

use anyhow::Result;
use code_core::ModelClient;
use futures::future::join_all;

/// Role definition for parallel instance execution
#[derive(Debug, Clone)]
pub enum ParallelRole {
    /// Primary coordinator role - orchestrates the overall task
    Coordinator,
    /// Executor role - implements code changes
    Executor,
    /// Reviewer role - reviews and validates changes
    Reviewer,
    /// QA role - tests and validates functionality
    QaAutomation,
    /// Cross-check role - provides second opinion
    CrossCheck,
}

impl ParallelRole {
    /// Returns the role-specific prompt prefix
    pub fn prompt_prefix(&self) -> &'static str {
        match self {
            Self::Coordinator => "As the COORDINATOR, orchestrate the task:",
            Self::Executor => "As the EXECUTOR, implement the following:",
            Self::Reviewer => "As the REVIEWER, check for issues in:",
            Self::QaAutomation => "As QA, verify and test:",
            Self::CrossCheck => "As CROSS-CHECKER, validate the approach:",
        }
    }

    /// Returns roles for a given parallel instance count
    pub fn roles_for_count(count: u8) -> Vec<Self> {
        match count.min(5) {
            1 => vec![Self::Coordinator],
            2 => vec![Self::Coordinator, Self::Executor],
            3 => vec![Self::Coordinator, Self::Executor, Self::Reviewer],
            4 => vec![
                Self::Coordinator,
                Self::Executor,
                Self::Reviewer,
                Self::QaAutomation,
            ],
            5 | _ => vec![
                Self::Coordinator,
                Self::Executor,
                Self::Reviewer,
                Self::QaAutomation,
                Self::CrossCheck,
            ],
        }
    }
}

/// Result from a parallel execution instance
#[derive(Debug)]
pub struct ParallelResult {
    pub role: ParallelRole,
    pub response: String,
    pub success: bool,
}

/// Configuration for parallel execution
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// Number of parallel instances (1-5)
    pub instance_count: u8,
    /// Base prompt to send to all instances
    pub base_prompt: String,
    /// Model to use for all instances
    pub model: String,
}

impl ParallelConfig {
    /// Create config from AutoDriveSettings.parallel_instances
    pub fn from_instances(count: u8, base_prompt: String, model: String) -> Self {
        Self {
            instance_count: count.clamp(1, 5),
            base_prompt,
            model,
        }
    }

    /// Returns true if parallel execution is enabled (count > 1)
    pub fn is_parallel(&self) -> bool {
        self.instance_count > 1
    }

    /// Get roles for this configuration
    pub fn roles(&self) -> Vec<ParallelRole> {
        ParallelRole::roles_for_count(self.instance_count)
    }
}

/// Execute parallel instances using the same model with different roles.
/// 
/// This function spawns multiple concurrent API calls, each with a role-specific
/// prompt prefix, and collects results from all instances.
pub async fn execute_parallel(
    _client: Arc<ModelClient>,
    config: &ParallelConfig,
) -> Result<Vec<ParallelResult>> {
    if !config.is_parallel() {
        // Single instance mode - no parallelization needed
        return Ok(vec![ParallelResult {
            role: ParallelRole::Coordinator,
            response: String::new(),
            success: true,
        }]);
    }

    let roles = config.roles();
    let futures: Vec<_> = roles
        .into_iter()
        .map(|role| {
            let _prompt = format!("{} {}", role.prompt_prefix(), config.base_prompt);
            async move {
                // TODO: Implement actual API call with role-specific prompt
                // For now, return placeholder result
                ParallelResult {
                    role,
                    response: String::new(),
                    success: true,
                }
            }
        })
        .collect();

    let results = join_all(futures).await;
    Ok(results)
}

/// Merge results from parallel execution into a unified response.
/// 
/// Uses the coordinator's output as the primary response, with insights
/// from other roles incorporated as additional context.
pub fn merge_parallel_results(results: Vec<ParallelResult>) -> String {
    let coordinator_result = results
        .iter()
        .find(|r| matches!(r.role, ParallelRole::Coordinator));

    if let Some(coord) = coordinator_result {
        // Primary response is from coordinator
        let mut merged = coord.response.clone();

        // Append insights from other roles
        for result in results.iter() {
            if !matches!(result.role, ParallelRole::Coordinator) && result.success {
                if !result.response.is_empty() {
                    merged.push_str(&format!(
                        "\n\n[{:?} Insight]: {}",
                        result.role, result.response
                    ));
                }
            }
        }

        merged
    } else {
        // Fallback: concatenate all results
        results
            .iter()
            .map(|r| r.response.as_str())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roles_for_count() {
        assert_eq!(ParallelRole::roles_for_count(1).len(), 1);
        assert_eq!(ParallelRole::roles_for_count(3).len(), 3);
        assert_eq!(ParallelRole::roles_for_count(5).len(), 5);
        // Clamped to max 5
        assert_eq!(ParallelRole::roles_for_count(10).len(), 5);
    }

    #[test]
    fn test_parallel_config() {
        let config = ParallelConfig::from_instances(3, "test".into(), "gpt-5.1".into());
        assert!(config.is_parallel());
        assert_eq!(config.roles().len(), 3);

        let single = ParallelConfig::from_instances(1, "test".into(), "gpt-5.1".into());
        assert!(!single.is_parallel());
    }
}
