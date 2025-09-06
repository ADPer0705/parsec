use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub type SessionId = String; // ULID for chronological ordering
pub type ConversationId = String;
pub type StepId = String;
pub type ModelProviderId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputKind {
    Shell,
    Prompt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConversationStatus {
    Planning,
    Ready,
    InProgress,
    Finished,
    Aborted,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepStatus {
    Pending,
    CommandSuggested,
    Running,
    Complete,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImportanceLevel {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextType {
    Environment,
    Command,
    Achievement,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub conversations: Vec<ConversationId>,
    pub command_history: Vec<DirectCommandExecution>,
    pub global_context: GlobalContext,
    pub settings: SessionSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSettings {
    pub max_conversation_history: usize,
    pub session_retention_days: u32,
    pub enable_cross_conversation_learning: bool,
    pub context_compression_threshold: f32,
    pub privacy_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalContext {
    pub working_directory: PathBuf,
    pub environment_snapshot: HashMap<String, String>,
    pub detected_project_type: Option<String>,
    pub active_tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectCommandExecution {
    pub command: String,
    pub executed_at: DateTime<Utc>,
    pub exit_status: i32,
    pub stdout: TruncatedText,
    pub stderr: TruncatedText,
    pub working_directory: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    pub id: ConversationId,
    pub session_id: SessionId,
    pub name: String,
    pub user_prompt: String,
    pub workflow: Option<WorkflowPlan>,
    pub steps: Vec<WorkflowStepState>,
    pub status: ConversationStatus,
    pub history: Vec<ConversationEvent>,
    pub model_provider: ModelProviderId,
    pub context_summary: ContextSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSummary {
    pub key_achievements: Vec<String>,
    pub generated_artifacts: Vec<ArtifactInfo>,
    pub environment_changes: Vec<EnvironmentChange>,
    pub learned_preferences: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactInfo {
    pub file_path: PathBuf,
    pub artifact_type: String,
    pub created_at: DateTime<Utc>,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentChange {
    pub variable_name: String,
    pub old_value: Option<String>,
    pub new_value: String,
    pub changed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPlan {
    pub steps: Vec<WorkflowStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: StepId,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStepState {
    pub step: WorkflowStep,
    pub status: StepStatus,
    pub command_attempts: Vec<CommandAttempt>,
    pub context_used: StepContext,
    pub artifacts_produced: Vec<ArtifactInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepContext {
    pub working_directory: PathBuf,
    pub environment_vars: HashMap<String, String>,
    pub previous_outputs: Vec<String>,
    pub error_context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandAttempt {
    pub candidate: GeneratedCommand,
    pub approved: bool,
    pub executed: bool,
    pub exit_status: Option<i32>,
    pub stdout: TruncatedText,
    pub stderr: TruncatedText,
    pub error: Option<ExecutionError>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedCommand {
    pub command: String,
    pub explanation: String,
    pub risk_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedCommands {
    pub commands: Vec<GeneratedCommand>,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruncatedText {
    pub content: String,
    pub truncated: bool,
    pub original_length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationEvent {
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    pub content: String,
    pub relevance_score: f32,
    pub recency_weight: f32,
    pub importance_level: ImportanceLevel,
    pub context_type: ContextType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningOptions {
    pub max_steps: usize,
    pub include_context: bool,
    pub provider_specific: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandGenOptions {
    pub max_alternatives: usize,
    pub risk_threshold: f32,
    pub include_explanations: bool,
    pub provider_specific: HashMap<String, serde_json::Value>,
}

// Error types
#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error("Planning timeout: {0}")]
    Timeout(String),
    #[error("Invalid JSON response: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("Model provider error: {0}")]
    ModelError(String),
    #[error("Context error: {0}")]
    ContextError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum CommandGenError {
    #[error("Command generation timeout: {0}")]
    Timeout(String),
    #[error("Invalid JSON response: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("Model provider error: {0}")]
    ModelError(String),
    #[error("Context error: {0}")]
    ContextError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub enum ExecutionError {
    #[error("Command execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Command not found: {0}")]
    CommandNotFound(String),
    #[error("Timeout: {0}")]
    Timeout(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ClassificationError {
    #[error("Classification failed: {0}")]
    ClassificationFailed(String),
    #[error("Invalid JSON response: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("Python error: {0}")]
    PythonError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    #[error("Context error: {0}")]
    ContextError(String),
    #[error("Storage error: {0}")]
    StorageError(#[from] StoreError),
}

#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error("Initialization error: {0}")]
    InitError(String),
    #[error("Python initialization failed: {0}")]
    PythonInitError(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

// Core traits
pub trait CommandClassifier: Send + Sync {
    fn classify(
        &self,
        input: &str,
        context: Option<&Session>,
    ) -> Result<InputKind, ClassificationError>;
}

#[async_trait]
pub trait WorkflowPlanner: Send + Sync {
    async fn plan(
        &self,
        user_prompt: &str,
        session_context: &Session,
        opts: PlanningOptions,
    ) -> Result<WorkflowPlan, PlanError>;
}

#[async_trait]
pub trait StepCommandGenerator: Send + Sync {
    async fn generate_command(
        &self,
        ctx: &ConversationContext,
        session: &Session,
        step_index: usize,
        opts: CommandGenOptions,
    ) -> Result<GeneratedCommands, CommandGenError>;
}

pub trait ModelProvider: Send + Sync {
    fn planner(&self) -> &dyn WorkflowPlanner;
    fn step_generator(&self) -> &dyn StepCommandGenerator;
    fn name(&self) -> &'static str;
}

pub trait SessionStore: Send + Sync {
    fn save_session(&self, session: &Session) -> Result<(), StoreError>;
    fn load_session(&self, session_id: &SessionId) -> Result<Session, StoreError>;
    fn save_conversation(&self, conversation: &ConversationContext) -> Result<(), StoreError>;
    fn load_conversation(
        &self,
        conversation_id: &ConversationId,
    ) -> Result<ConversationContext, StoreError>;
    fn list_active_sessions(&self) -> Result<Vec<SessionSummary>, StoreError>;
    fn prune_old_context(&self, retention_policy: &RetentionPolicy) -> Result<(), StoreError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: SessionId,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub conversation_count: usize,
    pub working_directory: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub session_retention_days: u32,
    pub conversation_retention_days: u32,
    pub max_sessions: Option<usize>,
}

pub trait ContextStore: Send + Sync {
    fn save_session(&self, session: &Session) -> Result<(), ContextError>;
    fn load_session(&self, session_id: &SessionId) -> Result<Session, ContextError>;
    fn save_conversation(&self, conversation: &ConversationContext) -> Result<(), ContextError>;
    fn load_conversation(
        &self,
        conversation_id: &ConversationId,
    ) -> Result<ConversationContext, ContextError>;
    fn prune_old_context(&self, retention_policy: &RetentionPolicy) -> Result<(), ContextError>;
}

impl Default for SessionSettings {
    fn default() -> Self {
        Self {
            max_conversation_history: 50,
            session_retention_days: 30,
            enable_cross_conversation_learning: true,
            context_compression_threshold: 0.8,
            privacy_mode: false,
        }
    }
}

impl Default for PlanningOptions {
    fn default() -> Self {
        Self {
            max_steps: 12,
            include_context: true,
            provider_specific: HashMap::new(),
        }
    }
}

impl Default for CommandGenOptions {
    fn default() -> Self {
        Self {
            max_alternatives: 3,
            risk_threshold: 0.7,
            include_explanations: true,
            provider_specific: HashMap::new(),
        }
    }
}

impl TruncatedText {
    pub fn new(content: String, max_length: usize) -> Self {
        let original_length = content.len();
        if content.len() <= max_length {
            Self {
                content,
                truncated: false,
                original_length,
            }
        } else {
            let truncated_content = content.chars().take(max_length).collect();
            Self {
                content: truncated_content,
                truncated: true,
                original_length,
            }
        }
    }
}
