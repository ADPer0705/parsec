# Parsec 🚀

[![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Linux](https://img.shields.io/badge/Linux-FCC624?style=for-the-badge&logo=linux&logoColor=black)](https://www.linux.org/)
[![MIT License](https://img.shields.io/badge/License-MIT-green.svg)](https://choosealicense.com/licenses/mit/)

A cutting-edge hybrid terminal and AI co-pilot desktop application, engineered for Linux environments. Open-source, minimalist, and privacy-first, Parsec seamlessly blends direct shell command execution with intelligent AI-assisted workflow planning.

## 🎯 Mission

Empower users with a unified, secure interface where:
1. **Direct Execution**: Shell commands run instantly with full transparency.
2. **AI Orchestration**: Natural language prompts trigger two-phase AI workflows: strategic planning followed by step-by-step command synthesis.
3. **Human-in-the-Loop**: Every AI-generated command requires explicit user approval—no auto-execution.
4. **Privacy by Design**: Local processing with optional remote model integration.

## 🚀 Features

- **Intelligent Classification**: Advanced ML-powered input detection distinguishing shell commands from natural language intents.
- **Workflow Planning**: AI-driven decomposition of complex tasks into logical, executable steps.
- **Secure Execution**: Mandatory approval gates with command preview and risk assessment.
- **Context Awareness**: Persistent session and conversation state for coherent multi-step interactions.
- **Extensible Architecture**: Pluggable model providers and classifier implementations.
- **Python Integration**: Embedded Python for ML classifiers via PyO3, with automatic fallback to Rust heuristics.

## 📊 Current Status

**Phase**: Core scaffolding with trait definitions and UI harness.  
**Readiness**: Placeholder implementations; no live model calls or terminal emulation yet.  
**Target**: MVP with Google AI integration and basic workflow execution.

## 🛠️ Quick Start

### Prerequisites
- Rust (stable toolchain)
- Linux environment
- (Optional) Google AI API key for model prototyping

### Installation & Run
```bash
# Clone and setup
git clone https://github.com/ADPer0705/parsec.git
cd parsec
cp .env.example .env  # Configure API keys if needed

# Launch the UI
cargo run -p parsec-ui
```

Expect a textual UI demonstrating classification stubs—ready for your contributions!

## 🏗️ Architecture Overview

```
crates/
├── core/                 # Domain models & core traits
├── model/                # AI provider abstractions (Google AI, local models)
├── executor/             # Command execution with approval & sandboxing
├── prompt/               # Workflow orchestration & step generation
├── classifier/           # ML-based input classification (Python embedded)
└── ui/                   # Desktop interface & interaction loop
```

### Data Flow
- **Shell Path**: Input → Classifier → Direct Execution
- **Prompt Path**: Input → Classifier → Workflow Planning → Step-by-Step Approval → Execution

## 🛡️ Design Principles

- **Safety First**: Zero auto-execution; all AI outputs reviewed.
- **Transparency**: Full visibility into AI reasoning and generated commands.
- **Modularity**: Trait-based design for easy provider swapping.
- **Minimalism**: Explicit context passing; no global state bloat.
- **Privacy**: Local-first with configurable remote options.

## 🔧 Python Embedding

Leverage PyO3 for seamless Python integration in classification. If Python fails, automatic fallback to Rust-based heuristics ensures reliability.

## 🗺️ Roadmap

- **Phase 1**: Core workflow execution with Google AI.
- **Phase 2**: Advanced classifiers and local model support.
- **Phase 3**: GUI polish, sandboxing, and enterprise features.

Detailed roadmap in `docs/ROADMAP.md`. Workflow specs in `docs/PROMPT_HANDLING.md`.

## 🤝 Contributing

We welcome PRs! See `CONTRIBUTING.md` for guidelines. Join us in building the future of AI-assisted terminals.

## 📄 License

MIT License - See `LICENSE` for details.
