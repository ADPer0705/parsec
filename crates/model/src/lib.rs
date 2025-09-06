use chrono::Utc;
use parsec_core::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod google_ai;

pub use google_ai::GoogleAiProvider;

#[derive(Debug, Serialize)]
struct PlanningPrompt {
    system: String,
    session_context: String,
    conversation_history: String,
    user_prompt: String,
    constraints: String,
}

#[derive(Debug, Serialize)]
struct StepCommandPrompt {
    system: String,
    session_context: String,
    conversation_context: String,
    workflow: String,
    current_step: String,
    execution_history: String,
    error_context: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlanningResponse {
    steps: Vec<StepDescription>,
}

#[derive(Debug, Deserialize)]
struct StepDescription {
    description: String,
}

#[derive(Debug, Deserialize)]
struct CommandGenerationResponse {
    commands: Vec<CommandDescription>,
    done: bool,
}

#[derive(Debug, Deserialize)]
struct CommandDescription {
    command: String,
    explanation: String,
}

pub trait ModelClient: Send + Sync {
    fn generate_text(
        &self,
        prompt: &str,
        max_tokens: Option<usize>,
    ) -> Result<String, anyhow::Error>;
}

pub struct InMemorySessionStore {
    sessions: std::sync::RwLock<HashMap<SessionId, Session>>,
    conversations: std::sync::RwLock<HashMap<ConversationId, ConversationContext>>,
}

impl InMemorySessionStore {
    pub fn new() -> Self {
        Self {
            sessions: std::sync::RwLock::new(HashMap::new()),
            conversations: std::sync::RwLock::new(HashMap::new()),
        }
    }
}

impl SessionStore for InMemorySessionStore {
    fn save_session(&self, session: &Session) -> Result<(), StoreError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| StoreError::StorageError("Failed to acquire write lock".to_string()))?;
        sessions.insert(session.id.clone(), session.clone());
        Ok(())
    }

    fn load_session(&self, session_id: &SessionId) -> Result<Session, StoreError> {
        let sessions = self
            .sessions
            .read()
            .map_err(|_| StoreError::StorageError("Failed to acquire read lock".to_string()))?;
        sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| StoreError::StorageError(format!("Session {} not found", session_id)))
    }

    fn save_conversation(&self, conversation: &ConversationContext) -> Result<(), StoreError> {
        let mut conversations = self
            .conversations
            .write()
            .map_err(|_| StoreError::StorageError("Failed to acquire write lock".to_string()))?;
        conversations.insert(conversation.id.clone(), conversation.clone());
        Ok(())
    }

    fn load_conversation(
        &self,
        conversation_id: &ConversationId,
    ) -> Result<ConversationContext, StoreError> {
        let conversations = self
            .conversations
            .read()
            .map_err(|_| StoreError::StorageError("Failed to acquire read lock".to_string()))?;
        conversations.get(conversation_id).cloned().ok_or_else(|| {
            StoreError::StorageError(format!("Conversation {} not found", conversation_id))
        })
    }

    fn list_active_sessions(&self) -> Result<Vec<SessionSummary>, StoreError> {
        let sessions = self
            .sessions
            .read()
            .map_err(|_| StoreError::StorageError("Failed to acquire read lock".to_string()))?;

        let summaries = sessions
            .values()
            .map(|session| SessionSummary {
                id: session.id.clone(),
                created_at: session.created_at,
                last_active: session.last_active,
                conversation_count: session.conversations.len(),
                working_directory: session.global_context.working_directory.clone(),
            })
            .collect();

        Ok(summaries)
    }

    fn prune_old_context(&self, retention_policy: &RetentionPolicy) -> Result<(), StoreError> {
        let cutoff_date =
            Utc::now() - chrono::Duration::days(retention_policy.session_retention_days as i64);

        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| StoreError::StorageError("Failed to acquire write lock".to_string()))?;

        sessions.retain(|_, session| session.last_active > cutoff_date);

        Ok(())
    }
}
