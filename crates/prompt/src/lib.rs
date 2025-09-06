use chrono::Utc;
use parsec_core::*;
use parsec_executor::SafeExecutor;
use std::sync::Arc;
use uuid::Uuid;

pub struct PromptOrchestrator {
    model_provider: Arc<dyn ModelProvider>,
    executor: SafeExecutor,
    session_store: Arc<dyn SessionStore>,
}

impl PromptOrchestrator {
    pub fn new(
        model_provider: Arc<dyn ModelProvider>,
        session_store: Arc<dyn SessionStore>,
    ) -> Self {
        Self {
            model_provider,
            executor: SafeExecutor::new(),
            session_store,
        }
    }

    pub fn with_executor(mut self, executor: SafeExecutor) -> Self {
        self.executor = executor;
        self
    }

    pub fn create_conversation(
        &self,
        session_id: &SessionId,
        user_prompt: String,
    ) -> Result<ConversationContext, anyhow::Error> {
        let conversation_id = Uuid::new_v4().to_string();
        let conversation_name = self.generate_conversation_name(&user_prompt);

        let conversation = ConversationContext {
            id: conversation_id,
            session_id: session_id.clone(),
            name: conversation_name,
            user_prompt,
            workflow: None,
            steps: Vec::new(),
            status: ConversationStatus::Planning,
            history: Vec::new(),
            model_provider: self.model_provider.name().to_string(),
            context_summary: ContextSummary {
                key_achievements: Vec::new(),
                generated_artifacts: Vec::new(),
                environment_changes: Vec::new(),
                learned_preferences: std::collections::HashMap::new(),
            },
        };

        self.session_store.save_conversation(&conversation)?;
        Ok(conversation)
    }

    pub async fn plan_workflow(
        &self,
        conversation: &mut ConversationContext,
        session: &Session,
    ) -> Result<(), anyhow::Error> {
        let planning_opts = PlanningOptions::default();
        let workflow = self
            .model_provider
            .planner()
            .plan(&conversation.user_prompt, session, planning_opts)
            .await?;

        // Initialize step states
        let step_states: Vec<WorkflowStepState> = workflow
            .steps
            .iter()
            .map(|step| WorkflowStepState {
                step: step.clone(),
                status: StepStatus::Pending,
                command_attempts: Vec::new(),
                context_used: StepContext {
                    working_directory: session.global_context.working_directory.clone(),
                    environment_vars: session.global_context.environment_snapshot.clone(),
                    previous_outputs: Vec::new(),
                    error_context: None,
                },
                artifacts_produced: Vec::new(),
            })
            .collect();

        conversation.workflow = Some(workflow);
        conversation.steps = step_states;
        conversation.status = ConversationStatus::Ready;

        // Add planning event to history
        conversation.history.push(ConversationEvent {
            event_type: "workflow_planned".to_string(),
            timestamp: Utc::now(),
            data: serde_json::json!({
                "step_count": conversation.steps.len(),
                "model_provider": conversation.model_provider
            }),
        });

        self.session_store.save_conversation(conversation)?;
        Ok(())
    }

    pub async fn generate_step_commands(
        &self,
        conversation: &ConversationContext,
        session: &Session,
        step_index: usize,
    ) -> Result<GeneratedCommands, anyhow::Error> {
        if step_index >= conversation.steps.len() {
            return Err(anyhow::anyhow!("Step index out of range"));
        }

        let opts = CommandGenOptions::default();
        let commands = self
            .model_provider
            .step_generator()
            .generate_command(conversation, session, step_index, opts)
            .await?;

        Ok(commands)
    }

    pub fn execute_step_command(
        &self,
        conversation: &mut ConversationContext,
        session: &Session,
        step_index: usize,
        command: &GeneratedCommand,
    ) -> Result<CommandAttempt, anyhow::Error> {
        if step_index >= conversation.steps.len() {
            return Err(anyhow::anyhow!("Step index out of range"));
        }

        // Validate the command first
        self.executor.validate_command(&command.command)?;

        // Execute the command
        let working_dir = &session.global_context.working_directory;
        let attempt = self.executor.execute_step_command(command, working_dir)?;

        // Update conversation state
        conversation.steps[step_index]
            .command_attempts
            .push(attempt.clone());

        if attempt.executed && attempt.exit_status == Some(0) {
            conversation.steps[step_index].status = StepStatus::Complete;

            // Check if this was the last step
            if step_index == conversation.steps.len() - 1 {
                conversation.status = ConversationStatus::Finished;
            }
        } else if attempt.error.is_some() {
            conversation.steps[step_index].status = StepStatus::Failed;
        }

        // Add execution event to history
        conversation.history.push(ConversationEvent {
            event_type: "command_executed".to_string(),
            timestamp: Utc::now(),
            data: serde_json::json!({
                "step_index": step_index,
                "command": command.command,
                "exit_status": attempt.exit_status,
                "success": attempt.error.is_none()
            }),
        });

        self.session_store.save_conversation(conversation)?;
        Ok(attempt)
    }

    pub fn abort_conversation(
        &self,
        conversation: &mut ConversationContext,
    ) -> Result<(), anyhow::Error> {
        conversation.status = ConversationStatus::Aborted;

        conversation.history.push(ConversationEvent {
            event_type: "conversation_aborted".to_string(),
            timestamp: Utc::now(),
            data: serde_json::json!({}),
        });

        self.session_store.save_conversation(conversation)?;
        Ok(())
    }

    pub fn get_next_pending_step(&self, conversation: &ConversationContext) -> Option<usize> {
        conversation
            .steps
            .iter()
            .position(|step| step.status == StepStatus::Pending)
    }

    pub fn get_conversation_status_summary(&self, conversation: &ConversationContext) -> String {
        let completed_steps = conversation
            .steps
            .iter()
            .filter(|step| step.status == StepStatus::Complete)
            .count();

        let total_steps = conversation.steps.len();
        let current_status = match conversation.status {
            ConversationStatus::Planning => "Planning",
            ConversationStatus::Ready => "Ready",
            ConversationStatus::InProgress => "In Progress",
            ConversationStatus::Finished => "Finished",
            ConversationStatus::Aborted => "Aborted",
            ConversationStatus::Error => "Error",
        };

        format!(
            "[{}] Step {}/{} ({}) | Provider: {} | Next: {}",
            conversation.name,
            completed_steps,
            total_steps,
            current_status,
            conversation.model_provider,
            if let Some(next_step) = self.get_next_pending_step(conversation) {
                format!("Step {}", next_step + 1)
            } else {
                "Complete".to_string()
            }
        )
    }

    fn generate_conversation_name(&self, user_prompt: &str) -> String {
        // Simple heuristic to generate a user-friendly name
        let words: Vec<&str> = user_prompt.split_whitespace().take(4).collect();
        let mut name = words.join(" ");

        // Capitalize first letter
        if let Some(first_char) = name.chars().next() {
            name = first_char.to_uppercase().collect::<String>() + &name[first_char.len_utf8()..];
        }

        // Truncate if too long
        if name.len() > 40 {
            name = format!("{}...", &name[..37]);
        }

        if name.is_empty() {
            "Untitled Task".to_string()
        } else {
            name
        }
    }

    pub fn update_session_context(
        &self,
        session: &mut Session,
        conversation: &ConversationContext,
    ) -> Result<(), anyhow::Error> {
        // Update last active time
        session.last_active = Utc::now();

        // Add conversation to session if not already present
        if !session.conversations.contains(&conversation.id) {
            session.conversations.push(conversation.id.clone());
        }

        // Update global context based on conversation outcomes
        if conversation.status == ConversationStatus::Finished {
            for achievement in &conversation.context_summary.key_achievements {
                // This would typically be more sophisticated
                // For now, we'll just ensure the achievement is noted
            }

            for env_change in &conversation.context_summary.environment_changes {
                session.global_context.environment_snapshot.insert(
                    env_change.variable_name.clone(),
                    env_change.new_value.clone(),
                );
            }
        }

        self.session_store.save_session(session)?;
        Ok(())
    }
}
