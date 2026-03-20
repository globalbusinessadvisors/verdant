use verdant_core::types::ConfirmedEvent;

use crate::mission::MissionResult;

/// Swarm coordination errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwarmError {
    /// No available robot for the task.
    NoAvailableRobot,
    /// Task not found in active task list.
    TaskNotFound,
}

/// A unique task identifier within the swarm.
pub type TaskId = u64;

/// Assignment of a task to a specific robot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskAssignment {
    pub task_id: TaskId,
    pub robot_id: u64,
}

/// How a task conflict was resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolution {
    /// Keep task_a, cancel task_b.
    KeepFirst,
    /// Keep task_b, cancel task_a.
    KeepSecond,
    /// Merge into a single task.
    Merge,
}

/// Result reported when a task is completed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskResult {
    pub task_id: TaskId,
    pub outcome: MissionResult,
}

/// Coordinates task assignment and conflict resolution across a robot swarm.
///
/// Concrete implementations maintain a registry of available robots and
/// pending tasks. Mocked for testing.
pub trait SwarmCoordinator {
    /// Request a robot to handle a confirmed event.
    fn request_task(&mut self, event: &ConfirmedEvent) -> Result<TaskAssignment, SwarmError>;

    /// Resolve a conflict between two overlapping tasks.
    fn resolve_conflict(&mut self, task_a: TaskId, task_b: TaskId) -> Resolution;

    /// Report the completion of a task.
    fn report_completion(&mut self, result: &TaskResult) -> Result<(), SwarmError>;
}
