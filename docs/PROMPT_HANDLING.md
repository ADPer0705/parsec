# Prompt Handling & Workflow Execution

This document specifies how Parsec processes natural-language prompts (classified as `InputKind::Prompt`) into an interactive, stepwise execution workflow. This workflow system is only activated for prompt-classified inputs; shell-classified commands are executed directly within the session context without conversation or step management.

The document covers the complete lifecycle, state machine design, trait definitions, model interactions, error handling, and future provider extensibility for AI-assisted multi-step workflows.

## Objectives
- Deterministic orchestration around inherently non-deterministic LLM output for prompt-classified inputs.
- Clear separation of CONVERSATION (prompt session), WORKFLOW (logical plan), and COMMAND GENERATION (per-step execution).
- Maintainable, resumable state with comprehensive auditable history.
- Model-agnostic abstractions (Google AI Studio implementation first, with support for local and other APIs planned).
- Direct shell command execution bypasses this system entirely, maintaining session context only.

## High-Level Process Flow
1. The classifier marks user input as a prompt.
2. Create or retrieve active `Session`, then create new `ConversationContext` with new `conversation_id` (ULID/UUIDv7), generate user-friendly conversation name, and store initial user prompt.
3. Invoke `WorkflowPlanner` (model) with session context and conversation history to return ordered logical steps (structured JSON) with human descriptions only.
4. Present complete plan to user with status indicator: `[<Conversation Name>] (Planning Complete)`.
5. For each step in sequential order:
   - Upon user continuation, call `StepCommandGenerator` with full conversation context, session history, and environment state.
   - Receive candidate command(s) via structured JSON response and display primary candidate.
   - User can approve, request alternative, or abort.
   - Upon approval, executor runs command with result appended to context and session history.
   - Repeat until step is satisfied (some steps may require multiple commands—model signals completion when none remain).
6. Proceed to next step until all steps are complete or conversation is aborted.
7. Persist transcript (future persistence layer).

## State Machine

**Conversation Status:** `Planning | Ready | InProgress | Finished | Aborted | Error`  
**Step Status:** `Pending | CommandSuggested | Running | Complete | Failed | Skipped`

**State Transitions:**
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

## Core Data Structures (Rust Implementation)
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
    pub name: String, // user-friendly display name for the conversation
    pub user_prompt: String,
    pub workflow: WorkflowPlan, // populated after planning phase
    pub steps: Vec<WorkflowStepState>,
    pub status: ConversationStatus,
    pub history: Vec<ConversationEvent>,
    pub model_provider: ModelProviderId,
    pub context_summary: ContextSummary, // key achievements and artifacts
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

## Trait Definitions
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

## Model Interactions

The system employs two distinct prompt templates:

### 1. Planning Prompt (WorkflowPlanner)
**Objective:** Generate a JSON array of high-level steps without shell commands. Emphasizes idempotent, minimal, ordered steps with full session and conversation context awareness.

**Context provided:**
```
SYSTEM: You are an assistant that decomposes a user goal into a small ordered workflow of logical steps. DO NOT produce shell commands. Output strict JSON format only.
SESSION_CONTEXT: <working directory, detected tools, project type, recent conversations>
CONVERSATION_HISTORY: <previous related conversations and their outcomes>
USER_PROMPT: <raw user text>
RESPONSE FORMAT (JSON): { "steps": [ { "description": "..." }, ... ] }
CONSTRAINTS: 1-12 steps maximum. Each description should be 3-14 words, starting with an imperative verb.
```
**Parser:** Strict JSON parsing with comprehensive error handling for malformed responses.

### 2. Step Command Generation Prompt (StepCommandGenerator)
**Objective:** Given comprehensive context including session state, conversation history, and current step, produce candidate shell commands or signal completion.
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

**Validation rules:**
- Reject commands containing unescaped newlines unless pipeline operations are required.
- Issue soft warnings and require explicit user confirmation for dangerous patterns: `rm -rf`, `:(){:|:&};:`, `dd if=/dev/`.
- Truncate displayed stdout/stderr beyond configurable size limit (X KB).

## Error Handling
| Failure Point | Strategy |
|---------------|----------|
| Planning timeout | Retry once with exponential backoff, otherwise surface PlanError and mark conversation as Error. |
| Invalid planning JSON | Return PlanError with descriptive message indicating JSON parsing failure. |
| Step generation invalid JSON | Return CommandGenError with descriptive message indicating JSON parsing failure. |
| Command execution non-zero exit | Mark attempt as failed; offer: (a) retry same step (new command), (b) skip (if user chooses), (c) abort conversation. |
| User abort | Mark conversation as `Aborted`; no further model calls. |

## Conversation Status Indicator
The user interface displays a status line:
`[Rust Project Setup] Step 2/5 (Pending) | Provider: google-ai | Next: Generate command`

This status updates after each state transition and helps maintain mental model context when switching between different conversations.

## Persistence (Future Implementation)
A pluggable `SessionStore` trait will be implemented for managing both sessions and conversations (currently in-memory; future implementations include SQLite/flat file storage). This will enable resuming conversations by persisting session state, conversation plans, and completed steps, then rehydrating context for remaining work.

```rust
pub trait SessionStore {
    fn save_session(&self, session: &Session) -> Result<(), StoreError>;
    fn load_session(&self, session_id: &SessionId) -> Result<Session, StoreError>;
    fn save_conversation(&self, conversation: &Conversation) -> Result<(), StoreError>;
    fn load_conversation(&self, conversation_id: &ConversationId) -> Result<Conversation, StoreError>;
    fn list_active_sessions(&self) -> Result<Vec<SessionSummary>, StoreError>;
}
```

## Extensibility and Multi-Provider Support
`ModelProvider` registry keyed by identifier: `google-ai`, `local-llm`, etc. Configuration precedence:
1. CLI flag or UI selection
2. Environment variable `PARSEC_MODEL_PROVIDER`
3. Configuration file `~/.config/parsec/config.toml`
4. Default: `google-ai`

## Security Layer Enhancements (Planned)
- Risk scoring system before user approval.
- Automatic sandboxing for high-risk scores above threshold.
- Red-team prompt injection filters (scan model outputs for disallowed tokens).

## Minimal Example (Narrative)
**User input:** "Initialize a new Rust crate with MIT license and run tests"

**Planner JSON output:**
Steps:
1. Create new cargo library project
2. Add MIT license file
3. Initialize git repository
4. Run tests

**Execution flow:**
- Step 1 generation returns: `cargo new mylib --lib`
- User approves → command executed successfully
- Step 1 marked complete; Step 2 generation uses previous result (directory created)
- Returns multi-candidate options with only first shown initially
- Process continues through remaining steps

## Non-Goals (Current Scope)
- Parallel step execution
- Automatic rollback or transactional execution
- Multi-user collaborative conversations

## Open Questions
- Should command alternatives be pre-fetched (N>1) versus on-demand? (Current approach: on-demand to maintain lower latency)
- Where should rate limiting be enforced? (Likely at provider wrapper level)

## Python Integration Architecture

The `parsec-prompt` crate integrates with Python-based ML/LLM workflows while maintaining modularity and performance. This hybrid approach leverages the Python ML ecosystem for AI operations while keeping core system components in Rust.

### Folder Structure
```
crates/prompt/
├── Cargo.toml
├── src/
│   └── lib.rs          # Rust API and glue code
└── py/
    ├── workflow.py     # ML/LLM workflow logic
    ├── llm_api.py      # Google AI Studio API helpers
    └── requirements.txt # Python dependencies
```

### Integration Methods
The Rust code interfaces with Python through two primary methods:

#### 1. PyO3 Embedding
Direct embedding of Python interpreter within the Rust process for optimal performance:

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
Process-based execution for environments requiring isolation:

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
The `src/lib.rs` provides a clean Rust API that abstracts the Python integration details:

```rust
// Public API exposed to other crates
pub trait WorkflowPlanner {
    fn plan(&self, user_prompt: &str, opts: PlanningOptions) -> Result<WorkflowPlan, PlanError>;
}

// Internal implementation wrapping Python code
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

### Benefits of This Architecture

1. **Modularity**: Python ML logic is isolated from core Rust systems, enabling independent updates and testing.

2. **Flexibility**: Easy to swap between different LLM providers or local models by modifying Python scripts without Rust recompilation.

3. **Performance**: PyO3 embedding maintains high terminal performance by avoiding subprocess overhead for frequent operations.

4. **Ecosystem Access**: Leverages the rich Python ML ecosystem (transformers, langchain, etc.) while maintaining Rust's safety and performance for system operations.

5. **Development Velocity**: Data scientists can iterate on ML logic in Python while systems engineers maintain core functionality in Rust.

### Error Handling
The integration includes comprehensive error handling for common failure modes:
- Python import errors
- JSON serialization/deserialization failures
- API rate limiting and network errors
- Model inference timeouts

## Summary
The prompt handling system separates planning from execution phases. Clear state management, strict JSON interfaces, and pluggable provider architecture ensure the system can evolve from Google AI Studio to local LLMs without requiring rework of the UI or executor components. The Python integration layer provides flexible access to ML/LLM capabilities while maintaining the performance and reliability characteristics of the core Rust system.

