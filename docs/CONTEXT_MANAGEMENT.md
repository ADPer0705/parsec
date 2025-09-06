# Context Management & History

## 🔍 Overview

Parsec implements a sophisticated hierarchical context system: **Session → Conversation → Steps**, enabling coherent AI-assisted workflows across extended interactions. This document details context management, history preservation, and intelligent prompt context injection for seamless multi-step AI orchestration.

## 🏗️ Hierarchical Structure

### Session
The foundational container for user interactions, representing continuous engagement with Parsec.

```rust
pub struct Session {
    pub id: SessionId, // ULID for temporal ordering
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub conversations: Vec<ConversationId>, // exclusive to prompt-classified inputs
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

### Conversation (Prompt-Classified Inputs Only)
A focused AI workflow targeting a specific user objective within a session. Exclusively created for natural language prompts.

```rust
pub struct Conversation {
    pub id: ConversationId,
    pub session_id: SessionId,
    pub name: String, // human-readable display name
    pub original_prompt: String,
    pub created_at: DateTime<Utc>,
    pub status: ConversationStatus,
    pub workflow: Option<WorkflowPlan>,
    pub steps: Vec<WorkflowStepState>,
    pub context_summary: ContextSummary,
}

pub struct ContextSummary {
    pub key_achievements: Vec<String>, // major accomplishments
    pub generated_artifacts: Vec<ArtifactInfo>, // created/modified files
    pub environment_changes: Vec<EnvironmentChange>, // PATH, env vars, etc.
    pub learned_preferences: HashMap<String, String>, // observed patterns
}
```

### Step (Conversation-Exclusive)
Individual actionable units within conversation workflows. Exists solely for prompt-classified inputs with multi-step plans.

```rust
pub struct WorkflowStepState {
    pub step: WorkflowStep,
    pub status: StepStatus,
    pub command_attempts: Vec<CommandAttempt>,
    pub context_used: StepContext, // context provided to model
    pub artifacts_produced: Vec<ArtifactInfo>,
}
```

## 🎯 Context Injection Strategy

### Direct Shell Commands (Shell-Classified)
Bypass conversation system; execute immediately within session context. No additional context required beyond current environment.

### Workflow Planning (Prompt-Classified)
Initial workflow generation receives:

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

### Step Command Generation (Within Conversations)
Comprehensive context for per-step command synthesis:

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
  "error_context": null // populated on retry
}
```

## 📚 History Preservation Tiers

### Session-Level (All Inputs)
- Direct command executions with results
- Environment state modifications
- Working directory updates
- Project evolution detection

### Conversation-Level (Prompt-Only)
- Complete step execution logs
- All command attempts with outputs
- Generated artifacts tracking
- Workflow completion status

### Cross-Conversation Intelligence (Prompt-Only)
- Achievements from completed workflows
- Learned user preferences
- Common workflow templates
- Project-specific optimizations

## 🗂️ Context Pruning Strategy

### Token Management
Approaching model limits:

1. **Core Preservation:**
   - Current step details and index
   - Last 3 successful executions
   - Current environment state
   - Active error context

2. **Historical Summarization:**
   - Aggregate older results into achievements
   - Compress repetitive patterns
   - Retain essential error data

3. **Progressive Reduction:**
   - Remove oldest non-critical history
   - Summarize vs. full outputs
   - Prioritize completion status over logs

### Relevance Scoring
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
    High,      // Recent successes, env changes
    Medium,    // Older achievements, patterns
    Low,       // Historical, repeated data
}
```

## 🤖 Cross-Conversation Learning

### Pattern Recognition
Track recurring patterns:
- Preferred command styles (`ls -la` vs `ll`)
- Common setup sequences
- Tool combination preferences
- Error recovery strategies

### Context Sharing
Intelligent sharing between related workflows:
- Project-specific configurations
- Established environment setups
- Successful patterns for similar tasks

## ⚙️ Implementation Considerations

### Performance
- Optimized serialization/deserialization
- Lazy historical data loading
- Efficient pruning algorithms
- Memory monitoring for long sessions

### Privacy & Security
- Sensitive data detection and masking
- Configurable retention policies
- Excludable commands from history
- Secure context storage

### Recovery & Persistence
```rust
pub trait ContextStore {
    fn save_session(&self, session: &Session) -> Result<(), ContextError>;
    fn load_session(&self, session_id: &SessionId) -> Result<Session, ContextError>;
    fn save_conversation(&self, conversation: &Conversation) -> Result<(), ContextError>;
    fn load_conversation(&self, conversation_id: &ConversationId) -> Result<Conversation, ContextError>;
    fn prune_old_context(&self, retention_policy: &RetentionPolicy) -> Result<(), ContextError>;
}
```

## 💡 Usage Examples

### New Session Initialization
```rust
let session = Session::new(
    working_directory: "/home/user/projects",
    detected_tools: vec!["git", "cargo", "npm"],
);

// Shell input - direct execution
session.execute_direct_command("ls -la");

// Prompt input - conversation creation
let conversation = Conversation::new(
    session_id: session.id,
    name: "Rust Web Server Setup",
    prompt: "Create a new Rust web server with database integration",
);
```

### Context-Aware Generation
Automatic relevant context provision:
- Prior outcomes in directory
- Detected project structure
- Learned preferences from similar tasks
- Current environment and tools

### Cross-Conversation Intelligence
New conversation in active directory:
```
Previous context: "Python API Setup" completed 2 hours ago
Artifacts: docker-compose.yml, requirements.txt, .env.example
Applied preferences: MIT license, GitHub Actions CI
```

## 🚀 Future Enhancements

### Smart Injection
- Semantic similarity for historical relevance
- Dynamic window optimization by complexity
- Predictive pre-loading for next steps

### Collaborative Context
- Team-shared contexts
- Project-level accessibility
- Multi-machine synchronization

### Analytics & Insights
- Context utilization metrics
- User pattern analysis
- Efficiency improvements via usage data

## ⚙️ Configuration

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

This advanced context management ensures optimal model information for relevant, aware command generation while maintaining performance and privacy.
