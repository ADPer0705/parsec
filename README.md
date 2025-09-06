# Parsec

Hybrid terminal + AI co-pilot desktop app (Linux first). Open source, minimal, privacy‑aware.

## Goal
Provide a single-window experience where the user can:
1. Type an input.
2. System classifies it as a shell command vs natural-language intent.
3. Shell commands run normally.
4. Prompts invoke a model in TWO phases: (a) workflow planning (logical steps, no shell) then (b) per-step command generation when you advance.
5. User explicitly reviews & approves every generated command before execution (no auto-run by default).

## Status
Scaffolding only. Core traits + placeholder UI harness. No real model calls or terminal emulation yet.

## Quick Start
Prereqs: Rust (stable). On Linux.

```bash
cp .env.example .env   # add your GOOGLE_AI_API_KEY if you plan to prototype model calls later
cargo run -p parsec-ui
```

You should see a placeholder textual UI indicating classification stubs.

## Workspace Layout
```
crates/
  core/                # Shared domain types + traits
  model/               # Model client abstraction (Google AI placeholder)
  executor/            # Shell command execution layer (approval gate, sandbox hooks later)
  prompt/              # Prompt workflow orchestration (planning + per-step command gen)
  classifier-python/   # Embedded Python classifier (optional lightweight local logic)
  ui/                  # Desktop/window layer (for now: simple terminal-like loop)
docs/                  # Architecture & design docs
```

## Design Tenets
- Safety First: No command produced by the model executes without explicit confirmation.
- Transparency: Show reasoning / steps from model (when available) before execution.
- Pluggable Models: Model crate exposes a trait so we can later support local and other providers.
- Minimal Global State: Pass contexts explicitly; ease of testing.

## Python Embedding (Experimental)
We embed a small Python snippet (PyO3) to classify inputs. If Python init fails, we fall back to a pure Rust heuristic automatically.

## Roadmap (High Level)
See `docs/ROADMAP.md` for a phased plan. Detailed prompt flow: `docs/PROMPT_HANDLING.md`.

## Contributing
See `CONTRIBUTING.md` for guidelines. PRs welcome once the MVP shell is in.

## License
MIT
