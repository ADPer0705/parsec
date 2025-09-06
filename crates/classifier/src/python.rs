use parsec_core::{CommandClassifier, InputKind, ClassificationError, Session};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
    pub classification: String,
    pub confidence: f64,
    pub reasoning: String,
    pub metadata: ClassificationMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClassificationMetadata {
    pub detected_patterns: Vec<String>,
    pub language_indicators: Vec<String>,
}

pub struct PythonClassifier {
    py_module: Arc<PyObject>,
}

impl PythonClassifier {
    pub fn new() -> Result<Self, ClassificationError> {
        Python::with_gil(|py| {
            // Add the Python script directory to sys.path
            let sys = py.import("sys")
                .map_err(|e| ClassificationError::PythonError(format!("Failed to import sys: {}", e)))?;
            let path = sys.getattr("path")
                .map_err(|e| ClassificationError::PythonError(format!("Failed to get sys.path: {}", e)))?;
            path.call_method1("append", ("./py",))
                .map_err(|e| ClassificationError::PythonError(format!("Failed to append to sys.path: {}", e)))?;

            // Import the classification module
            let py_module = py.import("classifier")
                .map_err(|e| ClassificationError::PythonError(format!("Failed to import classifier module: {}", e)))?;

            Ok(PythonClassifier {
                py_module: Arc::new(py_module.to_object(py)),
            })
        })
    }

    pub fn with_script_path(script_path: &str) -> Result<Self, ClassificationError> {
        Python::with_gil(|py| {
            let sys = py.import("sys")
                .map_err(|e| ClassificationError::PythonError(format!("Failed to import sys: {}", e)))?;
            let path = sys.getattr("path")
                .map_err(|e| ClassificationError::PythonError(format!("Failed to get sys.path: {}", e)))?;
            path.call_method1("append", (script_path,))
                .map_err(|e| ClassificationError::PythonError(format!("Failed to append to sys.path: {}", e)))?;

            let py_module = py.import("classifier")
                .map_err(|e| ClassificationError::PythonError(format!("Failed to import classifier module: {}", e)))?;

            Ok(PythonClassifier {
                py_module: Arc::new(py_module.to_object(py)),
            })
        })
    }
}

impl CommandClassifier for PythonClassifier {
    fn classify(&self, input: &str, context: Option<&Session>) -> Result<InputKind, ClassificationError> {
        Python::with_gil(|py| {
            // Prepare the classification request
            let mut request = ClassificationRequest {
                input: input.to_string(),
                context: None,
            };

            // Add context if available
            if let Some(session) = context {
                request.context = Some(ClassificationContext {
                    session_id: Some(session.id.clone()),
                    history: session.command_history.iter()
                        .map(|cmd| cmd.command.clone())
                        .take(5)  // Last 5 commands for context
                        .collect(),
                });
            }

            // Serialize request to JSON
            let request_json = serde_json::to_string(&request)
                .map_err(|e| ClassificationError::ClassificationFailed(format!("Failed to serialize request: {}", e)))?;

            // Call the Python classification function
            let result = self.py_module
                .call_method1(py, "classify_input", (request_json,))
                .map_err(|e| ClassificationError::PythonError(format!("Python function call failed: {}", e)))?;

            // Extract the JSON response
            let response_json: String = result.extract(py)
                .map_err(|e| ClassificationError::PythonError(format!("Failed to extract response: {}", e)))?;

            // Deserialize the response
            let response: ClassificationResponse = serde_json::from_str(&response_json)
                .map_err(|e| ClassificationError::InvalidJson(e))?;

            // Convert to InputKind
            match response.classification.as_str() {
                "shell" => Ok(InputKind::Shell),
                "prompt" => Ok(InputKind::Prompt),
                _ => Err(ClassificationError::ClassificationFailed(
                    format!("Unknown classification: {}", response.classification)
                )),
            }
        })
    }
}
