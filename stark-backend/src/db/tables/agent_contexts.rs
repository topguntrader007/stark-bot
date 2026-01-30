//! Agent contexts table - multi-agent orchestrator state persistence
//!
//! Stores AgentContext between messages so the orchestrator can continue
//! across a multi-turn conversation.

use crate::ai::multi_agent::types::{AgentContext, AgentMode, Finding, Task, TaskList};
use crate::db::Database;
use chrono::Utc;
use rusqlite::{params, Result as SqliteResult};
use serde::{Deserialize, Serialize};

/// Serializable wrapper for TaskList (since TaskList has private fields)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskListJson {
    tasks: Vec<Task>,
}

impl Database {
    /// Get agent context for a session (if exists)
    pub fn get_agent_context(&self, session_id: i64) -> SqliteResult<Option<AgentContext>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT original_request, mode, context_sufficient, plan_ready,
                    mode_iterations, total_iterations, exploration_notes,
                    findings, plan_summary, scratchpad, tasks_json
             FROM agent_contexts
             WHERE session_id = ?",
        )?;

        let result = stmt.query_row(params![session_id], |row| {
            let original_request: String = row.get(0)?;
            let mode_str: String = row.get(1)?;
            let context_sufficient: bool = row.get::<_, i32>(2)? != 0;
            let plan_ready: bool = row.get::<_, i32>(3)? != 0;
            let mode_iterations: u32 = row.get(4)?;
            let total_iterations: u32 = row.get(5)?;
            let notes_json: String = row.get(6)?;
            let findings_json: String = row.get(7)?;
            let plan_summary: Option<String> = row.get(8)?;
            let scratchpad: String = row.get(9)?;
            let tasks_json: String = row.get(10)?;

            // Parse mode
            let mode = AgentMode::from_str(&mode_str).unwrap_or_default();

            // Parse JSON fields
            let exploration_notes: Vec<String> =
                serde_json::from_str(&notes_json).unwrap_or_default();
            let findings: Vec<Finding> =
                serde_json::from_str(&findings_json).unwrap_or_default();

            // Parse tasks
            let task_list_json: TaskListJson =
                serde_json::from_str(&tasks_json).unwrap_or(TaskListJson { tasks: vec![] });
            let tasks = TaskList::from_vec(task_list_json.tasks);

            Ok(AgentContext {
                original_request,
                exploration_notes,
                findings,
                tasks,
                plan_summary,
                mode,
                mode_iterations,
                total_iterations,
                context_sufficient,
                plan_ready,
                scratchpad,
            })
        });

        match result {
            Ok(ctx) => Ok(Some(ctx)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Create or update agent context for a session
    pub fn save_agent_context(
        &self,
        session_id: i64,
        context: &AgentContext,
    ) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        // Serialize JSON fields
        let notes_json = serde_json::to_string(&context.exploration_notes)
            .unwrap_or_else(|_| "[]".to_string());
        let findings_json = serde_json::to_string(&context.findings)
            .unwrap_or_else(|_| "[]".to_string());
        let tasks_json = serde_json::to_string(&TaskListJson {
            tasks: context.tasks.all().to_vec(),
        })
        .unwrap_or_else(|_| "{\"tasks\":[]}".to_string());

        // Use INSERT OR REPLACE for upsert behavior
        conn.execute(
            "INSERT OR REPLACE INTO agent_contexts (
                session_id, original_request, mode, context_sufficient, plan_ready,
                mode_iterations, total_iterations, exploration_notes, findings,
                plan_summary, scratchpad, tasks_json, created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                COALESCE((SELECT created_at FROM agent_contexts WHERE session_id = ?1), ?13),
                ?13
            )",
            params![
                session_id,
                context.original_request,
                context.mode.to_string(),
                context.context_sufficient as i32,
                context.plan_ready as i32,
                context.mode_iterations,
                context.total_iterations,
                notes_json,
                findings_json,
                context.plan_summary,
                context.scratchpad,
                tasks_json,
                now,
            ],
        )?;

        Ok(())
    }

    /// Delete agent context for a session (e.g., on session reset)
    pub fn delete_agent_context(&self, session_id: i64) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM agent_contexts WHERE session_id = ?",
            params![session_id],
        )?;
        Ok(())
    }

    /// Check if a session has an agent context
    pub fn has_agent_context(&self, session_id: i64) -> SqliteResult<bool> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM agent_contexts WHERE session_id = ?",
            params![session_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
}
