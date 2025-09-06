# Context Management & History

## Overview
Parsec manages user interaction through a hierarchical context system: **Session → Conversation → Steps**. This document specifies the context management strategy, history preservation, and prompt context injection for maintaining coherent AI-assisted workflows across extended interactions.

## Hierarchical Structure

### Session
The top-level container representing a user's continuous interaction with Parsec.

```rust
pub struct Session {
    pub id: SessionId, // ULID for chronological ordering
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub conversations: Vec<ConversationId>, // only for prompt-classified inputs
    pub command_history: Vec<DirectCommandExecution>, // for shell-classified inputs
    pub global_context: GlobalContext,
    pub settings: SessionSettings,
}

pub struct GlobalContext {
    pub working_directory: PathBuf,
    pub environment_snapshot: HashMap<String, String>,
    pub detected_project_type: Option<ProjectType>, // rust, python, node, etc.
    pub active_tools: Vec<String>, // git, cargo, npm, etc.
}

pub struct DirectCommandExecution {
    pub command: String,
    pub executed_at: DateTime<Utc>,
    pub exit_status: i32,
    pub stdout: TruncatedText,
    pub stderr: TruncatedText,
    pub working_directory: PathBuf,
}
```

### Conversation (Prompt-Classified Input Only)
A focused AI-assisted workflow addressing a specific user goal within a session. Only created when user input is classified as a natural language prompt.

```rust
pub struct Conversation {
    pub id: ConversationId,
    pub session_id: SessionId,
    pub name: String, // user-friendly display name
    pub original_prompt: String,
    pub created_at: DateTime<Utc>,
    pub status: ConversationStatus,
    pub workflow: Option<WorkflowPlan>,
    pub steps: Vec<WorkflowStepState>,
    pub context_summary: ContextSummary,
}

pub struct ContextSummary {
    pub key_achievements: Vec<String>, // major accomplishments from this conversation
    pub generated_artifacts: Vec<ArtifactInfo>, // files created/modified
    pub environment_changes: Vec<EnvironmentChange>, // PATH, env vars, etc.
    pub learned_preferences: HashMap<String, String>, // user patterns observed
}
```

### Step (Within Conversations Only)
Individual actionable units within a conversation workflow. Only exists for prompt-classified inputs that generate multi-step workflows.

```rust
pub struct WorkflowStepState {
    pub step: WorkflowStep,
    pub status: StepStatus,
    pub command_attempts: Vec<CommandAttempt>,
    pub context_used: StepContext, // what context was provided to the model
    pub artifacts_produced: Vec<ArtifactInfo>,
}
```

## Context Injection Strategy

### For Direct Shell Commands (Shell-Classified Input)
Direct shell commands bypass the conversation system and execute immediately within session context. No additional context injection is needed beyond current environment state.

### For Workflow Planning (Prompt-Classified Input)
When generating the initial workflow plan for prompt-classified input, the model receives:

```json
{
  "user_prompt": "Initialize a new Rust project with CI/CD",
  "session_context": {
    "working_directory": "/home/user/projects",
    "detected_tools": ["git", "cargo", "docker"],
    "project_type": null,
    "recent_conversations": [
      {
        "name": "Python API Setup",
        "key_achievements": ["Created FastAPI project", "Set up Docker"],
        "completed_at": "2025-09-06T10:30:00Z"
      }
    ]
  },
  "conversation_history": [], // empty for new conversations
  "preferences": {
    "preferred_license": "MIT",
    "ci_provider": "github-actions"
  }
}
```

### For Step Command Generation (Within Prompt-Classified Conversations)
When generating commands for a specific step within a prompt-classified conversation, comprehensive context is provided:

```json
{
  "conversation": {
    "id": "01J123456789ABCDEF",
    "name": "Rust Project Setup",
    "original_prompt": "Initialize a new Rust project with CI/CD"
  },
  "workflow": {
    "steps": [
      {"description": "Create new Cargo project"},
      {"description": "Initialize git repository"}, 
      {"description": "Add CI/CD configuration"},
      {"description": "Create initial commit"}
    ]
  },
  "current_step": {
    "index": 1,
    "description": "Initialize git repository"
  },
  "execution_history": [
    {
      "step_index": 0,
      "commands_executed": ["cargo new myproject --lib"],
      "success": true,
      "artifacts_created": ["myproject/Cargo.toml", "myproject/src/lib.rs"],
      "working_directory_changed": "/home/user/projects/myproject"
    }
  ],
  "current_environment": {
    "working_directory": "/home/user/projects/myproject",
    "detected_files": ["Cargo.toml", "src/lib.rs"],
    "git_status": "not_a_repository"
  },
  "error_context": null // populated if retrying after failure
}
```

## History Preservation Levels

### Session-Level History (All Input Types)
- Direct shell command executions with results
- Environment state changes
- Working directory changes
- Detected project evolution

### Conversation-Level Context (Prompt-Classified Only)
- Complete step execution history
- All command attempts with outputs
- Generated artifacts
- Workflow completion status

### Cross-Conversation Learning (Prompt-Classified Only)
- Key achievements from completed conversations
- Learned user preferences and patterns
- Common workflow templates
- Project-specific optimization opportunities

## Context Pruning Strategy

### Token Limit Management
When context approaches model token limits:

1. **Preserve Core Information:**
   - Current step description and index
   - Last 3 successful command executions
   - Current environment state
   - Any error context

2. **Summarize Historical Data:**
   - Aggregate older step results into achievements
   - Compress repeated pattern information
   - Maintain only essential error context

3. **Progressive Truncation:**
   - Remove oldest non-essential conversation history
   - Summarize instead of including full command outputs
   - Preserve step completion status over detailed logs

### Context Relevance Scoring
```rust
pub struct ContextItem {
    pub content: String,
    pub relevance_score: f32, // 0.0 to 1.0
    pub recency_weight: f32,
    pub importance_level: ImportanceLevel, // Critical, High, Medium, Low
    pub context_type: ContextType, // Environment, Command, Achievement, Error
}

pub enum ImportanceLevel {
    Critical,  // Current step, active errors
    High,      // Recent successes, environment changes
    Medium,    // Older achievements, learned patterns
    Low,       // Historical context, repeated patterns
}
```

## Cross-Conversation Learning

### Pattern Recognition
Track recurring user patterns across conversations:
- Preferred command variations (e.g., `ls -la` vs `ll`)
- Common project setup sequences
- Frequently used tool combinations
- Error recovery preferences

### Context Sharing
Enable intelligent context sharing between related conversations:
- Project-specific settings and preferences
- Previously established environment configurations
- Successful command patterns for similar tasks

## Implementation Considerations

### Performance
- Context serialization/deseriization optimization
- Lazy loading of historical data
- Efficient context pruning algorithms
- Memory usage monitoring for long-running sessions

### Privacy and Security
- Sensitive information detection and redaction
- Configurable history retention policies
- Option to exclude certain commands from history
- Secure storage of context data

### Recovery and Persistence
```rust
pub trait ContextStore {
    fn save_session(&self, session: &Session) -> Result<(), ContextError>;
    fn load_session(&self, session_id: &SessionId) -> Result<Session, ContextError>;
    fn save_conversation(&self, conversation: &Conversation) -> Result<(), ContextError>;
    fn load_conversation(&self, conversation_id: &ConversationId) -> Result<Conversation, ContextError>;
    fn prune_old_context(&self, retention_policy: &RetentionPolicy) -> Result<(), ContextError>;
}
```

## Usage Examples

### Starting a New Session
```rust
let session = Session::new(
    working_directory: "/home/user/projects",
    detected_tools: vec!["git", "cargo", "npm"],
);

// For shell-classified input - direct execution
session.execute_direct_command("ls -la");

// For prompt-classified input - create conversation
let conversation = Conversation::new(
    session_id: session.id,
    name: "Rust Web Server Setup",
    prompt: "Create a new Rust web server with database integration",
);
```

### Context-Aware Command Generation
The system automatically provides relevant context to the model:
- Previous conversation outcomes in the same directory
- Detected project structure and tooling
- User preferences learned from similar tasks
- Current environment state and available tools

### Cross-Conversation Intelligence
When a user starts a new conversation in a directory where previous work was completed:
```
Previous context found: "Python API Setup" completed 2 hours ago
Key artifacts: docker-compose.yml, requirements.txt, .env.example
Applying learned preferences: MIT license, GitHub Actions CI
```

## Future Enhancements

### Smart Context Injection
- Semantic similarity matching for relevant historical context
- Dynamic context window optimization based on task complexity
- Predictive context pre-loading for anticipated next steps

### Collaborative Context
- Shared context for team environments
- Project-level context accessible across team members
- Context synchronization across multiple development machines

### Analytics and Insights
- Context utilization metrics for optimization
- User behavior pattern analysis
- Workflow efficiency improvements based on context usage

## Configuration Options

```toml
[context_management]
max_conversation_history = 50
session_retention_days = 30
enable_cross_conversation_learning = true
context_compression_threshold = 0.8
privacy_mode = false

[context_pruning]
max_command_output_length = 1024
preserve_error_context_steps = 5
importance_decay_factor = 0.9
```

This context management system ensures that AI models receive optimal information for generating relevant, context-aware commands while maintaining performance and respecting user privacy preferences.
