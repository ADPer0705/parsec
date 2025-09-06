# Classifier Details

## Purpose
Determines whether user input represents a shell command or a natural-language prompt requiring AI planning through structured JSON communication.

## Implementation

### Embedded Python
The classification system implemented via PyO3 integration provides sophisticated natural language processing capabilities. The classification process follows this sequential logic:

1. Input preprocessing: Preserves internal whitespace while removing only leading and trailing whitespace. Empty inputs default to shell classification.
2. Command recognition: Evaluates whether the first token matches a predefined set of shell command verbs using pattern matching algorithms.
3. Natural language detection: Identifies conversational markers ("please", "how do I", question patterns, etc.) that indicate natural language queries.
4. Machine learning classification: For inputs not definitively categorized by steps 2-3, employs trained ML models to distinguish between shell commands and natural language prompts.
5. JSON response generation: Returns structured classification results with confidence scores and reasoning.

The system communicates exclusively through JSON interfaces, ensuring reliable data exchange between Rust and Python components while maintaining type safety and error handling.

## JSON Communication Protocol

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

## Future Enhancements
- Context-aware classification using conversation history
- User preference learning and adaptation
- Confidence threshold configuration
- Custom classification rules via configuration
