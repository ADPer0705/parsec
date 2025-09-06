use chrono::Utc;
use clap::Parser;
use env_logger;
use log::{error, info, warn};
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

use parsec_classifier::{HeuristicClassifier, HuggingFaceClassifier};
use parsec_core::*;
use parsec_executor::SafeExecutor;
use parsec_model::{GoogleAiProvider, InMemorySessionStore};
use parsec_prompt::PromptOrchestrator;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Google AI Studio API key (or set GOOGLE_AI_API_KEY env var)
    #[arg(long)]
    api_key: Option<String>,

    /// Use Hugging Face for classification (requires HUGGINGFACE_API_TOKEN)
    #[arg(long)]
    use_huggingface_classifier: bool,

    /// Working directory
    #[arg(long)]
    working_dir: Option<PathBuf>,

    /// Interactive mode (default)
    #[arg(long)]
    interactive: bool,

    /// Command to execute directly
    #[arg(long)]
    execute: Option<String>,
}

struct ParsecApp {
    classifier: Box<dyn CommandClassifier>,
    orchestrator: PromptOrchestrator,
    session_store: Arc<InMemorySessionStore>,
    current_session: Option<Session>,
}

impl ParsecApp {
    fn new(args: &Args) -> Result<Self, anyhow::Error> {
        // Initialize classifier
        let classifier: Box<dyn CommandClassifier> = if args.use_huggingface_classifier {
            let token = env::var("HUGGINGFACE_API_TOKEN")
                .map_err(|_| anyhow::anyhow!("HUGGINGFACE_API_TOKEN environment variable required for Hugging Face classifier"))?;
            Box::new(HuggingFaceClassifier::new(token)?)
        } else {
            Box::new(HeuristicClassifier::default())
        };

        // Initialize model provider
        let api_key = args
            .api_key
            .clone()
            .or_else(|| env::var("GOOGLE_AI_API_KEY").ok())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Google AI API key required. Set --api-key or GOOGLE_AI_API_KEY env var"
                )
            })?;

        let model_provider = Arc::new(GoogleAiProvider::new(api_key)?);
        let session_store = Arc::new(InMemorySessionStore::new());

        let orchestrator = PromptOrchestrator::new(model_provider, session_store.clone());

        Ok(Self {
            classifier,
            orchestrator,
            session_store,
            current_session: None,
        })
    }

    fn get_or_create_session(
        &mut self,
        working_dir: PathBuf,
    ) -> Result<&mut Session, anyhow::Error> {
        if self.current_session.is_none() {
            let session_id = Uuid::new_v4().to_string();
            let now = Utc::now();

            let session = Session {
                id: session_id,
                created_at: now,
                last_active: now,
                conversations: Vec::new(),
                command_history: Vec::new(),
                global_context: GlobalContext {
                    working_directory: working_dir,
                    environment_snapshot: env::vars().collect(),
                    detected_project_type: None, // TODO: Implement project detection
                    active_tools: Self::detect_tools(),
                },
                settings: SessionSettings::default(),
            };

            self.session_store.save_session(&session)?;
            self.current_session = Some(session);
        }

        Ok(self.current_session.as_mut().unwrap())
    }

    fn get_session(&self, session_id: &str) -> Option<Session> {
        if let Some(session) = &self.current_session {
            if session.id == session_id {
                return Some(session.clone());
            }
        }
        self.session_store
            .load_session(&session_id.to_string())
            .ok()
    }

    fn update_session(&mut self, session: Session) -> Result<(), anyhow::Error> {
        self.session_store.save_session(&session)?;
        if let Some(current) = &mut self.current_session {
            if current.id == session.id {
                *current = session;
            }
        }
        Ok(())
    }

    fn detect_tools() -> Vec<String> {
        let tools = vec![
            "git", "cargo", "npm", "python", "node", "docker", "kubectl", "make", "cmake", "gcc",
            "clang", "rustc", "javac", "mvn",
        ];

        tools
            .into_iter()
            .filter(|tool| {
                std::process::Command::new("which")
                    .arg(tool)
                    .output()
                    .map(|output| output.status.success())
                    .unwrap_or(false)
            })
            .map(|s| s.to_string())
            .collect()
    }

    async fn run_interactive(&mut self, working_dir: PathBuf) -> Result<(), anyhow::Error> {
        println!("Parsec Interactive Mode");
        println!("Working directory: {}", working_dir.display());
        println!("Type 'exit' to quit, 'help' for help\n");

        let session = self.get_or_create_session(working_dir)?;
        let session_id = session.id.clone();

        loop {
            print!("parsec> ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input.is_empty() {
                continue;
            }

            match input {
                "exit" | "quit" => {
                    println!("Goodbye!");
                    break;
                }
                "help" => {
                    Self::print_help();
                    continue;
                }
                "status" => {
                    let session = self.get_session(&session_id).expect("Session should exist");
                    self.print_status(&session)?;
                    continue;
                }
                _ => {}
            }

            let mut session = self
                .get_session(&session_id)
                .expect("Session should exist")
                .clone();
            if let Err(e) = self.process_input(input, &mut session).await {
                error!("Error processing input: {}", e);
                println!("Error: {}", e);
            }
            // Update the session in storage
            self.update_session(session)?;
        }

        Ok(())
    }

    async fn process_input(
        &mut self,
        input: &str,
        session: &mut Session,
    ) -> Result<(), anyhow::Error> {
        let classification = self.classifier.classify(input, Some(session))?;

        match classification {
            InputKind::Shell => {
                info!("Classified as shell command: {}", input);
                self.execute_shell_command(input, session)?;
            }
            InputKind::Prompt => {
                info!("Classified as prompt: {}", input);
                self.handle_prompt(input, session).await?;
            }
        }

        // Update session
        session.last_active = Utc::now();
        self.session_store.save_session(session)?;

        Ok(())
    }

    fn execute_shell_command(
        &mut self,
        command: &str,
        session: &mut Session,
    ) -> Result<(), anyhow::Error> {
        let executor = SafeExecutor::new();
        let result =
            executor.execute_direct_command(command, &session.global_context.working_directory)?;

        println!("Exit status: {}", result.exit_status);
        if !result.stdout.content.is_empty() {
            println!("stdout:\n{}", result.stdout.content);
        }
        if !result.stderr.content.is_empty() {
            println!("stderr:\n{}", result.stderr.content);
        }

        // Add to command history
        session.command_history.push(result);

        Ok(())
    }

    async fn handle_prompt(
        &mut self,
        prompt: &str,
        session: &mut Session,
    ) -> Result<(), anyhow::Error> {
        println!("Creating workflow for: {}", prompt);

        // Create conversation
        let mut conversation = self
            .orchestrator
            .create_conversation(&session.id, prompt.to_string())?;

        // Plan workflow
        self.orchestrator
            .plan_workflow(&mut conversation, session)
            .await?;
        println!("✓ Workflow planned with {} steps", conversation.steps.len());

        // Display workflow
        println!("\nWorkflow: {}", conversation.name);
        for (i, step) in conversation.steps.iter().enumerate() {
            println!("  {}. {}", i + 1, step.step.description);
        }

        // Execute workflow interactively
        self.execute_workflow_interactive(&mut conversation, session)
            .await?;

        Ok(())
    }

    async fn execute_workflow_interactive(
        &mut self,
        conversation: &mut ConversationContext,
        session: &mut Session,
    ) -> Result<(), anyhow::Error> {
        conversation.status = ConversationStatus::InProgress;

        while let Some(step_index) = self.orchestrator.get_next_pending_step(conversation) {
            let step = &conversation.steps[step_index];
            println!("\n→ Step {}: {}", step_index + 1, step.step.description);

            // Generate commands for this step
            let generated_commands = self
                .orchestrator
                .generate_step_commands(conversation, session, step_index)
                .await?;

            if generated_commands.done {
                println!("  Step completed without commands.");
                conversation.steps[step_index].status = StepStatus::Complete;
                continue;
            }

            if generated_commands.commands.is_empty() {
                warn!("No commands generated for step {}", step_index + 1);
                conversation.steps[step_index].status = StepStatus::Failed;
                continue;
            }

            // Show primary command
            let primary_command = &generated_commands.commands[0];
            println!("  Command: {}", primary_command.command);
            println!("  Explanation: {}", primary_command.explanation);

            if let Some(risk_score) = primary_command.risk_score {
                if risk_score > 0.3 {
                    println!("  ⚠️  Risk score: {:.2}", risk_score);
                }
            }

            // Ask for approval
            print!("  Execute? (y/n/a/s) [y=yes, n=no, a=abort, s=skip]: ");
            io::stdout().flush()?;

            let mut response = String::new();
            io::stdin().read_line(&mut response)?;
            let response = response.trim().to_lowercase();

            match response.as_str() {
                "y" | "yes" | "" => {
                    // Execute the command
                    match self.orchestrator.execute_step_command(
                        conversation,
                        session,
                        step_index,
                        primary_command,
                    ) {
                        Ok(attempt) => {
                            if attempt.error.is_none() {
                                println!("  ✓ Command executed successfully");
                                if !attempt.stdout.content.is_empty() {
                                    println!("  Output: {}", attempt.stdout.content);
                                }
                            } else {
                                println!("  ✗ Command failed: {:?}", attempt.error);
                                if !attempt.stderr.content.is_empty() {
                                    println!("  Error: {}", attempt.stderr.content);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to execute command: {}", e);
                            println!("  ✗ Execution error: {}", e);
                        }
                    }
                }
                "n" | "no" => {
                    println!("  Command skipped by user");
                    conversation.steps[step_index].status = StepStatus::Skipped;
                }
                "a" | "abort" => {
                    println!("  Conversation aborted by user");
                    self.orchestrator.abort_conversation(conversation)?;
                    break;
                }
                "s" | "skip" => {
                    println!("  Step skipped by user");
                    conversation.steps[step_index].status = StepStatus::Skipped;
                }
                _ => {
                    println!("  Invalid response, skipping command");
                    conversation.steps[step_index].status = StepStatus::Skipped;
                }
            }

            // Update conversation context
            self.orchestrator
                .update_session_context(session, conversation)?;
        }

        // Print final status
        let status = self
            .orchestrator
            .get_conversation_status_summary(conversation);
        println!("\nFinal status: {}", status);

        Ok(())
    }

    fn print_help() {
        println!(
            r#"
Parsec Help:
  Shell commands: Execute directly (ls, git status, cargo build, etc.)
  Natural language: Create AI-assisted workflows ("create a new Rust project")
  
  Special commands:
    help     - Show this help
    status   - Show current session status  
    exit     - Exit the application
"#
        );
    }

    fn print_status(&self, session: &Session) -> Result<(), anyhow::Error> {
        println!("Session Status:");
        println!("  ID: {}", session.id);
        println!(
            "  Created: {}",
            session.created_at.format("%Y-%m-%d %H:%M:%S")
        );
        println!(
            "  Last active: {}",
            session.last_active.format("%Y-%m-%d %H:%M:%S")
        );
        println!(
            "  Working directory: {}",
            session.global_context.working_directory.display()
        );
        println!(
            "  Active tools: {}",
            session.global_context.active_tools.join(", ")
        );
        println!("  Commands executed: {}", session.command_history.len());
        println!("  Active conversations: {}", session.conversations.len());

        if let Some(project_type) = &session.global_context.detected_project_type {
            println!("  Project type: {}", project_type);
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Load .env file if it exists
    if let Err(_) = dotenvy::dotenv() {
        // .env file not found or couldn't be loaded, continue without it
    }

    env_logger::init();
    let args = Args::parse();

    let working_dir = args
        .working_dir
        .as_ref()
        .map(|p| p.clone())
        .unwrap_or_else(|| env::current_dir().expect("Failed to get current directory"));

    let mut app = ParsecApp::new(&args)?;

    if let Some(command) = args.execute {
        // Execute single command and exit
        let mut session = app.get_or_create_session(working_dir)?.clone();
        app.process_input(&command, &mut session).await?;
        app.update_session(session)?;
    } else {
        // Interactive mode
        app.run_interactive(working_dir).await?;
    }

    Ok(())
}
