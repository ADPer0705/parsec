use async_trait::async_trait;
use parsec_core::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Serialize)]
struct GoogleAiRequest {
    contents: Vec<Content>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Debug, Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct Part {
    text: String,
}

#[derive(Debug, Serialize)]
struct GenerationConfig {
    temperature: f32,
    #[serde(rename = "topK")]
    top_k: u32,
    #[serde(rename = "topP")]
    top_p: f32,
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct GoogleAiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: ResponseContent,
}

#[derive(Debug, Deserialize)]
struct ResponseContent {
    parts: Vec<ResponsePart>,
}

#[derive(Debug, Deserialize)]
struct ResponsePart {
    text: String,
}

pub struct GoogleAiClient {
    client: Client,
    api_key: String,
    model: String,
}

impl GoogleAiClient {
    pub fn new(api_key: String) -> Result<Self, InitError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| InitError::InitError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            api_key,
            model: "gemini-1.5-flash".to_string(),
        })
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    async fn generate_content(&self, prompt: &str) -> Result<String, anyhow::Error> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let request = GoogleAiRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: prompt.to_string(),
                }],
            }],
            generation_config: GenerationConfig {
                temperature: 0.1,
                top_k: 40,
                top_p: 0.95,
                max_output_tokens: 2048,
            },
        };

        let response = self.client.post(&url).json(&request).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Google AI API error: {}", error_text));
        }

        let ai_response: GoogleAiResponse = response.json().await?;

        ai_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .ok_or_else(|| anyhow::anyhow!("No response content from Google AI"))
    }
}

pub struct GoogleAiWorkflowPlanner {
    client: GoogleAiClient,
}

impl GoogleAiWorkflowPlanner {
    pub fn new(api_key: String) -> Result<Self, InitError> {
        let client = GoogleAiClient::new(api_key)?;
        Ok(Self { client })
    }

    fn build_planning_prompt(
        &self,
        user_prompt: &str,
        session_context: &Session,
        _opts: PlanningOptions,
    ) -> String {
        let session_info = format!(
            "Working Directory: {}\nDetected Tools: {}\nProject Type: {}",
            session_context.global_context.working_directory.display(),
            session_context.global_context.active_tools.join(", "),
            session_context
                .global_context
                .detected_project_type
                .as_deref()
                .unwrap_or("Unknown")
        );

        let recent_conversations = if session_context.conversations.len() > 0 {
            format!(
                "Recent conversations: {} active",
                session_context.conversations.len()
            )
        } else {
            "No recent conversations".to_string()
        };

        format!(
            r#"SYSTEM: You are an assistant that decomposes a user goal into a small ordered workflow of logical steps. DO NOT produce shell commands. Output strict JSON format only.

SESSION_CONTEXT:
{}

CONVERSATION_HISTORY:
{}

USER_PROMPT: {}

RESPONSE FORMAT (JSON): {{ "steps": [ {{ "description": "..." }}, ... ] }}

CONSTRAINTS: 
- 1-12 steps maximum
- Each description should be 3-14 words, starting with an imperative verb
- Focus on logical workflow, not specific commands
- Steps should be actionable and sequential
- Consider the current working directory and available tools

Example response:
{{ "steps": [ {{ "description": "Create new Rust project structure" }}, {{ "description": "Initialize git repository" }}, {{ "description": "Configure CI/CD pipeline" }} ] }}"#,
            session_info, recent_conversations, user_prompt
        )
    }
}

#[async_trait]
impl WorkflowPlanner for GoogleAiWorkflowPlanner {
    async fn plan(
        &self,
        user_prompt: &str,
        session_context: &Session,
        opts: PlanningOptions,
    ) -> Result<WorkflowPlan, PlanError> {
        let prompt = self.build_planning_prompt(user_prompt, session_context, opts);

        let response = self
            .client
            .generate_content(&prompt)
            .await
            .map_err(|e| PlanError::ModelError(format!("Model generation failed: {}", e)))?;

        // Parse the JSON response
        let json_start = response.find('{').unwrap_or(0);
        let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
        let json_str = &response[json_start..json_end];

        #[derive(Deserialize)]
        struct PlanResponse {
            steps: Vec<StepData>,
        }

        #[derive(Deserialize)]
        struct StepData {
            description: String,
        }

        let plan_response: PlanResponse = serde_json::from_str(json_str)?;

        let steps = plan_response
            .steps
            .into_iter()
            .map(|s| WorkflowStep {
                id: Uuid::new_v4().to_string(),
                description: s.description,
            })
            .collect();

        Ok(WorkflowPlan { steps })
    }
}

pub struct GoogleAiStepCommandGenerator {
    client: GoogleAiClient,
}

impl GoogleAiStepCommandGenerator {
    pub fn new(api_key: String) -> Result<Self, InitError> {
        let client = GoogleAiClient::new(api_key)?;
        Ok(Self { client })
    }

    fn build_command_prompt(
        &self,
        ctx: &ConversationContext,
        session: &Session,
        step_index: usize,
        _opts: CommandGenOptions,
    ) -> String {
        let current_step = ctx
            .workflow
            .as_ref()
            .and_then(|w| w.steps.get(step_index))
            .map(|s| s.description.clone())
            .unwrap_or_else(|| "Unknown step".to_string());

        let session_info = format!(
            "Working Directory: {}\nDetected Tools: {}\nProject Type: {}",
            session.global_context.working_directory.display(),
            session.global_context.active_tools.join(", "),
            session
                .global_context
                .detected_project_type
                .as_deref()
                .unwrap_or("Unknown")
        );

        let workflow_info = if let Some(workflow) = &ctx.workflow {
            workflow
                .steps
                .iter()
                .enumerate()
                .map(|(i, step)| {
                    let status = if i < step_index {
                        "✓ Complete"
                    } else if i == step_index {
                        "→ Current"
                    } else {
                        "Pending"
                    };
                    format!("{}. {} [{}]", i + 1, step.description, status)
                })
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            "No workflow available".to_string()
        };

        let execution_history = ctx
            .steps
            .iter()
            .take(step_index)
            .filter_map(|step_state| {
                step_state.command_attempts.last().map(|attempt| {
                    format!(
                        "Step: {}\nCommand: {}\nExit Status: {}\nOutput: {}",
                        step_state.step.description,
                        attempt.candidate.command,
                        attempt.exit_status.unwrap_or(-1),
                        if attempt.stdout.content.len() > 200 {
                            format!("{}...", &attempt.stdout.content[..200])
                        } else {
                            attempt.stdout.content.clone()
                        }
                    )
                })
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        format!(
            r#"SYSTEM: You generate safe shell commands for the CURRENT step only.

SECURITY: Avoid destructive commands unless explicitly required; NEVER use 'rm -rf /'. Ask for clarification if ambiguous.

SESSION_CONTEXT:
{}

CONVERSATION_CONTEXT:
Name: {}
Original Prompt: {}

WORKFLOW (all steps):
{}

CURRENT_STEP: Step {} - {}

EXECUTION_HISTORY:
{}

OUTPUT FORMAT (JSON): {{ "commands": [ {{ "command": "...", "explanation": "..." }} ], "done": false }}

If step complete without command: {{ "commands": [], "done": true }}

Provide 1-3 command options. Focus on the current step only. Commands should be safe and appropriate for the current environment."#,
            session_info,
            ctx.name,
            ctx.user_prompt,
            workflow_info,
            step_index + 1,
            current_step,
            if execution_history.is_empty() {
                "No previous commands executed"
            } else {
                &execution_history
            }
        )
    }
}

#[async_trait]
impl StepCommandGenerator for GoogleAiStepCommandGenerator {
    async fn generate_command(
        &self,
        ctx: &ConversationContext,
        session: &Session,
        step_index: usize,
        opts: CommandGenOptions,
    ) -> Result<GeneratedCommands, CommandGenError> {
        let prompt = self.build_command_prompt(ctx, session, step_index, opts);

        let response =
            self.client.generate_content(&prompt).await.map_err(|e| {
                CommandGenError::ModelError(format!("Model generation failed: {}", e))
            })?;

        // Parse the JSON response
        let json_start = response.find('{').unwrap_or(0);
        let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
        let json_str = &response[json_start..json_end];

        #[derive(Deserialize)]
        struct CommandResponse {
            commands: Vec<CommandData>,
            done: bool,
        }

        #[derive(Deserialize)]
        struct CommandData {
            command: String,
            explanation: String,
        }

        let command_response: CommandResponse = serde_json::from_str(json_str)?;

        let commands = command_response
            .commands
            .into_iter()
            .map(|c| {
                let risk_score = self.calculate_risk_score(&c.command);
                GeneratedCommand {
                    command: c.command,
                    explanation: c.explanation,
                    risk_score: Some(risk_score),
                }
            })
            .collect();

        Ok(GeneratedCommands {
            commands,
            done: command_response.done,
        })
    }
}

impl GoogleAiStepCommandGenerator {
    fn calculate_risk_score(&self, command: &str) -> f32 {
        let dangerous_patterns = vec![
            "rm -rf",
            "rm -f /",
            "dd if=",
            "mkfs",
            "format",
            "shutdown",
            "reboot",
            "kill -9",
            "chmod 777",
            ":(){:|:&};:",
        ];

        let mut risk: f32 = 0.0;
        let command_lower = command.to_lowercase();

        for pattern in dangerous_patterns {
            if command_lower.contains(pattern) {
                risk += 0.8;
            }
        }

        if command_lower.contains("sudo") {
            risk += 0.3;
        }

        if command_lower.contains("rm ") && command_lower.contains("*") {
            risk += 0.5;
        }

        risk.min(1.0)
    }
}

pub struct GoogleAiProvider {
    planner: GoogleAiWorkflowPlanner,
    step_generator: GoogleAiStepCommandGenerator,
}

impl GoogleAiProvider {
    pub fn new(api_key: String) -> Result<Self, InitError> {
        let planner = GoogleAiWorkflowPlanner::new(api_key.clone())?;
        let step_generator = GoogleAiStepCommandGenerator::new(api_key)?;

        Ok(Self {
            planner,
            step_generator,
        })
    }
}

impl ModelProvider for GoogleAiProvider {
    fn planner(&self) -> &dyn WorkflowPlanner {
        &self.planner
    }

    fn step_generator(&self) -> &dyn StepCommandGenerator {
        &self.step_generator
    }

    fn name(&self) -> &'static str {
        "google-ai"
    }
}
