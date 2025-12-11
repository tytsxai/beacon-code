//! Role communication channel for inter-role messaging.
//!
//! Enables roles to communicate during task execution for better coordination.

use std::collections::HashMap;
use tokio::sync::mpsc;

/// Message types between roles
#[derive(Debug, Clone, PartialEq)]
pub enum RoleMessage {
    /// Coordinator assigns a task to a role
    TaskAssignment {
        target_role: String,
        task_id: String,
        description: String,
    },
    /// Architect provides design for executors
    DesignReady { task_id: String, design: String },
    /// Executor signals implementation is ready
    ImplementationReady {
        executor_id: String,
        task_id: String,
        files_changed: Vec<String>,
        summary: String,
    },
    /// Tester reports test results
    TestResult {
        task_id: String,
        passed: bool,
        failures: Vec<String>,
        coverage: Option<f64>,
    },
    /// Debugger reports fix applied
    FixApplied {
        task_id: String,
        issue: String,
        fix_summary: String,
    },
    /// Role signals work is complete
    WorkComplete {
        role: String,
        task_id: Option<String>,
        success: bool,
        result: String,
    },
    /// Role reports an error
    ErrorOccurred {
        role: String,
        task_id: Option<String>,
        error: String,
    },
    /// Coordinator guidance to a specific role
    Guidance { to: String, content: String },
    /// Request for help/clarification
    Clarification {
        from_role: String,
        to_role: String,
        question: String,
    },
    /// Stage transition signal
    StageAdvance {
        task_id: Option<String>,
        from_stage: Option<String>,
        stage: String,
    },
}

/// Sender handle for a role
pub type RoleSender = mpsc::Sender<RoleMessage>;
/// Receiver handle for a role
pub type RoleReceiver = mpsc::Receiver<RoleMessage>;

/// Channel hub for role communication
pub struct RoleChannelHub {
    channels: HashMap<String, RoleSender>,
    buffer_size: usize,
}

impl RoleChannelHub {
    /// Creates a new channel hub
    pub fn new(buffer_size: usize) -> Self {
        Self {
            channels: HashMap::new(),
            buffer_size,
        }
    }

    /// Registers a role and returns its receiver
    pub fn register(&mut self, role_name: impl Into<String>) -> RoleReceiver {
        let name = role_name.into();
        let (tx, rx) = mpsc::channel(self.buffer_size);
        self.channels.insert(name, tx);
        rx
    }

    /// Sends a message to a specific role
    pub async fn send_to(&self, role: &str, msg: RoleMessage) -> Result<(), &'static str> {
        if let Some(tx) = self.channels.get(role) {
            tx.send(msg).await.map_err(|_| "Channel closed")
        } else {
            Err("Role not found")
        }
    }

    /// Broadcasts a message to all roles
    pub async fn broadcast(&self, msg: RoleMessage) {
        for tx in self.channels.values() {
            let _ = tx.send(msg.clone()).await;
        }
    }

    /// Sends to multiple specific roles
    pub async fn send_to_many(&self, roles: &[&str], msg: RoleMessage) {
        for role in roles {
            if let Some(tx) = self.channels.get(*role) {
                let _ = tx.send(msg.clone()).await;
            }
        }
    }

    /// Checks if a role is registered
    pub fn has_role(&self, role: &str) -> bool {
        self.channels.contains_key(role)
    }

    /// Returns list of registered roles
    pub fn roles(&self) -> Vec<&str> {
        self.channels
            .keys()
            .map(std::string::String::as_str)
            .collect()
    }
}

impl Default for RoleChannelHub {
    fn default() -> Self {
        Self::new(32)
    }
}

/// Helper struct for building coordination messages
pub struct CoordinationBuilder;

impl CoordinationBuilder {
    /// Creates a task assignment message
    pub fn assign_task(target: &str, task_id: &str, desc: &str) -> RoleMessage {
        RoleMessage::TaskAssignment {
            target_role: target.to_string(),
            task_id: task_id.to_string(),
            description: desc.to_string(),
        }
    }

    /// Creates a design ready message
    pub fn design_ready(task_id: &str, design: &str) -> RoleMessage {
        RoleMessage::DesignReady {
            task_id: task_id.to_string(),
            design: design.to_string(),
        }
    }

    /// Creates an implementation ready message
    pub fn impl_ready(
        executor_id: &str,
        task_id: &str,
        files: Vec<String>,
        summary: &str,
    ) -> RoleMessage {
        RoleMessage::ImplementationReady {
            executor_id: executor_id.to_string(),
            task_id: task_id.to_string(),
            files_changed: files,
            summary: summary.to_string(),
        }
    }

    /// Creates a test result message
    pub fn test_result(task_id: &str, passed: bool, failures: Vec<String>) -> RoleMessage {
        RoleMessage::TestResult {
            task_id: task_id.to_string(),
            passed,
            failures,
            coverage: None,
        }
    }

    pub fn error(role: &str, task_id: Option<&str>, error: &str) -> RoleMessage {
        RoleMessage::ErrorOccurred {
            role: role.to_string(),
            task_id: task_id.map(std::string::ToString::to_string),
            error: error.to_string(),
        }
    }

    pub fn guidance(to: &str, content: &str) -> RoleMessage {
        RoleMessage::Guidance {
            to: to.to_string(),
            content: content.to_string(),
        }
    }

    /// Creates a work complete message
    pub fn work_done(role: &str, task_id: &str, success: bool, output: &str) -> RoleMessage {
        RoleMessage::WorkComplete {
            role: role.to_string(),
            task_id: Some(task_id.to_string()),
            success,
            result: output.to_string(),
        }
    }

    /// Creates a stage advance message
    pub fn advance_stage(task_id: &str, from: &str, to: &str) -> RoleMessage {
        RoleMessage::StageAdvance {
            task_id: Some(task_id.to_string()),
            from_stage: Some(from.to_string()),
            stage: to.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use tokio::runtime::Runtime;

    #[tokio::test]
    async fn test_channel_hub() {
        let mut hub = RoleChannelHub::new(16);

        let _rx1 = hub.register("Coordinator");
        let _rx2 = hub.register("Executor-1");

        assert!(hub.has_role("Coordinator"));
        assert!(hub.has_role("Executor-1"));
        assert!(!hub.has_role("Unknown"));
    }

    #[tokio::test]
    async fn test_send_message() {
        let mut hub = RoleChannelHub::new(16);
        let mut rx = hub.register("Executor-1");

        let msg = CoordinationBuilder::assign_task("Executor-1", "task-1", "Do something");
        hub.send_to("Executor-1", msg).await.unwrap();

        let received = rx.recv().await.unwrap();
        match received {
            RoleMessage::TaskAssignment { task_id, .. } => {
                assert_eq!(task_id, "task-1");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[tokio::test]
    async fn test_broadcast() {
        let mut hub = RoleChannelHub::new(16);
        let mut rx1 = hub.register("Role1");
        let mut rx2 = hub.register("Role2");

        let msg = CoordinationBuilder::advance_stage("t1", "Planning", "Implementing");
        hub.broadcast(msg).await;

        // Both should receive
        assert!(rx1.recv().await.is_some());
        assert!(rx2.recv().await.is_some());
    }

    #[tokio::test]
    async fn work_complete_delivery() {
        let mut hub = RoleChannelHub::new(8);
        let mut rx = hub.register("Reviewer");
        let msg = RoleMessage::WorkComplete {
            role: "Reviewer".to_string(),
            task_id: Some("t1".to_string()),
            success: true,
            result: "done".to_string(),
        };

        hub.send_to("Reviewer", msg.clone()).await.unwrap();
        let received = rx.recv().await.unwrap();
        assert_eq!(received, msg);
    }

    proptest! {
        #[test]
        fn property_role_message_delivery(role in ".{1,10}", result in ".{1,20}") {
            let rt = Runtime::new().unwrap();
            let outcome = rt.block_on(async {
                let mut hub = RoleChannelHub::new(4);
                let mut rx = hub.register(role.as_str());
                let msg = RoleMessage::WorkComplete {
                    role: role.clone(),
                    task_id: None,
                    success: true,
                    result: result.clone(),
                };
                hub.send_to(role.as_str(), msg.clone()).await.unwrap();
                let received = rx.recv().await.unwrap();
                prop_assert_eq!(received, msg);
                Ok::<(), proptest::test_runner::TestCaseError>(())
            });
            outcome?;
        }
    }
}
