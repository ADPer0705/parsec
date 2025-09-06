# Prompt Handling & Workflow Execution

## 🎯 Objectives

Parsec orchestrates natural-language prompts (`InputKind::Prompt`) into deterministic, stepwise workflows. This isolates conversation management from command generation, ensuring auditable, resumable state with comprehensive history. Model-agnostic abstractions support Google AI Studio initially, with local/other providers planned. Direct shell commands bypass this entirely, maintaining session context only.

## 🔄 High-Level Process Flow

1. Classifier identifies input as prompt.
2. Create/retrieve active `Session`, generate unique Conversation ID, user-friendly name, and initialize `ConversationContext`.
3. Invoke `WorkflowPlanner` (Model Call #1) with session context and conversation history for ordered logical steps (structured JSON, no commands).
4. Display complete workflow with status: `[<Conversation Name>] (Planning Complete)`.
5. For each step sequentially:
   - Upon user approval, call `StepCommandGenerator` (Model Call #2+) with full context (session state, conversation history, step index, prior executions, environment deltas, error states).
   - Receive candidate commands via structured JSON; display primary option.
   - User can approve, request alternatives, or abort.
   - Approved commands execute via executor, updating conversation and session contexts.
   - Repeat until step satisfied (multiple commands possible; model signals completion).
6. Advance to next pending step until all complete or aborted.
7. Persist transcript (future persistence).

## 🔄 State Machine

**Conversation Status:** `Planning | Ready | InProgress | Finished | Aborted | Error`  
**Step Status:** `Pending | CommandSuggested | Running | Complete | Failed | Skipped`

**Transitions:**
```
Start -> Planning -> Ready (plan received)
Ready -> InProgress (user approves start)
InProgress + step Pending -> CommandSuggested (after model command generation)
CommandSuggested + user approve -> Running -> (success) -> Complete
CommandSuggested + user alternative -> CommandSuggested (new candidate)
Running (error) -> Failed (user may retry -> CommandSuggested)
Any state -> Aborted (user abort)
All steps Complete -> Finished
Model or parsing failure -> Error
```

## 🏗️ Core Data Structures

```rust
pub struct Session {
    pub id: SessionId,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub conversations: Vec<ConversationId>,
    pub global_context: GlobalContext,
}

pub struct ConversationContext {
    pub id: ConversationId,
    pub session_id: SessionId,
    pub name: String, // user-friendly display name
    pub user_prompt: String,
    pub workflow: WorkflowPlan, // populated after planning
    pub steps: Vec<WorkflowStepState>,
    pub status: ConversationStatus,
    pub history: Vec<ConversationEvent>,
    pub model_provider: ModelProviderId,
    pub context_summary: ContextSummary, // achievements and artifacts
}

pub struct WorkflowPlan { 
    pub steps: Vec<WorkflowStep>; 
}

pub struct WorkflowStep { 
    pub id: StepId, 
    pub description: String; 
}

pub struct WorkflowStepState {
    pub step: WorkflowStep,
    pub status: StepStatus,
    pub command_attempts: Vec<CommandAttempt>,
}

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
```

## 🔧 Trait Definitions

```rust
pub trait WorkflowPlanner {
    fn plan(&self, user_prompt: &str, session_context: &Session, opts: PlanningOptions) -> Result<WorkflowPlan, PlanError>;
}

pub trait StepCommandGenerator {
    fn generate_command(&self, ctx: &ConversationContext, session: &Session, step_index: usize, opts: CommandGenOptions) -> Result<GeneratedCommands, CommandGenError>;
}

pub trait ModelProvider: Send + Sync {
    fn planner(&self) -> &dyn WorkflowPlanner;
    fn step_generator(&self) -> &dyn StepCommandGenerator;
    fn name(&self) -> &'static str;
}
```

## 🤖 Model Interactions

Two distinct prompt templates:

### 1. Planning Prompt (WorkflowPlanner)
**Objective:** Generate JSON array of high-level steps without commands. Emphasizes idempotent, minimal, ordered steps with full context awareness.

**Context Provided:**
```
SYSTEM: You are an assistant that decomposes a user goal into a small ordered workflow of logical steps. DO NOT produce shell commands. Output strict JSON format only.
SESSION_CONTEXT: <working directory, detected tools, project type, recent conversations>
CONVERSATION_HISTORY: <previous related conversations and their outcomes>
USER_PROMPT: <raw user text>
RESPONSE FORMAT (JSON): { "steps": [ { "description": "..." }, ... ] }
CONSTRAINTS: 1-12 steps maximum. Each description should be 3-14 words, starting with an imperative verb.
```
**Parser:** Strict JSON parsing with comprehensive error handling.

### 2. Step Command Generation Prompt (StepCommandGenerator)
**Objective:** Produce candidate shell commands or signal completion with comprehensive context.

```
SYSTEM: You generate safe shell commands for the CURRENT step only.
SECURITY: Avoid destructive commands unless explicitly required; NEVER use 'rm -rf /'. Ask for clarification if ambiguous.
SESSION_CONTEXT: <working directory, environment, detected tools, project type>
CONVERSATION_CONTEXT: <conversation name, original prompt, previous conversations in this session>
WORKFLOW (all steps): <indexed list>
CURRENT_STEP: <index + description>
EXECUTION_HISTORY: Complete history of executed steps with results, artifacts created, environment changes
ERROR_CONTEXT (if retry): <last error message or stderr excerpt>
OUTPUT FORMAT (JSON): { "commands": [ { "command": "...", "explanation": "..." } ], "done": false }
If step complete without command: { "commands": [], "done": true }
```

**Validation Rules:**
- Reject commands with unescaped newlines unless pipelines required.
- Soft warnings for dangerous patterns: `rm -rf`, `:(){:|:&};:`, `dd if=/dev/`.
- Truncate stdout/stderr beyond configurable limits (X KB).

## 🚨 Error Handling

| Failure Point | Strategy |
|---------------|----------|
| Planning timeout | Retry with exponential backoff, otherwise surface PlanError and mark as Error. |
| Invalid planning JSON | Return PlanError with descriptive JSON parsing failure. |
| Step generation invalid JSON | Return CommandGenError with descriptive parsing failure. |
| Command execution non-zero exit | Mark failed; offer retry, skip, or abort. |
| User abort | Mark `Aborted`; no further calls. |

## 📊 Conversation Status Indicator

UI displays: `[Rust Project Setup] Step 2/5 (Pending) | Provider: google-ai | Next: Generate command`

Updates after transitions, maintaining context when switching conversations.

## 💾 Persistence (Future)

Pluggable `SessionStore` for sessions and conversations (currently in-memory; future: SQLite/flat files). Enables resuming by persisting state and rehydrating context.

```rust
pub trait SessionStore {
    fn save_session(&self, session: &Session) -> Result<(), StoreError>;
    fn load_session(&self, session_id: &SessionId) -> Result<Session, StoreError>;
    fn save_conversation(&self, conversation: &Conversation) -> Result<(), StoreError>;
    fn load_conversation(&self, conversation_id: &ConversationId) -> Result<Conversation, StoreError>;
    fn list_active_sessions(&self) -> Result<Vec<SessionSummary>, StoreError>;
}
```

## 🔌 Extensibility & Multi-Provider Support

`ModelProvider` registry by identifier: `google-ai`, `local-llm`, etc. Configuration precedence:
1. CLI/UI selection
2. Environment `PARSEC_MODEL_PROVIDER`
3. Config file `~/.config/parsec/config.toml`
4. Default: `google-ai`

## 🔒 Security Enhancements (Planned)

- Risk scoring pre-approval.
- Automatic sandboxing for high-risk scores.
- Prompt injection filters on model outputs.

## 📝 Minimal Example

**User Input:** "Initialize a new Rust crate with MIT license and run tests"

**Planner Output:**
Steps:
1. Create new cargo library project
2. Add MIT license file
3. Initialize git repository
4. Run tests

**Execution:**
- Step 1: `cargo new mylib --lib` → approved → executed
- Step 1 complete; Step 2 uses prior results
- Multi-candidate options, first shown
- Continues through steps

## 🚫 Non-Goals (Current)

- Parallel step execution
- Automatic rollback/transactional execution
- Multi-user collaborative conversations

## ❓ Open Questions

- Pre-fetch command alternatives (N>1) vs. on-demand? (Current: on-demand for latency)
- Rate limiting enforcement? (Likely at provider wrapper)

## 🐍 Python Integration Architecture

`parsec-prompt` integrates Python ML/LLM workflows while maintaining modularity. Hybrid approach leverages Python ML ecosystem with Rust performance.

### Folder Structure
```
crates/prompt/
├── Cargo.toml
├── src/
│   └── lib.rs          # Rust API and glue
└── py/
    ├── workflow.py     # ML/LLM logic
    ├── llm_api.py      # Google AI API helpers
    └── requirements.txt # Python dependencies
```

### Integration Methods

#### 1. PyO3 Embedding
Direct embedding for optimal performance:

```rust
use pyo3::prelude::*;
use pyo3::types::PyDict;

pub struct PythonWorkflowPlanner {
    py_module: PyObject,
}

impl PythonWorkflowPlanner {
    pub fn new() -> Result<Self, PyErr> {
        Python::with_gil(|py| {
            let sys = py.import("sys")?;
            let path = sys.getattr("path")?;
            path.call_method1("append", ("./py",))?;
            
            let py_module = py.import("workflow")?;
            Ok(PythonWorkflowPlanner {
                py_module: py_module.to_object(py),
            })
        })
    }
    
    pub fn generate_plan(&self, user_prompt: &str) -> Result<WorkflowPlan, PlanError> {
        Python::with_gil(|py| {
            let kwargs = PyDict::new(py);
            kwargs.set_item("prompt", user_prompt)?;
            
            let result = self.py_module
                .call_method(py, "generate_workflow", (), Some(kwargs))?;
            
            let json_str: String = result.extract(py)?;
            serde_json::from_str(&json_str).map_err(|e| PlanError::InvalidJson(e))
        })
    }
}
```

#### 2. Subprocess Execution
Process-based for isolation:

```rust
use std::process::{Command, Stdio};

pub struct SubprocessWorkflowPlanner {
    python_path: String,
    script_path: String,
}

impl SubprocessWorkflowPlanner {
    pub fn generate_plan(&self, user_prompt: &str) -> Result<WorkflowPlan, PlanError> {
        let output = Command::new(&self.python_path)
            .arg(&self.script_path)
            .arg("--prompt")
            .arg(user_prompt)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| PlanError::ProcessError(e))?;
            
        if !output.status.success() {
            return Err(PlanError::PythonError(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }
        
        let json_output = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(&json_output).map_err(|e| PlanError::InvalidJson(e))
    }
}
```

### API Design
`src/lib.rs` provides clean Rust API abstracting Python details:

```rust
// Public API for other crates
pub trait WorkflowPlanner {
    fn plan(&self, user_prompt: &str, opts: PlanningOptions) -> Result<WorkflowPlan, PlanError>;
}

// Internal implementation wrapping Python
pub struct ModelProvider {
    planner: Box<dyn WorkflowPlanner>,
    step_generator: Box<dyn StepCommandGenerator>,
}

impl ModelProvider {
    pub fn new_google_ai() -> Result<Self, InitError> {
        let planner = Box::new(PythonWorkflowPlanner::new()?);
        let step_generator = Box::new(PythonStepGenerator::new()?);
        
        Ok(ModelProvider {
            planner,
            step_generator,
        })
    }
}
```

### Benefits

1. **Modularity**: Python ML isolated from Rust core, enabling independent updates.
2. **Flexibility**: Swap providers by modifying Python scripts without Rust recompilation.
3. **Performance**: PyO3 embedding avoids subprocess overhead.
4. **Ecosystem Access**: Rich Python ML (transformers, langchain) with Rust safety.
5. **Development Velocity**: Data scientists iterate in Python, engineers maintain Rust core.

### Error Handling
Comprehensive for:
- Python import errors
- JSON serialization failures
- API rate limiting/network errors
- Model inference timeouts

## 📋 Summary

Prompt handling separates planning from execution. Clear state management, strict JSON interfaces, and pluggable providers ensure evolution from Google AI to local LLMs without UI/executor rework. Python integration provides flexible ML access while maintaining Rust performance and reliability.

