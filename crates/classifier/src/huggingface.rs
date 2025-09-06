use parsec_core::{ClassificationError, CommandClassifier, InputKind, Session};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize)]
struct HuggingFaceRequest {
    inputs: String,
    parameters: HuggingFaceParameters,
}

#[derive(Debug, Serialize)]
struct HuggingFaceParameters {
    candidate_labels: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct HuggingFaceResponse {
    sequence: String,
    labels: Vec<String>,
    scores: Vec<f64>,
}

pub struct HuggingFaceClassifier {
    client: Client,
    api_token: String,
    model_name: String,
    threshold: f64,
}

impl HuggingFaceClassifier {
    pub fn new(api_token: String) -> Result<Self, ClassificationError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| {
                ClassificationError::ClassificationFailed(format!(
                    "Failed to create HTTP client: {}",
                    e
                ))
            })?;

        Ok(Self {
            client,
            api_token,
            model_name: "facebook/bart-large-mnli".to_string(), // Zero-shot classification model
            threshold: 0.7,
        })
    }

    pub fn with_model(mut self, model_name: String) -> Self {
        self.model_name = model_name;
        self
    }

    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold;
        self
    }

    async fn classify_async(&self, input: &str) -> Result<InputKind, ClassificationError> {
        let url = format!(
            "https://api-inference.huggingface.co/models/{}",
            self.model_name
        );

        let request = HuggingFaceRequest {
            inputs: input.to_string(),
            parameters: HuggingFaceParameters {
                candidate_labels: vec![
                    "shell command".to_string(),
                    "natural language request".to_string(),
                    "system command".to_string(),
                    "conversational prompt".to_string(),
                ],
            },
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                ClassificationError::ClassificationFailed(format!("HTTP request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ClassificationError::ClassificationFailed(format!(
                "API request failed with status {}: {}",
                status, error_text
            )));
        }

        let hf_response: HuggingFaceResponse = response.json().await.map_err(|e| {
            ClassificationError::ClassificationFailed(format!("Failed to parse response: {}", e))
        })?;

        // Determine classification based on highest scoring label
        if let (Some(best_label), Some(&best_score)) =
            (hf_response.labels.first(), hf_response.scores.first())
        {
            if best_score < self.threshold {
                // If confidence is low, fall back to heuristic classification
                return Ok(self.heuristic_fallback(input));
            }

            match best_label.as_str() {
                "shell command" | "system command" => Ok(InputKind::Shell),
                "natural language request" | "conversational prompt" => Ok(InputKind::Prompt),
                _ => Ok(self.heuristic_fallback(input)),
            }
        } else {
            Ok(self.heuristic_fallback(input))
        }
    }

    fn heuristic_fallback(&self, input: &str) -> InputKind {
        let input_lower = input.trim().to_lowercase();
        let first_word = input_lower.split_whitespace().next().unwrap_or("");

        // Common shell commands
        let shell_commands = vec![
            "ls", "cd", "pwd", "mkdir", "rm", "cp", "mv", "cat", "grep", "find", "git", "cargo",
            "npm", "python", "node", "curl", "wget", "ssh", "vim", "nano", "docker", "kubectl",
            "make", "sudo", "chmod", "ps",
        ];

        if shell_commands.contains(&first_word) {
            return InputKind::Shell;
        }

        // Natural language indicators
        if input_lower.contains("please")
            || input_lower.contains("how do i")
            || input_lower.contains("help me")
            || input_lower.contains("can you")
            || input_lower.contains('?')
            || input_lower.starts_with("what")
            || input_lower.starts_with("how")
            || input_lower.starts_with("create")
        {
            return InputKind::Prompt;
        }

        // Default to prompt for ambiguous cases
        InputKind::Prompt
    }
}

impl CommandClassifier for HuggingFaceClassifier {
    fn classify(
        &self,
        input: &str,
        _context: Option<&Session>,
    ) -> Result<InputKind, ClassificationError> {
        // Since this is a sync trait, we need to use a blocking runtime
        // In a real implementation, you might want to use async traits or a different approach
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            ClassificationError::ClassificationFailed(format!(
                "Failed to create async runtime: {}",
                e
            ))
        })?;

        rt.block_on(self.classify_async(input))
    }
}
