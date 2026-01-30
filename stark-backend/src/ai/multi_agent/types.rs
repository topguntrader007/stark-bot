//! Multi-agent system types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The current mode/phase of the multi-agent system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentMode {
    /// Exploration phase - gather information, read files, understand context
    Explore,
    /// Planning phase - create a detailed plan from gathered context
    Plan,
    /// Execution phase - perform the planned actions using tools
    Perform,
}

impl Default for AgentMode {
    fn default() -> Self {
        AgentMode::Explore
    }
}

impl std::fmt::Display for AgentMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentMode::Explore => write!(f, "explore"),
            AgentMode::Plan => write!(f, "plan"),
            AgentMode::Perform => write!(f, "perform"),
        }
    }
}

impl AgentMode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "explore" | "exploration" => Some(AgentMode::Explore),
            "plan" | "planning" => Some(AgentMode::Plan),
            "perform" | "execute" | "execution" => Some(AgentMode::Perform),
            _ => None,
        }
    }

    /// Check if skills are available in this mode
    pub fn allows_skills(&self) -> bool {
        matches!(self, AgentMode::Explore | AgentMode::Plan)
    }

    /// Check if action tools (swap, transfer, etc.) are available in this mode
    pub fn allows_action_tools(&self) -> bool {
        matches!(self, AgentMode::Perform)
    }

    /// Human-readable label for UI display
    pub fn label(&self) -> &'static str {
        match self {
            AgentMode::Explore => "Exploring",
            AgentMode::Plan => "Planning",
            AgentMode::Perform => "Executing",
        }
    }
}

/// Task status in the task list
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl Default for TaskStatus {
    fn default() -> Self {
        TaskStatus::Pending
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::InProgress => write!(f, "in_progress"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Failed => write!(f, "failed"),
        }
    }
}

impl TaskStatus {
    pub fn emoji(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "⏳",
            TaskStatus::InProgress => "🔄",
            TaskStatus::Completed => "✅",
            TaskStatus::Failed => "❌",
        }
    }
}

/// A note attached to a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNote {
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

impl TaskNote {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            timestamp: Utc::now(),
        }
    }
}

/// A task in the agent's plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier (UUID)
    pub id: String,
    /// Short description of the task
    pub subject: String,
    /// Current status
    pub status: TaskStatus,
    /// Notes/updates about this task (grows over time)
    pub notes: Vec<TaskNote>,
    /// Result output after completion
    pub result: Option<String>,
    /// Error message if failed
    pub error: Option<String>,
    /// When this task was created
    pub created_at: DateTime<Utc>,
    /// When status was last updated
    pub updated_at: DateTime<Utc>,
}

impl Task {
    pub fn new(subject: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            subject: subject.into(),
            status: TaskStatus::Pending,
            notes: Vec::new(),
            result: None,
            error: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a note to this task
    pub fn add_note(&mut self, content: impl Into<String>) {
        self.notes.push(TaskNote::new(content));
        self.updated_at = Utc::now();
    }

    /// Update status and optionally add a note
    pub fn set_status(&mut self, status: TaskStatus, note: Option<&str>) {
        self.status = status;
        self.updated_at = Utc::now();
        if let Some(n) = note {
            self.notes.push(TaskNote::new(n));
        }
    }

    /// Mark as completed with result
    pub fn complete(&mut self, result: impl Into<String>) {
        self.status = TaskStatus::Completed;
        self.result = Some(result.into());
        self.updated_at = Utc::now();
    }

    /// Mark as failed with error
    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = TaskStatus::Failed;
        self.error = Some(error.into());
        self.updated_at = Utc::now();
    }

    /// Short ID for display (first 8 chars)
    pub fn short_id(&self) -> &str {
        &self.id[..8]
    }

    /// Format for display to the agent
    pub fn format_display(&self) -> String {
        let mut out = format!(
            "{} [{}] {}: {}",
            self.status.emoji(),
            self.short_id(),
            self.status,
            self.subject
        );
        if let Some(ref result) = self.result {
            out.push_str(&format!("\n   → Result: {}", result));
        }
        if let Some(ref error) = self.error {
            out.push_str(&format!("\n   ✗ Error: {}", error));
        }
        if !self.notes.is_empty() {
            out.push_str("\n   Notes:");
            for note in &self.notes {
                out.push_str(&format!("\n   - {}", note.content));
            }
        }
        out
    }
}

/// The task list for agent planning and execution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskList {
    tasks: Vec<Task>,
}

impl TaskList {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    /// Create from a vector of tasks (for deserialization)
    pub fn from_vec(tasks: Vec<Task>) -> Self {
        Self { tasks }
    }

    /// Add a new task (returns the ID)
    pub fn add(&mut self, subject: impl Into<String>) -> String {
        let task = Task::new(subject);
        let id = task.id.clone();
        self.tasks.push(task);
        id
    }

    /// Get a task by ID (supports partial ID match)
    pub fn get(&self, id: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id || t.id.starts_with(id))
    }

    /// Get mutable task by ID (supports partial ID match)
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id || t.id.starts_with(id))
    }

    /// Get the next pending task (FIFO)
    pub fn next_pending(&mut self) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.status == TaskStatus::Pending)
    }

    /// Get all tasks
    pub fn all(&self) -> &[Task] {
        &self.tasks
    }

    /// Check if there's work remaining
    pub fn has_pending(&self) -> bool {
        self.tasks.iter().any(|t| {
            t.status == TaskStatus::Pending || t.status == TaskStatus::InProgress
        })
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Get stats
    pub fn stats(&self) -> TaskStats {
        let mut stats = TaskStats::default();
        for task in &self.tasks {
            match task.status {
                TaskStatus::Pending => stats.pending += 1,
                TaskStatus::InProgress => stats.in_progress += 1,
                TaskStatus::Completed => stats.completed += 1,
                TaskStatus::Failed => stats.failed += 1,
            }
        }
        stats.total = self.tasks.len();
        stats
    }

    /// Format the entire list for agent display
    pub fn format_display(&self) -> String {
        if self.tasks.is_empty() {
            return "📋 Task List: (empty)".to_string();
        }

        let stats = self.stats();
        let mut out = format!(
            "📋 Task List: {} total ({} pending, {} in progress, {} completed, {} failed)\n",
            stats.total, stats.pending, stats.in_progress, stats.completed, stats.failed
        );

        for task in &self.tasks {
            out.push_str(&format!("\n{}", task.format_display()));
        }

        out
    }
}

/// Statistics for the task list
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskStats {
    pub total: usize,
    pub pending: usize,
    pub in_progress: usize,
    pub completed: usize,
    pub failed: usize,
}

impl std::fmt::Display for TaskStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}/{} complete ({} pending, {} in progress, {} failed)",
            self.completed, self.total, self.pending, self.in_progress, self.failed
        )
    }
}

/// Context accumulated during the multi-agent flow
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentContext {
    /// Original user request
    pub original_request: String,

    /// Notes gathered during exploration
    pub exploration_notes: Vec<String>,

    /// Key findings from exploration
    pub findings: Vec<Finding>,

    /// Task list for planning and execution
    #[serde(default)]
    pub tasks: TaskList,

    /// Plan summary/goal
    pub plan_summary: Option<String>,

    /// Current mode
    pub mode: AgentMode,

    /// Number of iterations in current mode
    pub mode_iterations: u32,

    /// Total iterations across all modes
    pub total_iterations: u32,

    /// Whether the agent believes it has enough context
    pub context_sufficient: bool,

    /// Whether the plan is ready for execution
    pub plan_ready: bool,

    /// Scratchpad for agent notes during execution
    pub scratchpad: String,
}

/// A finding discovered during exploration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub category: String,
    pub content: String,
    pub relevance: Relevance,
    /// Related file paths
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Relevance {
    High,
    Medium,
    Low,
}

/// Transition decision from the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeTransition {
    pub from: AgentMode,
    pub to: AgentMode,
    pub reason: String,
}
