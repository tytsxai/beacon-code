//! Task pipeline for managing task flow through the multi-agent system.
//!
//! Provides staged execution: Planning → Implementation → Testing → Review

use std::collections::HashMap;
use std::collections::HashSet;
use std::time::Instant;
use thiserror::Error;
use uuid::Uuid;

use crate::scheduler::AgentId;
use crate::scheduler::AgentTask;

/// Pipeline stage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PipelineStage {
    /// Initial stage - task received
    Queued,
    /// Planning stage - Coordinator + Architect
    Planning,
    /// Implementation stage - Executors working
    Implementing,
    /// Testing stage - Tester + Debugger
    Testing,
    /// Review stage - Reviewer merging
    Reviewing,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed,
}

impl PipelineStage {
    /// Returns the next stage in the pipeline
    pub fn next(&self) -> Option<Self> {
        match self {
            Self::Queued => Some(Self::Planning),
            Self::Planning => Some(Self::Implementing),
            Self::Implementing => Some(Self::Testing),
            Self::Testing => Some(Self::Reviewing),
            Self::Reviewing => Some(Self::Completed),
            Self::Completed | Self::Failed => None,
        }
    }

    /// Returns roles active in this stage
    pub fn active_roles(&self) -> &'static [&'static str] {
        match self {
            Self::Queued => &[],
            Self::Planning => &["Coordinator", "Architect"],
            Self::Implementing => &["Executor-1", "Executor-2", "Executor-3"],
            Self::Testing => &["Tester", "Debugger"],
            Self::Reviewing => &["Reviewer"],
            Self::Completed | Self::Failed => &[],
        }
    }

    /// Whether this stage can run in parallel with others
    pub fn is_parallel(&self) -> bool {
        matches!(self, Self::Implementing)
    }
}

/// A task moving through the pipeline
#[derive(Debug, Clone)]
pub struct PipelineTask {
    /// Unique task ID
    pub id: String,
    /// Task description
    pub description: String,
    /// Current stage
    pub stage: PipelineStage,
    /// Stage outputs
    pub stage_outputs: HashMap<PipelineStage, StageOutput>,
    /// Creation time
    pub created_at: Instant,
    /// Last stage change time
    pub stage_changed_at: Instant,
    /// Total retries
    pub retries: i64,
    /// Per-stage role results
    pub role_results: HashMap<PipelineStage, HashMap<String, RoleResult>>,
}

/// Output from a pipeline stage
#[derive(Debug, Clone)]
pub struct StageOutput {
    /// Stage that produced this output
    pub stage: PipelineStage,
    /// Content/result from the stage
    pub content: String,
    /// Tokens consumed
    pub tokens_used: i64,
    /// Duration of the stage
    pub duration_ms: i64,
    /// Whether stage succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Result emitted by an individual role within a stage
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleResult {
    pub output: String,
    pub success: bool,
}

/// Errors that can occur while manipulating pipeline tasks.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PipelineError {
    #[error("task {task_id} not found")]
    TaskNotFound { task_id: String },
    #[error("invalid stage transition from {from:?} to {to:?}")]
    InvalidTransition {
        from: PipelineStage,
        to: PipelineStage,
    },
    #[error("role {role} failed: {error}")]
    RoleFailed { role: String, error: String },
}

/// Action to take after handling a role completion
#[derive(Debug, PartialEq, Eq)]
pub enum StageAction {
    Advance(PipelineStage),
    Wait,
    Fail { role: String, error: String },
}

impl PipelineTask {
    /// Creates a new pipeline task
    pub fn new(id: impl Into<String>, description: impl Into<String>) -> Self {
        let now = Instant::now();
        Self {
            id: id.into(),
            description: description.into(),
            stage: PipelineStage::Queued,
            stage_outputs: HashMap::new(),
            created_at: now,
            stage_changed_at: now,
            retries: 0,
            role_results: HashMap::new(),
        }
    }

    /// Advances to the next stage
    pub fn advance(&mut self) -> bool {
        if let Some(next) = self.stage.next() {
            self.stage = next;
            self.stage_changed_at = Instant::now();
            true
        } else {
            false
        }
    }

    /// Records output for the current stage
    pub fn record_output(&mut self, output: StageOutput) {
        self.stage_outputs.insert(output.stage, output);
    }

    /// Marks the task as failed
    pub fn fail(&mut self, error: impl Into<String>) {
        self.stage_outputs.insert(
            self.stage,
            StageOutput {
                stage: self.stage,
                content: String::new(),
                tokens_used: 0,
                duration_ms: elapsed_ms(self.stage_changed_at),
                success: false,
                error: Some(error.into()),
            },
        );
        self.stage = PipelineStage::Failed;
        self.stage_changed_at = Instant::now();
    }

    /// Returns total tokens used across all stages
    pub fn total_tokens(&self) -> i64 {
        self.stage_outputs
            .values()
            .map(|o| o.tokens_used)
            .sum::<i64>()
    }

    /// Returns total duration in milliseconds
    pub fn total_duration_ms(&self) -> i64 {
        elapsed_ms(self.created_at)
    }

    /// Whether the task is complete (success or failure)
    pub fn is_terminal(&self) -> bool {
        matches!(self.stage, PipelineStage::Completed | PipelineStage::Failed)
    }
}

fn elapsed_ms(start: Instant) -> i64 {
    let millis = start.elapsed().as_millis();
    if millis > i64::MAX as u128 {
        i64::MAX
    } else {
        millis as i64
    }
}

/// Pipeline manager for tracking multiple tasks
pub struct TaskPipeline {
    tasks: HashMap<String, PipelineTask>,
    stage_counts: HashMap<PipelineStage, i64>,
    next_agent_id: i64,
}

impl TaskPipeline {
    /// Creates a new pipeline manager
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            stage_counts: HashMap::new(),
            next_agent_id: 1,
        }
    }

    /// Creates and registers a new task from a goal description.
    pub fn create_from_goal(&mut self, goal: &str) -> String {
        let id = Uuid::new_v4().to_string();
        let task = PipelineTask::new(id.clone(), goal);
        self.add(task);
        id
    }

    /// Adds a task to the pipeline
    pub fn add(&mut self, task: PipelineTask) {
        *self.stage_counts.entry(task.stage).or_insert(0) += 1;
        self.tasks.insert(task.id.clone(), task);
    }

    /// Gets a task by ID
    pub fn get(&self, id: &str) -> Option<&PipelineTask> {
        self.tasks.get(id)
    }

    /// Gets a mutable task by ID
    pub fn get_mut(&mut self, id: &str) -> Option<&mut PipelineTask> {
        self.tasks.get_mut(id)
    }

    /// Generates agent tasks for the active roles of the current stage.
    pub fn get_stage_tasks(&mut self, id: &str) -> Result<Vec<AgentTask>, PipelineError> {
        let task = self
            .tasks
            .get(id)
            .ok_or_else(|| PipelineError::TaskNotFound {
                task_id: id.to_string(),
            })?;

        let mut tasks = Vec::new();
        for (idx, role) in task.stage.active_roles().iter().enumerate() {
            let agent_id = self.next_agent_id.max(1) as u64;
            self.next_agent_id = self.next_agent_id.saturating_add(1);
            tasks.push(AgentTask {
                id: AgentId(agent_id),
                prompt: format!("[{}] {}", role, task.description),
                context: None,
                write_access: role.starts_with("Executor"),
                models: None,
                dispatch_order: idx,
            });
        }

        Ok(tasks)
    }

    /// Records a role result and determines the next pipeline action.
    pub fn handle_role_complete(
        &mut self,
        task_id: &str,
        role: &str,
        result: &str,
        success: bool,
    ) -> Result<StageAction, PipelineError> {
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| PipelineError::TaskNotFound {
                task_id: task_id.to_string(),
            })?;

        let stage = task.stage;
        let progress = task.role_results.entry(stage).or_default();
        progress.insert(
            role.to_string(),
            RoleResult {
                output: result.to_string(),
                success,
            },
        );

        let mut stage_action = StageAction::Wait;
        let mut new_stage: Option<PipelineStage> = None;

        if !success {
            let error = result.to_string();
            task.fail(error.clone());
            new_stage = Some(task.stage);
            stage_action = StageAction::Fail {
                role: role.to_string(),
                error,
            };
        } else {
            let active_roles: HashSet<String> = stage
                .active_roles()
                .iter()
                .map(std::string::ToString::to_string)
                .collect();
            let completed_roles: HashSet<String> = progress.keys().cloned().collect();

            if !active_roles.is_empty() && active_roles.is_subset(&completed_roles) {
                let content = progress
                    .iter()
                    .map(|(name, res)| format!("{name}: {}", res.output))
                    .collect::<Vec<_>>()
                    .join("\n");
                let duration_ms = elapsed_ms(task.stage_changed_at);
                task.record_output(StageOutput {
                    stage,
                    content,
                    tokens_used: 0,
                    duration_ms,
                    success: true,
                    error: None,
                });

                if task.advance() {
                    new_stage = Some(task.stage);
                    stage_action = StageAction::Advance(task.stage);
                }
            }
        }

        let _ = task;
        if let Some(next_stage) = new_stage {
            self.update_stage_counts(stage, next_stage);
        }

        Ok(stage_action)
    }

    /// Advances a task to the next stage
    pub fn advance(&mut self, id: &str) -> bool {
        if let Some(task) = self.tasks.get_mut(id) {
            let old_stage = task.stage;
            let advanced = task.advance();
            let new_stage = task.stage;
            let _ = task;
            if advanced {
                self.update_stage_counts(old_stage, new_stage);
                return true;
            }
        }
        false
    }

    /// Returns tasks at a specific stage
    pub fn tasks_at_stage(&self, stage: PipelineStage) -> Vec<&PipelineTask> {
        self.tasks.values().filter(|t| t.stage == stage).collect()
    }

    /// Returns count of tasks at each stage
    pub fn stage_counts(&self) -> &HashMap<PipelineStage, i64> {
        &self.stage_counts
    }

    /// Removes completed/failed tasks and returns them
    pub fn drain_terminal(&mut self) -> Vec<PipelineTask> {
        let terminal_ids: Vec<_> = self
            .tasks
            .iter()
            .filter(|(_, t)| t.is_terminal())
            .map(|(id, _)| id.clone())
            .collect();

        let mut result = Vec::new();
        for id in terminal_ids {
            if let Some(task) = self.tasks.remove(&id) {
                if let Some(count) = self.stage_counts.get_mut(&task.stage) {
                    *count = count.saturating_sub(1);
                }
                result.push(task);
            }
        }
        result
    }

    fn update_stage_counts(&mut self, old_stage: PipelineStage, new_stage: PipelineStage) {
        if old_stage == new_stage {
            return;
        }
        if let Some(count) = self.stage_counts.get_mut(&old_stage) {
            *count = count.saturating_sub(1);
        }
        *self.stage_counts.entry(new_stage).or_insert(0) += 1;
    }
}

impl Default for TaskPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use proptest::prelude::*;

    #[test]
    fn test_stage_progression() {
        let mut task = PipelineTask::new("t1", "Test task");
        assert_eq!(task.stage, PipelineStage::Queued);

        assert!(task.advance());
        assert_eq!(task.stage, PipelineStage::Planning);

        assert!(task.advance());
        assert_eq!(task.stage, PipelineStage::Implementing);

        assert!(task.advance());
        assert_eq!(task.stage, PipelineStage::Testing);

        assert!(task.advance());
        assert_eq!(task.stage, PipelineStage::Reviewing);

        assert!(task.advance());
        assert_eq!(task.stage, PipelineStage::Completed);

        assert!(!task.advance()); // No more stages
    }

    #[test]
    fn test_pipeline_manager() {
        let mut pipeline = TaskPipeline::new();

        pipeline.add(PipelineTask::new("t1", "Task 1"));
        pipeline.add(PipelineTask::new("t2", "Task 2"));

        assert_eq!(pipeline.tasks_at_stage(PipelineStage::Queued).len(), 2);

        pipeline.advance("t1");
        assert_eq!(pipeline.tasks_at_stage(PipelineStage::Queued).len(), 1);
        assert_eq!(pipeline.tasks_at_stage(PipelineStage::Planning).len(), 1);
    }

    #[test]
    fn test_create_from_goal_and_stage_tasks() {
        let mut pipeline = TaskPipeline::new();
        let id = pipeline.create_from_goal("Build feature");
        let tasks = pipeline.get_stage_tasks(&id).unwrap();

        assert_eq!(tasks.len(), 0);

        pipeline.advance(&id);
        let planning_tasks = pipeline.get_stage_tasks(&id).unwrap();
        assert_eq!(
            planning_tasks.len(),
            PipelineStage::Planning.active_roles().len()
        );
    }

    #[test]
    fn handle_role_complete_advances_stage_when_all_roles_done() {
        let mut pipeline = TaskPipeline::new();
        let id = pipeline.create_from_goal("Goal");
        pipeline.advance(&id); // Planning

        let roles = PipelineStage::Planning.active_roles();
        for role in roles {
            let action = pipeline
                .handle_role_complete(&id, role, "ok", true)
                .expect("role handled");
            if role == roles.last().unwrap() {
                assert!(matches!(
                    action,
                    StageAction::Advance(PipelineStage::Implementing)
                ));
            }
        }

        let task = pipeline.get(&id).unwrap();
        assert_eq!(task.stage, PipelineStage::Implementing);
    }

    #[test]
    fn handle_role_complete_marks_failure() {
        let mut pipeline = TaskPipeline::new();
        let id = pipeline.create_from_goal("Goal");
        pipeline.advance(&id); // Planning

        let action = pipeline
            .handle_role_complete(&id, "Coordinator", "error", false)
            .unwrap();
        assert!(matches!(action, StageAction::Fail { .. }));

        let task = pipeline.get(&id).unwrap();
        assert_eq!(task.stage, PipelineStage::Failed);
    }

    #[test]
    fn test_task_failure() {
        let mut task = PipelineTask::new("t1", "Test");
        task.advance(); // Planning
        task.fail("Something went wrong");

        assert_eq!(task.stage, PipelineStage::Failed);
        assert!(task.is_terminal());
    }

    proptest! {
        #[test]
        fn property_stage_progression(stage_index in 1usize..5usize) {
            let mut pipeline = TaskPipeline::new();
            let id = pipeline.create_from_goal("Goal");

            for _ in 0..stage_index {
                pipeline.advance(&id);
            }

            let current_stage = pipeline.get(&id).unwrap().stage;
            let roles = current_stage.active_roles();
            let mut last_action = StageAction::Wait;
            for role in roles {
                last_action = pipeline.handle_role_complete(&id, role, "done", true).unwrap();
            }

            let expected = current_stage.next().unwrap_or(current_stage);
            let task = pipeline.get(&id).unwrap();
            prop_assert!(matches!(last_action, StageAction::Advance(_)));
            prop_assert_eq!(task.stage, expected);
        }
    }
}
