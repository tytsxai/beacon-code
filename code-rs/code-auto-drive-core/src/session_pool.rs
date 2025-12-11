//! Session pool for managing multiple concurrent Auto Drive sessions.
//!
//! This module provides high-throughput execution by managing a pool of
//! concurrent sessions, each running with parallel role instances.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use thiserror::Error;
use tokio::sync::Mutex;
use tokio::sync::OwnedSemaphorePermit;
use tokio::sync::RwLock;
use tokio::sync::Semaphore;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::budget::BudgetAlert;
/// Session state in the pool
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Available for new tasks
    Idle,
    /// Currently executing a task
    Running,
    /// Running but slower than expected
    Slow,
    /// Stuck and needs intervention
    Stuck,
    /// Encountered an error
    Error,
    /// Gracefully shutting down
    ShuttingDown,
}

/// Configuration for the session pool
#[derive(Debug, Clone)]
pub struct SessionPoolConfig {
    /// Maximum number of concurrent sessions
    pub max_sessions: i32,
    /// Minimum number of sessions to keep warm
    pub min_sessions: i32,
    /// Threshold for scaling up (0.0-1.0)
    pub scale_up_threshold: f64,
    /// Threshold for scaling down (0.0-1.0)
    pub scale_down_threshold: f64,
    /// Time before a session is considered slow
    pub slow_threshold: Duration,
    /// Time before a session is considered stuck
    pub stuck_threshold: Duration,
    /// Maximum retries for failed tasks
    pub max_retries: i32,
    /// Threshold for backpressure rejection
    pub backpressure_threshold: i32,
}

impl Default for SessionPoolConfig {
    fn default() -> Self {
        Self {
            max_sessions: 20,
            min_sessions: 5,
            scale_up_threshold: 0.8,
            scale_down_threshold: 0.3,
            slow_threshold: Duration::from_secs(120),
            stuck_threshold: Duration::from_secs(300),
            max_retries: 3,
            backpressure_threshold: 200,
        }
    }
}

/// A task to be executed by a session
#[derive(Debug, Clone)]
pub struct PoolTask {
    /// Unique task identifier
    pub id: String,
    /// Task prompt/description
    pub prompt: String,
    /// Priority level (higher = more urgent)
    pub priority: i32,
    /// Number of retry attempts
    pub retries: i32,
    /// Creation timestamp
    pub created_at: Instant,
}

impl PoolTask {
    pub fn new(id: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            prompt: prompt.into(),
            priority: 1,
            retries: 0,
            created_at: Instant::now(),
        }
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

/// Result from a completed session task
#[derive(Debug, Clone)]
pub struct TaskResult {
    /// Task ID
    pub task_id: String,
    /// Session ID that executed this task
    pub session_id: String,
    /// Whether the task succeeded
    pub success: bool,
    /// Result content or error message
    pub content: String,
    /// Tokens consumed
    pub tokens_used: i64,
    /// Execution duration
    pub duration: Duration,
}

/// Information about a session in the pool
#[derive(Debug)]
struct SessionInfo {
    id: String,
    state: SessionState,
    current_task: Option<PoolTask>,
    started_at: Option<Instant>,
    tasks_completed: i32,
    tokens_used: i64,
    _errors: i32,
    permit: Option<OwnedSemaphorePermit>,
}

impl SessionInfo {
    fn new(id: String) -> Self {
        Self {
            id,
            state: SessionState::Idle,
            current_task: None,
            started_at: None,
            tasks_completed: 0,
            tokens_used: 0,
            _errors: 0,
            permit: None,
        }
    }
}

/// Priority queue for tasks
struct TaskQueue {
    /// Tasks sorted by priority
    queues: [VecDeque<PoolTask>; 3], // High, Normal, Low
    total: i32,
}

impl TaskQueue {
    fn new() -> Self {
        Self {
            queues: [VecDeque::new(), VecDeque::new(), VecDeque::new()],
            total: 0,
        }
    }

    fn push(&mut self, task: PoolTask) {
        let idx = match task.priority {
            3.. => 0, // High priority
            2 => 1,   // Normal priority
            _ => 2,   // Low priority
        };
        self.queues[idx].push_back(task);
        self.total = self.total.saturating_add(1);
    }

    fn push_front(&mut self, task: PoolTask) {
        let idx = match task.priority {
            3.. => 0,
            2 => 1,
            _ => 2,
        };
        self.queues[idx].push_front(task);
        self.total = self.total.saturating_add(1);
    }

    fn pop(&mut self) -> Option<PoolTask> {
        for queue in &mut self.queues {
            if let Some(task) = queue.pop_front() {
                self.total = self.total.saturating_sub(1);
                return Some(task);
            }
        }
        None
    }

    fn len(&self) -> i32 {
        self.total
    }
}

/// Metrics for the session pool
#[derive(Debug, Clone, Default)]
pub struct PoolMetrics {
    /// Total tasks submitted
    pub tasks_submitted: i64,
    /// Tasks completed successfully
    pub tasks_completed: i64,
    /// Tasks failed
    pub tasks_failed: i64,
    /// Total tokens consumed
    pub total_tokens: i64,
    /// Current queue size
    pub queue_size: i32,
    /// Active sessions
    pub active_sessions: i32,
    /// Idle sessions
    pub idle_sessions: i32,
    /// Average task duration in ms
    pub avg_task_duration_ms: i64,
    /// Average queue latency in ms
    pub avg_queue_latency_ms: i64,
    /// Total retry attempts performed
    pub retry_count: i32,
    /// Total failures recorded
    pub failure_count: i32,
    /// Sessions marked stuck
    pub stuck_count: i32,
    /// Count of task migrations
    pub migration_count: i32,
    /// Number of backpressure warnings emitted
    pub backpressure_warnings: i32,
    /// Number of backpressure rejections emitted
    pub backpressure_rejections: i32,
}

/// Errors that can occur while operating the session pool
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PoolError {
    #[error("backpressure: queue full ({current}/{max})")]
    BackpressureFull { current: i32, max: i32 },
    #[error("no available sessions")]
    NoAvailableSessions,
    #[error("session {session_id} stuck")]
    SessionStuck { session_id: String },
    #[error("task {task_id} exceeded max retries ({retries}/{max})")]
    MaxRetriesExceeded {
        task_id: String,
        retries: i32,
        max: i32,
    },
    #[error("pool shutting down")]
    ShuttingDown,
}

/// Event emitted when a stuck session is migrated to another session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationEvent {
    pub from_session: String,
    pub to_session: Option<String>,
    pub task_id: String,
    pub retry_count: i32,
}

/// Health report returned by pool checks.
#[derive(Debug, Default, Clone)]
pub struct HealthReport {
    /// Sessions running slower than expected.
    pub slow_sessions: Vec<(String, i64, Option<String>)>,
    /// Sessions marked stuck.
    pub stuck_sessions: Vec<(String, i64, Option<String>)>,
    /// Migration events triggered by stuck sessions.
    pub migrations: Vec<MigrationEvent>,
}

/// The session pool manager
pub struct SessionPool {
    config: SessionPoolConfig,
    sessions: RwLock<HashMap<String, SessionInfo>>,
    task_queue: Mutex<TaskQueue>,
    result_tx: mpsc::Sender<TaskResult>,
    result_rx: Mutex<mpsc::Receiver<TaskResult>>,
    semaphore: Arc<Semaphore>,
    metrics: RwLock<PoolMetrics>,
    backpressure_alert: RwLock<Option<BudgetAlert>>,
    shutdown: RwLock<bool>,
}

impl SessionPool {
    /// Creates a new session pool with the given configuration
    pub fn new(config: SessionPoolConfig) -> Self {
        let mut config = config;
        let default_threshold = SessionPoolConfig::default().backpressure_threshold;
        if config.backpressure_threshold <= 0
            || (config.backpressure_threshold == default_threshold
                && config.max_sessions != SessionPoolConfig::default().max_sessions)
        {
            config.backpressure_threshold = config.max_sessions * 10;
        }

        let max_sessions = config.max_sessions.max(1) as usize;
        let (result_tx, result_rx) = mpsc::channel(max_sessions * 2);
        let semaphore = Arc::new(Semaphore::new(max_sessions));

        Self {
            config,
            sessions: RwLock::new(HashMap::new()),
            task_queue: Mutex::new(TaskQueue::new()),
            result_tx,
            result_rx: Mutex::new(result_rx),
            semaphore,
            metrics: RwLock::new(PoolMetrics::default()),
            backpressure_alert: RwLock::new(None),
            shutdown: RwLock::new(false),
        }
    }

    async fn refresh_metrics(&self) {
        let queue_size = self.task_queue.lock().await.len();
        let sessions = self.sessions.read().await;
        let active = sessions
            .values()
            .filter(|session| session.state == SessionState::Running)
            .count() as i32;
        let idle = sessions
            .values()
            .filter(|session| session.state == SessionState::Idle)
            .count() as i32;

        let mut metrics = self.metrics.write().await;
        metrics.queue_size = queue_size;
        metrics.active_sessions = active;
        metrics.idle_sessions = idle;
    }

    /// Submits a task to the pool
    pub async fn submit(&self, task: PoolTask) -> Result<(), PoolError> {
        if *self.shutdown.read().await {
            return Err(PoolError::ShuttingDown);
        }

        let threshold = self.config.backpressure_threshold.max(1);
        let warning_threshold = ((threshold as f64) * 0.8).ceil() as i32;

        let mut queue = self.task_queue.lock().await;

        let current = queue.len();
        let projected = current.saturating_add(1);
        if current >= threshold {
            {
                let mut metrics = self.metrics.write().await;
                metrics.backpressure_rejections = metrics.backpressure_rejections.saturating_add(1);
            }
            {
                let mut alert = self.backpressure_alert.write().await;
                *alert = Some(BudgetAlert::BackpressureExceeded {
                    queue_size: current,
                    limit: threshold,
                });
            }
            return Err(PoolError::BackpressureFull {
                current,
                max: threshold,
            });
        }

        if projected >= warning_threshold {
            {
                let mut metrics = self.metrics.write().await;
                metrics.backpressure_warnings = metrics.backpressure_warnings.saturating_add(1);
            }
            {
                let mut alert = self.backpressure_alert.write().await;
                *alert = Some(BudgetAlert::BackpressureWarning {
                    queue_size: projected,
                    limit: threshold,
                });
            }
            tracing::warn!(
                queue_size = current,
                threshold,
                "SessionPool backpressure approaching limit"
            );
        }

        queue.push(task);
        let queue_size = queue.len();
        drop(queue);

        {
            let mut metrics = self.metrics.write().await;
            metrics.tasks_submitted = metrics.tasks_submitted.saturating_add(1);
            metrics.queue_size = queue_size;
        }

        self.refresh_metrics().await;
        // Attempt to dispatch queued work immediately.
        let _ = self.dispatch_from_queue().await;

        Ok(())
    }

    /// Creates a new idle session and returns its ID.
    async fn create_session(&self) -> String {
        let id = Uuid::new_v4().to_string();
        let mut sessions = self.sessions.write().await;
        sessions.insert(id.clone(), SessionInfo::new(id.clone()));
        drop(sessions);
        self.refresh_metrics().await;
        id
    }

    /// Ensures the pool keeps the minimum number of warm sessions.
    pub async fn warmup(&self) {
        let target = self.config.min_sessions.max(0) as usize;
        loop {
            let current = self.sessions.read().await.len();
            if current >= target {
                break;
            }
            self.create_session().await;
        }
    }

    /// Attempts to dispatch the next queued task to an idle session.
    pub async fn dispatch_from_queue(&self) -> Result<Option<String>, PoolError> {
        let task = {
            let mut queue = self.task_queue.lock().await;
            queue.pop()
        };

        let Some(task) = task else {
            self.refresh_metrics().await;
            return Ok(None);
        };

        match self.dispatch_task(task.clone()).await {
            Ok(session_id) => Ok(Some(session_id)),
            Err(PoolError::NoAvailableSessions) => {
                {
                    let mut queue = self.task_queue.lock().await;
                    queue.push_front(task);
                }
                self.refresh_metrics().await;
                Ok(None)
            }
            Err(err) => {
                {
                    let mut queue = self.task_queue.lock().await;
                    queue.push_front(task);
                }
                self.refresh_metrics().await;
                Err(err)
            }
        }
    }

    /// Assigns a task to an idle session, creating one if capacity allows.
    pub async fn dispatch_task(&self, task: PoolTask) -> Result<String, PoolError> {
        if *self.shutdown.read().await {
            return Err(PoolError::ShuttingDown);
        }

        let mut permit = Some(
            self.semaphore
                .clone()
                .acquire_owned()
                .await
                .map_err(|_| PoolError::ShuttingDown)?,
        );

        let mut session_id = None;
        let queue_latency_ms = task.created_at.elapsed().as_millis() as i64;

        {
            let mut sessions = self.sessions.write().await;
            if let Some((id, session)) = sessions
                .iter_mut()
                .find(|(_, session)| session.state == SessionState::Idle)
            {
                session.state = SessionState::Running;
                session.current_task = Some(task.clone());
                session.started_at = Some(Instant::now());
                session.permit = permit.take();
                session_id = Some(id.clone());
            } else if sessions.len() < self.config.max_sessions.max(1) as usize {
                let id = Uuid::new_v4().to_string();
                let mut info = SessionInfo::new(id.clone());
                info.state = SessionState::Running;
                info.current_task = Some(task.clone());
                info.started_at = Some(Instant::now());
                info.permit = permit.take();
                sessions.insert(id.clone(), info);
                session_id = Some(id);
            }
        }

        if let Some(id) = session_id {
            {
                let mut metrics = self.metrics.write().await;
                let denominator = metrics.tasks_submitted.max(1);
                metrics.avg_queue_latency_ms = ((metrics.avg_queue_latency_ms * (denominator - 1))
                    + queue_latency_ms)
                    / denominator;
            }
            self.refresh_metrics().await;
            return Ok(id);
        }

        if let Some(permit) = permit {
            drop(permit);
        }
        Err(PoolError::NoAvailableSessions)
    }

    /// Finds the session currently running a specific task.
    pub async fn session_for_task(&self, task_id: &str) -> Option<String> {
        let sessions = self.sessions.read().await;
        sessions
            .iter()
            .find(|(_, session)| {
                session
                    .current_task
                    .as_ref()
                    .map(|task| task.id == task_id)
                    .unwrap_or(false)
            })
            .map(|(id, _)| id.clone())
    }

    /// Marks a session as complete and returns it to the idle pool.
    pub async fn complete_session(
        &self,
        session_id: &str,
        result: TaskResult,
    ) -> Result<(), PoolError> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.state = SessionState::Idle;
            session.current_task = None;
            session.started_at = None;
            session.tasks_completed = session.tasks_completed.saturating_add(1);
            session.tokens_used = session.tokens_used.saturating_add(result.tokens_used);
            session.permit.take();
        } else {
            return Err(PoolError::SessionStuck {
                session_id: session_id.to_string(),
            });
        }
        drop(sessions);

        {
            let mut metrics = self.metrics.write().await;
            if result.success {
                metrics.tasks_completed = metrics.tasks_completed.saturating_add(1);
            } else {
                metrics.tasks_failed = metrics.tasks_failed.saturating_add(1);
                metrics.failure_count = metrics.failure_count.saturating_add(1);
            }
            metrics.total_tokens = metrics.total_tokens.saturating_add(result.tokens_used);

            let completed = (metrics.tasks_completed + metrics.tasks_failed).max(1);
            let duration_ms = result.duration.as_millis() as i64;
            metrics.avg_task_duration_ms =
                ((metrics.avg_task_duration_ms * (completed - 1)) + duration_ms) / completed;
        }

        let _ = self.result_tx.send(result).await;
        self.refresh_metrics().await;
        let _ = self.dispatch_from_queue().await;
        Ok(())
    }

    /// Applies auto scaling based on current utilization.
    pub async fn auto_scale(&self) {
        let utilization = self.utilization().await;
        if utilization > self.config.scale_up_threshold {
            self.scale_up().await;
        } else if utilization < self.config.scale_down_threshold {
            self.scale_down().await;
        }
    }

    async fn scale_up(&self) {
        let target = self.config.max_sessions.max(0) as usize;
        loop {
            let current = self.sessions.read().await.len();
            if current >= target {
                break;
            }
            self.create_session().await;
        }
    }

    async fn scale_down(&self) {
        let min_sessions = self.config.min_sessions.max(0) as usize;
        let mut sessions = self.sessions.write().await;
        let current = sessions.len();
        if current <= min_sessions {
            return;
        }

        let removable = current.saturating_sub(min_sessions);
        let mut removed = 0;
        let idle_ids: Vec<String> = sessions
            .iter()
            .filter(|(_, session)| session.state == SessionState::Idle)
            .map(|(id, _)| id.clone())
            .collect();

        for id in idle_ids.into_iter().take(removable) {
            sessions.remove(&id);
            removed += 1;
        }
        drop(sessions);

        if removed > 0 {
            self.refresh_metrics().await;
        }
    }

    /// Migrates tasks from stuck sessions to healthy sessions.
    pub async fn migrate_stuck(&self) -> Vec<MigrationEvent> {
        let mut to_retry: Vec<(String, PoolTask)> = Vec::new();
        {
            let mut sessions = self.sessions.write().await;
            for (id, session) in sessions.iter_mut() {
                if session.state == SessionState::Stuck
                    && let Some(mut task) = session.current_task.take()
                {
                    task.retries = task.retries.saturating_add(1);
                    session.state = SessionState::Idle;
                    session.started_at = None;
                    if let Some(permit) = session.permit.take() {
                        drop(permit);
                    }
                    to_retry.push((id.clone(), task));
                }
            }
        }

        let mut migrations = Vec::new();

        for (from_session, task) in to_retry {
            if task.retries > self.config.max_retries {
                {
                    let mut metrics = self.metrics.write().await;
                    metrics.tasks_failed = metrics.tasks_failed.saturating_add(1);
                    metrics.failure_count = metrics.failure_count.saturating_add(1);
                }

                let result = TaskResult {
                    task_id: task.id.clone(),
                    session_id: from_session.clone(),
                    success: false,
                    content: format!(
                        "max retries exceeded after {retry_count} attempts",
                        retry_count = task.retries
                    ),
                    tokens_used: 0,
                    duration: Duration::from_millis(0),
                };
                let _ = self.result_tx.send(result).await;
                continue;
            }

            let task_id = task.id.clone();
            let retry_count = task.retries;
            {
                let mut metrics = self.metrics.write().await;
                metrics.retry_count = metrics.retry_count.saturating_add(1);
            }
            let _ = self.submit(task.clone()).await;
            let to_session = match self.dispatch_from_queue().await {
                Ok(Some(id)) => Some(id),
                Ok(None) => None,
                Err(_) => None,
            };

            migrations.push(MigrationEvent {
                from_session,
                to_session,
                task_id,
                retry_count,
            });
        }

        self.refresh_metrics().await;
        migrations
    }

    /// Gets the next completed result (non-blocking)
    pub async fn try_next_result(&self) -> Option<TaskResult> {
        let mut rx = self.result_rx.lock().await;
        rx.try_recv().ok()
    }

    /// Waits for the next completed result
    pub async fn next_result(&self) -> Option<TaskResult> {
        let mut rx = self.result_rx.lock().await;
        rx.recv().await
    }

    /// Gets current pool metrics
    pub async fn metrics(&self) -> PoolMetrics {
        self.refresh_metrics().await;
        self.metrics.read().await.clone()
    }

    /// Takes the most recent backpressure alert if any.
    pub async fn take_backpressure_alert(&self) -> Option<BudgetAlert> {
        let mut alert = self.backpressure_alert.write().await;
        alert.take()
    }

    /// Checks session health and handles stuck sessions
    pub async fn health_check(&self) -> HealthReport {
        let now = Instant::now();
        let mut found_stuck = false;
        let mut slow_sessions = Vec::new();
        let mut stuck_sessions = Vec::new();

        {
            let mut sessions = self.sessions.write().await;
            for session in sessions.values_mut() {
                if session.state == SessionState::Running
                    && let Some(started) = session.started_at
                {
                    let elapsed = now.duration_since(started);
                    let elapsed_ms = duration_ms(elapsed);

                    if elapsed > self.config.stuck_threshold {
                        session.state = SessionState::Stuck;
                        found_stuck = true;
                        let task_id = session.current_task.as_ref().map(|task| task.id.clone());
                        stuck_sessions.push((session.id.clone(), elapsed_ms, task_id));
                        tracing::warn!(
                            session_id = %session.id,
                            elapsed_secs = elapsed.as_secs(),
                            "Session stuck, marking for reassignment"
                        );
                    } else if elapsed > self.config.slow_threshold {
                        session.state = SessionState::Slow;
                        let task_id = session.current_task.as_ref().map(|task| task.id.clone());
                        slow_sessions.push((session.id.clone(), elapsed_ms, task_id));
                        tracing::debug!(
                            session_id = %session.id,
                            elapsed_secs = elapsed.as_secs(),
                            "Session running slow"
                        );
                    }
                }
            }
        }

        if !stuck_sessions.is_empty() {
            let mut metrics = self.metrics.write().await;
            metrics.stuck_count = metrics
                .stuck_count
                .saturating_add(stuck_sessions.len() as i32);
        }

        let migrations = if found_stuck {
            self.migrate_stuck().await
        } else {
            Vec::new()
        };

        if !migrations.is_empty() {
            let mut metrics = self.metrics.write().await;
            metrics.migration_count = metrics
                .migration_count
                .saturating_add(migrations.len() as i32);
        }

        HealthReport {
            slow_sessions,
            stuck_sessions,
            migrations,
        }
    }

    /// Initiates graceful shutdown
    pub async fn shutdown(&self) {
        let mut shutdown = self.shutdown.write().await;
        *shutdown = true;

        // Mark all sessions as shutting down
        let mut sessions = self.sessions.write().await;
        for session in sessions.values_mut() {
            session.state = SessionState::ShuttingDown;
        }
    }

    /// Returns pool utilization (0.0 - 1.0)
    pub async fn utilization(&self) -> f64 {
        let sessions = self.sessions.read().await;
        if sessions.is_empty() {
            return 0.0;
        }
        let active = sessions
            .values()
            .filter(|s| s.state == SessionState::Running)
            .count();
        active as f64 / sessions.len() as f64
    }

    #[cfg(test)]
    pub async fn inject_session_state(
        &self,
        state: SessionState,
        task: PoolTask,
        started_at: Instant,
    ) {
        let mut sessions = self.sessions.write().await;
        let id = task.id.clone();
        let entry = sessions
            .entry(id.clone())
            .or_insert_with(|| SessionInfo::new(id.clone()));
        entry.state = state;
        entry.current_task = Some(task);
        entry.started_at = Some(started_at);
    }

    #[cfg(test)]
    pub fn slow_threshold(&self) -> Duration {
        self.config.slow_threshold
    }
}

fn duration_ms(duration: Duration) -> i64 {
    let millis = duration.as_millis();
    if millis > i64::MAX as u128 {
        i64::MAX
    } else {
        millis as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use tokio::runtime::Runtime;

    #[tokio::test]
    async fn test_pool_creation() {
        let config = SessionPoolConfig::default();
        let pool = SessionPool::new(config);

        let metrics = pool.metrics().await;
        assert_eq!(metrics.tasks_submitted, 0);
        assert_eq!(metrics.active_sessions, 0);
    }

    #[tokio::test]
    async fn warmup_creates_minimum_sessions() {
        let mut config = SessionPoolConfig::default();
        config.min_sessions = 3;
        config.max_sessions = 5;
        let pool = SessionPool::new(config);
        pool.warmup().await;

        let metrics = pool.metrics().await;
        assert!(metrics.idle_sessions >= 3);
    }

    #[tokio::test]
    async fn dispatch_and_complete_cycle_resets_session() {
        let pool = SessionPool::new(SessionPoolConfig::default());
        pool.warmup().await;

        pool.submit(PoolTask::new("task-1", "Test task"))
            .await
            .unwrap();

        if pool.metrics().await.active_sessions == 0 {
            let _ = pool.dispatch_from_queue().await.unwrap();
        }

        let session_id = {
            let sessions = pool.sessions.read().await;
            sessions
                .iter()
                .find(|(_, session)| session.state == SessionState::Running)
                .map(|(id, _)| id.clone())
                .expect("session running")
        };

        {
            let sessions = pool.sessions.read().await;
            assert_eq!(
                sessions.get(&session_id).unwrap().state,
                SessionState::Running
            );
        }

        let result = TaskResult {
            task_id: "task-1".to_string(),
            session_id: session_id.clone(),
            success: true,
            content: "done".to_string(),
            tokens_used: 10,
            duration: Duration::from_millis(50),
        };
        pool.complete_session(&session_id, result).await.unwrap();

        let sessions = pool.sessions.read().await;
        assert_eq!(sessions.get(&session_id).unwrap().state, SessionState::Idle);

        let metrics = pool.metrics().await;
        assert_eq!(metrics.tasks_completed, 1);
        assert_eq!(metrics.queue_size, 0);
    }

    #[tokio::test]
    async fn auto_scale_adjusts_session_count() {
        let mut config = SessionPoolConfig::default();
        config.max_sessions = 3;
        config.min_sessions = 1;
        config.scale_up_threshold = 0.5;
        config.scale_down_threshold = 0.2;
        let pool = SessionPool::new(config);
        pool.warmup().await;

        pool.submit(PoolTask::new("task", "scale-up"))
            .await
            .unwrap();
        if pool.metrics().await.active_sessions == 0 {
            let _ = pool.dispatch_from_queue().await.unwrap();
        }

        pool.auto_scale().await;
        {
            let sessions = pool.sessions.read().await;
            assert!(sessions.len() >= 2);
        }

        {
            let mut sessions = pool.sessions.write().await;
            for session in sessions.values_mut() {
                session.state = SessionState::Idle;
                session.current_task = None;
            }
        }

        pool.auto_scale().await;
        let sessions = pool.sessions.read().await;
        assert_eq!(sessions.len(), pool.config.min_sessions.max(0) as usize);
    }

    #[tokio::test]
    async fn migrate_stuck_retries_and_requeues() {
        let mut config = SessionPoolConfig::default();
        config.max_sessions = 2;
        config.min_sessions = 1;
        let pool = SessionPool::new(config);
        pool.warmup().await;

        pool.submit(PoolTask::new("task-stuck", "work"))
            .await
            .unwrap();
        if pool.metrics().await.active_sessions == 0 {
            let _ = pool.dispatch_from_queue().await.unwrap();
        }

        let session_id = {
            let sessions = pool.sessions.read().await;
            sessions
                .iter()
                .find(|(_, session)| session.state == SessionState::Running)
                .map(|(id, _)| id.clone())
                .expect("dispatched")
        };

        {
            let mut sessions = pool.sessions.write().await;
            let session = sessions.get_mut(&session_id).unwrap();
            session.state = SessionState::Stuck;
            session.started_at = Some(Instant::now() - pool.config.stuck_threshold);
            if let Some(task) = session.current_task.as_mut() {
                task.retries = 0;
            }
        }

        let migrations = pool.migrate_stuck().await;
        assert_eq!(migrations.len(), 1);
        assert_eq!(migrations[0].task_id, "task-stuck");
        assert_eq!(migrations[0].retry_count, 1);
    }

    #[tokio::test]
    async fn migrate_stuck_respects_max_retries() {
        let mut config = SessionPoolConfig::default();
        config.max_retries = 0;
        let pool = SessionPool::new(config);
        pool.warmup().await;

        pool.submit(PoolTask::new("task-max", "work"))
            .await
            .unwrap();
        if pool.metrics().await.active_sessions == 0 {
            let _ = pool.dispatch_from_queue().await.unwrap();
        }

        let session_id = {
            let sessions = pool.sessions.read().await;
            sessions
                .iter()
                .find(|(_, session)| session.state == SessionState::Running)
                .map(|(id, _)| id.clone())
                .expect("dispatched")
        };

        {
            let mut sessions = pool.sessions.write().await;
            let session = sessions.get_mut(&session_id).unwrap();
            session.state = SessionState::Stuck;
            if let Some(task) = session.current_task.as_mut() {
                task.retries = pool.config.max_retries;
            }
        }

        let events = pool.migrate_stuck().await;
        assert!(events.is_empty());

        let result = pool.try_next_result().await.unwrap();
        assert!(!result.success);
        assert_eq!(result.task_id, "task-max");
        let metrics = pool.metrics().await;
        assert_eq!(metrics.tasks_failed, 1);
    }

    #[tokio::test]
    async fn backpressure_thresholds_enforced() {
        let mut config = SessionPoolConfig::default();
        config.max_sessions = 1;
        config.min_sessions = 1;
        config.backpressure_threshold = 2;
        let pool = SessionPool::new(config);
        pool.warmup().await;

        {
            let mut sessions = pool.sessions.write().await;
            if let Some(session) = sessions.values_mut().next() {
                session.state = SessionState::Running;
                session.current_task = Some(PoolTask::new("busy", "work"));
            }
        }

        pool.submit(PoolTask::new("task-1", "work")).await.unwrap();
        pool.submit(PoolTask::new("task-2", "work")).await.unwrap();

        let warn_metrics = pool.metrics().await;
        assert_eq!(warn_metrics.backpressure_warnings, 1);

        let err = pool.submit(PoolTask::new("task-3", "work")).await;
        assert!(matches!(err, Err(PoolError::BackpressureFull { .. })));
        let metrics = pool.metrics().await;
        assert_eq!(metrics.backpressure_rejections, 1);
        let alert = pool.take_backpressure_alert().await.unwrap();
        match alert {
            BudgetAlert::BackpressureExceeded { queue_size, limit } => {
                assert_eq!(queue_size, 2);
                assert_eq!(limit, 2);
            }
            other => panic!("unexpected alert: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_priority_queue() {
        let mut queue = TaskQueue::new();

        queue.push(PoolTask::new("low", "low").with_priority(1));
        queue.push(PoolTask::new("high", "high").with_priority(3));
        queue.push(PoolTask::new("normal", "normal").with_priority(2));

        // Should get high priority first
        assert_eq!(queue.pop().unwrap().id, "high");
        assert_eq!(queue.pop().unwrap().id, "normal");
        assert_eq!(queue.pop().unwrap().id, "low");
    }

    #[tokio::test]
    async fn test_shutdown() {
        let pool = SessionPool::new(SessionPoolConfig::default());
        pool.shutdown().await;

        let task = PoolTask::new("task", "test");
        assert!(matches!(
            pool.submit(task).await,
            Err(PoolError::ShuttingDown)
        ));
    }

    #[tokio::test]
    async fn health_check_reports_slow_and_stuck() {
        let mut config = SessionPoolConfig::default();
        config.slow_threshold = Duration::from_millis(10);
        config.stuck_threshold = Duration::from_millis(20);
        let pool = SessionPool::new(config);
        pool.warmup().await;

        let task = PoolTask::new("task", "work");
        {
            let mut sessions = pool.sessions.write().await;
            let session = sessions.values_mut().next().unwrap();
            session.state = SessionState::Running;
            session.current_task = Some(task);
            session.started_at = Some(Instant::now() - Duration::from_millis(25));
        }

        let report = pool.health_check().await;
        assert_eq!(report.stuck_sessions.len(), 1);
        assert!(report.slow_sessions.is_empty());
    }

    #[tokio::test]
    async fn health_check_reports_slow_without_stuck() {
        let mut config = SessionPoolConfig::default();
        config.slow_threshold = Duration::from_millis(20);
        config.stuck_threshold = Duration::from_millis(100);
        let pool = SessionPool::new(config);
        pool.warmup().await;

        let task = PoolTask::new("task-slow", "work");
        {
            let mut sessions = pool.sessions.write().await;
            let session = sessions.values_mut().next().unwrap();
            session.state = SessionState::Running;
            session.current_task = Some(task);
            session.started_at = Some(Instant::now() - Duration::from_millis(50));
        }

        let report = pool.health_check().await;
        assert_eq!(report.slow_sessions.len(), 1);
        assert!(report.stuck_sessions.is_empty());
    }

    proptest! {
        #[test]
        fn min_sessions_invariant(min in 1i32..6i32) {
            let mut config = SessionPoolConfig::default();
            config.min_sessions = min;
            config.max_sessions = min + 2;
            let rt = Runtime::new().unwrap();

            let result = rt.block_on(async {
                let pool = SessionPool::new(config);
                pool.warmup().await;
                let metrics = pool.metrics().await;
                prop_assert!(metrics.active_sessions + metrics.idle_sessions >= min);
                Ok::<(), proptest::test_runner::TestCaseError>(())
            });
            result?;
        }
    }
}
