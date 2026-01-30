//! Multi-agent specific tools for mode transitions and work queue management
//!
//! These tools are designed for OpenAI-compatible APIs (Kimi, etc.)

use crate::tools::{PropertySchema, ToolDefinition, ToolGroup, ToolInputSchema};
use serde_json::json;
use std::collections::HashMap;

// =============================================================================
// EXPLORE MODE TOOLS
// =============================================================================

/// Create the `add_finding` tool for recording discoveries
pub fn add_finding_tool() -> ToolDefinition {
    let mut properties = HashMap::new();
    properties.insert(
        "category".to_string(),
        PropertySchema {
            schema_type: "string".to_string(),
            description: "Category of finding".to_string(),
            default: None,
            items: None,
            enum_values: Some(vec![
                "code_pattern".to_string(),
                "file_structure".to_string(),
                "dependency".to_string(),
                "constraint".to_string(),
                "risk".to_string(),
                "other".to_string(),
            ]),
        },
    );
    properties.insert(
        "content".to_string(),
        PropertySchema {
            schema_type: "string".to_string(),
            description: "What you discovered".to_string(),
            default: None,
            items: None,
            enum_values: None,
        },
    );
    properties.insert(
        "relevance".to_string(),
        PropertySchema {
            schema_type: "string".to_string(),
            description: "How relevant to the task".to_string(),
            default: Some(json!("medium")),
            items: None,
            enum_values: Some(vec![
                "high".to_string(),
                "medium".to_string(),
                "low".to_string(),
            ]),
        },
    );
    properties.insert(
        "files".to_string(),
        PropertySchema {
            schema_type: "array".to_string(),
            description: "Related file paths".to_string(),
            default: Some(json!([])),
            items: Some(Box::new(PropertySchema {
                schema_type: "string".to_string(),
                description: "File path".to_string(),
                default: None,
                items: None,
                enum_values: None,
            })),
            enum_values: None,
        },
    );

    ToolDefinition {
        name: "add_finding".to_string(),
        description: "Record an important discovery during exploration. Findings are used to inform the planning phase.".to_string(),
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties,
            required: vec!["category".to_string(), "content".to_string()],
        },
        group: ToolGroup::System,
    }
}

/// Create the `ready_to_plan` tool
pub fn ready_to_plan_tool() -> ToolDefinition {
    let mut properties = HashMap::new();
    properties.insert(
        "summary".to_string(),
        PropertySchema {
            schema_type: "string".to_string(),
            description: "Summary of what was learned and why you're ready to plan".to_string(),
            default: None,
            items: None,
            enum_values: None,
        },
    );

    ToolDefinition {
        name: "ready_to_plan".to_string(),
        description: "Signal that exploration is complete. Call this when you have gathered enough context to create a plan.".to_string(),
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties,
            required: vec!["summary".to_string()],
        },
        group: ToolGroup::System,
    }
}

// =============================================================================
// PLAN MODE TOOLS
// =============================================================================

/// Create the `set_plan_summary` tool
pub fn set_plan_summary_tool() -> ToolDefinition {
    let mut properties = HashMap::new();
    properties.insert(
        "summary".to_string(),
        PropertySchema {
            schema_type: "string".to_string(),
            description: "One-line summary of what the plan accomplishes".to_string(),
            default: None,
            items: None,
            enum_values: None,
        },
    );

    ToolDefinition {
        name: "set_plan_summary".to_string(),
        description: "Set the overall plan summary/goal.".to_string(),
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties,
            required: vec!["summary".to_string()],
        },
        group: ToolGroup::System,
    }
}

/// Create the `ready_to_perform` tool
pub fn ready_to_perform_tool() -> ToolDefinition {
    let mut properties = HashMap::new();
    properties.insert(
        "confirmation".to_string(),
        PropertySchema {
            schema_type: "string".to_string(),
            description: "Confirm the plan is complete and ready for execution".to_string(),
            default: None,
            items: None,
            enum_values: None,
        },
    );

    ToolDefinition {
        name: "ready_to_perform".to_string(),
        description: "Signal that planning is complete and execution can begin. Must have work items queued.".to_string(),
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties,
            required: vec!["confirmation".to_string()],
        },
        group: ToolGroup::System,
    }
}

// =============================================================================
// PERFORM MODE TOOLS
// =============================================================================

/// Create the `finish_execution` tool
pub fn finish_execution_tool() -> ToolDefinition {
    let mut properties = HashMap::new();
    properties.insert(
        "summary".to_string(),
        PropertySchema {
            schema_type: "string".to_string(),
            description: "Final summary of what was accomplished".to_string(),
            default: None,
            items: None,
            enum_values: None,
        },
    );
    properties.insert(
        "follow_up".to_string(),
        PropertySchema {
            schema_type: "string".to_string(),
            description: "Any recommended follow-up actions".to_string(),
            default: None,
            items: None,
            enum_values: None,
        },
    );

    ToolDefinition {
        name: "finish_execution".to_string(),
        description: "Signal that all work is complete and provide a final summary.".to_string(),
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties,
            required: vec!["summary".to_string()],
        },
        group: ToolGroup::System,
    }
}

// =============================================================================
// SHARED TOOLS (available in multiple modes)
// =============================================================================

/// Create the `add_note` tool
pub fn add_note_tool() -> ToolDefinition {
    let mut properties = HashMap::new();
    properties.insert(
        "note".to_string(),
        PropertySchema {
            schema_type: "string".to_string(),
            description: "Note content to remember".to_string(),
            default: None,
            items: None,
            enum_values: None,
        },
    );

    ToolDefinition {
        name: "add_note".to_string(),
        description: "Add a note to the scratchpad. Use for observations or information to remember.".to_string(),
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties,
            required: vec!["note".to_string()],
        },
        group: ToolGroup::System,
    }
}

// =============================================================================
// TASK TOOLS (Persistent Memory)
// =============================================================================

/// Create the `create_task` tool for adding tasks (Plan mode only)
pub fn create_task_tool() -> ToolDefinition {
    let mut properties = HashMap::new();
    properties.insert(
        "subject".to_string(),
        PropertySchema {
            schema_type: "string".to_string(),
            description: "Short description of the task to be done".to_string(),
            default: None,
            items: None,
            enum_values: None,
        },
    );
    properties.insert(
        "note".to_string(),
        PropertySchema {
            schema_type: "string".to_string(),
            description: "Optional initial note for this task".to_string(),
            default: None,
            items: None,
            enum_values: None,
        },
    );

    ToolDefinition {
        name: "create_task".to_string(),
        description: "Add a task to the persistent task list. Use this in Plan mode to build up the list of tasks to execute.".to_string(),
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties,
            required: vec!["subject".to_string()],
        },
        group: ToolGroup::System,
    }
}

/// Create the `add_task_note` tool for adding notes to tasks (any mode)
pub fn add_task_note_tool() -> ToolDefinition {
    let mut properties = HashMap::new();
    properties.insert(
        "id".to_string(),
        PropertySchema {
            schema_type: "string".to_string(),
            description: "Task ID (can use first 8 chars)".to_string(),
            default: None,
            items: None,
            enum_values: None,
        },
    );
    properties.insert(
        "note".to_string(),
        PropertySchema {
            schema_type: "string".to_string(),
            description: "Note content about progress, observations, or issues".to_string(),
            default: None,
            items: None,
            enum_values: None,
        },
    );

    ToolDefinition {
        name: "add_task_note".to_string(),
        description: "Add a note to a task. Use to record observations, progress updates, or issues encountered.".to_string(),
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties,
            required: vec!["id".to_string(), "note".to_string()],
        },
        group: ToolGroup::System,
    }
}

/// Create the `start_task` tool for getting next task (Perform mode only)
pub fn start_task_tool() -> ToolDefinition {
    ToolDefinition {
        name: "start_task".to_string(),
        description: "Start the next pending task (FIFO order). Marks it IN_PROGRESS and returns its details. Use this in Perform mode to begin executing tasks.".to_string(),
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: HashMap::new(),
            required: vec![],
        },
        group: ToolGroup::System,
    }
}

/// Create the `complete_task` tool for marking task done (Perform mode only)
pub fn complete_task_tool() -> ToolDefinition {
    let mut properties = HashMap::new();
    properties.insert(
        "id".to_string(),
        PropertySchema {
            schema_type: "string".to_string(),
            description: "Task ID (can use first 8 chars)".to_string(),
            default: None,
            items: None,
            enum_values: None,
        },
    );
    properties.insert(
        "result".to_string(),
        PropertySchema {
            schema_type: "string".to_string(),
            description: "Summary of what was accomplished".to_string(),
            default: None,
            items: None,
            enum_values: None,
        },
    );

    ToolDefinition {
        name: "complete_task".to_string(),
        description: "Mark a task as completed with a result summary.".to_string(),
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties,
            required: vec!["id".to_string(), "result".to_string()],
        },
        group: ToolGroup::System,
    }
}

/// Create the `fail_task` tool for marking task failed (Perform mode only)
pub fn fail_task_tool() -> ToolDefinition {
    let mut properties = HashMap::new();
    properties.insert(
        "id".to_string(),
        PropertySchema {
            schema_type: "string".to_string(),
            description: "Task ID (can use first 8 chars)".to_string(),
            default: None,
            items: None,
            enum_values: None,
        },
    );
    properties.insert(
        "error".to_string(),
        PropertySchema {
            schema_type: "string".to_string(),
            description: "Description of what went wrong".to_string(),
            default: None,
            items: None,
            enum_values: None,
        },
    );

    ToolDefinition {
        name: "fail_task".to_string(),
        description: "Mark a task as failed with an error description.".to_string(),
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties,
            required: vec!["id".to_string(), "error".to_string()],
        },
        group: ToolGroup::System,
    }
}

/// Create the `get_tasks` tool for viewing the task list (any mode)
pub fn get_tasks_tool() -> ToolDefinition {
    ToolDefinition {
        name: "get_tasks".to_string(),
        description: "Get the full task list with all tasks, their status, and notes.".to_string(),
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: HashMap::new(),
            required: vec![],
        },
        group: ToolGroup::System,
    }
}

// =============================================================================
// TOOL SETS PER MODE
// =============================================================================

/// Get tools for a specific agent mode
pub fn get_tools_for_mode(mode: super::types::AgentMode) -> Vec<ToolDefinition> {
    use super::types::AgentMode;

    match mode {
        AgentMode::Explore => vec![
            add_finding_tool(),
            add_note_tool(),
            ready_to_plan_tool(),
            // Task tools (read-only in explore)
            add_task_note_tool(),
            get_tasks_tool(),
        ],
        AgentMode::Plan => vec![
            set_plan_summary_tool(),
            add_note_tool(),
            ready_to_perform_tool(),
            // Task tools (can create tasks)
            create_task_tool(),
            add_task_note_tool(),
            get_tasks_tool(),
        ],
        AgentMode::Perform => vec![
            add_note_tool(),
            finish_execution_tool(),
            // Task tools (can start/complete/fail tasks)
            start_task_tool(),
            complete_task_tool(),
            fail_task_tool(),
            add_task_note_tool(),
            get_tasks_tool(),
        ],
    }
}

/// Get all multi-agent tools (for reference)
pub fn get_all_tools() -> Vec<ToolDefinition> {
    vec![
        // Explore
        add_finding_tool(),
        ready_to_plan_tool(),
        // Plan
        set_plan_summary_tool(),
        ready_to_perform_tool(),
        // Perform
        finish_execution_tool(),
        // Shared
        add_note_tool(),
        // Task tools
        create_task_tool(),
        add_task_note_tool(),
        start_task_tool(),
        complete_task_tool(),
        fail_task_tool(),
        get_tasks_tool(),
    ]
}
