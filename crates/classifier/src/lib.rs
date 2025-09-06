use parsec_core::{ClassificationError, CommandClassifier, InputKind, Session};
use serde::{Deserialize, Serialize};

pub mod huggingface;

pub use huggingface::HuggingFaceClassifier;

#[derive(Debug, Serialize, Deserialize)]
pub struct ClassificationRequest {
    pub input: String,
    pub context: Option<ClassificationContext>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClassificationContext {
    pub session_id: Option<String>,
    pub history: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClassificationResponse {
    pub classification: String, // "shell" | "prompt"
    pub confidence: f64,
    pub reasoning: String,
    pub metadata: ClassificationMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClassificationMetadata {
    pub detected_patterns: Vec<String>,
    pub language_indicators: Vec<String>,
}

pub struct HeuristicClassifier {
    shell_commands: Vec<&'static str>,
    prompt_indicators: Vec<&'static str>,
}

impl Default for HeuristicClassifier {
    fn default() -> Self {
        Self {
            shell_commands: vec![
                "ls", "cd", "pwd", "mkdir", "rm", "cp", "mv", "cat", "grep", "find", "git",
                "cargo", "npm", "python", "node", "curl", "wget", "ssh", "scp", "vim", "nano",
                "emacs", "docker", "kubectl", "make", "sudo", "chmod", "chown", "ps", "kill",
                "top", "htop", "df", "du", "tar", "unzip",
            ],
            prompt_indicators: vec![
                "please",
                "how do i",
                "help me",
                "can you",
                "i need",
                "i want",
                "what is",
                "how to",
                "show me",
                "explain",
                "create a",
                "build a",
                "set up",
                "configure",
                "install",
                "initialize",
            ],
        }
    }
}

impl CommandClassifier for HeuristicClassifier {
    fn classify(
        &self,
        input: &str,
        _context: Option<&Session>,
    ) -> Result<InputKind, ClassificationError> {
        let input_lower = input.trim().to_lowercase();

        if input_lower.is_empty() {
            return Ok(InputKind::Shell);
        }

        // Check for shell command patterns
        let first_word = input_lower.split_whitespace().next().unwrap_or("");
        if self.shell_commands.contains(&first_word) {
            return Ok(InputKind::Shell);
        }

        // Check for natural language indicators
        for indicator in &self.prompt_indicators {
            if input_lower.contains(indicator) {
                return Ok(InputKind::Prompt);
            }
        }

        // Check for question patterns
        if input_lower.contains('?')
            || input_lower.starts_with("what")
            || input_lower.starts_with("how")
            || input_lower.starts_with("why")
            || input_lower.starts_with("when")
            || input_lower.starts_with("where")
        {
            return Ok(InputKind::Prompt);
        }

        // Default fallback - if it looks like a command (starts with known pattern), classify as shell
        if first_word.len() > 0
            && (
                first_word.contains('/') ||
            first_word.starts_with("./") ||
            first_word.starts_with("../") ||
            input_lower.contains(" -") ||  // flags pattern
            input_lower.contains(" --")
                // long flags pattern
            )
        {
            return Ok(InputKind::Shell);
        }

        // Default to prompt for conversational input
        Ok(InputKind::Prompt)
    }
}
