//! Task topology module for dependency-based parallel execution.
//!
//! Implements topological sorting to organize tasks into layers where
//! each layer contains independent tasks that can execute in parallel.

use std::collections::{HashMap, HashSet, VecDeque};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TopologyError {
    #[error("circular dependency detected: {0}")]
    CircularDependency(String),
    #[error("unknown dependency: {0}")]
    UnknownDependency(String),
}

/// A task with dependencies for topological sorting.
#[derive(Debug, Clone)]
pub struct TopologicalTask {
    pub id: String,
    pub dependencies: Vec<String>,
    pub payload: String,
}

impl TopologicalTask {
    pub fn new(id: impl Into<String>, payload: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            dependencies: vec![],
            payload: payload.into(),
        }
    }

    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }
}

/// Result of topological sorting - tasks organized into parallel layers.
#[derive(Debug, Clone)]
pub struct TaskLayers {
    /// Each layer contains tasks that can run in parallel.
    pub layers: Vec<Vec<TopologicalTask>>,
    /// Total number of tasks.
    pub total_tasks: usize,
}

impl TaskLayers {
    /// Returns the maximum parallelism (largest layer size).
    pub fn max_parallelism(&self) -> usize {
        self.layers.iter().map(|l| l.len()).max().unwrap_or(0)
    }

    /// Returns the number of layers (sequential steps).
    pub fn depth(&self) -> usize {
        self.layers.len()
    }

    /// Returns true if all tasks are independent (single layer).
    pub fn is_fully_parallel(&self) -> bool {
        self.layers.len() <= 1
    }
}

/// Performs topological sort on tasks, returning layers for parallel execution.
///
/// Tasks in the same layer have no dependencies on each other and can run concurrently.
/// Layer N+1 only executes after all tasks in Layer N complete.
pub fn topological_sort(tasks: Vec<TopologicalTask>) -> Result<TaskLayers, TopologyError> {
    if tasks.is_empty() {
        return Ok(TaskLayers {
            layers: vec![],
            total_tasks: 0,
        });
    }

    let task_ids: HashSet<_> = tasks.iter().map(|t| t.id.as_str()).collect();
    let mut task_map: HashMap<&str, &TopologicalTask> = HashMap::new();
    let mut indegree: HashMap<&str, usize> = HashMap::new();
    let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();

    // Initialize
    for task in &tasks {
        task_map.insert(&task.id, task);
        indegree.insert(&task.id, 0);
        adjacency.insert(&task.id, vec![]);
    }

    // Build graph
    for task in &tasks {
        for dep in &task.dependencies {
            if !task_ids.contains(dep.as_str()) {
                return Err(TopologyError::UnknownDependency(dep.clone()));
            }
            adjacency.get_mut(dep.as_str()).unwrap().push(&task.id);
            *indegree.get_mut(task.id.as_str()).unwrap() += 1;
        }
    }

    // Kahn's algorithm with layer tracking
    let mut layers: Vec<Vec<TopologicalTask>> = vec![];
    let mut queue: VecDeque<&str> = indegree
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(&id, _)| id)
        .collect();

    let mut processed = 0;

    while !queue.is_empty() {
        let mut current_layer = vec![];
        let layer_size = queue.len();

        for _ in 0..layer_size {
            let task_id = queue.pop_front().unwrap();
            current_layer.push(task_map[task_id].clone());
            processed += 1;

            for &dependent in &adjacency[task_id] {
                let deg = indegree.get_mut(dependent).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.push_back(dependent);
                }
            }
        }

        if !current_layer.is_empty() {
            layers.push(current_layer);
        }
    }

    if processed != tasks.len() {
        return Err(TopologyError::CircularDependency(
            "cycle detected in task dependencies".to_string(),
        ));
    }

    Ok(TaskLayers {
        total_tasks: tasks.len(),
        layers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_tasks() {
        let result = topological_sort(vec![]).unwrap();
        assert!(result.layers.is_empty());
        assert_eq!(result.total_tasks, 0);
    }

    #[test]
    fn test_independent_tasks() {
        let tasks = vec![
            TopologicalTask::new("a", "task a"),
            TopologicalTask::new("b", "task b"),
            TopologicalTask::new("c", "task c"),
        ];
        let result = topological_sort(tasks).unwrap();
        assert_eq!(result.layers.len(), 1);
        assert_eq!(result.layers[0].len(), 3);
        assert_eq!(result.max_parallelism(), 3);
        assert!(result.is_fully_parallel());
    }

    #[test]
    fn test_sequential_tasks() {
        let tasks = vec![
            TopologicalTask::new("a", ""),
            TopologicalTask::new("b", "").with_dependencies(vec!["a".into()]),
            TopologicalTask::new("c", "").with_dependencies(vec!["b".into()]),
        ];
        let result = topological_sort(tasks).unwrap();
        assert_eq!(result.layers.len(), 3);
        assert_eq!(result.max_parallelism(), 1);
        assert!(!result.is_fully_parallel());
    }

    #[test]
    fn test_diamond_dependency() {
        let tasks = vec![
            TopologicalTask::new("a", ""),
            TopologicalTask::new("b", "").with_dependencies(vec!["a".into()]),
            TopologicalTask::new("c", "").with_dependencies(vec!["a".into()]),
            TopologicalTask::new("d", "").with_dependencies(vec!["b".into(), "c".into()]),
        ];
        let result = topological_sort(tasks).unwrap();
        assert_eq!(result.layers.len(), 3);
        assert_eq!(result.layers[0].len(), 1); // a
        assert_eq!(result.layers[1].len(), 2); // b, c parallel
        assert_eq!(result.layers[2].len(), 1); // d
    }

    #[test]
    fn test_circular_dependency() {
        let tasks = vec![
            TopologicalTask::new("a", "").with_dependencies(vec!["b".into()]),
            TopologicalTask::new("b", "").with_dependencies(vec!["a".into()]),
        ];
        assert!(matches!(
            topological_sort(tasks),
            Err(TopologyError::CircularDependency(_))
        ));
    }

    #[test]
    fn test_unknown_dependency() {
        let tasks = vec![TopologicalTask::new("a", "").with_dependencies(vec!["unknown".into()])];
        assert!(matches!(
            topological_sort(tasks),
            Err(TopologyError::UnknownDependency(_))
        ));
    }

    #[test]
    fn test_complex_dag() {
        // Layer 0: a, b (no deps)
        // Layer 1: c (depends on a), d (depends on b)
        // Layer 2: e (depends on c, d)
        let tasks = vec![
            TopologicalTask::new("a", ""),
            TopologicalTask::new("b", ""),
            TopologicalTask::new("c", "").with_dependencies(vec!["a".into()]),
            TopologicalTask::new("d", "").with_dependencies(vec!["b".into()]),
            TopologicalTask::new("e", "").with_dependencies(vec!["c".into(), "d".into()]),
        ];
        let result = topological_sort(tasks).unwrap();
        assert_eq!(result.depth(), 3);
        assert_eq!(result.max_parallelism(), 2);
    }
}
