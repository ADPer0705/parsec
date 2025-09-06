# Parsec Application Setup

## Successfully Built! 🎉

The Parsec application has been built successfully according to the documentation specifications. Here's how to set it up and run it:

## Prerequisites

1. **Google AI API Key**: 
   - Get an API key from Google AI Studio (https://makersuite.google.com/app/apikey)
   - Set the environment variable: `export GOOGLE_AI_API_KEY="your_api_key_here"`

2. **Hugging Face API Token** (optional for enhanced classification):
   - Get a token from https://huggingface.co/settings/tokens
   - Set the environment variable: `export HUGGINGFACE_API_TOKEN="your_token_here"`

## Running the Application

### Interactive Mode
```bash
cargo run
```

### Execute Single Command
```bash
cargo run -- --execute "create a new Rust project called hello-world"
```

### Specify Working Directory
```bash
cargo run -- --working-dir /path/to/project
```

## Architecture Overview

The application follows a 6-crate architecture as specified in the docs:

- **parsec-core**: Domain types, traits, and error definitions
- **parsec-classifier**: Input classification using Hugging Face (with heuristic fallback)
- **parsec-model**: Google AI Studio integration for workflow planning
- **parsec-executor**: Safe command execution with validation
- **parsec-prompt**: Orchestration layer managing conversation lifecycle
- **parsec-ui**: Command-line interface with interactive sessions

## Features Implemented

✅ Session management with hierarchical context (Session → Conversation → Steps)  
✅ Input classification (Shell vs Prompt) using Hugging Face BART-large-mnli  
✅ AI-powered workflow planning with Google AI Studio  
✅ Safe command execution with risk scoring  
✅ Step-by-step execution with user approval gates  
✅ Conversation context tracking and summarization  
✅ Interactive CLI with command history  
✅ Project-type detection and tool availability checking  

## Usage Examples

1. **Shell Commands**: Execute directly
   ```
   parsec> ls -la
   parsec> git status
   parsec> cargo build
   ```

2. **Natural Language Prompts**: Create AI-assisted workflows
   ```
   parsec> create a new Rust library with basic documentation
   parsec> set up a web server using actix-web
   parsec> analyze this codebase and suggest improvements
   ```

3. **Special Commands**:
   - `help` - Show available commands
   - `status` - Display current session status
   - `exit` - Exit the application

The application will classify your input and either execute shell commands directly or create multi-step AI-assisted workflows for complex tasks.

## Next Steps

1. Set up your API keys
2. Run `cargo run` to start the interactive mode
3. Try both shell commands and natural language prompts
4. The application will guide you through multi-step workflows with approval gates for safety

Enjoy using Parsec! 🚀
