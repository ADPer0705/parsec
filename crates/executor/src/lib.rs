use chrono::Utc;
use parsec_core::*;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

pub struct SafeExecutor {
    max_output_size: usize,
    timeout: Duration,
}

impl Default for SafeExecutor {
    fn default() -> Self {
        Self {
            max_output_size: 64 * 1024,        // 64KB
            timeout: Duration::from_secs(300), // 5 minutes
        }
    }
}

impl SafeExecutor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_max_output_size(mut self, size: usize) -> Self {
        self.max_output_size = size;
        self
    }

    pub fn execute_direct_command(
        &self,
        command: &str,
        working_dir: &Path,
    ) -> Result<DirectCommandExecution, ExecutionError> {
        let start_time = Utc::now();

        // Parse command into program and args
        let mut parts = command.split_whitespace();
        let program = parts
            .next()
            .ok_or_else(|| ExecutionError::CommandNotFound("Empty command".to_string()))?;
        let args: Vec<&str> = parts.collect();

        // Execute the command
        let mut cmd = Command::new(program);
        cmd.args(args)
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = cmd.output().map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => ExecutionError::CommandNotFound(program.to_string()),
            std::io::ErrorKind::PermissionDenied => {
                ExecutionError::PermissionDenied(program.to_string())
            }
            _ => ExecutionError::ExecutionFailed(format!("Failed to execute {}: {}", program, e)),
        })?;

        let stdout = TruncatedText::new(
            String::from_utf8_lossy(&output.stdout).to_string(),
            self.max_output_size,
        );

        let stderr = TruncatedText::new(
            String::from_utf8_lossy(&output.stderr).to_string(),
            self.max_output_size,
        );

        Ok(DirectCommandExecution {
            command: command.to_string(),
            executed_at: start_time,
            exit_status: output.status.code().unwrap_or(-1),
            stdout,
            stderr,
            working_directory: working_dir.to_path_buf(),
        })
    }

    pub fn execute_step_command(
        &self,
        command: &GeneratedCommand,
        working_dir: &Path,
    ) -> Result<CommandAttempt, ExecutionError> {
        let start_time = Utc::now();

        // Check for dangerous patterns
        if let Some(risk_score) = command.risk_score {
            if risk_score > 0.8 {
                return Ok(CommandAttempt {
                    candidate: command.clone(),
                    approved: false,
                    executed: false,
                    exit_status: None,
                    stdout: TruncatedText::new(
                        "Command blocked due to high risk score".to_string(),
                        self.max_output_size,
                    ),
                    stderr: TruncatedText::new(
                        format!("Risk score: {:.2}", risk_score),
                        self.max_output_size,
                    ),
                    error: Some(ExecutionError::ExecutionFailed(
                        "High risk command blocked".to_string(),
                    )),
                    timestamp: start_time,
                });
            }
        }

        // Execute the command
        let execution_result = self.execute_direct_command(&command.command, working_dir)?;

        Ok(CommandAttempt {
            candidate: command.clone(),
            approved: true,
            executed: true,
            exit_status: Some(execution_result.exit_status),
            stdout: execution_result.stdout,
            stderr: execution_result.stderr,
            error: if execution_result.exit_status == 0 {
                None
            } else {
                Some(ExecutionError::ExecutionFailed(format!(
                    "Command exited with status {}",
                    execution_result.exit_status
                )))
            },
            timestamp: start_time,
        })
    }

    pub fn validate_command(&self, command: &str) -> Result<(), ExecutionError> {
        // Basic validation checks
        if command.trim().is_empty() {
            return Err(ExecutionError::ExecutionFailed("Empty command".to_string()));
        }

        // Check for dangerous patterns
        let dangerous_patterns = vec![
            "rm -rf /",
            ":(){ :|:& };:", // Fork bomb
            "mkfs",
            "dd if=/dev/zero",
            "shutdown",
            "reboot",
        ];

        let command_lower = command.to_lowercase();
        for pattern in dangerous_patterns {
            if command_lower.contains(pattern) {
                return Err(ExecutionError::ExecutionFailed(format!(
                    "Dangerous command pattern detected: {}",
                    pattern
                )));
            }
        }

        // Check for unescaped newlines (except in valid cases)
        if command.contains('\n') && !command.contains("<<") {
            return Err(ExecutionError::ExecutionFailed(
                "Unescaped newlines in command".to_string(),
            ));
        }

        Ok(())
    }

    pub fn check_prerequisites(&self, working_dir: &Path) -> Vec<String> {
        let mut warnings = Vec::new();

        if !working_dir.exists() {
            warnings.push(format!(
                "Working directory does not exist: {}",
                working_dir.display()
            ));
        }

        if !working_dir.is_dir() {
            warnings.push(format!(
                "Working directory is not a directory: {}",
                working_dir.display()
            ));
        }

        warnings
    }
}
