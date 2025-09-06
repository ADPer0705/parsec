# Architecture

## Overview
Parsec mediates between user input and two distinct execution paths based on classification: direct shell execution for shell commands, or AI-assisted multi-step workflow planning for natural language prompts. A classifier determines which path to take; AI-suggested commands require explicit user approval. Prompt handling operates through two distinct layers: (1) workflow planning (logical steps) and (2) per-step command synthesis and execution.

```
 ┌────────┐   raw text   ┌─────────────┐  kind    ┌─────────────┐  shell cmd      ┌──────────┐
 │  User  │ ───────────▶ │ Classifier  │ ───────▶ │  Dispatcher │ ───────────────▶│ Executor │
 └────────┘               └─────────────┘          └─────────────┘                 └──────────┘
                                        │                                  output / errors ▲
                                        │ prompt → conversation creation             │
                                        ▼                                              │
                                   ┌────────┐ workflow (logical steps)  ┌───────────┐  approve step  ┌──────────┐
                                   │ Model  │──────────────────────────▶│ UI Review │──────────────▶│ Executor │
                                   └────────┘                            └───────────┘                └──────────┘
                                                                              ▲  per-step command synthesis │
                                                                              └─────────────┬──────────────┘
                                                                                            │
                                                                                            ▼
                                                                                       ┌────────┐
                                                                                       │ Model  │ (command for current step)
                                                                                       └────────┘
```

## Crates
| Crate | Responsibility |
|-------|----------------|
| `parsec-core` | Domain types & traits (`CommandClassifier`, `ModelProvider`, `WorkflowPlanner`, `StepCommandGenerator`, `ExecutionPlan`, `ConversationContext`). |
| `parsec-model` | Implementations of `ModelProvider` (Google AI Studio first; pluggable later). |
| `parsec-executor` | Safe step/command execution (approval gate, future sandbox). |
| `parsec-prompt` | Orchestrates prompt → workflow planning then per-step command generation using core traits. Integrates with Python ML/LLM code via PyO3 or subprocess execution. Python files located in `py/` subdirectory for ML workflow logic and API helpers. |
| `parsec-classifier` | Embedded Python logic for classification using machine learning models. |
| `parsec-ui` | User interaction loop / future GUI. |c mediates between user input and two possible execution paths: direct shell execution or AI-assisted multi-step workflow planning. A classifier determines which path to take; AI-suggested commands require explicit user approval. Prompt handling operates through two distinct layers: (1) workflow planning (logical steps) and (2) per-step command synthesis and execution.

```
 ┌────────┐   raw text   ┌─────────────┐  kind    ┌─────────────┐  approved cmds  ┌──────────┐
 │  User  │ ───────────▶ │ Classifier  │ ───────▶ │  Dispatcher │ ───────────────▶│ Executor │
 └────────┘               └─────────────┘          └─────────────┘                 └──────────┘
                                        │                                  output / errors ▲
                                        │ model prompt / plan                          │
                                        ▼                                              │
                                   ┌────────┐ workflow (logical steps)  ┌───────────┐  approve step  ┌──────────┐
                                   │ Model  │──────────────────────────▶│ UI Review │──────────────▶│ Executor │
                                   └────────┘                            └───────────┘                └──────────┘
                                                                              ▲  per-step command synthesis │
                                                                              └─────────────┬──────────────┘
                                                                                            │
                                                                                            ▼
                                                                                       ┌────────┐
                                                                                       │ Model  │ (command for current step)
                                                                                       └────────┘
```

## Crates
| Crate | Responsibility |
|-------|----------------|
| `parsec-core` | Domain types & traits (`Session`, `CommandClassifier`, `ModelProvider`, `WorkflowPlanner`, `StepCommandGenerator`, `ExecutionPlan`, `ConversationContext`). |
| `parsec-model` | Implementations of `ModelProvider` (Google AI Studio first; pluggable later). |
| `parsec-executor` | Safe step/command execution (approval gate, future sandbox). |
| `parsec-prompt` | Orchestrates prompt -> workflow planning then per‑step command generation using core traits. |
| `parsec-classifier` | Embedded Python logic for classification (pluggable). |
| `parsec-ui` | User interaction loop / future GUI. |

## Data Flow

### Shell Command Path (Shell-Classified Input)
1. User enters text input.
2. Classifier returns `InputKind::Shell`.
3. System retrieves or creates active session.
4. Command is executed directly within session context.
5. Execution results (stdout/stderr, exit code) are recorded in session command history.
6. Session global context is updated (working directory, environment changes if any).

### Prompt Path (Prompt-Classified Input)
1. User enters text input.
2. Classifier returns `InputKind::Prompt`.
3. System retrieves or creates active session, generates new Conversation ID and user-friendly name, then initializes `ConversationContext` within the session.
4. UI requests a workflow plan from `WorkflowPlanner` (model call #1) with session context and conversation history, returning logical steps only (no shell commands yet).
5. UI displays complete workflow (all steps) and shows indicator: "[<Conversation Name>] active".
6. For the CURRENT step (status = Pending): when user approves continuation, UI invokes `StepCommandGenerator` (model call #2+) with comprehensive context including: session state, conversation history, current step index, prior executed steps, environment changes, and any errors encountered.
7. Model returns candidate commands for that step via structured JSON response. UI shows primary option; user can (a) approve and run, (b) request alternative, or (c) abort conversation.
8. Upon approval, executor runs the command with stdout/stderr captured and updates both conversation and session context.
9. If successful and step has more commands remaining, repeat steps 7-8; otherwise mark step as Complete and advance to next Pending step.
10. If a command encounters errors: executor emits error information; error is appended to conversation and session context, then forwarded back to model on retry/alternative request. If unrecoverable (user chooses abort or model signals stop), conversation ends with partial completion.
11. When all steps are Complete, conversation status becomes Finished and session context is updated with achievements.

See `PROMPT_HANDLING.md` for detailed state machine specifications and data structures.

## Classification Strategy (Current Implementation)
Primary approach: embedded Python implementation utilizing machine learning models for accurate natural language detection and command classification. The system employs structured JSON communication between Rust and Python components for reliable data exchange.

## Security Considerations (Early Implementation)
- Never auto-execute model output without user approval.
- Display complete command line before execution.
- Log all executions (future enhancement: redact secrets heuristically).

## Extensibility
Trait-based architecture allows swapping classifier and model implementations without affecting UI/executor components. Planning and command synthesis are separated, enabling future scenarios where local/offline models can generate commands while remote models supply only high-level workflow (hybrid mode). A `ModelProvider` registry will allow runtime provider selection (configuration file, environment variable, or UI toggle).

The `parsec-prompt` crate's Python integration provides additional flexibility by isolating ML/LLM logic in Python scripts (located in `py/` subdirectory) while maintaining Rust performance for core operations. This hybrid approach enables rapid iteration on AI models and API integrations without requiring Rust recompilation, while leveraging the Python ML ecosystem for advanced capabilities.

## Future GUI Implementation
Phase 1 implements TUI (crossterm). Future phases: native window interface (egui or GTK) with embedded pty support.
