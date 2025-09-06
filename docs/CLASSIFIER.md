# Classifier Subsystem

## 🎯 Purpose

The classifier subsystem employs advanced machine learning to accurately discern user input types: shell commands versus natural language prompts requiring AI workflow orchestration. Communication occurs via robust JSON protocols ensuring type-safe, error-resilient Rust-Python interoperability.

## 🧠 Implementation Architecture

### Embedded Python Integration
Utilizing PyO3 for seamless Python embedding, the classification pipeline implements a multi-stage analysis:

1. **Input Sanitization**: Preserves internal whitespace, strips only leading/trailing spaces. Empty inputs default to shell classification.
2. **Command Pattern Matching**: Evaluates initial tokens against curated shell verb patterns using optimized algorithms.
3. **Natural Language Detection**: Identifies conversational cues ("please", "how do I", interrogative structures) indicating NL queries.
4. **ML Classification**: For ambiguous inputs, deploys trained models to differentiate shell vs. NL intents.
5. **Structured Response**: Generates JSON outputs with confidence metrics and explanatory reasoning.

All inter-component communication adheres to strict JSON schemas, guaranteeing reliability and type safety.

## 📡 JSON Communication Protocol

### Classification Request
```json
{
  "input": "string to classify",
  "context": {
    "session_id": "optional session identifier",
    "history": ["previous inputs for context"]
  }
}
```

### Classification Response
```json
{
  "classification": "shell" | "prompt",
  "confidence": 0.95,
  "reasoning": "Detected shell command pattern with high confidence",
  "metadata": {
    "detected_patterns": ["command_verb", "flag_pattern"],
    "language_indicators": []
  }
}
```

## 🚀 Future Enhancements

- **Contextual Awareness**: Leverage conversation history for adaptive classification.
- **User Learning**: Implement preference adaptation and personalization.
- **Configurable Thresholds**: Dynamic confidence tuning via user settings.
- **Custom Rules**: Extensible classification logic through configuration files.
