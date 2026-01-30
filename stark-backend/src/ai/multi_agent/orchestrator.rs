//! Multi-agent orchestrator - manages transitions between agent modes

use super::tools;
use super::types::{
    AgentContext, AgentMode, Finding, ModeTransition, Relevance, TaskStatus,
};
use crate::tools::ToolDefinition;
use serde_json::Value;

/// Maximum iterations per mode before forcing transition
const MAX_EXPLORE_ITERATIONS: u32 = 10;
const MAX_PLAN_ITERATIONS: u32 = 8;
const MAX_PERFORM_ITERATIONS: u32 = 50;

/// The multi-agent orchestrator manages the flow between agent modes
pub struct Orchestrator {
    context: AgentContext,
}

impl Orchestrator {
    /// Create a new orchestrator for a request
    pub fn new(original_request: String) -> Self {
        Self {
            context: AgentContext {
                original_request,
                mode: AgentMode::Explore, // Always start in Explore mode
                ..Default::default()
            },
        }
    }

    /// Create from existing context (for resuming)
    pub fn from_context(context: AgentContext) -> Self {
        Self { context }
    }

    /// Get the current mode
    pub fn current_mode(&self) -> AgentMode {
        self.context.mode
    }

    /// Get the full context
    pub fn context(&self) -> &AgentContext {
        &self.context
    }

    /// Get mutable context
    pub fn context_mut(&mut self) -> &mut AgentContext {
        &mut self.context
    }

    /// Get the system prompt for the current mode
    pub fn get_system_prompt(&self) -> String {
        let base_prompt = match self.context.mode {
            AgentMode::Explore => include_str!("prompts/explore.md"),
            AgentMode::Plan => include_str!("prompts/plan.md"),
            AgentMode::Perform => include_str!("prompts/perform.md"),
        };

        // Add context summary to the prompt
        let mut prompt = base_prompt.to_string();
        prompt.push_str("\n\n---\n\n");
        prompt.push_str(&self.format_context_summary());

        prompt
    }

    /// Format a summary of the current context for the prompt
    fn format_context_summary(&self) -> String {
        let mut summary = String::new();

        summary.push_str("## Current Context\n\n");
        summary.push_str(&format!("**Original Request**: {}\n\n", self.context.original_request));
        summary.push_str(&format!("**Current Mode**: {} (iteration {})\n\n", self.context.mode, self.context.mode_iterations));

        // Add exploration notes
        if !self.context.exploration_notes.is_empty() {
            summary.push_str("### Notes\n\n");
            for note in &self.context.exploration_notes {
                summary.push_str(&format!("- {}\n", note));
            }
            summary.push('\n');
        }

        // Add findings
        if !self.context.findings.is_empty() {
            summary.push_str("### Findings\n\n");
            for finding in &self.context.findings {
                let files = if finding.files.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", finding.files.join(", "))
                };
                summary.push_str(&format!(
                    "- **[{:?}]** {}: {}{}\n",
                    finding.relevance, finding.category, finding.content, files
                ));
            }
            summary.push('\n');
        }

        // Add plan summary
        if let Some(ref plan_summary) = self.context.plan_summary {
            summary.push_str(&format!("### Plan Goal\n\n{}\n\n", plan_summary));
        }

        // Add scratchpad if not empty
        if !self.context.scratchpad.is_empty() {
            summary.push_str("### Scratchpad\n\n");
            summary.push_str(&self.context.scratchpad);
            summary.push_str("\n\n");
        }

        // Add task list (persistent memory) - always shown
        summary.push_str("### Task List (Persistent Memory)\n\n");
        summary.push_str(&self.context.tasks.format_display());
        summary.push_str("\n\n");

        summary
    }

    /// Get the tools available for the current mode
    pub fn get_mode_tools(&self) -> Vec<ToolDefinition> {
        tools::get_tools_for_mode(self.context.mode)
    }

    /// Process a tool call result and potentially transition modes
    pub fn process_tool_result(&mut self, tool_name: &str, params: &Value) -> ProcessResult {
        self.context.mode_iterations += 1;
        self.context.total_iterations += 1;

        log::debug!(
            "[ORCHESTRATOR] Processing tool '{}' in mode {} (iteration {})",
            tool_name, self.context.mode, self.context.mode_iterations
        );

        match tool_name {
            // Explore tools
            "add_finding" => self.handle_add_finding(params),
            "ready_to_plan" => self.handle_ready_to_plan(params),

            // Plan tools
            "set_plan_summary" => self.handle_set_plan_summary(params),
            "ready_to_perform" => self.handle_ready_to_perform(params),

            // Perform tools
            "finish_execution" => self.handle_finish_execution(params),

            // Shared tools
            "add_note" => self.handle_add_note(params),

            // Task tools (persistent memory)
            "create_task" => self.handle_create_task(params),
            "add_task_note" => self.handle_add_task_note(params),
            "start_task" => self.handle_start_task(),
            "complete_task" => self.handle_complete_task(params),
            "fail_task" => self.handle_fail_task(params),
            "get_tasks" => self.handle_get_tasks(),

            _ => ProcessResult::Continue,
        }
    }

    /// Check if we should force a transition due to hitting max iterations
    pub fn check_forced_transition(&mut self) -> Option<ModeTransition> {
        let (max_iterations, next_mode) = match self.context.mode {
            AgentMode::Explore => (MAX_EXPLORE_ITERATIONS, AgentMode::Plan),
            AgentMode::Plan => (MAX_PLAN_ITERATIONS, AgentMode::Perform),
            AgentMode::Perform => (MAX_PERFORM_ITERATIONS, AgentMode::Perform),
        };

        if self.context.mode_iterations >= max_iterations {
            let transition = ModeTransition {
                from: self.context.mode,
                to: next_mode,
                reason: format!(
                    "Forced transition after {} iterations in {} mode",
                    max_iterations, self.context.mode
                ),
            };
            self.transition_to(next_mode);
            Some(transition)
        } else {
            None
        }
    }

    /// Transition to a new mode
    fn transition_to(&mut self, new_mode: AgentMode) {
        log::info!(
            "[ORCHESTRATOR] Transitioning {} → {}",
            self.context.mode,
            new_mode
        );
        self.context.mode = new_mode;
        self.context.mode_iterations = 0;
    }

    // =========================================================================
    // Tool handlers
    // =========================================================================

    fn handle_add_finding(&mut self, params: &Value) -> ProcessResult {
        let category = params.get("category").and_then(|v| v.as_str()).unwrap_or("other");
        let content = params.get("content").and_then(|v| v.as_str()).unwrap_or("");
        let relevance_str = params.get("relevance").and_then(|v| v.as_str()).unwrap_or("medium");
        let files: Vec<String> = params
            .get("files")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        let relevance = match relevance_str {
            "high" => Relevance::High,
            "low" => Relevance::Low,
            _ => Relevance::Medium,
        };

        self.context.findings.push(Finding {
            category: category.to_string(),
            content: content.to_string(),
            relevance,
            files,
        });

        ProcessResult::ToolResult(format!("Finding recorded: {}", content))
    }

    fn handle_add_note(&mut self, params: &Value) -> ProcessResult {
        if let Some(note) = params.get("note").and_then(|v| v.as_str()) {
            self.context.exploration_notes.push(note.to_string());
            ProcessResult::ToolResult(format!("Note added: {}", note))
        } else {
            ProcessResult::Error("Missing 'note' parameter".to_string())
        }
    }

    fn handle_ready_to_plan(&mut self, params: &Value) -> ProcessResult {
        let summary = params.get("summary").and_then(|v| v.as_str()).unwrap_or("");
        self.context.exploration_notes.push(format!("Exploration complete: {}", summary));
        self.context.context_sufficient = true;

        let transition = ModeTransition {
            from: AgentMode::Explore,
            to: AgentMode::Plan,
            reason: summary.to_string(),
        };
        self.transition_to(AgentMode::Plan);
        ProcessResult::Transition(transition)
    }

    fn handle_set_plan_summary(&mut self, params: &Value) -> ProcessResult {
        if let Some(summary) = params.get("summary").and_then(|v| v.as_str()) {
            self.context.plan_summary = Some(summary.to_string());
            ProcessResult::ToolResult(format!("Plan summary set: {}", summary))
        } else {
            ProcessResult::Error("Missing 'summary' parameter".to_string())
        }
    }

    fn handle_ready_to_perform(&mut self, _params: &Value) -> ProcessResult {
        // Check task list - must have at least one task
        if self.context.tasks.is_empty() {
            return ProcessResult::Error("Cannot perform without any tasks. Use create_task first.".to_string());
        }

        self.context.plan_ready = true;
        let stats = self.context.tasks.stats();
        let transition = ModeTransition {
            from: AgentMode::Plan,
            to: AgentMode::Perform,
            reason: format!("Plan ready with {} tasks", stats.total),
        };
        self.transition_to(AgentMode::Perform);
        ProcessResult::Transition(transition)
    }

    fn handle_finish_execution(&mut self, params: &Value) -> ProcessResult {
        let summary = params.get("summary").and_then(|v| v.as_str()).unwrap_or("");
        let follow_up = params.get("follow_up").and_then(|v| v.as_str());

        let stats = self.context.tasks.stats();
        log::info!(
            "[ORCHESTRATOR] Execution finished: {} - {} total ({} completed, {} failed)",
            summary,
            stats.total,
            stats.completed,
            stats.failed
        );

        if let Some(fu) = follow_up {
            log::info!("[ORCHESTRATOR] Follow-up: {}", fu);
        }

        ProcessResult::Complete(summary.to_string())
    }

    // =========================================================================
    // Task handlers (persistent memory)
    // =========================================================================

    /// Create a new task (Plan mode only)
    fn handle_create_task(&mut self, params: &Value) -> ProcessResult {
        // Only allow in Plan mode
        if self.context.mode != AgentMode::Plan {
            return ProcessResult::Error(
                "create_task is only available in Plan mode. Use add_task_note to add notes in other modes.".to_string()
            );
        }

        let subject = match params.get("subject").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s,
            _ => return ProcessResult::Error("Missing or empty 'subject' parameter".to_string()),
        };

        let id = self.context.tasks.add(subject);
        let short_id = &id[..8];

        // Add initial note if provided
        if let Some(note) = params.get("note").and_then(|v| v.as_str()) {
            if let Some(task) = self.context.tasks.get_mut(&id) {
                task.add_note(note);
            }
        }

        ProcessResult::ToolResult(format!(
            "Task created [{}]: {}",
            short_id, subject
        ))
    }

    /// Add a note to a task (any mode)
    fn handle_add_task_note(&mut self, params: &Value) -> ProcessResult {
        let id = match params.get("id").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s,
            _ => return ProcessResult::Error("Missing or empty 'id' parameter".to_string()),
        };

        let note = match params.get("note").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s,
            _ => return ProcessResult::Error("Missing or empty 'note' parameter".to_string()),
        };

        let task = match self.context.tasks.get_mut(id) {
            Some(t) => t,
            None => return ProcessResult::Error(format!("Task '{}' not found", id)),
        };

        let short_id = task.short_id().to_string();
        let subject = task.subject.clone();
        task.add_note(note);

        ProcessResult::ToolResult(format!(
            "Note added to task [{}] '{}': {}",
            short_id, subject, note
        ))
    }

    /// Start the next pending task (FIFO) for execution (Perform mode only)
    fn handle_start_task(&mut self) -> ProcessResult {
        // Only allow in Perform mode
        if self.context.mode != AgentMode::Perform {
            return ProcessResult::Error(
                "start_task is only available in Perform mode".to_string()
            );
        }

        match self.context.tasks.next_pending() {
            Some(task) => {
                task.set_status(TaskStatus::InProgress, Some("Started execution"));
                let task_details = task.format_display();
                ProcessResult::ToolResult(format!(
                    "Started task:\n{}\n\nNow execute this task.",
                    task_details
                ))
            }
            None => ProcessResult::ToolResult(
                "No pending tasks. All tasks have been started or completed!".to_string()
            ),
        }
    }

    /// Complete a task with a result (Perform mode only)
    fn handle_complete_task(&mut self, params: &Value) -> ProcessResult {
        // Only allow in Perform mode
        if self.context.mode != AgentMode::Perform {
            return ProcessResult::Error(
                "complete_task is only available in Perform mode".to_string()
            );
        }

        let id = match params.get("id").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s,
            _ => return ProcessResult::Error("Missing or empty 'id' parameter".to_string()),
        };

        let result = match params.get("result").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s,
            _ => return ProcessResult::Error("Missing or empty 'result' parameter".to_string()),
        };

        let task = match self.context.tasks.get_mut(id) {
            Some(t) => t,
            None => return ProcessResult::Error(format!("Task '{}' not found", id)),
        };

        let short_id = task.short_id().to_string();
        let subject = task.subject.clone();
        task.complete(result);

        ProcessResult::ToolResult(format!(
            "✅ Task completed [{}] '{}': {}",
            short_id, subject, result
        ))
    }

    /// Fail a task with an error (Perform mode only)
    fn handle_fail_task(&mut self, params: &Value) -> ProcessResult {
        // Only allow in Perform mode
        if self.context.mode != AgentMode::Perform {
            return ProcessResult::Error(
                "fail_task is only available in Perform mode".to_string()
            );
        }

        let id = match params.get("id").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s,
            _ => return ProcessResult::Error("Missing or empty 'id' parameter".to_string()),
        };

        let error = match params.get("error").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s,
            _ => return ProcessResult::Error("Missing or empty 'error' parameter".to_string()),
        };

        let task = match self.context.tasks.get_mut(id) {
            Some(t) => t,
            None => return ProcessResult::Error(format!("Task '{}' not found", id)),
        };

        let short_id = task.short_id().to_string();
        let subject = task.subject.clone();
        task.fail(error);

        ProcessResult::ToolResult(format!(
            "❌ Task failed [{}] '{}': {}",
            short_id, subject, error
        ))
    }

    /// Get the full task list display
    fn handle_get_tasks(&self) -> ProcessResult {
        ProcessResult::ToolResult(self.context.tasks.format_display())
    }
}

/// Result of processing a tool call
#[derive(Debug)]
pub enum ProcessResult {
    /// Continue in current mode (tool executed but no transition)
    Continue,
    /// Tool executed successfully with result
    ToolResult(String),
    /// Transition to a new mode
    Transition(ModeTransition),
    /// Task is complete
    Complete(String),
    /// Error occurred
    Error(String),
}
